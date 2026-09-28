//! Live-DB tests for the base half of the Black Market GM tools (BM-07):
//! `.bm_seed`, `.bm_expire`, `.bm_list`. The parsers and the GM gate are
//! tested on the cell (`cell-console` `tests/bm07_black_market.rs`).
//!
//! Sentinels: accounts and players `0x7000_AC00..=0x7000_AC3F`, entity ids
//! `0x7000_AD0x`, inside the Black Market's `0x7000_Axxx` block and past
//! every range `black_market/tests/mod.rs` lists. The GM is session 0 of the
//! harness, so its address receives only the GM's feedback lines.

use std::net::SocketAddr;

use sqlx::PgPool;
use tracing::Level;

use super::super::seed::{uat_specs, SYSTEM_ACCOUNT_ID, SYSTEM_SELLER_ID, SYSTEM_SELLER_NAME};
use super::super::tests::{
    bm_mails, cleanup, insert_account_and_player, insert_item, status_of, Harness, Session,
    ITEM_DEF_ID,
};
use super::super::types::auction_status;
use super::{gm_expire, gm_list, gm_seed, GmCtx};
use crate::cell::messages::BmGmActor;
use crate::test_support::{require_db_or_skip, LogCapture};

const GM: Session = (0x7000_AD01, 0x7000_AC00, 0x7000_AC01);
const SELLER: Session = (0x7000_AD02, 0x7000_AC10, 0x7000_AC11);
const BIDDER: Session = (0x7000_AD03, 0x7000_AC20, 0x7000_AC21);
const SQUATTER_NAME: &str = "bm07-gm-squatter";

fn gm_ctx(h: &Harness) -> GmCtx<'_> {
    GmCtx::new(
        BmGmActor {
            entity_id: GM.0,
            player_id: GM.2,
            account_id: Some(GM.1 as u32),
        },
        &h.transport,
        &h.connected,
        &h.e2a,
    )
}

/// The feedback lines the GM received, in order. The GM's address (session
/// 0) gets nothing else.
fn gm_lines(h: &Harness) -> Vec<String> {
    let addr: SocketAddr = "127.0.0.1:41000".parse().unwrap();
    let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
    h.tt.filter_to(addr)
        .iter()
        .map(|packet| {
            let pt = enc.decrypt(packet).expect("decrypt test packet");
            let args = &pt[1..pt.len() - 4][7..];
            let speaker_len = u32::from_le_bytes(args[0..4].try_into().unwrap()) as usize;
            let mut at = 4 + speaker_len * 2 + 2;
            let len = u32::from_le_bytes(args[at..at + 4].try_into().unwrap()) as usize;
            at += 4;
            let units: Vec<u16> = args[at..at + len * 2]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect();
            String::from_utf16(&units).unwrap()
        })
        .collect()
}

async fn system_listing_ids(pool: &PgPool) -> Vec<i32> {
    sqlx::query_scalar(
        "SELECT sequence_id FROM sgw_auction WHERE seller_id = $1 ORDER BY sequence_id",
    )
    .bind(SYSTEM_SELLER_ID)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn delete_auctions(pool: &PgPool, ids: &[i32]) {
    for id in ids {
        sqlx::query("DELETE FROM sgw_auction WHERE sequence_id = $1")
            .bind(id)
            .execute(pool)
            .await
            .unwrap();
    }
}

/// `.bm_seed 10` lists ten system-seller auctions that cycle through the
/// UAT set, active and due in the future, and tells the GM their ids.
#[tokio::test]
async fn gm_seed_lists_the_uat_set_as_the_system_seller() {
    let pool = require_db_or_skip!();
    let h = Harness::new(&pool, &[GM]);
    let before = system_listing_ids(&pool).await;

    gm_seed(gm_ctx(&h), 10, &h.db).await;

    let new: Vec<i32> = system_listing_ids(&pool)
        .await
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect();
    let rows: Vec<(i32, i32, i16, i32)> = sqlx::query_as(
        "SELECT item_def_id, stack_size, status, item_id FROM sgw_auction \
         WHERE sequence_id = ANY($1) ORDER BY sequence_id",
    )
    .bind(&new)
    .fetch_all(&pool)
    .await
    .unwrap();
    let lines = gm_lines(&h);
    delete_auctions(&pool, &new).await;

    assert_eq!(new.len(), 10, "ten listings");
    let specs = uat_specs();
    for (i, (item, stack, status, item_id)) in rows.iter().enumerate() {
        let spec = specs[i % specs.len()];
        assert_eq!(
            (*item, *stack),
            (spec.item_def_id, spec.stack_size),
            "row {i}"
        );
        assert_eq!((*status, *item_id), (auction_status::ACTIVE, 0), "row {i}");
    }
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with(&format!(
            "Listed 10 Black Market auction(s) from the system seller: ids {} to {}.",
            new[0], new[9]
        )),
        "{lines:?}"
    );
}

