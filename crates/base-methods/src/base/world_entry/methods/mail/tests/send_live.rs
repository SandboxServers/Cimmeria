//! Live-DB guards for a text-only gate-mail send: delivery, D-SS13 name
//! resolution, D-SS05 de-duplication and cap, the D-SS03 mailbox cap and
//! the schema backstop. Sentinels `0x7300_11xx`.

use std::time::{Duration, Instant};

use super::packets::{plain_send, Client, Received};
use super::*;
use crate::cell::mail::codes::{flags, MailResult};

const BASE: i32 = 0x7300_1100;

/// The `sendMailResult` among `received`, and the feedback lines.
fn result_of(received: &[Received]) -> (u8, Vec<String>, Vec<String>) {
    let mut result = None;
    let mut lines = Vec::new();
    for r in received {
        match r {
            Received::SendMailResult {
                result: code,
                failed,
                ..
            } => {
                assert!(result.is_none(), "exactly one sendMailResult per send");
                result = Some((*code, failed.clone()));
            }
            Received::Feedback(text) => lines.push(text.clone()),
            other => panic!("unexpected reply {other:?}"),
        }
    }
    let (code, failed) = result.expect("every send answers sendMailResult");
    (code, failed, lines)
}

#[derive(Debug, sqlx::FromRow)]
struct MailRow {
    sender_id: Option<i32>,
    sender_name: String,
    subject: String,
    message: String,
    cash: i64,
    flags: i32,
    item_id: Option<i32>,
    read_time: i32,
}

async fn rows_for(pool: &PgPool, character_id: i32) -> Vec<MailRow> {
    sqlx::query_as(
        "SELECT sender_id, sender_name, subject, message, cash, flags, item_id, read_time \
         FROM sgw_gate_mail WHERE character_id = $1 ORDER BY mail_id",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// A plain send to a character who is not online lands one row with the
/// sender's id and the name stored on the sender's own row (never the
/// session's), and answers `MAILRESULT_Sent` with no failures.
#[tokio::test]
async fn live_db_send_delivers_to_offline_recipient() {
    let pool = require_db_or_skip!();
    let capture = crate::test_support::LogCapture::install();
    let (acct, sender, rcpt) = (BASE, BASE + 1, BASE + 2);
    cleanup(&pool, acct).await;
    insert_players(
        &pool,
        acct,
        &[(sender, "SsmOneSender"), (rcpt, "SsmOneRcpt")],
    )
    .await;

    let c = Client::new(0x7300_1181, sender, 54_711, "SessionNameIsNotStored");
    let mut send = plain_send(&["SsmOneRcpt"]);
    send.subject = "Hello".into();
    send.body = "line 1\nline 2".into();
    c.op(MailOp::Send(send), Some(&pool), Instant::now()).await;

    let (code, failed, lines) = result_of(&c.take());
    assert_eq!(code, MailResult::Sent.code());
    assert!(failed.is_empty());
    assert!(
        lines.is_empty(),
        "a clean send needs no feedback line: {lines:?}"
    );

    let rows = rows_for(&pool, rcpt).await;
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(r.sender_id, Some(sender));
    assert_eq!(r.sender_name, "SsmOneSender");
    assert_eq!(
        (r.subject.as_str(), r.message.as_str()),
        ("Hello", "line 1\nline 2")
    );
    assert_eq!((r.cash, r.flags, r.item_id, r.read_time), (0, 0, None, 0));
    assert_eq!(
        mail_count(&pool, sender).await,
        0,
        "the sender keeps no copy"
    );
    // Rule 6: the sent row names the sender (the session's character and
    // login) next to the ids.
    let sent = capture
        .all()
        .into_iter()
        .find(|e| e.has_field("event", "mail.sent"))
        .expect("mail.sent");
    assert_actor_names(&sent, "SessionNameIsNotStored");

    cleanup(&pool, acct).await;
}

/// CAT-G-01 / D-SS05: one unknown name and one good one delivers to the
/// good one and lists the unknown one in `FailedRecipients` under
/// `MAILRESULT_Sent`, with a feedback line saying why.
#[tokio::test]
async fn live_db_send_partial_failure_delivers_to_the_rest() {
    let pool = require_db_or_skip!();
    let (acct, sender, rcpt) = (BASE + 10, BASE + 11, BASE + 12);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(sender, "SsmOnePfS"), (rcpt, "SsmOnePfR")]).await;

    let c = Client::new(0x7300_1182, sender, 54_712, "SsmOnePfS");
    c.op(
        MailOp::Send(plain_send(&["NoSuchSsmOneName", "SsmOnePfR"])),
        Some(&pool),
        Instant::now(),
    )
    .await;

    let (code, failed, lines) = result_of(&c.take());
    assert_eq!(code, MailResult::Sent.code());
    assert_eq!(failed, vec!["NoSuchSsmOneName".to_string()]);
    assert_eq!(
        lines,
        vec!["Gate-mail not delivered to: NoSuchSsmOneName (no such character).".to_string()]
    );
    assert_eq!(mail_count(&pool, rcpt).await, 1);

    cleanup(&pool, acct).await;
}

