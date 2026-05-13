# Mission State Machine — Client-Side Evidence

> **Date**: 2026-05-13
> **Session**: V5 Campaign — W-mission-state (Session 4b)
> **Confidence**: HIGH for handlers; MEDIUM for repeatable reset; LOW for ChosenRewards → inventory chain
> **Sources**: Ghidra decompilation of MissionSet handler functions; `Missionary.def`; `SGWPlayer.def`; `alias.xml`; `docs/reverse-engineering/findings/mission-wire-formats.md`; `docs/content/content-engine.md`

---

## Overview

The Stargate Worlds client maintains a complete in-memory mirror of the player's mission state, rooted in a singleton `MissionSet` object. The server drives all state transitions; the client is purely a display layer that reacts to five incoming wire messages (`onMissionUpdate`, `onStepUpdate`, `onObjectiveUpdate`, `onTaskUpdate`, `onMissionRewards`) plus timer and sharing signals. No client-authoritative mission logic exists in SGW.exe — every transition originates from a server-side Python event dispatched over the BigWorld entity method channel.

This document records the state machine as recovered from decompilation. See `docs/reverse-engineering/findings/mission-wire-formats.md` for wire-format byte layouts and `docs/content/content-engine.md` for the Rust content-engine actions that drive transitions from the server side.

---

## MissionSet Data Structures

### MissionSet Object (recovered field layout)

`MissionSet` is the central subscriber for all incoming mission events. Its field layout was reconstructed from four handler functions.

| Offset | Type | Name | Evidence |
|--------|------|------|----------|
| `+0x00` | `vtable*` | vtable | Confirmed from all handlers — `this[0]` is always the vtable |
| `+0x58` | `StepEntry* / list*` | `stepList` | Accessed in `FUN_00d18cf0` (onStepUpdate), `FUN_00d18a30` (TimerUpdate type 0x0A) |
| `+0x60` | unknown | mission list root or map | Accessed in `FUN_00d1a270` (onMissionUpdate) — store `missionGiverName` here |
| `+0x64` | `MissionEntry* / list*` | `missionList` | Accessed in `FUN_00d18a30` (TimerUpdate type 0x0B); also in `FUN_00d1a270` for status store |
| `+0x70` | `TaskEntry* / list*` | `taskList` | Accessed in `FUN_00d194b0` (onTaskUpdate) |
| `+0x7C` | `TimerState*` | `timerState` | Accessed in `FUN_00d18a30`: `FUN_00c6d1c0(this+0x7C, ...)` |
| `+0x9C` | `DWORD` | `missionLoadState[0]` | Zeroed in `FUN_00d19a40` (MissionOffer handler) |
| `+0xA0` | `DWORD` | `missionLoadState[1]` | Zeroed in `FUN_00d19a40` |
| `+0xA4` | `DWORD` | `missionLoadState[2]` | Zeroed in `FUN_00d19a40` |
| `+0xA8` | `DWORD` | `missionLoadState[3]` | Zeroed in `FUN_00d19a40` |

### MissionEntry Object (recovered field layout)

| Offset | Type | Name | Evidence |
|--------|------|------|----------|
| `+0x60` | `INT32` | `missionGiverName` | Written in `FUN_00d1a270` when mission already exists |
| `+0x64` | `INT8` / `INT32` | `status` | Read/written in `FUN_00d1a270`; value drives activation/propagation branch |
| `+0x100` | `INT8` | `status2` (duplicate or second field) | Written in `FUN_00d1a270` with the same Status value |

Note: `+0x64` and `+0x100` both receive the `Status` byte in the onMissionUpdate handler. This may reflect a primary status field vs. a cached/mirror field, or the decompiler conflated two different structures. Needs follow-up.

### StepEntry Object (recovered field layout)

| Offset | Type | Name | Evidence |
|--------|------|------|----------|
| `+0x3C` | `INT8` | `status` | Written in `FUN_00d18cf0` (onStepUpdate) |

### ObjectiveEntry Object (recovered field layout)

| Offset | Type | Name | Evidence |
|--------|------|------|----------|
| `+0x30` | `INT8` | `status` | Written in `FUN_00d18fd0` (onObjectiveUpdate) |
| `+0x34` | `INT8` | `optionalFlag` | Written in `FUN_00d18fd0` with the `Optional` wire field |

### TaskEntry Object (recovered field layout — array-of-scalar layout)

