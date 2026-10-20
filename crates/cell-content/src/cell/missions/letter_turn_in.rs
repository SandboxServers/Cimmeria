//! Install the durable Frost letter mission postimage and announce its completion.

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_COMPLETED,
};
use cimmeria_wire::cell::messages::SavedMission;
use tokio::sync::mpsc;

use super::{
    send_mission_frame, suppress_hidden_mission_frames, ON_MISSION_UPDATE, ON_OBJECTIVE_UPDATE,
    ON_STEP_UPDATE,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Reject a malformed response instead of presenting success from a synthesized row.
pub(crate) fn valid_letter_postimage(saved: &SavedMission) -> bool {
    saved.mission_id == 1360
        && saved.status == MISSION_COMPLETED
        && saved.current_step_id.is_none()
        && saved.repeats == 1
        && saved.completed_step_ids == [4037, 4038]
        && saved.completed_objective_ids == [4650, 4651]
        && saved.active_objective_ids == [4651]
        && saved.failed_objective_ids.is_empty()
}

/// Replace the cell's mission with A1's exact durable postimage before any success send.
/// The current objective definition supplies only the client presentation flags.
pub(crate) fn install_letter_turn_in(
    entity_id: u32,
    saved: &SavedMission,
    space_mgr: &mut SpaceManager,
) -> bool {
    if !valid_letter_postimage(saved) {
        return false;
    }
    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        return false;
    };
    if entity.missions.is_state_unknown() || entity.missions.is_load_pending() {
        return false;
    }
    let Some(prior) = entity.missions.get_mission(1360) else {
        return false;
    };
    let objective = prior
        .active_objectives
        .iter()
        .find(|o| o.objective_id == 4651)
        .cloned()
        .unwrap_or(MissionObjective {
            objective_id: 4651,
            status: STATUS_COMPLETED,
            hidden: false,
            optional: false,
        });
    let mut completed = MissionInstance::new(
        1360,
        4038,
        vec![MissionObjective {
            status: STATUS_COMPLETED,
            ..objective
        }],
    );
    completed.status = saved.status;
    completed.current_step_id = saved.current_step_id;
    completed.completed_steps = saved.completed_step_ids.clone();
    completed.completed_objectives = saved.completed_objective_ids.clone();
    completed.repeats = saved.repeats;
    completed.is_hidden = prior.is_hidden;
    entity.missions.add_mission(completed);
    true
}

/// Send the three terminal journal frames in objective, step, mission order.
pub(crate) async fn announce_letter_turn_in(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let who = space_mgr.player_identity(entity_id);
    let Some(mission) = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.missions.get_mission(1360))
    else {
        return;
    };
    if suppress_hidden_mission_frames(
        mission.is_hidden,
        entity_id,
        who.player_id,
        1360,
        "letter_turn_in",
    ) {
        return;
    }
    let Some(objective) = mission
        .active_objectives
        .iter()
        .find(|o| o.objective_id == 4651)
    else {
        return;
    };
    let mut args = Vec::with_capacity(7);
    args.extend_from_slice(&4651i32.to_le_bytes());
    args.extend_from_slice(&[
        STATUS_COMPLETED as u8,
        u8::from(objective.hidden),
        u8::from(objective.optional),
    ]);
    send_mission_frame(
        tx,
        entity_id,
        ON_OBJECTIVE_UPDATE,
        args,
        "onObjectiveUpdate",
        1360,
        &who,
    )
    .await;
    let mut args = Vec::with_capacity(5);
    args.extend_from_slice(&4038i32.to_le_bytes());
    args.push(STATUS_COMPLETED as u8);
    send_mission_frame(
        tx,
        entity_id,
        ON_STEP_UPDATE,
        args,
        "onStepUpdate",
        1360,
        &who,
    )
    .await;
    let mut args = Vec::with_capacity(9);
    args.extend_from_slice(&1360i32.to_le_bytes());
    args.push(MISSION_COMPLETED as u8);
    args.extend_from_slice(&0i32.to_le_bytes());
    send_mission_frame(
        tx,
        entity_id,
        ON_MISSION_UPDATE,
        args,
        "onMissionUpdate",
        1360,
        &who,
    )
    .await;
}
