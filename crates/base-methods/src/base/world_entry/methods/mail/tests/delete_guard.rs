//! Live-DB guards for `deleteMailMessage` once mail can hold value (SS-M2):
//! a mail with an escrowed item, gift cash or an unpaid COD is refused with
//! feedback and left whole; an empty mail deletes as before. Sentinels
//! `0x7300_15xx` and items `0x7300_17xx`.

use std::time::{Duration, Instant};

use super::packets::{plain_send, Client, Received};
use super::*;
use crate::cell::mail::codes::MailResult;
use crate::mercury::method_idx;
use crate::test_support::LogCapture;

const BASE: i32 = 0x7300_1500;
const ITEMS: i32 = 0x7300_1700;

/// Three attached mails for `rcpt` (an item, gift cash, COD with an item)
/// and one plain one, sent through the real send path. Returns their mail
/// ids in that order.
async fn attached_mails(pool: &PgPool, sender: i32, rcpt_name: &str, type_id: i32) -> [i32; 4] {
    let item = TestItem::main(ITEMS + (sender & 0xff), sender, 0, 1);
    let cod_item = TestItem::main(ITEMS + (sender & 0xff) + 1, sender, 1, 1);
    insert_item(pool, item, type_id).await;
    insert_item(pool, cod_item, type_id).await;
    set_naquadah(pool, sender, 1_000).await;

    let c = Client::new(0x7300_1580, sender, 54_750, "DelGuardSender");
    let t0 = Instant::now();
    let sends = [
        (0, false, item.item_id, 1),
        (60, false, 0, 0),
        (70, true, cod_item.item_id, 1),
    ];
    for (i, (cash, cod, item_id, quantity)) in sends.into_iter().enumerate() {
        let mut send = plain_send(&[rcpt_name]);
        send.cash = cash;
        send.cod = cod;
        send.item_id = item_id;
        send.item_quantity = quantity;
        c.op(
            MailOp::Send(send),
            Some(pool),
            t0 + Duration::from_secs(11 * i as u64),
        )
        .await;
    }
    c.op(
        MailOp::Send(plain_send(&[rcpt_name])),
        Some(pool),
        t0 + Duration::from_secs(33),
    )
    .await;
    for r in c.take() {
        if let Received::SendMailResult { result, .. } = r {
            assert_eq!(result, MailResult::Sent.code(), "fixture send failed");
        }
    }
    let ids: Vec<i32> = sqlx::query_scalar(
        "SELECT mail_id FROM sgw_gate_mail WHERE sender_id = $1 ORDER BY mail_id",
    )
    .bind(sender)
    .fetch_all(pool)
    .await
    .unwrap();
    ids.try_into().expect("four mails")
}

/// SS-M2 scope: `deleteMailMessage` on a mail that still holds an item,
/// gift cash or an unpaid COD leaves the mail and its escrow row as they
/// were, sends no `onMailHeaderRemove`, logs `mail.delete_refused` with the
/// reason, and tells the player why on the first press. A plain mail still
/// deletes. Fails when the delete guard is reverted (the three mails would
/// be deleted, and their escrow rows with them).
#[tokio::test]
async fn delete_refused_while_attachment_present() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, sender, rcpt) = (BASE, BASE + 1, BASE + 2);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(sender, "SsmTwoDelS"), (rcpt, "SsmTwoDelR")]).await;
    let type_id = any_type_id(&pool).await;
    let [item_mail, cash_mail, cod_mail, plain_mail] =
        attached_mails(&pool, sender, "SsmTwoDelR", type_id).await;

    let r = Client::new(0x7300_1581, rcpt, 54_751, "SsmTwoDelR");
    let now = Instant::now();
    let refused = [
        (
            item_mail,
            "attachment_item_present",
            "This gate-mail still holds an item. Take it or return the message before \
             deleting it.",
        ),
        (
            cash_mail,
            "attachment_cash_present",
            "This gate-mail still holds naquadah. Take it before deleting the message.",
        ),
        (
            cod_mail,
            "attachment_item_present",
            "This gate-mail still holds an item. Take it or return the message before \
             deleting it.",
        ),
    ];
    for (mail_id, reason, text) in refused {
        r.op(MailOp::Delete { mail_id }, Some(&pool), now).await;
        assert_eq!(
            r.take(),
            vec![Received::Feedback(text.to_string())],
            "mail {mail_id}: feedback only, no onMailHeaderRemove"
        );
        let ev = capture
            .all()
            .into_iter()
            .find(|e| {
                e.message_contains("delete refused")
                    && e.has_field("reason", reason)
                    && e.has_field("mail_id", &mail_id.to_string())
            })
            .unwrap_or_else(|| panic!("mail.delete_refused mail_id={mail_id} reason={reason}"));
        for key in ["account_id", "player_id", "entity_id", "mail_id"] {
            assert!(ev.fields.contains_key(key), "{key} missing: {ev:?}");
        }
    }
    assert_eq!(
        mail_count(&pool, rcpt).await,
        4,
        "no attached mail was deleted"
    );
    let escrowed: Vec<i32> = escrow_for(&pool, rcpt)
        .await
        .iter()
        .map(|e| e.mail_id)
        .collect();
    assert_eq!(escrowed, vec![item_mail, cod_mail]);

    r.op(
        MailOp::Delete {
            mail_id: plain_mail,
        },
        Some(&pool),
        now,
    )
    .await;
    assert_eq!(
        r.take(),
        vec![Received::Other(method_idx::ON_MAIL_HEADER_REMOVE)],
        "a plain mail deletes as before"
    );
    assert_eq!(mail_count(&pool, rcpt).await, 3);

    cleanup(&pool, acct).await;
}

