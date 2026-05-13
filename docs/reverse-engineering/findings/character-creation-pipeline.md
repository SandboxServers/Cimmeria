# Character Creation Pipeline — Full Analysis

> **Session**: W-character-creation, Session 4b  
> **Analyst**: Game Archaeology Specialist  
> **Date**: 2026-05-13  
> **RE Status**: Substantially complete. Open questions listed at end.  
> **Sources**: SGW.exe Ghidra analysis, `entities/defs/Account.def`, `docs/gameplay/character-creation.md`

---

## Overview

The character-creation pipeline spans six major phases in the client:

1. The player fills the character-creation form (name, archetype/race selection, visual customizations, skin tint).
2. The UI controller calls `EmitNetOut_CreateCharacter` (`0x00d32ce0`), which packages all fields into a CME `Event_NetOut_CreateCharacter` object and dispatches it.
3. The CME EventSignal system routes the event to `SGWNetworkManager`, which serializes it onto the Mercury wire as an Account base method call (`createCharacter`, method index 2 per `Account.def`).
4. The server validates the request, inserts DB rows, and responds with `onCharacterList`.
5. The client's `GameAccount_HandleNetIn_CharacterList` (`0x00e74060`) processes the response, building the local character roster.
6. The player selects a character and calls `playCharacter`, which triggers world entry (documented in `docs/protocol/world-entry-phases.md`).

---

## CreateCharacter Wire Format

### Client → Server: `createCharacter` (Account base method index 2)

Defined in `entities/defs/Account.def` under `<BaseMethods>`. The Mercury encoding uses the universal RPC dispatcher at `0x00c6fc40`.

**CME Event fields** (set by `EmitNetOut_CreateCharacter` at `0x00d32ce0`):

| Field | Type | Source in client | Notes |
|-------|------|-----------------|-------|
| `Name` | WSTRING | Global `param_1_019bbc58` | Character name string |
| `ExtraName` | WSTRING | `param_2` from UI | Asgard secondary name; empty string for non-Asgard |
| `CharDefId` | INT32 | `*local_64` — resolved from appearance chain | ID of the selected character definition |
| `SkinTintColorID` | INT32 | `this+0x40` on the controller | Index into server-side `Constants.SKIN_TINTS` list |
| `VisualChoiceList` | ARRAY of VisualChoices | Built by `BuildVisualChoiceList` (`0x00d328f0`) | See §Visual Choice List |

**Evidence**: `EmitNetOut_CreateCharacter` decompilation at `0x00d32ce0`. Field names confirmed by string literals at `0x019bbc58` (Name), `0x019bbc60` (ExtraName), `0x019bbc6c` (CharDefId), `0x019bbc78` (SkinTintColorID), `0x019bbc9c` (VisGroupId), `0x019bbca8` (ChoiceId).

### Appearance Chain Resolution

Before filling the event, `EmitNetOut_CreateCharacter` resolves the character's appearance chain through three BST lookups in sequence:

```
AppearanceChain_LookupRaceNode      (0x00d39160)  — race-keyed BST
    → AppearanceChain_LookupArchetypeNode (0x00d388d0)  — archetype-keyed BST
        → AppearanceChain_LookupVisualGroupNode (0x00d37f70) — visual-group BST
```

Each function takes the controller's current selection integer as the key and returns a pointer to the matching node payload (node + 4). If no node exists, a new one is inserted. The `CharDefId` is read from the final resolved node at `*local_64`.

---

## Visual Choice List Semantics

### `BuildVisualChoiceList` (`0x00d328f0`)

Iterates a C++ vector of `VisualGroupEntry` structs (confirmed 0x34 bytes each, stride = 0xD uint32s) and for each selected group emits a `(VisGroupId, ChoiceId)` pair.

**VisualGroupEntry layout** (observed from decompilation):

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| +0x00 | 4 | `VisGroupId` | Group identifier (uint32) — see enum below |
| +0x20 | 4 | `selectedChoiceIndex` | Index into the choices array; -1 (0xFFFFFFFF) = no selection → skipped |
| +0x28 | 4 | `choicesBegin` | Pointer to start of `VisualChoiceEntry[]` |
| +0x2C | 4 | `choicesEnd` | Pointer past end of `VisualChoiceEntry[]` |