Tasks appear to be stored as an array of scalar values (not structs with named fields). The task update handler accesses:

| Index | Type | Name | Evidence |
|-------|------|------|----------|
| `[2]` | `INT8` | `status` | Written in `FUN_00d194b0` (onTaskUpdate) |
| `[3]` | `INT32` | `count` | Written in `FUN_00d194b0` |

---

## CME Event Bus Architecture

### Startup Registration

All MissionSet subscriptions are installed at startup by `RegisterBulkNetOutSignals` (`0x00db3390`). This function spans approximately 64 KB and registers every CME event → handler mapping for the entire game.

The per-event registration wrapper for `onMissionUpdate` is at `0x00daa680`. It:
1. Stores the factory function pointer (`MissionSet_onMissionUpdate_Subscriber` @ `0x00d9fc00`) as `local_3c`
2. Calls `FUN_0158ed60` — signal name lookup (BigWorld signal registry)
3. Calls `FUN_004649a0` — RB-tree insert (installs the factory into the signal dispatch table)

The factory at `0x00d9fc00` (108 bytes):
- Allocates 0xC-byte NetworkEvent via `scalable_malloc`
- Stamps vtable
- Calls `FUN_00d80c70` — `Event_NetIn_onMissionUpdate` constructor

### Event Object Field Layout

All four mission update events share the same in-memory layout for the common fields:

```
[+0x00] vtable*          — dispatch table
[+0x04] subject_ptr*     — subscriber reference (MissionSet instance)
[+0x08] TypeDescriptor*  — RTTI accessor (slot 3 of MemberCallback)
[+0x0C] INT32            — primary ID field (MissionID / StepID / ObjectiveID / TaskID)
[+0x10] INT8             — Status byte
[+0x11] INT8             — extra byte (Hidden for objective; zero for others)
```

CME field reads use named field lookup:
- `FUN_00e3cba0` — reads INT32 field by name from CME event object
- `FUN_00d434d0` — reads INT8/byte field by name from CME event object
- `FUN_00e3cc20` — reads pointer/bool field by name

---

## Mission Lifecycle State Machine

### States

Three status values are confirmed from handler logic:

| Status Value | Semantic | Evidence |
|-------------|----------|----------|
| `0` | Removed / failed / inactive | `FUN_00d1a270` fires token `0x138F` ("mission removed") when Status==0 |
| `1` | Accepted / active | `FUN_00d1a270` fires token `0x1393`, calls `FUN_00d17c10` (ActivateMission) when Status==1 |
| `2` | Completed | Inferred from token logic; no explicit Status==2 branch confirmed in decompile |

### onMissionUpdate Handler (`FUN_00d1a270`)

**Wire fields read**: `MissionID` (INT32), `Status` (INT8), `MissionGiverName` (INT32)

**Algorithm**:
1. Call `FUN_00d16800(this, missionID)` — lookup MissionEntry in internal map
2. If NOT found:
   - Call `FUN_00d1e030` — allocate new MissionEntry
   - Write `status` and `receivedBy` fields on new entry
3. If found:
   - Write `missionGiverName` → entry+0x60
   - Write `status` → entry+0x64 and entry+0x100
   - If `status == 1`: call `FUN_00d17c10(entry)` — activate
   - Else: call `FUN_00d16dd0(this, missionID)` — propagate state update
4. UI events:
   - `status == 0`: `FUN_00d163e0(0x138F)` — "Mission Removed/Failed" toast
   - `status == 1`: `FUN_00d163e0(0x1393)` — "Mission Accepted" toast

**Key functions called**:
- `FUN_00d16800` — `MissionSet_FindMissionById` (lookup by INT32 ID in map)
- `FUN_00d1e030` — `MissionSet_AllocateMissionEntry` (new MissionEntry allocation)
- `FUN_00d17c10` — `MissionSet_ActivateMission` (sets up active step, fires first step activation)
- `FUN_00d16dd0` — `MissionSet_PropagateMissionUpdate` (cascades status change through step/objective/task hierarchy)
- `FUN_00d163e0` — `MissionSet_FireUiEvent` (fires token-indexed UI string)

### Transition Diagram (Mission-Level)

