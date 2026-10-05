//! Live-DB guards for `payCODForMailMessage` (SS-M3, audit § 6 CAT-G-05,
//! D-SS09). Sentinels: accounts, players and entities `0x7300_18xx`
//! (`0x7300_1880` up), items `0x7300_19xx`.

use std::time::Instant;

use super::packets::{Client, Received};
use super::*;
use crate::cell::mail::codes::flags::MAIL_COD;
use crate::mercury::method_idx;
use crate::test_support::LogCapture;

const BASE: i32 = 0x7300_1880;
const ITEMS: i32 = 0x7300_1980;

/// Payer (`base + 1`, 1,000 naquadah) and COD sender (`base + 2`, 0) on
/// account `base`, and a COD mail from the sender to the payer: an item
/// and a price of 300. Returns `(payer, sender, mail_id, item_id)`.
async fn cod_fixture(pool: &PgPool, base: i32, tag: &str) -> (i32, i32, i32, i32) {
    cleanup(pool, base).await;
    let (payer, sender) = (base + 1, base + 2);
    let sender_name = format!("SsmThreeCodS{tag}");
    insert_players(
        pool,
        base,
        &[
            (payer, &format!("SsmThreeCodP{tag}")),
            (sender, &sender_name),
        ],
    )
    .await;
    set_naquadah(pool, payer, 1_000).await;
    let type_id = any_type_id(pool).await;
    let item_id = ITEMS + (base & 0x7f);
    let mail_id = AttachedMail::from(payer, sender, &sender_name)
        .cod(300)
        .item(item_id, type_id, 1)
        .insert(pool)
        .await;
    (payer, sender, mail_id, item_id)
}

/// Payment mails the COD sender holds: `(mail_id, cash, sender_id,
/// sender_name)`.
async fn payment_mails(pool: &PgPool, sender: i32) -> Vec<(i32, i64, Option<i32>, String)> {
    sqlx::query_as(
        "SELECT mail_id, cash, sender_id, sender_name FROM sgw_gate_mail \
         WHERE character_id = $1 ORDER BY mail_id",
    )
    .bind(sender)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// CAT-G-05: paying twice debits once. The first payment debits the stored
/// 300, clears `MAIL_COD` and zeroes `cash` on the COD mail (the delete
/// guard keys on `cash = 0`, and the price must never be takeable), and
/// mails the 300 to the sender as server mail (`sender_id` NULL, the payer's
/// name). The payer's client gets `onCashChanged` and the refreshed header.
/// The second payment is refused `not_cod`. Revert that proves it: stop
/// zeroing `cash` in the clear and drop the `mail.cod()` gate (the second
/// payment debits 300 again). Dropping every COD gate while still zeroing
/// fails it too, on the second payment mail (of 0).
#[tokio::test]
async fn live_db_pay_cod_twice_debits_once() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (payer, sender, mail_id, _) = cod_fixture(&pool, BASE, "Twice").await;

    let c = Client::new(BASE as u32 + 0x40, payer, 55_120, "SsmThreeCodPTwice");
    let now = Instant::now();
    c.op(MailOp::PayCod { mail_id }, Some(&pool), now).await;
    let first = c.take();
    assert_eq!(first[0], Received::CashChanged(700), "{first:?}");
    assert_eq!(first[1], Received::Other(method_idx::ON_MAIL_HEADER_REMOVE));
    match &first[2] {
        Received::HeaderInfo {
            headers,
            cash,
            attachments,
            ..
        } => {
            assert_eq!(headers, &vec![(mail_id, 0)], "COD flag cleared");
            assert_eq!(cash, &vec![0], "price zeroed");
            assert_eq!(attachments.len(), 1, "the item is still there to take");
        }
        other => panic!("expected the refreshed header, got {other:?}"),
    }
    assert_eq!(naquadah(&pool, payer).await, 700);
    assert_eq!(mail_state(&pool, mail_id).await, Some((payer, 0, 0, false)));
    let payments = payment_mails(&pool, sender).await;
    assert_eq!(payments.len(), 1, "{payments:?}");
    assert_eq!(
        (payments[0].1, payments[0].2, payments[0].3.as_str()),
        (300, None, "SsmThreeCodPTwice")
    );
    let paid = capture
        .all()
        .into_iter()
        .find(|e| e.has_field("event", "mail.cod_paid"))
        .expect("mail.cod_paid");
    for (k, v) in [
        ("price", "300"),
        ("naquadah_before", "1000"),
        ("naquadah_after", "700"),
        ("target_player_id", &sender.to_string()),
        ("payment_mail_id", &payments[0].0.to_string()),
        ("target_player_name", "SsmThreeCodSTwice"),
    ] {
        assert!(paid.has_field(k, v), "{k}={v}: {paid:?}");
    }
    assert_actor_names(&paid, "SsmThreeCodPTwice");

    c.op(MailOp::PayCod { mail_id }, Some(&pool), now).await;
    assert_eq!(
        c.take(),
        vec![Received::Feedback(
            "That gate-mail message has no COD to pay.".to_string()
        )]
    );
    assert_eq!(naquadah(&pool, payer).await, 700, "debited once");
    assert_eq!(payment_mails(&pool, sender).await.len(), 1);
    assert_refused(&capture, "pay_cod", "not_cod", mail_id);

    cleanup(&pool, BASE).await;
}

