//! Live-DB rollback guards for attached sends (D-SS06): any failure inside
//! the send transaction, before or after the item has moved, leaves cash,
//! both inventories, both mailboxes and escrow as they were. Sentinels
//! `0x7300_147x`/`0x7300_148x` and items `0x7300_167x`, plus one fixed
//! escrow id `0x7300_17F0`.

use std::time::Instant;

use super::attach_live::{assert_untouched, attached, reply, setup};
use super::packets::{Client, Received};
use super::*;
use crate::cell::mail::codes::MailResult;
use crate::test_support::LogCapture;

const BASE: i32 = 0x7300_1470;
const ITEMS: i32 = 0x7300_1670;

/// D-SS06: a failure after the debit and the item lock (here the mail
/// insert: a subject past the column's 128 characters, which the cell's
/// text rules would have refused) rolls everything back. The balance, the
/// stack, both mailboxes and escrow are as they were, and the player is
/// told the mail was not sent.
#[tokio::test]
async fn send_rolls_back_on_insert_failure() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, sender, rcpt) = (BASE, BASE + 1, BASE + 2);
    let type_id = setup(
        &pool,
        acct,
        (sender, "SsmTwoRbSend"),
        (rcpt, "SsmTwoRbRcpt"),
        500,
    )
    .await;
    let whole = TestItem::main(ITEMS, sender, 0, 4);
    insert_item(&pool, whole, type_id).await;

    let c = Client::new(0x7300_1485, sender, 54_745, "SsmTwoRbSend");
    let mut send = attached("SsmTwoRbRcpt", 100, false, whole.item_id, 4);
    send.subject = "x".repeat(200);
    c.op(MailOp::Send(send), Some(&pool), Instant::now()).await;

    let r = reply(c.take());
    assert_eq!(r.code, Some(MailResult::NoRecipients.code()), "{r:?}");
    assert_eq!(
        r.lines,
        vec!["Gate-mail is unavailable right now. The message was not sent.".to_string()]
    );
    assert!(r.cash.is_empty() && r.removed.is_empty() && r.inventory_updates == 0);
    assert!(capture
        .find_event(tracing::Level::WARN, "sendMailMessage refused", "db_error")
        .is_some());
    assert_untouched(&pool, sender, rcpt, 500, &[whole]).await;

    cleanup(&pool, acct).await;
}

/// D-SS06, after the move: on the split path the sender's stack is
/// decremented and then the escrow insert fails (forced: the inventory
/// sequence is pointed at an id an escrow row already holds). The
/// decrement, the debit and the mail insert all roll back with it.
#[tokio::test]
async fn send_rolls_back_after_item_moved() {
    const TAKEN_ID: i32 = 0x7300_17F0;
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, sender, rcpt) = (BASE + 10, BASE + 11, BASE + 12);
    let type_id = setup(
        &pool,
        acct,
        (sender, "SsmTwoRb2Send"),
        (rcpt, "SsmTwoRb2Rcpt"),
        500,
    )
    .await;
    let stack = TestItem::main(ITEMS + 10, sender, 0, 5);
    insert_item(&pool, stack, type_id).await;

    // An escrow row already holding TAKEN_ID, on a mail of its own.
    let blocker_mail = insert_mail(&pool, rcpt, "blocker").await;
    sqlx::query(
        "INSERT INTO sgw_gate_mail_item \
            (mail_id, item_id, type_id, stack_size, charges, durability, flags, bound, \
             ammo, cur_ammo_type, ammo_type, ammo_types, source_character_id, escrowed_at) \
         VALUES ($1, $2, $3, 1, 0, -1, 0, false, 0, 0, 'AMMO_NONE', '{}', $4, 0)",
    )
    .bind(blocker_mail)
    .bind(TAKEN_ID)
    .bind(type_id)
    .bind(rcpt)
    .execute(&pool)
    .await
    .expect("blocker escrow row");
    let seq_before: i64 = sqlx::query_scalar("SELECT last_value FROM sgw_inventory_item_id_seq")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("SELECT setval('sgw_inventory_item_id_seq', $1)")
        .bind(i64::from(TAKEN_ID) - 1)
        .execute(&pool)
        .await
        .unwrap();

    let c = Client::new(0x7300_148A, sender, 54_756, "SsmTwoRb2Send");
    c.op(
        MailOp::Send(attached("SsmTwoRb2Rcpt", 100, false, stack.item_id, 2)),
        Some(&pool),
        Instant::now(),
    )
    .await;
    sqlx::query("SELECT setval('sgw_inventory_item_id_seq', $1)")
        .bind(seq_before)
        .execute(&pool)
        .await
        .unwrap();

    let r = reply(c.take());
    assert_eq!(r.code, Some(MailResult::NoRecipients.code()), "{r:?}");
    assert!(r.cash.is_empty() && r.removed.is_empty() && r.inventory_updates == 0);
    assert!(capture
        .find_event(tracing::Level::WARN, "sendMailMessage refused", "db_error")
        .is_some());
    assert_eq!(naquadah(&pool, sender).await, 500, "nothing debited");
    assert_eq!(
        inventory_row(&pool, stack.item_id).await,
        Some((sender, 5)),
        "the decrement rolled back"
    );
    assert_eq!(mail_count(&pool, rcpt).await, 1, "only the blocker mail");
    assert_eq!(
        escrow_from(&pool, sender).await,
        0,
        "no escrow from the sender"
    );

    cleanup(&pool, acct).await;
}

