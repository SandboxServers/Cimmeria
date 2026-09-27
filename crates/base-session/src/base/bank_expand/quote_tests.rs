//! The expansion quote (BV-05): below the ceiling the cell is sent the
//! offer, at the ceiling the player is told the vault is full, and every
//! outcome logs `expand_quote` with its reason.
//!
//! Sentinels: `tests::caller(0x110..=0x170)` (accounts and players
//! `0x7000_BC10..=0x7000_BC71`), entities `0x7000_BBF0..=0x7000_BBF6`.

use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::Level;

use super::tests::{caller, cleanup, in_world, one, setup, TestClient};
use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

const SPEAKER: u32 = 0x7000_BBD1;

async fn quote(
    pool: Option<&PgPool>,
    client: &TestClient,
    c: ExpandCaller,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    handle_expansion_quote(
        c,
        SPEAKER,
        &pool.map(|p| Arc::new(p.clone())),
        cell_tx,
        &client.dyn_transport,
        &client.conn,
    )
    .await;
}

fn offers(rx: &mut mpsc::Receiver<BaseToCellMsg>) -> Vec<BankBaseToCell> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let BaseToCellMsg::Bank(b) = msg {
            out.push(b);
        }
    }
    out
}

/// Below the ceiling: exactly one `OfferExpansion` to the cell, carrying
/// the current size, the next step's seeded price and the speaker; DEBUG
/// `expand_quote offered=true`. Nothing reaches the client yet: the cell
/// shows the dialog.
#[tokio::test]
async fn below_the_ceiling_the_cell_is_sent_the_offer() {
    let pool = require_db_or_skip!();
    let c = caller(0x100 + 0x10, 0x7000_BBF0);
    setup(&pool, c, 70, 300).await;
    let client = in_world(c, 40920);
    let (tx, mut rx) = mpsc::channel(8);
    let capture = LogCapture::install();

    quote(Some(&pool), &client, c, &Some(tx)).await;
    cleanup(&pool, c).await;

    assert_eq!(
        offers(&mut rx),
        vec![BankBaseToCell::OfferExpansion {
            entity_id: c.entity_id,
            player_id: c.player_id,
            speaker_id: SPEAKER,
            from_slots: 70,
            price: 100,
        }]
    );
    one(
        &capture,
        "expand_quote",
        Level::DEBUG,
        c,
        &[
            ("offered", "true"),
            ("bank_slots", "70"),
            ("cash", "300"),
            ("price", "100"),
        ],
    );
    assert_eq!(client.sent(), 0);
}

/// At 100 slots: no offer, DEBUG `expand_quote offered=false
/// reason=at_ceiling`, and a line so the player knows why there is no
/// Expand dialog (UAT step 11). Fails if the ceiling check is removed (the
/// quote then finds no price row for 110 and logs WARN `price_missing`
/// instead, with no line).
#[tokio::test]
async fn at_the_ceiling_there_is_no_offer_and_the_player_is_told() {
    let pool = require_db_or_skip!();
    let c = caller(0x100 + 0x20, 0x7000_BBF1);
    setup(&pool, c, 100, 300).await;
    let client = in_world(c, 40921);
    let (tx, mut rx) = mpsc::channel(8);
    let capture = LogCapture::install();

    quote(Some(&pool), &client, c, &Some(tx)).await;
    cleanup(&pool, c).await;

    assert!(offers(&mut rx).is_empty());
    one(
        &capture,
        "expand_quote",
        Level::DEBUG,
        c,
        &[
            ("offered", "false"),
            ("reason", "at_ceiling"),
            ("bank_slots", "100"),
        ],
    );
    assert!(client.saw_text("Your vault is already at its full size of 100 slots."));
}

/// The failures: no pool, no row, no cell channel. Each is WARN
/// `expand_quote offered=false` with its reason, and no offer is sent.
#[tokio::test]
async fn quote_failures_log_their_reason() {
    let pool = require_db_or_skip!();

    let c = caller(0x100 + 0x30, 0x7000_BBF2);
    let client = in_world(c, 40922);
    let capture = LogCapture::install();
    quote(None, &client, c, &None).await;
    one(
        &capture,
        "expand_quote",
        Level::WARN,
        c,
        &[("offered", "false"), ("reason", "db_unavailable")],
    );
    drop(capture);

    let c = caller(0x100 + 0x40, 0x7000_BBF3);
    cleanup(&pool, c).await;
    let capture = LogCapture::install();
    quote(Some(&pool), &client, c, &None).await;
    one(
        &capture,
        "expand_quote",
        Level::WARN,
        c,
        &[("offered", "false"), ("reason", "player_row_missing")],
    );
    drop(capture);

    let c = caller(0x100 + 0x50, 0x7000_BBF4);
    setup(&pool, c, 40, 0).await;
    let capture = LogCapture::install();
    quote(Some(&pool), &client, c, &None).await;
    cleanup(&pool, c).await;
    one(
        &capture,
        "expand_quote",
        Level::WARN,
        c,
        &[
            ("offered", "false"),
            ("reason", "cell_channel_closed"),
            ("bank_slots", "40"),
        ],
    );
}

/// An unreachable database: WARN `reason=query_failed` with the error.
#[tokio::test]
async fn an_unreachable_database_logs_quote_query_failed() {
    let unreachable = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_millis(200))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .expect("lazy pool");
    let c = caller(0x100 + 0x60, 0x7000_BBF5);
    let client = in_world(c, 40923);
    let capture = LogCapture::install();

    quote(Some(&unreachable), &client, c, &None).await;

    let e = one(
        &capture,
        "expand_quote",
        Level::WARN,
        c,
        &[("offered", "false"), ("reason", "query_failed")],
    );
    assert!(e.fields.contains_key("error"), "{e:#?}");
}

/// `price_missing` needs a seed gap a shared test database must not get,
/// so the row is pinned on the logging function the quote calls.
#[test]
fn quote_price_missing_logs_its_reason() {
    let c = caller(0x100 + 0x70, 0x7000_BBF6);
    let capture = LogCapture::install();
    let state = ExpansionState {
        bank_slots: 40,
        naquadah: 7,
        next_price: None,
    };
    quote_warn(c, "price_missing", Some(&state), None);
    one(
        &capture,
        "expand_quote",
        Level::WARN,
        c,
        &[
            ("offered", "false"),
            ("reason", "price_missing"),
            ("bank_slots", "40"),
        ],
    );
}
