//! `sendMailMessage` (CM 44) on the base: text-only gate mail (SS-M1).
//!
//! The cell has already decoded the payload, bounded the recipient list and
//! applied the D-SS12 text rules ([`MailSend`]), or forwarded why it could
//! not ([`MailSendReject`]). The gates here run in this order, and every
//! refusal answers `sendMailResult` plus one feedback line, so the player
//! sees why on the first press:
//!
//! 1. the mail-send bucket (D-SS14), before anything else, refusals included;
//! 2. the cell's decode refusal, if there was one;
//! 3. alias bits in `RecipientFlags` ([`resolve_recipient_flags`], D-SS07);
//! 4. attachments: two or more recipients with one is
//!    `AttachmentsAndMultipleRecipients` (D-SS05); any attachment at all is
//!    refused until SS-M2;
//! 5. an empty recipient list;
//! 6. delivery in one transaction ([`deliver`]): D-SS13 name resolution,
//!    de-duplication, the Ignore seam (D-SS15), the 100-message cap under a
//!    row lock (D-SS03, D-SS06), one row per recipient.
//!
//! The sender's name and id come from server state: the cell's `player_id`,
//! and the name stored on that `sgw_player` row, read under the lock.

mod deliver;
pub(super) mod recipients;

use std::net::SocketAddr;
use std::time::Instant;

use cimmeria_entity::organization::{TextField, TextReject};
use sqlx::PgPool;

use super::Caller;
use crate::base::feedback::{send_feedback_line, FeedbackCtx};
use crate::base::rate_limit::{log_exceeded, RateActor, RateCategory, RateDecision};
use crate::cell::mail::codes::{flags, MailResult};
use crate::cell::mail::serialize_send_mail_result;
use crate::cell::messages::{MailSend, MailSendReject};
use crate::mercury::method_idx;

use deliver::{deliver, DeliverError};
use recipients::FailReason;

/// Why a send's `RecipientFlags` was refused (D-SS07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FlagRefusal {
    /// The bits echoed back as `FailedRecipientFlags`.
    pub(super) failed_flags: i32,
    /// Stable `reason` log value.
    pub(super) reason: &'static str,
    /// The feedback line.
    pub(super) text: &'static str,
}

/// The one seam for mail aliases. Every alias is refused until its owner
/// lands it: the vault bit belongs to the Bank campaign (which replaces this
/// function and owns `VaultButNoItem`, `VaultPlusCash` and `SentToVault`),
/// the Team and Command bits to the organizations campaign once ORG-07
/// exposes membership. A bit that is no alias at all is refused too.
pub(super) fn resolve_recipient_flags(recipient_flags: i32) -> Result<(), FlagRefusal> {
    if recipient_flags == 0 {
        return Ok(());
    }
    let unknown = recipient_flags & !(flags::VAULT_ALIASES | flags::ORGANIZATION_ALIASES);
    if unknown != 0 {
        return Err(FlagRefusal {
            failed_flags: recipient_flags,
            reason: "unknown_recipient_flags",
            text: "Your gate-mail message could not be read. It was not sent.",
        });
    }
    if recipient_flags & flags::VAULT_ALIASES != 0 {
        return Err(FlagRefusal {
            failed_flags: recipient_flags & flags::VAULT_ALIASES,
            reason: "vault_alias_unsupported",
            text: "Gate-mail to your vault is not available yet. The message was not sent.",
        });
    }
    Err(FlagRefusal {
        failed_flags: recipient_flags & flags::ORGANIZATION_ALIASES,
        reason: "organization_alias_unsupported",
        text: "Gate-mail to your team or command is not available yet. \
               The message was not sent.",
    })
}

/// The sender's session, read under the lock that took the send token.
#[derive(Debug, Clone, Copy)]
struct SenderSession {
    addr: SocketAddr,
    account_id: u32,
}

/// One refusal: the result code, what failed, and why.
struct Refusal<'a> {
    result: MailResult,
    failed_recipients: &'a [String],
    failed_flags: i32,
    reason: &'static str,
    text: String,
}

