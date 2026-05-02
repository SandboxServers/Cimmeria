use super::*;

#[test]
fn saved_mission_debug_prints() {
    let m = SavedMission {
        mission_id: 622,
        status: 1,
        current_step_id: Some(700),
        completed_step_ids: vec![],
        completed_objective_ids: vec![],
        active_objective_ids: vec![800, 801],
        failed_objective_ids: vec![],
    };
    let debug = format!("{:?}", m);
    assert!(debug.contains("622"));
    assert!(debug.contains("800"));
}

#[test]
fn saved_mission_clone() {
    let m = SavedMission {
        mission_id: 622,
        status: 1,
        current_step_id: Some(700),
        completed_step_ids: vec![699],
        completed_objective_ids: vec![801],
        active_objective_ids: vec![800],
        failed_objective_ids: vec![],
    };
    let cloned = m.clone();
    assert_eq!(cloned.mission_id, 622);
    assert_eq!(cloned.status, 1);
    assert_eq!(cloned.current_step_id, Some(700));
    assert_eq!(cloned.completed_step_ids, vec![699]);
    assert_eq!(cloned.completed_objective_ids, vec![801]);
    assert_eq!(cloned.active_objective_ids, vec![800]);
    assert!(cloned.failed_objective_ids.is_empty());
}

#[test]
fn saved_mission_completed_status() {
    let m = SavedMission {
        mission_id: 622,
        status: 2, // MISSION_COMPLETED
        current_step_id: None,
        completed_step_ids: vec![700],
        completed_objective_ids: vec![800, 801],
        active_objective_ids: vec![],
        failed_objective_ids: vec![],
    };
    assert_eq!(m.status, 2);
    assert!(m.current_step_id.is_none());
    assert_eq!(m.completed_step_ids.len(), 1);
    assert_eq!(m.completed_objective_ids.len(), 2);
}

#[test]
fn npc_aoi_data_default() {
    let data = NpcAoIData::default();
    assert_eq!(data.faction, 0);
    assert_eq!(data.alignment, 0);
    assert_eq!(data.entity_flags, 0);
    assert_eq!(data.interaction_type, 0);
    assert!(data.name_id.is_none());
    assert!(data.components.is_empty());
}
