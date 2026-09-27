//! `sendMailResult` (client method 79, `SGWMailManager.def:43-47`).

use super::codes::MailResult;
use crate::mercury::write_wstring;

/// Serialize `sendMailResult(UINT8 ResultCode, ARRAY<WSTRING>
/// FailedRecipients, INT32 FailedRecipientFlags)`, in `.def` order.
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
    args.extend_from_slice(&(failed_recipients.len() as u32).to_le_bytes());
    for name in failed_recipients {
        write_wstring(&mut args, name);
    }
    args.extend_from_slice(&failed_recipient_flags.to_le_bytes());
    args
}
