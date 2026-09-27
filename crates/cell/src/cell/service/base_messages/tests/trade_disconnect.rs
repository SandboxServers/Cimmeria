use super::*;

/// Regression guard for the disconnect-mid-trade hook in
/// `BaseToCellMsg::DestroyEntity`. When a player disconnects while
/// mid-trade, the surviving partner MUST receive
/// `onTradeResults(Cancelled)` and have their session state cleared.
/// Without this hook the partner is left with a dangling
/// `trade_partner_entity_id` pointing at a destroyed entity — every
/// subsequent state-machine step against that ghost session crashes
/// or silently fails.
///
/// Python relied on BigWorld GC for this teardown; Rust has to hook
/// it explicitly. The hook lives in
/// `cell::cell_methods::player::trade::cancel_trade_on_disconnect`
/// and is called from BOTH `DestroyEntity` and `DisconnectEntity`
/// arms — this test pins the `DestroyEntity` arm, the next pins
/// `DisconnectEntity`.
///
/// Revert-verifier: deleting the
/// `cell_methods::player::trade::cancel_trade_on_disconnect(...).await`
/// call from the DestroyEntity arm causes the surviving partner's
/// trade_partner_entity_id to remain `Some(1)` after the destroy,
/// and no Cancelled packet to fire. Both assertions fail.
#[tokio::test]
async fn destroy_entity_cancels_in_flight_trade_with_surviving_partner() {
    use cimmeria_entity::trade::{TradeProposal, ETRADELOCKSTATE_NONE, ETRADERESULTS_CANCELLED};

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(2, "Castle_CellBlock", [1.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    // Hand-wire a Locked-and-known trade session on both sides.
    for &eid in &[1u32, 2u32] {
        if let Some(e) = mgr.get_entity_mut(eid) {
            e.is_player = true;
            e.player_id = Some(eid as i32 * 100);
            e.trade_partner_entity_id = Some(if eid == 1 { 2 } else { 1 });
            e.trade_proposal = Some(TradeProposal {
                version: 1,
                items: vec![],
                cash: 0,
                lock_state: ETRADELOCKSTATE_NONE,
            });
        }
    }

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::DestroyEntity { entity_id: 1 },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // Entity 1 is gone; entity 2 (the surviving partner) must have its
    // trade state cleared.
    assert!(mgr.get_entity(1).is_none(), "entity 1 must be destroyed");
    let survivor = mgr
        .get_entity(2)
        .expect("entity 2 (surviving partner) must still exist");
    assert!(
        survivor.trade_partner_entity_id.is_none(),
        "surviving partner's trade_partner_entity_id MUST be cleared on \
         disconnect. A regression that drops the trade-cancellation hook \
         from DestroyEntity leaves this pointing at a destroyed entity."
    );
    assert!(
        survivor.trade_proposal.is_none(),
        "surviving partner's trade_proposal must also be cleared"
    );

    // The surviving partner (entity 2) must have received an
    // onTradeResults(Cancelled) — that's the wire-level confirmation
    // their client needs to close the trade dialog. The disconnecting
    // entity (1) may or may not get one too, but we don't care — it's
    // about to be destroyed.
    use crate::cell::client_methods::player::ON_TRADE_RESULTS;
    let mut survivor_got_cancelled = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            if entity_id == 2 && method_index == ON_TRADE_RESULTS && args.len() >= 8 {
                let result = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                if result == ETRADERESULTS_CANCELLED {
                    survivor_got_cancelled = true;
                }
            }
        }
    }
    assert!(
        survivor_got_cancelled,
        "surviving partner MUST receive onTradeResults(Cancelled=2) — \
         distinct from user-initiated cancel which sends Completed (1). \
         The disconnect path is a fault path; the surviving partner needs \
         the dialog dismissed without ambiguity about whose fault it was."
    );
}

/// Symmetric guard for the `DisconnectEntity` arm. Many disconnect
/// paths reach the cell via `DisconnectEntity` first (the underlying
/// `space_mgr.disconnect_entity` then calls `destroy_entity`
/// internally), so the trade-cancellation hook must fire BEFORE that
/// internal destroy — otherwise the entity is gone by the time the
/// hook runs and the partner cleanup is a silent no-op.
///
/// Revert-verifier: removing the
/// `cancel_trade_on_disconnect(...).await` call from the
/// DisconnectEntity arm (or moving it AFTER
/// `space_mgr.disconnect_entity`) makes this test fail because the
/// surviving partner's trade state stays populated.
#[tokio::test]
async fn disconnect_entity_cancels_in_flight_trade_with_surviving_partner() {
    use cimmeria_entity::trade::{TradeProposal, ETRADELOCKSTATE_NONE, ETRADERESULTS_CANCELLED};

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    mgr.create_entity(2, "Castle_CellBlock", [1.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    for &eid in &[1u32, 2u32] {
        if let Some(e) = mgr.get_entity_mut(eid) {
            e.is_player = true;
            e.player_id = Some(eid as i32 * 100);
            e.trade_partner_entity_id = Some(if eid == 1 { 2 } else { 1 });
            e.trade_proposal = Some(TradeProposal {
                version: 1,
                items: vec![],
                cash: 0,
                lock_state: ETRADELOCKSTATE_NONE,
            });
        }
    }
    // Mark entity 1 as connected so the disconnect path actually runs.
    mgr.connect_entity(1);

    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::DisconnectEntity { entity_id: 1 },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // Entity 2 (surviving partner) must have its trade state cleared.
    let survivor = mgr
        .get_entity(2)
        .expect("entity 2 (surviving partner) must still exist");
    assert!(
        survivor.trade_partner_entity_id.is_none(),
        "DisconnectEntity arm MUST call cancel_trade_on_disconnect for \
         the disconnecting entity BEFORE the internal destroy fires. \
         If the hook is dropped or runs in the wrong order, the partner's \
         trade state stays populated (pointing at a destroyed entity)."
    );
    assert!(
        survivor.trade_proposal.is_none(),
        "surviving partner's trade_proposal must also be cleared"
    );

    use crate::cell::client_methods::player::ON_TRADE_RESULTS;
    let mut survivor_got_cancelled = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            if entity_id == 2 && method_index == ON_TRADE_RESULTS && args.len() >= 8 {
                let result = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                if result == ETRADERESULTS_CANCELLED {
                    survivor_got_cancelled = true;
                }
            }
        }
    }
    assert!(
        survivor_got_cancelled,
        "surviving partner MUST receive onTradeResults(Cancelled) via \
         the DisconnectEntity arm too"
    );
}
