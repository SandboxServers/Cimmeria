//! The `sendMailMessage` (CM 44) decoder: a bounded read of the `.def`
//! arguments, then the D-SS12 text rules, before anything reaches the base.

use cimmeria_entity::organization::org_text::validate;
use cimmeria_entity::organization::{TextField, TextReject};

use super::super::organization::{ArgReader, OrgDecodeError};
use crate::cell::messages::{MailSend, MailSendReject};

/// Most recipient names one send may carry (D-SS05). Project policy: the
/// shipped client allows 51 tokens (SS-E1 M-Q2), so this server cap is the
/// stricter of the two. The declared array count is checked against it
/// before a single name is read.
pub const MAX_MAIL_RECIPIENTS: u32 = 10;

const RECIPIENTS: &str = "Recipients";
const SUBJECT: &str = "Subject";
const BODY: &str = "Body";

/// Decode `sendMailMessage(INT32 RecipientFlags, ARRAY<WSTRING> Recipients,
/// WSTRING Subject, WSTRING Body, INT32 Cash, UINT8 bCOD, INT32 ItemId,
/// INT32 ItemQuantity)` (`SGWMailManager.def:56-66`).
///
/// Every read is bounded by the bytes left, so a forged `WSTRING` length
/// costs nothing, and a declared recipient count above
/// [`MAX_MAIL_RECIPIENTS`] is refused before any name is allocated. A
/// payload that does not decode is `Malformed`; one that decodes but breaks
/// a text rule is `Text` with the field that broke it.
pub fn decode_send_mail_message(args: &[u8]) -> Result<MailSend, MailSendReject> {
    let mut r = ArgReader::new(args);
    let recipient_flags = r.i32("RecipientFlags").map_err(structural)?;
    // The ARRAY count is a u32 on the wire; the reader has no u32, and the
    // bit pattern is the same.
    let declared = r.i32(RECIPIENTS).map_err(structural)? as u32;
    if declared > MAX_MAIL_RECIPIENTS {
        return Err(MailSendReject::TooManyRecipients { declared });
    }
    let mut recipients = Vec::with_capacity(declared as usize);
    for _ in 0..declared {
        recipients.push(r.wstring(RECIPIENTS).map_err(structural)?);
    }
    let subject = r.wstring(SUBJECT).map_err(structural)?;
    let body = r.wstring(BODY).map_err(structural)?;
    let cash = r.i32("Cash").map_err(structural)?;
    let cod = r.u8("bCOD").map_err(structural)? != 0;
    let item_id = r.i32("ItemId").map_err(structural)?;
    let item_quantity = r.i32("ItemQuantity").map_err(structural)?;
    r.finish().map_err(structural)?;

    // Text rules only once the whole payload decoded, so a truncated packet
    // is reported as malformed rather than as whichever string it cut.
    for name in &recipients {
        check(TextField::MailRecipient, name)?;
    }
    check(TextField::MailSubject, &subject)?;
    check(TextField::MailBody, &body)?;

    Ok(MailSend {
        recipient_flags,
        recipients,
        subject,
        body,
        cash,
        cod,
        item_id,
        item_quantity,
    })
}

fn check(field: TextField, text: &str) -> Result<(), MailSendReject> {
    validate(field, text)
        .map(|_| ())
        .map_err(|reject| MailSendReject::Text { field, reject })
}

/// Map a reader error. An unpaired surrogate is a text-rule refusal
/// (D-ORG10) on the field it was found in; everything else is a malformed
/// payload.
fn structural(e: OrgDecodeError) -> MailSendReject {
    match e {
        OrgDecodeError::LoneSurrogate { field } => MailSendReject::Text {
            field: match field {
                RECIPIENTS => TextField::MailRecipient,
                SUBJECT => TextField::MailSubject,
                _ => TextField::MailBody,
            },
            reject: TextReject::LoneSurrogate,
        },
        other => MailSendReject::Malformed {
            reason: other.reason(),
        },
    }
}
