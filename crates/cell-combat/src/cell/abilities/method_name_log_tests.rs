//! A log line that carries a client-method index
//! carries its `method_name` too, read from the entity's own type table.
//!
//! The bug shapes these reproduce: a `method_index` with no name next to it
//! (the reader has to look the index up in the dispatch table), and a name
//! read from the SGWPlayer table for an NPC. Index 27 is
//! `onSystemCommunication` on a player and `onAggressionOverrideUpdate` on a
//! mob, so the mob guard fails on either regression.

use tokio::sync::mpsc;
use tracing::Level;

use super::messaging::{send_entity_method, send_entity_method_to_witnesses};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;
use cimmeria_wire::names::{SGWMOB_CLASS_ID, SGWPLAYER_CLASS_ID};

const PLAYER: u32 = 1;
const MOB: u32 = 3;

/// A player and a mob that sees each other, and a cell-to-base channel
/// whose receiver is gone, so every send fails and logs its WARN.
fn fixture() -> (SpaceManager, mpsc::Sender<CellToBaseMsg>) {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(PLAYER, "Castle", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(MOB, "Castle", [0.0; 3], [0.0; 3])
        .unwrap();
    let player = mgr.get_entity_mut(PLAYER).unwrap();
    player.is_player = true;
    player.player_id = Some(100);
    player.class_id = SGWPLAYER_CLASS_ID;
    mgr.get_entity_mut(MOB).unwrap().class_id = SGWMOB_CLASS_ID;
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
    let (tx, rx) = mpsc::channel(8);
    drop(rx);
    (mgr, tx)
}

#[tokio::test]
async fn a_mob_method_is_named_from_the_mob_table() {
    let (mgr, tx) = fixture();
    let logs = LogCapture::install();

    send_entity_method_to_witnesses(MOB, 27, vec![1], &tx, &mgr).await;

    let warn = logs
        .find_event(
            Level::WARN,
            "not queued for its witnesses",
            "cell_to_base_closed",
        )
        .expect("a refused witness fan-out logs a WARN");
    assert!(warn.has_field("method_index", "27"), "{warn:#?}");
    assert!(
        warn.has_field("method_name", "onAggressionOverrideUpdate"),
        "the mob's 27 must be named from the SGWMob table, not SGWPlayer's \
         onSystemCommunication; got {warn:#?}"
    );
}

#[tokio::test]
async fn a_player_method_is_named_from_the_player_table() {
    let (mgr, tx) = fixture();
    let logs = LogCapture::install();

    send_entity_method(PLAYER, 27, vec![1], &tx, &mgr).await;

    let warn = logs
        .find_event(
            Level::WARN,
            "not queued for the owner's client",
            "cell_to_base_closed",
        )
        .expect("a refused self send logs a WARN");
    assert!(warn.has_field("method_index", "27"), "{warn:#?}");
    assert!(
        warn.has_field("method_name", "onSystemCommunication"),
        "{warn:#?}"
    );
}

/// An index no table has is left out, not written as a placeholder.
#[tokio::test]
async fn an_unknown_index_leaves_the_name_out() {
    let (mgr, tx) = fixture();
    let logs = LogCapture::install();

    send_entity_method(PLAYER, 999, vec![1], &tx, &mgr).await;

    let warn = logs
        .find_event(
            Level::WARN,
            "not queued for the owner's client",
            "cell_to_base_closed",
        )
        .expect("a refused self send logs a WARN");
    assert!(warn.has_field("method_index", "999"), "{warn:#?}");
    assert!(
        !warn.fields.contains_key("method_name"),
        "an unresolved name is omitted, never \"unknown\"; got {warn:#?}"
    );
}
