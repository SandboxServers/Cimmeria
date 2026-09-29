---
title: "Loot System"
type: reference
audience: engineers
last_updated: 2026-09-28
---

# Loot System

The loot system governs how enemies drop items and currency (naquadah) when killed. It consists of a three-layer architecture: static database definitions, Python def objects loaded from those definitions, and a per-entity interaction handler that manages loot state at runtime.

---

## Architecture

| Layer | Component | Role |
|-------|-----------|------|
| Database | `resources.loot_tables`, `resources.loot` | Static loot definitions |
| Def objects | `deprecated/python/common/defs/LootTable.py` | Python objects loaded from DB at startup |
| Interaction handler | `deprecated/python/cell/interactions/Lootable.py` | Per-entity loot instance, runtime state |

---

## Database Schema

```sql
CREATE TABLE loot_tables (
    loot_table_id integer NOT NULL,
    description   character varying NOT NULL
);

CREATE TABLE loot (
    loot_id       integer NOT NULL,
    loot_table_id integer NOT NULL,
    design_id     integer,           -- NULL = naquadah (cash) drop
    min_quantity  integer NOT NULL,
    max_quantity  integer NOT NULL,
    probability   real    NOT NULL   -- constraint: > 0.0 AND <= 1.0
    -- constraint: min_quantity > 0 AND max_quantity >= min_quantity
);
```

`design_id = NULL` indicates a cash drop (naquadah currency) rather than an item drop. Entity templates reference their loot pool via `entity_templates.loot_table_id`. A spawn can override its template's table with `spawnlist.loot_table_id` (NULL falls back to the template); the spawn loader COALESCEs the two. The Castle hall before the Interrogation Block uses it to roll table 7 on six spawns of templates shared with the rest of the Castle (Decision (@Cadacious, 2026-09-28)).

---

## Def Objects

`LootTable.py` defines two classes: `Loot` for individual drop entries and `LootTable` as the container.

```python
class Loot(object):
    def __init__(self, row, defMgr):
        self.id          = row['loot_id']
        self.design      = defMgr.require('item', row['design_id'])  # None if NULL
        self.minQuantity = row['min_quantity']
        self.maxQuantity = row['max_quantity']
        self.probability = row['probability']

class LootTable(Resource):
    def __init__(self, row, defMgr):
        self.id          = row['loot_table_id']
        self.description = row['description']
        self.loot        = []  # populated after init with Loot objects
```

`defMgr.require('item', design_id)` returns `None` when `design_id` is `NULL`, which is how cash drops are identified at the def level.

---

## Loot Types

```python
LOOT_Item = 1    # has design_id - grants an item to inventory
LOOT_Cash = 2    # naquadah - design_id is None, adds currency
```

---

## Loot Generation Algorithm

`Lootable.randomizeLoot(table)` is fully implemented. Each entry in the table is rolled independently with its own probability. This is a "roll each" system, not a "pick one" system. Multiple items can drop from a single table in a single pass.

```python
def randomizeLoot(self, table):
    for item in table:
        rand = random.random()          # [0.0, 1.0)
        if rand <= item.probability:
            quantity = item.minQuantity + random.randint(
                0, item.maxQuantity - item.minQuantity
            )
            if quantity > 0:
                if item.design is not None:
                    self.addLoot(item.design.id, quantity)  # item drop
                else:
                    self.addLoot(None, quantity)             # cash drop
```

The quantity is a uniform random integer in `[minQuantity, maxQuantity]`. Entries with `probability = 1.0` always drop at their rolled quantity. Entries with lower probabilities are skipped entirely when the roll exceeds the threshold.

---

## Interaction Flow

The following table shows the status of each method on the `Lootable` interaction handler:

| Method | Status | Description |
|--------|--------|-------------|
| `generateLoot()` | DONE | Gets template's loot table, calls `randomizeLoot()` |
| `randomizeLoot(table)` | DONE | Independent probability roll per entry |
| `onInteract(player, mapId)` | DONE | Opens loot window, sends item list to player |
| `sendLootList(player, initial)` | DONE | Filters by eligibility, sends `onLootDisplay` |
| `onLootItem(player, index)` | DONE | Validates, transfers item or adds cash |
| `addLoot(designId, quantity)` | DONE | Appends entry to per-entity loot list |

---

## Loot Transfer Mechanics

When a player takes an item from the loot window:

