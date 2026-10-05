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
    /// The reader's names (Rule 6); `None` when no session is at hand.
    player_name: Option<&'static str>,
    account_name: Option<&'static str>,
}

impl<'a> Reader<'a> {
    fn caller(ctx: &MailCtx<'a>) -> Self {
        let who = ctx.identity();
        Self {
            pool: ctx.pool,
            player_id: ctx.player_id,
            entity_id: Some(ctx.entity_id),
            account_id: ctx.account_id(),
            player_name: who.player_name,
            account_name: who.account_name,
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
        player_name: None,
        account_name: None,
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
    Ok(to_wire(reader, &rows, unix_now()))
}

/// The wire `sentTime` of a mail written at `sent_time` (epoch seconds), as
/// of `now`: its **age** in seconds, not the epoch value the database holds.
///
/// The client's header-record constructor (`SGW.exe@0x00eb5ab0`) truncates
/// `sentTime` to an integer `age` and builds the "Sent" date as local now
/// minus `age` (`FUN_00eb5a10`), and `ExpiresHours` as `720 - age / 3600`.
/// Sending the epoch value made every mail read "Thu Jan 1st, 1970" and
/// expire "Soon". A clock step that puts `sent_time` in the future sends 0.
fn wire_sent_time(sent_time: i32, now: i32) -> f32 {
    (i64::from(now) - i64::from(sent_time)).max(0) as f32
}

fn to_wire(reader: Reader<'_>, rows: &[MailRow], now: i32) -> Headers {
    let headers = rows
        .iter()
        .map(|r| {
            let cash = i32::try_from(r.cash).unwrap_or_else(|_| {
                tracing::warn!(
                    target: "mail",
                    entity_id = reader.entity_id,
                    entity_name = reader.player_name,
                    player_id = reader.player_id,
                    player_name = reader.player_name,
                    account_id = reader.account_id,
                    account_name = reader.account_name,
                    reason = "cash_out_of_i32_range",
                    mail_id = r.mail_id, // nt:id-only mail row, its subject is player text kept out of logs
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
                sent_time: wire_sent_time(r.sent_time, now),
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
///
/// The reply sets `ResetCategory` (SS-M4): it is the whole requested list,
/// so the client clears that list first and a mail deleted, returned,
/// expired or quarantined server-side since the last open drops out
/// instead of lingering until relog.
pub(super) async fn request_headers(ctx: &MailCtx<'_>, b_archive: u8) {
    let (entity_id, player_id, account_id) = (ctx.entity_id, ctx.player_id, ctx.account_id());
    let who = ctx.identity();
    tracing::debug!(
        target: "mail",
        entity_id,
        entity_name = who.player_name,
        player_id,
        player_name = who.player_name,
        account_id,
        account_name = who.account_name,
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
                    entity_name = who.player_name,
                    player_id,
                    player_name = who.player_name,
                    account_id,
                    account_name = who.account_name,
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
        entity_name = who.player_name,
        player_id,
        player_name = who.player_name,
        account_id,
        account_name = who.account_name,
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
                let who = ctx.identity();
                tracing::error!(
                    target: "mail",
                    entity_id = ctx.entity_id,
                    entity_name = who.player_name,
                    player_id = ctx.player_id,
                    player_name = who.player_name,
                    account_id = ctx.account_id(),
                    account_name = who.account_name,
                    mail_id, // nt:id-only mail row, its subject is player text kept out of logs
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

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i32 = 1_790_000_000;

    fn row(sent_time: i32) -> MailRow {
        MailRow {
            mail_id: 7,
            sender_name: "Black Market".into(),
            sender_id: None,
            subject: "Auction Won".into(),
            cash: 0,
            sent_time,
            read_time: 0,
            flags: 0,
            att_type_id: None,
            att_stack_size: None,
            att_durability: None,
            att_charges: None,
        }
    }

    fn header_sent_time(sent_time: i32) -> f32 {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/none")
            .expect("connect_lazy accepts any well-formed URL");
        let reader = Reader {
            pool: &pool,
            player_id: 1,
            entity_id: None,
            account_id: None,
            player_name: None,
            account_name: None,
        };
        let (headers, _) = to_wire(reader, &[row(sent_time)], NOW);
        headers[0].sent_time
    }

    /// The client builds "Sent" as local now minus `sentTime` and
    /// `ExpiresHours` as `720 - sentTime / 3600` (`SGW.exe@0x00eb5ab0`), so
    /// the wire carries the age. The epoch value showed "Jan 1st, 1970" and
    /// "Soon" on the first live Black Market payout.
    #[tokio::test]
    async fn header_sent_time_is_the_mails_age() {
        assert_eq!(header_sent_time(NOW), 0.0, "sent just now");
        assert_eq!(header_sent_time(NOW - 90), 90.0, "sent 90 s ago");
        assert_eq!(
            header_sent_time(NOW - 3 * 86_400),
            259_200.0,
            "three days old: the client shows 27 days left"
        );
    }

    #[tokio::test]
    async fn header_sent_time_in_the_future_is_zero() {
        assert_eq!(header_sent_time(NOW + 30), 0.0, "clock stepped back");
    }
}
