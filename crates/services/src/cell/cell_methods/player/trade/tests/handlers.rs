//! Handler-level happy-path + validation-reject coverage for the four
//! inbound trade methods (104-107).

use tokio::sync::mpsc;

use cimmeria_entity::trade::{
    TradeItem, TradeProposal, ETRADELOCKSTATE_NONE, ETRADERESULTS_CANCELLED,
    ETRADERESULTS_COMPLETED,
};

use crate::cell::cell_methods::player::constants::{
    TRADE_LOCK_STATE, TRADE_REQUEST, TRADE_REQUEST_CANCEL, TRADE_UPDATE_PROPOSAL,
};
use crate::cell::client_methods::player::{ON_TRADE_RESULTS, ON_TRADE_STATE};
use crate::cell::messages::CellToBaseMsg;
use crate::test_support::make_space_manager;

// Reach back to the sibling modules of `tests/` inside `trade/`.
use super::super::dispatch;
use super::super::state::cancel_trade_on_disconnect;

use super::{build_trade_request_args, make_two_players};

#[tokio::test]
async fn trade_request_opens_session_on_both_sides() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

    let (tx, mut rx) = mpsc::channel(16);
    let args = build_trade_request_args(
        2,
        &TradeProposal {
            version: 1,
            items: vec![],
            cash: 0,
            lock_state: ETRADELOCKSTATE_NONE,
        },
    );
    assert!(dispatch(1, TRADE_REQUEST, &args, &tx, &mut mgr).await);
    // Both sides should now be wired into the session.
    assert_eq!(mgr.get_entity(1).unwrap().trade_partner_entity_id, Some(2));
    assert_eq!(mgr.get_entity(2).unwrap().trade_partner_entity_id, Some(1));
    // And both clients should have received an onTradeState (one method
    // call per side).
    let mut on_state_count = 0;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            if method_index == ON_TRADE_STATE {
                on_state_count += 1;
            }
        }
    }
    assert_eq!(on_state_count, 2, "onTradeState must fire to both players");
}

#[tokio::test]
async fn trade_request_too_far_rejects_with_cancelled_to_caller_only() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 100.0); // way outside MAX_INTERACT_DISTANCE=5

    let (tx, mut rx) = mpsc::channel(8);
    let args = build_trade_request_args(2, &TradeProposal::default());
    dispatch(1, TRADE_REQUEST, &args, &tx, &mut mgr).await;
    // No session opened.
    assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
    assert!(mgr.get_entity(2).unwrap().trade_partner_entity_id.is_none());
    // Only the caller gets onTradeResults(Cancelled).
    let mut events = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        events.push(msg);
    }
    assert_eq!(events.len(), 1, "out-of-range rejects with one message");
    match &events[0] {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(*entity_id, 1);
            assert_eq!(*method_index, ON_TRADE_RESULTS);
            // INT32 result, expect Cancelled (2)
            assert_eq!(&args[4..8], &ETRADERESULTS_CANCELLED.to_le_bytes());
        }
        other => panic!("expected EntityMethodCall, got {other:?}"),
    }
}

/// TRAP #3 regression guard: QA 0.8384 client never sends `tradeRequest` —
/// it jumps straight to `tradeUpdateProposal`. The handler must
/// implicitly open the session.
#[tokio::test]
async fn trade_update_proposal_without_prior_request_begins_trading() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

    let (tx, _rx) = mpsc::channel(16);
    let proposal = TradeProposal {
        version: 1,
        items: vec![],
        cash: 0,
        lock_state: ETRADELOCKSTATE_NONE,
    };
    let args = build_trade_request_args(2, &proposal); // same layout — int32 partner + LocalTradeProposal

    // Pre-condition: nobody is trading.
    assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());

    // QA-client path: send TRADE_UPDATE_PROPOSAL with no open session.
    // The handler MUST open the session itself (per Python
    // SGWPlayer.py:1785-1790 workaround). Without this, every QA
    // client trade attempt silently no-ops.
    let handled = dispatch(1, TRADE_UPDATE_PROPOSAL, &args, &tx, &mut mgr).await;
    assert!(handled);

    // The session is now open on both sides — that's the assertion that
    // distinguishes "QA workaround applied" from "method silently rejected
    // because no session existed."
    assert_eq!(
        mgr.get_entity(1).unwrap().trade_partner_entity_id,
        Some(2),
        "tradeUpdateProposal with no prior tradeRequest MUST implicitly \
         begin a session (QA 0.8384 client never sends tradeRequest — \
         see SGWPlayer.py:1785-1790 workaround comment)"
    );
    assert_eq!(mgr.get_entity(2).unwrap().trade_partner_entity_id, Some(1));
}