**VisualChoiceEntry layout** (0x24 bytes, confirmed by `VisualChoiceVector_GetAt` stride):

| Offset | Size | Field |
|--------|------|-------|
| +0x00 | 4 | `ChoiceId` (uint32) |
| +0x04 | 20+ | Additional data (component name reference, etc.) |

**Output**: a `CME::BasicPropertyList` of trees, each with `VisGroupId` and `ChoiceId` keys. This maps directly to the `VisualChoices` entity-def type (confirmed by `FUN_015d4660` / `FUN_015ce700` entity-def serializers at those addresses).

### VisGroupId Enum (inferred from entity-def serializers)

The `FUN_015d4660` serializer for `VisualGroup` type writes fields `VisGroupId`, `VisType`, and `Choices[]`. The `VisType` is a wstring naming the group category. From surrounding string context and game content knowledge:

| VisGroupId | Probable meaning |
|------------|-----------------|
| (game-content specific) | Head shape |
| (game-content specific) | Hair style |
| (game-content specific) | Face details |
| (game-content specific) | Body variant |

**Open question**: exact VisGroupId integer values require tracing from the `character_creation` resource data (outside binary scope). The entity-def structure is confirmed but the ID-to-name mapping is in the resource files.

---

## SkinTint Resolution

### Client-side: `SkinTintColorID` (index)

The client sends an integer index `SkinTintColorID` (field at `this+0x40` of the character-creation controller). This is **not** an RGB value — it is an index into the server-side `Constants.SKIN_TINTS` list (Python list in the deprecated server).

**Evidence**: `Account.def` confirms field type is `INT32 SkinTintColorID` with comment "Index into Constants.py:SKIN_TINTS for skin tint color".

### Server-side: index → RGB lookup

The server resolves: `skinTint = Constants.SKIN_TINTS[skin_color_id]` and stores the RGB triple in `sgw_player.skin_color_id`.

### Client-side display: packed uint32

When displaying character previews, the server sends three packed uint32 tint values via `onCharacterVisuals`:
- `primaryTint` — UINT32, format `0xRRGGBB00`
- `secondaryTint` — UINT32, format `0xRRGGBB00`  
- `skinTint` — UINT32, format `0xRRGGBB00`

The client unpacks each via `GameEntity_ApplySkinTintColors` (`0x00e6f8b0`) / `FUN_004f6f20`:
- Shift right 8 bits to strip the low zero byte
- Force alpha to 0xFF (opaque)
- Store as 4-float RGBA struct

The source annotation confirms these field names: `primaryColorId`, `secondaryColorId`, `skinColorId` (lowercase; different from the wire field names `primaryTint` / `secondaryTint` / `skinTint`).

**Confirmed by**: `GameEntity_ApplySkinTintColors` at `0x00e6f8b0`, source debug strings `.\\Src\\GameEntity.cpp:0x194-0x196`.

---

## Server-Side Validation Rules

From `docs/gameplay/character-creation.md` (previously documented, cross-verified with binary):

1. **Validate character definition**: `DefMgr.get('character_creation', charDefId)` — fails if charDefId unknown.
2. **Validate visual choices**: each submitted VisGroupId must exist in the definition's `getAllChoices()`.
3. **Validate name uniqueness**: `SELECT player_id FROM sgw_player WHERE name = 'N'` — fails if any row returned.
4. **Validate skin tint**: `skinTintColorId` must be a valid index in `Constants.SKIN_TINTS`.

On failure: `onCharacterCreateFailed` (NetIn event) is sent with error code INT32.

**Client handler**: `register_NetIn_CharacterCreateFailed` at `0x00d78ec0` registers the event. The UI handler is `SGWScriptedWindow_X_UEvent_UI_CharacterCreateFailed___GameEventHandler__vfunc_0` at `0x00ce3630`.

---

## DB Write Structure

From `docs/gameplay/character-creation.md` (previously documented):

**`sgw_player` row** (INSERT):

