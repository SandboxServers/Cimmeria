//! Tells: `sendPlayerCommunication` on the tell channel, handled on the base
//! and never forwarded to the cell (SS-C1).
//!
//! The legacy server did the same (`python/base/Chat.py::sendPlayerMessage`,
//! lines 339-358): the recipient gets `onPlayerCommunication(speaker, flags,
//! tell, text)` and the sender `onTellSent(target, text)`. Every refusal is a
//! feedback line to the sender, never a silent drop:
//!
//! - no target, or the sender's own name;
//! - a name no online character has ("Player X is not online.", the legacy
//!   `Chat.py:351-354` shape), or one that matches two (D-SS13);
//! - a recipient whose Ignore list holds the sender (D-SS15): "X is not
//!   accepting your messages.", and nothing reaches the recipient.
//!
//! A recipient in AFK or DND still gets the tell; their away message goes
//! back to the sender on the tell channel, spoken by the recipient.
//!
//! The rate limit (D-SS14), the channel allowlist and the GM mute (SS-C3,
//! D-SS26) and the text rules (D-SS12) have already run in
//! `chat::send_player_communication_at` before this is reached.

use std::net::SocketAddr;

use cimmeria_wire::cell::chat::{serialize_on_player_communication, serialize_on_tell_sent};
use cimmeria_wire::cell::client_methods::communicator::{ON_PLAYER_COMMUNICATION, ON_TELL_SENT};

use super::super::contact_list::ignore::{not_accepting_text, session_ignores};
use super::super::feedback::{
    send_feedback_line, send_to_current_player, FeedbackCtx, FeedbackOutcome,
};
use super::super::player_index::{NameLookup, OnlinePlayerIndex};
use super::speaker_flags;
use cimmeria_entity::organization::org_text::{validate, TextField};

/// The tell channel byte the client sends and renders. `EChannel` tell is 10
/// in `enumerations.xml`, and the client hardcodes the same literal (ORG-E1
/// Q5, D-ORG14; cited by SS-E1 C-Q1). The workspace `CHAN_TELL` constants
/// still say 9 until ORG-09 aligns them (D-SS17), so this is a local
/// constant, not a second edit of theirs.
pub(super) const TELL_CHANNEL: u8 = 10;

/// Feedback for a tell with no target name.
pub(super) const TELL_NO_TARGET_TEXT: &str = "Who do you want to send a tell to?";
/// Feedback for a tell addressed to the sender.
pub(super) const TELL_SELF_TEXT: &str = "You cannot send a tell to yourself.";
/// Feedback for a target that is not a possible character name.
pub(super) const TELL_BAD_NAME_TEXT: &str = "That is not a valid character name.";

/// Longest prefix of a typed target name echoed back in feedback or logged.
/// Character names are far shorter; this bounds a hostile one.
const SHOWN_NAME_CHARS: usize = 64;

/// "Player X is not online." (legacy `Chat.py:351-354`).
pub(super) fn not_online_text(target: &str) -> String {
    format!("Player {} is not online.", shown(target))
}

/// D-SS13: two online characters match after case folding.
pub(super) fn ambiguous_text(target: &str) -> String {
    format!(
        "More than one player is named {}. Type the exact name.",
        shown(target)
    )
}

fn shown(name: &str) -> String {
    name.chars().take(SHOWN_NAME_CHARS).collect()
}

/// The sender, from server session state.
#[derive(Debug, Clone, Copy)]
pub(super) struct TellSender<'a> {
    pub addr: SocketAddr,
    pub name: &'a str,
    pub flags: u8,
    pub entity_id: Option<u32>,
    pub player_id: Option<i32>,
    pub account_id: u32,
}

/// The resolved recipient, read under one lock. Deliberately no entity id:
/// gate travel can replace it before the send, so the send reads it from the
/// session at that moment (`send_to_current_player`).
struct Recipient {
    addr: SocketAddr,
    name: String,
    player_id: i32,
    account_id: u32,
    ignores_sender: bool,
    flags: u8,
    away_message: Option<String>,
}

enum Resolution {
    Found(Recipient),
    Myself,
    Refused(NameLookup),
}

fn resolve(ctx: &FeedbackCtx<'_>, sender: &TellSender<'_>, target: &str) -> Resolution {
    let clients = ctx.connected.lock().unwrap();
    let found = match OnlinePlayerIndex::new(&clients).lookup(target) {
        NameLookup::Found(p) => p,
        other => return Resolution::Refused(other),
    };
    if found.addr == sender.addr {
        return Resolution::Myself;
    }
    let Some(c) = clients.get(&found.addr) else {
        return Resolution::Refused(NameLookup::NotFound);
    };
    let mut flags = 0u8;
    if c.access_level > 0 {
        flags |= speaker_flags::GM;
    }
    if c.dnd_message.is_some() {
        flags |= speaker_flags::DND;
    }
    Resolution::Found(Recipient {
        addr: found.addr,
        name: c.player_name.clone().unwrap_or_default(),
        player_id: found.player_id,
        account_id: c.account_id,
        ignores_sender: session_ignores(&clients, found.addr, sender.name),
        flags,
        // DND wins over AFK: it is the stronger "leave me be".
        away_message: c.dnd_message.clone().or_else(|| c.afk_message.clone()),
    })
}

