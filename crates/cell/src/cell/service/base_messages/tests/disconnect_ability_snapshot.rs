//! AB-T5: a logout writes one `abilities.snapshot` row (`trigger = logout`)
//! before the teardown removes the entity.

use super::*;

use std::time::Duration;

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;
use tracing::Level;

use crate::test_support::LogCapture;

/// Revert proof: drop the hook from `handle_disconnect_entity`, or move it
/// after `flush_and_disconnect`, and no row is found (the entity is gone).
#[tokio::test]
async fn ab_t5_logout_snapshots_the_players_ability_state() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.abilities
            .start_ability_cooldown(592, Duration::from_secs(30));
    }
    mgr.connect_entity(1);

    let (tx, _rx) = mpsc::channel(64);
    let logs = LogCapture::install();
    handle_base_message(
        disconnect_entity_msg(1),
        &tx,
        &mut mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;

    assert!(mgr.get_entity(1).is_none(), "fixture: torn down");
    let row = logs
        .find_message(Level::INFO, "ability state snapshot")
        .expect("one snapshot row on logout");
    assert_eq!(row.target, "abilities.snapshot");
    assert!(row.has_field("trigger", "logout"));
    assert!(row.has_field("player_id", "100"));
    assert!(row.has_field("cooldowns", "1"));
}