/// `.bm_seed` checks the reserved system seller exactly as the boot seed
/// does: with account 1 held by someone else it lists nothing, tells the
/// GM why, and logs `bm.gm_rejected reason=account_taken` with the GM's ids.
/// Fails if `.bm_seed` skips `ensure_system_seller`'s read-back.
#[tokio::test]
async fn gm_seed_refuses_when_the_reserved_ids_are_taken() {
    let pool = require_db_or_skip!();
    for sql in [
        "DELETE FROM sgw_auction WHERE seller_id = $1",
        "DELETE FROM sgw_player WHERE player_id = $1",
        "DELETE FROM account WHERE account_id = $1",
    ] {
        sqlx::query(sql)
            .bind(SYSTEM_SELLER_ID)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(SYSTEM_ACCOUNT_ID)
        .bind(SQUATTER_NAME)
        .execute(&pool)
        .await
        .unwrap();
    let h = Harness::new(&pool, &[GM]);
    let capture = LogCapture::install();

    gm_seed(gm_ctx(&h), 8, &h.db).await;

    let listed = system_listing_ids(&pool).await;
    let lines = gm_lines(&h);
    let row = capture.find_event(
        Level::WARN,
        "Black Market GM command refused",
        "account_taken",
    );
    sqlx::query("DELETE FROM account WHERE account_id = $1 AND account_name = $2")
        .bind(SYSTEM_ACCOUNT_ID)
        .bind(SQUATTER_NAME)
        .execute(&pool)
        .await
        .unwrap();

    assert!(listed.is_empty(), "nothing listed: {listed:?}");
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with(".bm_seed: refused"), "{lines:?}");
    let row = row.expect("bm.gm_rejected reason=account_taken");
    assert!(row.has_field("command", "bm_seed"));
    assert!(row.has_field("player_id", &GM.2.to_string()));
    assert_ne!(SQUATTER_NAME, SYSTEM_SELLER_NAME);
}

/// Seller and bidder online in the harness, the seller's item listed at
/// `start` with no buyout, the bidder holding 10,000 naquadah.
async fn listed(pool: &PgPool) -> (Harness, i32, i32) {
    cleanup(pool, &[SELLER.1, BIDDER.1], &[SELLER.2, BIDDER.2]).await;
    insert_account_and_player(pool, SELLER.1, SELLER.2, 0).await;
    insert_account_and_player(pool, BIDDER.1, BIDDER.2, 10_000).await;
    let item = insert_item(pool, SELLER.2, ITEM_DEF_ID).await;
    let h = Harness::new(pool, &[GM, SELLER, BIDDER]);
    h.create(SELLER, item, 100, 0, 5).await;
    let seq: i32 =
        sqlx::query_scalar("SELECT MAX(sequence_id) FROM sgw_auction WHERE seller_id = $1")
            .bind(SELLER.2)
            .fetch_one(pool)
            .await
            .unwrap();
    (h, seq, item)
}

/// `.bm_expire` on an auction with a bid settles it at once as sold,
/// through the sweep's own path: status SOLD, the item mailed to the buyer
/// and the winning bid to the seller (BM-02b system mail), and the GM is
/// told who bought it for how much. Fails if `.bm_expire` only makes the auction due and
/// leaves it for the next sweep (the status would still be ACTIVE).
#[tokio::test]
async fn gm_expire_settles_a_bid_auction_as_sold_now() {
    let pool = require_db_or_skip!();
    let (h, seq, item) = listed(&pool).await;
    h.bid(BIDDER, seq, 250).await;
    assert_eq!(status_of(&pool, seq).await, auction_status::ACTIVE);

    gm_expire(gm_ctx(&h), seq, &h.db).await;

    let status = status_of(&pool, seq).await;
    let buyer_mails = bm_mails(&pool, BIDDER.2).await;
    let seller_mails = bm_mails(&pool, SELLER.2).await;
    let lines = gm_lines(&h);
    cleanup(&pool, &[SELLER.1, BIDDER.1], &[SELLER.2, BIDDER.2]).await;

    assert_eq!(status, auction_status::SOLD);
    assert!(
        buyer_mails.iter().any(|m| m.2 == Some(item)),
        "the item is mailed to the buyer: {buyer_mails:?}"
    );
    assert!(
        seller_mails.iter().any(|m| m.1 == 250),
        "the winning bid is mailed to the seller: {seller_mails:?}"
    );
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with(&format!("Auction {seq} (")) && lines[0].contains("sold to bmp-"),
        "{lines:?}"
    );
    assert!(lines[0].contains("for 250 naquadah"), "{lines:?}");
}