/// CAT-G-05: a payer who cannot cover the price is refused
/// `not_enough_cash`; the COD, the price, the item and the sender's mailbox
/// are untouched. Fails when the debit's `naquadah >= $1` guard is removed
/// (the balance goes negative, or the schema check turns it into
/// `db_error`).
#[tokio::test]
async fn live_db_pay_cod_rejects_insufficient_cash() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (payer, sender, mail_id, _) = cod_fixture(&pool, BASE + 0x08, "Poor").await;
    set_naquadah(&pool, payer, 299).await;

    let c = Client::new(BASE as u32 + 0x48, payer, 55_121, "SsmThreeCodPPoor");
    c.op(MailOp::PayCod { mail_id }, Some(&pool), Instant::now())
        .await;
    assert_eq!(
        c.take(),
        vec![Received::Feedback(
            "You do not have enough naquadah to pay this COD.".to_string()
        )]
    );
    assert_eq!(naquadah(&pool, payer).await, 299);
    assert_eq!(
        mail_state(&pool, mail_id).await,
        Some((payer, 300, MAIL_COD, false))
    );
    assert!(has_escrow(&pool, mail_id).await);
    assert!(payment_mails(&pool, sender).await.is_empty());
    assert_refused(&capture, "pay_cod", "not_enough_cash", mail_id);

    cleanup(&pool, BASE + 0x08).await;
}

/// CAT-G-05: the price comes from the stored row. `payCODForMailMessage`
/// carries only the mail id, so there is no client number to trust; what
/// this pins is that the debit, the payment mail and the row agree at pay
/// time. The row's price is changed after the header went out; the payment
/// debits and forwards the stored 450, not the 300 the header showed. A
/// revert that debits a cached or constant price fails it.
#[tokio::test]
async fn live_db_pay_cod_amount_read_from_row() {
    let pool = require_db_or_skip!();
    let (payer, sender, mail_id, _) = cod_fixture(&pool, BASE + 0x10, "Row").await;
    sqlx::query("UPDATE sgw_gate_mail SET cash = 450 WHERE mail_id = $1")
        .bind(mail_id)
        .execute(&pool)
        .await
        .unwrap();

    let c = Client::new(BASE as u32 + 0x50, payer, 55_122, "SsmThreeCodPRow");
    c.op(MailOp::PayCod { mail_id }, Some(&pool), Instant::now())
        .await;
    assert_eq!(c.take()[0], Received::CashChanged(550));
    assert_eq!(naquadah(&pool, payer).await, 550);
    let payments = payment_mails(&pool, sender).await;
    assert_eq!(payments.len(), 1);
    assert_eq!(payments[0].1, 450);

    cleanup(&pool, BASE + 0x10).await;
}

/// D-SS09 / packet acceptance: a paid COD credits the sender exactly once,
/// by mail, while the sender is offline (no session exists for them at all
/// during the payment). When the sender later logs in and takes the
/// payment's cash, they are credited 300 once; a second take and a second
/// payment change nothing. The item is taken by an ordinary take-item
/// after the payment, and is refused before it (`cod_unpaid`).
#[tokio::test]
async fn live_db_paid_cod_credits_sender_once_by_mail_while_offline() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (payer, sender, mail_id, item_id) = cod_fixture(&pool, BASE + 0x18, "Off").await;
    let c = Client::new(BASE as u32 + 0x58, payer, 55_123, "SsmThreeCodPOff");
    let now = Instant::now();
    let take_item = || MailOp::TakeItem {
        mail_id,
        container_id: -1,
        slot_id: -1,
    };

    c.op(take_item(), Some(&pool), now).await;
    assert!(matches!(&c.take()[..], [Received::Feedback(_)]));
    assert_refused(&capture, "take_item", "cod_unpaid", mail_id);
    assert!(inventory_rows(&pool, item_id).await.is_empty());

    c.op(MailOp::PayCod { mail_id }, Some(&pool), now).await;
    c.op(MailOp::PayCod { mail_id }, Some(&pool), now).await;
    c.op(take_item(), Some(&pool), now).await;
    c.take();
    assert_eq!(naquadah(&pool, payer).await, 700);
    assert_eq!(inventory_rows(&pool, item_id).await.len(), 1);
    let payments = payment_mails(&pool, sender).await;
    assert_eq!(payments.len(), 1, "one payment mail: {payments:?}");
    assert_eq!(
        naquadah(&pool, sender).await,
        0,
        "paid by mail, not directly"
    );

    // The sender logs in later and takes the payment.
    let payment_id = payments[0].0;
    let s = Client::new(BASE as u32 + 0x59, sender, 55_124, "SsmThreeCodSOff");
    s.op(
        MailOp::TakeCash {
            mail_id: payment_id,
        },
        Some(&pool),
        now,
    )
    .await;
    s.op(
        MailOp::TakeCash {
            mail_id: payment_id,
        },
        Some(&pool),
        now,
    )
    .await;
    assert_eq!(naquadah(&pool, sender).await, 300, "credited exactly once");
    // Server mail cannot be returned to the payer (D-SS10).
    s.op(
        MailOp::Return {
            mail_id: payment_id,
        },
        Some(&pool),
        now,
    )
    .await;
    assert_refused(&capture, "return", "system_mail", payment_id);

    cleanup(&pool, BASE + 0x18).await;
}

