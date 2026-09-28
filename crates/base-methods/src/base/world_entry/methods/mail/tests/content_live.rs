//! Live-DB tests for the base half of the content engine's
//! `send_system_mail` action (SS-U3, `mail/content.rs`): the Gate Mail
//! Clerk's mail and its per-player cooldown. The chain and the executor arm
//! are guarded on the cell (`cell-content`
//! `chain_replay_tests/debug_hub_mail_clerk.rs`). Sentinels `0x7300_5300` to
//! `0x7300_53FF`.

use super::super::content::{
    refused_line, wait_text, write_content_mail, ContentRefusal, ContentSent,
};
use super::packets::{Client, Received};
use super::*;
use crate::cell::messages::{ContentMailCooldown, ContentSystemMail};
use crate::test_support::LogCapture;

/// Health Slappack TC1, max stack 10: the clerk's item.
const SLAPPACK: i32 = 2893;
const CLERK_KEY: &str = "send_system_mail/7011";

/// The clerk's mail, as chain 7011 sends it, for `c`.
fn clerk_mail(c: &Client) -> ContentSystemMail {
    ContentSystemMail {
        entity_id: c.entity_id,
        player_id: c.player_id,
        account_id: Some(0x7300_0001),
        chain_id: 7011,
        sender_name: "Gate Mail Clerk".into(),
        subject: "Gate Mail test delivery".into(),
        body: "A test mail.".into(),
        cash: 50,
        item: Some((SLAPPACK, 5)),
        cooldown: Some(ContentMailCooldown {
            key: CLERK_KEY.into(),
            secs: 600,
        }),
    }
}

fn lines(received: Vec<Received>) -> Vec<String> {
    received
        .into_iter()
        .filter_map(|r| match r {
            Received::Feedback(t) => Some(t),
            _ => None,
        })
        .collect()
}

/// `(sender_id, sender_name, cash, flags)` of every mail `character_id`
/// holds.
async fn mails(pool: &PgPool, character_id: i32) -> Vec<(Option<i32>, String, i64, i32)> {
    sqlx::query_as(
        "SELECT sender_id, sender_name, cash, flags FROM sgw_gate_mail \
         WHERE character_id = $1 ORDER BY mail_id",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn claim_at(pool: &PgPool, player_id: i32) -> Option<i32> {
    sqlx::query_scalar(
        "SELECT last_used_at FROM sgw_player_content_cooldown \
         WHERE player_id = $1 AND cooldown_key = $2",
    )
    .bind(player_id)
    .bind(CLERK_KEY)
    .fetch_optional(pool)
    .await
    .unwrap()
}

fn sent(r: Result<ContentSent, ContentRefusal>) -> ContentSent {
    r.unwrap_or_else(|e| panic!("the mail must be sent, got {e:?}"))
}

/// The packet's acceptance test. The first press writes exactly one system
/// mail (no sender character, 50 naquadah, one escrow row of 5 slappacks)
/// and tells the player; a second press straight after writes nothing, is
/// told the wait, and logs `content.send_system_mail reason=cooldown` with
/// the player's ids.
#[tokio::test]
async fn live_db_content_mail_sends_once_then_refuses_inside_the_cooldown() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let (acct, player) = (0x7300_5300, 0x7300_5301);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(player, "SsuThreeClerk")]).await;
    set_naquadah(&pool, player, 7).await;
    let c = Client::new(0x7300_5390, player, 54_830, "SsuThreeClerk");

    c.content(clerk_mail(&c), Some(&pool)).await;

    assert_eq!(
        mails(&pool, player).await,
        vec![(None, "Gate Mail Clerk".to_string(), 50, 0)]
    );
    let escrow = escrow_for(&pool, player).await;
    assert_eq!(escrow.len(), 1, "{escrow:?}");
    assert_eq!((escrow[0].type_id, escrow[0].stack_size), (SLAPPACK, 5));
    assert_eq!(
        naquadah(&pool, player).await,
        7,
        "the cash waits in the mail"
    );
    let mail_id = escrow[0].mail_id;
    assert_eq!(
        lines(c.take()),
        vec![format!(
            "Gate Mail Clerk sent you mail {mail_id} with 50 naquadah and 5 x Health \
             Slappack TC1. Open your mail to take them."
        )]
    );
    assert!(capture
        .all()
        .iter()
        .any(|e| e.has_field("event", "mail.system_sent")
            && e.has_field("mail_id", &mail_id.to_string())));

    c.content(clerk_mail(&c), Some(&pool)).await;

    assert_eq!(
        mail_count(&pool, player).await,
        1,
        "the second press wrote nothing"
    );
    assert_eq!(escrow_for(&pool, player).await.len(), 1);
    assert_eq!(
        lines(c.take()),
        vec!["Gate Mail Clerk has already sent you mail. You can ask again in 10 minutes."]
    );
    let refusal = capture
        .all()
        .into_iter()
        .find(|e| e.target == "content" && e.has_field("reason", "cooldown"))
        .expect("content.send_system_mail reason=cooldown");
    assert_eq!(refusal.level, tracing::Level::WARN);
    for (k, v) in [
        ("event", "content.send_system_mail".to_string()),
        ("player_id", player.to_string()),
        ("entity_id", 0x7300_5390_u32.to_string()),
        ("account_id", 0x7300_0001.to_string()),
        ("chain_id", "7011".to_string()),
        ("cooldown_key", CLERK_KEY.to_string()),
    ] {
        assert!(refusal.has_field(k, &v), "{k}={v}: {refusal:#?}");
    }

    cleanup(&pool, acct).await;
}

