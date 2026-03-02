# Loot System

The loot system governs how enemies drop items and currency (naquadah) when killed. It consists of a three-layer architecture: static database definitions, Python def objects loaded from those definitions, and a per-entity interaction handler that manages loot state at runtime.

---

## Architecture

| Layer | Component | Role |
|-------|-----------|------|
| Database | `resources.loot_tables`, `resources.loot` | Static loot definitions |
| Def objects | `python/common/defs/LootTable.py` | Python objects loaded from DB at startup |
| Interaction handler | `python/cell/interactions/Lootable.py` | Per-entity loot instance, runtime state |

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

`design_id = NULL` indicates a cash drop (naquadah currency) rather than an item drop. Entity templates reference their loot pool via `entity_templates.loot_table_id`.

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

After the transfer, the entry is removed from the entity's loot list. When the list becomes empty, the loot interaction ends. Items are not returned to the loot list if the player's inventory is full - this edge case is not currently handled.

---

## Eligible Player System

Each `LootableItem` carries an `eligiblePlayerList` (a list of player DBIDs). If the list is non-empty, only players whose DBID appears in the list are shown that item when `sendLootList` runs. If the list is empty, any player can loot the item (free-for-all by default).

Group loot mode constants are defined but not wired up:

```python
GROUP_LOOT_RoundRobin = 0
GROUP_LOOT_FreeForAll = 1
```

The infrastructure for per-item eligibility is in place. The missing piece is population of `eligiblePlayerList` at loot generation time based on group membership and loot mode.

---

## Integration with Mob Death

In `SGWMob.onDead()`:

```python
self.lootHandler.generateLoot()
self.interactionFlags |= Atrea.enums.INT_NormalLoot
```

When a mob dies, it immediately generates its loot and sets the `INT_NormalLoot` interaction flag on itself. This flag makes the mob appear as lootable to nearby players. The loot handler is the `Lootable` interaction interface, which `SGWMob` implements.

---

## Known Issues and TODOs

### Mission Loot Filtering (TODO)
`generateLoot()` has no support for `missionId`, `stepId`, or `objectiveId` filtering. Mission-gated loot (items that should only drop when the player has a specific active mission objective) cannot be configured at the loot table level. This would require augmenting the `loot` table schema and the generation logic.

### 64-bit Signedness Bug (FIXME)
A signedness issue in `interactionSetMapId()` prevents the `INTERACTION_MissionLoot` flag from being used correctly. Mission-specific loot interactions are blocked until this is resolved.

### Range Checking (TODO)
There is no automatic end-of-interaction trigger when a player walks out of range of the loot container. Players who open a loot window and then move away will retain the open window until they manually close it or the server resets it. A distance check on tick or on `onLootItem` would fix this.

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
| Inventory | `python/cell/Inventory.py` | Receives looted items via `pickedUpItem()` and `addCash()` |
| NPC AI | `python/cell/SGWMob.py` | Triggers `generateLoot()` on death, sets loot interaction flag |
| Tapping | `entities/defs/SGWMob.def` | Defines kill credit and loot rights fields (not yet implemented) |
| Groups | Group system | Loot mode affects eligible player assignment (not yet implemented) |
| Missions | Mission system | Mission-gated loot requires objective filtering (not yet implemented) |
