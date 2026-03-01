# Entity Types Wire Formats

> **Date**: 2026-03-01
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `Account.def`, `ClientCache.def`, `SGWEntity.def`, `SGWSpawnableEntity.def`, `SGWPet.def`, `alias.xml`

---

## Overview

This document covers the wire formats for non-player entity types and the Account entity. These are the base classes in the entity hierarchy:

```
Account (standalone — login/character management)
SGWEntity (root game entity)
├── SGWSpawnableEntity (entities with visual representation)
│   ├── SGWBeing (combat-capable entities — see combat-wire-formats.md)
│   │   ├── SGWMob (NPCs — see note below)
│   │   │   └── SGWPet (player pets)
│   │   └── SGWPlayer (player character — see other findings docs)
│   └── SGWDuelMarker (duel zone markers)
└── ... (other non-spawnable entities)
```

**Note**: `SGWBeing.def` and `SGWMob.def` methods are covered in `combat-wire-formats.md` and `inventory-wire-formats.md`. This document covers only additional entity-specific methods not documented elsewhere.

---

## 1. Account Entity

**Parent**: `GamePawn` (UE3)
**Implements**: `ClientCache`

The Account entity is the first entity created when a player connects. It handles character selection before any game entity exists.

### Client → Server (Exposed Base Methods)

#### `logOff` — Disconnect

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `createCharacter` — Create New Character

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `Name` | `WSTRING` | 4B len + N×2B | Character name |
| `ExtraName` | `WSTRING` | 4B len + N×2B | Extra name (surname/title) |
| `CharDefId` | `INT32` | 4B | Character definition ID |
| `VisualChoiceList` | `ARRAY<VisualChoices>` | 4B count + N×8B | Appearance selections |
| `SkinTintColorID` | `INT32` | 4B | Index into SKIN_TINT_COLORS |

**`VisualChoices` FIXED_DICT layout** (8 bytes):

| Field | Type | Size |
|-------|------|------|
| `VisGroupId` | `INT32` | 4B |
| `ChoiceId` | `INT32` | 4B |

#### `playCharacter` — Select Character to Play

| Field | Type | Size |
|-------|------|------|
| `PlayerId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `deleteCharacter` — Delete Character

| Field | Type | Size |
|-------|------|------|
| `PlayerId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `requestCharacterVisuals` — Request Character Appearance

