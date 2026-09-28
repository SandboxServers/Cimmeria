//! Type 5 (concurrency): a return racing a payment or a take on the same
//! mail (SS-M3, audit § 6 CAT-G-04 and CAT-G-06). Sentinels: accounts,
//! players and entities `0x7300_1C20` up, items `0x7300_1CA0` up.

use std::sync::atomic::AtomicI64;
use std::time::Instant;

use super::packets::Client;
use super::take_race::{counted, open_gate, release_when_parked, wide_pool};
use super::*;

const BASE: i32 = 0x7300_1C20;
const ITEMS: i32 = 0x7300_1CA0;

/// Buyer (`base + 1`, 1,000 naquadah) and seller (`base + 2`) on account
/// `base`.
async fn pair(pool: &PgPool, base: i32, tag: &str) -> (i32, i32) {
    cleanup(pool, base).await;
    let (buyer, seller) = (base + 1, base + 2);
    insert_players(
        pool,
        base,
        &[
            (buyer, &format!("SsmThreeRrB{tag}")),
            (seller, &format!("SsmThreeRrS{tag}")),
        ],
    )
    .await;
    set_naquadah(pool, buyer, 1_000).await;
    (buyer, seller)
}

/// Pay and return race on one unpaid COD (price 300, one item). Exactly
/// one wins, whatever the order:
///
/// - pay first: the buyer is debited 300 once, the seller holds one payment
///   mail of 300, and the COD mail stays with the buyer (paid, so the
///   return is refused `cod_paid`);
/// - return first: nothing is debited, there is no payment mail, and the
///   COD mail is back with the seller, cancelled, price zeroed (the payment
///   then finds no mail of the buyer's).
///
/// Either way the item exists once, in escrow on the one mail, and the
/// seller never ends up with both the item and the price. Revert that
/// proves it: remove the advisory lock, the mail row `FOR UPDATE` and the
/// return's `AND NOT cod_paid` together; both commit, and the seller gets
/// the item back as well as the payment.
#[tokio::test]
async fn live_db_concurrent_pay_and_return_exactly_one_wins() {
    let pool = require_db_or_skip!();
    let base = BASE;
    let (buyer, seller) = pair(&pool, base, "Pay").await;
    let type_id = any_type_id(&pool).await;
    let mail_id = AttachedMail::from(buyer, seller, "SsmThreeRrSPay")
        .cod(300)
        .item(ITEMS, type_id, 1)
        .insert(&pool)
        .await;

    let ca = Client::new(base as u32 + 0x10, buyer, 55_170, "SsmThreeRrBPay");
    let cb = Client::new(base as u32 + 0x11, buyer, 55_171, "SsmThreeRrBPay");
    let now = Instant::now();
    let ops = wide_pool().await;
    let (gate, gate_pid) = open_gate(&ops).await;
    let done = AtomicI64::new(0);
    tokio::join!(
        counted(ca.op(MailOp::PayCod { mail_id }, Some(&ops), now), &done),
        counted(cb.op(MailOp::Return { mail_id }, Some(&ops), now), &done),
        release_when_parked(&ops, gate, gate_pid, 2, &done),
    );
    ca.take();
    cb.take();

    let payments: Vec<i64> = sqlx::query_scalar(
        "SELECT cash FROM sgw_gate_mail WHERE character_id = $1 AND mail_id <> $2",
    )
    .bind(seller)
    .bind(mail_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    let state = mail_state(&pool, mail_id).await;
    if payments.is_empty() {
        assert_eq!(naquadah(&pool, buyer).await, 1_000, "return won: no debit");
        assert_eq!(state, Some((seller, 0, 0, true)), "return won");
    } else {
        assert_eq!(payments, vec![300], "pay won: one payment");
        assert_eq!(naquadah(&pool, buyer).await, 700, "pay won: one debit");
        assert_eq!(state, Some((buyer, 0, 0, false)), "pay won: not returned");
    }
    assert!(
        has_escrow(&pool, mail_id).await,
        "the item is on the one mail"
    );
    assert_eq!(escrow_from(&pool, seller).await, 1);

    cleanup(&pool, base).await;
}

/// Take-cash and return race on one mail with 500 gift cash. Both may
/// succeed (take then return sends the emptied mail back), but the 500
/// exists once: what the owner was credited plus what the returned mail
/// still holds is exactly 500.
#[tokio::test]
async fn live_db_concurrent_take_cash_and_return_move_the_cash_once() {
    let pool = require_db_or_skip!();
    let base = BASE + 0x08;
    let (owner, sender) = pair(&pool, base, "Cash").await;
    let mail_id = AttachedMail::from(owner, sender, "SsmThreeRrSCash")
        .cash(500)
        .insert(&pool)
        .await;

    let ca = Client::new(base as u32 + 0x10, owner, 55_172, "SsmThreeRrBCash");
    let cb = Client::new(base as u32 + 0x11, owner, 55_173, "SsmThreeRrBCash");
    let now = Instant::now();
    let ops = wide_pool().await;
    let (gate, gate_pid) = open_gate(&ops).await;
    let done = AtomicI64::new(0);
    tokio::join!(
        counted(ca.op(MailOp::TakeCash { mail_id }, Some(&ops), now), &done),
        counted(cb.op(MailOp::Return { mail_id }, Some(&ops), now), &done),
        release_when_parked(&ops, gate, gate_pid, 2, &done),
    );
    ca.take();
    cb.take();

    let credited = i64::from(naquadah(&pool, owner).await - 1_000);
    let (holder, left, _, returned) = mail_state(&pool, mail_id).await.expect("mail kept");
    assert_eq!(credited + left, 500, "the cash moved once");
    assert!(returned, "the return commits in either order");
    assert_eq!(holder, sender);

    cleanup(&pool, base).await;
}
