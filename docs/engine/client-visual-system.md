---
title: "Client Visual System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Client Visual System

> **Last updated**: 2026-07-25
> **RE Status**: Verified via Ghidra decompilation of `sgw.exe`
> **Sources**: Ghidra (`GameBeing_setAppearance` @ `0x00e00bc0`, `GameEntity__unknown_00e69150` @ `0x00e69150`), `deprecated/python/cell/SGWPlayer.py`, `deprecated/python/cell/Inventory.py`

---

## Overview

The visual system controls how character and NPC models are rendered on the client. Each entity has a **bodyset** (the base skeleton/mesh group) and a list of **components** (attachable mesh parts for body, armor, weapons, etc.). These are delivered via the `BeingAppearance` entity method call (flattened ClientMethod index 26).

The client resolves bodyset and component names to `.model`/`.visual` files in the game data, loads the meshes, and composites them onto the entity's scene node. When this process fails — due to missing data, empty strings, or timing issues — the client renders a **green placeholder rectangle** instead of the model.

## BeingAppearance Handler Chain

### Wire Format

BeingAppearance is SGWBeing's only own ClientMethod (index 26, direct encoding: msg_id = `0x9A`).

```
[0x9A: u8][word_len: u16 LE][entity_id: u32][bodyset: WSTRING][components: ARRAY<WSTRING>]
```

Where:
- `WSTRING` = `[char_count: u32 LE][UTF-16LE data: char_count × 2 bytes]`
- `ARRAY<WSTRING>` = `[element_count: u32 LE][WSTRING elements...]`

### Client Entry Point: `GameBeing_setAppearance` (0x00e00bc0)

When the client receives a BeingAppearance entity method call, the dispatcher invokes `GameBeing_setAppearance` which:

1. Calls `setBodySetName(bodySet)` — copies the bodyset string to `this+0xd4`
2. Copies the component list to `this+0x128` (internal component vector)
3. Calls the visual update trigger at `GameEntity__unknown_00e69150`

### Visual Update Trigger (0x00e69150) — Three Paths

The visual update trigger checks the entity's readiness state and branches:

#### Path 1: Entity NOT Ready
- **Condition**: A virtual method on the entity returns false (entity not fully initialized)
- **Action**: Logs via the `SGWAppearanceLog` sink (function at `0x00cfefe0`, unnamed in Ghidra; the class RTTI string `.?AVSGWAppearanceLog@@` is at `0x01e21894`). The literal is `' - ENTITY NOT READY'` at `0x019de4a0` — note the leading `' - `, which matters when grepping client logs
- **Result**: **BeingAppearance is SILENTLY DROPPED — no queue, no retry, no error**
- This is the most critical path for understanding the green model bug

#### Path 2: Entity Ready, NOT in Transaction
- **Condition**: Entity is ready and not in a visual transaction
- **Action**: Logs `' - SCHEDULING JOB'` (`0x019de384`), calls `FUN_00e998e0` to queue a model loading job
- **Result**: Model meshes are asynchronously loaded and composited — the normal success path

#### Path 3: Entity Ready, IN Transaction
- **Condition**: Entity is ready but currently in a visual transaction
- **Action**: Logs `' - HOLD FOR TRANSACTION'` (`0x019de40c`)
- **Result**: Appearance update is deferred or dropped

### `setBodySetName` Internals

- Copies the bodyset string to `this+0xd4`
- Checks `this+0xe8` (a readiness/initialization flag)
- If `this+0xe8 == 0`, the bodyset is stored but **no visual update is triggered**
- This means a bodyset can be set without any visual effect until the entity becomes ready

## Entity "Ready" State

The entity "ready" check is the key gating mechanism. An entity must be fully created and initialized before it will accept visual updates. The creation sequence for a player entity is:

1. `CREATE_BASE_PLAYER` (0x05) — creates the base entity
2. `onClientMapLoad` — tells the client which terrain to load
3. *(client loads terrain)*
4. `SPACE_VIEWPORT_INFO` (0x08) — sets up the viewport
5. `CREATE_CELL_PLAYER` (0x06) — creates the cell entity with position
6. `FORCED_POSITION` (0x31) — sets authoritative position

