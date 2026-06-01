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
