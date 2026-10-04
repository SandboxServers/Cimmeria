//! SGWPlayer base-method chat handlers.
//!
//! Extracted from `dispatch.rs` — the chat-family arms of
//! `dispatch_sgw_player_base_method`: `sendPlayerCommunication`, `chatJoin`,
//! `chatLeave`, `chatSetAFKMessage`, and `chatSetDNDMessage`. The tell
//! channel branches off to `tell.rs` after the flood and text gates, and the
//! team, command and officer channels to the base's organization chat
//! (ORG-09, `organization::handlers::chat`): membership lives in the base,
//! so those lines never reach the cell.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cimmeria_base_session::base::organization::handlers::chat::{self as org_chat, ChatSpeaker};
use cimmeria_base_session::base::organization::handlers::OrgCtx;
use cimmeria_base_session::base::session_identity::{
    identity_for_entity, player_name_for_entity, session_identity,
};
use cimmeria_base_session::base::user_channels::{
    user_channel_registry, JoinOutcome, LeaveOutcome,
};
use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::read_wstring;

use super::super::feedback::{
    send_feedback_line, send_player_method, FeedbackCtx, FeedbackOutcome,
};
use cimmeria_entity::organization::org_text::{name_key, validate, TextField, TextReject};
use cimmeria_wire::cell::chat::{
    serialize_on_chat_joined, serialize_on_chat_left, serialize_on_player_communication, CHAN_CHAT,
    CHAN_SQUAD, CHAN_TELL,
};
use cimmeria_wire::cell::client_methods::communicator::{
    ON_CHAT_JOINED, ON_CHAT_LEFT, ON_PLAYER_COMMUNICATION,
};

use super::super::rate_limit::limits::{CHAT_EXEMPT_ACCESS_LEVEL, MAX_CHAT_TEXT_UNITS};
use super::super::rate_limit::{log_exceeded, RateActor, RateCategory, RateDecision};
use super::super::ConnectedClientState;
use super::chat_gates::{refuse_channel, refuse_if_muted, Speaker};
use super::speaker_flags;
use super::tell::{self, TellSender};

const MAX_DND_MESSAGE_CHARS: usize = 128;

/// The base state `sendPlayerCommunication` reads beyond the speaker's
/// session: the cell forward, and the organization chat's entity map and
/// database (ORG-09).
#[derive(Clone, Copy)]
pub(super) struct ChatRoutes<'a> {
    pub cell_tx: &'a Option<mpsc::Sender<BaseToCellMsg>>,
    pub entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
    pub db_pool: &'a Option<Arc<PgPool>>,
}

/// `sendPlayerCommunication(UINT8 channel, WSTRING target, WSTRING text)`.
///
/// Routes spatial channels (say/emote/yell) to the CellService with the
/// computed `speaker_flags`, after four gates that run here, before the cell
/// ever sees the line:
///
/// 1. the per-player chat bucket (D-SS14; GameMaster and above exempt);
/// 2. the channel allowlist (SS-C3, CAT-L-03, `chat_gates.rs`);
/// 3. the GM mute (SS-C3, D-SS26, `chat_gates.rs`), for tells as well;
/// 4. the D-SS12 text rules: at most [`MAX_CHAT_TEXT_UNITS`] UTF-16 units and
///    none of the characters D-ORG10 forbids (controls, bidi, zero-width and
///    other format characters, line separators), through the one
///    implementation, `org_text::validate(TextField::ChatText, ..)`.
///
/// The bucket runs first, so a refused line also costs a token: a client
/// spamming bad lines is limited like any other flood, and cannot turn each
/// bad packet into a feedback packet.
pub(super) async fn handle_send_player_communication(
    payload: &[u8],
    player_name: &Option<String>,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    routes: ChatRoutes<'_>,
) {
    send_player_communication_at(
        payload,
        player_name,
        addr,
        transport,
        connected,
        routes,
        Instant::now(),
    )
    .await;
}

