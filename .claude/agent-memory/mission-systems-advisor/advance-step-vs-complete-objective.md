---
name: advance-step-vs-complete-objective
description: advance_step force-completes the current step's objectives server-side but never emits onObjectiveUpdate(COMPLETED) for them; complete_objective does. Also: MissionUpdate persists the STEP id in active_objective_ids.
metadata:
  type: project
---

# `advance_step` vs `complete_objective` (verified 2026-09-18)

`crates/services/src/cell/missions/progression.rs`.

## `advance_step(entity, mission, new_step)` does:

1. Collects every `active_objectives` entry with `status != STATUS_COMPLETED`
   and calls `mission.complete_objective(oid)` on each — sets `STATUS_COMPLETED`
   **and** pushes into `completed_objectives`. So **yes, it force-completes**.
2. Pushes the old `current_step_id` onto `completed_steps`, sets the new one,
   loads the new step's objectives from `space_mgr.get_step_objectives`.
3. Wire: `onStepUpdate(old, COMPLETED)`, `onStepUpdate(new, ACTIVE)`, and
   `onObjectiveUpdate(new_obj, ACTIVE)` per new objective.

**What it never sends: `onObjectiveUpdate(old_objective_id, COMPLETED)`.**
`complete_objective` *does* send that tick. `advance_step` completes the old
objectives and activates a new step; `complete_objective` can complete the whole
mission when it finishes the required active objectives. Every chain condition
reads server state — so **no chain logic is lost by never authoring
`complete_objective`**. The only exposure is cosmetic: if the
client's quest-log renders per-objective rows and does not clear them on
`onStepUpdate(COMPLETED)`, the prior step's line lingers un-ticked. UAT the
first step transition of any newly ported multi-step mission; if the line
lingers, fix `advance_step` to emit the ticks — do not paper over it by adding
`complete_objective` actions to the chain.

`complete_objective` also auto-completes the whole mission when every
non-optional `active_objectives` entry is COMPLETED.

`complete_mission` → `complete_mission_direct` completes all active objectives
*and* emits their COMPLETED ticks, then `MissionInstance::complete()`. Those
objective updates hardcode `hidden` and `optional` to `false`.

## Persistence gap (route to database-persistence)

Both `accept_or_advance` and `advance_step` in
`crates/services/src/cell/content/executor/mission.rs` send
`CellToBaseMsg::MissionUpdate` with:

- `active_objective_ids: vec![step_id]` — the **step** id in the objective list
- `completed_objective_ids: vec![]` — always empty
- `completed_step_ids: vec![]` — **also always empty, in all three arms**

So neither `completed_objectives` nor `completed_steps` is durably persisted
through the chain path.

**Correction (2026-09-18):** an earlier version of this note claimed
`step_status` was unaffected. Only half true. `step_status X eq active`
survives (derives from `current_step_id`, which *is* persisted);
`step_status X eq completed` does NOT — `populate_mission_context`'s
completed-step loop reads `mission.completed_steps`, hydrated from
`saved.completed_step_ids`, which the executor always writes as `[]`.

**Until H50 lands: do not author `objective_status ... eq completed` OR
`step_status ... eq completed` conditions that must survive a relog.**
