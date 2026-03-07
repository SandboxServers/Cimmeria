use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::read_wstring;

use super::ConnectedClientState;

/// SGWPlayer flattened EXPOSED BaseMethod indices.
///
/// After world entry, the client controls an SGWPlayer entity. Base method calls
/// use `msg_id = flat_index + 0xC0`. The ordering is: interface methods first
/// (in Implements order from SGWPlayer.def), then own methods.
///
/// ## Communicator interface (indices 0-14, msg_id 0xC0-0xCE)
/// | Idx | Method | Args |
/// |-----|--------|------|
/// | 0 | chatJoin | WSTRING channelName, WSTRING password |
/// | 1 | chatLeave | UINT8 channelId |
/// | 2 | sendPlayerCommunication | UINT8 channel, WSTRING target, WSTRING text |
/// | 3 | chatSetAFKMessage | WSTRING message |
/// | 4 | chatSetDNDMessage | WSTRING message |
/// | 5-14 | chatIgnore..announcePetition | (various, stub) |
pub(crate) mod sgw_player_base {
    pub const CHAT_JOIN: u8 = 0xC0;
    pub const CHAT_LEAVE: u8 = 0xC1;
    pub const SEND_PLAYER_COMMUNICATION: u8 = 0xC2;
    pub const CHAT_SET_AFK: u8 = 0xC3;
    pub const CHAT_SET_DND: u8 = 0xC4;
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
                if let Some(ref tx) = cell_tx {
                    let _ = tx.send(BaseToCellMsg::ChatMessage {
                        entity_id: player_eid,
                        speaker_name: speaker.to_string(),
                        speaker_flags: 0, // TODO: compute from AFK/DND/GM status
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

        sgw_player_base::CHAT_SET_AFK | sgw_player_base::CHAT_SET_DND => {
            tracing::debug!(%addr, msg_id = format_args!("{:#04x}", msg_id), "Chat status update -- acknowledged");
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