```
[Not in map]
     |
     | onMissionUpdate(Status=1)
     v
 AllocateMissionEntry --> ActivateMission --> onStepUpdate activates first step
     |
     | onMissionUpdate(Status=0)
     v
 PropagateMissionUpdate --> UI token 0x138F ("Removed")
     |
     | onMissionUpdate(Status=1) [re-accept after clear]
     v
 status=1 branch --> ActivateMission again (repeatable reset path — see §Repeatable Reset)
```

---

## Step Lifecycle State Machine

### onStepUpdate Handler (`FUN_00d18cf0`)

**Wire fields read**: `StepID` (INT32), `Status` (INT8)

**Algorithm**:
1. Iterate step list at `this+0x58` searching for matching StepID
2. Write `status` → step+0x3C
3. If `status == 1` and this is a newly activated step:
   - Display "Mission Advance: [step name]" toast
4. UI events:
   - `status == 1`: `FUN_00d163e0(0x1390)` — "Step Activated" toast

**States**:

| Status | Semantic |
|--------|----------|
| `0` | Inactive / not yet reached |
| `1` | Active (currently in progress) |
| `2` | Complete (inferred — no explicit branch confirmed) |

---

## Objective Lifecycle State Machine

### onObjectiveUpdate Handler (`FUN_00d18fd0`)

**Wire fields read**: `ObjectiveID` (INT32), `Status` (INT8), `Hidden` (INT8), `Optional` (INT8)

**Algorithm**:
1. Look up ObjectiveEntry by ObjectiveID
2. Write `status` → objective+0x30
3. Write `optional` → objective+0x34 (note: `Hidden` field is read but no confirmed write offset for hidden flag)
4. UI display logic:
   - `status == 0`: show "Objective Removed: [name]" (hidden/optional variants exist)
   - `status == 1`: show "Objective Unlocked: [name]"
   - `status == 2` (inferred): show "Objective Complete: [name]"
5. UI events:
   - `status == 0`: `FUN_00d163e0(0x1392)` — "Objective Removed" token
   - `status == 1`: `FUN_00d163e0(0x1391)` — "Objective Unlocked" token

**Open**: No confirmed write offset for the `Hidden` flag field. The byte is read from the wire but the target struct offset was not resolved in decompilation.

---

## Task Lifecycle State Machine

### onTaskUpdate Handler (`FUN_00d194b0`)

**Wire fields read**: `TaskID` (INT32), `Status` (INT8), `Count` (INT32)

**Algorithm**:
1. Iterate task list at `this+0x70` searching for matching TaskID
2. Write `status` → task[2]
3. Write `count` → task[3]
4. Call `FUN_00d16dd0(this, taskID)` — propagate task update upstream

**No UI token fired directly by the task update handler.** UI updates for task progress are driven by the propagation chain in `FUN_00d16dd0`, not the task handler itself.

### Task Primitive Types — Not Present in Binary

Task primitive type names (`KillCount`, `CollectItem`, `VisitRegion`, `TalkToNpc`, `UseObject`, `Timer`) do not appear as strings in SGW.exe. These identifiers exist in server-side Python/PAK data files only. The client receives only `TaskID` + `Status` + `Count` and tracks progress numerically without knowing the underlying primitive type.

**Implication for Cimmeria**: The server must record task primitive type during mission definition loading. The client needs none of this information — it will correctly display progress bars and completion states from `Count` and `Status` alone.

---

## Timer Subsystem

### TimerUpdate Handler (`FUN_00d18a30`)

This handler manages step timers, mission timers, and countdown displays.

**Wire fields read**: `timerType` (INT8), `duration` (UINT32), `BigWorldTimeComplete` (bool via `FUN_00e3cc20`)

**Routing by timer type**:

| Type Byte | Semantic | Handler Action |
|-----------|----------|----------------|
| `0x09` | Countdown display timer | Calls `FUN_00c6d1c0(this+0x7C, ...)` to start countdown UI |
| `0x0A` | Step-scoped timer | Searches `this+0x58` (step list) for the matching step; calls `FUN_00c6d1c0` then `FUN_00d16dd0` |
| `0x0B` | Mission-scoped timer | Searches `this+0x64` (mission list) for the matching mission; calls `FUN_00c6d1c0` then `FUN_00d16dd0` |

`FUN_00c6d1c0` — timer state update function operating on `this+0x7C` (the MissionSet timer state slot).
`FUN_00d16dd0` — `MissionSet_PropagateMissionUpdate` — fires after timer update to cascade state to UI.

