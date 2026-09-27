//! Chat message distribution for the CellService.
//!
//! Handles spatial chat channels (say, emote, yell) by broadcasting
//! `onPlayerCommunication` to witnesses in the sender's Area of Interest.
//!
//! Reference: `python/cell/SGWPlayer.py:processPlayerCommunication()`
//!
//! - [`spatial`]: the say/emote/yell broadcast to the speaker's player witnesses;
//! - [`feedback`]: the one-line refusal sent back to the speaker.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use crate::cell::console;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

mod feedback;
mod spatial;

use feedback::send_channel_feedback;
use spatial::broadcast_to_witnesses;

// ── Channel IDs and the onPlayerCommunication serializer ──────────────────
//
// The `EChannel` ids and `serialize_on_player_communication` are wire
// contract and live in `cimmeria-wire`; re-exported at their old paths.

pub use cimmeria_wire::cell::chat::{
    CHAN_COMMAND, CHAN_EMOTE, CHAN_FEEDBACK, CHAN_OFFICER, CHAN_SAY, CHAN_SERVER, CHAN_SPLASH,
    CHAN_SQUAD, CHAN_TEAM, CHAN_TELL, CHAN_YELL,
};

pub(crate) use cimmeria_wire::cell::chat::serialize_on_player_communication;

// ── onPlayerCommunication client method index ──────────────────────────────

/// Communicator interface ClientMethod: onPlayerCommunication
/// Flat index 28 in SGWPlayer ClientMethods.
const ON_PLAYER_COMMUNICATION: u16 = 28;

/// The tracing target every chat log used before the split into `chat/`.
/// Submodules pass it explicitly so SigNoz `scope_name` filters and
/// `OTEL_FILTER` pins keep matching the pre-split target.
const CHAT_LOG_TARGET: &str = "cimmeria_cell_console::cell::console::chat";

// ── Chat distribution ──────────────────────────────────────────────────────

/// Handle a chat message from a player entity.
///
/// For spatial channels (say, emote, yell), broadcasts to all witnesses
/// of the sender's entity. Each witness receives `onPlayerCommunication`.
///
/// Reference: `python/cell/SGWPlayer.py:processPlayerCommunication()`
/// - say/emote/yell: broadcast to witnesses
/// - Client does NOT echo say (channel 0) — server must send it back
/// - Client DOES echo emote/yell — but Python sends it anyway (no harm)
/// `text_len` is recorded but the message body itself is intentionally
/// excluded from the span — chat content is user-private and shouldn't
/// land in the SigNoz log/trace store. Operators get "who sent how
/// many bytes on which channel" without sniffing message bodies.
#[tracing::instrument(
    name = "chat.send",
    level = "info",
    skip_all,
    fields(entity_id, channel, text_len = text.len()),
)]
pub async fn handle_chat_message(
    entity_id: u32,
    speaker_name: &str,
    speaker_flags: u8,
    channel: u8,
    text: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    // GM `.`-console interception. The 2009 client forwards `.`-prefixed
    // input as an ordinary CHAN_SAY chat message rather than eating it (unlike
    // `/`, which the client consumes). When the sender is a GM, we consume the
    // line as a dev/authoring console command and never broadcast it to other
    // players; a non-GM's `.`-text falls through to normal chat. Auth is on the
    // server-side `access_level`, never a client-asserted byte.
    if channel == CHAN_SAY && text.starts_with('.') {
        let access_level = space_mgr
            .get_entity(entity_id)
            .map_or(0, |e| e.access_level);
        if console::is_gm(access_level) {
            console::handle_console_command(entity_id, text, tx, space_mgr, engine).await;
            return;
        }
        // A non-GM line that names a registered command is refused, not
        // broadcast: a player (or a GM demoted mid-session) typing
        // `.giveability 2826` must not echo the command to everyone nearby,
        // and the press needs visible feedback. Other `.`-text ("...",
        // ".hello") is ordinary chat and falls through to broadcast.
        if console::refuse_non_gm_command(entity_id, text, tx, space_mgr).await {
            return;
        }
    }

    match channel {
        CHAN_SAY | CHAN_EMOTE | CHAN_YELL => {
            broadcast_to_witnesses(
                entity_id,
                speaker_name,
                speaker_flags,
                channel,
                text,
                tx,
                space_mgr,
            )
            .await;
        }
        CHAN_SERVER => {
            // `python/base/Chat.py::ChatChannelManager.__init__` creates the
            // "server" channel with `CHANNEL_FLAG_DisallowPlayerMessages` --
            // it is a system-broadcast channel (server -> all players), never
            // a player -> player one. The legacy `sendPlayerMessage` rejected
            // this with only a server-side `warn()` and no client-visible
            // reply, which is exactly the silent-drop this tester tripped
            // over ("i dont see my own message sent to the server channel").
            // Per the "every button press gets visible feedback" project
            // rule, tell the sender why instead of dropping it on the floor.
            tracing::debug!(
                entity_id,
                channel,
                "Chat: player attempted to speak on the system-only server channel"
            );
            send_channel_feedback(
                entity_id,
                "The server channel is for system messages only -- players cannot post here.",
                tx,
            )
            .await;
        }
        _ => {
            // Every other registered channel (team/squad/command/officer/
            // tell) was never distributed on the cell in the legacy server
            // either: `python/cell/SGWPlayer.py::processPlayerCommunication`
            // only special-cases say/emote/yell and falls through to
            // `self.onError("Speaking on channel %d is not supported yet!")`
            // for anything else, which itself is a client-visible
            // `onPlayerCommunication` reply, not a silent drop. Match that
            // shape here instead of only logging.
            tracing::debug!(
                entity_id,
                channel,
                "Chat channel not handled by CellService -- feedback sent to sender"
            );
            send_channel_feedback(
                entity_id,
                &format!("Speaking on channel {channel} is not supported yet!"),
                tx,
            )
            .await;
        }
    }
}

#[cfg(test)]
mod tests;
