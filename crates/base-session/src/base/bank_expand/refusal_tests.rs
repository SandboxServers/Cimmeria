//! `expand_rejected` guards for every refusal after the purchase path
//! (BV-05): a closed verdict, no offer, a missing row, infrastructure
//! failures, a price the player was not shown, and results that must not
//! reach the wrong session. The shared fixtures are in `tests.rs`.
//!
//! Sentinels: players `0x7000_BB50..=0x7000_BBE1` from `tests::caller`,
//! entities `0x7000_BBE5..=0x7000_BBEF`. Skip when `DATABASE_URL` is unset.

use cimmeria_entity::cell_entity::ExpansionOffer;
use tracing::Level;

use super::tests::{
    bank_rows, caller, cleanup, expand, in_world, in_world_as, offered, one, row, setup, AT_BANKER,
    BANKER,
};
use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// The verdict refuses before any write: no session, and a Banker the
/// player walked away from. Each is WARN `expand_rejected` with the
/// verdict's own label, the size and the cash read for the log, and a line.
/// Fails if the verdict check is removed (the purchase goes through).
#[tokio::test]
async fn a_closed_vault_verdict_buys_nothing() {
    let pool = require_db_or_skip!();
    let cases = [
        (
            0x50,
            0x7000_BBE5,
            VaultAccess::NO_SESSION,
            "no_vault_session",
            "Talk to a Banker again to expand your vault. Nothing was charged.",
        ),
        (
            0x60,
            0x7000_BBE6,
            VaultAccess::Closed {
                reason: "banker_out_of_range",
                banker_id: Some(BANKER),
                distance: Some(9.0),
            },
            "banker_out_of_range",
            "You are too far from the Banker. Your vault was not expanded.",
        ),
    ];
    for (n, entity_id, vault, reason, line) in cases {
        let c = caller(n, entity_id);
        setup(&pool, c, 40, 500).await;
        let client = in_world(c, 40905);
        let capture = LogCapture::install();

        expand(&pool, &client, c, offered(40), vault).await;
        let after = row(&pool, c.player_id).await;
        cleanup(&pool, c).await;

        assert_eq!(after, (40, 500), "{reason}");
        one(
            &capture,
            "expand_rejected",
            Level::WARN,
            c,
            &[("reason", reason), ("bank_slots", "40"), ("cash", "500")],
        );
        assert!(client.saw_text(line), "{reason}: {line}");
    }
}

/// An open verdict with no offer on the session (the dialog was answered
/// twice, or after a re-open): WARN `expand_rejected reason=no_offer`.
#[tokio::test]
async fn an_answer_without_an_offer_buys_nothing() {
    let pool = require_db_or_skip!();
    let c = caller(0x70, 0x7000_BBE7);
    setup(&pool, c, 40, 500).await;
    let client = in_world(c, 40906);
    let capture = LogCapture::install();

    expand(&pool, &client, c, None, AT_BANKER).await;
    let after = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(after, (40, 500));
    let e = one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[("reason", "no_offer"), ("bank_slots", "40")],
    );
    assert!(!e.fields.contains_key("offered_slots"), "{e:#?}");
    assert!(client.saw_text("Talk to a Banker again to expand your vault. Nothing was charged."));
}

/// A character id with no row: WARN `reason=player_row_missing`.
#[tokio::test]
async fn a_missing_player_row_is_refused() {
    let pool = require_db_or_skip!();
    let c = caller(0x80, 0x7000_BBE8);
    cleanup(&pool, c).await;
    let client = in_world(c, 40907);
    let capture = LogCapture::install();

    expand(&pool, &client, c, offered(40), AT_BANKER).await;

    one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[("reason", "player_row_missing")],
    );
    assert!(client.saw_text("Your vault could not be expanded right now. Nothing was charged."));
}

/// No pool: WARN `reason=db_unavailable`, and a line.
#[tokio::test]
async fn no_pool_logs_db_unavailable() {
    let c = caller(0x90, 0x7000_BBE9);
    let client = in_world(c, 40908);
    let capture = LogCapture::install();

    handle_expand(
        c,
        offered(40),
        AT_BANKER,
        &None,
        &client.dyn_transport,
        &client.conn,
    )
    .await;

    one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[("reason", "db_unavailable"), ("offered_slots", "40")],
    );
    assert!(client.saw_text("Your vault could not be expanded right now. Nothing was charged."));
}

