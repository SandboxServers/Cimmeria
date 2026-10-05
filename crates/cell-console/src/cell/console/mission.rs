//! Mission-gap console commands: `.missionfail` and `.missionrewards`.
//!
//! These are normal GM gameplay actions the native client has siblings for
//! (complete/abandon/advance/clear) but not these two, so they have no native
//! slash binding.
//!
//! Legacy reference: `deprecated/python/cell/commands/Mission.py`
//! (`failMission`, `displayMissionRewards`).

use cimmeria_entity::missions::{MISSION_ACTIVE, MISSION_FAILED};
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

    // Only fail a mission that is currently ACTIVE. `get_mission_mut` returns
    // the record regardless of status, and `fail()` unconditionally sets
    // FAILED + bumps `repeats`, so without this filter `.missionfail` would
    // re-fail an already-completed/failed mission and corrupt its state.
    let failed = if let Some(m) = space_mgr
        .get_entity_mut(target)
        .and_then(|e| e.missions.get_mission_mut(mission_id))
        .filter(|m| m.status == MISSION_ACTIVE)
    {
        m.fail();
        Some(m.is_hidden)
    } else {
        None
    };

    let Some(is_hidden) = failed else {
        send_gm_feedback(
            caller_id,
            &format!("missionfail: target has no active mission {mission_id}"),
            tx,
        )
        .await;
        return;
    };

    // Tell the client the mission moved to FAILED -- unless it is hidden,
    // which the client never listed (#715, reference `MissionManager.py:724`).
    let player_id = space_mgr.get_entity(target).and_then(|e| e.player_id);
    if !crate::cell::missions::suppress_hidden_mission_frames(
        is_hidden,
        target,
        player_id,
        mission_id,
        "gm_missionfail",
    ) {
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
    }

    // Rule 5: `entity_*` / `player_*` name the GM; the character whose
    // mission failed is the `subject_*`.
    let gm = space_mgr.player_identity(caller_id);
    tracing::info!(
        entity_id = caller_id,
        entity_name = gm.player_name,
        account_id = gm.account_id,
        account_name = gm.account_name,
        player_id = gm.player_id,
        player_name = gm.player_name,
        subject_entity_id = target,
        subject_entity_name = space_mgr.entity_label(target),
        subject_player_id = player_id,
        subject_player_name = space_mgr.player_identity(target).player_name,
        mission_id,
        mission_name = cimmeria_names::book().mission(mission_id),
        "GM .missionfail"
    );

    // Discord gameplay-channel. This path is GM-forced, so the reason is
    // fixed; the mission's name comes from the NameBook.
    cimmeria_discord::emit_mission_failed(
        space_mgr.discord_character(target),
        cimmeria_discord::Named::new(
            mission_id,
            cimmeria_names::book()
                .mission(mission_id)
                .map(str::to_string),
        ),
        "gm_forced",
    );

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
