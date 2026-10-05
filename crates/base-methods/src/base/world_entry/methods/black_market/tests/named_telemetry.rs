//! Rule 6 guards for the Black Market's most-read events: `bm.refused`, the
//! transition rows and `bm.payout` name every ID they carry (the actor, the
//! seller or bidder when the line knows them, and the listed item).
//!
//! Each assertion fails if the paired name is dropped from the event, which is
//! the revert these guards exist for. No database: the events are emitted by
//! plain functions over an [`AuctionRow`].

use tracing::Level;

use crate::base::world_entry::methods::black_market::payout_mail::{
    Payout, PayoutReason, PayoutRole,
};
use crate::base::world_entry::methods::black_market::telemetry::{
    log_failure, log_outbid_refund, log_transition, Actor, Failure, Who,
};
use crate::base::world_entry::methods::black_market::types::{auction_status, AuctionRow};
use crate::base::world_entry::methods::black_market::wire::BMError;
use crate::base::world_entry::methods::mail::SystemMailSent;
use crate::test_support::LogCapture;

/// An item type the test book names.
const ITEM_DEF_ID: i32 = 912_345;
const ITEM_NAME: &str = "Zat'nik'tel";

/// Store (or clear) the process-global book for the item name.
fn named_book(on: bool) {
    let mut book = cimmeria_names::NameBook::empty();
    if on {
        book.insert(
            cimmeria_names::Table::Items,
            i64::from(ITEM_DEF_ID),
            ITEM_NAME,
        );
    }
    cimmeria_names::global().store(book);
}

fn actor() -> Actor {
    Actor {
        entity_id: 77,
        account_id: Some(6),
        account_name: Some("sgc_login"),
        player_id: 12,
        player_name: Some("Teal'c"),
    }
}

fn row() -> AuctionRow {
    AuctionRow {
        sequence_id: 900,
        seller_id: 12,
        item_id: 55,
        item_def_id: ITEM_DEF_ID,
        stack_size: 1,
        durability: 0,
        charges: 0,
        starting_price: 10,
        buyout_price: 100,
        current_bid: 20,
        current_bidder: Some(13),
        auction_length: 1,
        created_at: 0,
        expires_at: 1,
        status: auction_status::ACTIVE,
    }
}

#[test]
fn refusal_names_the_actor_and_pairs_the_error_id_with_bm_error() {
    let capture = LogCapture::install();
    log_failure(
        "bid",
        &actor(),
        Some(900),
        &Failure::Refused(BMError::BMUnavailable),
    );
    let ev = capture
        .find_event(
            Level::INFO,
            "Black Market request refused",
            BMError::BMUnavailable.reason(),
        )
        .unwrap_or_else(|| panic!("no refusal row: {:#?}", capture.all()));
    for (key, want) in [
        ("entity_name", "Teal'c"),
        ("player_name", "Teal'c"),
        ("account_name", "sgc_login"),
    ] {
        assert!(ev.has_field(key, want), "{key}: {ev:#?}");
    }
    assert!(ev.has_field("bm_error", "BMUnavailable"), "{ev:#?}");
}

#[test]
fn a_session_without_names_leaves_the_name_fields_off() {
    let capture = LogCapture::install();
    let mut nameless = actor();
    nameless.player_name = None;
    nameless.account_name = None;
    log_failure(
        "bid",
        &nameless,
        None,
        &Failure::Refused(BMError::BMUnavailable),
    );
    let ev = capture
        .find_event(
            Level::INFO,
            "Black Market request refused",
            BMError::BMUnavailable.reason(),
        )
        .unwrap();
    for key in ["player_name", "account_name", "entity_name"] {
        assert!(
            !ev.fields.contains_key(key),
            "{key} must be absent, not a sentinel: {ev:#?}"
        );
    }
}

#[test]
fn transition_names_the_item_and_the_parties_the_line_knows() {
    named_book(true);
    let capture = LogCapture::install();
    let who: Who = actor().who();
    log_transition("bm.bid", &who, None, &row());
    let ev = capture
        .all()
        .into_iter()
        .find(|e| e.message_contains("Black Market transition"))
        .expect("transition row");
    assert!(ev.has_field("player_name", "Teal'c"), "{ev:#?}");
    // The seller is the actor, so the line can name them; bidder 13 is not
    // known to the line and stays unnamed rather than costing a query.
    assert!(ev.has_field("seller_name", "Teal'c"), "{ev:#?}");
    assert!(!ev.fields.contains_key("bidder_name"), "{ev:#?}");
    assert!(
        ev.has_field("item_type_id", &ITEM_DEF_ID.to_string()),
        "{ev:#?}"
    );
    assert!(
        ev.has_field("item_name", ITEM_NAME),
        "item_name must pair item_type_id: {ev:#?}"
    );
    named_book(false);
}

#[test]
fn outbid_refund_and_payout_carry_their_names() {
    let capture = LogCapture::install();
    let me = actor();
    log_outbid_refund(&me, &row(), me.player_id, 20);
    let refund = capture
        .all()
        .into_iter()
        .find(|e| e.message_contains("held bid refunded"))
        .expect("refund row");
    assert!(
        refund.has_field("refunded_player_name", "Teal'c"),
        "{refund:#?}"
    );

    let payout = Payout {
        auction_id: 900,
        reason: PayoutReason::Outbid,
        role: PayoutRole::Bidder,
        mail: SystemMailSent {
            mail_id: 5,
            recipient_player_id: me.player_id,
            sender_name: "Black Market".to_owned(),
            cash: 20,
            item_source: "none",
            item: None,
            recipient_open_mail: 1,
        },
    };
    payout.log(&me.who());
    let ev = capture
        .all()
        .into_iter()
        .find(|e| e.message_contains("Black Market mail delivered"))
        .expect("payout row");
    assert!(ev.has_field("player_name", "Teal'c"), "{ev:#?}");
    assert!(ev.has_field("recipient_player_name", "Teal'c"), "{ev:#?}");
}
