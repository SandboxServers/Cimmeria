# Mission System Wire Formats

> **Date**: 2026-03-01
> **Phase**: 3 — Missing Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `Missionary.def`, `SGWPlayer.def`, `alias.xml`, Ghidra decompilation of universal RPC dispatcher (`0x00c6fc40`)

---

## Overview

The mission system tracks quest progress through a hierarchical structure: **Mission → Step → Objective → Task**. Each level has its own update message. Mission assignment and control are cell methods; progress notifications are client methods.

**Interface**: `Missionary` (implemented by `SGWPlayer`)

---

## Client → Server Messages

### Cell Methods (Exposed)

#### `abandonMission` — Drop a Mission

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `MissionID` | `INT32` | 4B | Mission definition ID |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `shareMission` — Share with Squad

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `MissionID` | `INT32` | 4B | Mission definition ID |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `shareMissionResponse` — Accept/Decline Shared Mission

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `Choice` | `INT8` | 1B | Non-zero = accept |

**Total wire size**: 1B header + 1B = **2 bytes**

### Base Methods (Exposed) — via SGWPlayer.def

These are on SGWPlayer directly, not the Missionary interface.

#### `chosenRewards`

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `MissionID` | `INT32` | 4B | Mission to claim rewards for |
| `Choices` | `RewardChoices` | FIXED_DICT (variable) | Selected reward options |

**`RewardChoices` FIXED_DICT layout**:

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `GroupChoices` | `ARRAY<GroupChoice>` | 4B count + N × GroupChoice |

**`GroupChoice` FIXED_DICT layout** (8+ bytes each):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `GroupId` | `UINT32` | 4B |
| `Choices` | `ARRAY<UINT32>` | 4B count + N × 4B |

---

## Server → Client Messages

### `onMissionUpdate` — Mission Status Change

Sent when a mission is assigned, advanced, completed, or failed.

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `MissionID` | `INT32` | 4B | Mission definition ID |
| `Status` | `INT8` | 1B | Mission status enum |
| `MissionGiverName` | `INT32` | 4B | String ID of mission giver NPC |

**Total wire size**: 1B header + 9B = **10 bytes**

### `onStepUpdate` — Step Status Change

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `StepID` | `INT32` | 4B | Step definition ID |
| `Status` | `INT8` | 1B | Step status enum |

**Total wire size**: 1B header + 5B = **6 bytes**

### `onObjectiveUpdate` — Objective Status Change

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `ObjectiveID` | `INT32` | 4B | Objective definition ID |
| `Status` | `INT8` | 1B | Objective status enum |
| `Hidden` | `INT8` | 1B | 1 = hidden from player |
| `Optional` | `INT8` | 1B | 1 = optional objective |

**Total wire size**: 1B header + 7B = **8 bytes**

### `onTaskUpdate` — Task Progress Change

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `TaskID` | `INT32` | 4B | Task definition ID |
| `Status` | `INT8` | 1B | Task status enum |
| `Count` | `INT32` | 4B | Progress count (e.g., 3/5 enemies killed) |

**Total wire size**: 1B header + 9B = **10 bytes**

### `offerSharedMission` — Shared Mission Prompt

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `MissionId` | `INT32` | 4B | Mission being shared |

**Total wire size**: 1B header + 4B = **5 bytes**

---

## Server-Only Methods (Not on Wire)

These cell methods are called by server systems (GM tools, other entities) and are NOT exposed to the client.

| Method | Args | Notes |
|--------|------|-------|
| `missionAssign` | `MAILBOX aCaller`, `WSTRING DesignID`, `UINT8 DisplayPopup` | Assign mission by design ID string |
| `missionClear` | `MAILBOX aCaller`, `WSTRING DesignID` | Clear a specific mission |
| `missionClearActive` | `MAILBOX aCaller` | Clear all active missions |
| `missionClearHistory` | `MAILBOX aCaller` | Clear completed mission history |
| `missionList` | `MAILBOX aCaller` | List active missions (GM tool) |
| `missionListFull` | *(none)* | List all missions with full detail |
| `missionDetails` | `MAILBOX aCaller`, `WSTRING DesignID` | Show mission details (GM tool) |
| `missionAdvance` | `MAILBOX aCaller`, `WSTRING DesignID`, `INT32 StepToAdvanceTo` | Force-advance mission step |
| `missionReset` | `MAILBOX aCaller`, `WSTRING DesignID`, `INT32 StepToAdvanceTo` | Reset mission to a specific step |
| `missionComplete` | `MAILBOX aCaller`, `WSTRING DesignID`, `INT8 TurnIn` | Force-complete mission |
| `missionSetAvailable` | `MAILBOX aCaller`, `WSTRING DesignID` | Make mission available to player |
| `shareMissionOffer` | `INT32 MissionID`, `MAILBOX SenderBaseMailbox`, `INT32 MissionGiverID` | Internal: deliver shared mission offer |
| `requestAbandonMission` | `INT32 MissionID` | Internal: process abandon request |
| `remoteMissionEvent` | `INT32 entityId`, `PYTHON event` | Cross-cell mission event delivery |

