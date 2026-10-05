//! `returnMailMessage` (CM 47): send a mail back to the player who sent it,
//! with its item and any gift cash (D-SS10, CAT-G-06). SS-M4's expiry
//! sweep reuses [`return_locked`] inside its own transaction. Lock order
//! and failure handling: [`super::claim`].

use sqlx::{PgConnection, PgPool};

use super::claim::{
    answer_failure, lock_escrow, lock_mail, lock_players, unix_now, LockedMail, Op, OpError,
    Refusal, NOT_FOUND,
};
use super::expiry::expires_at;
use super::notify::{notify_delivered, Delivery};
use super::MailCtx;
use crate::base::feedback::FeedbackCtx;
use crate::cell::mail;
use crate::cell::mail::codes::flags::{MAIL_ARCHIVE, MAIL_COD};
use crate::mercury::method_idx;

const ARCHIVED: Refusal = Refusal {
    reason: "archived",
    text: "Archived gate-mail cannot be returned.",
};
const ALREADY_RETURNED: Refusal = Refusal {
    reason: "already_returned",
    text: "That gate-mail message has already been returned once and cannot be \
           returned again.",
};
const COD_PAID: Refusal = Refusal {
    reason: "cod_paid",
    text: "You have already paid for this COD delivery, so it cannot be returned. \
           Take the item instead.",
};
/// No player to return to: server mail, or a sender whose character is
/// gone. The expiry sweep quarantines on it.
pub(super) const SYSTEM_MAIL: Refusal = Refusal {
    reason: "system_mail",
    text: "That gate-mail message has no player sender to return it to.",
};

/// A committed return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Returned {
    /// The stored `sender_id`, who now owns the mail.
    pub(super) to_player_id: i32,
    /// The stored `sender_name`, for the log line only.
    pub(super) to_player_name: String,
    /// Gift cash that went back with it.
    pub(super) cash: i64,
    /// The COD price cleared by the return (0 when it was no COD).
    pub(super) cod_cancelled: i64,
    /// The escrowed instance that went back with it, if any.
    pub(super) item_id: Option<i32>,
    /// That instance's item type, for the log line's `item_name` only.
    pub(super) item_type_id: Option<i32>,
}

/// CAT-G-06 / D-SS10: return `mail_id`, owned by `player_id`, to its stored
/// sender, once.
///
/// Refused for archived mail, mail already returned, a paid COD (the item
/// is the payer's; the seller already has the price), and mail with no
/// `sender_id` (server mail, or a sender whose character was deleted: the
/// foreign key sets it NULL). The destination is the stored `sender_id`,
/// never `sender_name`, so a mail cannot be redirected by a name. One
/// conditional `UPDATE` re-addresses the row: the returner becomes its
/// sender, `returned` is set so it can never loop, an unpaid COD is
/// cancelled with its amount **zeroed** (the price never becomes gift
/// cash), and it arrives unread with a fresh `sent_time` and a fresh
/// 30-day `expires_at` (SS-M4). The escrow row is keyed by `mail_id`, so
/// the item moves with the mail in the same transaction. Server mail:
/// exempt from the recipient's mailbox cap.
pub(super) async fn return_tx(
    pool: &PgPool,
    player_id: i32,
    mail_id: i32,
    now: i32,
) -> Result<Returned, OpError> {
    let mut tx = pool.begin().await?;
    let mail = lock_mail(&mut tx, player_id, mail_id)
        .await?
        .ok_or(NOT_FOUND)?;
    let returned = return_locked(&mut tx, player_id, mail_id, &mail, now).await?;
    tx.commit().await?;
    Ok(returned)
}