/// A pool that cannot connect: WARN `reason=query_failed` with the error.
#[tokio::test]
async fn an_unreachable_database_logs_query_failed() {
    let unreachable = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(200))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .expect("lazy pool");
    let c = caller(0xA0, 0x7000_BBEA);
    let client = in_world(c, 40909);
    let capture = LogCapture::install();

    expand(&unreachable, &client, c, offered(40), AT_BANKER).await;

    let e = one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[("reason", "query_failed")],
    );
    assert!(e.fields.contains_key("error"), "{e:#?}");
}

/// `price_missing` needs a seed gap, which a shared test database must not
/// get. The classification is pinned in `wire_tests`; this pins the row and
/// the line the refusal path produces for it.
#[tokio::test]
async fn price_missing_logs_its_reason_and_tells_the_player() {
    let c = caller(0xB0, 0x7000_BBEB);
    let client = in_world(c, 40910);
    let sends = Client {
        caller: c,
        transport: &client.dyn_transport,
        connected: &client.conn,
    };
    let capture = LogCapture::install();

    let snapshot = Snapshot {
        bank_slots: Some(40),
        cash: Some(500),
        price: None,
    };
    reject(
        &sends,
        ExpandRefusal::PriceMissing,
        &AT_BANKER,
        offered(40),
        snapshot,
        None,
    )
    .await;

    let e = one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[
            ("reason", "price_missing"),
            ("bank_slots", "40"),
            ("cash", "500"),
        ],
    );
    assert!(!e.fields.contains_key("price"), "{e:#?}");
    assert!(client.saw_text("Your vault could not be expanded right now. Nothing was charged."));
}

/// The character logged off before the answer: the purchase still commits,
/// and the dropped sends log `bank_feedback_send_failed
/// reason=no_client_address`.
#[tokio::test]
async fn a_player_with_no_client_address_logs_the_dropped_sends() {
    let pool = require_db_or_skip!();
    let c = caller(0xC0, 0x7000_BBEC);
    setup(&pool, c, 40, 100).await;
    // A session playing another character only.
    let client = in_world_as(0x7000_BBEF, c.player_id + 0x100, 40911);
    let capture = LogCapture::install();

    expand(&pool, &client, c, offered(40), AT_BANKER).await;
    let after = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(after, (50, 0));
    let dropped = bank_rows(&capture, "bank_feedback_send_failed");
    assert_eq!(dropped.len(), 3, "bag info, cash, line: {dropped:#?}");
    for d in &dropped {
        assert_eq!(d.level, Level::WARN);
        assert!(d.has_field("reason", "no_client_address"), "{d:#?}");
        assert!(d.has_field("player_id", &c.player_id.to_string()), "{d:#?}");
    }
}

/// The entity id the cell sent now belongs to **another** character's
/// session (the buyer gated and the id was reused): the purchase commits,
/// and nothing reaches that session, neither the buyer's vault size nor the
/// buyer's balance. Each dropped send logs `bank_feedback_send_failed
/// reason=no_client_address`. Fails if the sends are addressed by entity id.
#[tokio::test]
async fn a_recycled_entity_id_receives_nothing() {
    let pool = require_db_or_skip!();
    let c = caller(0xD0, 0x7000_BBED);
    setup(&pool, c, 40, 300).await;
    // The same entity id, played by someone else.
    let other = in_world_as(c.entity_id, c.player_id + 0x100, 40912);
    let capture = LogCapture::install();

    expand(&pool, &other, c, offered(40), AT_BANKER).await;
    let after = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(after, (50, 200), "the purchase itself commits");
    assert_eq!(
        other.sent(),
        0,
        "the other character's session gets nothing"
    );
    let dropped = bank_rows(&capture, "bank_feedback_send_failed");
    assert_eq!(dropped.len(), 3, "{dropped:#?}");
}

/// The price was retuned while the offer was open: nothing is charged,
/// WARN `expand_rejected reason=price_changed`. The seed price stays 100;
/// the offer claims 90.
#[tokio::test]
async fn a_price_the_player_was_not_shown_is_never_charged() {
    let pool = require_db_or_skip!();
    let c = caller(0xE0, 0x7000_BBEE);
    setup(&pool, c, 40, 300).await;
    let client = in_world(c, 40913);
    let capture = LogCapture::install();

    let shown = Some(ExpansionOffer {
        from_slots: 40,
        price: 90,
    });
    expand(&pool, &client, c, shown, AT_BANKER).await;
    let after = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(after, (40, 300));
    one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[
            ("reason", "price_changed"),
            ("offered_price", "90"),
            ("price", "100"),
        ],
    );
    assert!(client.saw_text("Talk to a Banker again to expand your vault. Nothing was charged."));
}
