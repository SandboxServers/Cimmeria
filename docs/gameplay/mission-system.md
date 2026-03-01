# Mission System

> **Last updated**: 2026-03-01
> **Status**: ~40% implemented

## Overview

Missions (quests) are the primary PvE progression mechanism. Each mission contains a linear sequence of steps, and each step contains one or more objectives. Objectives can be hidden or optional. Missions support acceptance, advancement, completion, failure, abandonment, repeating, and sharing. Dynamic Python scripts can be attached to missions for custom behavior.

The `MissionManager` and `MissionInstance` classes in `python/cell/MissionManager.py` implement mission lifecycle logic. Mission definitions are loaded from `db/resources.sql` via `DefMgr`.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| Mission accept | DONE | `MissionInstance.accept()` with step/objective init |
| Mission completion | DONE | `MissionInstance.complete()` with event firing |
| Mission failure | DONE | `MissionInstance.fail()` with canFail/canAbandon checks |
| Mission abandonment | DONE | Delegates to `fail()`, checks `isHidden`/`canAbandon` |
| Step advancement | DONE | `MissionInstance.advance()` with ordering validation |
| Objective completion/failure | DONE | Per-objective state tracking |
| Mission scripts | DONE | Dynamic loading from `cell.missions.<scriptName>` |
| Script hot-reload | DONE | `MissionInstance.reloadScript()` with persist/restore |
| DB persistence | DONE | `sgw_mission` table with step/objective arrays |
| Client sync (resend) | DONE | Full mission/step/objective resync on login |
| Mission loot tables | PARTIAL | `currentMissionLoot` property exists |
| Hidden missions | DONE | `isHidden` flag suppresses client notifications |
| Repeatable missions | DONE | `numRepeats`, `canRepeatOnFail` tracked |
| Mission sharing | STUB | `shareMission`, `shareMissionOffer` defined, not functional |
| Reward selection | PARTIAL | `displayRewards()` calls client but reward choice unhandled |
| Task tracking | NOT IMPL | `onTaskUpdate` client method exists, no task logic |
| Objective conditions | NOT IMPL | Pre-conditions from mission data not evaluated |

## Entity Definition (Missionary.def)

### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `missions` | PYTHON | CELL_PRIVATE | Active mission instances |
| `publicMissionData` | ARRAY\<PYTHON\> | CELL_PUBLIC | (missionId, currentStep) tuples for other cells |
| `completedMissions` | ARRAY\<PYTHON\> | CELL_PUBLIC | (missionId, completionCount) tuples |
| `currentMissionLoot` | PYTHON | CELL_PRIVATE | Mob loot tables for mission items |
| `bShowMissionDebug` | INT8 | CELL_PRIVATE | Debug output toggle |
| `missionProcessQueue` | PYTHON | CELL_PRIVATE | Queued ops for locked missions |
| `pendingMissionAccepts` | ARRAY\<INT32\> | CELL_PRIVATE | Pending accept queue |
| `pendingMissionShares` | PYTHON | CELL_PRIVATE | Pending share offers |

### Client Methods (Server -> Client)

| Method | Args | Purpose |
|--------|------|---------|
| `onMissionUpdate` | MissionID, Status, MissionGiverName | Mission state change |
| `onStepUpdate` | StepID, Status | Step state change |
| `onObjectiveUpdate` | ObjectiveID, Status, Hidden, Optional | Objective state change |
| `onTaskUpdate` | TaskID, Status, Count | Task progress (NOT IMPL) |
| `offerSharedMission` | MissionId | Shared mission offer prompt |

### Cell Methods (Client -> Server)

| Method | Exposed | Args | Purpose |
|--------|---------|------|---------|
| `abandonMission` | YES | MissionID | Player abandons mission |
| `shareMission` | YES | MissionID | Share mission with group |
| `shareMissionResponse` | YES | Choice | Accept/decline shared mission |
| `missionAssign` | NO | caller, DesignID, DisplayPopup | Server-side mission grant |
| `missionAdvance` | NO | caller, DesignID, StepToAdvanceTo | Server-side step advance |
| `missionComplete` | NO | caller, DesignID, TurnIn | Server-side completion |
| `missionClear` | NO | caller, DesignID | Remove mission data |
| `missionClearActive` | NO | caller | Clear all active missions |
| `missionClearHistory` | NO | caller | Clear completed history |
| `missionList` | NO | caller | Debug: list missions |
| `missionReset` | NO | caller, DesignID, StepToAdvanceTo | Debug: reset to step |

## Mission Instance Lifecycle

```
MissionManager.accept(missionId)
  |-> MissionInstance(missionId)
  |-> instance.accept()
       |-> status = MISSION_Active
       |-> updateCurrentStep(firstStep)
       |-> addStepObjectives(firstStep)
       |-> initScript()
       |-> fire("mission.accepted::<id>")
       |-> fire("mission_step.started::<stepId>")
       |-> fire("mission_objective.started::<objId>") for each

MissionManager.advance(missionId, stepId)
  |-> instance.advance(stepId)
       |-> completeStepObjectives(currentStep)
       |-> addStepObjectives(nextStep)
       |-> fire step completed/started events

MissionManager.complete(missionId)
  |-> instance.complete()
       |-> status = MISSION_Completed
       |-> repeats++
       |-> destroyScript()
       |-> client.onMissionUpdate(id, 1, 0)
```

## Mission Status Codes

| Code | Constant | Description |
|------|----------|-------------|
| 0 | `MISSION_Not_Active` | Not started or not visible |
| 1 | `MISSION_Active` | Currently in progress |
| 2 | `MISSION_Completed` | Successfully completed |
| 3 | `MISSION_Failed` | Failed or abandoned |

## Data References

- **Mission definitions**: 1,041 in `db/resources.sql`
- **Schema**: `Mission.xsd`
- **Mission scripts**: `python/cell/missions/` directory
- **Persistence**: `sgw_mission` table (player_id, mission_id, status, current_step_id, completed/active/failed objective arrays, repeats)

## RE Priorities

1. **Task system** - Understand task data structure and completion detection
2. **Reward selection** - Client-side reward choice dialog and server handling
3. **Mission sharing** - Group-based mission sharing protocol
4. **Mission conditions** - Pre-accept conditions from mission XML data
5. **Mission loot** - How `currentMissionLoot` integrates with mob drop tables

## Related Docs

- [inventory-system.md](inventory-system.md) - Mission items and rewards
- [combat-system.md](combat-system.md) - Kill-based mission objectives
