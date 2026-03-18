use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;
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
    socket: &Arc<UdpSocket>,
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
                // Compute speaker_flags from session state
                // Bit 0 = AFK, Bit 1 = DND, Bit 2 = GM (access_level >= 2)
                let speaker_flags = {
                    let clients = connected.lock().unwrap();
                    clients.get(&addr).map_or(0u8, |c| {
                        let mut flags = 0u8;
                        if c.is_afk { flags |= 0x01; }
                        if c.is_dnd { flags |= 0x02; }
                        if c.access_level >= 2 { flags |= 0x04; }
                        flags
                    })
                };

                if let Some(ref tx) = cell_tx {
                    let _ = tx.send(BaseToCellMsg::ChatMessage {
                        entity_id: player_eid,
                        speaker_name: speaker.to_string(),
                        speaker_flags,
                        channel,
                        text,
                    }).await;
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

        sgw_player_base::CHAT_SET_AFK => {
            // setAFK(UINT8 enabled)
            let enabled = payload.first().copied().unwrap_or(0) != 0;
            tracing::debug!(%addr, enabled, "setAFK");
            let mut clients = connected.lock().unwrap();
            if let Some(c) = clients.get_mut(&addr) {
                c.is_afk = enabled;
            }
        }

        sgw_player_base::CHAT_SET_DND => {
            // setDND(UINT8 enabled)
            let enabled = payload.first().copied().unwrap_or(0) != 0;
            tracing::debug!(%addr, enabled, "setDND");
            let mut clients = connected.lock().unwrap();
            if let Some(c) = clients.get_mut(&addr) {
                c.is_dnd = enabled;
            }
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
    let _ = (socket, key, entity_manager, entity_to_addr);

    Ok(())
}