/// The window is measured from the claim, not from the mail: deleting the
/// mail does not reopen it, one second short is still refused with 1 s
/// left, and at exactly `secs` a new mail goes out and moves the claim.
#[tokio::test]
async fn live_db_content_mail_cooldown_outlives_the_mail_and_expires_on_time() {
    let pool = require_db_or_skip!();
    let (acct, player) = (0x7300_5310, 0x7300_5311);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(player, "SsuThreeWindow")]).await;
    let c = Client::new(0x7300_5391, player, 54_831, "SsuThreeWindow");
    let msg = clerk_mail(&c);
    let t0: i64 = 1_900_000_000;

    sent(write_content_mail(&pool, &msg, t0).await);
    assert_eq!(claim_at(&pool, player).await, Some(t0 as i32));

    // Take-and-delete: the mail and its escrow row are gone.
    sqlx::query("DELETE FROM sgw_gate_mail WHERE character_id = $1")
        .bind(player)
        .execute(&pool)
        .await
        .unwrap();

    match write_content_mail(&pool, &msg, t0 + 599).await {
        Err(ContentRefusal::Cooldown {
            last_used_at,
            remaining_secs,
        }) => assert_eq!((last_used_at, remaining_secs), (t0 as i32, 1)),
        other => panic!("one second short must be refused, got {other:?}"),
    }
    assert_eq!(mail_count(&pool, player).await, 0);
    assert_eq!(
        claim_at(&pool, player).await,
        Some(t0 as i32),
        "claim unmoved"
    );

    sent(write_content_mail(&pool, &msg, t0 + 600).await);
    assert_eq!(mail_count(&pool, player).await, 1);
    assert_eq!(claim_at(&pool, player).await, Some((t0 + 600) as i32));

    cleanup(&pool, acct).await;
}

