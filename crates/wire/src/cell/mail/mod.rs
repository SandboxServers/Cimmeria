//! Mail payloads: the `onMailHeaderInfo`, `onMailRead`,
//! `onMailHeaderRemove` and `sendMailResult` serializers, the [`MailHeader`]
//! they carry, and the `EMailFlags` / `EMailResultCodes` values.
//!
//! The base builds these from its mail queries. The cell's mail handlers,
//! which forward requests to the base, are in
//! `cimmeria_cell_interactions::cell::mail`, which re-exports everything here.
//!
//! Reference: `python/cell/SGWPlayer.py:requestMailHeaders()`, `requestMailBody()`

use crate::mercury::write_wstring;

pub mod codes;
mod send_result;

pub use send_result::{serialize_send_mail_result, SEND_MAIL_RESULT_FLAGS_BEFORE_NAMES};

// ── Wire format helpers for BaseApp to build mail response packets ───────────

/// Serialize `onMailHeaderInfo` args.
///
/// `reset_category` is the client's "clear this list first" flag. SS-E1
/// M-Q7 (`SGW.exe@0x00e15450`): when it is set, the client clears only the
/// list named by `bArchive` (inbox or archive), then upserts every row by
/// id into the list its own `MAIL_Archive` bit names. Set it on a full list
/// reply, so mail deleted, returned or expired server-side drops out of the
/// client's list; leave it clear on a one-header upsert (a refresh or a
/// new-mail notification), which must not wipe the rest of the list.
///
/// Wire format:
/// - ResetCategory: UINT8 (1 = clear the `bArchive` list first)
/// - bArchive: UINT8
/// - MessageHeaders: ARRAY of MessageHeader FIXED_DICT
///   - count: u32 LE
///   - per header: id(i32), fromText(WSTRING), fromId(i32), subjectText(WSTRING),
///     subjectId(i32), cash(i32), sentTime(f32, **seconds-ago, not epoch** —
///     see [`MailHeader::sent_time`]), readTime(f32, epoch seconds), flags(i32)
/// - MessageAttachments: ARRAY of [`MailAttachment`] FIXED_DICT
///   (`alias.xml:103-111`), one per header that holds an escrowed item
///   - count: u32 LE
///   - per attachment: id(i32, the mail id the client joins on), itemId(i32),
///     stackSize(i32), durability(i32), charges(i32)
pub fn serialize_on_mail_header_info(
    reset_category: bool,
    b_archive: u8,
    headers: &[MailHeader],
    attachments: &[MailAttachment],
) -> Vec<u8> {
    let mut args = Vec::with_capacity(2 + 4 + headers.len() * 64 + 4 + attachments.len() * 20);

    // ResetCategory
    args.push(u8::from(reset_category));
    // bArchive
    args.push(b_archive);

    // MessageHeaders array
    args.extend_from_slice(&(headers.len() as u32).to_le_bytes());
    for h in headers {
        // id: INT32
        args.extend_from_slice(&h.id.to_le_bytes());
        // fromText: WSTRING
        write_wstring(&mut args, &h.from_text);
        // fromId: INT32
        args.extend_from_slice(&h.from_id.to_le_bytes());
        // subjectText: WSTRING
        write_wstring(&mut args, &h.subject_text);
        // subjectId: INT32 (always 0)
        args.extend_from_slice(&0i32.to_le_bytes());
        // cash: INT32
        args.extend_from_slice(&h.cash.to_le_bytes());
        // sentTime: FLOAT
        args.extend_from_slice(&h.sent_time.to_le_bytes());
        // readTime: FLOAT
        args.extend_from_slice(&h.read_time.to_le_bytes());
        // flags: INT32
        args.extend_from_slice(&h.flags.to_le_bytes());
    }

    // MessageAttachments
    args.extend_from_slice(&(attachments.len() as u32).to_le_bytes());
    for a in attachments {
        a.serialize(&mut args);
    }

    args
}

