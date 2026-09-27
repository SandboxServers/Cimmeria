//! The read side of gate mail: the header list, one body, archive and delete.

use super::MailCtx;
use crate::cell::mail;
use crate::cell::mail::codes::flags::MAIL_ARCHIVE;
use crate::mercury::method_idx;

/// `requestMailHeaders(bArchive)`: the caller's inbox (`bArchive` 0) or
/// archive (any other value).
///
/// Only the requested list is sent (audit A-08). The client keeps two lists
/// and files each row by its own `MAIL_Archive` bit, but it clears only the
/// requested list on a reset, so rows of the other list sent here would sit
/// in it un-reset (SS-E1 M-Q7).
pub(super) async fn request_headers(ctx: &MailCtx<'_>, b_archive: u8) {
    let (entity_id, player_id, account_id) = (ctx.entity_id, ctx.player_id, ctx.account_id());
    tracing::debug!(
        target: "mail",
        entity_id,
        player_id,
        account_id,
        b_archive,
        "Mail: querying headers"
    );

    #[derive(sqlx::FromRow)]
    struct MailRow {
        mail_id: i32,
        sender_name: String,
        sender_id: Option<i32>,
        subject: String,
        cash: i64,
        sent_time: i32,
        read_time: i32,
        flags: i32,
    }

    let archive_bit = if b_archive != 0 { MAIL_ARCHIVE } else { 0 };
    let rows = match sqlx::query_as::<_, MailRow>(
        "SELECT mail_id, sender_name, sender_id, subject, cash, sent_time, read_time, flags \
         FROM sgw_gate_mail WHERE character_id = $1 AND (flags & $2) = $3 \
         ORDER BY mail_id DESC",
    )
    .bind(player_id)
    .bind(MAIL_ARCHIVE)
    .bind(archive_bit)
    .fetch_all(ctx.pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(
                target: "mail",
                entity_id,
                player_id,
                account_id,
                reason = "db_error",
                error = %e,
                "Mail: header query failed"
            );
            return;
        }
    };

    let headers: Vec<mail::MailHeader> = rows
        .iter()
        .map(|r| {
            let cash = i32::try_from(r.cash).unwrap_or_else(|_| {
                tracing::warn!(
                    target: "mail",
                    entity_id,
                    player_id,
                    account_id,
                    reason = "cash_out_of_i32_range",
                    mail_id = r.mail_id,
                    db_cash = r.cash,
                    "Mail header cash truncated to i32 range"
                );
                r.cash.clamp(i32::MIN as i64, i32::MAX as i64) as i32
            });
            mail::MailHeader {
                id: r.mail_id,
                from_text: r.sender_name.clone(),
                from_id: r.sender_id.unwrap_or(0),
                subject_text: r.subject.clone(),
                cash,
                sent_time: r.sent_time as f32,
                read_time: r.read_time as f32,
                flags: r.flags,
            }
        })
        .collect();

    tracing::debug!(
        target: "mail",
        event = "mail.headers_sent",
        entity_id,
        player_id,
        account_id,
        b_archive,
        count = headers.len(),
        "Mail: sending headers to client"
    );

    let args = mail::serialize_on_mail_header_info(b_archive, &headers);
    ctx.send_to_caller(method_idx::ON_MAIL_HEADER_INFO, &args)
        .await;
}

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

/// `deleteMailMessage(MailId)`.
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
    match sqlx::query("DELETE FROM sgw_gate_mail WHERE mail_id = $1 AND character_id = $2")
        .bind(mail_id)
        .bind(player_id)
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
                "Mail: Delete affected 0 rows"
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
                "Mail: Delete failed"
            );
            return;
        }
    }

    let args = mail::serialize_on_mail_header_remove(mail_id);
    ctx.send_to_caller(method_idx::ON_MAIL_HEADER_REMOVE, &args)
        .await;
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
