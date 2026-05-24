use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::read_wstring;

use super::ConnectedClientState;

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
    /// SGWPlayer.logOff(INT8 Disconnect) — 0=return to char select, 1=full exit
    pub const LOG_OFF: u8 = 0xD6;
    /// SGWPlayer.cancelLogOff() — cancel pending logoff timer
    pub const CANCEL_LOG_OFF: u8 = 0xD7;
    pub const ON_CLIENT_READY: u8 = 0xD8;
}

/// Dispatch an SGWPlayer base method call (after world entry).
///
/// The entity type switches from Account to SGWPlayer when the player enters the
/// world. The same msg_id values (0xC0+) map to different methods.
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match msg_id {
        sgw_player_base::SEND_PLAYER_COMMUNICATION => {
            // sendPlayerCommunication(UINT8 channel, WSTRING target, WSTRING text)
            if payload.is_empty() {
                return Ok(());
            }
            let channel = payload[0];
            let mut offset = 1;

            // Parse target (WSTRING)
            let (target, new_offset) = match read_wstring(payload, offset) {
                Ok(v) => v,
                Err(_) => return Ok(()),
            };
            offset = new_offset;

            // Parse text (WSTRING)
            let (text, _) = match read_wstring(payload, offset) {
                Ok(v) => v,
                Err(_) => return Ok(()),
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

            // Route to CellService for spatial channels (say/emote/yell)
            let player_eid = {
                let clients = connected.lock().unwrap();
                clients.get(&addr).and_then(|c| c.player_entity_id)
            };

            if let Some(player_eid) = player_eid {
                if let Some(ref tx) = cell_tx {
                    let _ = tx
                        .send(BaseToCellMsg::ChatMessage {
                            entity_id: player_eid,
                            speaker_name: speaker.to_string(),
                            speaker_flags: 0, // TODO: compute from AFK/DND/GM status
                            channel,
                            text,
                        })
                        .await;
                }
            }
        }

        sgw_player_base::CHAT_JOIN => {
            // chatJoin(WSTRING channelName, WSTRING password)
            let (channel_name, offset) = match read_wstring(payload, 0) {
                Ok(v) => v,
                Err(_) => return Ok(()),
            };
            let (_password, _) = match read_wstring(payload, offset) {
                Ok(v) => v,
                Err(_) => return Ok(()),
            };
            tracing::debug!(%addr, channel_name, "chatJoin -- acknowledged (channels auto-joined)");
        }

        sgw_player_base::CHAT_LEAVE => {
            // chatLeave(UINT8 channelId)
            let channel_id = if !payload.is_empty() { payload[0] } else { 0 };
            tracing::debug!(%addr, channel_id, "chatLeave -- acknowledged");
        }

        sgw_player_base::CHAT_SET_AFK | sgw_player_base::CHAT_SET_DND => {
            tracing::debug!(%addr, msg_id = format_args!("{:#04x}", msg_id), "Chat status update -- acknowledged");
        }

        sgw_player_base::LOG_OFF => {
            let disconnect = if !payload.is_empty() { payload[0] } else { 0 };
            tracing::info!(%addr, disconnect, "SGWPlayer.logOff");

            // Get entity info before cleanup
            let entity_id = {
                let clients = connected.lock().unwrap();
                clients.get(&addr).and_then(|c| c.player_entity_id)
            };

            if let Some(entity_id) = entity_id {
                // Tell CellService to disconnect and destroy the entity
                if let Some(ref tx) = cell_tx {
                    // Issue #304: if either send fails on logoff, the cell
                    // leaks the entity in its space_manager. warn! so a
                    // memory leak / "ghost player" report can be traced
                    // back to the logoff path.
                    if let Err(e) = tx.send(BaseToCellMsg::DisconnectEntity { entity_id }).await {
                        tracing::warn!(
                            entity_id,
                            "logOff: DisconnectEntity send failed -- cell may leak player state: {e}"
                        );
                    }
                    if let Err(e) = tx.send(BaseToCellMsg::DestroyEntity { entity_id }).await {
                        tracing::warn!(
                            entity_id,
                            "logOff: DestroyEntity send failed -- cell may leak player entity: {e}"
                        );
                    }
                }

                // Remove entity→addr mapping
                entity_to_addr.lock().unwrap().remove(&entity_id);
            }

            if disconnect != 0 {
                // Full exit: send loggedOff system message (msg_id 0x06) and let client disconnect
                tracing::info!(%addr, "logOff: full exit — sending loggedOff");
                let (acks, seq) = super::helpers::drain_acks_and_seq(connected, addr)?;
                let pkt = crate::mercury::build_logged_off(&key, seq, &acks);
                transport.send_to(&pkt, addr).await?;
            } else {
                // Return to character select: reset state and send RESET_ENTITIES + char list
                tracing::info!(%addr, "logOff: returning to character select");

                // Reset client state for character select
                {
                    let mut clients = connected.lock().unwrap();
                    if let Some(c) = clients.get_mut(&addr) {
                        c.player_entity_id = None;
                        c.player_name = None;
                        c.player_level = None;
                        c.player_archetype = None;
                        c.world_name = None;
                        c.player_xp = None;
                        c.player_training_points = None;
                        c.pending_world_entry = None;
                        c.pending_player_load_data = None;
                        c.pending_map_loaded = None;
                        c.pending_client_ready = None;
                        c.pending_player_entity_id = None;
                        c.cached_appearance_args = None;
                        c.cached_tint_args = None;
                        c.world_entry_sent = false;
                        c.char_list_sent = false;
                    }
                }

                // Send RESET_ENTITIES to tear down the world
                let (acks, seq) = super::helpers::drain_acks_and_seq(connected, addr)?;
                let pkt = crate::mercury::build_reset_entities(&key, seq, &acks);
                transport.send_to(&pkt, addr).await?;

                // The client responds with ENABLE_ENTITIES, which triggers the
                // char list flow (same as initial login). The char_list_sent flag
                // was cleared above so handle_enable_entities will re-send.
            }
        }

        sgw_player_base::CANCEL_LOG_OFF => {
            tracing::debug!(%addr, "SGWPlayer.cancelLogOff — acknowledged");
        }

        _ => {
            tracing::trace!(
                %addr,
                msg_id = format_args!("{:#04x}", msg_id),
                base_method_index = msg_id.wrapping_sub(0xC0),
                "Unhandled SGWPlayer base method"
            );
        }
    }

    // Suppress unused warnings for parameters used in future handlers
    let _ = entity_manager;

    Ok(())
}
