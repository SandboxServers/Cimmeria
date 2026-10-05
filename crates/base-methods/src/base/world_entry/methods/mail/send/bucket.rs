//! The mail-send bucket (D-SS14): the first gate of every
//! `sendMailMessage`, before any SQL runs.

use std::time::Instant;

use super::super::Caller;
use super::{feedback, SenderSession};
use crate::base::rate_limit::{log_exceeded, RateActor, RateCategory, RateDecision};
use crate::base::session_identity::session_identity;
use crate::cell::mail::codes::MailResult;
use crate::cell::mail::serialize_send_mail_result;
use crate::mercury::method_idx;

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
pub(super) async fn take_send_token(
    caller: &Caller<'_>,
    typed: &[String],
    now: Instant,
) -> Option<SenderSession> {
    let Some(addr) = caller.addr() else {
        let who = caller.identity();
        tracing::warn!(
            target: "mail",
            entity_id = caller.entity_id,
            entity_name = who.player_name,
            player_id = caller.player_id,
            player_name = who.player_name,
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
            let who = caller.identity();
            tracing::warn!(
                target: "mail",
                entity_id = caller.entity_id,
                entity_name = who.player_name,
                player_id = caller.player_id,
                player_name = who.player_name,
                %addr,
                reason = "no_session",
                "sendMailMessage dropped: no session at the client address",
            );
            return None;
        };
        let who = session_identity(c);
        let decision = c.rate_limits.check(RateCategory::MailSend, now);
        if let RateDecision::Limited { notify } = decision {
            // Logged under the lock so the event carries the bucket state
            // the decision was made on.
            let actor = RateActor {
                player_id: Some(caller.player_id),
                entity_id: Some(caller.entity_id),
                ..RateActor::of(addr, c)
            };
            log_exceeded(RateCategory::MailSend, actor, notify, &c.rate_limits, now);
        }
        (
            SenderSession {
                addr,
                account_id: c.account_id,
                player_name: who.player_name,
                account_name: who.account_name,
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
