//! Live-DB guards for `returnMailMessage` (SS-M3, audit § 6 CAT-G-06,
//! D-SS10). Sentinels: accounts, players and entities `0x7300_18C0` up,
//! items `0x7300_19C0` up.

use std::time::Instant;

use cimmeria_entity::inventory::INV_MAIN;

use super::packets::{Client, Received};
use super::*;
use crate::cell::mail::codes::flags::MAIL_ARCHIVE;
use crate::mercury::method_idx;
use crate::test_support::LogCapture;

const BASE: i32 = 0x7300_18C0;
const ITEMS: i32 = 0x7300_19C0;

/// Returner (`base + 1`), sender (`base + 2`) and a bystander (`base + 3`)
/// on account `base`.
async fn three_players(pool: &PgPool, base: i32, tag: &str) -> (i32, i32, i32) {
    cleanup(pool, base).await;
    let (returner, sender, other) = (base + 1, base + 2, base + 3);
    insert_players(
        pool,
        base,
        &[
            (returner, &format!("SsmThreeRetR{tag}")),
            (sender, &format!("SsmThreeRetS{tag}")),
            (other, &format!("SsmThreeRetX{tag}")),
        ],
    )
    .await;
    (returner, sender, other)
}

/// `(sender_id, sender_name, read_time)` of a mail.
async fn sender_of(pool: &PgPool, mail_id: i32) -> (Option<i32>, String, i32) {
    sqlx::query_as("SELECT sender_id, sender_name, read_time FROM sgw_gate_mail WHERE mail_id = $1")
        .bind(mail_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// CAT-G-06: the destination is the stored `sender_id`, never
/// `sender_name`. The mail's `sender_name` names a third, real player; the
/// return still goes to `sender_id`, with its gift cash and its escrowed
/// item, marked returned, unread, "from" the returner. The returner's
/// client drops the header. Fails if the return resolves the name.
#[tokio::test]
async fn return_uses_sender_id_not_name() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (returner, sender, other) = three_players(&pool, BASE, "Name").await;
    let type_id = any_type_id(&pool).await;
    let item_id = ITEMS;
    let mail_id = AttachedMail::from(returner, sender, "SsmThreeRetXName")
        .cash(120)
        .item(item_id, type_id, 2)
        .insert(&pool)
        .await;
    sqlx::query("UPDATE sgw_gate_mail SET read_time = 55 WHERE mail_id = $1")
        .bind(mail_id)
        .execute(&pool)
        .await
        .unwrap();

    let c = Client::new(BASE as u32 + 0x20, returner, 55_140, "SsmThreeRetRName");
    c.op(MailOp::Return { mail_id }, Some(&pool), Instant::now())
        .await;
    assert_eq!(
        c.take(),
        vec![Received::Other(method_idx::ON_MAIL_HEADER_REMOVE)]
    );
    assert_eq!(
        mail_state(&pool, mail_id).await,
        Some((sender, 120, 0, true))
    );
    assert_eq!(
        sender_of(&pool, mail_id).await,
        (Some(returner), "SsmThreeRetRName".to_string(), 0)
    );
    assert_eq!(mail_count(&pool, other).await, 0, "never the named player");
    assert_eq!(
        escrow_for(&pool, sender).await.len(),
        1,
        "the item went back with it"
    );
    let ev = capture
        .all()
        .into_iter()
        .find(|e| e.has_field("event", "mail.returned"))
        .expect("mail.returned");
    for (k, v) in [
        ("target_player_id", sender.to_string()),
        ("cash", "120".to_string()),
        ("item_id", item_id.to_string()),
        ("cod_cancelled", "0".to_string()),
    ] {
        assert!(ev.has_field(k, &v), "{k}={v}: {ev:?}");
    }

    // The sender can take both back.
    let s = Client::new(BASE as u32 + 0x21, sender, 55_141, "SsmThreeRetSName");
    let now = Instant::now();
    s.op(MailOp::TakeCash { mail_id }, Some(&pool), now).await;
    s.op(
        MailOp::TakeItem {
            mail_id,
            container_id: -1,
            slot_id: -1,
        },
        Some(&pool),
        now,
    )
    .await;
    assert_eq!(naquadah(&pool, sender).await, 120);
    assert_eq!(
        inventory_rows(&pool, item_id).await,
        vec![(sender, INV_MAIN, 0, 2)]
    );

    cleanup(&pool, BASE).await;
}

/// CAT-G-06 / D-SS10: a returned mail cannot be returned again, so a return
/// can never loop. The original sender's return is refused
/// `already_returned` and the mail stays with them. Fails when both
/// `returned` gates are removed, the Rust check and the UPDATE's
/// `AND NOT returned` (the mail bounces back); either alone refuses.
#[tokio::test]
async fn return_rejects_already_returned() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (returner, sender, _) = three_players(&pool, BASE + 0x08, "Twice").await;
    let mail_id = AttachedMail::from(returner, sender, "SsmThreeRetSTwice")
        .cash(10)
        .insert(&pool)
        .await;
    let now = Instant::now();
    Client::new(BASE as u32 + 0x28, returner, 55_142, "SsmThreeRetRTwice")
        .op(MailOp::Return { mail_id }, Some(&pool), now)
        .await;

    let s = Client::new(BASE as u32 + 0x29, sender, 55_143, "SsmThreeRetSTwice");
    s.op(MailOp::Return { mail_id }, Some(&pool), now).await;
    assert_eq!(
        s.take(),
        vec![Received::Feedback(
            "That gate-mail message has already been returned once and cannot be returned \
             again."
                .to_string()
        )]
    );
    assert_eq!(
        mail_state(&pool, mail_id).await,
        Some((sender, 10, 0, true))
    );
    assert_refused(&capture, "return", "already_returned", mail_id);

    cleanup(&pool, BASE + 0x08).await;
}

