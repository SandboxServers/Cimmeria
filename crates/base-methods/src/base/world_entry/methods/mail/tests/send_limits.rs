//! Send refusals that happen before any SQL: the mail-send bucket (D-SS14),
//! the cell's decode refusals, aliases (D-SS07) and attachments (D-SS05,
//! SS-M2). No database: each test passes `pool = None`, so a send that got
//! past its gate would end on `no_db_pool` instead, and the pinned `reason`
//! tells the two apart.

use std::time::{Duration, Instant};

use tracing::Level;

use super::packets::{plain_send, Client, Received};
use super::*;
use crate::cell::mail::codes::{flags, MailResult};
use crate::cell::messages::MailSend;
use crate::test_support::LogCapture;
use cimmeria_wire::cell::cell_methods::mail::decode_send_mail_message;

fn client(port: u16) -> Client {
    Client::new(0x7300_1001, 0x7300_1002, port, "Sender")
}

/// The one `sendMailResult` and the feedback lines of one refused send.
fn refusal(received: &[Received]) -> (u8, Vec<String>, i32, Vec<String>) {
    let mut result = None;
    let mut lines = Vec::new();
    for r in received {
        match r {
            Received::SendMailResult {
                result: code,
                failed,
                failed_flags,
            } => {
                assert!(result.is_none(), "exactly one sendMailResult per send");
                result = Some((*code, failed.clone(), *failed_flags));
            }
            Received::Feedback(text) => lines.push(text.clone()),
            other => panic!("unexpected reply {other:?}"),
        }
    }
    let (code, failed, failed_flags) = result.expect("a refusal answers sendMailResult");
    (code, failed, failed_flags, lines)
}

fn refused_with(capture: &crate::test_support::LogCaptureGuard, reason: &str) -> bool {
    capture
        .find_event(Level::WARN, "sendMailMessage refused", reason)
        .is_some()
}

/// A send that passes the bucket and is then refused with
/// `ItemNotAvailable`, so it cannot be mistaken for a limited send.
fn attachment_send() -> MailSend {
    let mut send = plain_send(&["Bob"]);
    send.cash = 5;
    send
}

/// The reply to a limited send: `sendMailResult(NoRecipients)` with the
/// typed names, and the flood line only when `notify`.
fn limited_reply(with_line: bool) -> Vec<Received> {
    let mut want = vec![Received::SendMailResult {
        result: MailResult::NoRecipients.code(),
        failed: vec!["Bob".to_string()],
        failed_flags: 0,
    }];
    if with_line {
        want.push(Received::Feedback(
            "You are sending messages too quickly.".to_string(),
        ));
    }
    want
}

/// D-SS14: burst 3. Three sends inside the window get past the bucket (and
/// are then refused for their attachment, code 2). The fourth is dropped
/// before anything else runs, logs `rate_limit.exceeded
/// category=mail_send` at WARN, and still answers `sendMailResult` (code 1)
/// plus the flood line, because the client disables Send on every press
/// and only a result tells the player it did nothing. A fifth inside the
/// 5 s notify window gets the result without a second line. Fails when the
/// bucket is unwired (the fourth is then answered with code 2) and when the
/// limited path stops answering `sendMailResult`.
#[tokio::test]
async fn mail_send_bucket_rejects_fourth_in_burst() {
    let capture = LogCapture::install();
    let c = client(54_701);
    let t0 = Instant::now();
    for i in 0..3u64 {
        c.op(
            MailOp::Send(attachment_send()),
            None,
            t0 + Duration::from_millis(i * 100),
        )
        .await;
        let (code, _, _, _) = refusal(&c.take());
        assert_eq!(
            code,
            MailResult::ItemNotAvailable.code(),
            "send {i} gets past the bucket"
        );
    }

    c.op(
        MailOp::Send(attachment_send()),
        None,
        t0 + Duration::from_millis(300),
    )
    .await;
    assert_eq!(c.take(), limited_reply(true), "the fourth send is limited");
    let ev = capture
        .find_event(Level::WARN, "rate_limit.exceeded", "bucket_empty")
        .expect("rate_limit.exceeded at WARN");
    assert!(ev.has_field("category", "mail_send"), "{ev:?}");
    assert!(
        ev.fields.contains_key("player_id"),
        "player_id on the event: {ev:?}"
    );
    assert!(
        ev.fields.contains_key("account_id"),
        "account_id on the event: {ev:?}"
    );

    c.op(
        MailOp::Send(attachment_send()),
        None,
        t0 + Duration::from_millis(400),
    )
    .await;
    assert_eq!(
        c.take(),
        limited_reply(false),
        "every limited press is answered; the line is throttled"
    );

    // A token is back after 10 s.
    c.op(
        MailOp::Send(attachment_send()),
        None,
        t0 + Duration::from_secs(11),
    )
    .await;
    let (code, _, _, _) = refusal(&c.take());
    assert_eq!(code, MailResult::ItemNotAvailable.code());
}