| Column | Source |
|--------|--------|
| `account_id` | parent Account entity |
| `name` | `Name` field from createCharacter |
| `archetype` | from charDef |
| `alignment` | from charDef |
| `gender` | from charDef |
| `pos_x/y/z` | starting position from charDef or hardcoded default |
| `world_location` | starting world from charDef |
| `components` | from VisualChoiceList component names |
| `skin_color_id` | from `SkinTintColorID` index |
| `abilities` | starting ability list from charDef |
| `access_level` | inherited from account record |

**`sgw_inventory` rows**: starting items from charDef's equipment list, placed via `BagFillOrder` priority rules.

**Completion**: `sendCharacterList()` is called, which triggers `onCharacterList` NetIn.

---

## `onCharacterList` Wire Format and Client Processing

### Server → Client: `onCharacterList` (Account client method index 0)

The server sends a `CharacterInfoList` — an array of `CharacterInfo` objects.

### `CharacterInfo` struct layout (0xC0 bytes)

Recovered from `GameAccount_HandleNetIn_CharacterList` (`0x00e74060`):

| Offset | Type | Field | Notes |
|--------|------|-------|-------|
| +0x00 | uint32 | playerId | Database `player_id` |
| +0x04 | wstring | name | Character name (SSO wstring, 12-byte inline) |
| +0x20 | wstring | extraName | Asgard secondary name |
| +0x3C | byte | alignment | 0-5 enum |
| +0x3D | byte | level | 0-20 |
| +0x3E | byte | gender | 1-3 |
| +0x3F | byte | archetype | 0-7 |
| +0x40 | wstring | worldLocation | Current or starting world name |
| +0x5C | byte | title | Title ID |
| +0x60 | uint32 | playerType | Player type (SGWPlayer vs SGWGmPlayer) |
| +0x64 | byte | playable | Non-zero = can be selected for world entry |
| +0x68 | wstring | bodySet | Body set string (visuals) |
| +0x84 | vector | components | Vector of component wstrings |
| +0x90 | float[4] | primaryTint | RGBA (unpacked) |
| +0xA0 | float[4] | secondaryTint | RGBA (unpacked) |
| +0xB0 | float[4] | skinTint | RGBA (unpacked) |

Total: **0xC0 bytes** (confirmed by `scalable_malloc(0xC0)` in handler).

### Deduplication logic

When a new `onCharacterList` arrives, the handler first removes entries no longer in the server's list. It uses a case-insensitive wstring comparison (`_wcsicmp`) on the composite key `name + "-" + extraName` to identify removals.

### UI refresh

After updating the roster, the handler emits `Event_UI_CharacterListUpdate` to notify the character-select screen.

---

## `onCharacterVisuals` Wire Format

### Server → Client: `onCharacterVisuals` (Account client method index 2)

Sent in response to `requestCharacterVisuals`. Processed by `GameAccount_HandleNetIn_CharacterVisuals` (`0x00e74f50`).

**Wire fields** (from Account.def and handler decompilation):

| Field | Type | Notes |
|-------|------|-------|
| `PlayerId` | INT32 | Identifies which character to update |
| `BodySet` | WSTRING | Skeletal mesh set name |
| `Components` | ARRAY WSTRING | Equipment component names (items worn) |
| `primaryTint` | UINT32 | Packed 0xRRGGBB00 |
| `secondaryTint` | UINT32 | Packed 0xRRGGBB00 |
| `skinTint` | UINT32 | Packed 0xRRGGBB00 |

The handler looks up the character by `PlayerId` in the roster vector (linear scan), then unpacks and stores all three tints as RGBA float tuples.

---

## Character Selection and `playCharacter` → World Entry Handoff

### Client emitter: `GameAccount_EmitNetOut_PlayCharacter` (`0x00e755b0`)

Triggered when the player clicks "Enter World". 

**Guard**: checks `CharacterInfo+0x64` (playable flag) — if zero, the emit is suppressed. This is the client-side enforcement of the "character already in world" protection.

**Wire format**: `Event_NetOut_PlayCharacter` contains a single field `PlayerId` (INT32). This maps to Account base method `playCharacter` (index 3 in `Account.def`).

**Post-dispatch cleanup**: calls `FUN_00cf32f0` and `FUN_0057d070` to reset UI state (pending-preview index, etc.).

