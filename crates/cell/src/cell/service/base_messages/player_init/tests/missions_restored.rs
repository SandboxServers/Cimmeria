//! TG-MIS-05: one `missions_restored` row per login says what the saved
//! journal restored to: the active missions with their step (hidden ones
//! marked `h`), and the completed and failed counts.

use super::super::*;
use crate::cell::messages::SavedMission;
use crate::test_support::{Captured, LogCapture};
use cimmeria_entity::cell_entity::SystemOptions;
use cimmeria_entity::missions::{MISSION_ACTIVE, MISSION_COMPLETED, MISSION_FAILED};
use tracing::Level;

fn saved(mission_id: i32, status: i8, step: Option<i32>) -> SavedMission {
    SavedMission {
        mission_id,
        status,
        current_step_id: step,
        completed_step_ids: vec![],
        completed_objective_ids: vec![],
        active_objective_ids: vec![],
        failed_objective_ids: vec![],
        repeats: 0,
    }
}

/// Runs the `InitPlayerState` handler over four saved rows: a hidden active
/// mission (622, defined hidden), a plain active one, a completed one and a
/// failed one. Returns the `missions_restored` row.
async fn restored_row() -> Captured {
    let capture = LogCapture::install();
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
    }
    mgr.connect_entity(1);
    mgr.mission_defs.insert(
        622,
        crate::cell::spawner::MissionDefEntry {
            step_id: 2113,
            objectives: vec![],
            is_hidden: true,
            num_repeats: 0,
            can_repeat_on_fail: false,
        },
    );
    let rows = vec![
        saved(622, MISSION_ACTIVE, Some(2113)),
        saved(1360, MISSION_ACTIVE, Some(4037)),
        saved(3238, MISSION_COMPLETED, None),
        saved(900, MISSION_FAILED, None),
    ];
    let (tx, _rx) = mpsc::channel(256);
    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        1,
        rows,
        vec![],
        0,
        vec![],
        SystemOptions::default(),
        0, // access_level
        WorldEntryCause::Login,
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;
    capture
        .all()
        .into_iter()
        .find(|c| {
            c.level == Level::INFO
                && c.fields.get("event").map(String::as_str) == Some("missions_restored")
        })
        .unwrap_or_else(|| panic!("no missions_restored row: {:#?}", capture.all()))
}

#[tokio::test]
async fn login_snapshots_the_restored_journal() {
    let row = restored_row().await;
    assert_eq!(
        row.fields.get("active").map(String::as_str),
        Some("622:2113h,1360:4037"),
        "active lists the active missions with step, hidden marked h: {row:#?}"
    );
    assert_eq!(row.fields.get("saved_count").map(String::as_str), Some("4"));
    assert_eq!(
        row.fields.get("completed_count").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        row.fields.get("failed_count").map(String::as_str),
        Some("1")
    );
}
