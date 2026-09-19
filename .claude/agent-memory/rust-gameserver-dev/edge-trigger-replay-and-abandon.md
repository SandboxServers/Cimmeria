---
name: edge-trigger-replay-and-abandon
description: How the content engine closes spent edge events (H52 enter_region replay, H54 mission_abandoned) — the mission-gate filter, the SpaceManager re-entrancy guard, and where ring/stargate routing actually lives.
metadata:
  type: project
---

Playtest finding H9 (2026-09-18 Castle) is a class, not a bug: an edge event
(`enter_region`, `player_loaded`, cover, abandon) that fires before the step
or state gating its chain becomes true is **spent**, and never re-fires.
Castle lost objective 2484 to it. Two engine packets closed two of the forms.

**H52 — `enter_region` replay.** When a mission step activates, every
client-hinted region of the player's world containing the player's
server-known position is re-fired. Lives in
`cell/content/event_dispatch/step_activation/`. Hooked at the four sites that
own the `ChainEngine` and have just activated a step: the executor's accept
and advance-step arms, and `gmMissionAssign` / `gmMissionAdvance` — **not**
inside `cell::missions::progression`, which has no engine and sits below
`cell::content` in the layering.

Three facts worth keeping:

- **Only mission-gated chains replay.** `Chain::is_mission_gated` is true when
  a chain carries `mission_status` / `step_status` / `objective_status` — the
  only conditions whose value the chain's own actions move, which is what
  makes a double delivery (replay, then the client's real hint) a no-op.
  `world` and `archetype` deliberately do not count. `resolve_event_filtered`
  is the engine hook; an ungated chain is logged `reason = "filtered_out"`.
- **Ring transport and `REGION_FLAG_STARGATE` passage are NOT inside
  `fire_enter_region`.** They are sequenced by the dispatch arm in
  `cell_methods::player::world`, *after* it. So anything that calls the
  content path directly structurally cannot start a ring trip or a gate
  crossing. Worth knowing before adding a second caller.
- **The re-entrancy guard lives on `SpaceManager`** (`step_region_replay`),
  because the recursion runs through `executor::execute_actions`, which
  cannot thread a depth parameter back. A thread-local would be wrong: the
  cell task is async and tokio may move it between worker threads at an
  `await`. The `&mut SpaceManager` every frame already holds is the exclusive
  token.

**H54 — `mission_abandoned`.** `Trigger::OnMissionAbandoned`, `event_key` =
mission id, no wildcard. `fire_mission_abandoned` populates context **after**
`abandon_mission` removes the instance, so a repaint chain carries the offer
chain's own `mission_status eq not_active` gate verbatim. Firing before the
mutation makes every seed chain fail closed — that is the whole contract.

Abandon paths (enumerated, do not assume three): the client cell method
`abandonMission` (Missionary index **52**), the `abandon_mission` chain
action, and `gmMissionClear` (110) / `gmMissionAbandon` (120), which share
one handler. The console `.missionfail` is **not** one — it sets
`MISSION_FAILED` and never removes. `abandon_mission` returns whether it
removed anything and every hook is gated on that.

**Seed rule for any repaint chain:** unbind before you rebind.
`interactions/dispatch/interact.rs` takes the first bound entry on a template
that has a dialog, so a `remove_dialog_set` ordered after the
`add_dialog_set` leaves the NPC handing out the old dialog.
`remove_dialog_set` on an empty slot is a safe no-op, so clear every
candidate bind — the step is gone by the time an abandon chain runs, so it
cannot tell which one was live.

Still open: the **cover** form of H9 (objective 2484) belongs to the Castle
Cellblock lane; the seam in `cell/missions/progression.rs` already computes
`cover_sets` for its diagnostic log, so the data is there.

Related: [[content-chain-dispatch-traps]],
[[content-chain-condition-context-gaps]].
