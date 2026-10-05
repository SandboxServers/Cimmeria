//! `.mailbox [name]` (SS-U1): one mailbox's counts, what is in escrow,
//! what is quarantined (SS-M4) and the next expiry.

use sqlx::PgPool;

use super::super::claim::unix_now;
use super::super::Caller;
use super::{feedback, rejected, resolve_one, GmRefusal};
use crate::cell::mail::codes::flags::{MAIL_ARCHIVE, MAIL_COD};
use crate::cell::messages::MailGmActor;

/// One mailbox, as `.mailbox` reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub(super) struct MailboxSummary {
    pub(super) open: i64,
    pub(super) archived: i64,
    /// Mails holding an escrowed item.
    pub(super) items: i64,
    /// Gift cash waiting on mails that are not COD.
    pub(super) gift_cash: i64,
    /// Mails still flagged COD (unpaid).
    pub(super) cod: i64,
    /// Mails with no sender character (system mail).
    pub(super) system: i64,
    /// Expired mail kept for a GM (SS-M4, D-SS04 path 3). Not in `open`
    /// or `archived`.
    pub(super) quarantined: i64,
    /// The soonest `expires_at` of an open mail, if any expires.
    pub(super) next_expiry: Option<i32>,
}

pub(super) const MAILBOX_SUMMARY_SQL: &str = "SELECT \
       COUNT(*) FILTER (WHERE (m.flags & $2) = 0 AND NOT m.quarantined) AS open, \
       COUNT(*) FILTER (WHERE (m.flags & $2) <> 0 AND NOT m.quarantined) AS archived, \
       COUNT(i.mail_id) AS items, \
       COALESCE(SUM(m.cash) FILTER (WHERE (m.flags & $3) = 0), 0)::bigint AS gift_cash, \
       COUNT(*) FILTER (WHERE (m.flags & $3) <> 0) AS cod, \
       COUNT(*) FILTER (WHERE m.sender_id IS NULL) AS system, \
       COUNT(*) FILTER (WHERE m.quarantined) AS quarantined, \
       MIN(m.expires_at) FILTER (WHERE NOT m.quarantined AND (m.flags & $2) = 0) \
           AS next_expiry \
     FROM sgw_gate_mail m LEFT JOIN sgw_gate_mail_item i ON i.mail_id = m.mail_id \
     WHERE m.character_id = $1";

/// `.mailbox [name]`.
pub(super) async fn gm_mailbox(
    caller: &Caller<'_>,
    pool: &PgPool,
    actor: MailGmActor,
    name: Option<&str>,
) {
    let result: Result<(i32, String, MailboxSummary), GmRefusal> = async {
        let mut conn = pool.acquire().await?;
        let (player_id, stored) = match name {
            Some(name) => resolve_one(&mut conn, name).await?,
            None => {
                let own: Option<String> =
                    sqlx::query_scalar("SELECT player_name FROM sgw_player WHERE player_id = $1")
                        .bind(actor.player_id)
                        .fetch_optional(&mut *conn)
                        .await?;
                (actor.player_id, own.ok_or(GmRefusal::GmMissing)?)
            }
        };
        let summary: MailboxSummary = sqlx::query_as(MAILBOX_SUMMARY_SQL)
            .bind(player_id)
            .bind(MAIL_ARCHIVE)
            .bind(MAIL_COD)
            .fetch_one(&mut *conn)
            .await?;
        Ok((player_id, stored, summary))
    }
    .await;
    match result {
        Ok((player_id, stored, s)) => {
            let who = caller.identity();
            tracing::info!(
                target: "mail",
                event = "mail.gm_action",
                action = "mailbox",
                entity_id = actor.entity_id,
                entity_name = who.player_name,
                account_id = actor.account_id,
                account_name = who.account_name,
                player_id = actor.player_id,
                player_name = who.player_name,
                subject_player_id = player_id,
                subject_player_name = stored.as_str(),
                open = s.open,
                archived = s.archived,
                items = s.items,
                gift_cash = s.gift_cash,
                cod = s.cod,
                quarantined = s.quarantined,
                next_expiry = s.next_expiry,
                "GM .mailbox read",
            );
            for line in mailbox_lines(&stored, player_id, &s, unix_now()) {
                feedback(caller, &line).await;
            }
        }
        Err(refusal) => {
            rejected(caller, actor, "mailbox", refusal.reason());
            feedback(caller, &refusal.text().replacen(".mail:", ".mailbox:", 1)).await;
        }
    }
}

/// The feedback lines for one mailbox, at `now` (epoch seconds).
pub(super) fn mailbox_lines(
    name: &str,
    player_id: i32,
    s: &MailboxSummary,
    now: i32,
) -> Vec<String> {
    let next = match s.next_expiry {
        None => "Next expiry: none (no open mail expires).".to_string(),
        Some(at) if at <= now => {
            format!("Next expiry: due now (at {at}); the sweep takes it within 5 minutes.")
        }
        Some(at) => format!(
            "Next expiry: in {} hour(s) (at {at}).",
            (i64::from(at) - i64::from(now)) / 3600
        ),
    };
    vec![
        format!(
            "Mailbox of {name} ({player_id}): {} open of {}, {} archived, {} from the system, \
             {} quarantined.",
            s.open,
            super::super::send::MAILBOX_CAP,
            s.archived,
            s.system,
            s.quarantined
        ),
        format!(
            "In escrow: {} item(s), {} naquadah gift cash, {} unpaid COD.",
            s.items, s.gift_cash, s.cod
        ),
        next,
    ]
}
