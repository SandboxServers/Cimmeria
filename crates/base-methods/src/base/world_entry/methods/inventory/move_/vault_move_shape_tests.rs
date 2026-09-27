//! Live-DB guards for the shapes a vault move can take: a split, a merge
//! (partial and whole), a full stack that swaps instead, a GM session, and
//! a carried move that is no bank event. Split from `vault_move_tests`,
//! whose fixtures and sentinel range (`BASE + 0x70..`) these share.

use super::allowlist_tests::assert_fields;
use super::tests::insert_item;
use super::vault_move_tests::{
    bank_events, in_world, mv, rows, setup, teardown, AT_BANKER, BANKABLE, BASE,
};
use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// A split into the vault conserves the count: 10 becomes 6 carried and a
/// new vault row of 4, and `move_accepted` records both stacks.
#[tokio::test]
async fn a_split_into_the_vault_conserves_the_count() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x70, BASE + 0x71, 0x7000_B5E7);
    setup(&pool, account_id, player_id).await;
    let item = insert_item(&pool, player_id, BANKABLE, 1, 0, 10).await;
    let client = in_world(entity_id, 40837);
    let capture = LogCapture::install();

    mv(
        &pool, &client, entity_id, player_id, item, 17, 2, 4, AT_BANKER,
    )
    .await;

    let after = rows(&pool, player_id).await;
    assert_eq!(after.len(), 2, "{after:?}");
    assert_eq!(after[0], (item, 1, 0, 6));
    assert_eq!((after[1].1, after[1].2, after[1].3), (17, 2, 4));
    assert_eq!(
        after.iter().map(|r| r.3).sum::<i32>(),
        10,
        "count conserved"
    );
    let accepted = bank_events(&capture, "move_accepted");
    assert_eq!(accepted.len(), 1);
    assert_fields(
        &accepted[0],
        &[
            ("kind", "split".into()),
            ("quantity", "4".into()),
            ("source_stack_before", "10".into()),
            ("source_stack_after", "6".into()),
            ("target_stack_before", "0".into()),
            ("target_stack_after", "4".into()),
        ],
        &[],
    );

    teardown(&pool, account_id, player_id).await;
}

/// A deposit onto a same-type vault stack merges (legacy
/// `Inventory.py:391-395`): a partial merge takes from the source, a whole
/// one deletes it and sends `onRemoveItem`. The count never changes.
#[tokio::test]
async fn a_deposit_merges_into_a_same_type_vault_stack() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x80, BASE + 0x81, 0x7000_B5E8);
    setup(&pool, account_id, player_id).await;
    let carried = insert_item(&pool, player_id, BANKABLE, 1, 0, 6).await;
    let banked = insert_item(&pool, player_id, BANKABLE, 17, 5, 4).await;
    let client = in_world(entity_id, 40838);
    let capture = LogCapture::install();

    mv(
        &pool, &client, entity_id, player_id, carried, 17, 5, 2, AT_BANKER,
    )
    .await;
    assert_eq!(
        rows(&pool, player_id).await,
        vec![(carried, 1, 0, 4), (banked, 17, 5, 6)],
        "partial merge"
    );

    mv(
        &pool, &client, entity_id, player_id, carried, 17, 5, -1, AT_BANKER,
    )
    .await;
    assert_eq!(
        rows(&pool, player_id).await,
        vec![(banked, 17, 5, 10)],
        "whole merge deletes the source row"
    );

    let accepted = bank_events(&capture, "move_accepted");
    assert_eq!(accepted.len(), 2);
    assert_fields(
        &accepted[1],
        &[
            ("kind", "merge".into()),
            ("source_stack_before", "4".into()),
            ("source_stack_after", "0".into()),
            ("target_stack_before", "6".into()),
            ("target_stack_after", "10".into()),
        ],
        &[],
    );

    teardown(&pool, account_id, player_id).await;
}

/// A merge that would overflow the stack (20) swaps instead, as before.
#[tokio::test]
async fn a_full_same_type_stack_swaps_instead_of_merging() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0x90, BASE + 0x91, 0x7000_B5E9);
    setup(&pool, account_id, player_id).await;
    let carried = insert_item(&pool, player_id, BANKABLE, 1, 0, 6).await;
    let banked = insert_item(&pool, player_id, BANKABLE, 17, 5, 18).await;
    let client = in_world(entity_id, 40839);

    mv(
        &pool, &client, entity_id, player_id, carried, 17, 5, -1, AT_BANKER,
    )
    .await;

    assert_eq!(
        rows(&pool, player_id).await,
        vec![(banked, 1, 0, 18), (carried, 17, 5, 6)]
    );

    teardown(&pool, account_id, player_id).await;
}

/// A GM `.bank` session has no Banker and skips proximity: the deposit is
/// accepted and `move_accepted` says `gm_override=true`.
#[tokio::test]
async fn a_gm_session_deposits_without_a_banker() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0xA0, BASE + 0xA1, 0x7000_B5EA);
    setup(&pool, account_id, player_id).await;
    let item = insert_item(&pool, player_id, BANKABLE, 1, 0, 1).await;
    let client = in_world(entity_id, 40840);
    let capture = LogCapture::install();
    let gm = VaultAccess::Open {
        banker_id: None,
        distance: None,
    };

    mv(&pool, &client, entity_id, player_id, item, 17, 0, -1, gm).await;

    assert_eq!(rows(&pool, player_id).await, vec![(item, 17, 0, 1)]);
    let accepted = bank_events(&capture, "move_accepted");
    assert_eq!(accepted.len(), 1);
    assert_fields(
        &accepted[0],
        &[("gm_override", "true".into())],
        &["banker_id", "distance"],
    );

    teardown(&pool, account_id, player_id).await;
}

/// A move that never touches the vault logs no `move_accepted`: the event
/// is the bank's, not every move's.
#[tokio::test]
async fn a_carried_move_logs_no_bank_event() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (BASE + 0xB0, BASE + 0xB1, 0x7000_B5EB);
    setup(&pool, account_id, player_id).await;
    let item = insert_item(&pool, player_id, BANKABLE, 1, 0, 1).await;
    let client = in_world(entity_id, 40841);
    let capture = LogCapture::install();

    mv(
        &pool,
        &client,
        entity_id,
        player_id,
        item,
        1,
        7,
        -1,
        VaultAccess::NO_SESSION,
    )
    .await;

    assert_eq!(rows(&pool, player_id).await, vec![(item, 1, 7, 1)]);
    assert!(
        capture.all().iter().all(|c| c.target != "bank"),
        "{:#?}",
        capture.all()
    );

    teardown(&pool, account_id, player_id).await;
}
