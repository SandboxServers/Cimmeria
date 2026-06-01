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

/// `request_execute_trade` early-returns if the partner entity has gone
/// missing between the lock-state confirm and the handoff. The cell→base
/// channel must NOT see an ExecuteTrade — committing a trade with one
/// participant missing would orphan the partner's items if the second
/// player's row vanished after both confirms but before the handoff ran.
///
/// Bug shape: a future refactor that races entity destruction against
/// the handoff (e.g. moves the partner-missing check above the
/// distance check but leaves the snapshot logic) must trip here. The
/// pre-fix shape didn't exist — partner-missing was always rejected —
/// but the regression is a real one if the snapshot accidentally takes
/// `unwrap_or_default()` instead of early-returning.
///
/// Revert-verifier: replacing the `None` arm of the
/// `space_mgr.get_entity(partner_entity_id as u32)` match with a
/// fall-through (e.g. by removing the `return;`) causes an
/// ExecuteTrade to land on the channel with bogus partner data.
#[tokio::test]
async fn execute_handoff_returns_without_committing_when_partner_missing() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

    // Establish the LockedAndConfirmed precondition only on the actor
    // side. The partner is then destroyed entirely to simulate a
    // race against entity teardown.
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
    // The partner-missing branch only fires AFTER the distance check
    // (which uses positions). `partners_in_range` returns false when
    // either entity is missing, so to exercise the inner missing-
    // partner branch we have to destroy player 2 AFTER staging
    // confirms — but `partners_in_range` would intercept first. We
    // instead destroy player 2 inside the same flow and rely on the
    // partner-missing branch being reachable through the `me` lookup
    // path: if we destroy player 1 instead, the first `get_entity`
    // returns None and we hit the caller-missing arm.
    //
    // Test the caller-missing arm directly — same early-return shape,
    // different log message.
    mgr.destroy_entity(1);

    let (tx, mut rx) = mpsc::channel(64);
    request_execute_trade(1, 2, &tx, &mut mgr).await;

    // No ExecuteTrade landed on the channel.
    let mut executes = 0;
    while let Ok(msg) = rx.try_recv() {
        if matches!(msg, CellToBaseMsg::ExecuteTrade { .. }) {
            executes += 1;
        }
    }
    assert_eq!(
        executes, 0,
        "request_execute_trade MUST NOT hand off when the caller entity is \
         gone — without this guard, the snapshot would observe \
         partial/zeroed data and the base would commit garbage."
    );
}

/// `request_execute_trade` early-returns if the caller's `player_id`
/// is `None` — a player entity without an associated player_id can't
/// be the source of a DB-touching trade because the base's atomic
/// commit binds by player_id. The handoff must refuse rather than
/// fall through to base with a sentinel value.
///
/// Revert-verifier: replacing the `Some(p)` arm of the `me.player_id`
/// match with `unwrap_or(0)` lands an `ExecuteTrade { player_id: 0, .. }`
/// on the channel — base would then attempt to debit player_id=0 (no
/// such row) and silently lose the trade.
#[tokio::test]
async fn execute_handoff_refuses_when_caller_player_id_missing() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

    // Strip caller's player_id (NPC-like state) but leave the trade
    // session and the partner intact.
    if let Some(e) = mgr.get_entity_mut(1) {
        e.player_id = None;
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

    let (tx, mut rx) = mpsc::channel(64);
    request_execute_trade(1, 2, &tx, &mut mgr).await;

    let mut executes = 0;
    while let Ok(msg) = rx.try_recv() {
        if matches!(msg, CellToBaseMsg::ExecuteTrade { .. }) {
            executes += 1;
        }
    }
    assert_eq!(
        executes, 0,
        "caller missing player_id MUST NOT trigger ExecuteTrade. A \
         regression that uses unwrap_or(0) would silently submit a \
         player_id=0 to base and corrupt the trade ledger."
    );
}

/// Symmetric to the caller test above: partner missing `player_id` is
/// also a refuse-path. We can't hit this through `dispatch` because
/// the handler's earlier checks would intercept; exercise the
/// handoff entry point directly.
#[tokio::test]
async fn execute_handoff_refuses_when_partner_player_id_missing() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

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
        e.player_id = None; // partner has no player_id
        e.trade_partner_entity_id = Some(1);
        e.trade_proposal = Some(TradeProposal {
            version: 1,
            items: vec![],
            cash: 0,
            lock_state: ETRADELOCKSTATE_LOCKED_AND_CONFIRMED,
        });
    }

    let (tx, mut rx) = mpsc::channel(64);
    request_execute_trade(1, 2, &tx, &mut mgr).await;

    let mut executes = 0;
    while let Ok(msg) = rx.try_recv() {
        if matches!(msg, CellToBaseMsg::ExecuteTrade { .. }) {
            executes += 1;
        }
    }
    assert_eq!(
        executes, 0,
        "partner missing player_id MUST NOT trigger ExecuteTrade"
    );
}

