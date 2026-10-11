//! TG-NET-17: `session.start` carries `cimmeria.session_kind`, the lab or
//! player flag the base resolved from the account (TG-NET-16, D-TG13), so
//! one SigNoz filter separates lab runs from players on every service.
//! Driven through `handle_base_message`, so the stamp on the cell entity
//! (which the `playtest.friction` rows also read) is covered too.

use super::super::super::handle_base_message;
use super::super::*;
use crate::cell::messages::{BaseToCellMsg, WorldEntryCause};
use crate::test_support::LogCapture;
use tracing::Level;

const WORLD: &str = "Castle_CellBlock";
const ENTITY: u32 = 7011;
const PLAYER_ID: i32 = 4111;

fn manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr
}

/// The cell's entity stamp and the `session.start` field, for one world
/// entry whose `InitPlayerState` carries `session_kind`.
async fn enter_with(session_kind: Option<&'static str>) -> (Option<&'static str>, Option<String>) {
    let capture = LogCapture::install();
    let mut mgr = manager();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(256);
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    handle_base_message(
        BaseToCellMsg::CreateEntity {
            entity_id: ENTITY,
            world_name: WORLD.into(),
            position: [0.0; 3],
            rotation: [0.0; 3],
            destination_space_id: None,
            account_id: Some(6),
            player_id: Some(PLAYER_ID),
            account_name: Some("lab_login".into()),
            player_name: Some("Daniel".into()),
            reply_tx,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;
    reply_rx.await.expect("cell must ack the create");
    handle_base_message(
        BaseToCellMsg::InitPlayerState {
            entity_id: ENTITY,
            player_id: PLAYER_ID,
            account_id: 6,
            world_name: WORLD.into(),
            archetype_id: 1,
            saved_missions: vec![],
            abilities: vec![],
            active_bandolier_slot: 0,
            bandolier_items: vec![],
            system_options: cimmeria_entity::cell_entity::SystemOptions::default(),
            access_level: 0,
            known_stargates: vec![],
            tree_progress: Default::default(),
            level: 1,
            character_name: Some("Daniel".into()),
            body_set: None,
            looted_containers: Vec::new(),
            shown_tutorials: Vec::new(),
            entry_cause: WorldEntryCause::Login,
            session_kind,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;
    let stamped = mgr.get_entity(ENTITY).and_then(|e| e.session_kind);
    let starts: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.level == Level::INFO && c.target == "session.start")
        .collect();
    assert_eq!(starts.len(), 1, "one session.start per world entry");
    (
        stamped,
        starts[0].fields.get("cimmeria.session_kind").cloned(),
    )
}

#[tokio::test]
async fn session_start_carries_session_kind() {
    assert_eq!(
        enter_with(Some("lab")).await,
        (Some("lab"), Some("lab".to_string()))
    );
    assert_eq!(
        enter_with(Some("player")).await,
        (Some("player"), Some("player".to_string()))
    );
    // Unresolved: the field is left out, never "" or "unknown".
    assert_eq!(enter_with(None).await, (None, None));
}