**Open**: The `BigWorldTimeComplete` bool field semantics are not fully recovered. Hypothesis: when `true`, it signals the timer has expired and the server is simultaneously sending a step/mission failure. When `false`, it is a mid-timer progress update (e.g., "30 seconds remaining").

---

## State Propagation — `FUN_00d16dd0`

`MissionSet_PropagateMissionUpdate` is the central cascading function. It is called after any state change (task, objective, step, mission, or timer) and walks the hierarchy to:
1. Re-evaluate parent states when children complete
2. Fire UI update events
3. Potentially trigger mission-complete or step-complete conditions

Every handler that mutates state calls this function. It is the "reactions" engine of the client-side mission system.

---

## Reward Delivery Sequence

### onMissionRewards Handler (`FUN_00d1a500`)

**Wire fields read** (from Ghidra decompilation, confirms `mission-wire-formats.md` exactly):

```
aMissionId    — INT32
Rewards {
  XP          — UINT32
  Naquadah    — UINT32
  ItemGroups  — ARRAY<ItemGroup> {
    GroupId     — UINT32
    NumChoices  — UINT32
    Items       — ARRAY<RewardItem> {
      Index     — UINT32
      ItemId    — UINT32
    }
  }
}
```

**Algorithm**:
1. Read all reward fields from the event object
2. Construct a `MissionRewards` object
3. Subscribe `MissionRewards` to `Cache_ElementReady<DBInvItem>` — waits for inventory item defs to load from cache
4. Once items are ready: display reward UI

