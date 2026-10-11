//! TG-NET-15: `session.start` says whether the world entry was a login or a
//! travel re-entry, so SigNoz can count logins without the extra starts a
//! gate trip, space transfer or cross-world ring adds to one session.

use super::super::*;
use crate::test_support::LogCapture;
use cimmeria_entity::cell_entity::SystemOptions;
use tracing::Level;

fn make_mgr() -> SpaceManager {
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
    mgr
}

/// The `start_cause` on the one `session.start` row a world entry emits.
async fn start_cause_for(entry_cause: WorldEntryCause) -> String {
    let capture = LogCapture::install();
    let mut mgr = make_mgr();
    let (tx, _rx) = mpsc::channel(256);
    handle_init_player_state(
        1,
        100,
        "Castle_CellBlock".into(),
        1,
        vec![],
        vec![],
        0,
        vec![],
        SystemOptions::default(),
        0, // access_level
        entry_cause,
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;
    let starts: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.level == Level::INFO && c.target == "session.start")
        .collect();
    assert_eq!(starts.len(), 1, "one session.start per world entry");
    assert!(starts[0].message_contains("player entered world"));
    starts[0]
        .fields
        .get("start_cause")
        .cloned()
        .unwrap_or_else(|| panic!("session.start lacks start_cause: {:#?}", starts[0]))
}

#[tokio::test]
async fn session_start_marks_travel_entries() {
    assert_eq!(start_cause_for(WorldEntryCause::Travel).await, "travel");
    assert_eq!(start_cause_for(WorldEntryCause::Login).await, "login");
}
