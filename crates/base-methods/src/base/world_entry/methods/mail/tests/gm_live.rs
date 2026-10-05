//! Live-DB tests for the base half of the GM mail tools (SS-U1,
//! `mail/gm.rs`). The parser and the GM gate are tested on the cell
//! (`cell-console` `tests/ss_u1_mail.rs`). Sentinels `0x7300_516x` to
//! `0x7300_519x`.

use super::packets::{Client, Received};
use super::*;
use crate::cell::mail::codes::flags::{MAIL_ARCHIVE, MAIL_COD};
use crate::cell::messages::{MailGmActor, MailGmCellToBase};
use crate::test_support::LogCapture;

fn actor(c: &Client) -> MailGmActor {
    MailGmActor {
        entity_id: c.entity_id,
        player_id: c.player_id,
        account_id: Some(0x7300_0001),
    }
}

pub(super) fn send(
    c: &Client,
    to: Option<&str>,
    cash: i64,
    item: Option<(i32, i32)>,
    cod: Option<i32>,
) -> MailGmCellToBase {
    MailGmCellToBase::Send {
        actor: actor(c),
        to: to.map(str::to_string),
        cash,
        item,
        cod,
        subject: "GM test mail".into(),
    }
}

pub(super) fn lines(received: Vec<Received>) -> Vec<String> {
    received
        .into_iter()
        .filter_map(|r| match r {
            Received::Feedback(t) => Some(t),
            _ => None,
        })
        .collect()
}

/// `(mail_id, sender_id, sender_name, cash, flags)` of every mail
/// `character_id` holds.
async fn mails(pool: &PgPool, character_id: i32) -> Vec<(i32, Option<i32>, String, i64, i32)> {
    sqlx::query_as(
        "SELECT mail_id, sender_id, sender_name, cash, flags FROM sgw_gate_mail \
         WHERE character_id = $1 ORDER BY mail_id",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

/// The packet's acceptance test: a bare `.mail cash 50 item <t>` creates
/// exactly one mail (to the GM, from the GM's name, no `sender_id`, no COD)
/// and exactly one escrow row; nothing is debited; the GM is told the mail
/// id, and `mail.gm_action` names the GM and the subject.
#[tokio::test]
async fn live_db_gm_mail_creates_one_mail_and_one_escrow_row() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, gm) = (0x7300_5160, 0x7300_5161);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(gm, "SsuOneGmSelf")]).await;
    set_naquadah(&pool, gm, 10).await;
    let type_id = any_type_id(&pool).await;
    let c = Client::new(0x7300_5190, gm, 54_760, "SsuOneGmSelf");

    c.gm(send(&c, None, 50, Some((type_id, 1)), None), &pool)
        .await;

    let rows = mails(&pool, gm).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    let (mail_id, sender_id, sender_name, cash, flags) = rows[0].clone();
    assert_eq!(
        (sender_id, sender_name.as_str(), cash, flags),
        (None, "SsuOneGmSelf", 50, 0)
    );
    let escrow = escrow_for(&pool, gm).await;
    assert_eq!(escrow.len(), 1, "{escrow:?}");
    assert_eq!(
        (escrow[0].mail_id, escrow[0].type_id, escrow[0].stack_size),
        (mail_id, type_id, 1)
    );
    assert_eq!(naquadah(&pool, gm).await, 10, "no postage, nothing debited");
    assert_eq!(
        lines(c.take()),
        vec![format!(
            "Mail {mail_id} sent to SsuOneGmSelf with 50 naquadah, 1 x item {type_id}."
        )]
    );
    let row = capture
        .all()
        .into_iter()
        .find(|e| e.has_field("event", "mail.gm_action"))
        .expect("mail.gm_action");
    for (k, v) in [
        ("action", "mail".to_string()),
        ("player_id", gm.to_string()),
        ("subject_player_id", gm.to_string()),
        ("mail_id", mail_id.to_string()),
        ("account_id", 0x7300_0001.to_string()),
    ] {
        assert!(row.has_field(k, &v), "{k}={v}: {row:#?}");
    }

    cleanup(&pool, acct).await;
}

/// `.mail to <name> item <t> cod <n>` is a COD mail **from the GM's
/// character** (so the payment comes back to the GM): `sender_id` set,
/// `MAIL_COD`, the price in `cash`, the item minted into escrow, and no
/// postage taken from the GM. The name resolves by the D-SS13 fold.
#[tokio::test]
async fn live_db_gm_mail_cod_is_sent_from_the_gm() {
    let pool = require_db_or_skip!();
    let (acct, gm, rcpt) = (0x7300_5170, 0x7300_5171, 0x7300_5172);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(gm, "SsuOneGmCod"), (rcpt, "SsuOneCodRcpt")]).await;
    let type_id = any_type_id(&pool).await;
    let c = Client::new(0x7300_5191, gm, 54_761, "SsuOneGmCod");

    c.gm(
        send(&c, Some("ssuonecodrcpt"), 0, Some((type_id, 1)), Some(300)),
        &pool,
    )
    .await;

    let rows = mails(&pool, rcpt).await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    let (mail_id, sender_id, sender_name, cash, flags) = rows[0].clone();
    assert_eq!(
        (sender_id, sender_name.as_str(), cash, flags),
        (Some(gm), "SsuOneGmCod", 300, MAIL_COD)
    );
    let escrow = escrow_for(&pool, rcpt).await;
    assert_eq!(escrow.len(), 1);
    assert_eq!(escrow[0].mail_id, mail_id);
    assert_eq!(naquadah(&pool, gm).await, 0, "no postage");
    assert_eq!(
        lines(c.take()),
        vec![format!(
            "Mail {mail_id} sent to SsuOneCodRcpt with 1 x item {type_id}, COD 300."
        )]
    );

    cleanup(&pool, acct).await;
}

