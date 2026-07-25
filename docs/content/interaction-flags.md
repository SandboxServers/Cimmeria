---
title: "Interaction Flags Reference"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Interaction Flags Reference

Every clickable thing in the game world — NPCs, switches, corpses, vendors, ring transporters — exposes a 64-bit `EInteractionNotificationType` bitmask to the client. The client uses that mask to decide:

- whether the cursor highlights the entity at all on hover,
- which icon/category the right-click context menu shows ("talk to", "loot", "minigame", etc.),
- whether to draw a quest indicator (`?` available, `!` active, `?` turn-in) over the entity's head.

If a content chain forgets to set the right bit, the entity is invisible to right-click even though every other piece of wiring (chain trigger, conditions, server-side handler) is correct. That was the cause of the [HackTheRings_Switch bug](../../db/resources/Content/Seed/castle_cellblock_chains.sql) and is the reason this doc exists.

**Authoritative source**: [`entities/defs/enumerations.xml`](../../entities/defs/enumerations.xml) — the original BigWorld enum. Mirror in [`deprecated/python/Atrea/enums.py`](../../deprecated/python/Atrea/enums.py). Stored on the Rust side as raw `i64` in [`crates/entity/src/cell_entity/mod.rs`](../../crates/entity/src/cell_entity/mod.rs) field `interaction_type_flags`.

**Symbolic names are now preferred in chain SQL.** [`crates/entity/src/interaction_flags.rs`](../../crates/entity/src/interaction_flags.rs) defines the named constants and a `mask_for_name` lookup, and the content loader accepts either form for a `set_interaction_type` action's `mask` param — an integer literal (`'mask': 256`) or a symbolic name (`'mask': 'INT_MinigameLivewire'`), resolved at [loader/action.rs:86-103](../../crates/content-engine/src/loader/action.rs#L86-L103). Prefer the symbolic form; an unrecognized name logs a `warn!` and defaults the mask to 0. Both the misspelled `INT_*Avaliable` and the corrected `INT_*Available` spellings are accepted.

## How to use this in chain SQL

A `set_interaction_type` content action OR's, AND-NOT's, or replaces bits on every entity in the space whose `tag` matches `target_key`:

```sql
INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
VALUES (1014, 'set_interaction_type', NULL, '329_CellDoorButton',
        '{"op": "|", "mask": 256}', 0, 1);   -- becomes Livewire-clickable
```

Operations:

| `op` | Meaning | Use when |
|------|---------|----------|
| `\|` | Set bit (`flags \|= mask`) | Make an entity become clickable / show indicator |
| `~`  | Clear bit (`flags &= ~mask`) | Click is consumed; revoke clickability |
| `=`  | Replace (`flags = mask`) | Rare; prefer `\|`/`~` so flags compose |

**Lifecycle pattern.** Mirror every set with a clear. The canonical worked example is the cellblock door button (mission 638): chain 1014/1015 set `mask=256` when the player asks the prisoner to open the door, chain 1017 clears `mask=256` after Livewire is won. Same shape for [`329_CellDoorButton` in `Prisoner_329.py:41,77,84`](../../deprecated/python/cell/missions/Castle_CellBlock/Prisoner_329.py).

## When to use which bit

Pick by what the player should *see*. The bits group into four families.

### 1. NPC right-click categories — bits 1-12 (vendors, trainers, minigames)

These set the "verb" on the right-click cursor. Set when an NPC becomes interactable for that role; clear when they no longer should be.

| Bit | Mask | Constant | Use for |
|-----|------|----------|---------|
| 1 | `2` | `INT_Banker` | Banker NPC right-click → bank UI |
| 2 | `4` | `INT_Auction` | Auction-house NPC |
| 3 | `8` | `INT_Pvp` | PvP queue NPC |
| 4 | `16` | `INT_Dhd` | Dial-Home Device (stargate) |
| 5 | `32` | `INT_RingNetwork` | Ring transporter — right-click to use rings |
| 6 | `64` | `INT_Organization` | Faction / org NPC |
| 7 | `128` | `INT_Trainer` | Ability trainer |
| 8 | `256` | `INT_MinigameLivewire` | Hackable console (Livewire minigame) |
| 9 | `512` | `INT_MinigameActivate` | Activate-style minigame |
| 10 | `1024` | `INT_MinigameAnalyze` | Analyze-style minigame |
| 11 | `2048` | `INT_MinigameBypass` | Bypass-style minigame |
| 12 | `4096` | `INT_MinigameConverse` | Converse-style minigame |

**Pitfall**: a hackable switch needs its specific minigame bit (`256` for Livewire, `512` for Activate, etc.). Use whichever matches the `start_minigame` action's `target_key` in the same chain.