---

## Custom Types from alias.xml

### `MissionReward` FIXED_DICT (8 bytes)

| Field | Type | Size |
|-------|------|------|
| `itemDef` | `INT32` | 4B |
| `quantity` | `INT16` | 2B |
| `group` | `INT8` | 1B |
| `id` | `INT8` | 1B |

### `MissionTaskStatus` FIXED_DICT (6 bytes)

| Field | Type | Size |
|-------|------|------|
| `taskID` | `INT32` | 4B |
| `status` | `INT8` | 1B |
| `count` | `INT8` | 1B |

### `MissionObjectiveStatus` FIXED_DICT (variable)

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `objectiveID` | `INT32` | 4B |
| `status` | `INT8` | 1B |
| `tasks` | `ARRAY<MissionTaskStatus>` | 4B count + N × 6B |

### `MissionStepStatus` FIXED_DICT (variable)

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `stepID` | `INT32` | 4B |
| `status` | `INT8` | 1B |
| `objectives` | `ARRAY<MissionObjectiveStatus>` | 4B count + N × (9B + tasks) |

### `MissionStatus` FIXED_DICT (variable)

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `missionID` | `INT32` | 4B |
| `status` | `INT8` | 1B |
| `currentStep` | `INT8` | 1B |
| `recievedBy` | `INT32` | 4B (sic — typo in original) |
| `steps` | `ARRAY<MissionStepStatus>` | 4B count + N × (variable) |

### `Rewards` FIXED_DICT (variable)

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `XP` | `UINT32` | 4B |
| `Naquadah` | `UINT32` | 4B |
| `ItemGroups` | `ARRAY<ItemGroup>` | 4B count + N × ItemGroup |

### `ItemGroup` FIXED_DICT (variable)

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `GroupId` | `UINT32` | 4B |
| `NumChoices` | `UINT32` | 4B |
| `Items` | `ARRAY<RewardItem>` | 4B count + N × 8B |

### `RewardItem` FIXED_DICT (8 bytes)

| Field | Type | Size |
|-------|------|------|
| `ItemId` | `UINT32` | 4B |
| `Index` | `UINT32` | 4B |

---

## Properties

All Missionary properties are `CELL_PRIVATE` or `CELL_PUBLIC` — none have `ALL_CLIENTS` or `OWN_CLIENT` flags, so **none are sent to the client via property sync**.

| Property | Type | Flags | Notes |
|----------|------|-------|-------|
| `missions` | `PYTHON` | `CELL_PRIVATE` | Full mission state dict |
| `publicMissionData` | `ARRAY<PYTHON>` | `CELL_PUBLIC` | (missionId, currentStep) tuples — propagated to other cells only |
| `completedMissions` | `ARRAY<PYTHON>` | `CELL_PUBLIC` | (missionId, numCompletions) — excluded from client (see entity-property-sync.md) |
| `currentMissionLoot` | `PYTHON` | `CELL_PRIVATE` | Loot tables for mission items |
| `bShowMissionDebug` | `INT8` | `CELL_PRIVATE` | Debug flag |
| `missionProcessQueue` | `PYTHON` | `CELL_PRIVATE` | Queued mission operations |
| `pendingMissionAccepts` | `ARRAY<INT32>` | `CELL_PRIVATE` | Pending acceptance queue |
| `pendingMissionShares` | `PYTHON` | `CELL_PRIVATE` | Pending shared mission data |

**Note**: `publicMissionData` and `completedMissions` are `CELL_PUBLIC` but are in the client property exclusion list (see `entity-property-sync.md`), so they are NOT sent to the owning client.

---

## Implementation Notes

1. **Mission hierarchy**: Mission → Steps → Objectives → Tasks. Each level has independent status tracking and client notification.
2. **GM commands** use `WSTRING DesignID` (human-readable string) while player-facing commands use `INT32 MissionID` (numeric database ID).
3. **Mission sharing** is a multi-step flow: share → offer → response → assign.
4. **Cross-cell events**: `remoteMissionEvent` delivers mission events to entities on other cells via `PYTHON` event blob.
5. **The `recievedBy` field** in `MissionStatus` has a typo (should be "receivedBy") — preserved from original `.def`.
