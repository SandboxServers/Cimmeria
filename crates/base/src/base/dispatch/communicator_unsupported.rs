//! The Communicator base methods the server does not implement, 0xC6-0xCE
//! (SS-C3, D-SS26): `chatFriend`, `chatList`, `chatMute`, `chatKick`,
//! `chatOp`, `chatBan`, `chatPassword`, `petition` and `announcePetition`.
//!
//! Before SS-C3 they fell into the catch-all, which logged a WARN and sent
//! the player nothing. Each now answers the press with its own "not
//! available" line and logs `chat.method_unsupported` with the method name
//! and `reason = not_implemented`. The arguments are not decoded: nothing
//! is done with them, and the log carries only `payload_len`.
//!
//! A press costs a chat token (D-SS14), like a chat line, so a client that
//! floods these indices gets the one "too quickly" line instead of one
//! feedback packet per request packet.

use std::net::SocketAddr;
use std::time::Instant;

use super::super::feedback::{send_feedback_line, FeedbackCtx};
use super::super::rate_limit::limits::CHAT_EXEMPT_ACCESS_LEVEL;
use super::super::rate_limit::{log_exceeded, RateActor, RateCategory, RateDecision};
use super::sgw_player_base as idx;

pub(super) const CHAT_FRIEND_TEXT: &str =
    "Adding friends from chat is not available yet. Use the contact list instead.";
pub(super) const CHAT_LIST_TEXT: &str =
    "Listing the players in a chat channel is not available yet.";
pub(super) const CHAT_MUTE_TEXT: &str =
    "Muting players in a chat channel is not available yet. You can ignore a player instead.";
pub(super) const CHAT_KICK_TEXT: &str = "Kicking players from a chat channel is not available yet.";
pub(super) const CHAT_OP_TEXT: &str = "Chat channel operators are not available yet.";
pub(super) const CHAT_BAN_TEXT: &str = "Banning players from a chat channel is not available yet.";
pub(super) const CHAT_PASSWORD_TEXT: &str = "Chat channel passwords are not available yet.";
pub(super) const PETITION_TEXT: &str = "Petitions to the GMs are not available yet.";
pub(super) const ANNOUNCE_PETITION_TEXT: &str = "Petition announcements are not available yet.";

/// Route one of 0xC6-0xCE to its arm. `msg_id` outside that range is a
/// caller bug and is ignored.
pub(super) async fn handle_unsupported_communicator(
    msg_id: u8,
    payload_len: usize,
    feedback: &FeedbackCtx<'_>,
    addr: SocketAddr,
    now: Instant,
) {
    let press = Press {
        feedback,
        addr,
        payload_len,
        now,
    };
    match msg_id {
        idx::CHAT_FRIEND => chat_friend(press).await,
        idx::CHAT_LIST => chat_list(press).await,
        idx::CHAT_MUTE => chat_mute(press).await,
        idx::CHAT_KICK => chat_kick(press).await,
        idx::CHAT_OP => chat_op(press).await,
        idx::CHAT_BAN => chat_ban(press).await,
        idx::CHAT_PASSWORD => chat_password(press).await,
        idx::PETITION => petition(press).await,
        idx::ANNOUNCE_PETITION => announce_petition(press).await,
        _ => {}
    }
}

/// One press of an unsupported method.
#[derive(Clone, Copy)]
struct Press<'a, 'b> {
    feedback: &'a FeedbackCtx<'b>,
    addr: SocketAddr,
    payload_len: usize,
    now: Instant,
}

/// `chatFriend(WSTRING aPlayerName, WSTRING aPlayerNick, UINT8 aFlag)`.
async fn chat_friend(p: Press<'_, '_>) {
    refuse(p, "chatFriend", CHAT_FRIEND_TEXT).await;
}

/// `chatList(UINT8 aChannelID)`.
async fn chat_list(p: Press<'_, '_>) {
    refuse(p, "chatList", CHAT_LIST_TEXT).await;
}

/// `chatMute(UINT8 aChannelID, WSTRING aPlayerName, UINT8 aFlag)`. A
/// channel mute by a channel operator; the GM mute is `.mute`.
async fn chat_mute(p: Press<'_, '_>) {
    refuse(p, "chatMute", CHAT_MUTE_TEXT).await;
}

/// `chatKick(UINT8 aChannelID, WSTRING aPlayerName)`.
async fn chat_kick(p: Press<'_, '_>) {
    refuse(p, "chatKick", CHAT_KICK_TEXT).await;
}

/// `chatOp(UINT8 aChannelID, WSTRING aPlayerName)`.
async fn chat_op(p: Press<'_, '_>) {
    refuse(p, "chatOp", CHAT_OP_TEXT).await;
}

/// `chatBan(UINT8 aChannelID, WSTRING aPlayerName, UINT8 aFlag)`.
async fn chat_ban(p: Press<'_, '_>) {
    refuse(p, "chatBan", CHAT_BAN_TEXT).await;
}

/// `chatPassword(UINT8 aChannelID, WSTRING aChannelPassword)`. The password
/// is never decoded or logged.
async fn chat_password(p: Press<'_, '_>) {
    refuse(p, "chatPassword", CHAT_PASSWORD_TEXT).await;
}

/// `petition(WSTRING aMessage)`.
async fn petition(p: Press<'_, '_>) {
    refuse(p, "petition", PETITION_TEXT).await;
}

/// `announcePetition(WSTRING aMessage)`.
async fn announce_petition(p: Press<'_, '_>) {
    refuse(p, "announcePetition", ANNOUNCE_PETITION_TEXT).await;
}

/// Charge the chat bucket, log the press, and send its line.
async fn refuse(p: Press<'_, '_>, method: &'static str, text: &str) {
    let (actor, decision) = {
        let mut clients = p.feedback.connected.lock().unwrap();
        let Some(c) = clients.get_mut(&p.addr) else {
            return;
        };
        let actor = RateActor::of(p.addr, c);
        let decision = if c.access_level >= CHAT_EXEMPT_ACCESS_LEVEL {
            RateDecision::Allowed
        } else {
            c.rate_limits.check(RateCategory::Chat, p.now)
        };
        if let RateDecision::Limited { notify } = decision {
            log_exceeded(RateCategory::Chat, actor, notify, &c.rate_limits, p.now);
        }
        (actor, decision)
    };
    if let RateDecision::Limited { notify } = decision {
        if notify {
            send_feedback_line(p.feedback, p.addr, RateCategory::Chat.feedback_text()).await;
        }
        return;
    }
    tracing::warn!(
        target: "chat",
        event = "chat.method_unsupported",
        addr = %p.addr,
        player_id = actor.player_id,
        player_name = actor.player_name,
        account_id = actor.account_id,
        account_name = actor.account_name,
        entity_id = actor.entity_id,
        entity_name = actor.player_name,
        method,
        payload_len = p.payload_len,
        reason = "not_implemented",
        "Communicator base method not implemented on this server, the player was told",
    );
    send_feedback_line(p.feedback, p.addr, text).await;
}
