//! Field hygiene on cell rows the 2026-09-29 colo playtest had to read.
//!
//! SigNoz types a log attribute by the values it sees, so a row that logs
//! `player_id` Debug-formatted (`"Some(72)"`) creates a second, *string*
//! `player_id` key beside every other row's number, and a
//! `player_id = 72` filter silently misses it. These guards pin the
//! numeric form: `LogCapture` records a numeric field as `72` and a
//! Debug-formatted `Option` as `Some(72)`, so reverting to `?` fails them.

use super::*;
use crate::test_support::LogCapture;
use tracing::Level;

const PLAYER_ID: i32 = 72;
const ACCOUNT_ID: u32 = 6;

fn player_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(1).unwrap();
    e.is_player = true;
    e.player_id = Some(PLAYER_ID);
    e.account_id = Some(ACCOUNT_ID);
    mgr.connect_entity(1);
    mgr
}

fn pistol() -> BandolierItem {
    BandolierItem {
        instance_id: 0,
        item_id: 55,
        clip_size: 15,
        default_ammo_type: 2,
        current_ammo: 15,
        cur_ammo_type: 2,
    }
}

fn assert_numeric_identity(row: &crate::test_support::Captured) {
    assert!(
        row.has_field("player_id", &PLAYER_ID.to_string()),
        "player_id must be numeric, not `Some(..)`: {row:#?}"
    );
    assert!(
        row.has_field("account_id", &ACCOUNT_ID.to_string()),
        "{row:#?}"
    );
}

/// `UpdateBandolierItem: equip-display decision` was the only server row
/// logging `player_id` as a string (SigNoz, 7 days to 2026-09-29).
#[tokio::test]
async fn update_bandolier_decision_logs_numeric_player_id() {
    let mut mgr = player_mgr();
    let (tx, _rx) = mpsc::channel(32);
    let engine = ChainEngine::new();
    let capture = LogCapture::install();
    handle_base_message(
        BaseToCellMsg::UpdateBandolierItem {
            entity_id: 1,
            slot_id: 0,
            item: pistol(),
            make_active: true,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;
    let row = capture
        .find_message(Level::INFO, "UpdateBandolierItem: equip-display decision")
        .expect("decision row");
    assert_numeric_identity(&row);
}

/// Same for `SyncBandolierItems: equip-display decision`.
#[tokio::test]
async fn sync_bandolier_decision_logs_numeric_player_id() {
    let mut mgr = player_mgr();
    let (tx, _rx) = mpsc::channel(32);
    let engine = ChainEngine::new();
    let capture = LogCapture::install();
    handle_base_message(
        BaseToCellMsg::SyncBandolierItems {
            entity_id: 1,
            active_bandolier_slot: 0,
            bandolier_items: vec![(0, pistol())],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;
    let row = capture
        .find_message(Level::INFO, "SyncBandolierItems: equip-display decision")
        .expect("decision row");
    assert_numeric_identity(&row);
}

/// `Item granted to player` names the design id under `design_id` (the key
/// the loot and base grant rows share) and carries the player's identity,
/// which it had neither of before.
#[tokio::test]
async fn item_granted_row_carries_design_id_and_identity() {
    let mut mgr = player_mgr();
    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let capture = LogCapture::install();
    handle_base_message(
        BaseToCellMsg::InventoryItemGranted {
            entity_id: 1,
            item_id: 5224,
            container_id: 1,
            slot_id: 4,
            quantity: 2,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;
    let row = capture
        .find_message(Level::DEBUG, "Item granted to player")
        .expect("granted row");
    assert!(row.has_field("design_id", "5224"), "{row:#?}");
    assert!(row.has_field("item_type_id", "5224"), "{row:#?}");
    assert_numeric_identity(&row);
}