/// CAT-G-06: server mail (no `sender_id`) has nobody to return to; refused
/// `system_mail`, untouched. Revert that proves it: fall back to any
/// destination for a NULL `sender_id` (for example the returner); the mail
/// is then marked returned.
#[tokio::test]
async fn return_rejects_system_mail() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (returner, _, _) = three_players(&pool, BASE + 0x10, "Sys").await;
    let mut mail = AttachedMail::from(returner, 0, "System").cash(5);
    mail.sender_id = None;
    let mail_id = mail.insert(&pool).await;

    let c = Client::new(BASE as u32 + 0x30, returner, 55_144, "SsmThreeRetRSys");
    c.op(MailOp::Return { mail_id }, Some(&pool), Instant::now())
        .await;
    assert_eq!(
        c.take(),
        vec![Received::Feedback(
            "That gate-mail message has no player sender to return it to.".to_string()
        )]
    );
    assert_eq!(
        mail_state(&pool, mail_id).await,
        Some((returner, 5, 0, false))
    );
    assert_refused(&capture, "return", "system_mail", mail_id);

    cleanup(&pool, BASE + 0x10).await;
}

/// D-SS10: only non-archived mail can be returned. Refused `archived`.
#[tokio::test]
async fn return_rejects_archived() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (returner, sender, _) = three_players(&pool, BASE + 0x18, "Arch").await;
    let mut mail = AttachedMail::from(returner, sender, "SsmThreeRetSArch");
    mail.flags = MAIL_ARCHIVE;
    let mail_id = mail.insert(&pool).await;

    let c = Client::new(BASE as u32 + 0x38, returner, 55_145, "SsmThreeRetRArch");
    c.op(MailOp::Return { mail_id }, Some(&pool), Instant::now())
        .await;
    assert!(matches!(&c.take()[..], [Received::Feedback(_)]));
    assert_eq!(
        mail_state(&pool, mail_id).await,
        Some((returner, 0, MAIL_ARCHIVE, false))
    );
    assert_refused(&capture, "return", "archived", mail_id);

    cleanup(&pool, BASE + 0x18).await;
}

