//! Failures: a use the server cannot decide is refused with the
//! "unavailable" line and a WARN, and never grants anything.

use cimmeria_entity::inventory::INV_CRAFTING;

use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// Slappack TC1: an ordinary consumable with no crafting effect rows.
const SLAPPACK: i32 = 2893;

/// A row with an empty stack is corrupt: the use fails with a WARN, the
/// player gets the "unavailable" line, and nothing is granted.
#[tokio::test]
async fn a_corrupt_stack_grants_nothing() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let slot = Slot::new(7);
    player(&pool, slot, &[], None).await;
    give(
        &pool,
        slot.item,
        slot.player_id,
        STEEL_PLATING,
        INV_CRAFTING,
        0,
    )
    .await;
    let session = OneSession::new(slot.entity(), 55797);

    let consumed = use_item(Some(&pool), &session, slot, slot.item).await;

    let (blueprints, _) = crafting(&pool, slot.player_id).await;
    let left = instance(&pool, slot.item).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, slot).await;
    assert!(consumed.is_none());
    assert!(blueprints.is_empty(), "no free grant: {blueprints:?}");
    assert_eq!(left, Some((slot.player_id, 0)));
    assert_eq!(
        sent,
        vec![refusal(
            slot.entity(),
            0,
            "Using that item is unavailable right now. Nothing was changed."
        )]
    );
    let warn = event(&capture, "persist_failed", slot);
    assert_eq!(warn.level, tracing::Level::WARN);
    assert_fields(
        &warn,
        &[("phase", "lock_item"), ("reason", "bad_stack_size")],
    );
}

/// Without a database the use is refused with a line and a WARN.
#[tokio::test]
async fn no_database_is_a_refusal_with_a_line() {
    let capture = LogCapture::install();
    let slot = Slot::new(15);
    let session = OneSession::new(slot.entity(), 55799);

    let consumed = use_item(None, &session, slot, slot.item).await;

    assert!(consumed.is_none());
    assert_eq!(
        session.typed.filter_to(session.addr),
        vec![refusal(
            slot.entity(),
            0,
            "Using that item is unavailable right now. Nothing was changed."
        )]
    );
    let warn = event(&capture, "persist_failed", slot);
    assert_eq!(warn.level, tracing::Level::WARN);
    assert_fields(&warn, &[("phase", "no_pool")]);
}

/// An item with no effect rows reaching the crafting use (the item-use path
/// only routes items that have some) is a lookup miss: a WARN, the
/// "unavailable" line, and the item stays.
#[tokio::test]
async fn an_item_without_effect_rows_is_a_lookup_miss() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let slot = Slot::new(12);
    player(&pool, slot, &[], None).await;
    give(&pool, slot.item, slot.player_id, SLAPPACK, INV_CRAFTING, 1).await;
    let session = OneSession::new(slot.entity(), 55804);

    let consumed = use_item(Some(&pool), &session, slot, slot.item).await;

    let after = crafting(&pool, slot.player_id).await;
    let left = instance(&pool, slot.item).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, slot).await;
    assert!(consumed.is_none());
    assert!(after.0.is_empty());
    assert_eq!(left, Some((slot.player_id, 1)), "not consumed");
    assert_eq!(
        sent,
        vec![refusal(
            slot.entity(),
            0,
            "Using that item is unavailable right now. Nothing was changed."
        )]
    );
    let warn = event(&capture, "lookup_failed", slot);
    assert_eq!(warn.level, tracing::Level::WARN);
    assert_fields(
        &warn,
        &[
            ("phase", "item_effects"),
            ("reason", "no_effects"),
            ("item_id", &slot.item.to_string()),
        ],
    );
}
