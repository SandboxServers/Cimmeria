---
name: objective-state-not-persisted
description: CRITICAL — per-objective mission state is never persisted; after relog/gate-travel, objective_status `eq active` and `eq completed` conditions for real objectives fail. Full evidence chain.
metadata:
  type: project
---

# Per-objective mission state does not survive a relog (confirmed 2026-09-18)

## The write side is hardcoded empty

`crates/services/src/cell/content/executor/mission.rs` sends
`CellToBaseMsg::MissionUpdate` from three places, and **all three** hardcode:

- accept (`:84-85`)  → `completed_objective_ids: vec![]`, `active_objective_ids: vec![step_id]`
- complete (`:172-173`) → both `vec![]`
- advance_step (`:241`) → `completed_objective_ids: vec![]`, `active_objective_ids: vec![step_id]`

Note `active_objective_ids: vec![step_id]` — it writes the **step id** into the
objective-id array. There is no code path anywhere that writes real objective ids.

`Action::CompleteObjective` (`executor/mission.rs:274-291`) sends **no
`MissionUpdate` at all**. It can auto-complete the mission in memory and emit
mission/step wire updates, but persists neither objective state nor a
`mission_completed` event.

## The read side faithfully restores the garbage

`crates/services/src/cell/service/base_messages/player_init/mod.rs:168-210`
rebuilds `active_objectives` **from `saved.active_objective_ids`**, with
`hidden: false, optional: false` hardcoded. So after a relog at step N the
player's `MissionInstance.active_objectives == [MissionObjective{ objective_id: N }]`
— a pseudo-objective whose id is the step id — and the real objective ids are in
neither `active_objectives` nor `completed_objectives`.

## Consequence for chain authoring

`populate_mission_context` (`cell/content/mission_context.rs:143-176`) only emits
`mission_{m}_obj_{o}_status` for ids in those two lists. Every real objective id
therefore falls through `Condition::ObjectiveStatus`'s
`unwrap_or("not_active")` (`conditions.rs:271`) after a relog.

- `objective_status <real id> eq active`   → false forever after relog
- `objective_status <real id> eq completed` → false forever after relog
- `complete_objective <real id>` → `MissionInstance::complete_objective` returns
  `false` (`crates/entity/src/missions.rs:101-113`) → services-level handler
  early-returns, no wire frame, no progress.

Any multi-objective step gated on `objective_status ... eq active` or
`objective_status ... eq completed` is hard-stuck after the player relogs,
gate-travels, or cross-world teleports. `neq` evaluates against the fallback
`not_active`. (`fire_player_loaded` fires on all three.)

`entity.counters` are also session-only — `player_init` never restores them — so
Condition::Counter` is not a workaround. `current_step_id` persists, so
`step_status ... eq active` is the durable mission predicate today.

## `completed_step_ids` is empty too

All three arms also hardcode `completed_step_ids: vec![]`, so
`step_status X eq completed` is dead after relog for the same reason.
`step_status X eq active` survives (derives from `current_step_id`).

## Design decisions settled during the H50 review (2026-09-18)

- **`sgw_mission.active_objective_ids` is NOT disjoint from
  `completed_objective_ids` in the Rust model.** Python
  (`MissionManager.py:392-393,428-429`) *removes* an objective from
  `activeObjectives` on completion; Rust keeps it in `active_objectives`
  with `status = STATUS_COMPLETED` *and* pushes to `completed_objectives`.
  Persist the Rust shape: `active_objective_ids` = the full tracked
  objective set of `current_step_id` (completed ones included).
  Decisive reason: `MissionManager::serialize_resend`
  (`crates/entity/src/missions.rs:198`) iterates `active_objectives` to
  emit `onObjectiveUpdate` — drop completed ones and the post-relog quest
  log loses those rows entirely instead of showing them ticked.
- **Legacy-row self-heal is mandatory.** Every live `sgw_mission` row
  written before the fix has `active_objective_ids = [step_id]`. Hydrating
  that verbatim creates a required+ACTIVE pseudo-objective that no
  `complete_objective` can match, so `all_required_complete`
  (`progression.rs:176`) is permanently false for that player+mission.
  Hydration must rebuild `active_objectives` from
  `space_mgr.get_step_objectives(saved.current_step_id)` and use the saved
  arrays only as a status overlay. (Repo rule: no migration scripts, so
  self-heal at hydration is the only option.)
- **`failed_objective_ids` stays `[]`.** `Action::FailObjective` exists in
  `content-engine/src/actions.rs:313` + `loader/action.rs:337` but has **no
  executor arm**, and `MissionInstance` has no failed-objective list
  (python's `fail()` moves activeObjectives→failedObjectives;
  `entity/src/missions.rs:91` does not). Nothing can populate it.
- **`hidden`/`optional` are definitional, never per-player.** Sole source is
  `resources.mission_objectives.is_hidden/is_optional` via the
  `step_objectives` cache. No schema column is warranted.
  `mission_defs[m].objectives` is the *same data for the first step only*
  (`spawner/missions.rs:84-113`) — redundant as a fallback.

## Auto-complete asymmetry (surfaces once persistence is fixed)

The auto-complete branch inside `cell::missions::complete_objective`
(`progression.rs:182-213`) calls `mission.complete()` — flips status to
COMPLETED and bumps `repeats` — but never fires `fire_mission_completed`.
The only caller of that dispatcher is `executor/mission.rs::complete`.
Once the objective path persists, the DB row reads `status=2` while the
`mission_completed` chains (auto-accept of the follow-on mission, rewards)
never ran, and the offer guard (`lifecycle.rs:79`) then refuses re-accept
at the cap → player permanently stuck. Fix in the executor arm using the
same `transitioned_from_active` snapshot shape as `complete`
(`executor/mission.rs:155-159`), where `engine` is already in scope.

## The fix (not yet done)

In `executor/mission.rs`, after each mutation read the `MissionInstance` back
and populate `active_objective_ids` / `completed_objective_ids` from
`m.active_objectives` / `m.completed_objectives`; add a `MissionUpdate` send to
the `complete_objective` arm. Also carry `hidden`/`optional` through hydration
(currently hardcoded `false`, which would make a restored optional objective
count toward `all_required_complete` in `progression.rs:176-180`).

Routing: `mission-systems-advisor` + `database-persistence` + `rust-gameserver-dev`.
Live-DB regression guard shape: accept → advance → complete one objective →
re-hydrate → assert the objective id is in `completed_objective_ids`.
