//! Mission operations for the CellService.
//!
//! Handles mission accept, abandon, complete, and objective updates.
//! Sends wire-format mission state to the client.
//!
//! Reference: `python/cell/MissionManager.py`

use tokio::sync::mpsc;

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective,
    MISSION_ACTIVE, STATUS_ACTIVE, STATUS_COMPLETED,
};

use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// ── Method indices for mission client methods ────────────────────────────────
// Missionary interface: flat indices 80-84

/// onMissionUpdate(INT32 missionId, INT8 status, INT32 giverName)
const ON_MISSION_UPDATE: u16 = 80;
/// onStepUpdate(INT32 stepId, INT8 status)
const ON_STEP_UPDATE: u16 = 81;
/// onObjectiveUpdate(INT32 objectiveId, INT8 status, INT8 hidden, INT8 optional)
const ON_OBJECTIVE_UPDATE: u16 = 82;

/// Send all active mission state to the client (called during mapLoaded).
///
/// Reference: `python/cell/MissionManager.py:559-574 resend()`
pub async fn resend_missions(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    let entity = match space_mgr.get_entity(entity_id) {
        Some(e) => e,
        None => return,
    };

    let messages = entity.missions.serialize_resend();
    for (method_index, args) in messages {
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        }).await;
    }
}

/// Accept a mission: create a MissionInstance and send initial state to client.
pub async fn accept_mission(
    entity_id: u32,
    mission_id: i32,
    step_id: i32,
    objectives: Vec<MissionObjective>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => return,
    };

    let mission = MissionInstance::new(mission_id, step_id, objectives.clone());
    entity.missions.add_mission(mission);

    tracing::info!(entity_id, mission_id, step_id, "Mission accepted");

    // Send onMissionUpdate
    let mut args = Vec::with_capacity(9);
    args.extend_from_slice(&mission_id.to_le_bytes());
    args.push(STATUS_ACTIVE as u8);
    args.extend_from_slice(&0i32.to_le_bytes());
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: ON_MISSION_UPDATE,
        args,
    }).await;

    // Send onStepUpdate
    let mut args = Vec::with_capacity(5);
    args.extend_from_slice(&step_id.to_le_bytes());
    args.push(STATUS_ACTIVE as u8);
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: ON_STEP_UPDATE,
        args,
    }).await;

    // Send onObjectiveUpdate per objective
    for obj in &objectives {
        let mut args = Vec::with_capacity(7);
        args.extend_from_slice(&obj.objective_id.to_le_bytes());
        args.push(obj.status as u8);
        args.push(if obj.hidden { 1 } else { 0 });
        args.push(if obj.optional { 1 } else { 0 });
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_OBJECTIVE_UPDATE,
            args,
        }).await;
    }
}

/// Abandon a mission: remove it and send removal to client.
pub async fn abandon_mission(
    entity_id: u32,
    mission_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => return,
    };

    if entity.missions.remove_mission(mission_id).is_some() {
        tracing::info!(entity_id, mission_id, "Mission abandoned");

        // Send onMissionUpdate with status=completed (removes from client log)
        let mut args = Vec::with_capacity(9);
        args.extend_from_slice(&mission_id.to_le_bytes());
        args.push(STATUS_COMPLETED as u8);
        args.extend_from_slice(&0i32.to_le_bytes());
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_MISSION_UPDATE,
            args,
        }).await;
    }
}

/// Complete a mission objective and check if the mission advances.
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
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: ON_OBJECTIVE_UPDATE,
        args,
    }).await;

    // Check if all objectives are completed → advance mission
    let all_required_complete = mission.active_objectives.iter()
        .filter(|o| !o.optional)
        .all(|o| o.status == STATUS_COMPLETED);

    if all_required_complete {
        mission.complete();

        // Send onStepUpdate completed
        if let Some(&step_id) = mission.completed_steps.last() {
            let mut args = Vec::with_capacity(5);
            args.extend_from_slice(&step_id.to_le_bytes());
            args.push(STATUS_COMPLETED as u8);
            let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: ON_STEP_UPDATE,
                args,
            }).await;
        }

        // Send onMissionUpdate completed
        let mut args = Vec::with_capacity(9);
        args.extend_from_slice(&mission_id.to_le_bytes());
        args.push(MISSION_ACTIVE as u8); // Status sent as "completed" removal
        args.extend_from_slice(&0i32.to_le_bytes());
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_MISSION_UPDATE,
            args,
        }).await;

        tracing::info!(entity_id, mission_id, "Mission completed!");
    }
}

