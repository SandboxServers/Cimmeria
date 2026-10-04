//! AB-T2: the client packet's Mercury seq reaches the `useAbility` receipt
//! row through the whole cell path: `BaseToCellMsg::CellMethodCall`'s
//! `packet_seq`, `handle_base_message`, the router and the player dispatch.
//! Dropping it anywhere on the way (say the router passing `None`) leaves
//! the row without `mercury_seq` and fails this guard.

use super::*;
use crate::test_support::LogCapture;

#[tokio::test]
async fn a_cell_method_calls_packet_seq_reaches_the_use_ability_receipt_row() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(1).unwrap();
    e.is_player = true;
    e.player_id = Some(72);
    e.account_id = Some(6);
    mgr.connect_entity(1);
    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let mut args = 7i32.to_le_bytes().to_vec();
    args.extend_from_slice(&42i32.to_le_bytes());
    let logs = LogCapture::install();

    let call = BaseToCellMsg::CellMethodCall {
        entity_id: 1,
        method_index: crate::cell::cell_methods::player::USE_ABILITY,
        args,
        packet_seq: Some(4242),
    };
    handle_base_message(call, &tx, &mut mgr, &engine, &[]).await;

    let all = logs.all();
    let recv = all
        .iter()
        .find(|c| c.has_field("event", "use_ability_recv"))
        .unwrap_or_else(|| panic!("the receipt row: {all:#?}"));
    assert!(recv.has_field("mercury_seq", "4242"), "{recv:?}");
    assert!(recv.has_field("player_id", "72"), "{recv:?}");
}
