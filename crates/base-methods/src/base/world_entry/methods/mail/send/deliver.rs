//! The delivery transaction for a text-only gate-mail send (D-SS06 shape).

use std::collections::HashMap;
use std::fmt;

use sqlx::PgPool;

use super::recipients::{
    candidate_rows, ignoring_sender, resolve_names, FailReason, FailedRecipient, Resolution,
};
use crate::cell::mail::codes::flags::MAIL_ARCHIVE;
use crate::cell::messages::MailSend;

/// Open messages a mailbox may hold before player mail to it is refused
/// (D-SS03: the client warns at 90 and shows "100% Full" at 100). Counts
/// mail that is not archived. Project policy; server-generated mail (SS-M3,
/// SS-M4) is exempt.
pub(super) const MAILBOX_CAP: i64 = 100;

/// One delivered copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Delivered {
    pub(super) player_id: i32,
    pub(super) mail_id: i32,
}

/// The outcome of a committed send.
#[derive(Debug, Default)]
pub(super) struct Delivery {
    pub(super) delivered: Vec<Delivered>,
    /// Resolution failures in the order typed, then lock-time failures
    /// (mailbox full, ignoring) in the order typed.
    pub(super) failed: Vec<FailedRecipient>,
}

#[derive(Debug)]
pub(super) enum DeliverError {
    /// The sender's own `sgw_player` row is gone.
    SenderMissing,
    Db(sqlx::Error),
}

impl fmt::Display for DeliverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeliverError::SenderMissing => f.write_str("sender row missing"),
            DeliverError::Db(e) => write!(f, "{e}"),
        }
    }
}

impl From<sqlx::Error> for DeliverError {
    fn from(e: sqlx::Error) -> Self {
        DeliverError::Db(e)
    }
}

/// Deliver `send` from `sender_id` in one transaction.
///
/// 1. Resolve every typed name (D-SS13) and drop duplicates by `player_id`.
/// 2. Drop recipients who ignore the sender (D-SS15, a seam until SS-C1).
/// 3. Lock the sender's and every recipient's `sgw_player` row with
///    `FOR UPDATE`, in ascending `player_id` order, so two sends to the same
///    mailbox serialise and two sends in opposite directions cannot deadlock.
/// 4. Count each recipient's open mail **under that lock** and refuse those
///    at [`MAILBOX_CAP`]. Without the lock, two senders racing for a 99-mail
///    box would both count 99 and both insert.
/// 5. Insert one row per remaining recipient, with the sender's id and the
///    name stored on its own row, and commit.
///
/// Any error rolls the whole send back (the transaction drops uncommitted).
pub(super) async fn deliver(
    pool: &PgPool,
    sender_id: i32,
    send: &MailSend,
    sent_time: i32,
) -> Result<Delivery, DeliverError> {
    let mut tx = pool.begin().await?;

    let rows = candidate_rows(&mut tx, &send.recipients).await?;
    let resolutions = resolve_names(&send.recipients, &rows);

    let mut delivery = Delivery::default();
    // (typed name, player_id), first occurrence of each player only.
    let mut targets: Vec<(String, i32)> = Vec::new();
    for (typed, res) in send.recipients.iter().zip(resolutions) {
        match res {
            Resolution::Found { player_id } => {
                if !targets.iter().any(|(_, id)| *id == player_id) {
                    targets.push((typed.clone(), player_id));
                }
            }
            Resolution::Failed(reason) => delivery.failed.push(FailedRecipient {
                typed: typed.clone(),
                player_id: None,
                reason,
            }),
        }
    }

    let ids: Vec<i32> = targets.iter().map(|(_, id)| *id).collect();
    let ignoring = ignoring_sender(&mut tx, sender_id, &ids).await?;

    let mut lock_ids = ids.clone();
    lock_ids.push(sender_id);
    lock_ids.sort_unstable();
    lock_ids.dedup();
    let locked: Vec<(i32, String)> = sqlx::query_as(
        "SELECT player_id, player_name FROM sgw_player \
         WHERE player_id = ANY($1) ORDER BY player_id FOR UPDATE",
    )
    .bind(&lock_ids)
    .fetch_all(&mut *tx)
    .await?;
    let names: HashMap<i32, String> = locked.into_iter().collect();
    let Some(sender_name) = names.get(&sender_id).cloned() else {
        return Err(DeliverError::SenderMissing);
    };

    let open: HashMap<i32, i64> = sqlx::query_as::<_, (i32, i64)>(
        "SELECT character_id, COUNT(*) FROM sgw_gate_mail \
         WHERE character_id = ANY($1) AND (flags & $2) = 0 \
         GROUP BY character_id",
    )
    .bind(&ids)
    .bind(MAIL_ARCHIVE)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .collect();

    let mut deliver_to: Vec<i32> = Vec::new();
    for (typed, player_id) in targets {
        let reason = if !names.contains_key(&player_id) {
            // Deleted between the name lookup and the lock.
            Some(FailReason::Unknown)
        } else if ignoring.contains(&player_id) {
            Some(FailReason::Ignoring)
        } else if open.get(&player_id).copied().unwrap_or(0) >= MAILBOX_CAP {
            Some(FailReason::MailboxFull)
        } else {
            None
        };
        match reason {
            Some(reason) => delivery.failed.push(FailedRecipient {
                typed,
                player_id: Some(player_id),
                reason,
            }),
            None => deliver_to.push(player_id),
        }
    }

    if !deliver_to.is_empty() {
        let inserted: Vec<(i32, i32)> = sqlx::query_as(
            "INSERT INTO sgw_gate_mail \
                (character_id, sender_id, sender_name, subject, message, cash, \
                 sent_time, read_time, flags, item_id) \
             SELECT r, $2, $3, $4, $5, 0, $6, 0, 0, NULL FROM unnest($1::int[]) AS r \
             RETURNING character_id, mail_id",
        )
        .bind(&deliver_to)
        .bind(sender_id)
        .bind(&sender_name)
        .bind(&send.subject)
        .bind(&send.body)
        .bind(sent_time)
        .fetch_all(&mut *tx)
        .await?;
        delivery.delivered = inserted
            .into_iter()
            .map(|(player_id, mail_id)| Delivered { player_id, mail_id })
            .collect();
    }

    tx.commit().await?;
    Ok(delivery)
}