### 2. Vendor sub-categories — bits 13-21

Vendor NPCs OR multiple of these together to advertise their stock filter. The client decides which tab to show in the vendor UI.

| Bit | Mask | Constant |
|-----|------|----------|
| 13 | `8192` | `INT_VendorArmor` |
| 14 | `16384` | `INT_VendorWeapons` |
| 15 | `32768` | `INT_VendorConsumables` |
| 16 | `65536` | `INT_VendorGeneral` |
| 17 | `131072` | `INT_VendorMission` |
| 18 | `262144` | `INT_VendorCraftBio` |
| 19 | `524288` | `INT_VendorCraftPower` |
| 20 | `1048576` | `INT_VendorCraftMaterials` |
| 21 | `2097152` | `INT_VendorCraftElectronics` |

### 3. Mission state indicators — bits 22-29

These draw the floating quest icon over an NPC's head. Always paired: set the new state on the same chain that completes the previous one.

| Bit | Mask | Constant | Cursor / icon |
|-----|------|----------|---------------|
| 22 | `4194304` | `INT_AStoryMissionPending` | A-story mission about to be offered |
| 23 | `8388608` | `INT_AStoryMissionAvaliable` *(sic)* | `?` — main-story mission available |
| 24 | `16777216` | `INT_AStoryMissionActive` | `!` — main-story mission in progress (talk to advance) |
| 25 | `33554432` | `INT_AStoryMissionTurnIn` | `?` — main-story mission ready to turn in |
| 26 | `67108864` | `INT_NonAStoryMissionPending` | Side-quest pending |
| 27 | `134217728` | `INT_NonAStoryMissionAvaliable` *(sic)* | `?` side quest |
| 28 | `268435456` | `INT_NonAStoryMissionActive` | `!` side quest |
| 29 | `536870912` | `INT_NonAStoryMissionTurnIn` | `?` side-quest turn-in |

> **Spelling**: the original tokens are `Avaliable` (typo). Keep the typo in any constant names mirrored from BigWorld; the values are the values.