### Server processing (from `docs/gameplay/character-creation.md`)

1. Validates `playerId` ownership against account.
2. Checks `ChannelManager.isPlayerOnline()` to prevent duplicate world entry.
3. Selects entity class (`SGWPlayer` or `SGWGmPlayer` based on `access_level`).
4. Calls `Atrea.createCellEntity()` to spawn at stored position.

### World entry sequence (from `docs/protocol/world-entry-phases.md`)

After `playCharacter`:
- Server sends `RESET_ENTITIES (0x04)` → client sends `onClientReady (0x01)`.
- Server sends `CREATE_BASE_PLAYER (0x05)` + `onClientMapLoad` (method 117).
- Client loads terrain, then sends a base method call.
- Server sends `SPACE_VIEWPORT_INFO (0x08)` + `CREATE_CELL_PLAYER (0x07)` + `FORCED_POSITION (0x0C)`.

---

## `deleteCharacter` Wire Format

### Client emitter: `GameAccount_EmitNetOut_DeleteCharacter` (`0x00e756e0`)

**Wire format**: `Event_NetOut_DeleteCharacter` contains a single field `PlayerId` (INT32). Maps to Account base method `deleteCharacter` (index 4 in `Account.def`). No playable-flag check before emit.

**Server processing**: `DELETE FROM sgw_player WHERE player_id = N AND account_id = M` (ownership check). Foreign-key cascade handles cleanup.

---

## Archetype Defaults

From `docs/gameplay/character-creation.md`:

| ID | Name | Alignment |
|----|------|-----------|
| 0 | Soldier | SGC |
| 1 | Commando | SGC |
| 2 | Scientist | SGC |
| 3 | Archeologist | SGC |
| 4 | Asgard | System Lords |
| 5 | Goa'uld | System Lords |
| 6 | Sholva | System Lords |
| 7 | Jaffa | System Lords |

Each archetype definition in `resources.archetypes` includes base stats (coordination, engagement, fortitude, morale, perception, intelligence), derived stats (health, focus, per-level values), and three ability trees.

The `charDef` resource (looked up by `CharDefId`) determines: archetype, alignment, gender, starting position, starting world, starting abilities, starting items.

---

## Key Function Address Table

| Address | Name | Role |
|---------|------|------|
| `0x00d32ce0` | `EmitNetOut_CreateCharacter` | Main emit; packages all 5 fields and dispatches |
| `0x00d37010` | `Event_NetOut_CreateCharacter_Ctor` | 12-byte NetworkEvent constructor |
| `0x00d328f0` | `BuildVisualChoiceList` | Iterates VisualGroup vector → VisGroupId+ChoiceId pairs |
| `0x00d34d30` | `VisualChoiceVector_GetAt` | Array-of-structs accessor, stride=0x24 |
| `0x00d39160` | `AppearanceChain_LookupRaceNode` | Race-keyed BST lookup |
| `0x00d388d0` | `AppearanceChain_LookupArchetypeNode` | Archetype-keyed BST lookup |
| `0x00d37f70` | `AppearanceChain_LookupVisualGroupNode` | VisualGroup-keyed BST lookup |
| `0x00d32370` | `CreateCharacter_PostEmitReset` | Post-dispatch subscription cleanup |
| `0x00d67970` | `SGWNetworkManager_VEvent_NetOut_CreateCharacter___EventHandler__vfunc_0` | SGWNetworkManager dispatch stub |
| `0x00d55bd0` | (unnamed) | CreateCharacter MemberCallback builder → calls `FUN_00a374a0` |
| `0x00e74060` | `GameAccount_HandleNetIn_CharacterList` | Processes onCharacterList, builds CharacterInfo vector |
| `0x00e74f50` | `GameAccount_HandleNetIn_CharacterVisuals` | Processes onCharacterVisuals, unpacks tints |
| `0x00e755b0` | `GameAccount_EmitNetOut_PlayCharacter` | Emits playCharacter (checks playable flag) |
| `0x00e756e0` | `GameAccount_EmitNetOut_DeleteCharacter` | Emits deleteCharacter |
| `0x00d9ab80` | `Event_NetOut_PlayCharacter_Ctor` | PlayCharacter 12-byte ctor |
| `0x00d9a3a0` | `Event_NetOut_DeleteCharacter_Ctor` | DeleteCharacter 12-byte ctor |
| `0x00e6f8b0` | `GameEntity_ApplySkinTintColors` | Unpacks primaryColorId/secondaryColorId/skinColorId from entity events |
| `0x015d4660` | (unnamed) | EntityDef serializer for VisualGroup type |
| `0x015ce700` | (unnamed) | EntityDef serializer for VisualChoice type |
| `0x015d4570` | (unnamed) | EntityDef serializer for CharacterDefinition type |
| `0x00d78980` | `register_NetIn_CharacterList` | Returns `"Event_NetIn_CharacterList"` |
| `0x00d78ec0` | `register_NetIn_CharacterCreateFailed` | Returns `"Event_NetIn_CharacterCreateFailed"` |
| `0x00d78c20` | `register_NetIn_CharacterVisuals` | Returns `"Event_NetIn_CharacterVisuals"` |
| `0x00d79160` | `register_NetIn_onCharacterLoadFailed` | Returns `"Event_NetIn_onCharacterLoadFailed"` |
| `0x00d37070` | `register_NetOut_CreateCharacter` | Returns `"Event_NetOut_CreateCharacter"` |
| `0x00d9abe0` | `register_NetOut_PlayCharacter` | Returns `"Event_NetOut_PlayCharacter"` |
| `0x00d9a400` | `register_NetOut_DeleteCharacter` | Returns `"Event_NetOut_DeleteCharacter"` |
| `0x00d9ae80` | `register_NetOut_RequestCharacterVisuals` | Returns `"Event_NetOut_RequestCharacterVisuals"` |

