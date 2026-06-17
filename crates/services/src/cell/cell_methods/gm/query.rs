//! GM query handlers that report text back to the caller via the
//! single-recipient [`super::feedback`] channel — `gmUsers` and `testLOS`.
//!
//! These were deferred until a feedback channel existed; with
//! [`super::feedback::send_gm_feedback`] in place they land cell-side.

use tokio::sync::mpsc;

use super::feedback::send_gm_feedback;
use super::read_i32;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `gmUsers()` — list the players in the caller's space.
///
/// Scope note: this is **space-scoped**, not all-shard. The cell only knows the
/// entities in its own spaces; a true server-wide user list would need a
/// base-side round-trip to `online_players`. For a dev tool, "who's in my
/// space" is the useful and self-contained answer.
pub(super) async fn handle_users(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some(space_id) = space_mgr.get_entity_space_id(entity_id) else {
        send_gm_feedback(entity_id, "gmUsers: you are not in a space.", tx).await;
        return true;
    };

    let mut players: Vec<(u32, Option<i32>)> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&pid| space_mgr.get_entity_space_id(pid) == Some(space_id))
        .map(|pid| (pid, space_mgr.get_entity(pid).and_then(|e| e.player_id)))
        .collect();
    players.sort_by_key(|&(pid, _)| pid);

    let text = if players.is_empty() {
        "gmUsers: no players in your space.".to_string()
    } else {
        let list = players
            .iter()
            .map(|(pid, player_id)| match player_id {
                Some(p) => format!("{pid} (char {p})"),
                None => format!("{pid}"),
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("gmUsers ({} in space): {list}", players.len())
    };
    send_gm_feedback(entity_id, &text, tx).await;
    true
}

/// `testLOS(INT32 aSourceEntityID, INT32 aTargetEntityID)` — report whether the
/// navmesh has line-of-sight between two entities in the caller's space.
///
/// Reuses the canonical [`SpaceManager::has_line_of_sight`] primitive (the same
/// one the NPC AI uses), which resolves the space, projects to the navmesh, and
/// raycasts — returning `true` for clear LoS (and conservatively `true` when no
/// navmesh is loaded). Both ids are validated to be in the caller's space first
/// so a typo'd id reports "not found" rather than a misleading CLEAR.
pub(super) async fn handle_test_los(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let (Some(source), Some(target)) = (read_i32(args, 0), read_i32(args, 4)) else {
        send_gm_feedback(entity_id, "testLOS: need two INT32 entity ids.", tx).await;
        return true;
    };
    let (Ok(source_eid), Ok(target_eid)) = (u32::try_from(source), u32::try_from(target)) else {
        send_gm_feedback(entity_id, "testLOS: entity ids out of range.", tx).await;
        return true;
    };

    let caller_space = space_mgr.get_entity(entity_id).map(|e| e.space_id.0);
    let in_caller_space =
        |eid: u32| space_mgr.get_entity(eid).map(|e| e.space_id.0) == caller_space;
    if !in_caller_space(source_eid) || !in_caller_space(target_eid) {
        send_gm_feedback(
            entity_id,
            "testLOS: source/target not found in your space.",
            tx,
        )
        .await;
        return true;
    }

    let clear = space_mgr.has_line_of_sight(source_eid, target_eid);
    let verdict = if clear { "CLEAR" } else { "BLOCKED" };
    let text = format!("testLOS {source_eid} → {target_eid}: {verdict}");
    tracing::info!(entity_id, source_eid, target_eid, clear, "testLOS");
    send_gm_feedback(entity_id, &text, tx).await;
    true
}