- **Item drops:** `player.inventory.pickedUpItem(item.design.id, item.quantity)`
- **Cash drops:** `player.inventory.addCash(item.quantity)`

After the transfer, the entry is removed from the entity's loot list. When the list becomes empty, the loot interaction ends. The Python reference did not return an item whose transfer failed.

In Cimmeria the cell removes the entry and sends `GrantItem` with the corpse and index it came from. If the base refuses the grant before anything commits (the bag is full, the item may only sit in a vault, a database error), it answers `LootGrantRefused` and the cell puts the item back at its index on the same corpse, restores the loot bit if the list had emptied, refreshes an open loot window and tells the looter why. A corpse that respawned in the meantime does not get it. See [inventory-system.md](inventory-system.md) for where a looted item lands.

---

## Eligible Player System

Each `LootableItem` carries an `eligiblePlayerList` (a list of player DBIDs). If the list is non-empty, only players whose DBID appears in the list are shown that item when `sendLootList` runs. If the list is empty, any player can loot the item (free-for-all by default).

Group loot mode constants are defined but not wired up:

```python
GROUP_LOOT_RoundRobin = 0
GROUP_LOOT_FreeForAll = 1
```

The infrastructure for per-item eligibility is in place. The missing piece is population of `eligiblePlayerList` at loot generation time based on group membership and loot mode.

## Live Containers

Decision (@Cadacious, 2026-09-28). A chest or crate opens the corpse loot window **without being killed**. The content action `open_loot` ([content-engine.md](../content/content-engine.md)) does it from an `interact_tag` chain on the container:

- It rolls the loot table with the same per-row algorithm as a corpse (`roll_loot_entries` in `crates/cell-combat/src/cell/abilities/loot_drop.rs`), **for the clicking player only**, and stores the roll on the container in `CellEntity::container_loot`, keyed by `player_id`. Two players never share or take each other's roll; a guessed index from another player finds nothing.
- It sends the same `onLootDisplay` bytes the corpse window uses (`cimmeria_wire::cell::loot::serialize_on_loot_display`), and `lootItem` / Loot All take from the looter's roll through `SpaceManager::loot_list_mut`, with the same range re-check and the same refused-grant restore (the item goes back into the looter's roll, and the line says "left in the container").
- The container keeps its template interaction flags (`INT_NormalLoot` for the loot cursor) and never dies. When a roll empties, only that looter's entry is removed.
- `once_per_character` makes the roll happen once per character, ever: the container key (the chain's `container_key`, else the spawn tag) is added to `sgw_player.looted_containers` when the window opens with loot, carried into the cell by `InitPlayerState`, so a relog, a respawn or a new instance never re-rolls. Without it (the debug-hub crate) every open re-rolls and replaces the pending roll.
- Loot left in the window stays pending for that character for as long as the container entity lives, because the client sends nothing when the window closes. The next press reopens it. A server restart, or a fresh per-player instance, loses it.
- Every press that opens nothing sends one feedback line, with a `reason=` on `event=loot.container_refused`.

| Container | Tag | Loot tables | Chains |
|---|---|---|---|
| Debug-hub crate (template 304) | `DebugHub_LootCrate` | 3, repeatable | 7020 |
| Cellblock weapon/armor crate (template 13) | `Cellblock_WoodenCrate` | 10 (non-Jaffa), 11 (Jaffa), once per character | 1098, 1099, 1191 |
| Castle pre-Romney chest (template 410) | `Castle_PreRomneyChest` | 8 (non-Jaffa), 9 (Jaffa), once per character, 703 active | 1274, 1275, 1276 |

Not client-verified yet: `Loot.lua` has no dead-target check, but a loot window on a live entity has not been seen in the client ([unified UAT guide](../guides/unified-uat.md), K22).

---

## Integration with Mob Death

In `SGWMob.onDead()`:

```python
self.lootHandler.generateLoot()
self.interactionFlags |= Atrea.enums.INT_NormalLoot
```

When a mob dies, it immediately generates its loot and sets the `INT_NormalLoot` interaction flag on itself. This flag makes the mob appear as lootable to nearby players. The loot handler is the `Lootable` interaction interface, which `SGWMob` implements.

**Rust: NPC-only kills roll nothing (#1009).** When the killer is a plain NPC (NPC-vs-NPC combat; not a player, not a pet), `abilities::death::apply_death_transition` skips the roll, so the corpse gets no loot and no loot cursor, and writes `loot.drop event=skipped reason=npc_only_kill` instead. See [combat-system.md, NPC-vs-NPC kills pay nobody](combat-system.md#npc-vs-npc-kills-pay-nobody-1009).

---

## Known Issues and TODOs

### Mission Loot Filtering (TODO)
`generateLoot()` has no support for `missionId`, `stepId`, or `objectiveId` filtering. Mission-gated loot (items that should only drop when the player has a specific active mission objective) cannot be configured at the loot table level. This would require augmenting the `loot` table schema and the generation logic.

### 64-bit Signedness Bug (FIXME)
A signedness issue in `interactionSetMapId()` prevents the `INTERACTION_MissionLoot` flag from being used correctly. Mission-specific loot interactions are blocked until this is resolved.

### Range Checking (server-authority gate, #446)
`handle_loot_item` re-validates the looter's **live** distance to the corpse
against `MAX_INTERACT_DISTANCE` (5.0) on **every** `lootItem` call, not just
once at interact time. Out-of-range takes are denied (the drop stays on the
corpse, nothing is granted, a `warn!` is logged). This closes the
"vacuum loot" exploit — chained with a position spoof, the interact-time
`looting_entity` pin could otherwise be replayed to loot every corpse in
the zone without traversing to them (CAT-D, #446).

There is still no automatic end-of-interaction trigger that *closes the loot
window* when a player walks out of range — the window lingers client-side
until manually closed — but a take from that stale window now fails the
range gate. **Not yet implemented**: kill-credit / loot ownership (a player
who dealt no damage can still loot a corpse they walk up to) — the larger
SGW lootability-window model, the follow-up half of #446.

### Dynamic Interaction Type Update (FIXME)
There is a known workaround in place for the lack of a proper dynamic interaction type update. When the loot list empties, the entity's interaction flags should clear `INT_NormalLoot`, but the mechanism for pushing that update to nearby clients is not cleanly implemented.

### Per-Player Interaction Flags (TODO)
Per-player interaction flags are not updated after a player loots the last item visible to them. A player who has taken everything eligible for them should see the loot interaction end, but the flag update is not propagated correctly.

---

## Content Gap: Empty Loot Tables

The loot system code is complete and functional. The current gap is data, not code. The `resources.sql` schema defines the loot table structure, and at least one loot table (table ID 2, assigned to the Cellblock Guard at `template_id = 15`) exists, but the population of actual drop entries in the `loot` table is sparse or empty for most enemy types.

This means enemies die and become lootable, but `randomizeLoot()` iterates over an empty list and produces no drops. Populating `resources.sql` with appropriate entries is the primary content task before the loot system becomes visible to players.

---

## Recommended Implementation Path

1. **Populate loot tables** - Add drop entries to `resources.sql` for all existing entity templates. Define probabilities, quantity ranges, and item design IDs for each enemy type.

2. **Wire group loot** - When a mob dies with a group participating in combat, assign `eligiblePlayerList` per `LootableItem` based on the group's loot mode (`RoundRobin` or `FreeForAll`).

3. **Implement tapping** - Tie loot rights to the entity or group that dealt first damage or most damage. `SGWMob.tappedEntity` and `SGWMob.tappedSquad` fields exist but are not connected to loot eligibility.

4. **Implement mission loot** - Extend the `loot` table with optional `mission_id`, `step_id`, and `objective_id` columns. Filter entries in `generateLoot()` against the looting player's active objectives.

5. **Add XP on kill** - Mob death currently triggers loot generation but no experience grant. A `giveExperience()` call alongside `generateLoot()` in `SGWMob.onDead()` is the natural insertion point.

---

## Related Systems

| System | File | Relationship |
|--------|------|--------------|
| Inventory | `deprecated/python/cell/Inventory.py` | Receives looted items via `pickedUpItem()` and `addCash()` |
| NPC AI | `deprecated/python/cell/SGWMob.py` | Triggers `generateLoot()` on death, sets loot interaction flag |
| Tapping | `entities/defs/SGWMob.def` | Defines kill credit and loot rights fields (not yet implemented) |
| Groups | Group system | Loot mode affects eligible player assignment (not yet implemented) |
| Missions | Mission system | Mission-gated loot requires objective filtering (not yet implemented) |