/// `.bm_expire` on an auction with no bid settles it as unsold: the item
/// is mailed back to its seller and the GM is told so.
#[tokio::test]
async fn gm_expire_returns_an_unsold_item_to_its_seller() {
    let pool = require_db_or_skip!();
    let (h, seq, item) = listed(&pool).await;

    gm_expire(gm_ctx(&h), seq, &h.db).await;

    let status = status_of(&pool, seq).await;
    let seller_mails = bm_mails(&pool, SELLER.2).await;
    let lines = gm_lines(&h);
    cleanup(&pool, &[SELLER.1, BIDDER.1], &[SELLER.2, BIDDER.2]).await;

    assert_eq!(status, auction_status::EXPIRED);
    assert!(
        seller_mails.iter().any(|m| m.2 == Some(item)),
        "the item is mailed back to its seller: {seller_mails:?}"
    );
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].contains("expired unsold. The item is mailed back to its seller"),
        "{lines:?}"
    );
}

/// Type 12: `.bm_expire` changes nothing for an id that is not an active
/// auction, names why, and logs `bm.gm_rejected` with the reason. A
/// settled auction stays settled: `expires_at` is untouched.
#[tokio::test]
async fn gm_expire_refuses_unknown_and_settled_auctions() {
    let pool = require_db_or_skip!();
    let (h, seq, _item) = listed(&pool).await;
    h.cancel(SELLER, seq).await;
    let expires_before: i32 =
        sqlx::query_scalar("SELECT expires_at FROM sgw_auction WHERE sequence_id = $1")
            .bind(seq)
            .fetch_one(&pool)
            .await
            .unwrap();
    let capture = LogCapture::install();

    gm_expire(gm_ctx(&h), seq, &h.db).await;
    gm_expire(gm_ctx(&h), i32::MAX, &h.db).await;

    let expires_after: i32 =
        sqlx::query_scalar("SELECT expires_at FROM sgw_auction WHERE sequence_id = $1")
            .bind(seq)
            .fetch_one(&pool)
            .await
            .unwrap();
    let lines = gm_lines(&h);
    let cancelled = capture.find_event(
        Level::WARN,
        "Black Market GM command refused",
        "auction_cancelled",
    );
    let unknown = capture.find_event(
        Level::WARN,
        "Black Market GM command refused",
        "auction_not_found",
    );
    cleanup(&pool, &[SELLER.1, BIDDER.1], &[SELLER.2, BIDDER.2]).await;

    assert_eq!(
        expires_after, expires_before,
        "a settled auction is not touched"
    );
    assert_eq!(
        lines,
        vec![
            format!(".bm_expire: auction {seq} was cancelled by its seller."),
            format!(
                ".bm_expire: no auction has id {}. Type .bm_list for ids.",
                i32::MAX
            ),
        ]
    );
    assert!(cancelled.is_some() && unknown.is_some());
}

/// `.bm_list` shows the newest active auctions, each with its id, then the
/// total.
#[tokio::test]
async fn gm_list_shows_the_newest_auctions_with_their_ids() {
    let pool = require_db_or_skip!();
    let (h, seq, _item) = listed(&pool).await;

    gm_list(gm_ctx(&h), &h.db).await;

    let lines = gm_lines(&h);
    cleanup(&pool, &[SELLER.1, BIDDER.1], &[SELLER.2, BIDDER.2]).await;

    assert!(
        lines
            .iter()
            .any(|l| l.starts_with(&format!("#{seq} ")) && l.contains("from bmp-")),
        "the new listing, by id and seller: {lines:?}"
    );
    assert!(
        lines
            .last()
            .is_some_and(|l| l.contains("active auction(s), newest")),
        "{lines:?}"
    );
}