#[tokio::test]
async fn cancel_clears_state_on_both_sides_and_sends_completed_to_both() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

    let (tx, mut rx) = mpsc::channel(64);
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
    // Drain the initial onTradeState packets.
    while rx.try_recv().is_ok() {}

    let mut cancel_args = Vec::new();
    cancel_args.extend_from_slice(&2i32.to_le_bytes());
    dispatch(1, TRADE_REQUEST_CANCEL, &cancel_args, &tx, &mut mgr).await;

    // State cleared on both sides.
    assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
    assert!(mgr.get_entity(1).unwrap().trade_proposal.is_none());
    assert!(mgr.get_entity(2).unwrap().trade_partner_entity_id.is_none());
    assert!(mgr.get_entity(2).unwrap().trade_proposal.is_none());

    // Both clients get onTradeResults(Completed).
    // TRAP #2: user-initiated cancel sends Completed (1), NOT Cancelled.
    let mut results = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == ON_TRADE_RESULTS {
                let result = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                results.push(result);
            }
        }
    }
    assert_eq!(results.len(), 2, "both players must receive onTradeResults");
    assert!(
        results.iter().all(|&r| r == ETRADERESULTS_COMPLETED),
        "user cancel MUST send Completed (1), not Cancelled (2) — \
         see Trade.py:225-228. Got results: {results:?}"
    );
}

#[tokio::test]
async fn distance_break_during_update_auto_cancels() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

    let (tx, mut rx) = mpsc::channel(64);
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
    while rx.try_recv().is_ok() {}

    // Move player 2 far away.
    if let Some(e) = mgr.get_entity_mut(2) {
        e.position = cimmeria_common::Vector3::new(1000.0, 0.0, 0.0);
    }

    // Next tradeUpdateProposal — must auto-cancel.
    let next = TradeProposal {
        version: 2,
        items: vec![TradeItem {
            instance_id: 42,
            slot_id: 0,
        }],
        cash: 0,
        lock_state: ETRADELOCKSTATE_NONE,
    };
    dispatch(
        1,
        TRADE_UPDATE_PROPOSAL,
        &build_trade_request_args(2, &next),
        &tx,
        &mut mgr,
    )
    .await;

    // Both sides cleared, both clients got onTradeResults(Cancelled=2)
    // (distance break is NOT a user-initiated cancel).
    assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
    let mut last_results = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == ON_TRADE_RESULTS {
                let result = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                last_results.push(result);
            }
        }
    }
    assert_eq!(last_results.len(), 2);
    assert!(
        last_results.iter().all(|&r| r == ETRADERESULTS_CANCELLED),
        "distance-break MUST send Cancelled (2), not Completed (1). Got: {last_results:?}"
    );
}

#[tokio::test]
async fn proposal_version_must_increment_by_one() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

    let (tx, _rx) = mpsc::channel(64);
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
    assert_eq!(
        mgr.get_entity(1)
            .unwrap()
            .trade_proposal
            .as_ref()
            .unwrap()
            .version,
        1
    );

    // version=5 instead of 2 → rejected → cancels the session.
    let bad = TradeProposal {
        version: 5,
        items: vec![],
        cash: 0,
        lock_state: ETRADELOCKSTATE_NONE,
    };
    dispatch(
        1,
        TRADE_UPDATE_PROPOSAL,
        &build_trade_request_args(2, &bad),
        &tx,
        &mut mgr,
    )
    .await;
    // Session is torn down.
    assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
}

