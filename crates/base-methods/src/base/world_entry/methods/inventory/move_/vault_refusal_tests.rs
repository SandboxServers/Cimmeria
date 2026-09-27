//! Live-DB guards for the vault moves that used to fail silently (BV-03
//! review): each is now a `move_rejected` under `bank` with a feedback line
//! and the snap-back. Shares `vault_move_tests`' fixtures. Sentinels:
//! players `0x7000_B700..=0x7000_B721`, entities `0x7000_B7E0..=0x7000_B7E2`,
//! ports 40846-40848.

use super::tests::insert_item;
use super::vault_move_tests::{
    in_world, mv, rejected, rows, setup, teardown, AT_BANKER, BANKABLE, CARRIED_ONLY,
};
use crate::test_support::{require_db_or_skip, LogCapture};

const RBASE: i32 = 0x7000_B700;

/// An item whose `container_sets` has no 17 cannot be deposited:
/// `item_not_allowed_in_container`, `vault_end=target`. Before, the move
/// was dropped with a plain warning and the client kept the item drawn in
/// the vault.
#[tokio::test]
async fn an_item_the_vault_does_not_take_is_refused_visibly() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (RBASE, RBASE + 1, 0x7000_B7E0);
    setup(&pool, account_id, player_id).await;
    let item = insert_item(&pool, player_id, CARRIED_ONLY, 1, 0, 1).await;
    let client = in_world(entity_id, 40846);
    let capture = LogCapture::install();

    mv(
        &pool, &client, entity_id, player_id, item, 17, 0, -1, AT_BANKER,
    )
    .await;

    assert_eq!(rows(&pool, player_id).await, vec![(item, 1, 0, 1)]);
    let event = rejected(
        &capture,
        "item_not_allowed_in_container",
        account_id,
        player_id,
        entity_id,
        item,
    );
    assert!(event.has_field("vault_end", "target"), "{event:#?}");
    assert!(client.saw_text("That item cannot be placed there."));
    assert_eq!(
        client.transport.send_count_to(client.addr),
        2,
        "line + snap-back"
    );

    teardown(&pool, account_id, player_id).await;
}

/// A split onto an occupied vault slot is refused visibly:
/// `split_onto_occupied_slot`.
#[tokio::test]
async fn a_split_onto_an_occupied_vault_slot_is_refused_visibly() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (RBASE + 0x10, RBASE + 0x11, 0x7000_B7E1);
    setup(&pool, account_id, player_id).await;
    let carried = insert_item(&pool, player_id, BANKABLE, 1, 0, 10).await;
    // A full stack of the same type: no room to merge 4 into it.
    let banked = insert_item(&pool, player_id, BANKABLE, 17, 0, 20).await;
    let client = in_world(entity_id, 40847);
    let capture = LogCapture::install();

    mv(
        &pool, &client, entity_id, player_id, carried, 17, 0, 4, AT_BANKER,
    )
    .await;

    assert_eq!(
        rows(&pool, player_id).await,
        vec![(carried, 1, 0, 10), (banked, 17, 0, 20)]
    );
    rejected(
        &capture,
        "split_onto_occupied_slot",
        account_id,
        player_id,
        entity_id,
        carried,
    );
    assert!(client.saw_text("Split a stack onto an empty slot."));

    teardown(&pool, account_id, player_id).await;
}

/// A vault slot past the ceiling of 100 is refused in the transaction as
/// `target_slot_beyond_bank_slots` (with the player's `bank_slots`), not
/// dropped by the pre-transaction range check.
#[tokio::test]
async fn a_slot_past_the_ceiling_is_refused_visibly() {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = (RBASE + 0x20, RBASE + 0x21, 0x7000_B7E2);
    setup(&pool, account_id, player_id).await;
    let item = insert_item(&pool, player_id, BANKABLE, 1, 0, 1).await;
    let client = in_world(entity_id, 40848);
    let capture = LogCapture::install();

    mv(
        &pool, &client, entity_id, player_id, item, 17, 100, -1, AT_BANKER,
    )
    .await;

    assert_eq!(rows(&pool, player_id).await, vec![(item, 1, 0, 1)]);
    let event = rejected(
        &capture,
        "target_slot_beyond_bank_slots",
        account_id,
        player_id,
        entity_id,
        item,
    );
    assert!(event.has_field("bank_slots", "40"), "{event:#?}");
    assert!(client.saw_text("That vault slot is locked."));

    teardown(&pool, account_id, player_id).await;
}