/// A COD whose sender's character was deleted (the foreign key sets
/// `sender_id` NULL) has nobody to pay and nobody to return to, and take and
/// delete refuse an unpaid COD, so without a way out its item is stranded
/// for good. Paying it cancels the COD instead: nothing is debited, the
/// price is zeroed and the flag cleared, the player is told, and the item
/// becomes an ordinary take. Fails when that branch is reverted to a plain
/// refusal (the take after it is refused `cod_unpaid`).
#[tokio::test]
async fn live_db_pay_cod_with_deleted_sender_cancels_cod_and_frees_item() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (payer, sender, mail_id, item_id) = cod_fixture(&pool, BASE + 0x20, "Gone").await;
    sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(sender)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        mail_state(&pool, mail_id).await,
        Some((payer, 300, MAIL_COD, false))
    );

    let c = Client::new(BASE as u32 + 0x60, payer, 55_125, "SsmThreeCodPGone");
    let now = Instant::now();
    c.op(MailOp::PayCod { mail_id }, Some(&pool), now).await;
    let seen = c.take();
    assert_eq!(
        seen[0],
        Received::Feedback(
            "The sender of that COD message no longer exists. The COD is cancelled and \
             nothing was charged; the item is yours to take."
                .to_string()
        )
    );
    assert_eq!(naquadah(&pool, payer).await, 1_000, "nothing charged");
    assert_eq!(mail_state(&pool, mail_id).await, Some((payer, 0, 0, false)));
    assert!(capture.all().iter().any(|e| {
        e.has_field("event", "mail.cod_cancelled")
            && e.has_field("reason", "sender_gone")
            && e.has_field("price", "300")
            && e.has_field("sender_name", "SsmThreeCodSGone")
            && e.has_field("mail_id", &mail_id.to_string())
    }));

    c.op(
        MailOp::TakeItem {
            mail_id,
            container_id: -1,
            slot_id: -1,
        },
        Some(&pool),
        now,
    )
    .await;
    assert_eq!(inventory_rows(&pool, item_id).await.len(), 1);
    assert!(!has_escrow(&pool, mail_id).await);

    cleanup(&pool, BASE + 0x20).await;
}

/// Pay and return are owner-scoped like the takes: another character naming
/// the mail id is refused `not_found_for_owner`, and nothing moves.
#[tokio::test]
async fn live_db_pay_and_return_refuse_another_players_mail() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (payer, sender, mail_id, _) = cod_fixture(&pool, BASE + 0x30, "Owner").await;
    let intruder = Client::new(BASE as u32 + 0x70, sender, 55_127, "SsmThreeCodSOwner");
    let now = Instant::now();
    intruder
        .op(MailOp::PayCod { mail_id }, Some(&pool), now)
        .await;
    intruder
        .op(MailOp::Return { mail_id }, Some(&pool), now)
        .await;
    assert_refused(&capture, "pay_cod", "not_found_for_owner", mail_id);
    assert_refused(&capture, "return", "not_found_for_owner", mail_id);
    assert_eq!(naquadah(&pool, sender).await, 0);
    assert_eq!(
        mail_state(&pool, mail_id).await,
        Some((payer, 300, MAIL_COD, false))
    );

    cleanup(&pool, BASE + 0x30).await;
}

/// A COD with no item left has nothing to sell: refused
/// `cod_without_item`, nothing debited (the send path never makes one; this
/// pins the defence).
#[tokio::test]
async fn live_db_pay_cod_refuses_without_item() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (payer, _, mail_id, _) = cod_fixture(&pool, BASE + 0x28, "NoItem").await;
    sqlx::query("DELETE FROM sgw_gate_mail_item WHERE mail_id = $1")
        .bind(mail_id)
        .execute(&pool)
        .await
        .unwrap();

    let c = Client::new(BASE as u32 + 0x68, payer, 55_126, "SsmThreeCodPNoItem");
    c.op(MailOp::PayCod { mail_id }, Some(&pool), Instant::now())
        .await;
    assert!(matches!(&c.take()[..], [Received::Feedback(_)]));
    assert_eq!(naquadah(&pool, payer).await, 1_000);
    assert_refused(&capture, "pay_cod", "cod_without_item", mail_id);

    cleanup(&pool, BASE + 0x28).await;
}

/// The payment subject fits `varchar(128)` whatever the original subject.
#[test]
fn payment_subject_is_capped_at_the_column_width() {
    use super::super::cod::payment_subject;
    assert_eq!(payment_subject("Sword"), "COD payment: Sword");
    let long = "x".repeat(128);
    let s = payment_subject(&long);
    assert_eq!(s.chars().count(), 128);
    assert!(s.starts_with("COD payment: x"));
}
