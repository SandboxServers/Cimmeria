---
name: project-crafting-induction-review-2026-09-27
description: CR06 induction engine + consume/grant tx review (craft/cr06-induction); lock-order deadlock vs trade, fail-open quantity<=0, stale world guard
metadata:
  type: project
---

Reviewed crates/base-session/src/base/crafting/{session,transaction}/ on 2026-09-27 (commits e755ef86, 3c6b5683).

Findings worth carrying forward:
- ConnectedClientState.world_name is set only in play_character.rs and cleared on log_off; gate travel keeps the same entity id and never updates it. Any "same world" guard built on world_name is dead across gate travel; the drop hook in handle_gate_travel is the only defense.
- InductionEnv::player_world checks player_entity_id but not active_player_id, so entity-id recycling to another character in the same world passes the guard (see [[exploit-entity-id-recycling]]).
- Craft tx locks sgw_player FOR UPDATE before pg_advisory_xact_lock(player, 1); trade swap takes advisory(player, INV_MAIN=1) then sgw_player FOR UPDATE -> lock-order cycle. See [[advisory-lock-namespaces]].
- consume_design treats quantity <= 0 as a no-op (fail-open); a verb that overflows client count * recipe qty gets free products.
- Named instances are only ownership/bag-checked; consumption is by design, so the named row need not be consumed and its type_id is not checked against the plan.

Status after the follow-up commit on the same branch (5948edda): the lock order now takes every advisory lock (player-wide key 0, then bags) before the player row; the completion guard checks active_player_id; a non-positive consume quantity refuses the transaction. Still open: world_name is stale across gate travel (the gate-travel drop is the only world-change defense), and named instances are not tied to the plan's designs (a verb's job).

**Why:** the verbs (craft/research/RE/alloy) plug in later and inherit these seams.
**How to apply:** when reviewing any verb PR, check it builds CraftTransaction with checked arithmetic, ties named_items to consume designs, and that the above fixes landed.