#[tokio::test]
async fn cannot_trade_with_self() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);
    let (tx, _rx) = mpsc::channel(8);
    let args = build_trade_request_args(1, &TradeProposal::default()); // partner = self
    dispatch(1, TRADE_REQUEST, &args, &tx, &mut mgr).await;
    assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
}

/// `begin_trading` must reject when the partner is already in another
/// trade session. Without this check, a popular player could be
/// trade-hijacked: a malicious client could spam tradeRequest against
/// a target already mid-trade with a different player, racing to be
/// the one whose session sticks.
///
/// Revert-verifier: removing the `partner.trade_partner_entity_id.is_some()`
/// check causes the caller's session to overwrite the partner's existing
/// partner pointer — the test detects this by asserting the partner's
/// original trade_partner_entity_id stayed pointing at the original
/// caller, not the new one.
#[tokio::test]
async fn cannot_trade_with_partner_already_in_session() {
    let mut mgr = make_space_manager();
    // Three players: entities 1, 2, 3 — all in range of each other.
    make_two_players(&mut mgr, 1, 2, 2.0);
    mgr.create_entity(3, "Agnos", [1.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(3) {
        e.is_player = true;
        e.player_id = Some(3000);
    }

    let (tx, _rx) = mpsc::channel(64);
    // Player 1 opens a trade with player 2.
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
    assert_eq!(mgr.get_entity(2).unwrap().trade_partner_entity_id, Some(1));

    // Now player 3 tries to trade with player 2 — must be rejected.
    dispatch(
        3,
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

    // Player 2's session is STILL with player 1, not hijacked to player 3.
    assert_eq!(
        mgr.get_entity(2).unwrap().trade_partner_entity_id,
        Some(1),
        "partner's session must not be hijacked by a second tradeRequest. \
         A revert that omits the partner-already-trading check would \
         overwrite this to Some(3)."
    );
    // Player 3 has no session.
    assert!(
        mgr.get_entity(3).unwrap().trade_partner_entity_id.is_none(),
        "rejected caller must NOT have a trade session opened"
    );
}

/// `begin_trading` must reject when the caller is already mid-trade.
/// A client that's mid-trade with player A and sends tradeRequest for
/// player B should keep the original session intact, not get a second
/// concurrent session.
#[tokio::test]
async fn cannot_open_second_trade_while_already_trading() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);
    mgr.create_entity(3, "Agnos", [1.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(3) {
        e.is_player = true;
        e.player_id = Some(3000);
    }

    let (tx, _rx) = mpsc::channel(64);
    // Player 1 opens a trade with player 2.
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
    assert_eq!(mgr.get_entity(1).unwrap().trade_partner_entity_id, Some(2));

    // Now player 1 tries to ALSO trade with player 3.
    dispatch(
        1,
        TRADE_REQUEST,
        &build_trade_request_args(
            3,
            &TradeProposal {
                version: 1,
                ..Default::default()
            },
        ),
        &tx,
        &mut mgr,
    )
    .await;

    // Player 1's original session with player 2 must be intact.
    assert_eq!(
        mgr.get_entity(1).unwrap().trade_partner_entity_id,
        Some(2),
        "caller's original session must not be hijacked by a second \
         tradeRequest. Revert of caller-already-trading check would \
         overwrite this to Some(3)."
    );
    // Player 3 has no session — wasn't pulled in.
    assert!(mgr.get_entity(3).unwrap().trade_partner_entity_id.is_none());
}

#[tokio::test]
async fn cannot_trade_with_npc() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);
    // Demote entity 2 to a non-player.
    if let Some(e) = mgr.get_entity_mut(2) {
        e.is_player = false;
    }
    let (tx, _rx) = mpsc::channel(8);
    let args = build_trade_request_args(2, &TradeProposal::default());
    dispatch(1, TRADE_REQUEST, &args, &tx, &mut mgr).await;
    assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());
}