/// [`handle_send_player_communication`] on an explicit clock, so the flood
/// guards can step time exactly.
pub(super) async fn send_player_communication_at(
    payload: &[u8],
    player_name: &Option<String>,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    routes: ChatRoutes<'_>,
    now: Instant,
) {
    let cell_tx = routes.cell_tx;
    // sendPlayerCommunication(UINT8 channel, WSTRING target, WSTRING text)
    if payload.is_empty() {
        return;
    }
    let channel = payload[0];
    let mut offset = 1;

    // Parse target (WSTRING). `read_wstring` returns the number
    // of BYTES CONSUMED (not the new absolute offset), so
    // accumulate with `+=` — `offset = ret` would drop the +1
    // for the channel byte and mis-align the subsequent text
    // WSTRING read. Empty-target spatial channels (say / emote /
    // yell) are the case this matters most: with a 0-length
    // target the text length lives at the byte right after the
    // channel byte, and the old `=` assignment made the text
    // read see garbage.
    let (target, target_bytes) = match read_wstring(payload, offset) {
        Ok(v) => v,
        Err(_) => return,
    };
    offset += target_bytes;

    // Parse text (WSTRING)
    let (text, _) = match read_wstring(payload, offset) {
        Ok(v) => v,
        Err(_) => return,
    };

    let speaker = player_name.as_deref().unwrap_or("Unknown");

    // Read player_eid + speaker flags and take the chat token under a
    // single lock acquisition. Computing `speaker_flags` matches
    // `python/base/Chat.py::getSpeakerFlags`:
    //   - SPEAKER_GM  if accessLevel > 0  (Moderator or higher)
    //   - SPEAKER_DND if dndMessage is not None
    // SPEAKER_Petition (0x02) is in the enum but never set by the
    // Python reference, so it is intentionally not computed.
    let (player_eid, speaker_flags_value, player_id, account_id, access_level, decision, ident) = {
        let mut clients = connected.lock().unwrap();
        match clients.get_mut(&addr) {
            Some(c) => {
                let mut flags: u8 = 0;
                if c.access_level > 0 {
                    flags |= speaker_flags::GM;
                }
                if c.dnd_message.is_some() {
                    flags |= speaker_flags::DND;
                }
                let decision = if c.access_level >= CHAT_EXEMPT_ACCESS_LEVEL {
                    RateDecision::Allowed
                } else {
                    c.rate_limits.check(RateCategory::Chat, now)
                };
                if let RateDecision::Limited { notify } = decision {
                    // Logged here, under the lock, so the event carries the
                    // bucket state the decision was made on.
                    let actor = RateActor {
                        addr,
                        player_id: c.active_player_id,
                        account_id: c.account_id,
                        entity_id: c.player_entity_id,
                    };
                    log_exceeded(RateCategory::Chat, actor, notify, &c.rate_limits, now);
                }
                (
                    c.player_entity_id,
                    flags,
                    c.active_player_id,
                    c.account_id,
                    c.access_level,
                    decision,
                    session_identity(c),
                )
            }
            None => return,
        }
    };

    let feedback = FeedbackCtx {
        transport,
        connected,
    };

    let text_units = text.encode_utf16().count();
    let org_speaker = ChatSpeaker {
        addr,
        name: speaker,
        flags: speaker_flags_value,
        account_id: Some(account_id),
        player_id,
        entity_id: player_eid,
        identity: ident,
    };
    let squad_refusal = |reason: &'static str, log_row: bool| {
        if channel == CHAN_SQUAD {
            squad_chat_rejected(
                reason, log_row, account_id, player_id, player_eid, ident, text_units,
            );
        }
        org_chat::log_refused_before_relay(channel, reason, log_row, &org_speaker, text_units);
    };

    if let RateDecision::Limited { notify } = decision {
        // The row follows the feedback throttle: a flooding client gets one
        // `squad.chat` row per notice, not one per dropped packet. The
        // counter sees every drop.
        squad_refusal("rate_limited", notify);
        if notify {
            send_feedback_line(&feedback, addr, RateCategory::Chat.feedback_text()).await;
        }
        return;
    }

    let who = Speaker {
        addr,
        player_id,
        account_id,
        entity_id: player_eid,
        access_level,
        identity: ident,
    };
    if refuse_channel(&feedback, who, channel).await {
        return;
    }
    if refuse_if_muted(&feedback, who, channel, now).await {
        return;
    }

    // D-SS12: reject, never truncate. Lengths count UTF-16 units, the unit
    // the client's WSTRING and its input box use.
    if let Err(reject) = validate(TextField::ChatText, &text) {
        tracing::warn!(
            target: "chat",
            event = "chat.rejected",
            %addr,
            player_id,
            player_name = ident.player_name,
            account_id,
            account_name = ident.account_name,
            entity_id = player_eid,
            entity_name = ident.player_name,
            channel,
            text_units,
            max_units = MAX_CHAT_TEXT_UNITS,
            reason = reject.reason(),
            detail = %reject,
            "sendPlayerCommunication rejected: text breaks the chat text rules, not forwarded",
        );
        squad_refusal("text_invalid", true);
        send_feedback_line(&feedback, addr, chat_reject_text(&reject)).await;
        return;
    }

    // Tells are delivered here, on the base, and never reach the cell.
    if channel == CHAN_TELL {
        let sender = TellSender {
            addr,
            name: speaker,
            flags: speaker_flags_value,
            entity_id: player_eid,
            player_id,
            account_id,
            identity: ident,
        };
        tell::handle_tell(&feedback, sender, &target, &text, now).await;
        return;
    }

    // Team, command and officer: the base holds the membership (ORG-09).
    if org_chat::org_channel(channel).is_some() {
        let ctx = OrgCtx {
            db_pool: routes.db_pool,
            transport,
            connected,
            entity_to_addr: routes.entity_to_addr,
            cell_tx,
        };
        org_chat::relay_org_chat(&ctx, org_speaker, channel, &text).await;
        return;
    }

    // A user channel (12 and up): the base holds this membership too, and
    // it never reaches the cell (see the `user_channels` module doc for
    // why -- no spatial component to distribute).
    if channel >= CHAN_CHAT {
        post_to_user_channel(
            &feedback,
            routes.entity_to_addr,
            addr,
            player_eid,
            player_id,
            account_id,
            ident,
            speaker,
            speaker_flags_value,
            channel,
            &text,
        )
        .await;
        return;
    }

    // Logged only once both gates pass: every field here is client-supplied,
    // so a flooding client must not get one INFO row per packet.
    tracing::info!(
        %addr,
        player_id,
        player_name = ident.player_name,
        account_id,
        account_name = ident.account_name,
        entity_id = player_eid,
        entity_name = ident.player_name,
        speaker,
        channel,
        chat_target = if target.is_empty() { "<none>" } else { &target },
        text_len = text.len(),
        "sendPlayerCommunication"
    );

    if let Some(player_eid) = player_eid {
        if let Some(ref tx) = cell_tx {
            let _ = tx
                .send(BaseToCellMsg::ChatMessage {
                    entity_id: player_eid,
                    speaker_name: speaker.to_string(),
                    speaker_flags: speaker_flags_value,
                    channel,
                    text,
                })
                .await;
        }
    }
}

