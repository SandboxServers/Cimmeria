//! Unit coverage for [`super::progression`]: advance step and complete
//! objective frames. Split out of `progression.rs` to keep it under the
//! file-size cap; the hidden-mission gate has its own file,
//! [`super::hidden_frames_tests`].

use tokio::sync::mpsc;

use cimmeria_entity::missions::{MissionObjective, STATUS_ACTIVE, STATUS_COMPLETED};

use super::lifecycle::accept_mission;
use super::progression::{advance_step, complete_objective};
use super::{ON_OBJECTIVE_UPDATE, ON_STEP_UPDATE};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

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

/// A step transition must report every old-step objective it silently
/// completes as a completed-status onObjectiveUpdate before the step
/// disappears — otherwise the last objective of a multi-objective AND-gate
/// resolved via AdvanceStep never reaches the client as completed.
#[tokio::test]
async fn advance_step_reports_completed_old_step_objectives() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

    let (tx, mut rx) = mpsc::channel(16);
    accept_mission(
        1,
        100,
        200,
        vec![
            // Hidden + optional: the completion frame must preserve these
            // flags, not collapse them to the completed path's 0/0.
            MissionObjective {
                objective_id: 300,
                status: STATUS_ACTIVE,
                hidden: true,
                optional: true,
            },
            MissionObjective {
                objective_id: 301,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            },
        ],
        &tx,
        &mut mgr,
    )
    .await;
    // Drain accept's initial messages: mission + step + 2 objectives.
    while rx.try_recv().is_ok() {}
    assert!(
        rx.try_recv().is_err(),
        "drain must empty the accept messages"
    );

    advance_step(1, 100, 201, &tx, &mut mgr).await;

    let mut msgs = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        msgs.push(msg);
    }

    // 2 completed-objective updates, then onStepUpdate(old, COMPLETED),
    // then onStepUpdate(new, ACTIVE). The objective updates must precede
    // the step updates so the client sees them while the old step is still
    // current.
    assert_eq!(
        msgs.len(),
        4,
        "advance_step must report each old-step objective it completes"
    );

    let decode_id = |args: &[u8]| i32::from_le_bytes([args[0], args[1], args[2], args[3]]);

    // First objective: real flags carried through (hidden=1, optional=1).
    match &msgs[0] {
        CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } => {
            assert_eq!(*method_index, ON_OBJECTIVE_UPDATE);
            assert_eq!(decode_id(args), 300);
            assert_eq!(args[4], STATUS_COMPLETED as u8);
            assert_eq!(args[5], 1, "hidden flag must be preserved");
            assert_eq!(args[6], 1, "optional flag must be preserved");
        }
        _ => panic!("expected onObjectiveUpdate"),
    }
    match &msgs[1] {
        CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } => {
            assert_eq!(*method_index, ON_OBJECTIVE_UPDATE);
            assert_eq!(decode_id(args), 301);
            assert_eq!(args[4], STATUS_COMPLETED as u8);
            assert_eq!(args[5], 0);
            assert_eq!(args[6], 0);
        }
        _ => panic!("expected onObjectiveUpdate at index 1"),
    }
    match &msgs[2] {
        CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } => {
            assert_eq!(*method_index, ON_STEP_UPDATE);
            assert_eq!(decode_id(args), 200);
            assert_eq!(args[4], STATUS_COMPLETED as u8);
        }
        _ => panic!("expected onStepUpdate"),
    }
    match &msgs[3] {
        CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } => {
            assert_eq!(*method_index, ON_STEP_UPDATE);
            assert_eq!(decode_id(args), 201);
            assert_eq!(args[4], STATUS_ACTIVE as u8);
        }
        _ => panic!("expected onStepUpdate"),
    }
}

/// Regression guard for the objective-completed fan-out on
/// `advance_step`: a closed cell→base channel must surface
/// `reason=advance_step_objective_send_failed`, not drop silently.
#[tokio::test]
async fn advance_step_warns_when_objective_update_send_fails() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

    let (tx, rx) = mpsc::channel(16);
    accept_mission(
        1,
        100,
        200,
        vec![
            MissionObjective {
                objective_id: 300,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            },
            MissionObjective {
                objective_id: 301,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            },
        ],
        &tx,
        &mut mgr,
    )
    .await;
    drop(rx); // close the cell→base channel

    advance_step(1, 100, 201, &tx, &mut mgr).await;

    assert!(
        capture
            .find_event(
                Level::WARN,
                "onObjectiveUpdate send failed",
                "advance_step_objective_send_failed"
            )
            .is_some(),
        "negative-logging convention: advance_step must WARN when the \
         objective-completed onObjectiveUpdate send fails; reverting to \
         `let _ = tx.send(...)` hides the unchecked-objective window. \
         Captured: {:#?}",
        capture.all()
    );
}