/// CAT-G-01: a send whose only recipient does not exist is
/// `MAILRESULT_NoRecipients`, lists the name, and inserts nothing.
#[tokio::test]
async fn live_db_send_rejects_unknown_recipient() {
    let pool = require_db_or_skip!();
    let (acct, sender) = (BASE + 20, BASE + 21);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(sender, "SsmOneUnkS")]).await;
    let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sgw_gate_mail")
        .fetch_one(&pool)
        .await
        .unwrap();

    let c = Client::new(0x7300_1183, sender, 54_713, "SsmOneUnkS");
    c.op(
        MailOp::Send(plain_send(&["NoSuchSsmOneName"])),
        Some(&pool),
        Instant::now(),
    )
    .await;

    let (code, failed, lines) = result_of(&c.take());
    assert_eq!(code, MailResult::NoRecipients.code());
    assert_eq!(failed, vec!["NoSuchSsmOneName".to_string()]);
    assert_eq!(lines.len(), 1);
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sgw_gate_mail")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(after, before, "nothing inserted");

    cleanup(&pool, acct).await;
}

/// D-SS13: a name resolves case-insensitively when exactly one character
/// matches, is refused when two do, and an exact match beats a fold.
/// D-SS05: the same recipient typed twice gets one copy.
#[tokio::test]
async fn live_db_send_resolves_names_per_d_ss13_and_dedupes() {
    let pool = require_db_or_skip!();
    let (acct, sender, solo, upper, lower) =
        (BASE + 30, BASE + 31, BASE + 32, BASE + 33, BASE + 34);
    cleanup(&pool, acct).await;
    insert_players(
        &pool,
        acct,
        &[
            (sender, "SsmOneResS"),
            (solo, "SsmOneSolo"),
            (upper, "SsmOneTwin"),
            (lower, "ssmonetwin"),
        ],
    )
    .await;
    let c = Client::new(0x7300_1184, sender, 54_714, "SsmOneResS");
    let t0 = Instant::now();

    // Case-folded, unique; typed three times: one copy.
    c.op(
        MailOp::Send(plain_send(&["ssmonesolo", "SSMONESOLO", "SsmOneSolo"])),
        Some(&pool),
        t0,
    )
    .await;
    let (code, failed, _) = result_of(&c.take());
    assert_eq!(code, MailResult::Sent.code());
    assert!(failed.is_empty(), "{failed:?}");
    assert_eq!(mail_count(&pool, solo).await, 1, "de-duplicated by player");

    // Two characters fold to the same name: an exact match still resolves...
    c.op(
        MailOp::Send(plain_send(&["ssmonetwin"])),
        Some(&pool),
        t0 + Duration::from_secs(10),
    )
    .await;
    assert_eq!(result_of(&c.take()).0, MailResult::Sent.code());
    assert_eq!(mail_count(&pool, lower).await, 1);
    assert_eq!(mail_count(&pool, upper).await, 0);

    // ...but a fold that matches both is refused, not guessed.
    c.op(
        MailOp::Send(plain_send(&["SSMONETWIN"])),
        Some(&pool),
        t0 + Duration::from_secs(20),
    )
    .await;
    let (code, failed, lines) = result_of(&c.take());
    assert_eq!(code, MailResult::NoRecipients.code());
    assert_eq!(failed, vec!["SSMONETWIN".to_string()]);
    assert!(
        lines[0].contains("more than one character matches"),
        "{lines:?}"
    );
    assert_eq!(mail_count(&pool, upper).await, 0);
    assert_eq!(mail_count(&pool, lower).await, 1);

    cleanup(&pool, acct).await;
}

/// D-SS05: ten distinct recipients all get the mail in one send.
#[tokio::test]
async fn live_db_send_delivers_to_ten_recipients() {
    let pool = require_db_or_skip!();
    let acct = BASE + 40;
    let sender = BASE + 41;
    let rcpts: Vec<(i32, String)> = (0..10)
        .map(|i| (BASE + 50 + i, format!("SsmOneTen{i}")))
        .collect();
    cleanup(&pool, acct).await;
    let mut players: Vec<(i32, &str)> = vec![(sender, "SsmOneTenS")];
    players.extend(rcpts.iter().map(|(id, n)| (*id, n.as_str())));
    insert_players(&pool, acct, &players).await;

    let c = Client::new(0x7300_1185, sender, 54_715, "SsmOneTenS");
    let names: Vec<&str> = rcpts.iter().map(|(_, n)| n.as_str()).collect();
    c.op(
        MailOp::Send(plain_send(&names)),
        Some(&pool),
        Instant::now(),
    )
    .await;
    let (code, failed, _) = result_of(&c.take());
    assert_eq!(code, MailResult::Sent.code());
    assert!(failed.is_empty());
    for (id, _) in &rcpts {
        assert_eq!(mail_count(&pool, *id).await, 1, "recipient {id}");
    }

    cleanup(&pool, acct).await;
}

