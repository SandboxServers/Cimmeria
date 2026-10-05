//! Every outcome of a crafting item use against the seeded database: what
//! is written, what is consumed, what the client receives and what is
//! logged.

use cimmeria_entity::inventory::{INV_BANK, INV_BUYBACK, INV_CRAFTING, INV_MAIN};
use cimmeria_observability::testing::{counter_total, install as install_meter};
use cimmeria_wire::crafting::{known_crafts_args, racial_paradigm_level_args};

use super::*;
use crate::base::crafting::telemetry::{METRIC_REJECTIONS, METRIC_REQUESTS};
use crate::test_support::{require_db_or_skip, LogCapture};

/// A Blueprint item in the crafting bag teaches its blueprint, is consumed
/// with the change, and the client gets the full 139 list. The removal is
/// queued for the cell in the same transaction.
#[tokio::test]
async fn blueprint_item_teaches_is_consumed_and_pushes_the_list() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let slot = Slot::new(0);
    player(&pool, slot, &[40], None).await;
    give(
        &pool,
        slot.item,
        slot.player_id,
        STEEL_PLATING,
        INV_CRAFTING,
        1,
    )
    .await;
    let session = OneSession::new(slot.entity(), 55790);
    let accepted = [("verb", VERB), ("outcome", "accepted")];
    let accepted_before = counter_total(METRIC_REQUESTS, &accepted);

    let consumed = use_item(Some(&pool), &session, slot, slot.item).await;

    let (blueprints, _) = crafting(&pool, slot.player_id).await;
    let left = instance(&pool, slot.item).await;
    let outbox = outbox_rows(&pool, slot).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, slot).await;

    let consumed = consumed.expect("a committed use");
    assert_eq!(
        (
            consumed.item_id,
            consumed.container_id,
            consumed.removed_all
        ),
        (slot.item, INV_CRAFTING, true)
    );
    assert!(consumed.outbox.is_some(), "removal queued for the cell");
    assert_eq!(blueprints, vec![25, 40]);
    assert_eq!(left, None, "the item was consumed");
    assert_eq!(outbox, vec!["inventory_item_removed".to_string()]);
    assert_eq!(
        sent,
        vec![packet(
            slot.entity(),
            0,
            method_idx::ON_UPDATE_KNOWN_CRAFTS,
            &known_crafts_args(&[25, 40])
        )]
    );
    let learned = event(&capture, "blueprint_learned", slot);
    assert_fields(
        &learned,
        &[
            ("item_id", &slot.item.to_string()),
            ("item_type_id", "6483"),
            ("blueprints", "25:false→true"),
            ("known_before", "1"),
            ("known_after", "2"),
            ("consumed", &format!("{}:6483:1→0", slot.item)),
        ],
    );
    event(&capture, "request", slot);
    assert!(counter_total(METRIC_REQUESTS, &accepted) > accepted_before);
}

/// Item 8882 names 367 and 369. With 367 known it teaches 369 and is used.
#[tokio::test]
async fn two_blueprint_item_teaches_the_unknown_one() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let slot = Slot::new(1);
    player(&pool, slot, &[367], None).await;
    give(
        &pool,
        slot.item,
        slot.player_id,
        HEALTH_ANTIDOTE,
        INV_MAIN,
        1,
    )
    .await;
    let session = OneSession::new(slot.entity(), 55791);

    let consumed = use_item(Some(&pool), &session, slot, slot.item).await;

    let (blueprints, _) = crafting(&pool, slot.player_id).await;
    let left = instance(&pool, slot.item).await;
    cleanup(&pool, slot).await;
    assert!(consumed.is_some());
    assert_eq!(blueprints, vec![367, 369]);
    assert_eq!(left, None);
    assert_fields(
        &event(&capture, "blueprint_learned", slot),
        &[
            ("blueprints", "367:true→true,369:false→true"),
            ("known_before", "1"),
            ("known_after", "2"),
        ],
    );
}

/// With both of 8882's blueprints known the use is refused: a text line,
/// the item stays, nothing is written or queued.
#[tokio::test]
async fn known_blueprint_item_is_refused_and_not_consumed() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let slot = Slot::new(2);
    player(&pool, slot, &[367, 369], None).await;
    give(
        &pool,
        slot.item,
        slot.player_id,
        HEALTH_ANTIDOTE,
        INV_CRAFTING,
        1,
    )
    .await;
    let session = OneSession::new(slot.entity(), 55792);
    let rejected = [("verb", VERB), ("reason", "already_known")];
    let rejected_before = counter_total(METRIC_REJECTIONS, &rejected);

    let consumed = use_item(Some(&pool), &session, slot, slot.item).await;

    let (blueprints, _) = crafting(&pool, slot.player_id).await;
    let left = instance(&pool, slot.item).await;
    let outbox = outbox_rows(&pool, slot).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, slot).await;
    assert!(consumed.is_none());
    assert_eq!(blueprints, vec![367, 369]);
    assert_eq!(left, Some((slot.player_id, 1)), "not consumed");
    assert!(outbox.is_empty(), "nothing queued: {outbox:?}");
    assert_eq!(
        sent,
        vec![refusal(
            slot.entity(),
            0,
            "You already know these blueprints. The item was not used."
        )]
    );
    assert_fields(
        &event(&capture, "rejected", slot),
        &[
            ("reason", "already_known"),
            ("design_item_type_id", "8882"),
            ("blueprint_ids", "[367, 369]"),
        ],
    );
    assert!(counter_total(METRIC_REJECTIONS, &rejected) > rejected_before);
}

