//! SS-C1 × SS-M1: mail to a recipient whose Ignore list holds the sender is
//! refused for that recipient (D-SS15), read from the database so it works
//! for an offline recipient. Sentinels `0x7300_C4xx`.

use std::time::Instant;

use super::packets::{plain_send, Client, Received};
use super::*;
use crate::cell::mail::codes::MailResult;

const BASE: i32 = 0x7300_C400;

/// The recipient "SsC1Ignorer" has the sender on their Ignore list, stored
/// in a different case than the sender's name. Their copy is not delivered,
/// `sendMailResult` lists them in `FailedRecipients` under
/// `MAILRESULT_Sent`, the sender reads "SsC1Ignorer is not accepting your
/// messages.", and the second recipient still gets the mail. Fails when
/// `send::recipients::ignoring_sender` is the empty-set stub again.
#[tokio::test]
async fn live_db_send_skips_recipient_who_ignores_the_sender() {
    let pool = require_db_or_skip!();
    let (acct, sender, ignorer, friend) = (BASE, BASE + 1, BASE + 2, BASE + 3);
    cleanup(&pool, acct).await;
    insert_players(
        &pool,
        acct,
        &[
            (sender, "SsC1MailSender"),
            (ignorer, "SsC1Ignorer"),
            (friend, "SsC1Friend"),
        ],
    )
    .await;
    let list = crate::base::contact_list::ignore::ensure_ignore_list(&pool, ignorer)
        .await
        .unwrap();
    sqlx::query("INSERT INTO sgw_contact_list_member (list_id, player_name) VALUES ($1, $2)")
        .bind(list)
        .bind("ssc1mailSENDER")
        .execute(&pool)
        .await
        .unwrap();

    let c = Client::new(0x7300_C481, sender, 54_791, "SsC1MailSender");
    c.op(
        MailOp::Send(plain_send(&["SsC1Ignorer", "SsC1Friend"])),
        Some(&pool),
        Instant::now(),
    )
    .await;

    let mut result = None;
    let mut lines = Vec::new();
    for r in c.take() {
        match r {
            Received::SendMailResult {
                result: code,
                failed,
                ..
            } => result = Some((code, failed)),
            Received::Feedback(text) => lines.push(text),
            other => panic!("unexpected reply {other:?}"),
        }
    }
    let (code, failed) = result.expect("sendMailResult");
    assert_eq!(code, MailResult::Sent.code(), "the other recipient got it");
    assert_eq!(failed, vec!["SsC1Ignorer".to_string()]);
    assert_eq!(
        lines,
        vec!["SsC1Ignorer is not accepting your messages.".to_string()]
    );
    assert_eq!(
        mail_count(&pool, ignorer).await,
        0,
        "nothing for the ignorer"
    );
    assert_eq!(
        mail_count(&pool, friend).await,
        1,
        "the friend still gets it"
    );

    cleanup(&pool, acct).await;
}