/// A refusal costs a token too, so a client cannot turn a flood of bad
/// sends into a flood of replies.
#[tokio::test]
async fn decode_refusals_are_charged_to_the_bucket() {
    let c = client(54_702);
    let t0 = Instant::now();
    for _ in 0..3 {
        c.op(
            MailOp::SendRejected(crate::cell::messages::MailSendReject::Malformed {
                reason: "truncated",
            }),
            None,
            t0,
        )
        .await;
    }
    c.take();
    c.op(MailOp::Send(plain_send(&["Bob"])), None, t0).await;
    assert_eq!(c.take(), limited_reply(true));
}

/// CAT-G-01 / D-SS05: eleven names are refused whole, from the declared
/// count, with `MAILRESULT_NoRecipients` and a feedback line. End to end
/// through the cell's decoder, so raising the cap fails here: the send
/// would reach delivery and end on `no_db_pool`.
#[tokio::test]
async fn send_rejects_eleven_recipients() {
    let capture = LogCapture::install();
    let c = client(54_703);
    let mut p = 0i32.to_le_bytes().to_vec();
    p.extend_from_slice(&11u32.to_le_bytes());
    for i in 0..11 {
        crate::mercury::write_wstring(&mut p, &format!("Name{i}"));
    }
    crate::mercury::write_wstring(&mut p, "Subject");
    crate::mercury::write_wstring(&mut p, "Body");
    p.extend_from_slice(&[0; 13]);
    let op = match decode_send_mail_message(&p) {
        Ok(send) => MailOp::Send(send),
        Err(reject) => MailOp::SendRejected(reject),
    };
    c.op(op, None, Instant::now()).await;

    let (code, failed, failed_flags, lines) = refusal(&c.take());
    assert_eq!(code, MailResult::NoRecipients.code());
    assert!(failed.is_empty());
    assert_eq!(failed_flags, 0);
    assert_eq!(
        lines,
        vec!["A gate-mail message can have at most 10 recipients. It was not sent.".to_string()]
    );
    assert!(refused_with(&capture, "too_many_recipients"));
}

/// D-SS05: an attachment with two recipients is
/// `MAILRESULT_AttachmentsAndMultipleRecipients`, whichever attachment it
/// is, and the names are echoed back. "Bob" and "bob" count as one name.
#[tokio::test]
async fn send_rejects_attachment_with_two_recipients() {
    let capture = LogCapture::install();
    let c = client(54_704);
    let t0 = Instant::now();
    let attachments: [fn(&mut MailSend); 3] = [
        |s| s.cash = 50,
        |s| s.item_id = 10_042,
        |s| {
            s.cod = true;
            s.cash = 10;
        },
    ];
    for (i, attach) in attachments.iter().enumerate() {
        let mut send = plain_send(&["Bob", "Al"]);
        attach(&mut send);
        c.op(
            MailOp::Send(send),
            None,
            t0 + Duration::from_secs(10 * i as u64),
        )
        .await;
        let (code, failed, _, lines) = refusal(&c.take());
        assert_eq!(
            code,
            MailResult::AttachmentsAndMultipleRecipients.code(),
            "attachment {i}"
        );
        assert_eq!(failed, vec!["Bob".to_string(), "Al".to_string()]);
        assert_eq!(lines.len(), 1);
    }
    assert!(refused_with(
        &capture,
        "attachment_with_multiple_recipients"
    ));

    // The same name twice, in two cases, is one recipient: not code 3.
    let mut send = plain_send(&["Bob", "bob"]);
    send.cash = 50;
    c.op(MailOp::Send(send), None, t0 + Duration::from_secs(30))
        .await;
    let (code, _, _, _) = refusal(&c.take());
    assert_eq!(code, MailResult::ItemNotAvailable.code());
}

