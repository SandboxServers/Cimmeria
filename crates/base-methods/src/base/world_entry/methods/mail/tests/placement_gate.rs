//! Live-DB guards that mail never takes an item, or a COD price, that the
//! take could not deliver (ss-fix1). The take places an item by its
//! `container_sets` (SS-M4, `take::carried_bag`) and refuses a type no
//! carried bag may hold, such as the 801 mission-only `{2}` types. Before
//! ss-fix1 the send escrowed such an item from the backpack (a GM grant
//! puts one there) and the COD payment charged for it; the take then
//! refused it for good, and a paid COD cannot be returned, so the payer
//! lost the price and never got the item. The two-client wire test
//! `two_client_mail_cod` hit exactly this with a `{2}` fixture.
//! Sentinels: accounts and players `0x7300_2700` up, entities
//! `0x7300_2740` up, items `0x7300_2780` up.

use std::time::Instant;

use super::attach_live::{assert_untouched, attached, reply, setup};
use super::packets::Client;
use super::*;
use crate::cell::mail::codes::flags::MAIL_COD;
use crate::cell::mail::codes::MailResult;
use crate::test_support::LogCapture;

const BASE: i32 = 0x7300_2700;
const ENTITIES: u32 = 0x7300_2740;
const ITEMS: i32 = 0x7300_2780;

/// A type whose `container_sets` names no carried bag.
async fn mission_only_type(pool: &PgPool) -> i32 {
    sqlx::query_scalar(
        "SELECT item_id FROM resources.items WHERE container_sets = ARRAY[2] \
         ORDER BY item_id LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("the seed has mission-only items")
}

/// A mission-only item in the sender's backpack is refused
/// `item_no_carried_bag` with `ItemNotAvailable` and a feedback line;
/// nothing is debited, escrowed or mailed. Fails without the send's
/// `carried_bag` gate (the item is escrowed and the mail sent).
#[tokio::test]
async fn send_refuses_an_item_no_take_could_place() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, sender, rcpt) = (BASE, BASE + 1, BASE + 2);
    setup(
        &pool,
        acct,
        (sender, "SsFixOneSend"),
        (rcpt, "SsFixOneRcpt"),
        500,
    )
    .await;
    let item = TestItem::main(ITEMS, sender, 0, 1);
    insert_item(&pool, item, mission_only_type(&pool).await).await;

    let c = Client::new(ENTITIES, sender, 55_700, "SsFixOneSend");
    c.op(
        MailOp::Send(attached("SsFixOneRcpt", 300, true, item.item_id, 1)),
        Some(&pool),
        Instant::now(),
    )
    .await;

    let r = reply(c.take());
    assert_eq!(r.code, Some(MailResult::ItemNotAvailable.code()), "{r:?}");
    assert_eq!(
        r.lines,
        vec![
            "That item cannot be carried in a backpack or crafting bag, so it cannot be \
             sent by gate-mail. The message was not sent."
                .to_string()
        ]
    );
    assert!(r.cash.is_empty() && r.removed.is_empty(), "{r:?}");
    assert!(
        capture
            .all()
            .iter()
            .any(|e| e.message_contains("sendMailMessage refused")
                && e.has_field("reason", "item_no_carried_bag")),
        "reason=item_no_carried_bag"
    );
    assert_untouched(&pool, sender, rcpt, 500, &[item]).await;

    cleanup(&pool, acct).await;
}

/// The round trip `two_client_mail_cod` broke on: a COD whose escrowed item
/// no carried bag may hold (already in escrow, e.g. mailed before the send
/// gate) is refused at payment, `no_carried_bag`, before any debit. The
/// mail stays an unpaid, returnable COD with its escrow, no payment mail is
/// written, and the take is still refused. Fails without the payment's
/// `carried_bag` gate: the payer is charged 300 and the take then refuses
/// the item for good.
#[tokio::test]
async fn pay_cod_refuses_an_item_no_take_could_place() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let base = BASE + 0x10;
    cleanup(&pool, base).await;
    let (payer, sender) = (base + 1, base + 2);
    insert_players(
        &pool,
        base,
        &[(payer, "SsFixOnePayer"), (sender, "SsFixOneSeller")],
    )
    .await;
    set_naquadah(&pool, payer, 1_000).await;
    let item_id = ITEMS + 1;
    let mail_id = AttachedMail::from(payer, sender, "SsFixOneSeller")
        .cod(300)
        .item(item_id, mission_only_type(&pool).await, 1)
        .insert(&pool)
        .await;

    let c = Client::new(ENTITIES + 1, payer, 55_701, "SsFixOnePayer");
    c.op(MailOp::PayCod { mail_id }, Some(&pool), Instant::now())
        .await;
    c.op(
        MailOp::TakeItem {
            mail_id,
            container_id: -1,
            slot_id: -1,
        },
        Some(&pool),
        Instant::now(),
    )
    .await;

    assert_eq!(naquadah(&pool, payer).await, 1_000, "nothing charged");
    let (cash, flags, cod_paid): (i64, i32, bool) =
        sqlx::query_as("SELECT cash, flags, cod_paid FROM sgw_gate_mail WHERE mail_id = $1")
            .bind(mail_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        (cash, flags & MAIL_COD, cod_paid),
        (300, MAIL_COD, false),
        "still an unpaid, returnable COD"
    );
    let payment_mails: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_gate_mail WHERE character_id = $1")
            .bind(sender)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(payment_mails, 0, "no payment mail");
    assert!(has_escrow(&pool, mail_id).await);
    assert_eq!(inventory_rows(&pool, item_id).await, vec![]);
    assert_refused(&capture, "pay_cod", "no_carried_bag", mail_id);
    c.take();

    cleanup(&pool, base).await;
}