/// Handle one `sendMailMessage`, from the bucket to the reply.
pub(super) async fn send_mail(
    caller: &Caller<'_>,
    request: Result<MailSend, MailSendReject>,
    pool: Option<&PgPool>,
    now: Instant,
) {
    let typed: &[String] = match &request {
        Ok(send) => &send.recipients,
        Err(_) => &[],
    };
    let Some(session) = take_send_token(caller, typed, now).await else {
        return;
    };

    let send = match request {
        Ok(send) => send,
        Err(reject) => {
            let (reason, text) = (reject.reason(), decode_refusal_text(&reject));
            let refusal = Refusal {
                result: MailResult::NoRecipients,
                failed_recipients: &[],
                failed_flags: 0,
                reason,
                text: text.to_string(),
            };
            return refuse(caller, session, refusal).await;
        }
    };

    if let Err(f) = resolve_recipient_flags(send.recipient_flags) {
        let refusal = Refusal {
            result: MailResult::NoRecipients,
            failed_recipients: &send.recipients,
            failed_flags: f.failed_flags,
            reason: f.reason,
            text: f.text.to_string(),
        };
        return refuse(caller, session, refusal).await;
    }

    if send.has_attachment() {
        tracing::debug!(
            target: "mail",
            event = "mail.attachment_seen",
            entity_id = caller.entity_id,
            player_id = caller.player_id,
            account_id = session.account_id,
            cash = send.cash,
            cod = send.cod,
            item_id = send.item_id,
            item_quantity = send.item_quantity,
            recipients = send.recipients.len(),
            "sendMailMessage carries an attachment",
        );
        let refusal = if distinct_names(&send.recipients) > 1 {
            Refusal {
                result: MailResult::AttachmentsAndMultipleRecipients,
                failed_recipients: &send.recipients,
                failed_flags: 0,
                reason: "attachment_with_multiple_recipients",
                text: "Gate-mail with naquadah or an item can go to one recipient only. \
                       The message was not sent."
                    .to_string(),
            }
        } else {
            // TODO(SS-M2): cash, COD and item attachments with escrow.
            Refusal {
                result: MailResult::ItemNotAvailable,
                failed_recipients: &send.recipients,
                failed_flags: 0,
                reason: "attachment_not_supported",
                text: "Gate-mail attachments (naquadah, items and COD) are not available \
                       yet. Send the message without them."
                    .to_string(),
            }
        };
        return refuse(caller, session, refusal).await;
    }

    if send.recipients.is_empty() {
        let refusal = Refusal {
            result: MailResult::NoRecipients,
            failed_recipients: &[],
            failed_flags: 0,
            reason: "no_recipients",
            text: "Enter at least one recipient. The message was not sent.".to_string(),
        };
        return refuse(caller, session, refusal).await;
    }

    let Some(pool) = pool else {
        tracing::error!(
            target: "mail",
            entity_id = caller.entity_id,
            player_id = caller.player_id,
            account_id = session.account_id,
            reason = "no_db_pool",
            "sendMailMessage cannot be delivered: no database pool",
        );
        let refusal = Refusal {
            result: MailResult::NoRecipients,
            failed_recipients: &send.recipients,
            failed_flags: 0,
            reason: "no_db_pool",
            text: GATE_MAIL_UNAVAILABLE.to_string(),
        };
        return refuse(caller, session, refusal).await;
    };

    let sent_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i32;
    let delivery = match deliver(pool, caller.player_id, &send, sent_time).await {
        Ok(d) => d,
        Err(e) => {
            let reason = match &e {
                DeliverError::SenderMissing => "sender_missing",
                DeliverError::Db(_) => "db_error",
            };
            tracing::error!(
                target: "mail",
                entity_id = caller.entity_id,
                player_id = caller.player_id,
                account_id = session.account_id,
                reason,
                error = %e,
                "sendMailMessage delivery failed, transaction rolled back",
            );
            let refusal = Refusal {
                result: MailResult::NoRecipients,
                failed_recipients: &send.recipients,
                failed_flags: 0,
                reason,
                text: GATE_MAIL_UNAVAILABLE.to_string(),
            };
            return refuse(caller, session, refusal).await;
        }
    };

    let failed_names: Vec<String> = delivery.failed.iter().map(|f| f.typed.clone()).collect();
    for f in &delivery.failed {
        tracing::debug!(
            target: "mail",
            event = "mail.recipient_failed",
            entity_id = caller.entity_id,
            player_id = caller.player_id,
            account_id = session.account_id,
            target_player_id = f.player_id,
            reason = f.reason.reason(),
            "gate-mail recipient not delivered",
        );
    }
    let failure_text = failure_line(&delivery.failed);

    if delivery.delivered.is_empty() {
        let refusal = Refusal {
            result: MailResult::NoRecipients,
            failed_recipients: &failed_names,
            failed_flags: 0,
            reason: "no_deliverable_recipients",
            text: failure_text.unwrap_or_else(|| GATE_MAIL_UNAVAILABLE.to_string()),
        };
        return refuse(caller, session, refusal).await;
    }

    let mail_ids: Vec<i32> = delivery.delivered.iter().map(|d| d.mail_id).collect();
    let recipient_ids: Vec<i32> = delivery.delivered.iter().map(|d| d.player_id).collect();
    tracing::info!(
        target: "mail",
        event = "mail.sent",
        entity_id = caller.entity_id,
        player_id = caller.player_id,
        account_id = session.account_id,
        target_player_ids = ?recipient_ids,
        mail_ids = ?mail_ids,
        delivered = mail_ids.len(),
        failed = failed_names.len(),
        subject_units = send.subject.encode_utf16().count(),
        body_units = send.body.encode_utf16().count(),
        result = MailResult::Sent.token(),
        "gate mail sent",
    );
    let args = serialize_send_mail_result(MailResult::Sent, &failed_names, 0);
    caller
        .send_to_caller(method_idx::SEND_MAIL_RESULT, &args)
        .await;
    if let Some(text) = failure_text {
        feedback(caller, session, &text).await;
    }
}

