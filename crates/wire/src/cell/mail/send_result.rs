//! `sendMailResult` (client method 79, `SGWMailManager.def:43-47`).

use super::codes::MailResult;
use crate::mercury::write_wstring;

/// Wire order of `sendMailResult`'s two trailing arguments.
///
/// **Open question.** `SGWMailManager.def:43-47` declares `ResultCode`,
/// `FailedRecipients`, `FailedRecipientFlags`, and BigWorld streams a
/// method's arguments in `.def` order. SS-E1's M-Q1 note says the client
/// handler (`0x00e13b90`) reads `FailedRecipientFlags` before
/// `FailedRecipients`, which may be field access on an already-decoded
/// struct rather than the stream order. Until a Ghidra pass or a capture
/// settles it, this is the `.def` order; flipping it is this one constant.
pub const SEND_MAIL_RESULT_FLAGS_BEFORE_NAMES: bool = false;

/// Serialize `sendMailResult(UINT8 ResultCode, ARRAY<WSTRING>
/// FailedRecipients, INT32 FailedRecipientFlags)`; see
/// [`SEND_MAIL_RESULT_FLAGS_BEFORE_NAMES`] for the argument order.
///
/// The client shows `FailedRecipients` even on [`MailResult::Sent`]
/// ("Your message could not be delivered to the following recipients: …",
/// SS-E1 M-Q1), so a partial delivery must list every name that failed.
/// `FailedRecipientFlags` is decoded by the client but drives no text.
pub fn serialize_send_mail_result(
    result: MailResult,
    failed_recipients: &[String],
    failed_recipient_flags: i32,
) -> Vec<u8> {
    let mut args = Vec::with_capacity(
        1 + 4
            + failed_recipients
                .iter()
                .map(|n| 4 + n.len() * 2)
                .sum::<usize>()
            + 4,
    );
    args.push(result.code());
    if SEND_MAIL_RESULT_FLAGS_BEFORE_NAMES {
        args.extend_from_slice(&failed_recipient_flags.to_le_bytes());
    }
    args.extend_from_slice(&(failed_recipients.len() as u32).to_le_bytes());
    for name in failed_recipients {
        write_wstring(&mut args, name);
    }
    if !SEND_MAIL_RESULT_FLAGS_BEFORE_NAMES {
        args.extend_from_slice(&failed_recipient_flags.to_le_bytes());
    }
    args
}