/// D-SS10 / D-SS04: returning an unpaid COD cancels it and **zeroes the
/// price**, so the price never reaches the sender as gift cash; the item
/// goes back in escrow for the sender to take. Fails when the return keeps
/// `cash` for a COD (the sender could take their own price).
#[tokio::test]
async fn return_cancels_cod_and_zeroes_price() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (returner, sender, _) = three_players(&pool, BASE + 0x20, "Cod").await;
    let type_id = any_type_id(&pool).await;
    let item_id = ITEMS + 0x20;
    let mail_id = AttachedMail::from(returner, sender, "SsmThreeRetSCod")
        .cod(300)
        .item(item_id, type_id, 1)
        .insert(&pool)
        .await;
    let now = Instant::now();
    Client::new(BASE as u32 + 0x40, returner, 55_146, "SsmThreeRetRCod")
        .op(MailOp::Return { mail_id }, Some(&pool), now)
        .await;
    assert_eq!(mail_state(&pool, mail_id).await, Some((sender, 0, 0, true)));
    assert!(has_escrow(&pool, mail_id).await);
    assert!(capture
        .all()
        .iter()
        .any(|e| e.has_field("event", "mail.returned") && e.has_field("cod_cancelled", "300")));

    let s = Client::new(BASE as u32 + 0x41, sender, 55_147, "SsmThreeRetSCod");
    s.op(MailOp::TakeCash { mail_id }, Some(&pool), now).await;
    assert_refused(&capture, "take_cash", "no_cash", mail_id);
    s.op(
        MailOp::TakeItem {
            mail_id,
            container_id: -1,
            slot_id: -1,
        },
        Some(&pool),
        now,
    )
    .await;
    assert_eq!(naquadah(&pool, sender).await, 0);
    assert_eq!(inventory_rows(&pool, item_id).await.len(), 1);

    cleanup(&pool, BASE + 0x20).await;
}

/// Coordinator decision on the SS-M3 review (F2): a paid COD belongs to the
/// buyer. Once paid, the mail looks like an ordinary item mail, so without
/// `cod_paid` its recipient could return it and the seller would get both
/// the item and the price (and SS-M4's expiry would do it unasked). The
/// return is refused `cod_paid` with feedback; the seller is credited the
/// price exactly once, by the payment mail; the item stays in escrow for
/// the buyer, who can still take it. Fails when both `cod_paid` gates (the
/// Rust check and the UPDATE's `AND NOT cod_paid`) are removed, or when the
/// payment stops setting it.
#[tokio::test]
async fn return_rejects_paid_cod() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (buyer, seller, _) = three_players(&pool, BASE + 0x28, "Paid").await;
    set_naquadah(&pool, buyer, 1_000).await;
    let type_id = any_type_id(&pool).await;
    let item_id = ITEMS + 0x28;
    let mail_id = AttachedMail::from(buyer, seller, "SsmThreeRetSPaid")
        .cod(300)
        .item(item_id, type_id, 1)
        .insert(&pool)
        .await;

    let b = Client::new(BASE as u32 + 0x48, buyer, 55_148, "SsmThreeRetRPaid");
    let now = Instant::now();
    b.op(MailOp::PayCod { mail_id }, Some(&pool), now).await;
    b.take();
    b.op(MailOp::Return { mail_id }, Some(&pool), now).await;
    assert_eq!(
        b.take(),
        vec![Received::Feedback(
            "You have already paid for this COD delivery, so it cannot be returned. Take \
             the item instead."
                .to_string()
        )]
    );
    assert_refused(&capture, "return", "cod_paid", mail_id);
    assert_eq!(mail_state(&pool, mail_id).await, Some((buyer, 0, 0, false)));
    assert!(
        has_escrow(&pool, mail_id).await,
        "the item waits for the buyer"
    );

    // The seller holds exactly the one payment mail, and is credited once.
    let seller_mail: Vec<(i32, i64)> =
        sqlx::query_as("SELECT mail_id, cash FROM sgw_gate_mail WHERE character_id = $1")
            .bind(seller)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(seller_mail.len(), 1, "{seller_mail:?}");
    assert_eq!(seller_mail[0].1, 300);
    let s = Client::new(BASE as u32 + 0x49, seller, 55_149, "SsmThreeRetSPaid");
    s.op(
        MailOp::TakeCash {
            mail_id: seller_mail[0].0,
        },
        Some(&pool),
        now,
    )
    .await;
    assert_eq!(naquadah(&pool, seller).await, 300);
    assert!(
        escrow_for(&pool, seller).await.is_empty(),
        "no item for the seller"
    );

    b.op(
        MailOp::TakeItem {
            mail_id,
            container_id: -1,
            slot_id: -1,
        },
        Some(&pool),
        now,
    )
    .await;
    assert_eq!(
        inventory_rows(&pool, item_id).await,
        vec![(buyer, INV_MAIN, 0, 1)]
    );

    cleanup(&pool, BASE + 0x28).await;
}
