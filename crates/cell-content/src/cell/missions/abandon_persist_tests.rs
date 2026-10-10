//! Guards for saving an abandon (#1315).
//!
//! Every abandon path (the player's `abandonMission`, the `abandon_mission`
//! chain action, `gmMissionClear` / `gmMissionAbandon`) goes through
//! [`abandon_mission`], which used to change cell memory only: the saved row
//! stayed active and the mission came back after a relog. These drive
//! `abandon_mission` directly and check the `MissionUpdate` it hands the base
//! and what it leaves in memory. The base half (delete at `repeats` 0, keep a
//! not-active row otherwise) is the live-DB test in `cimmeria-services`
//! (`mission_abandon_round_trip_tests`).

use tokio::sync::mpsc;

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_COMPLETED, MISSION_FAILED, MISSION_NOT_ACTIVE,
    STATUS_ACTIVE,
};

use super::{abandon_mission, accept_mission};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::MissionDefEntry;

const EID: u32 = 1;
const PLAYER_ID: i32 = 1315;
const MISSION: i32 = 1001;
const STEP: i32 = 200;

fn make_mgr(num_repeats: i32) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(EID, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.get_entity_mut(EID).unwrap().player_id = Some(PLAYER_ID);
    mgr.mission_defs.insert(
        MISSION,
        MissionDefEntry {
            step_id: STEP,
            objectives: vec![],
            is_hidden: false,
            num_repeats,
            can_repeat_on_fail: false,
        },
    );
    mgr
}

fn objectives() -> Vec<MissionObjective> {
    vec![MissionObjective {
        objective_id: 300,
        status: STATUS_ACTIVE,
        hidden: false,
        optional: false,
    }]
}

/// `(player_id, mission_id, status, current_step_id, every array empty,
/// repeats)` of each `MissionUpdate` in `msgs`.
fn updates(msgs: &[CellToBaseMsg]) -> Vec<(i32, i32, i8, Option<i32>, bool, i32)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::MissionUpdate {
                player_id,
                mission_id,
                status,
                current_step_id,
                completed_step_ids,
                completed_objective_ids,
                active_objective_ids,
                failed_objective_ids,
                repeats,
            } => Some((
                *player_id,
                *mission_id,
                *status,
                *current_step_id,
                completed_step_ids.is_empty()
                    && completed_objective_ids.is_empty()
                    && active_objective_ids.is_empty()
                    && failed_objective_ids.is_empty(),
                *repeats,
            )),
            _ => None,
        })
        .collect()
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

/// **Regression guard (#1315).** Abandoning a mission hands the base exactly
/// one `MissionUpdate`: not active, no step, no objectives, `repeats` 0 (the
/// base deletes that row). Before the fix there was none, so the saved row
/// stayed active. A mission with no completed run leaves nothing in memory.
#[tokio::test]
async fn abandon_saves_a_not_active_row() {
    let mut mgr = make_mgr(0);
    let (tx, mut rx) = mpsc::channel(32);
    assert!(accept_mission(EID, MISSION, STEP, objectives(), &tx, &mut mgr).await);
    drain(&mut rx);

    assert!(abandon_mission(EID, MISSION, &tx, &mut mgr).await);
    assert_eq!(
        updates(&drain(&mut rx)),
        vec![(PLAYER_ID, MISSION, MISSION_NOT_ACTIVE, None, true, 0)]
    );
    assert!(mgr
        .get_entity(EID)
        .unwrap()
        .missions
        .get_mission(MISSION)
        .is_none());
}

/// **Regression guard (#1315, #118).** A repeatable mission completed twice,
/// re-accepted and abandoned keeps its count: the save carries `repeats` 2,
/// memory keeps a not-active record with it, and a re-accept in the same
/// session counts from it. The record is not a held mission, so abandoning
/// it again does nothing and saves nothing.
#[tokio::test]
async fn abandon_of_a_repeated_mission_keeps_its_repeats() {
    let mut mgr = make_mgr(5);
    let mut done = MissionInstance::new(MISSION, STEP, vec![]);
    done.status = MISSION_COMPLETED;
    done.current_step_id = None;
    done.repeats = 2;
    mgr.get_entity_mut(EID).unwrap().missions.add_mission(done);
    let (tx, mut rx) = mpsc::channel(32);

    assert!(accept_mission(EID, MISSION, STEP, objectives(), &tx, &mut mgr).await);
    drain(&mut rx);
    assert!(abandon_mission(EID, MISSION, &tx, &mut mgr).await);
    assert_eq!(
        updates(&drain(&mut rx)),
        vec![(PLAYER_ID, MISSION, MISSION_NOT_ACTIVE, None, true, 2)]
    );
    let record = mgr
        .get_entity(EID)
        .unwrap()
        .missions
        .get_mission(MISSION)
        .expect("a not-active record keeps the count");
    assert_eq!(
        (record.status, record.current_step_id, record.repeats),
        (MISSION_NOT_ACTIVE, None, 2)
    );

    assert!(
        !abandon_mission(EID, MISSION, &tx, &mut mgr).await,
        "the not-active record is not a held mission"
    );
    assert!(drain(&mut rx).is_empty(), "nothing to save or send");

    assert!(accept_mission(EID, MISSION, STEP, objectives(), &tx, &mut mgr).await);
    assert_eq!(
        mgr.get_entity(EID)
            .unwrap()
            .missions
            .get_mission(MISSION)
            .unwrap()
            .repeats,
        2,
        "a re-accept counts from the kept repeats"
    );
}

/// **Regression guard (CS-08 review R1).** Only an active mission can be
/// abandoned. A completed or a failed record, including a pre-#118 one with
/// `repeats` 0 (which the base would DELETE), is left exactly as it is:
/// `false`, no `MissionUpdate`, no client frame. Letting it through saved a
/// not-active row over the completion for good, so a forged `abandonMission`
/// made a finished mission re-earnable.
#[tokio::test]
async fn abandon_of_a_finished_mission_is_refused_and_saves_nothing() {
    for (status, repeats) in [
        (MISSION_COMPLETED, 1),
        (MISSION_COMPLETED, 0),
        (MISSION_FAILED, 1),
        (MISSION_FAILED, 0),
    ] {
        let mut mgr = make_mgr(0);
        let mut finished = MissionInstance::new(MISSION, STEP, vec![]);
        finished.status = status;
        finished.current_step_id = None;
        finished.repeats = repeats;
        mgr.get_entity_mut(EID)
            .unwrap()
            .missions
            .add_mission(finished);
        let (tx, mut rx) = mpsc::channel(32);

        assert!(
            !abandon_mission(EID, MISSION, &tx, &mut mgr).await,
            "status {status} repeats {repeats}: refused"
        );
        assert!(
            drain(&mut rx).is_empty(),
            "status {status} repeats {repeats}: nothing saved or sent"
        );
        let kept = mgr
            .get_entity(EID)
            .unwrap()
            .missions
            .get_mission(MISSION)
            .expect("the finished record stays");
        assert_eq!((kept.status, kept.repeats), (status, repeats));
    }
}
