//! SGWPlayer base-method chat handlers.
//!
//! Extracted from `dispatch.rs` — the chat-family arms of
//! `dispatch_sgw_player_base_method`: `sendPlayerCommunication`, `chatJoin`,
//! `chatLeave`, `chatSetAFKMessage`, and `chatSetDNDMessage`. Pure code
//! movement; each function carries the exact arm body it replaced.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::read_wstring;

use super::super::feedback::{send_feedback_line, FeedbackCtx};
use cimmeria_entity::organization::org_text::{validate, TextField, TextReject};

use super::super::rate_limit::limits::{CHAT_EXEMPT_ACCESS_LEVEL, MAX_CHAT_TEXT_UNITS};
use super::super::rate_limit::{log_exceeded, RateActor, RateCategory, RateDecision};
use super::super::ConnectedClientState;
use super::speaker_flags;

const MAX_DND_MESSAGE_CHARS: usize = 128;

/// `sendPlayerCommunication(UINT8 channel, WSTRING target, WSTRING text)`.
///
/// Routes spatial channels (say/emote/yell) to the CellService with the
/// computed `speaker_flags`, after two gates that run here, before the cell
/// ever sees the line:
///
/// 1. the per-player chat bucket (D-SS14; GameMaster and above exempt);
/// 2. the D-SS12 text rules: at most [`MAX_CHAT_TEXT_UNITS`] UTF-16 units and
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
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    send_player_communication_at(
        payload,
        player_name,
        addr,
        transport,
        connected,
        cell_tx,
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
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    now: Instant,
) {
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
    let (player_eid, speaker_flags_value, player_id, account_id, decision) = {
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
                    };
                    log_exceeded(RateCategory::Chat, actor, notify, &c.rate_limits, now);
                }
                (
                    c.player_entity_id,
                    flags,
                    c.active_player_id,
                    c.account_id,
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

    if let RateDecision::Limited { notify } = decision {
        if notify {
            send_feedback_line(&feedback, addr, RateCategory::Chat.feedback_text()).await;
        }
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
            channel,
            text_units = text.encode_utf16().count(),
            max_units = MAX_CHAT_TEXT_UNITS,
            reason = reject.reason(),
            detail = %reject,
            "sendPlayerCommunication rejected: text breaks the chat text rules, not forwarded",
        );
        send_feedback_line(&feedback, addr, chat_reject_text(&reject)).await;
        return;
    }

    // Logged only once both gates pass: every field here is client-supplied,
    // so a flooding client must not get one INFO row per packet.
    tracing::info!(
        %addr,
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

/// `chatSetAFKMessage` — intentionally log-only.
pub(super) fn handle_chat_set_afk(addr: SocketAddr) {
    // AFK is intentionally log-only. AFK is NOT a speaker flag:
    // `entities/defs/enumerations.xml` has no `SPEAKER_AFK`
    // token, and `python/base/Chat.py::getSpeakerFlags` only
    // checks `accessLevel > 0` / `dndMessage is not None`. In
    // Python, `chatSetAFKMessage` only affects the
    // auto-reply-tell path in `sendPlayerMessage`, which is a
    // separate feature we have not ported yet.
    tracing::debug!(
        %addr,
        "chatSetAFKMessage -- acknowledged (auto-reply not yet implemented)",
    );
}

/// `chatSetDNDMessage(WSTRING message)`.
///
/// Mirrors `python/base/SGWPlayer.py::chatSetDNDMessage`: an empty or 1-char
/// message clears DND; anything longer sets it. The stored text is truncated
/// to 128 Unicode scalar values.
pub(super) fn handle_chat_set_dnd(
    payload: &[u8],
    addr: SocketAddr,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
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