The entity becomes "ready" after the cell entity is created and positioned. Only then will `BeingAppearance` calls succeed.

### Timing Implications

In the C++ reference server, there is a natural delay between steps 5-6 and the `mapLoaded()` bundle (which contains BeingAppearance) because:
- The BaseApp sends VIEWPORT + CREATE_CELL_PLAYER + FORCED_POSITION
- The CellApp processes the entity creation asynchronously
- The CellApp then calls `mapLoaded()` after the entity is fully constructed
- This inter-service round-trip provides enough time for the client to process CREATE_CELL_PLAYER

In the Rust server, VIEWPORT + CELL + POSITION body bytes are prepended to the `mapLoaded` body and fragmented as one bundle (`crates/services/src/mercury/world_data/map_loaded.rs:34-36`), matching the C++ CellApp behaviour where these share a channel bundle. Historically that meant BeingAppearance could arrive before the client had finished processing CREATE_CELL_PLAYER, firing Path 1 and dropping the appearance silently.

Two mitigations are now in place, so do not treat the above as a live bug:

1. **Appearance pre-warm.** `build_create_player` appends `BeingAppearance` + `onEntityTint` immediately after CREATE_BASE_PLAYER and *before* `onClientMapLoad` (`crates/services/src/mercury/world_data/phases.rs:59-91`). The client's async asset load then runs in parallel with the terrain load, rather than starting only when the `mapLoaded` bundle lands.
2. **The enter-world step waits on the client.** The server sends CREATE_BASE_PLAYER + pre-warm + `onClientMapLoad`, then waits for the client's `mapLoaded` (cell method index 25, msg_id `0x99`) before sending viewport + cell player + forced position (`phases.rs:27-35`). `build_enter_world_body` re-emits `BeingAppearance` + `onEntityTint` ahead of `createCellPlayer` (`phases.rs:132-195`).

A related delivery gap does remain open for *non-player* entities — see the AoI create/appearance investigation tracked in issue #582 — but it is downstream of this player-entry path.

## onEntityTint

`onEntityTint` (SGWSpawnableEntity method index 10, msg_id = `0x8A`) sets the color tints on the entity:

```
[0x8A: u8][word_len: u16 LE][entity_id: u32][primaryColorId: u32][secondaryColorId: u32][skinTint: u32]
```

- `primaryColorId` and `secondaryColorId`: Armor/clothing dye indices (default 0 in C++)
- `skinTint`: ARGB color value from the `SKIN_TINTS` lookup table (16 entries, indexed by `SkinTintColorID` 0-15)

The skin tint values are defined in `deprecated/python/common/Constants.py:4-9` (mirrored in Rust as `SKIN_TINTS`, used by both `phases.rs` and `map_loaded.rs`):

```
SKIN_TINTS[0]  = 0x2F1308FF    SKIN_TINTS[8]  = 0xB45B32FF
SKIN_TINTS[1]  = 0x180A08FF    SKIN_TINTS[9]  = 0x632319FF
SKIN_TINTS[2]  = 0x15100DFF    SKIN_TINTS[10] = 0x3A2417FF
SKIN_TINTS[3]  = 0x9C4F22FF    SKIN_TINTS[11] = 0xF8B487FF
SKIN_TINTS[4]  = 0x370405FF    SKIN_TINTS[12] = 0xD57D51FF
SKIN_TINTS[5]  = 0x2F1219FF    SKIN_TINTS[13] = 0xC36141FF
SKIN_TINTS[6]  = 0x6C1F0DFF    SKIN_TINTS[14] = 0xDF8250FF
SKIN_TINTS[7]  = 0x4F1A09FF    SKIN_TINTS[15] = 0x8D3F24FF
```

## Visual Component Assembly

### Two Types of Components

Character appearance is composed of two types of visual components:

1. **Body Components** — base body meshes (torso, legs, head, boots)
   - Source: `char_creation_choices` rows where `item_id IS NULL` (joined to `char_creation_visgroups` for the slot)
   - Stored in: `sgw_player.components` (PostgreSQL `varchar(200)[]` array)
   - Examples: `BS_HumanMale.BS_HM_Torso_00`, `BS_HumanMale.BS_HM_Legs_00`