/// The `squad.chat` outcome row (ORG-04) for a squad line the base refused
/// before the cell forward, so a squad line's refusal is found under the
/// same event as the cell's `not_in_squad` and `ok` rows. `log_row` is false
/// for a rate-limited drop whose feedback is throttled; the counter still
/// counts it.
fn squad_chat_rejected(
    reason: &'static str,
    log_row: bool,
    account_id: u32,
    player_id: Option<i32>,
    entity_id: Option<u32>,
    ident: PlayerIdentity,
    text_units: usize,
) {
    if log_row {
        tracing::info!(
            target: "squad",
            event = "squad.chat",
            outcome = "rejected",
            reason,
            account_id,
            account_name = ident.account_name,
            player_id,
            player_name = ident.player_name,
            entity_id,
            entity_name = ident.player_name,
            recipients = 0,
            text_units,
            "squad chat rejected"
        );
    }
    cimmeria_observability::counter!(
        "squad_actions_total",
        "action" => "chat",
        "outcome" => "rejected",
        "reason" => reason,
    );
}

/// Feedback for a chat line over [`MAX_CHAT_TEXT_UNITS`].
pub(super) const CHAT_TOO_LONG_TEXT: &str = "Your message is too long.";

/// Feedback for a chat line with a character the text rules forbid.
pub(super) const CHAT_BAD_CHARACTER_TEXT: &str =
    "Your message contains a character that cannot be sent.";

