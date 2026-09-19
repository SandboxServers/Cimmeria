//! `setMovementType` selects the mob animation on every witness, and two of
//! its three outcomes put nothing on the wire. Each must leave a record, or a
//! "moonwalking" NPC (translating in a stale pose) cannot be diagnosed.

use cimmeria_entity::cell_entity::MobMovementType;
use tokio::sync::mpsc;
use tracing::Level;

use super::broadcast_movement_type;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

fn fixture() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.spawn_npc(101, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr
}

#[tokio::test]
async fn movement_type_sent_and_cleared_are_both_logged() {
    let mut mgr = fixture();
    let (tx, _rx) = mpsc::channel(8);
    let logs = LogCapture::install();

    broadcast_movement_type(101, Some(MobMovementType::Follow), &tx, &mut mgr).await;
    let sent = logs
        .find_message(Level::DEBUG, "setMovementType sent")
        .expect("a sent movement type must be logged");
    assert_eq!(sent.target, "movement.movement_type");
    assert!(sent.has_field("outcome", "sent"));
    assert!(sent.has_field("kind_byte", "3"));

    // Clearing sends NOTHING -- the client keeps the Follow animation. That
    // silent divergence is exactly what has to be visible.
    broadcast_movement_type(101, None, &tx, &mut mgr).await;
    let cleared = logs
        .find_message(Level::DEBUG, "cache cleared")
        .expect("a cleared movement type must be logged");
    assert!(cleared.has_field("outcome", "cleared"));
    assert!(cleared.has_field("prior_kind", "Some(Follow)"));
}
