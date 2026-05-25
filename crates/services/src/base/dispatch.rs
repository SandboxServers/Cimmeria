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
                    // If either send fails on logoff, the cell leaks
                    // the entity in its space_manager. warn! so a
                    // memory leak / "ghost player" report can be
                    // traced back to the logoff path.
                    //
                    // Failure mode: this is `mpsc::Sender::send().await`
                    // (NOT `try_send`), so it backpressures rather than
                    // failing on a full channel. The only Err path is
                    // the receiver having been dropped — i.e. cell
                    // service is shut down. That makes WARN safe at
                    // any load (no spam during normal backpressure).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
    use tracing::Level;

    /// logOff with a closed cell→base channel must surface
    /// the dropped DisconnectEntity / DestroyEntity sends so a "ghost
    /// player in space_manager" report can be traced back to the
    /// logoff path. Reverting either `if let Err` to `let _ = tx.send`
    /// trips this guard.
    ///
    /// Uses `disconnect=0` (return-to-char-select) so the path runs
    /// through both cell-tx sends and then the RESET_ENTITIES wire
    /// emit. TestTransport captures the wire send; the cell-tx sends
    /// fail because the receiver was dropped.
    #[tokio::test]
    async fn logoff_warns_when_cell_to_base_channel_closed_for_both_sends() {
        let capture = LogCapture::install();

        let addr: SocketAddr = "127.0.0.1:54321".parse().unwrap();
        let entity_id: u32 = 4242;
        let key = [0u8; 32];

        let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());

        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(entity_id);
        let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
        let entity_manager = Arc::new(Mutex::new(EntityManager::new()));

        // CLOSED channel.
        let (tx, rx) = mpsc::channel::<BaseToCellMsg>(8);
        drop(rx);
        let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = Some(tx);

        // payload[0] == 0 => return-to-char-select branch. Skips the
        // loggedOff packet but still runs the RESET_ENTITIES path,
        // which uses transport.send_to (TestTransport swallows it).
        let payload = [0u8];

        dispatch_sgw_player_base_method(
            sgw_player_base::LOG_OFF,
            &payload,
            &None,
            addr,
            &transport,
            key,
            &connected,
            &entity_manager,
            &cell_tx,
            &entity_to_addr,
        )
        .await
        .expect("logOff dispatch should not propagate Err for closed cell_tx");

        // Both sends fail independently — assert each produces its own
        // WARN so a partial revert (only one of two) is also caught.
        assert!(
            capture
                .find_message(Level::WARN, "logOff: DisconnectEntity send failed")
                .is_some(),
            "DisconnectEntity WARN missing. Captured: {:#?}",
            capture.all()
        );
        assert!(
            capture
                .find_message(Level::WARN, "logOff: DestroyEntity send failed")
                .is_some(),
            "DestroyEntity WARN missing. Captured: {:#?}",
            capture.all()
        );

        // entity_to_addr cleanup still runs regardless of cell_tx failure.
        assert!(
            entity_to_addr.lock().unwrap().get(&entity_id).is_none(),
            "logOff must still clean up entity_to_addr even when cell_tx is closed"
        );
    }
}
