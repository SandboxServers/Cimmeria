//! Mission progression: advance step, complete objective, complete direct.

use tokio::sync::mpsc;

use cimmeria_entity::missions::{
    MissionObjective, MISSION_ACTIVE, STATUS_ACTIVE, STATUS_COMPLETED,
};

use super::{ON_MISSION_UPDATE, ON_OBJECTIVE_UPDATE, ON_STEP_UPDATE};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Advance a mission to a new step: complete old objectives, set new step, load new objectives.
#[tracing::instrument(
    name = "mission.advance_step",
    level = "info",
    skip_all,
    fields(entity_id, mission_id, new_step_id)
)]
pub async fn advance_step(
    entity_id: u32,
    mission_id: i32,
    new_step_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Load new step objectives from the cache before borrowing entity mutably
    let new_objectives: Vec<MissionObjective> = space_mgr
        .get_step_objectives(new_step_id)
        .into_iter()
        .map(|o| MissionObjective {
            objective_id: o.objective_id,
            status: STATUS_ACTIVE,
            hidden: o.is_hidden,
            optional: o.is_optional,
        })
        .collect();

    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => return,
    };

    let mission = match entity.missions.get_mission_mut(mission_id) {
        Some(m) => m,
        None => {
            tracing::warn!(
                entity_id,
                mission_id,
                new_step_id,
                "advance_step: mission not found"
            );
            return;
        }
    };

    // Complete all active objectives in the current step
    let old_objective_ids: Vec<i32> = mission
        .active_objectives
        .iter()
        .filter(|o| o.status != STATUS_COMPLETED)
        .map(|o| o.objective_id)
        .collect();
    for oid in &old_objective_ids {
        mission.complete_objective(*oid);
    }

    let old_step_id = mission.current_step_id;

    // Complete the old step
    if let Some(sid) = old_step_id {
        mission.completed_steps.push(sid);
    }

    // Set the new step
    mission.current_step_id = Some(new_step_id);
    mission.active_objectives = new_objectives.clone();

    tracing::info!(
        entity_id,
        mission_id,
        ?old_step_id,
        new_step_id,
        new_objectives = new_objectives.len(),
        "Mission step advanced"
    );

    // Send onStepUpdate(old_step_id, COMPLETED)
    if let Some(sid) = old_step_id {
        let mut args = Vec::with_capacity(5);
        args.extend_from_slice(&sid.to_le_bytes());
        args.push(STATUS_COMPLETED as u8);
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_STEP_UPDATE,
                args,
            })
            .await;
    }

    // Send onStepUpdate(new_step_id, ACTIVE)
    let mut args = Vec::with_capacity(5);
    args.extend_from_slice(&new_step_id.to_le_bytes());
    args.push(STATUS_ACTIVE as u8);
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_STEP_UPDATE,
            args,
        })
        .await;

    // Send onObjectiveUpdate for each new objective
    for obj in &new_objectives {
        let mut args = Vec::with_capacity(7);
        args.extend_from_slice(&obj.objective_id.to_le_bytes());
        args.push(STATUS_ACTIVE as u8);
        args.push(if obj.hidden { 1 } else { 0 });
        args.push(if obj.optional { 1 } else { 0 });
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_OBJECTIVE_UPDATE,
                args,
            })
            .await;
    }
}