/// Type 12: an unknown recipient writes nothing, answers the GM, and logs
/// `mail.gm_rejected reason=unknown_recipient` with the GM's ids.
#[tokio::test]
async fn live_db_gm_mail_unknown_recipient_writes_nothing() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, gm) = (0x7300_5180, 0x7300_5181);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(gm, "SsuOneGmMiss")]).await;
    let c = Client::new(0x7300_5192, gm, 54_762, "SsuOneGmMiss");

    c.gm(send(&c, Some("SsuOneNobodyAtAll"), 5, None, None), &pool)
        .await;

    assert_eq!(mail_count(&pool, gm).await, 0);
    let written: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_gate_mail WHERE sender_name = $1")
            .bind("SsuOneGmMiss")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(written, 0);
    assert_eq!(
        lines(c.take()),
        vec![".mail: no character is called SsuOneNobodyAtAll.".to_string()]
    );
    let row = capture
        .find_event(
            tracing::Level::WARN,
            "GM mail command refused",
            "unknown_recipient",
        )
        .expect("mail.gm_rejected reason=unknown_recipient");
    assert!(row.has_field("event", "mail.gm_rejected"));
    assert!(row.has_field("player_id", &gm.to_string()));

    cleanup(&pool, acct).await;
}

/// `.mailbox <name>` reports open and archived counts, system mail, and
/// what is in escrow: items, gift cash (COD prices excluded) and unpaid
/// COD.
#[tokio::test]
async fn live_db_gm_mailbox_reports_counts_and_escrow() {
    let pool = require_db_or_skip!();
    let (acct, gm, rcpt) = (0x7300_5188, 0x7300_5189, 0x7300_518A);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(gm, "SsuOneGmBox"), (rcpt, "SsuOneBoxRcpt")]).await;
    let type_id = any_type_id(&pool).await;
    let c = Client::new(0x7300_5193, gm, 54_763, "SsuOneGmBox");
    fill_mailbox(&pool, rcpt, 2, 0).await;
    fill_mailbox(&pool, rcpt, 1, MAIL_ARCHIVE).await;
    c.gm(
        send(&c, Some("SsuOneBoxRcpt"), 70, Some((type_id, 1)), None),
        &pool,
    )
    .await;
    c.gm(
        send(&c, Some("SsuOneBoxRcpt"), 0, Some((type_id, 1)), Some(40)),
        &pool,
    )
    .await;
    c.take();

    c.gm(
        MailGmCellToBase::Mailbox {
            actor: actor(&c),
            name: Some("SsuOneBoxRcpt".into()),
        },
        &pool,
    )
    .await;

    let got = lines(c.take());
    assert_eq!(got.len(), 3, "{got:?}");
    assert_eq!(
        got[0],
        format!(
            "Mailbox of SsuOneBoxRcpt ({rcpt}): 4 open of 100, 1 archived, 4 from the system, \
             0 quarantined."
        )
    );
    assert_eq!(
        got[1],
        "In escrow: 2 item(s), 70 naquadah gift cash, 1 unpaid COD."
    );
    // SS-M4: the two GM mails expire 720 h after they were written; the
    // fillers carry no expiry.
    let next: i32 =
        sqlx::query_scalar("SELECT MIN(expires_at) FROM sgw_gate_mail WHERE character_id = $1")
            .bind(rcpt)
            .fetch_one(&pool)
            .await
            .unwrap();
    // 720 h less the seconds the test took, floored.
    assert!(
        [719, 720]
            .iter()
            .any(|h| got[2] == format!("Next expiry: in {h} hour(s) (at {next}).")),
        "{got:?}"
    );

    cleanup(&pool, acct).await;
}

/// Type 12: the base re-checks the COD rules the cell parses, so a COD mail
/// that could never be paid (a price below 1) is never written:
/// `mail.gm_rejected reason=cod_price_invalid` with what the GM asked for.
#[tokio::test]
async fn live_db_gm_mail_refuses_cod_without_a_price() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, gm) = (0x7300_5184, 0x7300_5185);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(gm, "SsuOneGmZeroCod")]).await;
    let type_id = any_type_id(&pool).await;
    let c = Client::new(0x7300_5194, gm, 54_764, "SsuOneGmZeroCod");

    c.gm(send(&c, None, 0, Some((type_id, 1)), Some(0)), &pool)
        .await;

    assert_eq!(mail_count(&pool, gm).await, 0);
    assert!(escrow_for(&pool, gm).await.is_empty());
    assert_eq!(lines(c.take()).len(), 1);
    let row = capture
        .find_event(
            tracing::Level::WARN,
            "GM mail command refused",
            "cod_price_invalid",
        )
        .expect("mail.gm_rejected reason=cod_price_invalid");
    assert!(row.has_field("cod", "0"));
    assert!(row.has_field("item_type_id", &type_id.to_string()));

    cleanup(&pool, acct).await;
}

