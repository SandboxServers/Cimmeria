//! The read side of gate mail: one body, archive and delete. The header
//! list is [`super::headers`].

use super::MailCtx;
use crate::base::feedback::{send_feedback_line, FeedbackCtx};
use crate::cell::mail;
use crate::cell::mail::codes::flags::{MAIL_ARCHIVE, MAIL_COD};
use crate::mercury::method_idx;

/// `requestMailBody(MailId)`: one body, marking the mail read on first open.
///
/// `ToText` is the name stored on the recipient's `sgw_player` row (audit
/// A-10, CAT-G-08), not whatever name the reader's session holds.
pub(super) async fn request_body(ctx: &MailCtx<'_>, mail_id: i32) {
    let (entity_id, player_id, account_id) = (ctx.entity_id, ctx.player_id, ctx.account_id());
    tracing::debug!(
        target: "mail",
        entity_id,
        player_id,
        account_id,
        mail_id,
        "Mail: querying body"
    );

    #[derive(sqlx::FromRow)]
    struct BodyRow {
        message: String,
        recipient_name: String,
    }

    let row = match sqlx::query_as::<_, BodyRow>(
        "SELECT m.message, p.player_name AS recipient_name \
         FROM sgw_gate_mail m JOIN sgw_player p ON p.player_id = m.character_id \
         WHERE m.mail_id = $1 AND m.character_id = $2",
    )
    .bind(mail_id)
    .bind(player_id)
    .fetch_optional(ctx.pool)
    .await
    {
        Ok(Some(row)) => row,
        // Distinguish "row missing for this character" (legitimate
        // permission boundary or stale client request) from "DB
        // error" (operator-actionable). Folding both into the
        // same warn string would hide connection failures /
        // schema mismatches behind a benign-looking message.
        Ok(None) => {
            tracing::warn!(
                target: "mail",
                entity_id,
                mail_id,
                player_id,
                account_id,
                reason = "not_found_for_owner",
                "Mail body not found for this character_id"
            );
            return;
        }
        Err(e) => {
            tracing::error!(
                target: "mail",
                entity_id,
                mail_id,
                player_id,
                account_id,
                reason = "db_error",
                error = %e,
                "Mail body query failed"
            );
            return;
        }
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i32;
    if let Err(e) = mark_read(ctx.pool, mail_id, player_id, now).await {
        tracing::warn!(
            target: "mail",
            entity_id,
            player_id,
            account_id,
            mail_id,
            reason = "db_error",
            error = %e,
            "Mail: read_time UPDATE failed"
        );
    }

    let args = mail::serialize_on_mail_read(mail_id, &row.message, &row.recipient_name);
    ctx.send_to_caller(method_idx::ON_MAIL_READ, &args).await;
}

/// Stamp the first read of `mail_id` by its owner `player_id`; returns the
/// rows changed (0 or 1).
///
/// Owner-scoped on its own (audit A-09, CAT-G-07), not only through the
/// owner-scoped SELECT that precedes it in [`request_body`]: a later caller
/// that skips the SELECT must still be unable to mark another character's
/// mail read. `AND read_time = 0` keeps the first-read time.
pub(super) async fn mark_read(
    pool: &sqlx::PgPool,
    mail_id: i32,
    player_id: i32,
    now: i32,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "UPDATE sgw_gate_mail SET read_time = $1 \
         WHERE mail_id = $2 AND character_id = $3 AND read_time = 0",
    )
    .bind(now)
    .bind(mail_id)
    .bind(player_id)
    .execute(pool)
    .await
    .map(|r| r.rows_affected())
}

/// Delete `mail_id` for its owner `player_id`, but only when it holds
/// nothing: no escrowed item, and no cash (gift cash, or an unpaid COD
/// price; paying a COD clears the amount, D-SS09). One statement, so a
/// take running at the same time either commits first (and the re-checked
/// row now qualifies) or keeps the mail. Returns the rows deleted (0 or 1).
///
/// Split out, like [`mark_read`], so the guard tests reach the SQL itself.
pub(super) async fn delete_if_empty(
    pool: &sqlx::PgPool,
    mail_id: i32,
    player_id: i32,
) -> Result<u64, sqlx::Error> {
    sqlx::query(
        "DELETE FROM sgw_gate_mail m \
         WHERE m.mail_id = $1 AND m.character_id = $2 AND m.cash = 0 \
           AND NOT EXISTS (SELECT 1 FROM sgw_gate_mail_item i WHERE i.mail_id = m.mail_id)",
    )
    .bind(mail_id)
    .bind(player_id)
    .execute(pool)
    .await
    .map(|r| r.rows_affected())
}

/// What an undeletable mail still holds.
#[derive(Debug, sqlx::FromRow)]
struct Held {
    cash: i64,
    flags: i32,
    has_item: bool,
}