/// The claim commits with the mail or not at all: a mail the writer refuses
/// (an unknown item type) leaves no claim behind, so the player can ask
/// again at once; and a mail without a cooldown is sent on every firing.
#[tokio::test]
async fn live_db_content_mail_claim_rolls_back_with_a_refused_mail() {
    let pool = require_db_or_skip!();
    let (acct, player) = (0x7300_5320, 0x7300_5321);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(player, "SsuThreeRollback")]).await;
    let c = Client::new(0x7300_5392, player, 54_832, "SsuThreeRollback");
    let t0: i64 = 1_900_000_000;

    let mut bad = clerk_mail(&c);
    bad.item = Some((0x7300_53FF, 1));
    match write_content_mail(&pool, &bad, t0).await {
        Err(ContentRefusal::Mail(SystemMailError::UnknownItemType)) => {}
        other => panic!("an unknown item type must be refused, got {other:?}"),
    }
    assert_eq!(claim_at(&pool, player).await, None, "the claim rolled back");
    assert_eq!(mail_count(&pool, player).await, 0);
    sent(write_content_mail(&pool, &clerk_mail(&c), t0 + 1).await);

    let mut free = clerk_mail(&c);
    free.cooldown = None;
    sent(write_content_mail(&pool, &free, t0 + 2).await);
    sent(write_content_mail(&pool, &free, t0 + 2).await);
    assert_eq!(mail_count(&pool, player).await, 3);

    cleanup(&pool, acct).await;
}

/// Two presses at the same instant (two cell messages racing on the base)
/// write one mail: the recipient row lock orders them, and the second's
/// claim finds the first's.
#[tokio::test]
async fn live_db_content_mail_concurrent_presses_write_one_mail() {
    let pool = require_db_or_skip!();
    let (acct, player) = (0x7300_5330, 0x7300_5331);
    cleanup(&pool, acct).await;
    insert_players(&pool, acct, &[(player, "SsuThreeRace")]).await;
    let c = Client::new(0x7300_5393, player, 54_833, "SsuThreeRace");
    let msg = clerk_mail(&c);
    let t0: i64 = 1_900_000_000;

    let (a, b) = tokio::join!(
        write_content_mail(&pool, &msg, t0),
        write_content_mail(&pool, &msg, t0)
    );
    let oks = [a.is_ok(), b.is_ok()].iter().filter(|x| **x).count();
    assert_eq!(oks, 1, "exactly one press wins: {a:?} / {b:?}");
    assert!(
        matches!(a, Err(ContentRefusal::Cooldown { .. }))
            || matches!(b, Err(ContentRefusal::Cooldown { .. })),
        "the loser is a cooldown refusal"
    );
    assert_eq!(mail_count(&pool, player).await, 1);

    cleanup(&pool, acct).await;
}

/// A player row that does not exist is `RecipientNotFound` and writes no
/// claim; with no database the player is still answered (type 12).
#[tokio::test]
async fn live_db_content_mail_refusals_answer_the_player() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let ghost = Client::new(0x7300_5394, 0x7300_53FE, 54_834, "SsuThreeGhost");
    match write_content_mail(&pool, &clerk_mail(&ghost), 1_900_000_000).await {
        Err(ContentRefusal::Mail(SystemMailError::RecipientNotFound)) => {}
        other => panic!("a missing player must be RecipientNotFound, got {other:?}"),
    }

    ghost.content(clerk_mail(&ghost), None).await;
    assert_eq!(
        lines(ghost.take()),
        vec!["Gate Mail Clerk could not send your mail: the mail service is unavailable."]
    );
    let row = capture
        .all()
        .into_iter()
        .find(|e| e.target == "content" && e.has_field("reason", "no_db_pool"))
        .expect("content.send_system_mail reason=no_db_pool");
    assert!(row.has_field("player_id", &0x7300_53FE_i32.to_string()));
}

/// The wait is rounded up to whole minutes, never down.
#[test]
fn content_mail_wait_text_rounds_up() {
    assert_eq!(wait_text(600), "10 minutes");
    assert_eq!(wait_text(541), "10 minutes");
    assert_eq!(wait_text(61), "2 minutes");
    assert_eq!(wait_text(60), "1 minute");
    assert_eq!(wait_text(59), "59 seconds");
    assert_eq!(wait_text(1), "1 second");
    assert_eq!(wait_text(0), "1 second");
    assert_eq!(
        refused_line(
            "Clerk",
            &ContentRefusal::Cooldown {
                last_used_at: 0,
                remaining_secs: 30
            }
        ),
        "Clerk has already sent you mail. You can ask again in 30 seconds."
    );
}
