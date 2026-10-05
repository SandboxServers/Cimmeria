use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_base_session::base::plugin;
use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::BaseToCellMsg;

/// The message id of SGWPlayer base method 0 (`0xC0 | index`).
pub(crate) const BASE_METHOD_ID: u8 = 0xC0;

use super::feedback;
use super::ConnectedClientState;

mod chat;
mod chat_gates;
mod communicator_unsupported;
mod diagnostics;
mod duel;
mod ignore;
mod organization;
mod organization_squad;
mod session;
mod tell;

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
    pub(crate) const GM: u8 = 0x01;
    /// Set when the speaker has a non-empty DND auto-reply message.
    /// Python parity: `if player.dndMessage is not None`.
    pub(crate) const DND: u8 = 0x04;
}

/// SGWPlayer base-method message IDs we currently handle explicitly.
///
/// The client also sends protocol-level messages such as `versionInfoRequest`
/// and `elementDataRequest` while in-world. Those are dispatched separately in
/// `connect_loop.rs` and must not be treated as SGWPlayer methods.
pub(crate) mod sgw_player_base {
    pub(crate) const CHAT_JOIN: u8 = 0xC0;
    pub(crate) const CHAT_LEAVE: u8 = 0xC1;
    pub(crate) const SEND_PLAYER_COMMUNICATION: u8 = 0xC2;
    pub(crate) const CHAT_SET_AFK: u8 = 0xC3;
    pub(crate) const CHAT_SET_DND: u8 = 0xC4;
    /// `chatIgnore(WSTRING aPlayerName, UINT8 aFlag)`: 1 adds the name to
    /// the caller's contact-list Ignore list, 0 removes it
    /// (`Communicator.def`). Handled in `dispatch/ignore.rs`.
    pub(crate) const CHAT_IGNORE: u8 = 0xC5;
    /// Communicator base methods 0xC6-0xCE, not implemented on this server.
    /// Each answers with its own feedback line (SS-C3, D-SS26), in
    /// `dispatch/communicator_unsupported.rs`.
    pub(crate) const CHAT_FRIEND: u8 = 0xC6;
    pub(crate) const CHAT_LIST: u8 = 0xC7;
    pub(crate) const CHAT_MUTE: u8 = 0xC8;
    pub(crate) const CHAT_KICK: u8 = 0xC9;
    pub(crate) const CHAT_OP: u8 = 0xCA;
    pub(crate) const CHAT_BAN: u8 = 0xCB;
    pub(crate) const CHAT_PASSWORD: u8 = 0xCC;
    pub(crate) const PETITION: u8 = 0xCD;
    pub(crate) const ANNOUNCE_PETITION: u8 = 0xCE;
    /// OrganizationMember base methods 0xCF-0xD2 (`organizationInvite`,
    /// `organizationInviteByType`, `organizationKick`,
    /// `organizationRankChange`). Handled in `dispatch/organization.rs`.
    pub(crate) const ORGANIZATION_INVITE: u8 =
        cimmeria_wire::base::organization::ORGANIZATION_INVITE;
    pub(crate) const ORGANIZATION_RANK_CHANGE: u8 =
        cimmeria_wire::base::organization::ORGANIZATION_RANK_CHANGE;
    /// SGWPlayer.elementDataRequest(INT32 categoryId, INT32 key): an
    /// in-world cache miss (`ClientCache.def`; the old table's UINT16
    /// category was wrong, which is why logged keys looked shifted by 16
    /// bits). Served since #840: `account_arms` routes it to
    /// `cooked_data::handle_element_data_request` before this dispatch,
    /// because serving needs the resource cache.
    pub(crate) const ELEMENT_DATA_REQUEST: u8 = 0xD5;
    /// SGWPlayer.logOff(INT8 Disconnect) — 0=return to char select, 1=full exit
    pub(crate) const LOG_OFF: u8 = 0xD6;
    /// SGWPlayer.cancelLogOff() — cancel pending logoff timer
    pub(crate) const CANCEL_LOG_OFF: u8 = 0xD7;
    pub(crate) const ON_CLIENT_READY: u8 = 0xD8;
    /// SGWPlayer.perfStats(12 × FLOAT) — client-side perf telemetry
    /// (FPS, frame time variance, etc.) pushed every ~15 s. Sink-only
    /// on the server: there is no actionable response, no persistence,
    /// and no metric extraction wired yet. Acknowledged as a known
    /// handler so the unhandled-WARN catch-all stays alert-worthy for
    /// genuinely missing methods. If we later want this telemetry on
    /// SigNoz, the right entry point is to parse the 12 floats here
    /// and emit a metric — until then, the DEBUG line is enough to
    /// confirm the client is still ticking.
    pub(crate) const PERF_STATS: u8 = 0xDD;
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
    fields(
        peer = %addr,
        msg_id,
        msg_name = cimmeria_wire::names::server_msg_name(msg_id),
        method_name = cimmeria_wire::names::player_inbound_method(msg_id, payload),
        payload_len = payload.len()
    ),
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
    // A plugin-owned base method (#962 step 5) goes to the plugin that
    // registered it; every other index falls through to the static arms
    // below. The registry is the one the session was admitted under. The
    // entity-type gate (Account vs SGWPlayer) ran before this call, in
    // `connect_loop`, so a plugin handler sits behind it like any arm.
    if let Some(method_index) = msg_id.checked_sub(BASE_METHOD_ID) {
        let plugins = plugin::session_plugins(connected, addr);
        if let Some(handler) = plugins.base_method(method_index) {
            handler(plugin::BaseMethodCall {
                addr,
                method_index,
                args: payload,
                player_name,
                key,
                entity_manager,
                ctx: plugin::BaseCtx {
                    db_pool,
                    cell_tx,
                    transport,
                    connected,
                    entity_to_addr,
                },
            })
            .await;
            return Ok(());
        }
    }

