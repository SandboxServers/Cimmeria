//! Cell→base atomic-commit handoff guards.
//!
//! The first test walks the full state machine to `LockedAndConfirmed`
//! on both sides and confirms exactly one `ExecuteTrade` lands on the
//! channel. The second is the regression guard for the distance
//! recheck at the handoff entry — the trust-boundary fix proved that
//! we can't rely on the lock-state handler's distance check alone.

use tokio::sync::mpsc;

use cimmeria_entity::trade::{
    TradeProposal, ETRADELOCKSTATE_LOCKED, ETRADELOCKSTATE_LOCKED_AND_CONFIRMED,
    ETRADERESULTS_CANCELLED,
};

use crate::cell::cell_methods::player::constants::{
    TRADE_LOCK_STATE, TRADE_REQUEST, TRADE_UPDATE_PROPOSAL,
};
use crate::cell::client_methods::player::ON_TRADE_RESULTS;
use crate::cell::messages::CellToBaseMsg;
use crate::test_support::make_space_manager;

use super::super::dispatch;
use super::super::handoff::request_execute_trade;
use super::{build_trade_request_args, make_two_players};

#[tokio::test]
async fn both_confirm_triggers_execute_trade_hand_off() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

    let (tx, mut rx) = mpsc::channel(64);
    // Open session.
    dispatch(
        1,
        TRADE_REQUEST,
        &build_trade_request_args(
            2,
            &TradeProposal {
                version: 1,
                ..Default::default()
            },
        ),
        &tx,
        &mut mgr,
    )
    .await;
    // Partner pushes a proposal too so both versions are at 1.
    dispatch(
        2,
        TRADE_UPDATE_PROPOSAL,
        &build_trade_request_args(
            1,
            &TradeProposal {
                version: 1,
                ..Default::default()
            },
        ),
        &tx,
        &mut mgr,
    )
    .await;
    while rx.try_recv().is_ok() {} // drain

    // Both sides walk: None → Locked → LockedAndConfirmed.
    for actor in [1u32, 2u32] {
        let mut args = Vec::new();
        args.extend_from_slice(&1i32.to_le_bytes());
        args.extend_from_slice(&1i32.to_le_bytes());
        args.push(ETRADELOCKSTATE_LOCKED as u8);
        dispatch(actor, TRADE_LOCK_STATE, &args, &tx, &mut mgr).await;
    }
    for actor in [1u32, 2u32] {
        let mut args = Vec::new();
        args.extend_from_slice(&1i32.to_le_bytes());
        args.extend_from_slice(&1i32.to_le_bytes());
        args.push(ETRADELOCKSTATE_LOCKED_AND_CONFIRMED as u8);
        dispatch(actor, TRADE_LOCK_STATE, &args, &tx, &mut mgr).await;
    }

    // ExecuteTrade should have been emitted exactly once.
    let mut executes = 0;
    while let Ok(msg) = rx.try_recv() {
        if matches!(msg, CellToBaseMsg::ExecuteTrade { .. }) {
            executes += 1;
        }
    }
    assert_eq!(
        executes, 1,
        "both-confirmed should trigger exactly one ExecuteTrade hand-off"
    );

    // Cell-side state is cleared regardless of base outcome.
    assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
    assert!(mgr.get_entity(2).unwrap().trade_partner_entity_id.is_none());
}