/// Serialize `onMailRead` args.
///
/// Wire format:
/// - MailId: INT32
/// - BodyText: WSTRING
/// - BodyId: INT32 (always 0)
/// - ToText: WSTRING (recipient name)
pub fn serialize_on_mail_read(mail_id: i32, body_text: &str, recipient_name: &str) -> Vec<u8> {
    let mut args =
        Vec::with_capacity(4 + 4 + body_text.len() * 2 + 8 + recipient_name.len() * 2 + 8);

    // MailId: INT32
    args.extend_from_slice(&mail_id.to_le_bytes());
    // BodyText: WSTRING
    write_wstring(&mut args, body_text);
    // BodyId: INT32 (always 0)
    args.extend_from_slice(&0i32.to_le_bytes());
    // ToText: WSTRING
    write_wstring(&mut args, recipient_name);

    args
}

/// Serialize `onMailHeaderRemove` args.
///
/// Wire format:
/// - MailId: INT32
pub fn serialize_on_mail_header_remove(mail_id: i32) -> Vec<u8> {
    mail_id.to_le_bytes().to_vec()
}

/// Mail header data from the database.
#[derive(Debug, Clone)]
pub struct MailHeader {
    pub id: i32,
    pub from_text: String,
    pub from_id: i32,
    pub subject_text: String,
    pub cash: i32,
    /// **Seconds elapsed since the mail was sent (an age), not a Unix
    /// epoch timestamp.** `FUN_00eb5ab0` (the client's header-record
    /// constructor, `ghidra://SGW.exe@0x00eb5ab0`) rounds this field to a
    /// 64-bit integer and feeds it straight to `FUN_00eb5a10`
    /// (`ghidra://SGW.exe@0x00eb5a10`), which computes
    /// `GetSystemTime()` (converted to local time) **minus**
    /// `this_field * 10_000_000` (100ns FILETIME ticks) to get the
    /// SYSTEMTIME it shows as "Sent: <date>", and divides the same
    /// rounded value by 3600 for `ExpiresHours = 720 - hours`. Sending
    /// the raw Unix epoch here (as the pre-2026-09-28 code did) makes the
    /// client compute `now - epoch_seconds`, which lands near Unix epoch
    /// 0 (displayed "Dec 31 1969") and drives `ExpiresHours` deeply
    /// negative (displayed "Soon" — `GateMail.lua:138`,
    /// `ExpiresHours < 2`). The caller must compute
    /// `(unix_now() - sent_time_unix).max(0)` at serialize time, not
    /// store this value.
    pub sent_time: f32,
    /// Unix epoch seconds, unlike `sent_time` above. `FUN_00eb5ab0` only
    /// ever compares this field against a small float threshold
    /// (`ghidra://SGW.exe@0x00eb5bef`, `COMISS`/`JC`) to set the
    /// client's `HasBeenRead` flag; it never reaches the date/age
    /// arithmetic that makes `sent_time`'s units matter, so the raw
    /// epoch value (0 = unread, non-zero = read) is correct as-is.
    pub read_time: f32,
    pub flags: i32,
}

/// `MessageAttachment` (`alias.xml:103-111`): the item a mail holds in
/// escrow, joined to its header by `id` (SS-E1 M-Q4).
///
/// `item_id` is the item's **type** (design) id, the value `InvItem.dbid`
/// carries, not the escrowed instance id. The recipient's client builds the
/// attachment's name, icon, tech comp and quality from this one number
/// (`GateMail.lua` `mailGetItemAttachmentInfo` → `.Name`, `.Icon`,
/// `.TechComp`, `.Quality`), and it has no inventory record for an instance
/// it does not own, so only a type id can resolve. The instance id stays on
/// the server (`sgw_gate_mail_item.item_id`); the take paths key on the
/// mail id and never read this field back.
///
/// `durability` is `INT32` per `alias.xml`, although the client's UI decode
/// reads it into a float (SS-E1 M-Q4); both are 4 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailAttachment {
    pub id: i32,
    pub item_id: i32,
    pub stack_size: i32,
    pub durability: i32,
    pub charges: i32,
}

impl MailAttachment {
    /// Append the 20-byte FIXED_DICT, fields in `alias.xml` order.
    pub fn serialize(&self, out: &mut Vec<u8>) {
        for v in [
            self.id,
            self.item_id,
            self.stack_size,
            self.durability,
            self.charges,
        ] {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests;
