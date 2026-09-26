//! `tradeLockState` (107) progression, downgrade, and value-range
//! rejection guards.

use tokio::sync::mpsc;

use cimmeria_entity::trade::{
    TradeProposal, ETRADELOCKSTATE_LOCKED, ETRADELOCKSTATE_LOCKED_AND_CONFIRMED,
    ETRADELOCKSTATE_NONE,
};

use crate::cell::cell_methods::player::constants::{
    TRADE_LOCK_STATE, TRADE_REQUEST, TRADE_UPDATE_PROPOSAL,
};
use crate::test_support::make_space_manager;

use super::super::dispatch;
use super::{build_trade_request_args, make_two_players};

#[tokio::test]
async fn lock_state_progression_none_to_locked_to_confirmed() {
    let mut mgr = make_space_manager();
    make_two_players(&mut mgr, 1, 2, 2.0);

    // Open a session with the standard tradeRequest path.
    let (tx, _rx) = mpsc::channel(64);
    let proposal = TradeProposal {
        version: 1,
        items: vec![],
        cash: 0,
        lock_state: ETRADELOCKSTATE_NONE,
    };
    dispatch(
        1,
        TRADE_REQUEST,
        &build_trade_request_args(2, &proposal),
        &tx,
        &mut mgr,
    )
    .await;
    // Partner pushes their own initial proposal too, so both
    // versions are at 1 — otherwise the "stale partner view"
    // downgrade in tradeLockState rejects our test lock attempts.
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
    // Each side starts with version=1, lock = None.
    assert_eq!(
        mgr.get_entity(1)
            .unwrap()
            .trade_proposal
            .as_ref()
            .unwrap()
            .version,
        1
    );
    assert_eq!(
        mgr.get_entity(2)
            .unwrap()
            .trade_proposal
            .as_ref()
            .unwrap()
            .version,
        1
    );

    // Send tradeLockState(local=1, remote=1, Locked) from entity 1.
    let mut args = Vec::new();
    args.extend_from_slice(&1i32.to_le_bytes()); // local_version
    args.extend_from_slice(&1i32.to_le_bytes()); // remote_version
    args.push(ETRADELOCKSTATE_LOCKED as u8);
    dispatch(1, TRADE_LOCK_STATE, &args, &tx, &mut mgr).await;
    assert_eq!(
        mgr.get_entity(1)
            .unwrap()
            .trade_proposal
            .as_ref()
            .unwrap()
            .lock_state,
        ETRADELOCKSTATE_LOCKED
    );
    // Partner unaffected.
    assert_eq!(
        mgr.get_entity(2)
            .unwrap()
            .trade_proposal
            .as_ref()
            .unwrap()
            .lock_state,
        ETRADELOCKSTATE_NONE
    );

    // Stale-version reject: replay with local_version=999.
    let mut bad = Vec::new();
    bad.extend_from_slice(&999i32.to_le_bytes());
    bad.extend_from_slice(&1i32.to_le_bytes());
    bad.push(ETRADELOCKSTATE_LOCKED_AND_CONFIRMED as u8);
    dispatch(1, TRADE_LOCK_STATE, &bad, &tx, &mut mgr).await;
    // Did NOT change to LockedAndConfirmed.
    assert_eq!(
        mgr.get_entity(1)
            .unwrap()
            .trade_proposal
            .as_ref()
            .unwrap()
            .lock_state,
        ETRADELOCKSTATE_LOCKED
    );

    // Stale-partner downgrade: send with remote_version=0 (we know
    // it's actually 1). Lock=Locked should silently downgrade to None.
    let mut downgrade_args = Vec::new();
    downgrade_args.extend_from_slice(&1i32.to_le_bytes()); // local OK
    downgrade_args.extend_from_slice(&0i32.to_le_bytes()); // remote stale
    downgrade_args.push(ETRADELOCKSTATE_LOCKED as u8);
    dispatch(1, TRADE_LOCK_STATE, &downgrade_args, &tx, &mut mgr).await;
    assert_eq!(
        mgr.get_entity(1)
            .unwrap()
            .trade_proposal
            .as_ref()
            .unwrap()
            .lock_state,
        ETRADELOCKSTATE_NONE,
        "stale remote_version must silently downgrade Locked → None"
    );
}

