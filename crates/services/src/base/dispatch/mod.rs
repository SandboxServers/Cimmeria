use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::BaseToCellMsg;

use super::ConnectedClientState;

mod chat;
mod diagnostics;
pub(crate) mod ignore;
mod session;

/// `ESpeakerFlags` bitfield constants from `entities/defs/enumerations.xml`.
///
/// The wire field is a UINT8 sent in every `onPlayerCommunication` message.
/// Only `SPEAKER_GM` and `SPEAKER_DND` are computed today — matches
/// `python/base/Chat.py::getSpeakerFlags`. `SPEAKER_Petition` (0x02) is
/// declared in the enum but never set by the Python reference, so it is
/// intentionally omitted here.
pub(crate) mod speaker_flags {
    /// Set when the speaker's `access_level > 0` (Moderator or higher).
    /// Python parity: `if player.accessLevel > 0`.
    pub const GM: u8 = 0x01;
    /// Set when the speaker has a non-empty DND auto-reply message.
    /// Python parity: `if player.dndMessage is not None`.
    pub const DND: u8 = 0x04;
}

/// SGWPlayer base-method message IDs we currently handle explicitly.
///
/// The client also sends protocol-level messages such as `versionInfoRequest`
/// and `elementDataRequest` while in-world. Those are dispatched separately in
/// `connect_loop.rs` and must not be treated as SGWPlayer methods.
pub(crate) mod sgw_player_base {
    pub const CHAT_JOIN: u8 = 0xC0;
    pub const CHAT_LEAVE: u8 = 0xC1;
    pub const SEND_PLAYER_COMMUNICATION: u8 = 0xC2;
    pub const CHAT_SET_AFK: u8 = 0xC3;
    pub const CHAT_SET_DND: u8 = 0xC4;
    /// SGWPlayer.chatIgnore(WSTRING playerName, UINT8 flag) — flag 1=add to
    /// the Ignore contact list, 0=remove. The `.def` carries the UINT8 flag
    /// even though the dispatch table lists only the WSTRING.
    pub const CHAT_IGNORE: u8 = 0xC5;
    /// SGWPlayer.elementDataRequest(UINT16 categoryId, UINT32 key) — cache
    /// miss request for a server resource. Same wire shape as the
    /// pre-world-entry 0xC1 cache flow (handled in `cooked_data.rs`),
    /// but routed through the SGWPlayer namespace while the entity is
    /// in-world. Currently a documented no-op — the catalog and
    /// per-key push happens in `cooked_data.rs::send_initial_caches`,
    /// so in-world cache misses are diagnostic rather than a service
    /// the server must fulfil. Demoted from the unhandled-WARN catch-all
    /// so the perfStats-style benign telemetry doesn't trip operator
    /// alerts. See per-method dispatch table in
    /// `docs/protocol/sgwplayer-base-method-dispatch-table.md`.
    pub const ELEMENT_DATA_REQUEST: u8 = 0xD5;
    /// SGWPlayer.logOff(INT8 Disconnect) — 0=return to char select, 1=full exit
    pub const LOG_OFF: u8 = 0xD6;
    /// SGWPlayer.cancelLogOff() — cancel pending logoff timer
    pub const CANCEL_LOG_OFF: u8 = 0xD7;
    pub const ON_CLIENT_READY: u8 = 0xD8;
    /// SGWPlayer.perfStats(12 × FLOAT) — client-side perf telemetry
    /// (FPS, frame time variance, etc.) pushed every ~15 s. Sink-only
    /// on the server: there is no actionable response, no persistence,
    /// and no metric extraction wired yet. Acknowledged as a known
    /// handler so the unhandled-WARN catch-all stays alert-worthy for
    /// genuinely missing methods. If we later want this telemetry on
    /// SigNoz, the right entry point is to parse the 12 floats here
    /// and emit a metric — until then, the DEBUG line is enough to
    /// confirm the client is still ticking.
    pub const PERF_STATS: u8 = 0xDD;
}

/// Dispatch an SGWPlayer base method call (after world entry).
///
/// The entity type switches from Account to SGWPlayer when the player enters the
/// world. The same msg_id values (0xC0+) map to different methods.
///
/// `level = "debug"` — these are per-player-message-rate, similar to
/// the cell dispatch span. The `msg_id` field lets SigNoz group chat
/// vs. logoff vs. ready-state separately.
#[tracing::instrument(
    name = "base.player_method",
    level = "debug",
    skip_all,
    fields(peer = %addr, msg_id, payload_len = payload.len()),
)]
#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch_sgw_player_base_method(
    msg_id: u8,
    payload: &[u8],
    player_name: &Option<String>,
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match msg_id {
        sgw_player_base::SEND_PLAYER_COMMUNICATION => {
            chat::handle_send_player_communication(payload, player_name, addr, connected, cell_tx)
                .await;
        }

        sgw_player_base::CHAT_JOIN => {
            chat::handle_chat_join(payload, addr);
        }

        sgw_player_base::CHAT_LEAVE => {
            chat::handle_chat_leave(payload, addr);
        }

        sgw_player_base::CHAT_SET_AFK => {
            chat::handle_chat_set_afk(addr);
        }

        sgw_player_base::CHAT_SET_DND => {
            chat::handle_chat_set_dnd(payload, addr, connected);
        }

        sgw_player_base::CHAT_IGNORE => {
            ignore::handle_chat_ignore(
                payload,
                addr,
                transport,
                connected,
                cell_tx,
                entity_to_addr,
                db_pool,
            )
            .await;
        }

        sgw_player_base::LOG_OFF => {
            session::handle_log_off(
                payload,
                addr,
                transport,
                key,
                connected,
                cell_tx,
                entity_to_addr,
                db_pool,
            )
            .await?;
        }

        sgw_player_base::CANCEL_LOG_OFF => {
            session::handle_cancel_log_off(addr);
        }

        sgw_player_base::ELEMENT_DATA_REQUEST => {
            diagnostics::handle_element_data_request(payload, addr);
        }

        sgw_player_base::PERF_STATS => {
            diagnostics::handle_perf_stats(payload, addr);
        }

        _ => {
            // Promoted from trace! per #311 (Tier 4 follow-up to #304).
            // Below-ops-filter trace! masked unimplemented client→server
            // method indices: when the client called a base method we had
            // no handler for, the server silently returned Ok and the
            // client's session would behave as if the method had run. A
            // greppable warn turns every unimplemented method into an ops
            // signal that maps directly to a missing handler.
            tracing::warn!(
                %addr,
                msg_id = format_args!("{:#04x}", msg_id),
                base_method_index = msg_id.wrapping_sub(0xC0),
                "Unhandled SGWPlayer base method -- no registered handler for this index; client behaviour may diverge silently"
            );
        }
    }

    // Suppress unused warnings for parameters used in future handlers
    let _ = entity_manager;

    Ok(())
}

#[cfg(test)]
mod tests;
