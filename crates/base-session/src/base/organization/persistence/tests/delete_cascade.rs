//! The `character.delete_cascade` row: what a character delete destroys
//! (listings, held bids, mail and its escrowed items) is counted inside the
//! delete transaction and logged once the delete commits.

use super::super::super::character_delete::delete_character;
use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};
use tracing::Level;

/// Seed one open auction sold by `seller` with `bidder` holding `bid` on it.
async fn seed_auction(pool: &PgPool, seller: i32, bidder: Option<(i32, i32)>, item: i32, def: i32) {
    let (current_bidder, current_bid) = bidder.unwrap_or((0, 0));
    sqlx::query(
        "INSERT INTO sgw_auction \
            (seller_id, item_id, item_def_id, current_bid, current_bidder, created_at, expires_at, status) \
         VALUES ($1, $2, $3, $4, NULLIF($5, 0), 0, 0, 0)",
    )
    .bind(seller)
    .bind(item)
    .bind(def)
    .bind(current_bid)
    .bind(current_bidder)
    .execute(pool)
    .await
    .expect("insert auction");
}

/// Seed one mail to `character` from `sender`, with `flags`, `cash` and an
/// item in escrow; returns its `mail_id`.
async fn seed_mail(
    pool: &PgPool,
    character: i32,
    sender: i32,
    flags: i32,
    cash: i64,
    item: i32,
    type_id: i32,
) -> i32 {
    let mail_id: i32 = sqlx::query_scalar(
        "INSERT INTO sgw_gate_mail (character_id, sender_id, subject, message, cash, sent_time, read_time, flags) \
         VALUES ($1, $2, 's', 'b', $3, 0, 0, $4) RETURNING mail_id",
    )
    .bind(character)
    .bind(sender)
    .bind(cash)
    .bind(flags)
    .fetch_one(pool)
    .await
    .expect("insert mail");
    sqlx::query(
        "INSERT INTO sgw_gate_mail_item \
            (mail_id, item_id, type_id, stack_size, charges, durability, flags, bound, \
             ammo, cur_ammo_type, ammo_type, ammo_types, source_character_id, escrowed_at) \
         VALUES ($1, $2, $3, 1, 0, -1, 0, false, 0, 0, 'AMMO_NONE', '{}', $4, 0)",
    )
    .bind(mail_id)
    .bind(item)
    .bind(type_id)
    .bind(sender)
    .execute(pool)
    .await
    .expect("insert mail item");
    mail_id
}

/// Deleting a character who sold one auction, held a 50-naquadah bid on
/// another, and received a gift and an unpaid COD (each with an item in
/// escrow) logs one WARN with those counts. Removing the audit block fails it.
#[tokio::test]
async fn live_db_character_delete_logs_cascade_counts() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 46, 2, &[]).await;
    let (p0, p1) = (fx.player(0), fx.player(1));
    // Item instance ids must be unique and at least 10000; this block owns
    // 0x6000_0000 + its offset within the module's sentinel range.
    let item_base = 0x6000_0000 + (fx.account_id - 0x7000_4800);
    let type_id: i32 =
        sqlx::query_scalar("SELECT item_id FROM resources.items ORDER BY item_id LIMIT 1")
            .fetch_one(&pool)
            .await
            .expect("an item definition");

    // p0 sells one auction, and holds the standing bid on p1's auction.
    seed_auction(&pool, p0, None, item_base + 10, type_id).await;
    seed_auction(&pool, p1, Some((p0, 50)), item_base + 11, type_id).await;
    // p0 has a gift and an unpaid COD in escrow, both from p1.
    seed_mail(&pool, p0, p1, 0, 0, item_base, type_id).await;
    seed_mail(&pool, p0, p1, 2, 500, item_base + 1, type_id).await;

    let capture = LogCapture::install();
    let deletion = delete_character(&pool, p0, fx.account_id)
        .await
        .expect("character delete");
    assert!(deletion.deleted);

    let c = capture
        .all()
        .into_iter()
        .find(|c| {
            c.level == Level::WARN
                && c.target == "org"
                && c.has_field("event", "character.delete_cascade")
        })
        .expect("character.delete_cascade logged at WARN");
    for (k, v) in [
        ("player_id", p0.to_string()),
        ("account_id", fx.account_id.to_string()),
        ("auctions_destroyed", "1".to_string()),
        ("held_cash_destroyed", "50".to_string()),
        ("bids_held_lost", "1".to_string()),
        ("mail_destroyed", "2".to_string()),
        ("mail_attachments_destroyed", "2".to_string()),
        ("cod_mails", "1".to_string()),
    ] {
        assert!(c.has_field(k, &v), "{k} = {v} in {c:?}");
    }
    drop(capture);

    teardown(&pool, &fx).await;
}