---

## Open Questions

1. **VisGroupId enum values**: the integer values for head/hair/face/body visual groups are in the `character_creation` resource files, not the binary. Requires tracing from the resource loader or a complete resource dump.

2. **`CharDefId` resolution path**: `AppearanceChain_LookupRaceNode` / `LookupArchetypeNode` / `LookupVisualGroupNode` are BST operations but the BST is populated from data outside the binary (resource files). The exact mapping of race+archetype → CharDefId is not in the binary.

3. **`FUN_00a374a0` full call chain**: this is the signal-tree dispatch/subscription inserter called by all NetOut EventHandlers. It routes through a subscriber set but we did not trace it to the Mercury serializer (the actual `startProxyMessage` / `startEntityMessage` call). The Mercury path for `createCharacter` goes through the universal RPC at `0x00c6fc40` via the SGWNetworkManager subscription.

4. **`SKIN_TINTS` list**: the actual RGB values are in `python/base/Constants.py` (deprecated server). The count of skin tints is unknown from binary alone.

5. **`requestCharacterVisuals` emit path**: we confirmed the `NetIn` handler but did not trace the `NetOut` emitter for `requestCharacterVisuals` (the client-side trigger). It uses `Event_NetOut_RequestCharacterVisuals` (registered at `0x00d9ae80`) — the actual emit function was not located.

6. **`playable` flag source**: `CharacterInfo+0x64` (playable byte) is set from the server's `CharacterInfoList` entry. It is unclear when the server sets this to zero — presumably when `activePlayerID` != 0 for that character (already in world). Requires server-side verification.

---

## Cross-Reference Targets

| Finding | Update Target |
|---------|--------------|
| `CharacterInfo` struct layout (0xC0 bytes, field offsets) | Append as pending struct to W0 checkpoint; `docs/gameplay/character-creation.md` §DB schema |
| `VisualGroupEntry` (0x34 bytes) and `VisualChoiceEntry` (0x24 bytes) struct layouts | Append as pending structs to W0 checkpoint |
| `onCharacterVisuals` tint wire format (`0xRRGGBB00`) | `docs/protocol/message-catalog.md`; `entities/defs/Account.def` comment |
| New addresses in table above | `docs/reverse-engineering/address-map.md` §Character creation |
| `playable` flag client enforcement | `docs/gameplay/character-creation.md` §Playing a Character |