/// Clara G11 regression guard: the `propagate_unlock_to_partner`
/// truth table — `unlock propagates` ⇔ `new_lock == None && prev_my_lock != Locked`.
///
/// The conditional from Python `Trade.py:207-210` is subtle: it does
/// NOT mean "any unlock propagates." It means a *unilateral* `Locked →
/// None` step-back stays on the actor's side only; every other path
/// *to* None (including `None → None` no-op and `LockedAndConfirmed →
/// None` back-out-of-confirm) DOES propagate. This test pins all four
/// rows.
///
/// Revert-verifier: flipping `prev_my_lock != ETRADELOCKSTATE_LOCKED`
/// to `prev_my_lock == ETRADELOCKSTATE_LOCKED` inverts the cascade —
/// the `Locked → None` row would propagate (it shouldn't), and the
/// other two rows would NOT propagate (they should). Either of those
/// flips fails this test.
#[tokio::test]
async fn propagate_unlock_to_partner_truth_table() {
    use cimmeria_common::Vector3;

    /// Drive a single row of the propagation truth table. Sets up a
    /// session with the requested `prev_my_lock` on the actor and
    /// `partner_lock` on the partner, sends a `tradeLockState`
    /// transition to `new_lock`, then returns `(actor_after, partner_after)`.
    async fn run_transition(prev_my_lock: i8, partner_lock: i8, new_lock: i8) -> (i8, i8) {
        let mut mgr = make_space_manager();
        make_two_players(&mut mgr, 1, 2, 2.0);

        let (tx, _rx) = mpsc::channel(64);
        // Open the session through tradeRequest so the validation
        // paths exercised by the truth-table reflect production.
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

        // Stage the precondition lock states directly on the proposals.
        if let Some(e) = mgr.get_entity_mut(1) {
            if let Some(p) = e.trade_proposal.as_mut() {
                p.lock_state = prev_my_lock;
            }
        }
        if let Some(e) = mgr.get_entity_mut(2) {
            if let Some(p) = e.trade_proposal.as_mut() {
                p.lock_state = partner_lock;
            }
        }
        // Keep both positions in range (the handler re-checks).
        if let Some(e) = mgr.get_entity_mut(2) {
            e.position = Vector3::new(2.0, 0.0, 0.0);
        }

        // Send tradeLockState from entity 1 transitioning to `new_lock`.
        let mut args = Vec::new();
        args.extend_from_slice(&1i32.to_le_bytes()); // local_version
        args.extend_from_slice(&1i32.to_le_bytes()); // remote_version
        args.push(new_lock as u8);
        dispatch(1, TRADE_LOCK_STATE, &args, &tx, &mut mgr).await;

        let actor_after = mgr
            .get_entity(1)
            .unwrap()
            .trade_proposal
            .as_ref()
            .unwrap()
            .lock_state;
        let partner_after = mgr
            .get_entity(2)
            .unwrap()
            .trade_proposal
            .as_ref()
            .unwrap()
            .lock_state;
        (actor_after, partner_after)
    }

    // Row 1: prev=None, new=None — no-op self-unlock cascades partner → None.
    // (Partner is staged as Locked; we expect it to be cleared.)
    let (_a, p) = run_transition(
        ETRADELOCKSTATE_NONE,
        ETRADELOCKSTATE_LOCKED,
        ETRADELOCKSTATE_NONE,
    )
    .await;
    assert_eq!(
        p, ETRADELOCKSTATE_NONE,
        "prev=None, new=None: partner's Locked must be cleared \
         (propagate=TRUE per the truth table)"
    );

    // Row 2: prev=Locked, new=None — unilateral step-back, partner stays.
    let (_a, p) = run_transition(
        ETRADELOCKSTATE_LOCKED,
        ETRADELOCKSTATE_LOCKED,
        ETRADELOCKSTATE_NONE,
    )
    .await;
    assert_eq!(
        p, ETRADELOCKSTATE_LOCKED,
        "prev=Locked, new=None: partner's Locked must NOT be cleared \
         (propagate=FALSE — this is the key row that distinguishes \
         this conditional from \"any unlock propagates\"). Flipping the \
         `prev_my_lock != Locked` test trips here."
    );

    // Row 3: prev=LockedAndConfirmed, new=None — backing out of confirm
    // cascades partner → None.
    let (_a, p) = run_transition(
        ETRADELOCKSTATE_LOCKED_AND_CONFIRMED,
        ETRADELOCKSTATE_LOCKED,
        ETRADELOCKSTATE_NONE,
    )
    .await;
    assert_eq!(
        p, ETRADELOCKSTATE_NONE,
        "prev=LockedAndConfirmed, new=None: backing out of confirm must \
         clear the partner's Locked (propagate=TRUE per the truth table)"
    );

    // Row 4: prev=None, new=Locked — not an unlock, propagate must be
    // FALSE regardless. Partner's prior Locked stays.
    let (_a, p) = run_transition(
        ETRADELOCKSTATE_NONE,
        ETRADELOCKSTATE_LOCKED,
        ETRADELOCKSTATE_LOCKED,
    )
    .await;
    assert_eq!(
        p, ETRADELOCKSTATE_LOCKED,
        "prev=None, new=Locked: this is a LOCK, not an unlock — \
         partner state must not be affected (propagate=FALSE)"
    );
}

#[tokio::test]
async fn invalid_lock_state_value_is_rejected() {
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

    // Send a junk lock state (5 is outside [0, 2]).
    let mut args = Vec::new();
    args.extend_from_slice(&1i32.to_le_bytes());
    args.extend_from_slice(&1i32.to_le_bytes());
    args.push(5u8);
    dispatch(1, TRADE_LOCK_STATE, &args, &tx, &mut mgr).await;
    // State unchanged.
    assert_eq!(
        mgr.get_entity(1)
            .unwrap()
            .trade_proposal
            .as_ref()
            .unwrap()
            .lock_state,
        ETRADELOCKSTATE_NONE
    );
}