/// CAT-G-01: negative cash is refused (as every attachment is until SS-M2)
/// and never reaches delivery.
#[tokio::test]
async fn send_rejects_negative_cash() {
    let capture = LogCapture::install();
    let c = client(54_705);
    let mut send = plain_send(&["Bob"]);
    send.cash = -500;
    c.op(MailOp::Send(send), None, Instant::now()).await;
    let (code, failed, _, lines) = refusal(&c.take());
    assert_eq!(code, MailResult::ItemNotAvailable.code());
    assert_eq!(failed, vec!["Bob".to_string()]);
    assert_eq!(lines.len(), 1);
    assert!(refused_with(&capture, "attachment_not_supported"));
    assert!(!refused_with(&capture, "no_db_pool"));
    let ev = capture
        .find_event(
            Level::WARN,
            "sendMailMessage refused",
            "attachment_not_supported",
        )
        .unwrap();
    for key in ["account_id", "player_id", "entity_id", "result"] {
        assert!(ev.fields.contains_key(key), "{key} missing: {ev:?}");
    }
}

/// Every single-recipient attachment is refused with feedback until SS-M2.
#[tokio::test]
async fn send_refuses_every_attachment_until_ss_m2() {
    let c = client(54_706);
    let t0 = Instant::now();
    let attachments: [fn(&mut MailSend); 4] = [
        |s| s.cash = 1,
        |s| s.cod = true,
        |s| s.item_id = 10_042,
        |s| s.item_quantity = 1,
    ];
    for (i, attach) in attachments.iter().enumerate() {
        let mut send = plain_send(&["Bob"]);
        attach(&mut send);
        c.op(
            MailOp::Send(send),
            None,
            t0 + Duration::from_secs(10 * i as u64),
        )
        .await;
        let (code, _, _, lines) = refusal(&c.take());
        assert_eq!(code, MailResult::ItemNotAvailable.code(), "attachment {i}");
        assert_eq!(
            lines,
            vec![
                "Gate-mail attachments (naquadah, items and COD) are not available yet. \
                 Send the message without them."
                    .to_string()
            ]
        );
    }
}

/// D-SS07: any alias bit refuses the whole send with the alias bits echoed
/// in `FailedRecipientFlags`, vault and organization separately.
#[tokio::test]
async fn send_rejects_mail_aliases() {
    let capture = LogCapture::install();
    let c = client(54_707);
    let t0 = Instant::now();
    let cases = [
        (
            flags::MAIL_TO_VAULT,
            flags::MAIL_TO_VAULT,
            "vault_alias_unsupported",
        ),
        (
            flags::MAIL_TO_TEAM,
            flags::MAIL_TO_TEAM,
            "organization_alias_unsupported",
        ),
        (
            flags::MAIL_TO_COMMAND_RANK6,
            flags::MAIL_TO_COMMAND_RANK6 & flags::VAULT_ALIASES,
            "vault_alias_unsupported",
        ),
    ];
    for (i, (bits, echoed, reason)) in cases.into_iter().enumerate() {
        let mut send = plain_send(&["Bob"]);
        send.recipient_flags = bits;
        c.op(
            MailOp::Send(send),
            None,
            t0 + Duration::from_secs(10 * i as u64),
        )
        .await;
        let (code, failed, failed_flags, lines) = refusal(&c.take());
        assert_eq!(code, MailResult::NoRecipients.code(), "{reason}");
        assert_eq!(failed_flags, echoed, "{reason}");
        assert_eq!(failed, vec!["Bob".to_string()]);
        assert_eq!(lines.len(), 1);
        assert!(refused_with(&capture, reason), "{reason}");
    }
}

/// The feedback line names what was wrong with the text (D-SS12).
#[tokio::test]
async fn text_refusals_explain_themselves() {
    use cimmeria_entity::organization::{TextField, TextReject};
    let c = client(54_708);
    let t0 = Instant::now();
    let cases = [
        (
            TextField::MailSubject,
            TextReject::TooShort { units: 0, min: 1 },
            "Your gate-mail message needs a subject. It was not sent.",
        ),
        (
            TextField::MailBody,
            TextReject::TooLong {
                units: 1_001,
                max: 1_000,
            },
            "Your gate-mail message is too long (1,000 characters at most). It was not sent.",
        ),
    ];
    for (i, (field, reject, text)) in cases.into_iter().enumerate() {
        c.op(
            MailOp::SendRejected(crate::cell::messages::MailSendReject::Text { field, reject }),
            None,
            t0 + Duration::from_secs(10 * i as u64),
        )
        .await;
        let (code, _, _, lines) = refusal(&c.take());
        assert_eq!(code, MailResult::NoRecipients.code());
        assert_eq!(lines, vec![text.to_string()]);
    }
}