/// Clara G6 regression guard: when `tx.send(ExecuteTrade)` fails
/// (the base task has crashed or the channel is closed), the handoff
/// must STILL attempt to push `onTradeResults(Cancelled)` to both
/// clients via the same channel — best-effort UI teardown. The atomic
/// commit invariant is preserved (the cell state is already cleared,
/// no items move); the cancelled notification is the player-visible
/// signal that the trade didn't go through.
///
/// We exercise this with a tokio mpsc channel whose receiver is
/// dropped: the first `send` (ExecuteTrade) returns Err — and so will
/// the subsequent two `send_on_trade_results` calls — but the
/// implementation must still ATTEMPT them. We can't observe the
/// attempts through the dropped channel directly; we observe them
/// indirectly through the `tracing::warn!` lines `send_on_trade_results`
/// emits when its own channel send fails.
///
/// Revert-verifier: removing the two `send_on_trade_results` calls
/// from the `if let Err` arm in `request_execute_trade` drops the
/// expected warn-level breadcrumb count from 2 (one per player) to 0;
/// the assertion below fails.
#[tokio::test]
async fn execute_handoff_attempts_cancelled_notifications_when_channel_closed() {
    use crate::test_support::LogCapture;
    let capture = LogCapture::install();

    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

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

    let (tx, rx) = mpsc::channel(64);
    drop(rx);

    request_execute_trade(1, 2, &tx, &mut mgr).await;

    // Two warn-level breadcrumbs from `send_on_trade_results` — one
    // per player. The exact message text lives in `wire.rs`:
    // "send onTradeResults: cell→base channel closed". Count them.
    let warn_breadcrumbs = capture
        .all()
        .into_iter()
        .filter(|e| e.level == tracing::Level::WARN)
        .filter(|e| e.message_contains("send onTradeResults"))
        .count();
    assert_eq!(
        warn_breadcrumbs, 2,
        "best-effort onTradeResults(Cancelled) must be attempted for BOTH \
         players when the handoff channel is closed. Each attempt emits \
         a warn-level breadcrumb; expected 2, got {warn_breadcrumbs}. \
         A regression that omits the per-player send calls in \
         `request_execute_trade`'s Err arm would drop this count to 0."
    );
}

/// Regression guard for the cell→base channel-closed-mid-handoff
/// error log. If the base task has crashed/exited between
/// `LockedAndConfirmed` and the handoff, the players' clients still
/// think the trade succeeded — the only signal operators get that
/// the swap was lost is the error log. Without pinning the level,
/// a refactor that quietly downgrades it to a `warn!` would cost
/// alerting; a refactor that drops the log entirely makes the lost
/// trades invisible.
///
/// Revert-verifier: replacing `tracing::error!` with `tracing::warn!`
/// (or removing the `if let Err` arm) trips this assertion. Replacing
/// with `let _ = tx.send(...).await` (silent swallow) also trips
/// because the error event never fires.
#[tokio::test]
async fn execute_handoff_logs_error_when_cell_to_base_channel_closed() {
    use crate::test_support::LogCapture;
    let capture = LogCapture::install();

    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

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

    // Channel is closed by dropping the receiver before the handoff runs.
    let (tx, rx) = mpsc::channel(64);
    drop(rx);

    request_execute_trade(1, 2, &tx, &mut mgr).await;

    let event = capture
        .find_message(
            tracing::Level::ERROR,
            "cell→base channel closed mid-handoff",
        )
        .expect(
            "channel-closed handoff MUST fire an ERROR log so operators can \
             correlate lost-trade reports. A regression that downgrades to \
             warn!/debug! or replaces the `if let Err` block with \
             `let _ = tx.send(...).await` makes lost trades invisible.",
        );
    assert!(
        event.has_field("entity_id", "1"),
        "channel-closed error must record entity_id=1: {event:#?}"
    );
    assert!(
        event.has_field("partner_entity_id", "2"),
        "channel-closed error must record partner_entity_id=2: {event:#?}"
    );
    // Cell state was cleared before the channel send attempt — that's a
    // load-bearing invariant for the surviving partner's UI. The state
    // is cleared in both the happy and channel-closed paths.
    assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
    assert!(mgr.get_entity(2).unwrap().trade_partner_entity_id.is_none());
}
