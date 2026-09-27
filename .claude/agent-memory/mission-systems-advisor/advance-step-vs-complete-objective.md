---
name: advance-step-vs-complete-objective
description: advance_step force-completes the current step's objectives AND (since ~2026-09-19) sends their COMPLETED ticks with real flags; complete_objective can auto-complete the mission; H50 persistence fixed. Turn-in objective shape.
metadata:
  type: project
---

# `advance_step` vs `complete_objective` (re-verified 2026-09-26)

`crates/cell-content/src/cell/missions/progression.rs`.

## `advance_step(entity, mission, new_step)`

1. Force-completes every open objective of the old step and sends
   `onObjectiveUpdate(oid, COMPLETED, hidden, optional)` for each, BEFORE the
   step frames. (The 2026-09-18 version of this note said it sent no ticks —
   that was fixed; do not re-report it.)
2. Pushes old step to `completed_steps`, loads new step objectives.
3. Does NOT run the all-required-done auto-complete, which is why chains use
   it to leave a step whose last required objective would otherwise end the
   mission (688 chain 1107 → 80688; 622 → 80622).

## `complete_objective`

Flips one objective and auto-completes the whole mission when every
non-optional `active_objectives` entry is COMPLETED (vacuous on an
all-optional step). Completing an OPTIONAL objective while a required one is
open is safe (688/4647, flank 2725/2731).

## `complete_mission` (turn-in shape)

`complete_mission_direct` force-completes the final step's objectives. This
is the canonical Atrea shape — `missions.complete(622)` in ArmYourself.py
closes step objective 90622 without ever completing it. Most Cellblock
final-step objectives (90622, 4444, 2716, 2463, 5209, 4118, 2724, 2726-2730,
2733, 90688) have NO `complete_objective` anywhere; only CompleteMission can
close them. `ChainEngine::has_objective_completer(m, o)` tells the two apart;
the `objective_never_completed` friction signal reports only objectives that
have a completer.

A finished mission keeps its final step's objectives in `active_objectives`
(status COMPLETED) and is persisted with `current_step_id = NULL` — expected,
not a data gap (restore only WARNs for an ACTIVE mission on an unseeded step).

## Persistence

H50 landed: `MissionUpdate` carries real objective ids, completed objectives
and completed steps. `objective_status … eq completed` and
`step_status … eq completed` survive a relog.