| Field | Type | Size |
|-------|------|------|
| `PlayerId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `onClientVersion` — Report Client Version

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `VersionSeed` | `INT32` | 4B | Version check seed |
| `ClientVersion` | `WSTRING` | 4B len + N×2B | Client version string |
| `LanguageId` | `INT32` | 4B | Client language |

### Server → Client

#### `onCharacterList` — Character Selection Screen

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `CharacterList` | `CharacterInfoList` | ARRAY<CharacterInfo> | All characters on account |

**`CharacterInfo` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `playerId` | `INT32` | 4B |
| `name` | `WSTRING` | 4B len + N×2B |
| `extraName` | `WSTRING` | 4B len + N×2B |
| `alignment` | `INT8` | 1B |
| `level` | `INT8` | 1B |
| `gender` | `INT8` | 1B |
| `worldLocation` | `WSTRING` | 4B len + N×2B |
| `archetype` | `INT8` | 1B |
| `title` | `INT8` | 1B |
| `playerType` | `INT32` | 4B |
| `playable` | `INT8` | 1B |

#### `onCharacterCreateFailed` — Creation Error

| Field | Type | Size |
|-------|------|------|
| `ErrorID` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `onCharacterVisuals` — Character Appearance Data

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `PlayerId` | `INT32` | 4B |
| `BodySet` | `WSTRING` | 4B len + N×2B |
| `Components` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) |
| `primaryTint` | `UINT32` | 4B |
| `secondaryTint` | `UINT32` | 4B |
| `skinTint` | `UINT32` | 4B |

#### `onCharacterLoadFailed`

*(No arguments)* — 1 byte

### Account Properties

| Property | Type | Flags | Notes |
|----------|------|-------|-------|
| `characterList` | `CharacterInfoList` | `BASE` | Character list (persisted) |
| `activePlayerID` | `INT32` | `BASE` | Currently selected character |

---

## 2. ClientCache Interface

**Implemented by**: `Account`

Handles client-side data cache synchronization for cooked game data.

### Client → Server (Exposed Base Methods)

#### `versionInfoRequest` — Check Cache Version

| Field | Type | Size |
|-------|------|------|
| `CategoryId` | `INT32` | 4B |
| `Version` | `INT32` | 4B |

**Total wire size**: 1B header + 8B = **9 bytes**

#### `elementDataRequest` — Fetch Cache Element

| Field | Type | Size |
|-------|------|------|
| `CategoryId` | `INT32` | 4B |
| `Key` | `INT32` | 4B |

**Total wire size**: 1B header + 8B = **9 bytes**

### Server → Client

#### `onVersionInfo` — Cache Version Response

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `CategoryId` | `INT32` | 4B |
| `Version` | `INT32` | 4B |
| `RequiredUpdates` | `INT32` | 4B |
| `InvalidateAll` | `INT8` | 1B |
| `InvalidKeys` | `ARRAY<INT32>` | 4B count + N×4B |

#### `onCookedDataError` — Cache Error

| Field | Type | Size |
|-------|------|------|
| `categoryID` | `INT32` | 4B |
| `elementKey` | `INT32` | 4B |

**Total wire size**: 1B header + 8B = **9 bytes**

---

## 3. SGWEntity (Root Entity)

**Implements**: `DistributionGroupMember`, `EventParticipant`
**Flag**: `<ServerOnly />`

Base entity type. Has NO client methods — all methods are server-internal.

### Properties

| Property | Type | Flags | Notes |
|----------|------|-------|-------|
| `dbID` | `DBID` (INT64) | `CELL_PRIVATE` | Database ID |
| `nextRequestID` | `INT32` | `CELL_PRIVATE` | Request counter |
| `pendingRequests` | `PYTHON` | `CELL_PRIVATE` | Active request tracking |
| `timers` | `PYTHON` | `CELL_PRIVATE` | Timer data |
| `createOnCell` | `MAILBOX` | `BASE` | Cell creation mailbox |

### Volatile Properties

`position`, `yaw`, `pitch`, `roll` — synced by BigWorld engine automatically, not via entity methods.

---

## 4. SGWSpawnableEntity

**Parent**: `SGWEntity`

Entities with visual representation in the world. Contains the core visual/interaction client methods that all game entities inherit.

### Server → Client

#### `onStaticMeshNameUpdate` — Visual Mesh Change

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `StaticMeshName` | `WSTRING` | 4B len + N×2B |
| `BodySetName` | `WSTRING` | 4B len + N×2B |

#### `onSequence` — Kismet Animation Sequence

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `KismetEventSetSeqID` | `INT32` | 4B | Sequence ID |
| `SourceID` | `INT32` | 4B | Entity that triggered |
| `TargetID` | `INT32` | 4B | Target entity |
| `PrimaryTarget` | `INT8` | 1B | 1=primary, 0=secondary |
| `ImpactTime` | `FLOAT` | 4B | Projectile impact delay |
| `NameValuePairs` | `ARRAY<NameValuePair>` | 4B count + N×NVP | Custom overrides |
| `ViewType` | `INT8` | 1B | EKismetViewType |
| `InstanceId` | `INT32` | 4B | Ability/effect instance |

**`NameValuePair` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `name` | `WSTRING` | 4B len + N×2B |
| `value` | `WSTRING` | 4B len + N×2B |

#### `onEntityMove` — Server-Forced Movement

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `locationX` | `FLOAT` | 4B | Position X |
| `locationY` | `FLOAT` | 4B | Position Y |
| `locationZ` | `FLOAT` | 4B | Position Z |
| `velocityX` | `FLOAT` | 4B | Velocity X |
| `velocityY` | `FLOAT` | 4B | Velocity Y |
| `velocityZ` | `FLOAT` | 4B | Velocity Z |
| `yaw` | `FLOAT` | 4B | Rotation yaw |
| `pitch` | `FLOAT` | 4B | Rotation pitch |
| `roll` | `FLOAT` | 4B | Rotation roll |

**Total wire size**: 1B header + 36B = **37 bytes**

**Note**: This is the server-driven entity move (NPC/mob movement, teleport). Player movement uses BigWorld's built-in position updates via the volatile `position` property.

#### `InteractionType` — Interaction Type Notification

| Field | Type | Size |
|-------|------|------|
| `TypeId` | `UINT64` | 8B — EInteractionNotificationType |

**Total wire size**: 1B header + 8B = **9 bytes**

#### `onEntityFlags` — Entity Flags Update

| Field | Type | Size |
|-------|------|------|
| `aFlags` | `UINT64` | 8B |

**Total wire size**: 1B header + 8B = **9 bytes**

#### `onEntityProperty` — Named Property Update

| Field | Type | Size |
|-------|------|------|
| `type` | `INT32` | 4B — property type enum |
| `value` | `INT32` | 4B — property value |

**Total wire size**: 1B header + 8B = **9 bytes**

#### `onVisible` — Visibility Change

| Field | Type | Size |
|-------|------|------|
| `visible` | `INT8` | 1B |

**Total wire size**: 1B header + 1B = **2 bytes**

#### `onKismetEventSetUpdate` — Kismet Set Change

| Field | Type | Size |
|-------|------|------|
| `kismetEventSetId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `onEntityTint` — Color Change

