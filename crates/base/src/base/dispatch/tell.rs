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
//! The rate limit (D-SS14) and the text rules (D-SS12) have already run in
//! `chat::send_player_communication_at` before this is reached.

use std::net::SocketAddr;

use cimmeria_wire::cell::chat::{serialize_on_player_communication, serialize_on_tell_sent};
use cimmeria_wire::cell::client_methods::communicator::{ON_PLAYER_COMMUNICATION, ON_TELL_SENT};

use super::super::contact_list::ignore::session_ignores;
use super::super::feedback::{
    send_feedback_line, send_player_method, FeedbackCtx, FeedbackOutcome,
};
use super::super::player_index::{NameLookup, OnlinePlayerIndex};
use super::speaker_flags;

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

/// D-SS15: the recipient ignores the sender.
pub(super) fn not_accepting_text(recipient: &str) -> String {
    format!("{recipient} is not accepting your messages.")
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

/// The resolved recipient, read under one lock.
struct Recipient {
    addr: SocketAddr,
    name: String,
    player_id: i32,
    account_id: u32,
    entity_id: Option<u32>,
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
        entity_id: c.player_entity_id,
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

    // TODO(SS-C3): refuse a muted sender here, before the name lookup.

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

    if recipient.ignores_sender {
        refuse("recipient_ignores_sender", Some(recipient.player_id));
        send_feedback_line(ctx, sender.addr, &not_accepting_text(&recipient.name)).await;
        return;
    }

    let Some(recipient_eid) = recipient.entity_id else {
        refuse("recipient_not_in_world", Some(recipient.player_id));
        send_feedback_line(ctx, sender.addr, &not_online_text(&recipient.name)).await;
        return;
    };

    let line = serialize_on_player_communication(sender.name, sender.flags, TELL_CHANNEL, text);
    let delivered = send_player_method(
        ctx,
        recipient.addr,
        recipient_eid,
        ON_PLAYER_COMMUNICATION,
        &line,
    )
    .await;
    if delivered != FeedbackOutcome::Sent {
        // The recipient's session went away (or its socket failed) between
        // the lookup and the send: tell the sender it did not arrive.
        refuse("recipient_send_failed", Some(recipient.player_id));
        send_feedback_line(ctx, sender.addr, &not_online_text(&recipient.name)).await;
        return;
    }

    if let Some(sender_eid) = sender.entity_id {
        let confirm = serialize_on_tell_sent(&recipient.name, text);
        send_player_method(ctx, sender.addr, sender_eid, ON_TELL_SENT, &confirm).await;
        if let Some(away) = &recipient.away_message {
            let reply = serialize_on_player_communication(
                &recipient.name,
                recipient.flags,
                TELL_CHANNEL,
                away,
            );
            send_player_method(
                ctx,
                sender.addr,
                sender_eid,
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