/// D-SS03 with an attachment: a recipient at the 100-message cap gets
/// nothing, the name is in `FailedRecipients` with the reason, and neither
/// the cash nor the item leaves the sender.
#[tokio::test]
async fn attached_send_to_full_mailbox_moves_nothing() {
    let pool = require_db_or_skip!();
    let (acct, sender, rcpt) = (BASE + 20, BASE + 21, BASE + 22);
    let type_id = setup(
        &pool,
        acct,
        (sender, "SsmTwoFullSend"),
        (rcpt, "SsmTwoFullRcpt"),
        500,
    )
    .await;
    let whole = TestItem::main(ITEMS + 20, sender, 0, 1);
    insert_item(&pool, whole, type_id).await;
    fill_mailbox(&pool, rcpt, 100, 0).await;

    let c = Client::new(0x7300_148B, sender, 54_757, "SsmTwoFullSend");
    c.op(
        MailOp::Send(attached("SsmTwoFullRcpt", 50, false, whole.item_id, 1)),
        Some(&pool),
        Instant::now(),
    )
    .await;
    let received = c.take();
    let failed = received.iter().find_map(|p| match p {
        Received::SendMailResult { failed, .. } => Some(failed.clone()),
        _ => None,
    });
    assert_eq!(failed, Some(vec!["SsmTwoFullRcpt".to_string()]));
    let r = reply(received);
    assert_eq!(r.code, Some(MailResult::NoRecipients.code()), "{r:?}");
    assert_eq!(
        r.lines,
        vec!["Gate-mail not delivered to: SsmTwoFullRcpt (gate-mail box is full).".to_string()]
    );
    assert!(r.cash.is_empty() && r.removed.is_empty());
    assert_eq!(naquadah(&pool, sender).await, 500);
    assert_eq!(inventory_row(&pool, whole.item_id).await, Some((sender, 1)));
    assert_eq!(mail_count(&pool, rcpt).await, 100);
    assert_eq!(escrow_from(&pool, sender).await, 0);

    cleanup(&pool, acct).await;
}

/// D-SS15 with an attachment: the only recipient ignores the sender. The
/// Ignore check runs under the player-row lock, before the debit and the
/// escrow move, so the send is `NoRecipients` with the shared "not
/// accepting" line, and neither the cash nor the item leaves the sender.
/// Fails when the Ignore check is skipped (the mail, the debit and the
/// escrow row all happen).
#[tokio::test]
async fn attached_send_to_ignoring_recipient_moves_nothing() {
    let pool = require_db_or_skip!();
    let (acct, sender, rcpt) = (BASE + 30, BASE + 31, BASE + 32);
    let type_id = setup(
        &pool,
        acct,
        (sender, "SsmTwoIgnSend"),
        (rcpt, "SsmTwoIgnRcpt"),
        500,
    )
    .await;
    let whole = TestItem::main(ITEMS + 30, sender, 0, 1);
    insert_item(&pool, whole, type_id).await;
    let list = crate::base::contact_list::ignore::ensure_ignore_list(&pool, rcpt)
        .await
        .unwrap();
    sqlx::query("INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES ($1, $2)")
        .bind(list)
        .bind("SsmTwoIgnSend")
        .execute(&pool)
        .await
        .unwrap();

    let c = Client::new(0x7300_148C, sender, 54_758, "SsmTwoIgnSend");
    c.op(
        MailOp::Send(attached("SsmTwoIgnRcpt", 50, true, whole.item_id, 1)),
        Some(&pool),
        Instant::now(),
    )
    .await;
    let r = reply(c.take());
    assert_eq!(r.code, Some(MailResult::NoRecipients.code()), "{r:?}");
    assert_eq!(
        r.lines,
        vec!["SsmTwoIgnRcpt is not accepting your messages.".to_string()]
    );
    assert!(r.cash.is_empty() && r.removed.is_empty() && r.inventory_updates == 0);
    assert_untouched(&pool, sender, rcpt, 500, &[whole]).await;

    cleanup(&pool, acct).await;
}