/// Regression guard for the security review on the trading PR.
///
/// `request_execute_trade` is the very last cell-side checkpoint
/// before the swap commits at base. The earlier distance checks
/// (`tradeUpdateProposal`, `tradeLockState`) fire on every inbound
/// message — but a small window exists between the `tradeLockState`
/// handler's distance check (top of the handler) and the actual
/// `request_execute_trade` call (bottom): any future refactor that
/// introduces an `.await` between those points opens the gap for
/// one player to walk out of range while the second confirmation
/// is being applied. The handoff must independently verify, not
/// trust the earlier check.
///
/// We exercise the **handoff entry point directly** (not via
/// `dispatch`) because the `tradeLockState` handler's own distance
/// check would otherwise intercept the out-of-range mutation
/// before `request_execute_trade` ever ran — that would test the
/// handler check, not the handoff check. The point of this guard
/// is the *handoff* check.
///
/// This test:
/// 1. Manually sets up both sides as if they had reached
///    `LockedAndConfirmed` (the precondition that normally calls
///    `request_execute_trade`).
/// 2. Moves player 2 out of range.
/// 3. Calls `request_execute_trade` directly.
/// 4. Asserts no `ExecuteTrade` was queued, both sides got
///    `onTradeResults(Cancelled)`, and both sides have their
///    trade state cleared.
///
/// Revert-verifier: removing the `partners_in_range` block at the
/// top of `request_execute_trade` causes an `ExecuteTrade` to
/// appear in the channel even though the players walked apart.
#[tokio::test]
async fn execute_handoff_rechecks_distance_and_cancels_on_break() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

    // Hand-roll the "both sides at LockedAndConfirmed" precondition
    // without going through the lock-state handler (which would
    // intercept our position mutation via its own distance check
    // and we want to exercise the handoff check, not the handler).
    if let Some(e) = mgr.get_entity_mut(1) {
        e.trade_partner_entity_id = Some(2);
        e.trade_proposal = Some(TradeProposal {
            version: 1,
            items: vec![],
            cash: 0,
            lock_state: ETRADELOCKSTATE_LOCKED_AND_CONFIRMED,
        });
    }
    if let Some(e) = mgr.get_entity_mut(2) {
        e.trade_partner_entity_id = Some(1);
        e.trade_proposal = Some(TradeProposal {
            version: 1,
            items: vec![],
            cash: 0,
            lock_state: ETRADELOCKSTATE_LOCKED_AND_CONFIRMED,
        });
    }

    // Now walk player 2 far out of range — the proximity invariant
    // is broken at exactly the moment the handoff is about to run.
    if let Some(e) = mgr.get_entity_mut(2) {
        e.position = cimmeria_common::Vector3::new(1000.0, 0.0, 0.0);
    }

    let (tx, mut rx) = mpsc::channel(64);
    // Drive the handoff path directly — this is the function the
    // lock-state handler calls when both sides reach
    // LockedAndConfirmed.
    request_execute_trade(1, 2, &tx, &mut mgr).await;

    // Collect what landed in the channel.
    let mut executes = 0;
    let mut results: Vec<(u32, i32)> = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::ExecuteTrade { .. } => executes += 1,
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } if method_index == ON_TRADE_RESULTS => {
                let result = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                results.push((entity_id, result));
            }
            _ => {}
        }
    }

    assert_eq!(
        executes, 0,
        "request_execute_trade MUST NOT hand off to base when \
         partners are out of range at the handoff. Saw ExecuteTrade \
         in the channel — distance recheck missing."
    );
    // Both sides see Cancelled, not Completed: distance-break is a
    // fault path, same convention as the disconnect cancel.
    assert_eq!(results.len(), 2, "both sides must receive onTradeResults");
    assert!(
        results.iter().all(|(_, r)| *r == ETRADERESULTS_CANCELLED),
        "distance-break at handoff must send Cancelled (2), \
         not Completed (1). Got: {results:?}"
    );
    // Cell state is gone on both sides — same teardown as any
    // other cancel path.
    assert!(
        mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none(),
        "entity 1's trade_partner_entity_id must be cleared"
    );
    assert!(
        mgr.get_entity(2).unwrap().trade_partner_entity_id.is_none(),
        "entity 2's trade_partner_entity_id must be cleared"
    );
    assert!(mgr.get_entity(1).unwrap().trade_proposal.is_none());
    assert!(mgr.get_entity(2).unwrap().trade_proposal.is_none());
}