**Worked example**: [`SGC_W1` chains 3001/3003/3015`](../../db/resources/Content/Seed/sgc_w1_chains.sql) toggle bit 24 (`16777216`) on Hammond → Tealc → ... as the SGU intro mission progresses.

### 4. Quest world objects, loot, machines, attack — bits 30+

These are mostly used for environmental items (containers, corpses, machines, loot drops). Bits 53-63 are at the high end of the UINT64 — be careful in JSON, JavaScript can't represent them precisely; the chain executor in [`crates/services/src/cell/content/executor/world/`](../../crates/services/src/cell/content/executor/world/) reads them as `i64` so only bits 0-62 are usable.

| Bit | Mask | Constant | Use for |
|-----|------|----------|---------|
| 30 | `1073741824` | `INT_MissionWorldObject` | Quest item glow (interactable corpses, vials, etc.) |
| 31 | `2147483648` | `INT_MissionWaypoint` | Quest waypoint marker |
| 32 | `4294967296` | `INT_DrossPile` | Crafting-resource node |
| 53 | `9007199254740992` | `INT_Attackable_In_Poor_Cover` | Auto-set by combat system |
| 54 | `18014398509481984` | `INT_Attackable_In_Normal_Cover` | Auto-set by combat system |
| 55 | `36028797018963968` | `INT_Attackable_In_Good_Cover` | Auto-set by combat system |
| 56 | `72057594037927936` | `INT_Machine_ReverseEng` | Reverse-engineering machine |
| 57 | `144115188075855872` | `INT_Machine_Biomedical` | Bio crafting machine |
| 58 | `288230376151711744` | `INT_Machine_Power` | Power crafting machine |
| 59 | `576460752303423488` | `INT_Machine_Materials` | Materials crafting machine |
| 60 | `1152921504606846976` | `INT_Machine_Electronics` | Electronics crafting machine |
| 61 | `2305843009213693952` | `INT_Attackable` | Attackable mob (set by [`SGWMob.py`](../../deprecated/python/cell/SGWMob.py)) |
| 62 | `4611686018427387904` | `INT_NormalLoot` | Lootable corpse — auto-set on death by [`SGWMob.py:129`](../../deprecated/python/cell/SGWMob.py#L129) |
| 63 | `9223372036854775808` | `INT_MissionLoot` | **Bit 63 — sign bit of `i64`. Cannot be expressed; do not use until [`interaction_type_flags`](../../crates/entity/src/cell_entity.rs) is widened to `u64`.** |

The `Attackable_In_*Cover` bits are set/cleared by the combat system, not by content chains. Don't write them in seed SQL.

`INT_NormalLoot` and `INT_Attackable` are set automatically by NPC scripts — content chains should set `INT_MissionLoot` only when a body should be lootable *only* for mission purposes (not normal loot). At time of writing the loot system has 3 entries total ([content audit](README.md)), so this case is rare.

## Worked patterns

### Pattern A — single-use console (start a minigame)

The 329 cellblock door:

```sql
-- Player picks dialog option that asks Prisoner 329 to open the cell:
(1014, 'advance_step', 638, '2115', '{}', 0, 0),
(1014, 'set_interaction_type', NULL, '329_CellDoorButton',
        '{"op": "|", "mask": 256}', 0, 1);  -- now Livewire-clickable

-- Player wins Livewire:
(1017, 'advance_step', 638, '2116', '{}', 0, 0),
(1017, 'set_interaction_type', NULL, '329_CellDoorButton',
        '{"op": "~", "mask": 256}', 0, 2);  -- revoke
```

### Pattern B — same entity, two phases (HackTheRings)

A console that's first hackable (Livewire) and then usable (rings) on the same right-click target:

```sql
-- Mission accept → Livewire-clickable
(1034, 'set_interaction_type', NULL, 'HackTheRings_Switch',
        '{"op": "|", "mask": 256}', 0, 2);

-- Livewire victory → swap icons: clear Livewire, set RingNetwork
(1042, 'set_interaction_type', NULL, 'HackTheRings_Switch',
        '{"op": "~", "mask": 256}', 0, 1),
(1042, 'set_interaction_type', NULL, 'HackTheRings_Switch',
        '{"op": "|", "mask": 32}', 0, 2);

-- Used the rings (teleport into region 2) → clear RingNetwork
(1044, 'set_interaction_type', NULL, 'HackTheRings_Switch',
        '{"op": "~", "mask": 32}', 0, 2);
```

### Pattern C — quest indicator over an NPC

Col. Marsh becomes the next-step NPC after mission 640:

```sql
-- Teleport completes 640 → put `?` over ColMarsh's head
(1044, 'set_interaction_type', NULL, 'Preparation_ColMarsh',
        '{"op": "|", "mask": 8388608}', 0, 1);  -- INT_AStoryMissionAvaliable

-- After Marsh hands off mission 641, swap to Active marker on the next NPC.
```

## Verifying it worked

After loading new chain SQL, the easiest sanity check is:

1. Restart the server, log in past the chain trigger.
2. Tail [`logs/content.log`](../../logs/content.log) — you should see a `Content: set interaction type entity_id=... target_id=... operation=... mask=... old=N new=M` line at the moment the chain fires.
3. In game, hover over the entity. If your cursor doesn't change, the bit didn't reach the client — check that the entity tag matches exactly (case-sensitive) and that `target_id` resolved (not `0`).
4. Right-click. If the click is silent on the client side, [`logs/dispatch.log`](../../logs/dispatch.log) and [`logs/interactions.log`](../../logs/interactions.log) will be empty for the entity — that's the smoking gun for "interaction bit missing."

## Gotchas

- **Tag mismatch**: bit-set fails silently when the spawn's `tag` differs from `target_key` by even one character. The classic cellblock had `Preparation_ColMarshr` (extra `r`); the seed SQL fixes it to `Preparation_ColMarsh`.
- **AoI window**: a chain that fires `set_interaction_type` before the entity is in the player's AoI defers the update until the entity enters AoI. See [`crates/services/src/cell/content/executor/`](../../crates/services/src/cell/content/executor/) — search for `deferring InteractionType to AoI create`.
- **Per-player vs broadcast**: dialog-driven interaction sets are per-player ([dialog_set_map flow in executor/dialog.rs](../../crates/services/src/cell/content/executor/dialog.rs)); `set_interaction_type` is global on the entity. If two players are on different mission steps and need different cursors on the same NPC, you need a per-player override (see `add_dialog_set` action), not raw flag bits.
- **Bit 63**: `INT_MissionLoot` is the sign bit of `i64` and cannot be set as a positive integer in PostgreSQL `bigint`. Live with it until the field is widened.

## Related

- [docs/technical/game-systems.md § Interaction Type Bitmask](../technical/game-systems.md#interaction-type-bitmask-einteractionnotificationtype) — partial summary table (older)
- [docs/content/mission-chains.md](mission-chains.md) — full chain catalog
- [docs/architecture/data-driven-content-engine.md](../architecture/data-driven-content-engine.md) — how chains, triggers, conditions, and actions fit together