/// The one line the player reads for a refused chat line.
fn chat_reject_text(reject: &TextReject) -> &'static str {
    match reject {
        TextReject::TooLong { .. } => CHAT_TOO_LONG_TEXT,
        _ => CHAT_BAD_CHARACTER_TEXT,
    }
}

/// The line for a `chatJoin` whose channel name breaks the chat text rules.
const CHANNEL_NAME_INVALID_TEXT: &str =
    "That is not a valid channel name. Channel names may only use letters, digits, spaces, \
     ' - and . characters.";
/// The line for a `chatJoin` that would put the caller's entity over
/// [`cimmeria_entity::organization::limits::MAX_CHANNELS_PER_PLAYER`].
const CHANNEL_PLAYER_LIMIT_TEXT: &str =
    "You are in too many chat channels already. Leave one before joining another.";
/// The line for a `chatJoin` that would put the server over
/// [`cimmeria_entity::organization::limits::MAX_USER_CHANNELS`].
const CHANNEL_SERVER_LIMIT_TEXT: &str =
    "No more chat channels can be created right now. Try again later.";
/// The line for a `chatLeave` or a channel post naming a channel the
/// caller's entity is not a member of (or that does not exist -- the two
/// are not distinguished, so a client cannot probe which names exist).
const CHANNEL_NOT_MEMBER_TEXT: &str = "You are not in that chat channel.";

/// `chatJoin(WSTRING channelName, WSTRING password)`.
///
/// Creates the named user channel if none exists yet, or joins the
/// existing one (matched case-insensitively, D-SS13 style, on
/// [`name_key`]). This is a deliberate improvement over the legacy
/// `ChatChannelManager.joinChannel` (`Chat.py:234-247`), which only ever
/// joined a *pre-existing* channel and otherwise failed with a server-side
/// warning and no feedback at all: the one code path that created a
/// channel, `requestCreateChannel`, is never called from any base method
/// the client can reach, so the legacy server's own players could never
/// actually create one. Auto-creating here is what makes the client's
/// three login auto-joins (`channel-chat`, `channel-roleplay`,
/// `channel-alliance`) — and a manual `/chatjoin <new name>` — work.
///
/// On success, sends `onChatJoined`, which is itself the client's "You have
/// joined channel" feedback (`ChatWindow.lua::onChannelJoined`); see the
/// `user_channels` module doc for why no separate feedback line follows a
/// success, for an auto-join or a manual one alike. Every refusal (a bad
/// name, already a member, or either channel-count limit) gets exactly one
/// feedback line and logs `chat.channel_join_rejected` (WARN).
///
/// The password argument is decoded, so a malformed payload is still
/// refused before any state changes, and otherwise unused: no channel
/// created here ever has one, matching `chatPassword` (0xCC) staying "not
/// available yet".
pub(super) async fn handle_chat_join(
    payload: &[u8],
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    // chatJoin(WSTRING channelName, WSTRING password)
    let (channel_name, offset) = match read_wstring(payload, 0) {
        Ok(v) => v,
        Err(_) => return,
    };
    let (_password, _) = match read_wstring(payload, offset) {
        Ok(v) => v,
        Err(_) => return,
    };
    // Every routing test needs is this handler reached with the raw name;
    // everything past this line may bail out early for a session with no
    // character yet (`who_at` returning `None`).
    tracing::debug!(%addr, channel_name, "chatJoin: join requested");

    let feedback = FeedbackCtx {
        transport,
        connected,
    };
    let Some(who) = who_at(connected, addr) else {
        return;
    };

    let normalized = match validate(TextField::ChannelName, &channel_name) {
        Ok(n) => n,
        Err(reject) => {
            tracing::warn!(
                target: "chat",
                event = "chat.channel_join_rejected",
                %addr,
                player_id = who.player_id,
                player_name = who.identity.player_name,
                account_id = who.account_id,
                account_name = who.identity.account_name,
                entity_id = who.entity_id,
                entity_name = who.identity.player_name,
                reason = reject.reason(),
                detail = %reject,
                "chatJoin refused: the channel name breaks the chat text rules, no channel joined",
            );
            send_feedback_line(&feedback, addr, CHANNEL_NAME_INVALID_TEXT).await;
            return;
        }
    };
    let key = name_key(&normalized);

    match user_channel_registry().join(&normalized, &key, who.entity_id) {
        JoinOutcome::Joined {
            wire_id,
            display_name,
            created,
        } => {
            let display_id = wire_id - CHAN_CHAT;
            let args = serialize_on_chat_joined(&display_name, display_id);
            send_player_method(&feedback, addr, who.entity_id, ON_CHAT_JOINED, &args).await;
            tracing::info!(
                target: "chat",
                event = "chat.channel_joined",
                %addr,
                player_id = who.player_id,
                player_name = who.identity.player_name,
                account_id = who.account_id,
                account_name = who.identity.account_name,
                entity_id = who.entity_id,
                entity_name = who.identity.player_name,
                wire_id,
                wire_name = %display_name,
                display_id,
                display_name = %display_name,
                channel_name = %display_name,
                created,
                "chatJoin: joined a user channel",
            );
        }
        JoinOutcome::AlreadyMember { display_name } => {
            tracing::warn!(
                target: "chat",
                event = "chat.channel_join_rejected",
                %addr,
                player_id = who.player_id,
                player_name = who.identity.player_name,
                account_id = who.account_id,
                account_name = who.identity.account_name,
                entity_id = who.entity_id,
                entity_name = who.identity.player_name,
                reason = "already_member",
                channel_name = %display_name,
                "chatJoin refused: already a member of that channel",
            );
            send_feedback_line(
                &feedback,
                addr,
                &format!("You are already in channel {display_name}."),
            )
            .await;
        }
        JoinOutcome::PlayerLimitReached => {
            tracing::warn!(
                target: "chat",
                event = "chat.channel_join_rejected",
                %addr,
                player_id = who.player_id,
                player_name = who.identity.player_name,
                account_id = who.account_id,
                account_name = who.identity.account_name,
                entity_id = who.entity_id,
                entity_name = who.identity.player_name,
                reason = "player_limit",
                "chatJoin refused: the caller already holds the maximum number of channels",
            );
            send_feedback_line(&feedback, addr, CHANNEL_PLAYER_LIMIT_TEXT).await;
        }
        JoinOutcome::ServerLimitReached => {
            tracing::warn!(
                target: "chat",
                event = "chat.channel_join_rejected",
                %addr,
                player_id = who.player_id,
                player_name = who.identity.player_name,
                account_id = who.account_id,
                account_name = who.identity.account_name,
                entity_id = who.entity_id,
                entity_name = who.identity.player_name,
                reason = "server_limit",
                "chatJoin refused: the server already holds the maximum number of channels",
            );
            send_feedback_line(&feedback, addr, CHANNEL_SERVER_LIMIT_TEXT).await;
        }
    }
}