/// An unpaid COD price is value too: a COD mail whose item was already
/// taken (simulated by removing its escrow row, as SS-M3's take will) is
/// still refused while the price is unpaid.
#[tokio::test]
async fn delete_refused_for_unpaid_cod_without_item() {
    let pool = require_db_or_skip!();
    let (acct, sender, rcpt) = (BASE + 10, BASE + 11, BASE + 12);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(sender, "SsmTwoCodS"), (rcpt, "SsmTwoCodR")]).await;
    let type_id = any_type_id(&pool).await;
    let [_, _, cod_mail, _] = attached_mails(&pool, sender, "SsmTwoCodR", type_id).await;
    sqlx::query("DELETE FROM sgw_gate_mail_item WHERE mail_id = $1")
        .bind(cod_mail)
        .execute(&pool)
        .await
        .unwrap();

    let r = Client::new(0x7300_1582, rcpt, 54_752, "SsmTwoCodR");
    r.op(
        MailOp::Delete { mail_id: cod_mail },
        Some(&pool),
        Instant::now(),
    )
    .await;
    assert_eq!(
        r.take(),
        vec![Received::Feedback(
            "This gate-mail is a COD delivery that has not been paid. Return it instead of \
             deleting it."
                .to_string()
        )]
    );
    assert_eq!(mail_count(&pool, rcpt).await, 4);

    cleanup(&pool, acct).await;
}

/// No orphaned escrow: after refused deletes, every escrow row still has
/// its mail, and the attached item is still recoverable, byte for byte.
/// The escrow row cascades with its mail, so a reverted guard would not
/// orphan the row but destroy it with the mail; either way this fails.
/// Also checks the guard is owner-scoped: another character's delete of
/// the same mail id reports `not_found_for_owner` and changes nothing.
#[tokio::test]
async fn no_orphaned_escrow_after_delete() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, sender, rcpt) = (BASE + 20, BASE + 21, BASE + 22);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(sender, "SsmTwoOrpS"), (rcpt, "SsmTwoOrpR")]).await;
    let type_id = any_type_id(&pool).await;
    let [item_mail, _, cod_mail, _] = attached_mails(&pool, sender, "SsmTwoOrpR", type_id).await;
    let before = escrow_for(&pool, rcpt).await;
    assert_eq!(before.len(), 2);

    let r = Client::new(0x7300_1583, rcpt, 54_753, "SsmTwoOrpR");
    let s = Client::new(0x7300_1584, sender, 54_754, "SsmTwoOrpS");
    let now = Instant::now();
    for mail_id in [item_mail, cod_mail] {
        r.op(MailOp::Delete { mail_id }, Some(&pool), now).await;
        // The sender does not own the mail.
        s.op(MailOp::Delete { mail_id }, Some(&pool), now).await;
    }
    r.take();
    s.take();
    assert!(capture
        .find_event(
            tracing::Level::WARN,
            "Delete affected 0 rows",
            "not_found_for_owner"
        )
        .is_some());

    assert_eq!(
        escrow_for(&pool, rcpt).await,
        before,
        "escrow rows unchanged"
    );
    let orphans: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_gate_mail_item i \
         WHERE i.source_character_id = $1 \
           AND NOT EXISTS (SELECT 1 FROM sgw_gate_mail m WHERE m.mail_id = i.mail_id)",
    )
    .bind(sender)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(orphans, 0);

    cleanup(&pool, acct).await;
}
