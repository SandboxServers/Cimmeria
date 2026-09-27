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

/// NA25: the `deduped` TRACE row fires once per NPC per AI tick and now
/// reaches SigNoz, so it is sampled 1-in-53 with `suppressed`. The sampler
/// is a process-wide static, so the count is bounded rather than exact.
/// Removing the sampler writes all 200 rows and fails the bound.
#[tokio::test]
async fn deduped_movement_type_row_is_sampled() {
    let mut mgr = fixture();
    let (tx, _rx) = mpsc::channel(8);
    broadcast_movement_type(101, Some(MobMovementType::Patrol), &tx, &mut mgr).await;
    let logs = LogCapture::install();
    for _ in 0..200 {
        broadcast_movement_type(101, Some(MobMovementType::Patrol), &tx, &mut mgr).await;
    }
    let rows: Vec<_> = logs
        .all()
        .into_iter()
        .filter(|c| c.target == "movement.movement_type" && c.has_field("outcome", "deduped"))
        .collect();
    assert!(
        (3..=5).contains(&rows.len()),
        "200 deduped calls must write ~200/53 rows, got {}",
        rows.len()
    );
    assert!(rows.iter().all(|c| c.level == Level::TRACE
        && c.has_field("sampled_1_in", "53")
        && c.fields.contains_key("suppressed")));
}
