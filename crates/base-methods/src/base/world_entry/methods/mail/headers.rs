//! `onMailHeaderInfo`: the caller's header list, and the one-header refresh
//! the attachment ops (SS-M3) send after they change a mail.

use super::MailCtx;
use crate::cell::mail;
use crate::cell::mail::codes::flags::MAIL_ARCHIVE;
use crate::mercury::method_idx;

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
    att_type_id: Option<i32>,
    att_stack_size: Option<i32>,
    att_durability: Option<i32>,
    att_charges: Option<i32>,
}

/// The owner-scoped header SELECT, closed by `$tail` (a compile-time
/// literal, as sqlx requires).
macro_rules! header_select {
    ($tail:literal) => {
        concat!(
            "SELECT m.mail_id, m.sender_name, m.sender_id, m.subject, m.cash, ",
            "m.sent_time, m.read_time, m.flags, i.type_id AS att_type_id, ",
            "i.stack_size AS att_stack_size, i.durability AS att_durability, ",
            "i.charges AS att_charges ",
            "FROM sgw_gate_mail m LEFT JOIN sgw_gate_mail_item i ON i.mail_id = m.mail_id ",
            "WHERE m.character_id = $1 AND ",
            $tail
        )
    };
}

/// Which headers to read: one list (`bArchive`), or one mail by id.
#[derive(Debug, Clone, Copy)]
enum Select {
    List { b_archive: u8 },
    One { mail_id: i32 },
}

/// The caller's headers and their attachments, owner-scoped.
async fn read_headers(
    ctx: &MailCtx<'_>,
    select: Select,
) -> Result<(Vec<mail::MailHeader>, Vec<mail::MailAttachment>), sqlx::Error> {
    let rows = match select {
        Select::List { b_archive } => {
            let archive_bit = if b_archive != 0 { MAIL_ARCHIVE } else { 0 };
            sqlx::query_as::<_, MailRow>(header_select!(
                "(m.flags & $2) = $3 ORDER BY m.mail_id DESC"
            ))
            .bind(ctx.player_id)
            .bind(MAIL_ARCHIVE)
            .bind(archive_bit)
            .fetch_all(ctx.pool)
            .await?
        }
        Select::One { mail_id } => {
            sqlx::query_as::<_, MailRow>(header_select!("m.mail_id = $2"))
                .bind(ctx.player_id)
                .bind(mail_id)
                .fetch_all(ctx.pool)
                .await?
        }
    };
    Ok(to_wire(ctx, &rows))
}

fn to_wire(
    ctx: &MailCtx<'_>,
    rows: &[MailRow],
) -> (Vec<mail::MailHeader>, Vec<mail::MailAttachment>) {
    let headers = rows
        .iter()
        .map(|r| {
            let cash = i32::try_from(r.cash).unwrap_or_else(|_| {
                tracing::warn!(
                    target: "mail",
                    entity_id = ctx.entity_id,
                    player_id = ctx.player_id,
                    account_id = ctx.account_id(),
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

    // `itemId` is the type id: the recipient's client resolves the name and
    // icon from it (see `MailAttachment`).
    let attachments = rows
        .iter()
        .filter_map(|r| {
            Some(mail::MailAttachment {
                id: r.mail_id,
                item_id: r.att_type_id?,
                stack_size: r.att_stack_size?,
                durability: r.att_durability?,
                charges: r.att_charges?,
            })
        })
        .collect();
    (headers, attachments)
}

/// `requestMailHeaders(bArchive)`: the caller's inbox (`bArchive` 0) or
/// archive (any other value), with one `MessageAttachment` per mail that
/// holds an escrowed item (SS-M2). Cash and COD ride on the header itself
/// (`cash`, and `MAIL_COD` in `flags`).
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

    let (headers, attachments) = match read_headers(ctx, Select::List { b_archive }).await {
        Ok(read) => read,
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

    tracing::debug!(
        target: "mail",
        event = "mail.headers_sent",
        entity_id,
        player_id,
        account_id,
        b_archive,
        count = headers.len(),
        attachments = attachments.len(),
        "Mail: sending headers to client"
    );

    let args = mail::serialize_on_mail_header_info(b_archive, &headers, &attachments);
    ctx.send_to_caller(method_idx::ON_MAIL_HEADER_INFO, &args)
        .await;
}

/// Re-send one of the caller's headers after an attachment op changed it:
/// `onMailHeaderRemove`, then `onMailHeaderInfo` with the row as it now
/// stands (no reset). A mail that is no longer the caller's (it was
/// returned) gets the remove only.
///
/// The remove comes first because the client upserts a header by id
/// (SS-E1 M-Q7) and writes the attachment fields only when an attachment
/// row arrives; an upsert alone could leave a taken item's name and icon
/// on the old record. Removing and re-adding builds a fresh record.
pub(super) async fn refresh_one(ctx: &MailCtx<'_>, mail_id: i32) {
    ctx.send_to_caller(
        method_idx::ON_MAIL_HEADER_REMOVE,
        &mail::serialize_on_mail_header_remove(mail_id),
    )
    .await;
    let (headers, attachments) = match read_headers(ctx, Select::One { mail_id }).await {
        Ok(read) => read,
        Err(e) => {
            tracing::error!(
                target: "mail",
                entity_id = ctx.entity_id,
                player_id = ctx.player_id,
                account_id = ctx.account_id(),
                mail_id,
                reason = "db_error",
                error = %e,
                "Mail: header refresh query failed"
            );
            return;
        }
    };
    let Some(header) = headers.first() else {
        return;
    };
    let b_archive = u8::from(header.flags & MAIL_ARCHIVE != 0);
    let args = mail::serialize_on_mail_header_info(b_archive, &headers, &attachments);
    ctx.send_to_caller(method_idx::ON_MAIL_HEADER_INFO, &args)
        .await;
}