/// [`return_tx`]'s checks and write, on a mail the caller already locked
/// with [`lock_mail`] in `conn`'s transaction. Commits nothing. The expiry
/// sweep (SS-M4, D-SS04 path 1) calls it after its own lock and decision,
/// so the return is the same code whoever triggers it.
pub(super) async fn return_locked(
    conn: &mut PgConnection,
    player_id: i32,
    mail_id: i32,
    mail: &LockedMail,
    now: i32,
) -> Result<Returned, OpError> {
    if mail.archived() {
        return Err(ARCHIVED.into());
    }
    if mail.returned {
        return Err(ALREADY_RETURNED.into());
    }
    if mail.cod_paid {
        return Err(COD_PAID.into());
    }
    let sender_id = mail.sender_id.ok_or(SYSTEM_MAIL)?;
    let item = lock_escrow(&mut *conn, mail_id).await?;
    let players = lock_players(&mut *conn, &[player_id, sender_id]).await?;
    if !players.iter().any(|p| p.player_id == sender_id) {
        return Err(SYSTEM_MAIL.into());
    }
    let Some(returner) = players.iter().find(|p| p.player_id == player_id) else {
        return Err(NOT_FOUND.into());
    };
    let (cash, cod_cancelled) = if mail.cod() {
        (0, mail.cash)
    } else {
        (mail.cash, 0)
    };
    let moved = sqlx::query(
        "UPDATE sgw_gate_mail SET character_id = $3, sender_id = $2, sender_name = $4, \
                cash = $5, flags = flags & ~$6, returned = true, read_time = 0, sent_time = $7, \
                expires_at = $9 \
         WHERE mail_id = $1 AND character_id = $2 AND sender_id = $3 \
           AND NOT returned AND NOT cod_paid AND (flags & $8) = 0",
    )
    .bind(mail_id)
    .bind(player_id)
    .bind(sender_id)
    .bind(&returner.player_name)
    .bind(cash)
    .bind(MAIL_COD)
    .bind(now)
    .bind(MAIL_ARCHIVE)
    .bind(expires_at(now))
    .execute(&mut *conn)
    .await?
    .rows_affected();
    if moved != 1 {
        // Only reachable without the row lock: the other return won.
        return Err(ALREADY_RETURNED.into());
    }
    Ok(Returned {
        to_player_id: sender_id,
        to_player_name: mail.sender_name.clone(),
        cash,
        cod_cancelled,
        item_id: item.map(|i| i.item_id),
        item_type_id: item.map(|i| i.type_id),
    })
}

/// `returnMailMessage(MailId)`. The mail leaves the caller's list, and its
/// sender, if online, is told it arrived (D-SS11), after the commit.
pub(super) async fn return_mail(ctx: &MailCtx<'_>, mail_id: i32) {
    match return_tx(ctx.pool, ctx.player_id, mail_id, unix_now()).await {
        Ok(returned) => {
            let who = ctx.identity();
            let book = cimmeria_names::book();
            tracing::info!(
                target: "mail",
                event = "mail.returned",
                entity_id = ctx.entity_id,
                entity_name = who.player_name,
                player_id = ctx.player_id,
                player_name = who.player_name,
                account_id = ctx.account_id(),
                account_name = who.account_name,
                target_player_id = returned.to_player_id,
                target_player_name = returned.to_player_name.as_str(),
                mail_id, // nt:id-only mail row, its subject is player text kept out of logs
                cash = returned.cash,
                cod_cancelled = returned.cod_cancelled,
                item_id = returned.item_id,
                item_name = returned
                    .item_type_id
                    .and_then(|t| book.item(t)),
                "gate-mail returned to its sender with its attachments",
            );
            ctx.send_to_caller(
                method_idx::ON_MAIL_HEADER_REMOVE,
                &mail::serialize_on_mail_header_remove(mail_id),
            )
            .await;
            let fb = FeedbackCtx {
                transport: ctx.transport,
                connected: ctx.connected,
            };
            notify_delivered(
                ctx.pool,
                &fb,
                returned.to_player_id,
                mail_id,
                Delivery::Returned,
            )
            .await;
        }
        Err(err) => answer_failure(ctx, Op::Return, mail_id, err).await,
    }
}
