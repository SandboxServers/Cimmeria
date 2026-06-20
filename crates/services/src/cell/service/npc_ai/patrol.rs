//! Patrol state: walk a waypoint loop with arrival-based dwell pauses.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::record_decision_outcome;

/// NPC patrol behavior: walk a waypoint loop with arrival-based
/// dwell pauses.
///
/// State machine within Patrol (in evaluation order):
/// 1. **`patrol_path` empty** → drop to `Idle` and clear the
///    movement-type cache. Reachable only if a content action wipes
///    the path mid-tick.
/// 2. **`nav_path` non-empty** → `npc_movement_tick` is walking the
///    NPC toward the current target waypoint; no work this tick.
/// 3. **`nav_path` empty + close to target + dwell `None`** → just
///    arrived (or first entry). Stamp `patrol_dwell_until = now +
///    delay_secs`.
/// 4. **`nav_path` empty + close to target + dwell in the future**
///    → still pausing at the waypoint; no-op.
/// 5. **`nav_path` empty + close to target + dwell elapsed** →
///    advance `patrol_next_index` modulo path length and clear the
///    dwell. The next tick observes `not close` against the new
///    target index and queues movement.
/// 6. **`nav_path` empty + NOT close to target** → pathfind to the
///    current target waypoint and push the result into `nav_path`.
///    Also clears `patrol_dwell_until` — leaving a `Some(past)` here
///    would cause the re-arrival from a knockback to skip the
///    dwell.
///
/// The "close" threshold is `< 1.0` world units. `npc_movement_tick`
/// snaps to a waypoint when `distance <= move_speed` (default 0.6),
/// so post-arrival position is exactly on the waypoint; the 1.0
/// slack absorbs floating-point round-trips and keeps the
/// comparison well under any meaningful patrol distance.
///
/// Threat preemption is handled outside: when `generate_threat`
/// flips the state from Patrol → Fighting, the next AI tick routes
/// through `npc_ai_fight`. On Fighting → Leashing → Idle, the
/// tick's Idle branch transitions back to Patrol and the saved
/// `patrol_next_index` resumes the route from where it left off
/// (no progress lost).
pub(super) async fn npc_ai_patrol(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::MobMovementType;

    let now = std::time::Instant::now();

    // Read patrol state without holding a borrow across the
    // pathfind / nav_path write below.
    let (path_empty, nav_empty, dwell, target_index, delay_secs, target_waypoint, npc_pos) = {
        let npc = match space_mgr.get_entity(npc_id) {
            Some(e) => e,
            None => return,
        };
        if npc.patrol_path.is_empty() {
            (
                true,
                true,
                None,
                0,
                0.0,
                None,
                cimmeria_common::Vector3::zero(),
            )
        } else {
            let next_idx = npc.patrol_next_index % npc.patrol_path.len();
            (
                false,
                npc.nav_path.is_empty(),
                npc.patrol_dwell_until,
                next_idx,
                npc.patrol_point_delay_secs,
                Some(npc.patrol_path[next_idx]),
                npc.position,
            )
        }
    };

    // Empty-path drop fires BEFORE the Patrol broadcast so the wire
    // doesn't see a Patrol byte for an NPC that's about to leave
    // the state. The drop also broadcasts None to clear the cache.
    if path_empty {
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_state = cimmeria_entity::cell_entity::AiState::Idle;
        }
        // decision_outcome left empty — empty-path is a transition
        // out of Patrol, not a Patrol outcome. The next AI tick's
        // Idle branch will record its own outcome.
        crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    }

    // Broadcast Patrol movement-type. Dedup'd against
    // `last_movement_type` — subsequent Patrol ticks are no-ops on
    // the wire (the cache stays Some(Patrol) until a state
    // transition clears it).
    crate::cell::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::Patrol),
        tx,
        space_mgr,
    )
    .await;

    if !nav_empty {
        // Movement in flight — npc_movement_tick is walking the NPC
        // toward the current waypoint. Nothing to do this tick.
        record_decision_outcome("patrol_continue");
        return;
    }

    // nav_path is empty. Are we at the target waypoint (arrived) or
    // never started (still need to queue movement)?
    //
    // The "close" threshold is 1.0 world units — `npc_movement_tick`
    // snaps to a waypoint when `dist <= move_speed` (default 0.6),
    // so the position will be exactly on the waypoint after arrival.
    // 1.0 gives a small slack for floating-point round-trips while
    // staying well under the smallest meaningful patrol distance.
    let Some(waypoint) = target_waypoint else {
        return;
    };
    let close = npc_pos.distance_to(&waypoint) < 1.0;

    if close {
        // At the waypoint. Dwell logic.
        // patrol_dwell covers all three dwell sub-states (just-arrived,
        // still-dwelling, advance-to-next) — all three are "paused at
        // a waypoint" from the outcome perspective. The per-transition
        // `event = "patrol_arrived"` breadcrumb (debug log below)
        // discriminates the sub-state for log queries.
        record_decision_outcome("patrol_dwell");
        match dwell {
            None => {
                // Just arrived — stamp the dwell deadline. Subsequent
                // ticks observe `Some(deadline)` and either keep
                // waiting or advance.
                if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                    let secs = delay_secs.max(0.5);
                    npc.patrol_dwell_until = Some(now + std::time::Duration::from_secs_f32(secs));
                }
                tracing::debug!(
                    target: "npc_ai",
                    event = "patrol_arrived",
                    npc_id,
                    target_index,
                    delay_secs,
                    "NPC AI: patrol → arrived, dwelling"
                );
            }
            Some(deadline) if now < deadline => {
                // Still dwelling — no-op.
            }
            Some(_) => {
                // Dwell elapsed — advance to next waypoint. Clear the
                // dwell deadline; the next tick will observe
                // `close = false` (because the target is now a
                // different waypoint) and queue movement.
                if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                    let len = npc.patrol_path.len();
                    npc.patrol_next_index = (npc.patrol_next_index + 1) % len;
                    npc.patrol_dwell_until = None;
                }
            }
        }
    } else {
        // Not at the target — pathfind and queue movement. Clearing
        // `patrol_dwell_until` here matters for the knockback case:
        // if the NPC dwelled at the waypoint, got pushed off, and is
        // now walking back, leaving `Some(past)` on the entity would
        // make the next arrival fall into the "elapsed → advance"
        // branch and skip the remainder of the dwell. Clearing means
        // the re-arrival re-stamps from scratch, which is the
        // expected "pause for delay_secs after arriving" semantic.
        let path = space_mgr
            .find_path(npc_id, &npc_pos, &waypoint)
            .unwrap_or_default();
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.patrol_dwell_until = None;
            npc.nav_path.clear();
            if path.len() > 1 {
                // Skip the first entry (start position). Detour returns
                // a straight-path that includes both endpoints.
                for wp in path.into_iter().skip(1) {
                    npc.nav_path.push_back(wp);
                }
            } else {
                // Pathfind failed or returned a single point — direct push.
                npc.nav_path.push_back(waypoint);
            }
        }
        // patrol_continue covers both "walking the current waypoint"
        // and "queueing the next waypoint". The dispatcher span
        // already carries npc_id + ai_state; the discriminator field
        // names the sub-state (patrol_waypoint_set).
        record_decision_outcome("patrol_continue");
        tracing::debug!(
            target: "npc_ai",
            event = "patrol_waypoint_set",
            npc_id,
            target_index,
            wp_x = waypoint.x,
            wp_y = waypoint.y,
            wp_z = waypoint.z,
            "NPC AI: patrol → next waypoint queued"
        );
    }
}
