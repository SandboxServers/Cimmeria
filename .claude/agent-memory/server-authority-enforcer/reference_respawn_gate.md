---
name: reference-respawn-gate
description: callForAid/respawn (67/70) dead gate + offered-respawner check live in the dispatch arms, not handle_respawn; offered_in_world is the single predicate
metadata:
  type: reference
---

Fixed 2026-09-28 (#799, CAT-C-01/02).

- Gate: `respawn_refusal` in `crates/cell-methods/src/cell/cell_methods/player/combat/mod.rs`, called by the `CALL_FOR_AID` and `RESPAWN` arms before `handle_respawn` and before journal/friction rows.
- Authority: `BSF_DEAD` on `state_field` (not HP). Offered set: `cimmeria_cell_catalog::cell::spawner::offered_in_world` (world_name filter), shared with `send_begin_aid_wait` in cell-combat `death/side_effects.rs`. Ids `<= 0` always accepted (server world default / synthetic "Respawn Point").
- Deliberately NOT inside `handle_respawn` (cell-interactions `respawn/mod.rs`): GM `gmRespawn` calls it on a living GM, and `resolve_respawn_target` still does a global id lookup. Any NEW caller of `handle_respawn` with a client-supplied id must re-apply the gate.
- Refusals log DEBUG (client-controlled input) on target `player.respawn`, reasons `respawn_not_dead` / `respawner_not_offered`.
- Open: when `unstuck` lands, the dead gate must widen to "dead OR server-recorded pending unstuck aid-wait". #233 should narrow `offered_in_world`, not add a second filter. CAT-C-14 (focus refill) is now dead-only.
- Guards: `combat/tests/respawn_gate.rs` (verified red with each gate reverted).

Related: [[reference-combat-exploit-classes]]
