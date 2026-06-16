//! SGWGmPlayer tail (109+) routing through the gate (CAT-N-04).
//!
//! End-to-end dispatch tests that exercise the GM gate for the SGWGmPlayer
//! own cell-method tail (flattened index >= 109): non-GM rejection, GM
//! pass-through to an implemented handler, and the authorized-but-
//! unimplemented fall-through.

use tokio::sync::mpsc;

use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::*;

fn make_test_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

/// **negative case + regression guard.** A non-GM caller (access_level
/// 0) sending an SGWGmPlayer cell method (>= 109) must be rejected at the
/// dispatch gate BEFORE any handler runs: the rejection `warn!` fires, an
/// `onErrorCode` goes back, and the gm handler never executes (so no
/// GrantItem / TeleportPlayer side effect leaks onto the wire).
///
/// We use `gmGiveItem` (133) with valid args — if the gate were reverted, the
/// handler WOULD run and emit a `GrantItem`. The assertion that no `GrantItem`
/// appears (only the `onErrorCode`) is the revert-verifier: drop the
/// `index >= 109` arm in `requires_gm` and a non-GM reaches the handler,
/// failing this test.
#[tokio::test]
async fn gm_tail_method_rejected_for_non_gm_caller() {
    use crate::test_support::LogCapture;

    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.player_id = Some(100);
        e.access_level = 0; // Player — NOT a GM
    }

    let capture = LogCapture::install();
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    // gmGiveItem(133): WSTRING "1234" + INT32 qty=1 — well-formed, so the only
    // reason it wouldn't grant is the gate intercepting it.
    let mut args = Vec::new();
    crate::mercury::write_wstring(&mut args, "1234");
    args.extend_from_slice(&1i32.to_le_bytes());

    dispatch_cell_method(
        1,
        crate::cell::cell_methods::gm::GM_GIVE_ITEM,
        &args,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    // Rejection audit log with the gm* index.
    let event = capture
        .find_message(tracing::Level::WARN, "GM-gated cell method rejected")
        .expect("non-GM call to a 109+ gm method must be rejected by the gate");
    assert!(event.has_field("method_index", "133"));
    assert!(event.has_field("access_level", "0"));

    // Exactly one wire message — the onErrorCode. NO GrantItem (the handler
    // never ran). This is the revert-verifier assertion.
    let msg = rx
        .try_recv()
        .expect("rejection must send an onErrorCode response");
    match msg {
        CellToBaseMsg::EntityMethodCall { method_index, .. } => {
            assert_eq!(method_index, 121, "onErrorCode expected");
        }
        other => panic!("expected onErrorCode EntityMethodCall, got {other:?}"),
    }
    assert!(
        rx.try_recv().is_err(),
        "a rejected gm call must produce ONLY the onErrorCode — no GrantItem \
         side effect. A GrantItem here means a non-GM reached the handler \
         (the `index >= 109` gate arm was reverted)."
    );
}

/// **positive case.** A GM (access_level 2) sending an implemented gm*
/// index (gmGiveItem 133) passes the gate and the handler executes — the
/// observable proof is a `GrantItem` on the wire (and no onErrorCode).
#[tokio::test]
async fn gm_tail_method_executes_for_gm_caller() {
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.player_id = Some(100);
        e.access_level = 2; // GameMaster
    }

    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    let mut args = Vec::new();
    crate::mercury::write_wstring(&mut args, "1234");
    args.extend_from_slice(&3i32.to_le_bytes());

    dispatch_cell_method(
        1,
        crate::cell::cell_methods::gm::GM_GIVE_ITEM,
        &args,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let mut saw_grant = false;
    let mut saw_error = false;
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::GrantItem { item_id, count, .. } => {
                assert_eq!(item_id, 1234);
                assert_eq!(count, 3);
                saw_grant = true;
            }
            CellToBaseMsg::EntityMethodCall {
                method_index: 121, ..
            } => {
                saw_error = true;
            }
            other => panic!("unexpected wire message in gmGiveItem happy-path: {other:?}"),
        }
    }
    assert!(saw_grant, "authorized gmGiveItem must emit GrantItem");
    assert!(!saw_error, "authorized gm call must not emit onErrorCode");
}

/// A GM sending an UNimplemented 109+ index (gmSetGodMode 142) passes the gate
/// and hits the auth-gated router fall-through without panic and without any
/// stray side-effect message.
#[tokio::test]
async fn gm_tail_unimplemented_index_falls_through_without_panic() {
    let mut mgr = make_test_space_mgr();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.player_id = Some(100);
        e.access_level = 2; // GameMaster
    }

    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    // 142 = gmSetGodMode — gated (>= 109) but no handler wired yet.
    dispatch_cell_method(1, 142, &[1u8], &tx, &mut mgr, &engine).await;

    // No onErrorCode (gate passed), no side-effect message (no handler).
    assert!(
        rx.try_recv().is_err(),
        "an authorized-but-unimplemented gm index must produce no wire message"
    );
}