    match msg_id {
        sgw_player_base::SEND_PLAYER_COMMUNICATION => {
            chat::handle_send_player_communication(
                payload,
                player_name,
                addr,
                transport,
                connected,
                chat::ChatRoutes {
                    cell_tx,
                    entity_to_addr,
                    db_pool,
                },
            )
            .await;
        }

        sgw_player_base::CHAT_JOIN => {
            chat::handle_chat_join(payload, addr, transport, connected).await;
        }

        sgw_player_base::CHAT_LEAVE => {
            chat::handle_chat_leave(payload, addr, transport, connected).await;
        }

        sgw_player_base::CHAT_SET_AFK => {
            let feedback = feedback::FeedbackCtx {
                transport,
                connected,
            };
            chat::handle_chat_set_afk(payload, addr, &feedback).await;
        }

        sgw_player_base::CHAT_SET_DND => {
            let feedback = feedback::FeedbackCtx {
                transport,
                connected,
            };
            chat::handle_chat_set_dnd(payload, addr, &feedback).await;
        }

        sgw_player_base::CHAT_IGNORE => {
            ignore::handle_chat_ignore(
                payload,
                addr,
                transport,
                connected,
                entity_to_addr,
                cell_tx,
                db_pool,
                std::time::Instant::now(),
            )
            .await;
        }

        sgw_player_base::CHAT_FRIEND..=sgw_player_base::ANNOUNCE_PETITION => {
            let feedback = feedback::FeedbackCtx {
                transport,
                connected,
            };
            communicator_unsupported::handle_unsupported_communicator(
                msg_id,
                payload.len(),
                &feedback,
                addr,
                std::time::Instant::now(),
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

        sgw_player_base::PERF_STATS => {
            diagnostics::handle_perf_stats(payload, addr);
        }

        sgw_player_base::ORGANIZATION_INVITE..=sgw_player_base::ORGANIZATION_RANK_CHANGE => {
            organization::handle_org_base_method(
                msg_id,
                payload,
                addr,
                transport,
                connected,
                entity_to_addr,
                cell_tx,
                db_pool,
            )
            .await;
        }

        // sendDuelChallenge(WSTRING playerName, INT8 squadDuel) (SS-D1).
        cimmeria_wire::base::duel::SEND_DUEL_CHALLENGE => {
            duel::handle_send_duel_challenge(payload, addr, transport, connected, cell_tx).await;
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
                base_method_name =
                    cimmeria_wire::names::player_base_method(u16::from(msg_id.wrapping_sub(0xC0))),
                msg_name = cimmeria_wire::names::server_msg_name(msg_id),
                method_name = cimmeria_wire::names::player_inbound_method(msg_id, payload),
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