/// `deleteMailMessage(MailId)`.
///
/// From SS-M2 on, a mail that still holds an item, gift cash or an unpaid
/// COD is not deleted: that would destroy the escrowed value (the escrow
/// row cascades with its mail). The row stays, no `onMailHeaderRemove` is
/// sent, and the player is told to take the attachment or return the mail
/// first. Mail with nothing attached deletes as before.
pub(super) async fn delete(ctx: &MailCtx<'_>, mail_id: i32) {
    let (entity_id, player_id, account_id) = (ctx.entity_id, ctx.player_id, ctx.account_id());
    tracing::debug!(
        target: "mail",
        entity_id,
        player_id,
        account_id,
        mail_id,
        "Mail: deleting"
    );
    match delete_if_empty(ctx.pool, mail_id, player_id).await {
        Ok(0) => {
            let held = sqlx::query_as::<_, Held>(
                "SELECT m.cash, m.flags, \
                        EXISTS (SELECT 1 FROM sgw_gate_mail_item i WHERE i.mail_id = m.mail_id) \
                            AS has_item \
                 FROM sgw_gate_mail m WHERE m.mail_id = $1 AND m.character_id = $2",
            )
            .bind(mail_id)
            .bind(player_id)
            .fetch_optional(ctx.pool)
            .await;
            match held {
                Ok(Some(held)) => return refuse_delete(ctx, mail_id, held).await,
                Ok(None) => {
                    tracing::warn!(
                        target: "mail",
                        entity_id,
                        player_id,
                        mail_id,
                        account_id,
                        reason = "not_found_for_owner",
                        "Mail: Delete affected 0 rows"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        target: "mail",
                        entity_id,
                        player_id,
                        account_id,
                        mail_id,
                        reason = "db_error",
                        error = %e,
                        "Mail: Delete follow-up query failed"
                    );
                    return;
                }
            }
        }
        Ok(_) => {
            tracing::debug!(
                target: "mail",
                event = "mail.deleted",
                entity_id,
                player_id,
                account_id,
                mail_id,
                "Mail: deleted"
            );
        }
        Err(e) => {
            tracing::error!(
                target: "mail",
                entity_id,
                player_id,
                account_id,
                mail_id,
                reason = "db_error",
                error = %e,
                "Mail: Delete failed"
            );
            return;
        }
    }

    let args = mail::serialize_on_mail_header_remove(mail_id);
    ctx.send_to_caller(method_idx::ON_MAIL_HEADER_REMOVE, &args)
        .await;
}

/// Refuse a delete of a mail that still holds something: log it, and tell
/// the player why on the first press.
async fn refuse_delete(ctx: &MailCtx<'_>, mail_id: i32, held: Held) {
    let cod = held.flags & MAIL_COD != 0;
    let (reason, text) = if held.has_item {
        (
            "attachment_item_present",
            "This gate-mail still holds an item. Take it or return the message \
             before deleting it.",
        )
    } else if cod {
        (
            "attachment_cod_unpaid",
            "This gate-mail is a COD delivery that has not been paid. Return it \
             instead of deleting it.",
        )
    } else {
        (
            "attachment_cash_present",
            "This gate-mail still holds naquadah. Take it before deleting the message.",
        )
    };
    tracing::warn!(
        target: "mail",
        event = "mail.delete_refused",
        entity_id = ctx.entity_id,
        player_id = ctx.player_id,
        account_id = ctx.account_id(),
        mail_id,
        reason,
        has_item = held.has_item,
        cash = held.cash,
        cod,
        "Mail: delete refused, the mail still holds an attachment"
    );
    if let Some(addr) = ctx.addr() {
        let fb = FeedbackCtx {
            transport: ctx.transport,
            connected: ctx.connected,
        };
        send_feedback_line(&fb, addr, text).await;
    }
}

/// `archiveMailMessage(MailId)`: sets `MAIL_Archive` and drops the row from
/// the open list.
pub(super) async fn archive(ctx: &MailCtx<'_>, mail_id: i32) {
    let (entity_id, player_id, account_id) = (ctx.entity_id, ctx.player_id, ctx.account_id());
    tracing::debug!(
        target: "mail",
        entity_id,
        player_id,
        account_id,
        mail_id,
        "Mail: archiving"
    );
    match sqlx::query(
        "UPDATE sgw_gate_mail SET flags = flags | $3 WHERE mail_id = $1 AND character_id = $2",
    )
    .bind(mail_id)
    .bind(player_id)
    .bind(MAIL_ARCHIVE)
    .execute(ctx.pool)
    .await
    {
        Ok(r) if r.rows_affected() == 0 => {
            tracing::warn!(
                target: "mail",
                entity_id,
                player_id,
                mail_id,
                account_id,
                reason = "not_found_for_owner",
                "Mail: Archive affected 0 rows"
            );
        }
        Ok(_) => {}
        Err(e) => {
            tracing::error!(
                target: "mail",
                entity_id,
                player_id,
                account_id,
                mail_id,
                reason = "db_error",
                error = %e,
                "Mail: Archive failed"
            );
            return;
        }
    }

    let args = mail::serialize_on_mail_header_remove(mail_id);
    ctx.send_to_caller(method_idx::ON_MAIL_HEADER_REMOVE, &args)
        .await;
}
