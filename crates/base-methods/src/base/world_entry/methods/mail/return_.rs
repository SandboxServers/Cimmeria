//! `returnMailMessage` (CM 47): send a mail back to the player who sent it,
//! with its item and any gift cash (D-SS10, CAT-G-06). SS-M4's expiry
//! sweep reuses [`return_tx`]. Lock order and failure handling:
//! [`super::claim`].

use sqlx::PgPool;

use super::claim::{
    answer_failure, lock_escrow, lock_mail, lock_players, unix_now, Op, OpError, Refusal, NOT_FOUND,
};
use super::MailCtx;
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
const SYSTEM_MAIL: Refusal = Refusal {
    reason: "system_mail",
    text: "That gate-mail message has no player sender to return it to.",
};

/// A committed return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Returned {
    /// The stored `sender_id`, who now owns the mail.
    pub(super) to_player_id: i32,
    /// Gift cash that went back with it.
    pub(super) cash: i64,
    /// The COD price cleared by the return (0 when it was no COD).
    pub(super) cod_cancelled: i64,
    /// The escrowed instance that went back with it, if any.
    pub(super) item_id: Option<i32>,
}

/// CAT-G-06 / D-SS10: return `mail_id`, owned by `player_id`, to its stored
/// sender, once.
///
/// Refused for archived mail, mail already returned, and mail with no
/// `sender_id` (server mail, or a sender whose character was deleted: the
/// foreign key sets it NULL). The destination is the stored `sender_id`,
/// never `sender_name`, so a mail cannot be redirected by a name. One
/// conditional `UPDATE` re-addresses the row: the returner becomes its
/// sender, `returned` is set so it can never loop, an unpaid COD is
/// cancelled with its amount **zeroed** (the price never becomes gift
/// cash), and it arrives unread with a fresh `sent_time`. The escrow row is
/// keyed by `mail_id`, so the item moves with the mail in the same
/// transaction. Server mail: exempt from the recipient's mailbox cap.
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
    if mail.archived() {
        return Err(ARCHIVED.into());
    }
    if mail.returned {
        return Err(ALREADY_RETURNED.into());
    }
    let sender_id = mail.sender_id.ok_or(SYSTEM_MAIL)?;
    let item = lock_escrow(&mut tx, mail_id).await?;
    let players = lock_players(&mut tx, &[player_id, sender_id]).await?;
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
                cash = $5, flags = flags & ~$6, returned = true, read_time = 0, sent_time = $7 \
         WHERE mail_id = $1 AND character_id = $2 AND sender_id = $3 \
           AND NOT returned AND (flags & $8) = 0",
    )
    .bind(mail_id)
    .bind(player_id)
    .bind(sender_id)
    .bind(&returner.player_name)
    .bind(cash)
    .bind(MAIL_COD)
    .bind(now)
    .bind(MAIL_ARCHIVE)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if moved != 1 {
        // Only reachable without the row lock: the other return won.
        return Err(ALREADY_RETURNED.into());
    }
    tx.commit().await?;
    Ok(Returned {
        to_player_id: sender_id,
        cash,
        cod_cancelled,
        item_id: item.map(|i| i.item_id),
    })
}

/// `returnMailMessage(MailId)`. The mail leaves the caller's list.
pub(super) async fn return_mail(ctx: &MailCtx<'_>, mail_id: i32) {
    match return_tx(ctx.pool, ctx.player_id, mail_id, unix_now()).await {
        Ok(returned) => {
            tracing::info!(
                target: "mail",
                event = "mail.returned",
                entity_id = ctx.entity_id,
                player_id = ctx.player_id,
                account_id = ctx.account_id(),
                target_player_id = returned.to_player_id,
                mail_id,
                cash = returned.cash,
                cod_cancelled = returned.cod_cancelled,
                item_id = returned.item_id,
                "gate-mail returned to its sender with its attachments",
            );
            ctx.send_to_caller(
                method_idx::ON_MAIL_HEADER_REMOVE,
                &mail::serialize_on_mail_header_remove(mail_id),
            )
            .await;
        }
        Err(err) => answer_failure(ctx, Op::Return, mail_id, err).await,
    }
}
