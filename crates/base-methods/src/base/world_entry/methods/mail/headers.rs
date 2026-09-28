//! `onMailHeaderInfo`: the caller's header list, the one-header refresh
//! the attachment ops (SS-M3) send after they change a mail, and the
//! one-header read the new-mail notification (SS-M4) pushes.
//!
//! Quarantined mail (SS-M4, D-SS04 path 3) is never read here: it is out of
//! the mailbox until a GM recovers it.

use sqlx::PgPool;

use super::claim::unix_now;
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
            "WHERE m.character_id = $1 AND NOT m.quarantined AND ",
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

/// Who a header read is for, for its logs.
#[derive(Debug, Clone, Copy)]
struct Reader<'a> {
    pool: &'a PgPool,
    player_id: i32,
    entity_id: Option<u32>,
    account_id: Option<u32>,
}

impl<'a> Reader<'a> {
    fn caller(ctx: &MailCtx<'a>) -> Self {
        Self {
            pool: ctx.pool,
            player_id: ctx.player_id,
            entity_id: Some(ctx.entity_id),
            account_id: ctx.account_id(),
        }
    }
}

/// Headers and their attachments, as the wire carries them.
pub(super) type Headers = (Vec<mail::MailHeader>, Vec<mail::MailAttachment>);

/// One of `player_id`'s mails as a header and its attachment, owner-scoped;
/// empty when it is not theirs, gone, or quarantined. For a push to a
/// player who did not ask (the new-mail notification).
pub(super) async fn read_one(
    pool: &PgPool,
    player_id: i32,
    mail_id: i32,
) -> Result<Headers, sqlx::Error> {
    let reader = Reader {
        pool,
        player_id,
        entity_id: None,
        account_id: None,
    };
    read_headers(reader, Select::One { mail_id }).await
}

/// The reader's headers and their attachments, owner-scoped.
async fn read_headers(reader: Reader<'_>, select: Select) -> Result<Headers, sqlx::Error> {
    let rows = match select {
        Select::List { b_archive } => {
            let archive_bit = if b_archive != 0 { MAIL_ARCHIVE } else { 0 };
            sqlx::query_as::<_, MailRow>(header_select!(
                "(m.flags & $2) = $3 ORDER BY m.mail_id DESC"
            ))
            .bind(reader.player_id)
            .bind(MAIL_ARCHIVE)
            .bind(archive_bit)
            .fetch_all(reader.pool)
            .await?
        }
        Select::One { mail_id } => {
            sqlx::query_as::<_, MailRow>(header_select!("m.mail_id = $2"))
                .bind(reader.player_id)
                .bind(mail_id)
                .fetch_all(reader.pool)
                .await?
        }
    };
    Ok(to_wire(reader, &rows))
}

fn to_wire(reader: Reader<'_>, rows: &[MailRow]) -> Headers {
    let now = unix_now();
    let headers = rows
        .iter()
        .map(|r| {
            let cash = i32::try_from(r.cash).unwrap_or_else(|_| {
                tracing::warn!(
                    target: "mail",
                    entity_id = reader.entity_id,
                    player_id = reader.player_id,
                    account_id = reader.account_id,
                    reason = "cash_out_of_i32_range",
                    mail_id = r.mail_id,
                    db_cash = r.cash,
                    "Mail header cash truncated to i32 range"
                );
                r.cash.clamp(i32::MIN as i64, i32::MAX as i64) as i32
            });
            if r.sent_time > now {
                tracing::warn!(
                    target: "mail",
                    entity_id = reader.entity_id,
                    player_id = reader.player_id,
                    account_id = reader.account_id,
                    reason = "sent_time_in_future",
                    mail_id = r.mail_id,
                    now,
                    db_sent_time = r.sent_time,
                    "Mail header sent_time is after now; clamping age to 0"
                );
            }
            mail::MailHeader {
                id: r.mail_id,
                from_text: r.sender_name.clone(),
                from_id: r.sender_id.unwrap_or(0),
                subject_text: r.subject.clone(),
                cash,
                sent_time: sent_time_age_secs(now, r.sent_time),
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

/// The wire `sentTime` field: seconds elapsed since `sent_time_unix`,
/// clamped to 0 (a future `sent_time` is a clock-skew bug the caller
/// should log, not a negative age).
///
/// **Not a Unix epoch value.** The client's mail-header constructor
/// (`FUN_00eb5ab0`, `ghidra://SGW.exe@0x00eb5ab0`) rounds this field to a
/// 64-bit integer (`Mercury__unknown_012379f6`, a misnamed shared
/// float-to-int64 rounding helper — 60+ unrelated call sites, not
/// `Mercury`-specific) and hands it to `FUN_00eb5a10`
/// (`ghidra://SGW.exe@0x00eb5a10`), which is, byte for byte:
///
/// ```text
/// GetTimeZoneInformation(&tz);
/// GetSystemTime(&utcNow);
/// SystemTimeToTzSpecificLocalTime(&tz, &utcNow, &localNow);
/// SystemTimeToFileTime(&localNow, &fileTimeNow);
/// fileTimeNow -= sentTimeField * 10_000_000;   // 100ns FILETIME ticks
/// FileTimeToSystemTime(&fileTimeNow, &out);    // -> "Sent: <date>"
/// ```
///
/// The identical rounded 64-bit value is also divided by 3600
/// (`__aulldiv`, `ghidra://SGW.exe@0x01237e00`) for
/// `ExpiresHours = 720 - hours` (`ghidra://SGW.exe@0x00eb5c12`..`0x00eb5c19`).
/// Sending the raw Unix epoch here (as the code did before 2026-09-28)
/// makes the client compute `localNow - epoch_seconds`, landing the
/// display near Unix epoch 0 ("Sent: Dec 31 1969") and driving
/// `ExpiresHours` deeply negative, which `GateMail.lua:138`
/// (`ExpiresHours < 2`) renders as "Soon" regardless of how fresh the
/// mail actually is.
pub(super) fn sent_time_age_secs(now: i32, sent_time_unix: i32) -> f32 {
    (now - sent_time_unix).max(0) as f32
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
///
/// The reply sets `ResetCategory` (SS-M4): it is the whole requested list,
/// so the client clears that list first and a mail deleted, returned,
/// expired or quarantined server-side since the last open drops out
/// instead of lingering until relog.
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

    let (headers, attachments) =
        match read_headers(Reader::caller(ctx), Select::List { b_archive }).await {
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

    let args = mail::serialize_on_mail_header_info(true, b_archive, &headers, &attachments);
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
    let (headers, attachments) =
        match read_headers(Reader::caller(ctx), Select::One { mail_id }).await {
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
    let args = mail::serialize_on_mail_header_info(false, b_archive, &headers, &attachments);
    ctx.send_to_caller(method_idx::ON_MAIL_HEADER_INFO, &args)
        .await;
}