/// SS-M4 (SS-U1 integration edit 1): `.mail_expire <id>` makes the mail due
/// now and expires it at once by the sweep's path. A gift mail from a
/// player goes back to that player; the GM is told which path it took, and
/// `mail.gm_action action=mail_expire` names the GM and the mailbox. Fails
/// if the refusal comes back or the command stops expiring the mail.
#[tokio::test]
async fn live_db_gm_mail_expire_expires_the_mail_now() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, gm, owner, sender) = (0x7300_2400, 0x7300_2401, 0x7300_2402, 0x7300_2403);
    cleanup(&pool, acct).await;
    insert_players(
        &pool,
        acct,
        &[
            (gm, "SsmFourGmExp"),
            (owner, "SsmFourGmExpO"),
            (sender, "SsmFourGmExpS"),
        ],
    )
    .await;
    let mail_id = AttachedMail::from(owner, sender, "SsmFourGmExpS")
        .cash(30)
        .insert(&pool)
        .await;
    let c = Client::new(0x7300_2410, gm, 55_250, "SsmFourGmExp");

    c.gm(
        MailGmCellToBase::Expire {
            actor: actor(&c),
            mail_id,
        },
        &pool,
    )
    .await;

    let row = expiry_row(&pool, mail_id)
        .await
        .expect("returned, not gone");
    assert_eq!((row.character_id, row.cash), (sender, 30), "{row:?}");
    assert!(row.returned);
    assert_eq!(
        lines(c.take()),
        vec![format!(
            "Mail {mail_id} expired and was returned to its sender ({sender}) with its attachments."
        )]
    );
    let ev = capture
        .all()
        .into_iter()
        .find(|e| e.has_field("event", "mail.gm_action") && e.has_field("action", "mail_expire"))
        .expect("mail.gm_action action=mail_expire");
    assert!(ev.has_field("player_id", &gm.to_string()));
    assert!(ev.has_field("subject_player_id", &owner.to_string()));
    assert!(ev.has_field("path", "returned"));

    cleanup(&pool, acct).await;
}

/// Type 12: `.mail_expire` refuses archived mail (it never expires), a
/// quarantined mail (it already took its path) and an unknown id, each with
/// `mail.gm_rejected reason=<why>` and the mail id, a line naming why, and
/// no change to the row.
#[tokio::test]
async fn live_db_gm_mail_expire_refuses_archived_quarantined_and_unknown_mail() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, gm, owner) = (0x7300_2420, 0x7300_2421, 0x7300_2422);
    cleanup(&pool, acct).await;
    insert_players(
        &pool,
        acct,
        &[(gm, "SsmFourGmRef"), (owner, "SsmFourGmRefO")],
    )
    .await;
    let archived = AttachedMail::from(owner, gm, "SsmFourGmRef")
        .insert(&pool)
        .await;
    sqlx::query("UPDATE sgw_gate_mail SET flags = $2 WHERE mail_id = $1")
        .bind(archived)
        .bind(MAIL_ARCHIVE)
        .execute(&pool)
        .await
        .unwrap();
    let quarantined = AttachedMail::from(owner, gm, "SsmFourGmRef")
        .cash(4)
        .insert(&pool)
        .await;
    sqlx::query("UPDATE sgw_gate_mail SET quarantined = true WHERE mail_id = $1")
        .bind(quarantined)
        .execute(&pool)
        .await
        .unwrap();
    let c = Client::new(0x7300_2430, gm, 55_251, "SsmFourGmRef");

    for (mail_id, reason, says) in [
        (archived, "archived", "archived mail never expires"),
        (quarantined, "quarantined", "already quarantined"),
        (0x7300_2439, "mail_not_found", "no mail has id"),
    ] {
        let before = expiry_row(&pool, mail_id).await;
        c.gm(
            MailGmCellToBase::Expire {
                actor: actor(&c),
                mail_id,
            },
            &pool,
        )
        .await;
        let got = lines(c.take());
        assert!(got.len() == 1 && got[0].contains(says), "{reason}: {got:?}");
        assert_eq!(expiry_row(&pool, mail_id).await, before, "{reason}");
        let ev = capture
            .find_event(tracing::Level::WARN, "GM mail command refused", reason)
            .unwrap_or_else(|| panic!("mail.gm_rejected reason={reason}"));
        assert!(ev.has_field("command", "mail_expire"));
        assert!(ev.has_field("mail_id", &mail_id.to_string()));
    }

    cleanup(&pool, acct).await;
}