const GATE_MAIL_UNAVAILABLE: &str = "Gate-mail is unavailable right now. The message was not sent.";

/// Take one mail-send token (D-SS14). `None` means stop: either the send
/// was limited, or the caller has no session any more.
///
/// A limited send still answers `sendMailResult` (`NoRecipients`, the typed
/// names in `FailedRecipients`) on every press. The client disables its
/// Send button when pressed and only a new compose re-enables it
/// (`GateMail.lua` `onSendMessage` / `onCreateNewMessage`), so the result
/// line ("Gate-mail message was not sent.") is the only thing that tells
/// the player that press did nothing. The explanatory feedback line is
/// throttled to once per 5 s like every other limited action. One result
/// per received packet is no amplification.
async fn take_send_token(
    caller: &Caller<'_>,
    typed: &[String],
    now: Instant,
) -> Option<SenderSession> {
    let Some(addr) = caller.addr() else {
        tracing::warn!(
            target: "mail",
            entity_id = caller.entity_id,
            player_id = caller.player_id,
            reason = "no_client_addr",
            "sendMailMessage dropped: the entity has no client address",
        );
        return None;
    };
    let (session, decision) = {
        let mut clients = match caller.connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let Some(c) = clients.get_mut(&addr) else {
            drop(clients);
            tracing::warn!(
                target: "mail",
                entity_id = caller.entity_id,
                player_id = caller.player_id,
                %addr,
                reason = "no_session",
                "sendMailMessage dropped: no session at the client address",
            );
            return None;
        };
        let decision = c.rate_limits.check(RateCategory::MailSend, now);
        if let RateDecision::Limited { notify } = decision {
            // Logged under the lock so the event carries the bucket state
            // the decision was made on.
            let actor = RateActor {
                addr,
                player_id: Some(caller.player_id),
                account_id: c.account_id,
                entity_id: Some(caller.entity_id),
            };
            log_exceeded(RateCategory::MailSend, actor, notify, &c.rate_limits, now);
        }
        (
            SenderSession {
                addr,
                account_id: c.account_id,
            },
            decision,
        )
    };
    match decision {
        RateDecision::Allowed => Some(session),
        RateDecision::Limited { notify } => {
            let args = serialize_send_mail_result(MailResult::NoRecipients, typed, 0);
            caller
                .send_to_caller(method_idx::SEND_MAIL_RESULT, &args)
                .await;
            if notify {
                feedback(caller, session, RateCategory::MailSend.feedback_text()).await;
            }
            None
        }
    }
}