2. **Item Visual Components** — equipment meshes that overlay body parts
   - Source: `resources.items.visual_component` column
   - Stored in: `sgw_inventory` table (linked to items via `type_id`)
   - Examples: `AR_Global.Prisoner_Torso`, `AR_Global.Prisoner_Legs`, `AR_H_Ballistic00.AR_HM_BB1_BH100`

### Assembly Flow (C++/Python Reference)

In the C++ server with Python scripting:

1. `SGWPlayer.onPlayerLoaded()` (Python) loads `self.components` from DB (body components only)
2. `self.inventory.loadItems()` iterates equipped items, calling `addDbItem()` → `onSlotUpdate()` which **appends item visual components** to `self.components`
3. `mapLoaded()` sends `self.client.BeingAppearance(self.bodySet, self.components)` — by this point, components already includes BOTH body and item visuals

### Assembly Flow (Rust)

The Rust server pre-merges body and item components in a single query:

1. Load `bodyset` and `components` (body components) from `sgw_characters`
2. Query equipped items' `visual_component` from `resources.items` (equipment bags 3-14)
3. `components.extend(item_visuals)` — merge into one list
4. Pass to `build_map_loaded()` → `append_entity_method(BeingAppearance, ...)`

Both approaches produce the same final component list for BeingAppearance.

### Example: Praxis Soldier Male (CharDefId 1)

Component strings are stored fully qualified (`Package.Name`); the bare leaf name is not a valid component. Body vs item is decided by `char_creation_choices.item_id IS NULL`.

| Type | Component Name | Source |
|------|---------------|--------|
| Body | `BS_HumanMale.BS_HM_Torso_00` | char_creation_choices (item_id NULL) |
| Body | `BS_HumanMale.BS_HM_Legs_00` | char_creation_choices (item_id NULL) |
| Body | `BS_HumanMale.BS_HM_Head_XX` | char_creation_choices, `Head` slot is `VIS_Optional` (player choice) |
| Item | `AR_Global.Prisoner_Torso` | char_creation_choices → item 3440 |
| Item | `AR_Global.Prisoner_Legs` | char_creation_choices → item 3437 |
| Item | `AR_H_Ballistic00.AR_HM_BB1_BH100` | char_creation_choices → item 3438 |

Bodyset: `BS_HumanMale.BS_HumanMale` (from `char_creation.body_set` for `char_def_id = 1`)

## Green Placeholder Diagnosis

When the client shows green rectangles instead of a model:

| Symptom | Likely Cause |
|---------|-------------|
| Green rectangle in char sheet, minimap, portrait | Visual slot allocated but model unresolvable |
| Text data (name, level, etc.) works | Entity exists, receives stat data correctly |
| Stats, missions, inventory all work | Only appearance/visual data is affected |

### Possible Causes (ranked by probability)

1. **Timing issue**: BeingAppearance arrives before entity is "ready" → Path 1 silently drops it
2. **Empty bodyset**: `bodyset = ""` at runtime → client can't find `.BS` file
3. **Empty components**: `components = []` at runtime → partial render (green placeholder)
4. **Wrong component names**: Component strings don't match any game data file

### Diagnostic Approach

1. Check server logs for `"mapLoaded: BeingAppearance + onEntityTint data"` — verify bodyset is non-empty
2. Check server logs for `"Player load data: final appearance after visual merge"` — verify components has entries
3. Enable client-side logging to look for `"ENTITY NOT READY"` or `"SCHEDULING JOB"` in SGWAppearanceLog
4. If data is present and entity is ready, compare wire hex bytes against a C++ reference pcap

## Related Documents

- [Entity Type Catalog](entity-type-catalog.md) — Entity class definitions and hierarchy
- [Mercury Wire Format](../protocol/mercury-wire-format.md) — Message encoding details
- [World Entry Phases](../protocol/world-entry-phases.md) — Phase sequence and timing
- [Character Visual Components](character-visual-components.md) — Bodyset/component data sources