/// `chatLeave(UINT8 channelId)`.
///
/// `channelId` is the **display id** the client tracks (the second
/// argument `onChatJoined` sent when it joined): the wire channel id is
/// `channelId + CHAN_CHAT`, mirroring the legacy
/// `SGWPlayer.py::chatLeave` (`channelId + Constants.MIN_USER_CHANNEL`).
///
/// On success, sends `onChatLeft`, itself the client's "You have left
/// channel" feedback and the trigger that removes the channel's tab
/// subscriptions (`ChatWindow.lua::onChannelLeft`); see the
/// `user_channels` module doc for why no separate feedback line follows. A
/// display id the caller's entity is not currently a member of (including
/// one that names no channel at all) gets one feedback line and logs
/// `chat.channel_leave_rejected` (WARN).
pub(super) async fn handle_chat_leave(
    payload: &[u8],
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    // chatLeave(UINT8 channelId)
    let display_id = if !payload.is_empty() { payload[0] } else { 0 };
    tracing::debug!(
        %addr,
        channel_id = display_id, // nt:id-only client-supplied id read before the registry lookup
        "chatLeave: leave requested"
    );

    let feedback = FeedbackCtx {
        transport,
        connected,
    };
    let Some(who) = who_at(connected, addr) else {
        return;
    };
    let wire_id = CHAN_CHAT.wrapping_add(display_id);

    match user_channel_registry().leave(wire_id, who.entity_id) {
        LeaveOutcome::Left {
            display_name,
            deleted,
        } => {
            let args = serialize_on_chat_left(&display_name);
            send_player_method(&feedback, addr, who.entity_id, ON_CHAT_LEFT, &args).await;
            tracing::info!(
                target: "chat",
                event = "chat.channel_left",
                %addr,
                player_id = who.player_id,
                player_name = who.identity.player_name,
                account_id = who.account_id,
                account_name = who.identity.account_name,
                entity_id = who.entity_id,
                entity_name = who.identity.player_name,
                wire_id,
                wire_name = %display_name,
                display_id,
                display_name = %display_name,
                channel_name = %display_name,
                deleted,
                "chatLeave: left a user channel",
            );
        }
        LeaveOutcome::NotFound | LeaveOutcome::NotMember => {
            tracing::warn!(
                target: "chat",
                event = "chat.channel_leave_rejected",
                %addr,
                player_id = who.player_id,
                player_name = who.identity.player_name,
                account_id = who.account_id,
                account_name = who.identity.account_name,
                entity_id = who.entity_id,
                entity_name = who.identity.player_name,
                wire_id, // nt:id-only the registry has no name for an id the caller never joined
                display_id, // nt:id-only client-supplied display id; it may name no channel at all
                reason = "not_member",
                "chatLeave refused: not a member of that channel, nothing left",
            );
            send_feedback_line(&feedback, addr, CHANNEL_NOT_MEMBER_TEXT).await;
        }
    }
}