/// A Goa'uld guide raises paradigm 3 from 1 to 2, is consumed, and the
/// client gets 138.
#[tokio::test]
async fn guide_raises_its_paradigm_is_consumed_and_pushes_138() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let slot = Slot::new(3);
    player(&pool, slot, &[], None).await;
    give(
        &pool,
        slot.item,
        slot.player_id,
        GOAULD_GUIDE,
        INV_CRAFTING,
        1,
    )
    .await;
    let session = OneSession::new(slot.entity(), 55793);

    let consumed = use_item(Some(&pool), &session, slot, slot.item).await;

    let (_, levels) = crafting(&pool, slot.player_id).await;
    let left = instance(&pool, slot.item).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, slot).await;
    assert!(consumed.is_some());
    assert_eq!(levels, vec![(1, 5), (2, 1), (3, 2), (4, 1), (5, 1)]);
    assert_eq!(left, None);
    assert_eq!(
        sent,
        vec![packet(
            slot.entity(),
            0,
            cimmeria_wire::cell::client_methods::player::ON_UPDATE_RACIAL_PARADIGM_LEVEL,
            &racial_paradigm_level_args(GOAULD, 2)
        )]
    );
    assert_fields(
        &event(&capture, "paradigm_raised", slot),
        &[
            ("paradigm_id", "3"),
            ("level_before", "1"),
            ("level_after", "2"),
            ("item_type_id", "7808"),
            ("consumed", &format!("{}:7808:1→0", slot.item)),
        ],
    );
}

/// A guide for a paradigm at 10 is refused and stays in the bag.
#[tokio::test]
async fn guide_at_the_maximum_is_refused_and_not_consumed() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let slot = Slot::new(4);
    player(&pool, slot, &[], Some((GOAULD, 10))).await;
    give(
        &pool,
        slot.item,
        slot.player_id,
        GOAULD_GUIDE,
        INV_CRAFTING,
        1,
    )
    .await;
    let session = OneSession::new(slot.entity(), 55794);

    let consumed = use_item(Some(&pool), &session, slot, slot.item).await;

    let (_, levels) = crafting(&pool, slot.player_id).await;
    let left = instance(&pool, slot.item).await;
    let outbox = outbox_rows(&pool, slot).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, slot).await;
    assert!(consumed.is_none());
    assert_eq!(levels[2], (3, 10));
    assert_eq!(left, Some((slot.player_id, 1)), "not consumed");
    assert!(outbox.is_empty());
    assert_eq!(
        sent,
        vec![refusal(
            slot.entity(),
            0,
            "Your Goa'uld racial paradigm is already at 10, the maximum. The guide was not used."
        )]
    );
    assert_fields(
        &event(&capture, "rejected", slot),
        &[
            ("reason", "paradigm_max"),
            ("paradigm_id", "3"),
            ("paradigm_level", "10"),
            ("required_level", "10"),
        ],
    );
}

/// An item in the bank or on the buyback list cannot be used: sold items
/// would otherwise teach and then be bought back.
#[tokio::test]
async fn item_outside_the_carried_bags_is_refused() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let slot = Slot::new(5);
    player(&pool, slot, &[], None).await;
    give(&pool, slot.item, slot.player_id, STEEL_PLATING, INV_BANK, 1).await;
    give(
        &pool,
        slot.other_item,
        slot.player_id,
        GOAULD_GUIDE,
        INV_BUYBACK,
        1,
    )
    .await;
    let session = OneSession::new(slot.entity(), 55795);

    let from_bank = use_item(Some(&pool), &session, slot, slot.item).await;
    let from_buyback = use_item(Some(&pool), &session, slot, slot.other_item).await;

    let after = crafting(&pool, slot.player_id).await;
    let left = (
        instance(&pool, slot.item).await,
        instance(&pool, slot.other_item).await,
    );
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, slot).await;
    assert!(from_bank.is_none() && from_buyback.is_none());
    assert_eq!(after.0, Vec::<i32>::new(), "nothing learned");
    assert_eq!(after.1[2], (3, 1), "nothing raised");
    assert_eq!(left, (Some((slot.player_id, 1)), Some((slot.player_id, 1))));
    let line = "Move that item to your crafting bag to use it.";
    assert_eq!(
        sent,
        vec![
            refusal(slot.entity(), 0, line),
            refusal(slot.entity(), 1, line)
        ]
    );
    assert_fields(
        &event(&capture, "rejected", slot),
        &[
            ("reason", "not_carried"),
            ("container_id", &INV_BANK.to_string()),
            ("item_id", &slot.item.to_string()),
        ],
    );
}

/// A stack of two loses one and stays: no `onRemoveItem`, no cell
/// notification.
#[tokio::test]
async fn a_stack_of_two_loses_one() {
    let pool = require_db_or_skip!();
    let slot = Slot::new(6);
    player(&pool, slot, &[], None).await;
    give(
        &pool,
        slot.item,
        slot.player_id,
        GOAULD_GUIDE,
        INV_CRAFTING,
        2,
    )
    .await;
    let session = OneSession::new(slot.entity(), 55796);

    let consumed = use_item(Some(&pool), &session, slot, slot.item).await;

    let (_, levels) = crafting(&pool, slot.player_id).await;
    let left = instance(&pool, slot.item).await;
    let outbox = outbox_rows(&pool, slot).await;
    cleanup(&pool, slot).await;
    let consumed = consumed.expect("a committed use");
    assert!(!consumed.removed_all && consumed.outbox.is_none());
    assert_eq!(levels[2], (3, 2));
    assert_eq!(left, Some((slot.player_id, 1)));
    assert!(outbox.is_empty());
}