/// D-SS03: a recipient holding 100 open (not archived) messages is listed
/// in `FailedRecipients` and gets nothing; archived mail does not count, so
/// 99 open plus any number archived still receives.
#[tokio::test]
async fn live_db_send_refuses_full_mailbox() {
    let pool = require_db_or_skip!();
    let (acct, sender, full, roomy) = (BASE + 60, BASE + 61, BASE + 62, BASE + 63);
    cleanup(&pool, acct).await;
    insert_players(
        &pool,
        acct,
        &[
            (sender, "SsmOneCapS"),
            (full, "SsmOneFull"),
            (roomy, "SsmOneRoomy"),
        ],
    )
    .await;
    fill_mailbox(&pool, full, 100, 0).await;
    fill_mailbox(&pool, roomy, 99, 0).await;
    fill_mailbox(&pool, roomy, 5, flags::MAIL_ARCHIVE).await;

    let c = Client::new(0x7300_1186, sender, 54_716, "SsmOneCapS");
    c.op(
        MailOp::Send(plain_send(&["SsmOneFull", "SsmOneRoomy"])),
        Some(&pool),
        Instant::now(),
    )
    .await;
    let (code, failed, lines) = result_of(&c.take());
    assert_eq!(code, MailResult::Sent.code());
    assert_eq!(failed, vec!["SsmOneFull".to_string()]);
    assert_eq!(
        lines,
        vec!["Gate-mail not delivered to: SsmOneFull (gate-mail box is full).".to_string()]
    );
    assert_eq!(
        mail_count(&pool, full).await,
        100,
        "the full box got nothing"
    );
    assert_eq!(
        mail_count(&pool, roomy).await,
        105,
        "the 100th open slot filled"
    );

    // Now both are full: the whole send is refused.
    c.op(
        MailOp::Send(plain_send(&["SsmOneFull", "SsmOneRoomy"])),
        Some(&pool),
        Instant::now() + Duration::from_secs(10),
    )
    .await;
    let (code, failed, _) = result_of(&c.take());
    assert_eq!(code, MailResult::NoRecipients.code());
    assert_eq!(failed.len(), 2);

    cleanup(&pool, acct).await;
}

/// Schema backstop: `sgw_gate_mail.cash` may not go negative.
#[tokio::test]
async fn live_db_schema_rejects_negative_mail_cash() {
    let pool = require_db_or_skip!();
    let (acct, owner) = (BASE + 70, BASE + 71);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(owner, "SsmOneChk")]).await;
    let err = sqlx::query(
        "INSERT INTO sgw_gate_mail \
            (character_id, subject, message, cash, sent_time, read_time, flags) \
         VALUES ($1, 's', 'b', -1, 0, 0, 0)",
    )
    .bind(owner)
    .execute(&pool)
    .await
    .expect_err("negative cash must violate the CHECK");
    assert!(
        err.to_string()
            .contains("sgw_gate_mail_cash_nonnegative_chk"),
        "{err}"
    );
    cleanup(&pool, acct).await;
}

/// The D-SS13 case-fold arm of the recipient query can use
/// `sgw_player_player_name_lower_idx`: with sequential scans disabled, the
/// plan for the exact statement the send path runs names the index. Fails
/// when the index is dropped from `db/sgw/_indexes.sql` or the query's
/// expression stops matching it.
#[tokio::test]
async fn live_db_recipient_case_fold_uses_the_lower_name_index() {
    let pool = require_db_or_skip!();
    let mut conn = pool.acquire().await.unwrap();
    sqlx::query("SET enable_seqscan = off")
        .execute(&mut *conn)
        .await
        .unwrap();
    let plan: Vec<String> = // A test-only EXPLAIN of a constant statement: nothing player-supplied.
    sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "EXPLAIN {}",
        super::super::send::recipients::CANDIDATE_ROWS_SQL
    )))
    .bind(vec!["SsmOneIdx".to_string()])
    .bind(vec!["ssmoneidx".to_string()])
    .fetch_all(&mut *conn)
    .await
    .unwrap();
    sqlx::query("RESET enable_seqscan")
        .execute(&mut *conn)
        .await
        .unwrap();
    let plan = plan.join("\n");
    assert!(
        plan.contains("sgw_player_player_name_lower_idx"),
        "the case-fold arm must use the lower(player_name) index:\n{plan}"
    );
}