/// A `sendPlayerCommunication` line on a user channel id (12 and up):
/// reaches every member of that channel, the speaker included -- unlike
/// say/emote/yell there is no local client echo to avoid doubling (that
/// rule is about *spatial* channels specifically; see
/// `chat-speaker-echo.md`), and the legacy `ChatChannel.sendMessage`
/// (`Chat.py:125-138`) always sent to every member without excluding the
/// sender. Server authority: a channel id the caller's entity never joined
/// is refused here, with feedback, before anything is forwarded --
/// identical in spirit to the system/unknown-channel refusals in
/// `chat_gates.rs`, just one layer down because it needs the membership
/// table.
#[allow(clippy::too_many_arguments)]
async fn post_to_user_channel(
    feedback: &FeedbackCtx<'_>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    addr: SocketAddr,
    player_eid: Option<u32>,
    player_id: Option<i32>,
    account_id: u32,
    ident: PlayerIdentity,
    speaker: &str,
    speaker_flags: u8,
    channel: u8,
    text: &str,
) {
    let text_units = text.encode_utf16().count();
    let Some(entity_id) = player_eid else {
        return;
    };
    let Some(members) = user_channel_registry().members_if_joined(channel, entity_id) else {
        tracing::warn!(
            target: "chat",
            event = "chat.channel_post_rejected",
            %addr,
            player_id,
            player_name = ident.player_name,
            account_id,
            account_name = ident.account_name,
            entity_id,
            entity_name = ident.player_name,
            channel,
            reason = "not_member",
            "sendPlayerCommunication refused: not a member of that channel, not forwarded",
        );
        send_feedback_line(feedback, addr, CHANNEL_NOT_MEMBER_TEXT).await;
        return;
    };

    let args = serialize_on_player_communication(speaker, speaker_flags, channel, text);
    let mut recipients = 0usize;
    for member_entity in &members {
        let member_addr = entity_to_addr.lock().unwrap().get(member_entity).copied();
        let Some(member_addr) = member_addr else {
            tracing::debug!(
                target: "chat",
                event = "chat.channel_send_skipped",
                channel,
                member_entity_id = member_entity,
                member_entity_name = identity_for_entity(
                    feedback.connected,
                    entity_to_addr,
                    *member_entity,
                )
                .player_name,
                reason = "entity_to_addr_miss",
                "user channel post: member has no known address, skipped",
            );
            continue;
        };
        match send_player_method(
            feedback,
            member_addr,
            *member_entity,
            ON_PLAYER_COMMUNICATION,
            &args,
        )
        .await
        {
            FeedbackOutcome::Sent => recipients += 1,
            outcome => tracing::warn!(
                target: "chat",
                event = "chat.channel_send_failed",
                channel,
                member_entity_id = member_entity,
                member_entity_name = player_name_for_entity(
                    feedback.connected,
                    entity_to_addr,
                    *member_entity,
                ),
                outcome = ?outcome,
                "user channel post: send failed for a member",
            ),
        }
    }
    tracing::info!(
        target: "chat",
        event = "chat.channel_post",
        %addr,
        player_id,
        player_name = ident.player_name,
        account_id,
        account_name = ident.account_name,
        entity_id,
        entity_name = ident.player_name,
        channel,
        recipients,
        text_units,
        "sendPlayerCommunication delivered on a user channel",
    );
}