/// Log the refusal, answer `sendMailResult`, and send its feedback line.
async fn refuse(caller: &Caller<'_>, session: SenderSession, refusal: Refusal<'_>) {
    tracing::warn!(
        target: "mail",
        event = "mail.send_refused",
        entity_id = caller.entity_id,
        player_id = caller.player_id,
        account_id = session.account_id,
        reason = refusal.reason,
        result = refusal.result.token(),
        failed_recipients = refusal.failed_recipients.len(),
        failed_flags = refusal.failed_flags,
        "sendMailMessage refused",
    );
    let args = serialize_send_mail_result(
        refusal.result,
        refusal.failed_recipients,
        refusal.failed_flags,
    );
    caller
        .send_to_caller(method_idx::SEND_MAIL_RESULT, &args)
        .await;
    feedback(caller, session, &refusal.text).await;
}

async fn feedback(caller: &Caller<'_>, session: SenderSession, text: &str) {
    let ctx = FeedbackCtx {
        transport: caller.transport,
        connected: caller.connected,
    };
    send_feedback_line(&ctx, session.addr, text).await;
}

/// Distinct names in a recipient list, compared case-insensitively like
/// D-SS13's fallback, so "Bob; bob" still counts as one recipient.
fn distinct_names(names: &[String]) -> usize {
    let mut folded: Vec<String> = names.iter().map(|n| n.to_lowercase()).collect();
    folded.sort();
    folded.dedup();
    folded.len()
}

/// The feedback line for a refusal the cell's decode produced.
fn decode_refusal_text(reject: &MailSendReject) -> &'static str {
    match reject {
        MailSendReject::TooManyRecipients { .. } => {
            "A gate-mail message can have at most 10 recipients. It was not sent."
        }
        MailSendReject::Malformed { .. } => {
            "Your gate-mail message could not be read. It was not sent."
        }
        MailSendReject::Text { field, reject } => match (field, reject) {
            (TextField::MailSubject, TextReject::TooShort { .. }) => {
                "Your gate-mail message needs a subject. It was not sent."
            }
            (TextField::MailSubject, TextReject::TooLong { .. }) => {
                "Your gate-mail subject is too long (128 characters at most). \
                 It was not sent."
            }
            (TextField::MailBody, TextReject::TooLong { .. }) => {
                "Your gate-mail message is too long (1,000 characters at most). \
                 It was not sent."
            }
            (TextField::MailRecipient, _) => {
                "A recipient name is not a valid character name. The message was not sent."
            }
            _ => {
                "Your gate-mail message contains a character that cannot be sent. \
                 It was not sent."
            }
        },
    }
}

/// One line naming every recipient that did not get the mail, and why.
fn failure_line(failed: &[recipients::FailedRecipient]) -> Option<String> {
    if failed.is_empty() {
        return None;
    }
    let parts: Vec<String> = failed
        .iter()
        .map(|f| format!("{} ({})", f.typed, f.reason.player_text()))
        .collect();
    Some(format!("Gate-mail not delivered to: {}.", parts.join(", ")))
}

impl FailReason {
    fn player_text(self) -> &'static str {
        match self {
            FailReason::Unknown => "no such character",
            FailReason::Ambiguous => "more than one character matches; check the capitals",
            FailReason::MailboxFull => "gate-mail box is full",
            FailReason::Ignoring => "not accepting your messages",
        }
    }
}

#[cfg(test)]
mod tests;