/// Deliver one tell, or refuse it with a feedback line. `text` has passed
/// the chat text rules.
#[tracing::instrument(
    name = "chat.tell",
    level = "info",
    skip_all,
    fields(
        player_id = sender.player_id,
        account_id = sender.account_id,
        entity_id = sender.entity_id,
    ),
)]
pub(super) async fn handle_tell(
    ctx: &FeedbackCtx<'_>,
    sender: TellSender<'_>,
    target: &str,
    text: &str,
) {
    let refuse = |reason: &'static str, target_player_id: Option<i32>| {
        tracing::debug!(
            target: "chat",
            event = "chat.tell_refused",
            addr = %sender.addr,
            player_id = sender.player_id,
            account_id = sender.account_id,
            entity_id = sender.entity_id,
            target_player_id,
            target_name = %shown(target),
            reason,
            "tell refused, feedback sent to the sender",
        );
    };

    if target.is_empty() {
        refuse("no_target", None);
        send_feedback_line(ctx, sender.addr, TELL_NO_TARGET_TEXT).await;
        return;
    }

    // The target is client text that feedback lines echo and logs carry, so
    // it gets the character-name rules first (64 UTF-16 units, no control,
    // bidi or format characters), the same bound `chatIgnore` and gate-mail
    // recipients get. A refusal echoes nothing back and logs only the
    // length, never the text.
    if let Err(reject) = validate(TextField::MailRecipient, target) {
        tracing::debug!(
            target: "chat",
            event = "chat.tell_refused",
            addr = %sender.addr,
            player_id = sender.player_id,
            account_id = sender.account_id,
            entity_id = sender.entity_id,
            target_units = target.encode_utf16().count(),
            reason = reject.reason(),
            "tell refused: the target is not a valid character name",
        );
        send_feedback_line(ctx, sender.addr, TELL_BAD_NAME_TEXT).await;
        return;
    }

    // A muted sender never gets here: the mute gate (SS-C3,
    // `chat_gates::refuse_if_muted`) runs in `send_player_communication_at`
    // before the tell branch, so spatial lines and tells share one check.

    let recipient = match resolve(ctx, &sender, target) {
        Resolution::Found(r) => r,
        Resolution::Myself => {
            refuse("self", sender.player_id);
            send_feedback_line(ctx, sender.addr, TELL_SELF_TEXT).await;
            return;
        }
        Resolution::Refused(NameLookup::Ambiguous) => {
            refuse("ambiguous", None);
            send_feedback_line(ctx, sender.addr, &ambiguous_text(target)).await;
            return;
        }
        Resolution::Refused(_) => {
            refuse("not_online", None);
            send_feedback_line(ctx, sender.addr, &not_online_text(target)).await;
            return;
        }
    };

    #[cfg(test)]
    after_resolve_hook::run(ctx);

    if recipient.ignores_sender {
        refuse("recipient_ignores_sender", Some(recipient.player_id));
        send_feedback_line(ctx, sender.addr, &not_accepting_text(&recipient.name)).await;
        return;
    }

    let line = serialize_on_player_communication(sender.name, sender.flags, TELL_CHANNEL, text);
    let (delivered, recipient_eid) = send_to_current_player(
        ctx,
        recipient.addr,
        recipient.player_id,
        ON_PLAYER_COMMUNICATION,
        &line,
    )
    .await;
    if delivered != FeedbackOutcome::Sent {
        // The recipient logged off, left the world (mid gate travel) or its
        // socket failed between the lookup and the send: the tell did not
        // arrive, so the sender hears so.
        let reason = match delivered {
            FeedbackOutcome::NoSession => "recipient_left",
            FeedbackOutcome::NotInWorld => "recipient_not_in_world",
            _ => "recipient_send_failed",
        };
        refuse(reason, Some(recipient.player_id));
        send_feedback_line(ctx, sender.addr, &not_online_text(&recipient.name)).await;
        return;
    }

    if let Some(sender_player_id) = sender.player_id {
        let confirm = serialize_on_tell_sent(&recipient.name, text);
        send_to_current_player(ctx, sender.addr, sender_player_id, ON_TELL_SENT, &confirm).await;
        if let Some(away) = &recipient.away_message {
            let reply = serialize_on_player_communication(
                &recipient.name,
                recipient.flags,
                TELL_CHANNEL,
                away,
            );
            send_to_current_player(
                ctx,
                sender.addr,
                sender_player_id,
                ON_PLAYER_COMMUNICATION,
                &reply,
            )
            .await;
        }
    }

    // Text content is never logged, only its length.
    tracing::info!(
        target: "chat",
        event = "chat.tell_delivered",
        addr = %sender.addr,
        player_id = sender.player_id,
        account_id = sender.account_id,
        entity_id = sender.entity_id,
        target_player_id = recipient.player_id,
        target_account_id = recipient.account_id,
        target_entity_id = recipient_eid,
        text_units = text.encode_utf16().count(),
        away_reply = recipient.away_message.is_some(),
        "tell delivered",
    );
}

/// Test seam: a callback run between the recipient lookup and the send, so
/// a test can move the recipient to a new entity (gate travel) or out of the
/// world in exactly the window PR #893's review flagged.
#[cfg(test)]
pub(super) mod after_resolve_hook {
    use std::cell::RefCell;

    use super::FeedbackCtx;

    type Hook = Box<dyn Fn(&FeedbackCtx<'_>)>;

    thread_local! {
        static HOOK: RefCell<Option<Hook>> = const { RefCell::new(None) };
    }

    /// Install `f` for the current thread (a `#[tokio::test]` runs on one).
    pub(in crate::base::dispatch) fn set(f: impl Fn(&FeedbackCtx<'_>) + 'static) {
        HOOK.with(|h| *h.borrow_mut() = Some(Box::new(f)));
    }

    pub(in crate::base::dispatch) fn clear() {
        HOOK.with(|h| *h.borrow_mut() = None);
    }

    pub(super) fn run(ctx: &FeedbackCtx<'_>) {
        HOOK.with(|h| {
            if let Some(f) = h.borrow().as_ref() {
                f(ctx);
            }
        });
    }
}