| Field | Type | Size |
|-------|------|------|
| `primaryColorId` | `UINT32` | 4B |
| `secondaryColorId` | `UINT32` | 4B |
| `skinColorId` | `UINT32` | 4B |

**Total wire size**: 1B header + 12B = **13 bytes**

#### `onBeingNameIDUpdate` — Name String ID Change

| Field | Type | Size |
|-------|------|------|
| `BeingNameID` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

### Properties

| Property | Type | Flags | Notes |
|----------|------|-------|-------|
| `kismetEventSetId` | `INT32` | `CELL_PUBLIC` | Active Kismet event set |
| `SpawnSetID` | `INT32` | `BASE` | Spawn set this entity belongs to |
| `staticMeshName` | `WSTRING` | `CELL_PUBLIC` | Visual mesh name |
| `bodySet` | `WSTRING` | `CELL_PUBLIC` | Body set for animation |
| `mobId` | `INT32` | `CELL_PUBLIC` | Mob type definition ID |
| `baseMobId` | `INT32` | `BASE` | Base-stored mob type ID |
| `minigamePlayers` | `ARRAY<INT32>` | `CELL_PRIVATE` | Players in minigame at this entity |
| `entityVariables` | `PYTHON` | `CELL_PRIVATE` | Custom entity variables dict |
| `interactDebug` | `INT8` | `CELL_PRIVATE` | Interaction debug flag |
| `shouldSendKismet` | `INT8` | `CELL_PRIVATE` | Kismet sending enabled |
| `craftingStationControllerID` | `CONTROLLER_ID` (INT32) | `CELL_PRIVATE` | Crafting station controller |
| `spaceCreatorMailbox` | `MAILBOX` | `CELL_PRIVATE` | Space creator reference |

---

## 5. SGWPet Entity

**Parent**: `SGWMob` (→ `SGWBeing` → `SGWSpawnableEntity` → `SGWEntity`)

### Server → Client

#### `onPetAbilityList` — Pet's Abilities

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aAbilityList` | `ARRAY<INT32>` | 4B count + N×4B |

#### `onPetStanceList` — Available Stances

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aStanceList` | `ARRAY<INT8>` | 4B count + N×1B |

**Note**: Stance list uses `INT8` (not `INT32` like the combat doc noted). This is the actual `.def` type.

#### `onPetStanceUpdate` — Current Stance Changed

| Field | Type | Size |
|-------|------|------|
| `aStance` | `INT8` | 1B |

**Total wire size**: 1B header + 1B = **2 bytes**

### Properties

| Property | Type | Flags | Notes |
|----------|------|-------|-------|
| `ownerID` | `INT32` | `CELL_PUBLIC` | Owner player entity ID |
| `ownerBase` | `MAILBOX` | `CELL_PUBLIC` | Owner base mailbox |
| `transferXP` | `FLOAT` | `CELL_PRIVATE` | XP transfer rate |
| `petDespawnTimerId` | `CONTROLLER_ID` | `CELL_PRIVATE` | Despawn timer controller |
| `abilityToResolve` | `INT32` | `CELL_PRIVATE` | Pending ability to execute on spawn |
| `abilityInformation` | `PYTHON` | `CELL_PRIVATE` | Runtime ability params |
| `toggledAbilities` | `ARRAY<INT32>` | `CELL_PRIVATE` | Abilities toggled OFF |
| `lastOwnerPositionCheck` | `FLOAT` | `CELL_PRIVATE` | Last position check time |
| `lastTeleportTime` | `FLOAT` | `CELL_PRIVATE` | Last teleport timestamp |
| `ownerLastPosition` | `VECTOR3` | `CELL_PRIVATE` | Owner's last known position |
| `petLastPosition` | `VECTOR3` | `CELL_PRIVATE` | Pet's last known position |
| `petStance` | `INT8` | `CELL_PRIVATE` | Current stance (default: 1) |

---

## Implementation Notes

1. **Account lifecycle**: Account entity exists from login until character is selected. After `playCharacter`, the Account entity transitions to creating an SGWPlayer entity.
2. **ClientCache**: Used for cooked data versioning — the client tells the server what cache version it has, and the server tells it which elements need updating.
3. **SGWEntity is `<ServerOnly />`**: The entity framework ignores client methods on ServerOnly entities. All SGWEntity methods are server-internal.
4. **onEntityMove vs volatile position**: `onEntityMove` is a server-driven RPC (37 bytes) used for NPCs. Player movement uses BigWorld's optimized volatile position system (much more efficient, delta-compressed).
5. **VisualChoices**: 8-byte FIXED_DICT pairs map visual group IDs to choices for character creation customization.
6. **SGWPet stances use INT8** — more compact than the INT32 used by most IDs. Only a few stances exist (aggressive, defensive, passive, etc.).
7. **Entity flags and properties use UINT64** — 64-bit bitfields allow many flags per entity without multiple properties.
