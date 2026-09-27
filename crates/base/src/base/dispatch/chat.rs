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
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::read_wstring;

use super::super::feedback::{send_feedback_line, FeedbackCtx};
use cimmeria_entity::organization::org_text::{validate, TextField, TextReject};
use cimmeria_wire::cell::chat::{CHAN_SQUAD, CHAN_TELL};

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
    let (player_eid, speaker_flags_value, player_id, account_id, access_level, decision) = {
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
    };
    let squad_refusal = |reason: &'static str, log_row: bool| {
        if channel == CHAN_SQUAD {
            squad_chat_rejected(
                reason, log_row, account_id, player_id, player_eid, text_units,
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
            account_id,
            entity_id = player_eid,
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

    // Logged only once both gates pass: every field here is client-supplied,
    // so a flooding client must not get one INFO row per packet.
    tracing::info!(
        %addr,
        player_id,
        account_id,
        entity_id = player_eid,
        speaker,
        channel,
        target = if target.is_empty() { "<none>" } else { &target },
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
    text_units: usize,
) {
    if log_row {
        tracing::info!(
            target: "squad",
            event = "squad.chat",
            outcome = "rejected",
            reason,
            account_id,
            player_id,
            entity_id,
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

/// `chatJoin(WSTRING channelName, WSTRING password)` — acknowledged (channels
/// are auto-joined).
pub(super) fn handle_chat_join(payload: &[u8], addr: SocketAddr) {
    // chatJoin(WSTRING channelName, WSTRING password)
    let (channel_name, offset) = match read_wstring(payload, 0) {
        Ok(v) => v,
        Err(_) => return,
    };
    let (_password, _) = match read_wstring(payload, offset) {
        Ok(v) => v,
        Err(_) => return,
    };
    tracing::debug!(%addr, channel_name, "chatJoin -- acknowledged (channels auto-joined)");
}

/// `chatLeave(UINT8 channelId)` — acknowledged.
pub(super) fn handle_chat_leave(payload: &[u8], addr: SocketAddr) {
    // chatLeave(UINT8 channelId)
    let channel_id = if !payload.is_empty() { payload[0] } else { 0 };
    tracing::debug!(%addr, channel_id, "chatLeave -- acknowledged");
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
            account_id = c.account_id,
            entity_id = c.player_entity_id,
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
    let (player_id, account_id, entity_id) = feedback
        .connected
        .lock()
        .unwrap()
        .get(&addr)
        .map(|c| (c.active_player_id, Some(c.account_id), c.player_entity_id))
        .unwrap_or_default();
    tracing::warn!(
        target: "chat",
        event = "chat.away_rejected",
        %addr,
        player_id,
        account_id,
        entity_id,
        kind,
        reason = reject.reason(),
        "away message refused: it breaks the chat text rules; previous state kept",
    );
    send_feedback_line(feedback, addr, AWAY_BAD_CHARACTER_TEXT).await;
    false
}
