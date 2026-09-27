//! Read-side fixes (SS-M1): the `bArchive` filter (audit A-08), the
//! owner-scoped read-time update (CAT-G-07, A-09) and the stored-recipient
//! `ToText` (CAT-G-08, A-10). Sentinels `0x7300_13xx`.

use std::time::Instant;

use super::packets::{Client, Received};
use super::*;
use crate::cell::mail::codes::flags;

const BASE: i32 = 0x7300_1300;

/// Audit A-08 / SS-E1 M-Q7: `bArchive` 0 returns only inbox mail and 1 only
/// archived mail. Fails when the filter is reverted: both lists then carry
/// both rows.
#[tokio::test]
async fn request_headers_archive_filter_returns_requested_category() {
    let pool = require_db_or_skip!();
    let (acct, owner) = (BASE, BASE + 1);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(owner, "SsmOneHdr")]).await;
    let inbox = insert_mail(&pool, owner, "inbox").await;
    let archived = insert_mail(&pool, owner, "archived").await;
    sqlx::query("UPDATE sgw_gate_mail SET flags = flags | $2 WHERE mail_id = $1")
        .bind(archived)
        .bind(flags::MAIL_ARCHIVE | flags::MAIL_COD)
        .execute(&pool)
        .await
        .unwrap();

    let c = Client::new(0x7300_1381, owner, 54_731, "SsmOneHdr");
    for (b_archive, want) in [(0u8, inbox), (1u8, archived)] {
        c.op(
            MailOp::RequestHeaders { b_archive },
            Some(&pool),
            Instant::now(),
        )
        .await;
        match c.take().as_slice() {
            [Received::HeaderInfo {
                b_archive: echoed,
                headers,
            }] => {
                assert_eq!(*echoed, b_archive);
                let ids: Vec<i32> = headers.iter().map(|(id, _)| *id).collect();
                assert_eq!(ids, vec![want], "bArchive {b_archive}");
            }
            other => panic!("expected one onMailHeaderInfo, got {other:?}"),
        }
    }

    cleanup(&pool, acct).await;
}

/// CAT-G-07: the read-time `UPDATE` is owner-scoped on its own, not only
/// through the SELECT before it. Another character's `mark_read` on this
/// mail changes nothing; the owner's changes it once. Fails when
/// `AND character_id = $3` is removed from the update.
#[tokio::test]
async fn read_time_update_scoped_to_owner() {
    let pool = require_db_or_skip!();
    let (acct, owner, other) = (BASE + 10, BASE + 11, BASE + 12);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(owner, "SsmOneRtO"), (other, "SsmOneRtX")]).await;
    let mail_id = insert_mail(&pool, owner, "unread").await;

    let changed = super::super::read::mark_read(&pool, mail_id, other, 1_000)
        .await
        .unwrap();
    assert_eq!(changed, 0, "another character cannot mark this mail read");
    let read_time: i32 =
        sqlx::query_scalar("SELECT read_time FROM sgw_gate_mail WHERE mail_id = $1")
            .bind(mail_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(read_time, 0);

    let changed = super::super::read::mark_read(&pool, mail_id, owner, 1_000)
        .await
        .unwrap();
    assert_eq!(changed, 1, "the owner can");

    cleanup(&pool, acct).await;
}

/// CAT-G-08: `onMailRead.ToText` is the name stored on the recipient's
/// row, not the reader's session name. The session here holds a different
/// name; with the old session lookup `ToText` was that name.
#[tokio::test]
async fn mail_read_to_text_is_stored_recipient() {
    let pool = require_db_or_skip!();
    let (acct, owner) = (BASE + 20, BASE + 21);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(owner, "SsmOneStored")]).await;
    let mail_id = insert_mail(&pool, owner, "subject").await;

    let c = Client::new(0x7300_1382, owner, 54_732, "SessionImpostor");
    c.op(MailOp::RequestBody { mail_id }, Some(&pool), Instant::now())
        .await;
    assert_eq!(
        c.take(),
        vec![Received::MailRead {
            mail_id,
            to_text: "SsmOneStored".to_string()
        }]
    );

    cleanup(&pool, acct).await;
}

/// Instrumentation rule 5 on the read side: a read-side miss carries
/// `account_id`, `player_id` and `entity_id`, and so do the send path's
/// refusals (`send_rejects_negative_cash` checks those). Here character A
/// asks for character B's body: WARN `reason=not_found_for_owner`.
#[tokio::test]
async fn read_side_events_carry_account_player_and_entity() {
    let capture = crate::test_support::LogCapture::install();
    let pool = require_db_or_skip!();
    let (acct, a, b) = (BASE + 30, BASE + 31, BASE + 32);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(a, "SsmOneEvA"), (b, "SsmOneEvB")]).await;
    let mail_b = insert_mail(&pool, b, "for B").await;

    let c = Client::new(0x7300_1383, a, 54_733, "SsmOneEvA");
    c.op(
        MailOp::RequestBody { mail_id: mail_b },
        Some(&pool),
        Instant::now(),
    )
    .await;
    let ev = capture
        .find_event(
            tracing::Level::WARN,
            "Mail body not found",
            "not_found_for_owner",
        )
        .expect("the miss is logged");
    for key in ["account_id", "player_id", "entity_id", "mail_id"] {
        assert!(ev.fields.contains_key(key), "{key} missing: {ev:?}");
    }
    assert_eq!(ev.target, "mail");

    cleanup(&pool, acct).await;
}
