//! The delivery transaction for a gate-mail send (D-SS06 shape), text-only
//! or with one attachment (SS-M2).

use std::collections::HashMap;
use std::fmt;

use sqlx::PgPool;

use super::attachment::{Attachment, AttachmentRefusal};
use super::escrow::{check_source, debit, escrow_item, lock_source_item, Debit, EscrowedItem};
use super::recipients::{
    candidate_rows, ignoring_sender, resolve_names, FailReason, FailedRecipient, Resolution,
};
use crate::cell::mail::codes::flags::MAIL_ARCHIVE;
use crate::cell::mail::codes::MailResult;
use crate::cell::messages::MailSend;

/// Open messages a mailbox may hold before player mail to it is refused
/// (D-SS03: the client warns at 90 and shows "100% Full" at 100). Counts
/// mail that is not archived. Project policy; server-generated mail (SS-M3,
/// SS-M4) is exempt.
pub(in super::super) const MAILBOX_CAP: i64 = 100;

/// One delivered copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Delivered {
    pub(super) player_id: i32,
    pub(super) mail_id: i32,
}

/// What a committed attached send moved: the sender's balance and, if an
/// item was attached, the escrowed item. Read inside the transaction, so
/// the client is told exactly what committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct AttachedOutcome {
    pub(in super::super) mail_id: i32,
    pub(in super::super) recipient_id: i32,
    pub(in super::super) debit: Debit,
    pub(in super::super) item: Option<EscrowedItem>,
}

/// The outcome of a committed send.
#[derive(Debug, Default)]
pub(super) struct Delivery {
    pub(super) delivered: Vec<Delivered>,
    /// Resolution failures in the order typed, then lock-time failures
    /// (mailbox full, ignoring) in the order typed.
    pub(super) failed: Vec<FailedRecipient>,
    /// Set when the send carried an attachment and was delivered.
    pub(super) attached: Option<AttachedOutcome>,
}

#[derive(Debug)]
pub(super) enum DeliverError {
    /// The sender's own `sgw_player` row is gone.
    SenderMissing,
    /// The attachment failed a check that needs the database: the item, or
    /// the balance. Nothing was written.
    Refused {
        refusal: AttachmentRefusal,
        /// The sender's balance under the lock, for the refusal log.
        balance: Option<i32>,
    },
    Db(sqlx::Error),
}

impl fmt::Display for DeliverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeliverError::SenderMissing => f.write_str("sender row missing"),
            DeliverError::Refused { refusal, .. } => f.write_str(refusal.reason),
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
/// 2. With an item attached, take the sender's inventory advisory locks and
///    lock the item row, before any player row (the shared inventory lock
///    order, `crafting/inventory_locks.rs`).
/// 3. Lock the sender's and every recipient's `sgw_player` row with
///    `FOR UPDATE`, in ascending `player_id` order, so two sends to the same
///    mailbox serialise and two sends in opposite directions cannot deadlock.
/// 4. Drop recipients whose Ignore list holds the sender's stored name
///    (D-SS15), read under the lock. An attached send to an ignoring
///    recipient therefore ends here, before any debit or escrow move.
/// 5. Count each recipient's open mail **under that lock** and refuse those
///    at [`MAILBOX_CAP`]. Without the lock, two senders racing for a 99-mail
///    box would both count 99 and both insert.
/// 6. Text only: insert one row per remaining recipient, with the sender's
///    id and the name stored on its own row. With an attachment (one
///    recipient, D-SS05): check the item, debit cash plus postage, insert
///    the mail, move the item into escrow (D-SS06, D-SS08).
/// 7. Commit.
///
/// Any error rolls the whole send back (the transaction drops uncommitted):
/// cash, inventory and mail are untouched.
pub(super) async fn deliver(
    pool: &PgPool,
    sender_id: i32,
    send: &MailSend,
    attachment: Option<&Attachment>,
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

    let item_request = attachment.and_then(|a| a.item);
    let source = match item_request {
        Some(item) => lock_source_item(&mut tx, sender_id, item).await?,
        None => None,
    };

    let mut lock_ids = ids.clone();
    lock_ids.push(sender_id);
    lock_ids.sort_unstable();
    lock_ids.dedup();
    let locked: Vec<(i32, String, i32)> = sqlx::query_as(
        "SELECT player_id, player_name, naquadah FROM sgw_player \
         WHERE player_id = ANY($1) ORDER BY player_id FOR UPDATE",
    )
    .bind(&lock_ids)
    .fetch_all(&mut *tx)
    .await?;
    let balance = locked
        .iter()
        .find(|(id, _, _)| *id == sender_id)
        .map(|(_, _, cash)| *cash);
    let names: HashMap<i32, String> = locked.into_iter().map(|(id, name, _)| (id, name)).collect();
    let Some(sender_name) = names.get(&sender_id).cloned() else {
        return Err(DeliverError::SenderMissing);
    };
    let ignoring = ignoring_sender(&mut tx, &sender_name, &ids).await?;

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

    match (attachment, deliver_to.as_slice()) {
        // Nobody to deliver to: nothing is written, nothing is debited.
        (_, []) => {}
        (None, _) => {
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
        (Some(attachment), [recipient_id]) => {
            let refused = |refusal| DeliverError::Refused { refusal, balance };
            let source = match item_request {
                Some(item) => Some((
                    check_source(source.as_ref(), item.quantity).map_err(refused)?,
                    item.quantity,
                )),
                None => None,
            };
            let debit = debit(&mut tx, sender_id, attachment.sender_cost())
                .await?
                .map_err(refused)?;
            let mail_id: i32 = sqlx::query_scalar(
                "INSERT INTO sgw_gate_mail \
                    (character_id, sender_id, sender_name, subject, message, cash, \
                     sent_time, read_time, flags, item_id) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, 0, $8, NULL) \
                 RETURNING mail_id",
            )
            .bind(recipient_id)
            .bind(sender_id)
            .bind(&sender_name)
            .bind(&send.subject)
            .bind(&send.body)
            .bind(i64::from(attachment.cash))
            .bind(sent_time)
            .bind(attachment.mail_flags())
            .fetch_one(&mut *tx)
            .await?;
            let item = match source {
                Some((source, quantity)) => Some(
                    escrow_item(&mut tx, mail_id, sender_id, source, quantity, sent_time).await?,
                ),
                None => None,
            };
            delivery.delivered = vec![Delivered {
                player_id: *recipient_id,
                mail_id,
            }];
            delivery.attached = Some(AttachedOutcome {
                mail_id,
                recipient_id: *recipient_id,
                debit,
                item,
            });
        }
        // The caller refuses an attachment with two or more typed names
        // before delivery, but two names can still resolve to two players
        // ("Bob" and a unique fold of another). Refuse the whole send.
        (Some(_), _) => {
            return Err(DeliverError::Refused {
                refusal: AttachmentRefusal::new(
                    MailResult::AttachmentsAndMultipleRecipients,
                    "attachment_with_multiple_recipients",
                    "Gate-mail with naquadah or an item can go to one recipient only. \
                     The message was not sent.",
                ),
                balance,
            });
        }
    }

    tx.commit().await?;
    Ok(delivery)
}
