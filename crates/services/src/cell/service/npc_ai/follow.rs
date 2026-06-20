//! Follow state: maintain a distance band to a target entity.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::record_decision_outcome;

/// NPC follow behavior: maintain a distance band to a target entity.
///
/// State machine within Follow:
/// - **No follow_target_id** → drop to Idle.
/// - **Target gone (entity removed)** → clear follow_target_id,
///   drop to Idle.
/// - **Target in band** (`min <= dist <= max`) → no work; stay put.
/// - **Target above max** → pathfind to a point one `min_distance`
///   short of the target so the NPC settles inside the band rather
///   than running all the way up to the target.
/// - **Target below min** → no work (NPCs don't back away).
pub(super) async fn npc_ai_follow(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::{AiState, MobMovementType};

    let (target_id, npc_pos, min_d, max_d, nav_empty) = match space_mgr.get_entity(npc_id) {
        Some(e) => (
            e.follow_target_id,
            e.position,
            e.follow_min_distance,
            e.follow_max_distance,
            e.nav_path.is_empty(),
        ),
        None => return,
    };

    // No-target / gone-target drops fire BEFORE the Follow broadcast
    // so the wire doesn't see a Follow byte for an NPC that's about
    // to leave Follow this same tick.
    let Some(target_id) = target_id else {
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.ai_state = AiState::Idle;
        }
        crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    };

    let Some(target_pos) = space_mgr.get_entity(target_id).map(|e| e.position) else {
        // Target despawned/disconnected. Clear and drop to Idle.
        if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
            npc.follow_target_id = None;
            npc.ai_state = AiState::Idle;
        }
        crate::cell::abilities::broadcast_movement_type(npc_id, None, tx, space_mgr).await;
        return;
    };

    crate::cell::abilities::broadcast_movement_type(
        npc_id,
        Some(MobMovementType::Follow),
        tx,
        space_mgr,
    )
    .await;

    let dist = npc_pos.distance_to(&target_pos);
    if dist < min_d {
        // Too close — hold position.
        record_decision_outcome("follow_band");
        return;
    }
    if dist <= max_d {
        // In band — hold position.
        record_decision_outcome("follow_band");
        return;
    }

    if !nav_empty {
        // Movement in flight toward the target.
        record_decision_outcome("follow_band");
        return;
    }

    // Out of band — pathfind to a point one min_distance short of
    // the target along the line between the NPC and the target.
    let dx = target_pos.x - npc_pos.x;
    let dy = target_pos.y - npc_pos.y;
    let dz = target_pos.z - npc_pos.z;
    let mag = (dx * dx + dy * dy + dz * dz).sqrt();
    let stop_distance = min_d.max(0.1);
    let scale = ((mag - stop_distance) / mag).max(0.0);
    let dest = cimmeria_common::Vector3::new(
        npc_pos.x + dx * scale,
        npc_pos.y + dy * scale,
        npc_pos.z + dz * scale,
    );
    let path = space_mgr
        .find_path(npc_id, &npc_pos, &dest)
        .unwrap_or_default();
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.nav_path.clear();
        if path.len() > 1 {
            for wp in path.into_iter().skip(1) {
                npc.nav_path.push_back(wp);
            }
        } else {
            npc.nav_path.push_back(dest);
        }
    }
    // Out-of-band pathfind queued — the next tick observes
    // nav_empty=false (movement in flight) and records follow_band
    // until back in range.
    record_decision_outcome("follow_band");
    tracing::debug!(
        target: "npc_ai",
        event = "follow_routed",
        npc_id,
        target_id,
        dist,
        max_d,
        "NPC AI: follow → pathfinding toward target"
    );
}