/// Complete a mission objective and check if the mission advances.
#[tracing::instrument(
    name = "mission.complete_objective",
    level = "info",
    skip_all,
    fields(entity_id, mission_id, objective_id)
)]
pub async fn complete_objective(
    entity_id: u32,
    mission_id: i32,
    objective_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => return,
    };

    let mission = match entity.missions.get_mission_mut(mission_id) {
        Some(m) => m,
        None => return,
    };

    if !mission.complete_objective(objective_id) {
        return;
    }

    tracing::debug!(entity_id, mission_id, objective_id, "Objective completed");

    // Send onObjectiveUpdate with completed status
    let mut args = Vec::with_capacity(7);
    args.extend_from_slice(&objective_id.to_le_bytes());
    args.push(STATUS_COMPLETED as u8);
    args.push(0); // hidden
    args.push(0); // optional
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_OBJECTIVE_UPDATE,
            args,
        })
        .await;

    // Check if all objectives are completed → advance mission
    let all_required_complete = mission
        .active_objectives
        .iter()
        .filter(|o| !o.optional)
        .all(|o| o.status == STATUS_COMPLETED);

    if all_required_complete {
        mission.complete();

        // Send onStepUpdate completed
        if let Some(&step_id) = mission.completed_steps.last() {
            let mut args = Vec::with_capacity(5);
            args.extend_from_slice(&step_id.to_le_bytes());
            args.push(STATUS_COMPLETED as u8);
            let _ = tx
                .send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: ON_STEP_UPDATE,
                    args,
                })
                .await;
        }

        // Send onMissionUpdate completed
        let mut args = Vec::with_capacity(9);
        args.extend_from_slice(&mission_id.to_le_bytes());
        args.push(MISSION_ACTIVE as u8); // Status sent as "completed" removal
        args.extend_from_slice(&0i32.to_le_bytes());
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_MISSION_UPDATE,
                args,
            })
            .await;

        tracing::info!(entity_id, mission_id, "Mission completed!");
    }
}

/// Complete a mission directly (all objectives + step + mission update).
///
/// Used by the content engine when a chain action completes a mission
/// without stepping through individual objectives.
#[tracing::instrument(
    name = "mission.complete_direct",
    level = "info",
    skip_all,
    fields(entity_id, mission_id, player_id = tracing::field::Empty)
)]
pub async fn complete_mission_direct(
    entity_id: u32,
    mission_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => return,
    };
    if let Some(pid) = entity.player_id {
        tracing::Span::current().record("player_id", pid);
    }

    let mission = match entity.missions.get_mission_mut(mission_id) {
        Some(m) => m,
        None => {
            tracing::warn!(
                entity_id,
                mission_id,
                "complete_mission_direct: mission not found"
            );
            return;
        }
    };

    // Complete all objectives
    let objective_ids: Vec<i32> = mission
        .active_objectives
        .iter()
        .map(|o| o.objective_id)
        .collect();
    for oid in &objective_ids {
        mission.complete_objective(*oid);
    }
    mission.complete();

    let step_id = mission.completed_steps.last().copied();

    tracing::info!(entity_id, mission_id, "Mission completed directly");

    // Send objective updates
    for oid in &objective_ids {
        let mut args = Vec::with_capacity(7);
        args.extend_from_slice(&oid.to_le_bytes());
        args.push(STATUS_COMPLETED as u8);
        args.push(0); // hidden
        args.push(0); // optional
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_OBJECTIVE_UPDATE,
                args,
            })
            .await;
    }

    // Send step completed
    if let Some(sid) = step_id {
        let mut args = Vec::with_capacity(5);
        args.extend_from_slice(&sid.to_le_bytes());
        args.push(STATUS_COMPLETED as u8);
        let _ = tx
            .send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_STEP_UPDATE,
                args,
            })
            .await;
    }

    // Send mission completed
    let mut args = Vec::with_capacity(9);
    args.extend_from_slice(&mission_id.to_le_bytes());
    args.push(STATUS_COMPLETED as u8);
    args.extend_from_slice(&0i32.to_le_bytes());
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_MISSION_UPDATE,
            args,
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::missions::lifecycle::accept_mission;

    fn make_objectives() -> Vec<MissionObjective> {
        vec![MissionObjective {
            objective_id: 300,
            status: STATUS_ACTIVE,
            hidden: false,
            optional: false,
        }]
    }

    #[tokio::test]
    async fn complete_objective_completes_mission() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

        let (tx, mut rx) = mpsc::channel(16);
        accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;
        while rx.try_recv().is_ok() {}

        complete_objective(1, 100, 300, &tx, &mut mgr).await;

        // Should get: onObjectiveUpdate(completed) + onStepUpdate(completed) + onMissionUpdate
        let mut msgs = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            msgs.push(msg);
        }
        assert_eq!(msgs.len(), 3);

        // First: objective completed
        match &msgs[0] {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } => {
                assert_eq!(*method_index, 82); // onObjectiveUpdate
                assert_eq!(args[4], STATUS_COMPLETED as u8);
            }
            _ => panic!("unexpected"),
        }
    }
}
