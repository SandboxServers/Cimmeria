---
name: ring-fsm-departure-hooks
description: How player-departure cleanup reaches the ring FSM (the cancel_trade_on_disconnect seam), which destroy_entity call sites are departures vs the legitimate ring handoff, and three ring-FSM failure modes to block on sight
metadata:
  type: project
---

Found 2026-09-17 reviewing the H02 (Harset ring FSM timeouts) design against
`crates/services/src/cell/ring_transport/`.

**`SpaceManager::destroy_entity` is sync with no `tx`, and that is deliberate.**
The codebase's established answer for "a subsystem needs async cleanup when a
player departs" is NOT a deferred effect queue — it is an explicit
`...(entity_id, tx, space_mgr).await` hook placed immediately *before* the
`destroy_entity` / `disconnect_entity` call at each departure site. The living
example is `cell_methods::player::trade::cancel_trade_on_disconnect`, called at
`base_messages/lifecycle.rs` (handle_destroy_entity + handle_disconnect_entity)
and `gate_travel.rs`. `lifecycle.rs` carries the comment explaining why it lives
in two places. Copy this shape; do not add a queue.

**Player-departure `destroy_entity` sites (need the hook) vs the one that must not:**
departures are `base_messages/lifecycle.rs` (x2), `gate_travel.rs`,
`cell_methods/gm/travel.rs`, `content/executor/transport.rs`,
`cell_methods/player/combat/respawn.rs`. The exception is
`ring_transport/dispatch.rs` `Effect::TeleportCrossWorld`, which calls
`destroy_entity` **as part of a legitimate ring handoff** — hooking it would make
the destination ring treat its own arriving passenger as a dropped player.
Nothing on `CellEntity` distinguishes the two cases: by that point the player has
been `mem::take`n out of `send_players`, is not yet in `players_loaded`, and
`destination_ring_id` is set in both. The call site is the only discriminator —
which is the argument for per-site hooks over a central hook plus an
in-transit marker map.

**Three ring-FSM failure modes to block on sight:**

1. **Raw bitmask writes to `BSF_MOVEMENT_LOCK`.** The ring's `update_state_flag`
   (`ring_transport/wire_helpers.rs`) does `state_field |= / &= !flag` directly.
   `crates/entity/src/cell_entity/state_flags.rs` names BSF_MOVEMENT_LOCK as a
   ref-counted flag (death + stun are the other writers) and documents mixing the
   two patterns as "a real production hazard". Ring unlock can therefore free a
   corpse's death-lock, and ring lock leaves the bit stuck against a later
   `unset_state_flag`. Use `set_state_flag` / `unset_state_flag` and send
   `onStateFieldUpdate` only on the returned transition.

2. **SendWarmup has an unbounded stall the H-B3 audit did not list.** In
   `runtime.rs::run_one_deadline`, the warmup branch returns early when the
   destination region is not in `space_mgr.ring_regions`, *without clearing
   `timers.warmup_at`* — so it re-fires every tick forever and every player in
   `send_players` stays locked and hidden permanently. Note `ring_regions` and
   `ring_transporters` are separate maps and can disagree.

3. **Locked+hidden players can end up in no set at all.** `Effect::TeleportPlayer`
   early-returns (entity missing from space; cell→base send failed) *before*
   `mark_player_loaded`, and `warmup_timer_expired` has already `mem::take`n
   `send_players`. A destination-side abort that re-shows only
   `send_players ∪ players_loaded` will not recover them. Carry the source's
   player snapshot onto the destination (an `expected_players` set backing
   `num_remote_players`) so the abort set is complete and the forget path is a
   set removal rather than a count decrement that can underflow.

Invariant that survives all of this: nothing in the ring path may leave a player
hidden or movement-locked, and any reposition is
`BASEMSG_FORCED_POSITION` (`mercury/aoi.rs::build_forced_position`), never method
116 `onPlayerTeleport`. The same-world ring teleport already routes correctly
through `CellToBaseMsg::TeleportPlayer`; see [[authorized-teleport-paths]] and
[[facing-preservation-primitive]]. Arrival coords are still unvalidated against
the navmesh — see [[arrival-coordinate-offnavmesh]] and
[[harset-travel-ground-truth]].
