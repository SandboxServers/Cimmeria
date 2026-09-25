//! A movement-type change is recorded server-side only. Nothing goes on the
//! wire: no server-to-client movement-type message exists, and the old send
//! reached witnesses as a truncated `onSequence` (NA10). Each change must
//! still leave a record, so the NPC's intended movement mode can be read out
//! of SigNoz.

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
async fn movement_type_suppressed_and_cleared_are_both_logged() {
    let mut mgr = fixture();
    let (tx, _rx) = mpsc::channel(8);
    let logs = LogCapture::install();

    broadcast_movement_type(101, Some(MobMovementType::Follow), &tx, &mut mgr).await;
    let recorded = logs
        .find_message(Level::DEBUG, "movement type recorded")
        .expect("a recorded movement type must be logged");
    assert_eq!(recorded.target, "movement.movement_type");
    assert!(recorded.has_field("outcome", "suppressed"));
    assert!(recorded.has_field("kind_byte", "3"));

    broadcast_movement_type(101, None, &tx, &mut mgr).await;
    let cleared = logs
        .find_message(Level::DEBUG, "cache cleared")
        .expect("a cleared movement type must be logged");
    assert!(cleared.has_field("outcome", "cleared"));
    assert!(cleared.has_field("prior_kind", "Some(Follow)"));
}