/// The caller's identity for `chatJoin`/`chatLeave`: only meaningful once a
/// character is in the world, so `None` (session mid-world-entry or at
/// character select) is simply dropped by both handlers, the same as
/// `sendPlayerCommunication`'s own `None => return` on a session lookup
/// miss.
struct ChatSessionWho {
    entity_id: u32,
    player_id: Option<i32>,
    account_id: u32,
    /// The session's names, for the log lines (Rule 6).
    identity: PlayerIdentity,
}

fn who_at(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> Option<ChatSessionWho> {
    let clients = connected.lock().unwrap();
    let c = clients.get(&addr)?;
    Some(ChatSessionWho {
        entity_id: c.player_entity_id?,
        player_id: c.active_player_id,
        account_id: c.account_id,
        identity: session_identity(c),
    })
}

/// `chatSetAFKMessage(WSTRING message)`.
///
/// Stores the away message the tell path sends back to anyone who tells this
/// player (`tell.rs`). Same rules as DND (`python/base/SGWPlayer.py:195-199`):
/// an empty or 1-char message clears it, a longer one is stored truncated to
/// [`MAX_DND_MESSAGE_CHARS`], and a malformed payload leaves it untouched.
///
/// AFK is NOT a speaker flag: `entities/defs/enumerations.xml` has no
/// `SPEAKER_AFK` token, and `python/base/Chat.py::getSpeakerFlags` only checks
/// `accessLevel > 0` / `dndMessage is not None`.
///
/// Other players read the message in the tell auto-reply, so it must pass
/// the D-SS12 / D-ORG10 character rules; a message that does not is refused
/// with a feedback line and the previous state is kept.
pub(super) async fn handle_chat_set_afk(
    payload: &[u8],
    addr: SocketAddr,
    feedback: &FeedbackCtx<'_>,
) {
    let connected = feedback.connected;
    let message = match read_wstring(payload, 0) {
        Ok((s, _)) => s,
        Err(e) => {
            tracing::warn!(
                %addr,
                payload_len = payload.len(),
                reason = "read_wstring_failed",
                error = %e,
                "chatSetAFKMessage: WSTRING decode failed -- existing AFK state preserved",
            );
            return;
        }
    };
    let active = message.chars().count() > 1;
    // Check the whole decoded text before the bound cuts it: a forbidden
    // character past scalar 128 must refuse the message, not vanish.
    if active && !away_message_allowed(feedback, addr, "afk", &message).await {
        return;
    }
    let stored: String = message.chars().take(MAX_DND_MESSAGE_CHARS).collect();
    let mut clients = connected.lock().unwrap();
    if let Some(c) = clients.get_mut(&addr) {
        c.afk_message = active.then_some(stored);
        tracing::debug!(
            target: "chat",
            event = "chat.afk_set",
            %addr,
            player_id = c.active_player_id,
            player_name = c.player_name.as_deref(),
            account_id = c.account_id,
            account_name = c.account_name.as_deref(),
            entity_id = c.player_entity_id,
            entity_name = c.player_name.as_deref(),
            afk_active = active,
            "chatSetAFKMessage",
        );
    }
}

/// `chatSetDNDMessage(WSTRING message)`.
///
/// Mirrors `python/base/SGWPlayer.py::chatSetDNDMessage`: an empty or 1-char
/// message clears DND; anything longer sets it. The stored text is truncated
/// to 128 Unicode scalar values, and must pass the same character rules as
/// the AFK message (it is sent back to anyone who tells this player).
pub(super) async fn handle_chat_set_dnd(
    payload: &[u8],
    addr: SocketAddr,
    feedback: &FeedbackCtx<'_>,
) {
    let connected = feedback.connected;
    // chatSetDNDMessage(WSTRING message)
    //
    // Mirrors `python/base/SGWPlayer.py::chatSetDNDMessage`: an
    // empty or 1-char message clears DND; anything longer sets
    // it, stored truncated to MAX_DND_MESSAGE_CHARS so the per-client
    // state is bounded (#471 CAT-L-02). Truncating rather than
    // refusing keeps the player's /dnd visibly taking effect. The stored message itself is currently only used as
    // an "is DND active?" signal for the speaker_flags bit —
    // the auto-reply-tell path is future work.
    //
    // A decode failure (truncated / malformed payload) must NOT
    // be coerced to `""` and then treated as a clear — that
    // silently destroys existing DND state on a garbage packet.
    // Bind the Result explicitly, warn-log on Err per
    // `docs/architecture/negative-logging-convention.md`, and
    // leave `dnd_message` untouched so a flaky packet doesn't
    // surprise the user.
    let message = match read_wstring(payload, 0) {
        Ok((s, _)) => s,
        Err(e) => {
            tracing::warn!(
                %addr,
                payload_len = payload.len(),
                reason = "read_wstring_failed",
                error = %e,
                "chatSetDNDMessage: WSTRING decode failed -- existing DND state preserved",
            );
            return;
        }
    };
    let message_chars = message.chars().count();
    // Check the whole decoded text before the bound cuts it (see AFK).
    if message_chars > 1 && !away_message_allowed(feedback, addr, "dnd", &message).await {
        return;
    }
    let message = match message.char_indices().nth(MAX_DND_MESSAGE_CHARS) {
        Some((cut, _)) => {
            tracing::debug!(
                %addr,
                message_chars,
                limit = MAX_DND_MESSAGE_CHARS,
                reason = "dnd_message_truncated",
                "chatSetDNDMessage: message exceeds limit -- stored truncated",
            );
            let mut message = message;
            message.truncate(cut);
            message
        }
        None => message,
    };
    let mut clients = connected.lock().unwrap();
    if let Some(c) = clients.get_mut(&addr) {
        c.dnd_message = if message_chars > 1 {
            Some(message)
        } else {
            None
        };
        tracing::debug!(
            %addr,
            dnd_active = c.dnd_message.is_some(),
            "chatSetDNDMessage",
        );
    }
}

/// Feedback for an AFK or DND message with a character the text rules forbid.
pub(super) const AWAY_BAD_CHARACTER_TEXT: &str =
    "Your away message contains a character that cannot be sent. It was not set.";

/// The D-SS12 / D-ORG10 character rules on an away message, applied to the
/// whole decoded text before it is cut to 128 scalars. Length is not this
/// check's business: the caller bounds the stored text. A refusal logs `chat.away_rejected` with `reason` and sends one
/// feedback line; the caller keeps the previous state.
async fn away_message_allowed(
    feedback: &FeedbackCtx<'_>,
    addr: SocketAddr,
    kind: &'static str,
    text: &str,
) -> bool {
    // The character rules run before the length check inside `validate`, so
    // a `TooLong` means the characters passed. The length rule is the away
    // message's own 128-scalar truncation, applied by the caller, not the
    // 255-unit chat-line cap.
    let reject = match validate(TextField::ChatText, text) {
        Ok(_) | Err(TextReject::TooLong { .. }) => return true,
        Err(reject) => reject,
    };
    let (ident, entity_id) = feedback
        .connected
        .lock()
        .unwrap()
        .get(&addr)
        .map_or((PlayerIdentity::UNKNOWN, None), |c| {
            (session_identity(c), c.player_entity_id)
        });
    tracing::warn!(
        target: "chat",
        event = "chat.away_rejected",
        %addr,
        player_id = ident.player_id,
        player_name = ident.player_name,
        account_id = ident.account_id,
        account_name = ident.account_name,
        entity_id,
        entity_name = ident.player_name,
        kind,
        reason = reject.reason(),
        "away message refused: it breaks the chat text rules; previous state kept",
    );
    send_feedback_line(feedback, addr, AWAY_BAD_CHARACTER_TEXT).await;
    false
}
