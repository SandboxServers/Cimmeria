//! Mission-gap console commands: `.missionfail` and `.missionrewards`.
//!
//! These are normal GM gameplay actions the native client has siblings for
//! (complete/abandon/advance/clear) but not these two, so they have no native
//! slash binding.
//!
//! Legacy reference: `deprecated/python/cell/commands/Mission.py`
//! (`failMission`, `displayMissionRewards`).

use cimmeria_entity::missions::MISSION_FAILED;
use tokio::sync::mpsc;

use super::send_gm_feedback;
use crate::cell::client_methods::missionary::ON_MISSION_UPDATE;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// `.missionfail <missionId>` — force-fail an active mission on the target.
/// (Native has complete/abandon/advance/clear but no fail.)
pub(super) async fn fail(
    caller_id: u32,
    args: &[&str],
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target) = target_id else {
        send_gm_feedback(caller_id, "missionfail: a player target is required.", tx).await;
        return;
    };
    let Some(mission_id) = super::parse_i32(caller_id, args, 0, "missionId", tx).await else {
        return;
    };

    let failed = if let Some(m) = space_mgr
        .get_entity_mut(target)
        .and_then(|e| e.missions.get_mission_mut(mission_id))
    {
        m.fail();
        true
    } else {
        false
    };

    if !failed {
        send_gm_feedback(
            caller_id,
            &format!("missionfail: target has no active mission {mission_id}"),
            tx,
        )
        .await;
        return;
    }

    // Tell the client the mission moved to FAILED.
    let mut update = Vec::with_capacity(9);
    update.extend_from_slice(&mission_id.to_le_bytes());
    update.push(MISSION_FAILED as u8);
    update.extend_from_slice(&0i32.to_le_bytes());
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: target,
            method_index: ON_MISSION_UPDATE,
            args: update,
        })
        .await;

    tracing::info!(entity_id = target, mission_id, "GM .missionfail");
    send_gm_feedback(
        caller_id,
        &format!("missionfail [{target}] mission {mission_id} -> FAILED"),
        tx,
    )
    .await;
}

/// `.missionrewards <missionId>` — preview a mission's reward set on the target.
///
/// Reward dispatch is tracked separately; the cell doesn't cache the
/// reward catalog, so this reports the target's mission state (present + step)
/// and points at the reward-dispatch issue rather than inventing reward data.
pub(super) async fn rewards(
    caller_id: u32,
    args: &[&str],
    target_id: Option<u32>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let Some(target) = target_id else {
        send_gm_feedback(
            caller_id,
            "missionrewards: a player target is required.",
            tx,
        )
        .await;
        return;
    };
    let Some(mission_id) = super::parse_i32(caller_id, args, 0, "missionId", tx).await else {
        return;
    };

    let state = space_mgr
        .get_entity(target)
        .and_then(|e| e.missions.get_mission(mission_id))
        .map(|m| (m.status, m.current_step_id));

    let text = match state {
        Some((status, step)) => format!(
            "missionrewards [{target}] mission {mission_id}: status {status}, step {step:?}. \
             Reward dispatch is tracked separately (no reward catalog cell-side)."
        ),
        None => format!("missionrewards: target has no mission {mission_id}"),
    };
    send_gm_feedback(caller_id, &text, tx).await;
}
