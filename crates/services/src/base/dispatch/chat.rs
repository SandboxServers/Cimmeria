//! SGWPlayer base-method chat handlers.
//!
//! Extracted from `dispatch.rs` — the chat-family arms of
//! `dispatch_sgw_player_base_method`: `sendPlayerCommunication`, `chatJoin`,
//! `chatLeave`, `chatSetAFKMessage`, and `chatSetDNDMessage`. Pure code
//! movement; each function carries the exact arm body it replaced.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::read_wstring;

use super::super::ConnectedClientState;
use super::speaker_flags;

/// `sendPlayerCommunication(UINT8 channel, WSTRING target, WSTRING text)`.
///
/// Routes spatial channels (say/emote/yell) to the CellService with the
/// computed `speaker_flags`.
pub(super) async fn handle_send_player_communication(
    payload: &[u8],
    player_name: &Option<String>,
    addr: SocketAddr,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
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

    tracing::info!(
        %addr,
        speaker,
        channel,
        target = if target.is_empty() { "<none>" } else { &target },
        text_len = text.len(),
        "sendPlayerCommunication"
    );

    // Route to CellService for spatial channels (say/emote/yell).
    //
    // Read player_eid + access_level + dnd_message under a single
    // lock acquisition. Computing `speaker_flags` matches
    // `python/base/Chat.py::getSpeakerFlags`:
    //   - SPEAKER_GM  if accessLevel > 0  (Moderator or higher)
    //   - SPEAKER_DND if dndMessage is not None
    // SPEAKER_Petition (0x02) is in the enum but never set by the
    // Python reference, so it is intentionally not computed.
    let (player_eid, speaker_flags_value) = {
        let clients = connected.lock().unwrap();
        match clients.get(&addr) {
            Some(c) => {
                let mut flags: u8 = 0;
                if c.access_level > 0 {
                    flags |= speaker_flags::GM;
                }
                if c.dnd_message.is_some() {
                    flags |= speaker_flags::DND;
                }
                (c.player_entity_id, flags)
            }
            None => (None, 0),
        }
    };

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
/// message clears DND; anything longer sets it.
pub(super) fn handle_chat_set_dnd(
    payload: &[u8],
    addr: SocketAddr,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    // chatSetDNDMessage(WSTRING message)
    //
    // Mirrors `python/base/SGWPlayer.py::chatSetDNDMessage`: an
    // empty or 1-char message clears DND; anything longer sets
    // it. The stored message itself is currently only used as
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
    let mut clients = connected.lock().unwrap();
    if let Some(c) = clients.get_mut(&addr) {
        c.dnd_message = if message.chars().count() > 1 {
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