/// Complete a mission directly (all objectives + step + mission update).
///
/// Used by the content engine when a chain action completes a mission
/// without stepping through individual objectives.
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

    let mission = match entity.missions.get_mission_mut(mission_id) {
        Some(m) => m,
        None => {
            tracing::warn!(entity_id, mission_id, "complete_mission_direct: mission not found");
            return;
        }
    };

    // Complete all objectives
    let objective_ids: Vec<i32> = mission.active_objectives.iter()
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
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_OBJECTIVE_UPDATE,
            args,
        }).await;
    }

    // Send step completed
    if let Some(sid) = step_id {
        let mut args = Vec::with_capacity(5);
        args.extend_from_slice(&sid.to_le_bytes());
        args.push(STATUS_COMPLETED as u8);
        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: ON_STEP_UPDATE,
            args,
        }).await;
    }

    // Send mission completed
    let mut args = Vec::with_capacity(9);
    args.extend_from_slice(&mission_id.to_le_bytes());
    args.push(STATUS_COMPLETED as u8);
    args.extend_from_slice(&0i32.to_le_bytes());
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: ON_MISSION_UPDATE,
        args,
    }).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_objectives() -> Vec<MissionObjective> {
        vec![
            MissionObjective {
                objective_id: 300,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            },
        ]
    }

    #[tokio::test]
    async fn accept_sends_three_messages() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

        let (tx, mut rx) = mpsc::channel(16);
        accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;

        // Should get: onMissionUpdate + onStepUpdate + onObjectiveUpdate = 3
        let mut msgs = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            msgs.push(msg);
        }
        assert_eq!(msgs.len(), 3);

        // Verify method indices
        let indices: Vec<u16> = msgs.iter().map(|m| match m {
            CellToBaseMsg::EntityMethodCall { method_index, .. } => *method_index,
            _ => panic!("unexpected message"),
        }).collect();
        assert_eq!(indices, vec![80, 81, 82]);
    }

    #[tokio::test]
    async fn abandon_removes_mission() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

        let (tx, mut rx) = mpsc::channel(16);
        accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;
        // Drain accept messages
        while rx.try_recv().is_ok() {}

        abandon_mission(1, 100, &tx, &mut mgr).await;

        // Should get onMissionUpdate with completed status
        let msg = rx.try_recv().unwrap();
        match msg {
            CellToBaseMsg::EntityMethodCall { method_index, args, .. } => {
                assert_eq!(method_index, 80);
                assert_eq!(args[4], STATUS_COMPLETED as u8);
            }
            _ => panic!("unexpected"),
        }

        // Mission should be gone
        assert_eq!(mgr.get_entity(1).unwrap().missions.count(), 0);
    }

    #[tokio::test]
    async fn complete_objective_completes_mission() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
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
            CellToBaseMsg::EntityMethodCall { method_index, args, .. } => {
                assert_eq!(*method_index, 82); // onObjectiveUpdate
                assert_eq!(args[4], STATUS_COMPLETED as u8);
            }
            _ => panic!("unexpected"),
        }
    }

    #[tokio::test]
    async fn resend_sends_active_missions() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

        let (tx, mut rx) = mpsc::channel(16);
        accept_mission(1, 100, 200, make_objectives(), &tx, &mut mgr).await;
        while rx.try_recv().is_ok() {}

        resend_missions(1, &tx, &mgr).await;

        let mut msgs = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            msgs.push(msg);
        }
        // 1 mission × (1 update + 1 step + 1 objective) = 3
        assert_eq!(msgs.len(), 3);
    }
}
