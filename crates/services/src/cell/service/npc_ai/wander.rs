//! Wander state: random walk within `wander_radius` of spawn, with
//! random dwell pauses between hops.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::record_decision_outcome;

/// NPC wander behavior: pick a random point within `wander_radius` of
/// `spawn_position`, walk there via the navmesh, pause for a random
/// dwell drawn from `[wander_min_dwell_secs, wander_max_dwell_secs]`,
/// repeat.
///
/// State machine within Wander:
/// - **nav_path empty + no/elapsed dwell** → sample a fresh waypoint,
///   pathfind, queue into `nav_path`, stamp `wander_next_at`. RNG is
///   seeded from `(npc_id, current_dwell_deadline_nanos)` so the
///   sample is reproducible per-tick for tests but varies across
///   real-world ticks.
/// - **nav_path empty + future dwell deadline** → no-op (pausing).
/// - **nav_path non-empty** → movement-tick is walking, no work.
///
/// Off-mesh rejection: when `space_mgr.is_position_valid` fails for
/// the sampled point, the handler falls back to `spawn_position` so
/// the NPC heads home rather than walking through a wall. This is
/// deliberately simple — a future "re-sample N times before giving
/// up" would be a small refinement.
pub(super) async fn npc_ai_wander(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::MobMovementType;
    use rand::{RngExt, SeedableRng};

    let now = std::time::Instant::now();

    // Read wander config + current position via an immutable borrow.
    let (spawn_pos, npc_pos, radius, min_dwell, max_dwell, nav_empty, wander_next) =
        match space_mgr.get_entity(npc_id) {
            Some(e) => (
                e.spawn_position,
                e.position,
                e.wander_radius,
                e.wander_min_dwell_secs,
                e.wander_max_dwell_secs,
                e.nav_path.is_empty(),
                e.wander_next_at,
            ),
            None => return,
        };

    // Zero-radius drop fires BEFORE the Wander broadcast so the wire
    // doesn't see a Wander byte for an NPC that's about to leave
    // Wander this same tick.
    if radius <= 0.0 {
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_state = cimmeria_entity::cell_entity::AiState::Idle;
        }
        crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    }

    crate::cell::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::Wander),
        tx,
        space_mgr,
    )
    .await;

    if !nav_empty {
        // Movement tick is walking — nothing to do.
        record_decision_outcome("wander_dwell");
        return;
    }

    // Arrival-based dwell semantics (matches Patrol + Investigating
    // and the column comment on `wander_min/max_dwell_secs`):
    //
    // - `wander_next_at` None + nav_empty → just arrived (or first
    //   entry). Sample a dwell duration and stamp the deadline.
    // - `wander_next_at` Some(future) + nav_empty → dwelling at the
    //   destination, no-op.
    // - `wander_next_at` Some(elapsed) + nav_empty → dwell complete,
    //   pick a fresh destination, route, clear the deadline so the
    //   next arrival re-stamps.
    //
    // The "first entry" case stamps dwell at spawn_position: the NPC
    // pauses at spawn for [min, max] seconds before its first hop.
    // Acceptable initial behavior; alternative would be to sample
    // immediately and skip the dwell-at-spawn beat.
    match wander_next {
        Some(deadline) if now < deadline => {
            // Dwelling at the current destination.
            record_decision_outcome("wander_dwell");
            return;
        }
        None => {
            // Just arrived (or first call). Stamp dwell.
            record_decision_outcome("wander_dwell");
            //
            // Seed the per-call RNG from a wall-clock source for real
            // entropy (`Instant::elapsed()` from a same-function-frame
            // `now` would give only nanosecond jitter). The
            // multiplicative golden-ratio constant per `npc_id` ensures
            // two NPCs sampling on the same nanosecond get different
            // dwell durations.
            let wall_nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            let seed = u64::from(npc_id).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ wall_nanos;
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
            let dwell_secs = rng
                .random_range(min_dwell..=max_dwell.max(min_dwell))
                .max(0.5);
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.wander_next_at = Some(now + std::time::Duration::from_secs_f32(dwell_secs));
            }
            tracing::debug!(
                target: "npc_ai",
                event = "wander_arrived",
                npc_id,
                dwell_secs,
                "NPC AI: wander → arrived, dwelling"
            );
            return;
        }
        Some(_) => {
            // Dwell elapsed → fall through to "pick a fresh destination".
        }
    }

    let Some(spawn) = spawn_pos else {
        // No spawn anchor — wander can't pick a destination. Drop to Idle.
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_state = cimmeria_entity::cell_entity::AiState::Idle;
        }
        return;
    };

    let wall_nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let seed = u64::from(npc_id).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ wall_nanos;
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
    let angle = rng.random_range(0.0..(std::f32::consts::TAU));
    let distance = rng.random_range(0.0..radius);
    let _ = (min_dwell, max_dwell); // sampled in the arrival branch above
    let candidate = cimmeria_common::Vector3::new(
        spawn.x + angle.cos() * distance,
        spawn.y,
        spawn.z + angle.sin() * distance,
    );

    // Off-mesh rejection: fall back to spawn_position. A single
    // re-sample would be a nicer behavior but adds complexity for
    // marginal benefit at this stage.
    let target = if space_mgr.is_position_valid(npc_id, &candidate) {
        candidate
    } else {
        spawn
    };

    let path = space_mgr
        .find_path(npc_id, &npc_pos, &target)
        .unwrap_or_default();
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        // Clear the dwell deadline now that we're starting the next
        // hop. The next arrival (`nav_empty` again) will see
        // `wander_next_at = None` and re-stamp from the arrival
        // branch above.
        npc.wander_next_at = None;
        npc.nav_path.clear();
        if path.len() > 1 {
            for wp in path.into_iter().skip(1) {
                npc.nav_path.push_back(wp);
            }
        } else {
            npc.nav_path.push_back(target);
        }
    }
    // Picked a fresh waypoint and queued the path — the next tick
    // observes `nav_empty = false` and records wander_dwell while the
    // movement plays out.
    record_decision_outcome("wander_pick");
    tracing::debug!(
        target: "npc_ai",
        event = "wander_waypoint_set",
        npc_id,
        target_x = target.x,
        target_z = target.z,
        radius,
        "NPC AI: wander → fresh waypoint queued"
    );
}
