//! Durable mission row installation for the Frost letter receipt.

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
    STATUS_COMPLETED,
};
use cimmeria_wire::cell::messages::SavedMission;

use super::letter_turn_in::{install_letter_turn_in, valid_letter_postimage};
use crate::cell::space_manager::SpaceManager;

fn postimage() -> SavedMission {
    SavedMission {
        mission_id: 1360,
        status: MISSION_COMPLETED,
        current_step_id: None,
        completed_step_ids: vec![4037, 4038],
        completed_objective_ids: vec![4650, 4651],
        active_objective_ids: vec![4651],
        failed_objective_ids: vec![],
        repeats: 1,
    }
}

#[test]
fn exact_postimage_installs_before_any_effect() {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset_CmdCenter" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(101, "Harset_CmdCenter", [0.0; 3], [0.0; 3])
        .unwrap();
    let player = mgr.get_entity_mut(101).unwrap();
    player.is_player = true;
    player.missions.add_mission(MissionInstance::new(
        1360,
        4038,
        vec![MissionObjective {
            objective_id: 4651,
            status: STATUS_ACTIVE,
            hidden: true,
            optional: true,
        }],
    ));
    assert!(install_letter_turn_in(101, &postimage(), &mut mgr));
    let mission = mgr
        .get_entity(101)
        .unwrap()
        .missions
        .get_mission(1360)
        .unwrap();
    assert_eq!(mission.status, MISSION_COMPLETED);
    assert_eq!(mission.current_step_id, None);
    assert_eq!(mission.completed_steps, [4037, 4038]);
    assert_eq!(mission.completed_objectives, [4650, 4651]);
    assert_eq!(mission.repeats, 1);
    assert_eq!(mission.active_objectives.len(), 1);
    assert_eq!(mission.active_objectives[0].status, STATUS_COMPLETED);
    assert!(mission.active_objectives[0].hidden && mission.active_objectives[0].optional);
}

#[test]
fn malformed_or_incomplete_postimage_is_rejected() {
    let good = postimage();
    assert!(valid_letter_postimage(&good));
    let mut invalid = good.clone();
    invalid.status = MISSION_ACTIVE;
    assert!(!valid_letter_postimage(&invalid));
    let mut invalid = good.clone();
    invalid.completed_objective_ids.pop();
    assert!(!valid_letter_postimage(&invalid));
    let mut invalid = good.clone();
    invalid.active_objective_ids.clear();
    assert!(!valid_letter_postimage(&invalid));
    let mut invalid = good;
    invalid.repeats = 2;
    assert!(!valid_letter_postimage(&invalid));
}
