---
name: mission-persist-hydrate-roundtrip
description: How per-objective mission state round-trips through sgw_mission (H50/#657) — the def-driven hydration rule, the no-op persist gate, and the self-agreeing-test trap that let the defect survive
metadata:
  type: project
---

Mission progress round-trips through exactly one serializer and one
hydrator. Read both before touching either side.

- **Serialize:** `cell/missions/persist.rs::mission_update_msg`, called
  via `send_mission_update(entity_id, player_id, mission_id, site, tx,
  space_mgr)` **after** the mutation, never before. Every executor arm in
  `cell/content/executor/mission.rs` goes through it. Do not hand-roll a
  `CellToBaseMsg::MissionUpdate` in a new arm.
- **Hydrate:** `cell/service/base_messages/player_init/mission_restore.rs::build_restored_missions`.

**Array semantics (both sides must agree):** `active_objective_ids` is
the *current step's whole roster*, completed entries included.
`completed_objective_ids` is the union of `completed_objectives` and any
`active_objectives` entry already flipped to `STATUS_COMPLETED`, deduped.
`failed_objective_ids` is always empty — nothing tracks per-objective
failure.

**The roster is rebuilt from `resources.mission_objectives`, not from the
saved array.** The saved arrays are only a status overlay (an id in
`completed_objective_ids` comes back `STATUS_COMPLETED`). Three reasons,
all load-bearing:

1. `hidden` / `optional` have no DB column and no runtime reveal path —
   only `advance_step` and `accept_or_advance` ever set them, both from
   `MissionObjectiveDef`. The seed is the single source of truth.
2. It self-heals rows written before #657, which hold the **step id** in
   the objective array. Hydrating that verbatim yields a required,
   `STATUS_ACTIVE` pseudo-objective nothing can match, so
   `all_required_complete` is false forever and the mission bricks.
   **The repo does no DB migrations — hydration is the only lever.**
3. Fallback when the step isn't in the `step_objectives` cache: saved
   array verbatim with `(false, false)`, and it warns.

**`complete_objective` returns `bool` and the executor skips the persist
on `false`.** This is test integrity, not tidiness: if a no-op persisted,
a dead executor arm and a live one produce identical DB traffic and no
regression guard can tell them apart.

## The trap that let #657 live

The pinned defect test hand-built the `MissionInstance` it claimed
`player_init` produced, so it agreed with itself and could never fail
when production changed. **A persistence guard must build its
`SavedMission` from the `MissionUpdate` the executor actually emitted,
and load `mission_defs` / `step_objectives` from the seeded DB** — not
from hand-written `MissionObjectiveDef`s. Worked pattern:
`cell/content/chain_replay_tests/mission_relog_persistence.rs`
(`run_action` → `saved_from_wire` → `relog` → `post_relog_context`).
Same trap as [[vacuous-guard-and-sentinel-collision-review]].

Note `cell::service::base_messages` and `::player_init` are `pub(crate)`
specifically so those guards can call the real hydrator.

## Gotchas

- `populate_mission_context` has **two** loops — `active_objectives` then
  `completed_objectives` (skipping ids already in the first). A
  prior-step objective reaches `objective_status … eq completed` only
  through the second. It needed no change for #657.
- `onMissionUpdate`'s wire byte is the two-value `STATUS_*` enum, never
  `MISSION_*`. See [[content-chain-condition-context-gaps]].
- `resend_missions(entity_id, tx, space_mgr)` is the client-facing
  mission-log burst and takes nothing from `player_init`. It is the seam
  a respawn re-send would call (playtest finding H8). Keep it that way.
