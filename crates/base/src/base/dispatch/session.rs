//! SGWPlayer base-method session handlers.
//!
//! Extracted from `dispatch.rs` — the session-lifecycle arms of
//! `dispatch_sgw_player_base_method`: `logOff` and `cancelLogOff`. Pure code
//! movement; each function carries the exact arm body it replaced.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;

use super::super::ConnectedClientState;

/// `SGWPlayer.logOff(INT8 Disconnect)` — 0=return to char select, 1=full exit.
pub(super) async fn handle_log_off(
    payload: &[u8],
    addr: SocketAddr,
    transport: &Arc<dyn Transport>,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let disconnect = if !payload.is_empty() { payload[0] } else { 0 };
    tracing::info!(%addr, disconnect, "SGWPlayer.logOff");

    // Snapshot entity info before cleanup. Capture the session's wire-encryption
    // version (the logoff / reset-entities packets below must be built with the
    // version this session speaks) and the player_name (needed for the
    // contact-list offline fanout).
    //
    // Both variants unlist the character from the online name index here:
    // a full exit keeps the session (and `player_name`) until the client's
    // disconnect reaps it, but the character has already left the world and
    // must not be reachable by tells or duel challenges in that window.
    let (entity_id, enc_version, player_name) = {
        let mut clients = connected.lock().unwrap();
        match clients.get_mut(&addr) {
            Some(c) => {
                let path = if disconnect != 0 {
                    "logoff_full_exit"
                } else {
                    "logoff_character_select"
                };
                cimmeria_base_session::base::player_index::log_unlisted(addr, c, path);
                c.listed_online = false;
                (c.player_entity_id, c.enc_version, c.player_name.clone())
            }
            None => (None, Default::default(), None),
        }
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

        // Logging off drops any queued crafting without consuming it.
        cimmeria_base_session::base::crafting::session::drop_player_inductions(
            entity_id,
            cimmeria_base_session::base::crafting::session::DropReason::Logout,
            "log_off",
        );
    }

    // Fan out offline status to contact-list watchers. Fire-and-forget via
    // tokio::spawn so the logout response (loggedOff / RESET_ENTITIES) is
    // not blocked on the DB query + per-watcher sends.
    if let Some(name) = player_name {
        let pool_c = db_pool.clone();
        let transport_c = transport.clone();
        let connected_c = connected.clone();
        let entity_to_addr_c = entity_to_addr.clone();
        tokio::spawn(async move {
            crate::base::contact_list::handlers::fanout_login_status(
                &name,
                false, // offline
                &pool_c,
                &transport_c,
                &connected_c,
                &entity_to_addr_c,
            )
            .await;
        });
    }

    if disconnect != 0 {
        // Full exit: send loggedOff system message (msg_id 0x06) and let client disconnect
        tracing::info!(%addr, "logOff: full exit — sending loggedOff");
        let (acks, seq) = super::super::helpers::drain_acks_and_seq(connected, addr)?;
        let pkt = crate::mercury::build_logged_off(&key, seq, &acks, enc_version);
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
                c.player_alignment = None;
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
                // DND is per-character state. Without this reset,
                // char A's /dnd would leak into char B on the
                // same connection — every subsequent
                // `sendPlayerCommunication` would carry
                // `SPEAKER_DND` until char B's user toggled DND
                // explicitly. Mirrors the other per-character
                // fields cleared on return-to-character-select.
                c.dnd_message = None;
                // AFK and the Ignore cache are per-character too; the next
                // character's `onClientReady` loads its own list.
                c.afk_message = None;
                c.ignore = Default::default();
            }
        }

        // Send RESET_ENTITIES to tear down the world
        let (acks, seq) = super::super::helpers::drain_acks_and_seq(connected, addr)?;
        let pkt = crate::mercury::build_reset_entities(&key, seq, &acks, enc_version);
        transport.send_to(&pkt, addr).await?;

        // The client responds with ENABLE_ENTITIES, which triggers the
        // char list flow (same as initial login). The char_list_sent flag
        // was cleared above so handle_enable_entities will re-send.
    }

    Ok(())
}

/// `SGWPlayer.cancelLogOff()` — cancel pending logoff timer. Acknowledged.
pub(super) fn handle_cancel_log_off(addr: SocketAddr) {
    tracing::debug!(%addr, "SGWPlayer.cancelLogOff — acknowledged");
}