/// Clara G12 regression guard: `begin_trading` must reject when the
/// CALLER is not a player, not just when the partner isn't. Without
/// the symmetric `is_player` gate, an NPC entity that ever gets a
/// `player_id` assigned (content-engine bug, future hybrid entity type)
/// could open a trade session server-side from the caller direction.
/// The partner check is structural; the caller check has to be
/// enforced here because cell-method dispatch doesn't structurally
/// guarantee the source entity is a player.
///
/// Revert-verifier: removing the `if !me.is_player { return false; }`
/// block in `begin_trading` lets this test pass when it shouldn't —
/// the partner-entity gets wired into a trade with an NPC caller.
#[tokio::test]
async fn cannot_initiate_trade_as_npc() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);
    // Demote the CALLER (entity 1) to a non-player. Partner is still a player.
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = false;
    }
    let (tx, _rx) = mpsc::channel(8);
    let args = build_trade_request_args(2, &TradeProposal::default());
    dispatch(1, TRADE_REQUEST, &args, &tx, &mut mgr).await;
    assert!(
        mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none(),
        "non-player caller MUST NOT open a trade session — without the \
         symmetric is_player check, an NPC with a player_id could become \
         the source of a trade. Revert the `if !me.is_player` guard in \
         begin_trading() to fail this test."
    );
    assert!(
        mgr.get_entity(2).unwrap().trade_partner_entity_id.is_none(),
        "partner must NOT be wired into a trade session when the caller \
         was rejected"
    );
}

/// Clara G10 regression guard: a proposal containing the same item
/// instance twice gets the second occurrence silently dropped (Python
/// `Trade.py:53-58` parity). The cell-side dedup is the first line of
/// defense; base-side `lock_items` rejects duplicates with
/// `TradeAbort::DuplicateInstance` as a second line.
///
/// The behavior the client sees: the actor's proposal claims
/// `[A, A]` but the actor's saved state contains only `[A]`. The
/// partner's `onTradeState` packet reflects the deduped list. This is
/// INTENTIONAL and matches Python — see the design note on
/// `apply_proposal` in `state.rs`.
///
/// Revert-verifier: replacing the `HashSet`-based filter with a
/// straight `clone()` causes the proposal to retain two `instance_id=42`
/// entries and trips this assertion.
#[tokio::test]
async fn apply_proposal_dedups_duplicate_instance_ids() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

    let (tx, mut rx) = mpsc::channel(64);
    // Open a session first.
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
    while rx.try_recv().is_ok() {}

    // Update proposal with two copies of the same instance id.
    let dup_proposal = TradeProposal {
        version: 2,
        items: vec![
            TradeItem {
                instance_id: 42,
                slot_id: 0,
            },
            TradeItem {
                instance_id: 42,
                slot_id: 1, // different slot but same instance
            },
            TradeItem {
                instance_id: 99,
                slot_id: 2,
            },
        ],
        cash: 0,
        lock_state: ETRADELOCKSTATE_NONE,
    };
    dispatch(
        1,
        TRADE_UPDATE_PROPOSAL,
        &build_trade_request_args(2, &dup_proposal),
        &tx,
        &mut mgr,
    )
    .await;

    let saved_items = mgr
        .get_entity(1)
        .unwrap()
        .trade_proposal
        .as_ref()
        .unwrap()
        .items
        .clone();
    assert_eq!(
        saved_items.len(),
        2,
        "duplicate instance_id=42 must be dropped — proposal claimed 3 \
         items, post-dedup we expect 2 (instance_id 42 and 99). Got \
         {saved_items:?}"
    );
    // Order is preserved (first-occurrence wins).
    assert_eq!(saved_items[0].instance_id, 42);
    assert_eq!(saved_items[0].slot_id, 0);
    assert_eq!(saved_items[1].instance_id, 99);
}