**Open**: The complete sequence from `FUN_00d1a500` to reward display is not fully traced. The `Cache_ElementReady` subscription pattern means reward display is deferred until item definitions are available in the client cache. The inventory-side handling (actually granting items to the local player's inventory display) is a separate system.

---

## Mission Sharing Flow

### Outbound — shareMission (Client → Server)

The player initiates sharing via a slash command or UI button. The client emits:
- Wire method: `shareMission` (Missionary cell method)
- Wire: 1B header + 4B `MissionID` (INT32)

### Inbound — offerSharedMission (Server → Client, sharer's target)

Handler: `FUN_00d19870` — `MissionSet_HandleMissionSharedOffer`

**Algorithm**:
1. Read `MissionId` (INT32) from event
2. Display: "Shared Mission Offer: use /sharemissionaccept or /sharemissiondecline"

**Note**: The client displays command hints rather than a GUI dialog. Player types `/sharemissionaccept` or `/sharemissiondecline` to respond.

### Outbound — shareMissionResponse (Client → Server, after player command)

Wire method: `shareMissionResponse` (Missionary cell method)
Wire: 1B header + 1B `Choice` (non-zero = accept)

### Inbound — MissionOffer (Server → Client, after server assigns shared mission)

Handler: `FUN_00d19a40` — `MissionSet_HandleMissionOffer`

**Algorithm**:
1. Read `MissionID` (INT32) from `event+0xC` (direct offset access, not named field lookup)
2. Construct outbound `NetworkEvent` (Pattern B ctor: 0xC-byte `scalable_malloc`, stamp vtable)
3. Call `CmeEventSignal_SetFieldHelper` (`0x00cb1f00`) to set `"MissionID"` field
4. Dispatch via `FUN_00d1d270`
5. Zero `this+0x9C`, `+0xA0`, `+0xA4`, `+0xA8` — clear mission load state flags

The zeroing of load state flags in step 5 is significant: it resets whatever partial-load state was pending before the offer arrived. This prevents stale data from contaminating the incoming shared mission's load.

**Confirmed full sharing flow**:
```
Player A: /sharemission → shareMission wire → Server
Server → offerSharedMission wire → Player B
Player B: /sharemissionaccept → shareMissionResponse(Choice=1) wire → Server
Server: missionAssign → onMissionUpdate(Status=1) → Player B's client
Player B's client: MissionOffer handler → load state reset → mission appears in journal
```

---

## Repeatable Mission Reset

### Hypothesis (not yet confirmed by decompilation)

No explicit "reset" wire message has been found. The repeatable reset mechanism is inferred to work as follows:

1. Server sends `onMissionUpdate(MissionID, Status=0)` — client fires token `0x138F` ("Removed"), propagates Status=0 through hierarchy
2. Server sends `onMissionUpdate(MissionID, Status=1)` — client finds existing MissionEntry (now with Status=0), enters the "found" branch, writes Status=1, calls `MissionSet_ActivateMission`
3. `MissionSet_ActivateMission` re-initializes the first step, which causes `onStepUpdate` to fire for the first step

**Evidence supporting this hypothesis**:
- The "found" branch in `FUN_00d1a270` handles Status=1 explicitly and calls `FUN_00d17c10` (ActivateMission)
- There is no branch that checks "was this mission already completed?" before calling ActivateMission
- `FUN_00d16800` finds missions by ID regardless of their current status

**What would confirm/deny**: Decompiling `FUN_00d17c10` (ActivateMission) to see if it reinitializes child step/objective/task structures or only sets up from a clean slate.

---

## UI Token Table (Confirmed)

Tokens passed to `MissionSet_FireUiEvent` (`FUN_00d163e0`):

| Token | Decimal | Semantic | Fired By |
|-------|---------|----------|---------|
| `0x138F` | 5007 | Mission Removed/Failed | onMissionUpdate, Status==0 |
| `0x1390` | 5008 | Step Activated | onStepUpdate, Status==1 |
| `0x1391` | 5009 | Objective Unlocked | onObjectiveUpdate, Status==1 |
| `0x1392` | 5010 | Objective Removed | onObjectiveUpdate, Status==0 |
| `0x1393` | 5011 | Mission Accepted | onMissionUpdate, Status==1 |

Token values are string table IDs. `FUN_00d163e0` performs a lookup in the localization string table and fires the resulting string as a UI toast/notification.

---

## Cross-Reference: Content Engine Actions

The following Rust content-engine actions (from `docs/content/content-engine.md`) correspond to server-side triggers that produce the client-side state transitions documented above:

| Content Engine Action | Server Wire Message Produced | Client Handler |
|----------------------|------------------------------|----------------|
| `AcceptMission` | `onMissionUpdate(Status=1)` | `FUN_00d1a270` — allocate + activate |
| `AdvanceMission` | `onMissionUpdate(Status=1)` for new step context | `FUN_00d1a270` — ActivateMission branch |
| `CompleteMission` | `onMissionUpdate(Status=2)` + `onMissionRewards(...)` | `FUN_00d1a270` + `FUN_00d1a500` |
| `AbandonMission` | `onMissionUpdate(Status=0)` | `FUN_00d1a270` — propagate + token 0x138F |
| `AdvanceStep` | `onStepUpdate(Status=1)` for new step + `onStepUpdate(Status=2)` for old | `FUN_00d18cf0` |
| `FailObjective` | `onObjectiveUpdate(Status=0)` | `FUN_00d18fd0` — token 0x1392 |
| `CompleteObjective` | `onObjectiveUpdate(Status=2)` | `FUN_00d18fd0` |
| `IncrementCounter` | `onTaskUpdate(Count=N, Status=...)` | `FUN_00d194b0` |

**Note**: Status value `2` (Complete) for mission/step/objective is inferred from the pattern — the binary confirms Status==0 and Status==1 branches explicitly, and the wire format docs confirm Status is an INT8 field. A Status==2 "complete" semantic is the only coherent state left.

---

## Function Inventory — Mission State Machine

All functions identified and their documentation status:

| Address | Name (recovered) | Role | V5 Status |
|---------|-----------------|------|-----------|
| `0x00d1a270` | `MissionSet_HandleOnMissionUpdate` | Main mission update handler | Rename pending |
| `0x00d18cf0` | `MissionSet_HandleOnStepUpdate` | Step update handler | Rename pending |
| `0x00d18fd0` | `MissionSet_HandleOnObjectiveUpdate` | Objective update handler | Rename pending |
| `0x00d194b0` | `MissionSet_HandleOnTaskUpdate` | Task update handler | Rename pending |
| `0x00d1a500` | `MissionSet_HandleMissionRewards` | Reward delivery handler | Rename pending |
| `0x00d19a40` | `MissionSet_HandleMissionOffer` | Mission offer (sharing target) handler | Rename pending |
| `0x00d19870` | `MissionSet_HandleMissionSharedOffer` | Shared mission offer display | Rename pending |
| `0x00d18a30` | `MissionSet_HandleTimerUpdate` | Timer type routing (0x09/0x0A/0x0B) | Rename pending |
| `0x00d163e0` | `MissionSet_FireUiEvent` | Token-indexed UI toast dispatch | Rename pending |
| `0x00d16800` | `MissionSet_FindMissionById` | Mission lookup by INT32 ID | Rename pending |
| `0x00d16dd0` | `MissionSet_PropagateMissionUpdate` | State cascade (task→obj→step→mission) | Rename pending |
| `0x00d17c10` | `MissionSet_ActivateMission` | First-step activation on mission accept | Rename pending |
| `0x00d1e030` | `MissionSet_AllocateMissionEntry` | New MissionEntry allocation | Rename pending |
| `0x00daa680` | `MissionSet_SubscribeOnMissionUpdate` | Startup registration for onMissionUpdate | Rename pending |
| `0x00d9fc00` | `MissionSet_onMissionUpdate_Subscriber` | Event factory wrapper (already named in Ghidra) | Named |
| `0x00db3390` | `RegisterBulkNetOutSignals` | Bulk startup registration (all CME events) | Named |
| `0x00c6d1c0` | (unknown) | Timer state update (`this+0x7C`) | Not yet renamed |
| `0x00d1d270` | (unknown) | MissionOffer dispatch (called from `FUN_00d19a40`) | Not yet renamed |
| `0x00d205c0` | (unknown) | Mission data loading (triggered when `local_50 > 0`) | Not yet investigated |
| `0x00e3cba0` | (named elsewhere) | CME event INT32 field reader | Named |
| `0x00d434d0` | (named elsewhere) | CME event INT8 field reader | Named |
| `0x00e3cc20` | (named elsewhere) | CME event pointer/bool field reader | Named |
| `0x00cb1f00` | `CmeEventSignal_SetFieldHelper` | CME event field setter (outbound) | Named |

---

## Open Questions

1. **Repeatable mission reset confirmation**: What does `MissionSet_ActivateMission` (`0x00d17c10`) do with pre-existing child entries? Does it clear and reinitialize, or does it assert they don't exist? This determines whether repeatable missions work via Status=0 → Status=1 or via a separate wire message.

2. **MissionEntry `+0x64` vs `+0x100` status fields**: Two different offsets receive the same `Status` byte in `FUN_00d1a270`. These may belong to two different structs that overlap in the decompiler's view, or they may genuinely be a primary + mirror field.

3. **Hidden flag write offset**: `ObjectiveUpdate` reads a `Hidden` byte from the wire but no confirmed write offset in the struct was found. The hidden flag likely gates UI display — finding its offset would complete the ObjectiveEntry layout.

4. **`FUN_00d205c0` role**: This function is triggered in `FUN_00d1a270` when `local_50 > 0` (a count or flag). Given its address proximity to the mission handlers, it likely handles async PAK/definition loading for newly received missions. Needs decompilation.

5. **ChosenRewards → server flow**: The client emits `chosenRewards` (via `SGWPlayer.def` base method) but the SGWNetworkManager handler for this outbound event has not been traced. The path from UI reward selection → wire emit → server receipt is undocumented on the client side.

6. **`BigWorldTimeComplete` semantics**: The bool field in the TimerUpdate handler — does `true` mean "timer expired" or "BigWorld server clock agrees this step is complete"? The distinction matters for how Cimmeria should emit timer events.

7. **`FUN_00c6d1c0` role**: Called with `(this+0x7C, duration, BigWorldTimeComplete)` in the timer handler. Is this a countdown timer UI object, a server-time-sync mechanism, or a pure duration tracker?

---

## Recommended Rust Fixes (for Orchestrator)

None identified that affect correctness of current implementation. The findings confirm that:
- Wire formats in `mission-wire-formats.md` are accurate
- Content-engine action → wire message mapping in `content-engine.md` is correct
- No client-authoritative mission logic needs to be replicated server-side

One area to verify: the `IncrementCounter` action should emit `onTaskUpdate` with the new `Count` value. If the current Rust implementation sends `Count` as an INT8 (as specified in the `MissionTaskStatus` FIXED_DICT in `alias.xml`) rather than INT32 (as the `onTaskUpdate` handler reads it), there is a wire-format mismatch. The handler at `FUN_00d194b0` reads `Count` via `FUN_00e3cba0` (INT32 reader), but `alias.xml` defines `MissionTaskStatus.count` as INT8. Recommend verifying which encoding the current Rust serializer uses against the handler's reader function.
