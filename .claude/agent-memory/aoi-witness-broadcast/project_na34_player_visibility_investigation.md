---
name: project-na34-player-visibility-investigation
description: NA34 (2026-09-25) shared-world player-visibility report — SigNoz found no failure post-#737, added a two-order regression test and aoi.introduce telemetry
metadata:
  type: project
---

Owner reported (2026-09-25) that two players in a shared world (Castle,
Harset) can't reliably see each other, guessing "first player sees second
but not vice versa." 30 days of SigNoz `cimmeria-server`/`cimmeria-trace`
logs show every player-to-player `aoi.entity_enter` pair since PR #737
(2026-09-19) introducing bidirectionally with `is_player=true` on both legs
and zero `aoi.player_ghost_incomplete` / `aoi.entered_no_witness_addr` /
`aoi.create_send_failed` occurrences — no telemetry at all covers the
window the report describes (last player-to-player activity in the dev
overlay is 2026-09-21). The one asymmetric `is_player=false` pair found
(space 65544, 2026-09-17) predates #737 and is Root Cause 2 from
[[reference_witness_fanout_helper]]'s cascade doc firing pre-fix, not a
live regression.

**Why:** `SpaceManager::compute_aoi_changes` (`cell/space_manager/aoi.rs`)
iterates every player in `space.players` symmetrically every 100 ms tick;
`CellEntity::is_introducible()` and the `entity_to_addr`
identity-stamp-at-`CreateEntity` path
(`cell/service/base_messages/lifecycle.rs`) are both synchronous with no
`.await` gap an AoI tick could race. No server-side bug reproduces a
*permanent* one-direction failure on current `main`.

**How to apply:** Added
`base::world_entry::cell_dispatch::tests_dispatch_arms::two_player_visibility::both_arrival_directions_deliver_the_observee_identity`
— two real sessions sharing one `connected`/`entity_to_addr` map (A ready,
B mid-load), both `EnteredAoI` directions in one tick, then B's
deferred-buffer flush. It passes on `main` and is revert-proven against
`player_ghost::compose_cascade_body`'s ghost branch. Also added
`target: "aoi.introduce"` DEBUG rows (`outcome=deferred_not_ready` in
`aoi_dispatch::entered_aoi`, `outcome=flushed_on_ready` in
`deferred_flush::dispatch_segment`) so the next real two-client session has
per-(witness_id, entity_id) hold-duration evidence this investigation
lacked. If a future report recurs, check `aoi.introduce` first before
re-deriving the SpaceManager symmetry argument above.