/// `cancel_trade_on_disconnect` MUST be a no-op when the disconnecting
/// entity has no open trade — otherwise every logout would spuriously
/// clear non-existent partner state and emit Cancelled to nobody.
///
/// Revert-verifier: replacing the `?` early-return on
/// `trade_partner_entity_id` with `unwrap_or(0)` would attempt to
/// cancel a trade with entity_id=0, panicking on the partner lookup
/// or sending a spurious onTradeResults to whichever entity happens
/// to have id 0.
#[tokio::test]
async fn cancel_trade_on_disconnect_returns_none_for_non_trading_entity() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);
    // Neither side is trading.
    assert!(mgr.get_entity(1).unwrap().trade_partner_entity_id.is_none());

    let (tx, mut rx) = mpsc::channel(8);
    let result = cancel_trade_on_disconnect(1, &tx, &mut mgr).await;
    assert_eq!(
        result, None,
        "no-op disconnect for non-trading entity must return None"
    );
    // No outbound message — confirms no spurious cancel was sent.
    assert!(
        rx.try_recv().is_err(),
        "disconnect of non-trading entity must NOT send any cell→base \
         message (would notify a non-existent partner)"
    );
}

/// Truncated `tradeLockState` args (less than 9 bytes) must be rejected
/// before any session lookup — otherwise the unchecked indexing
/// `args[8]` panics the cell task. Hostile/malformed packets must
/// fail closed.
///
/// Revert-verifier: removing the `args.len() < 9` early-return
/// causes a panic at `args[8]` when args is shorter than 9 bytes.
#[tokio::test]
async fn trade_lock_state_truncated_args_rejected_silently() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);
    // Open a session so the truncation path is the only thing that
    // can reject — otherwise "no session" would also reject and we
    // wouldn't be sure which branch fired.
    let (tx, _rx) = mpsc::channel(64);
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
    let before = mgr
        .get_entity(1)
        .unwrap()
        .trade_proposal
        .as_ref()
        .unwrap()
        .lock_state;

    // Send a TRADE_LOCK_STATE with 8 bytes (one short of the 9-byte minimum).
    let truncated = [0u8; 8];
    let handled = dispatch(1, TRADE_LOCK_STATE, &truncated, &tx, &mut mgr).await;
    assert!(
        handled,
        "TRADE_LOCK_STATE is still a recognised method — the truncation \
         path returns true (handled) but rejects the payload"
    );
    // Lock state unchanged.
    assert_eq!(
        mgr.get_entity(1)
            .unwrap()
            .trade_proposal
            .as_ref()
            .unwrap()
            .lock_state,
        before,
        "lock_state must not change when the args are truncated"
    );
}

#[tokio::test]
async fn disconnect_cancellation_clears_partner_state() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

    let (tx, mut rx) = mpsc::channel(16);
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
    while rx.try_recv().is_ok() {}

    // Simulate entity 1 disconnecting mid-trade.
    let partner = cancel_trade_on_disconnect(1, &tx, &mut mgr).await;
    assert_eq!(partner, Some(2));

    // Partner has been cleared, and was notified with Cancelled.
    assert!(mgr.get_entity(2).unwrap().trade_partner_entity_id.is_none());
    let mut got_partner_cancel = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            if entity_id == 2 && method_index == ON_TRADE_RESULTS {
                let result = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                if result == ETRADERESULTS_CANCELLED {
                    got_partner_cancel = true;
                }
            }
        }
    }
    assert!(
        got_partner_cancel,
        "partner must receive onTradeResults(Cancelled) on disconnect — \
         NOT Completed: the surviving player did not initiate this cancel"
    );
}
