//! `ENABLE_ENTITIES` (0x08) dispatch.
//!
//! Two distinct phases share this opcode and are differentiated by whether
//! `pending_player_entity_id` is set:
//!
//! - **Initial char list**: first ENABLE_ENTITIES after connect. Sends the
//!   character list using the previously allocated Account entity.
//! - **Create player**: second ENABLE_ENTITIES, after `playCharacter`'s
//!   RESET_ENTITIES. Sends `CREATE_BASE_PLAYER` + `onClientMapLoad` and
//!   waits for the client's `mapLoaded`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_char_list, build_create_player};

use super::super::ConnectedClientState;
use super::super::character::query_character_list;
use super::super::helpers::{drain_acks_and_seq, get_account_entity_id};

/// Handle `ENABLE_ENTITIES` (0x08) -- dispatches char list or create-player step.
///
/// - **Char list** (no `pending_player_entity_id`): First ENABLE_ENTITIES after connect.
///   Creates the Account entity and sends the character list, then starts tick-sync.
/// - **Create player** (has `pending_player_entity_id`): After world entry RESET_ENTITIES.
///   Sends `CREATE_BASE_PLAYER` + `onClientMapLoad`. The client loads terrain and
///   then sends `mapLoaded`, which triggers the enter-world step.
pub(crate) async fn handle_enable_entities(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    _entity_manager: &Arc<Mutex<EntityManager>>,
    _cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    _entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Check if we have a pending world entry (create-player step).
    let pending = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            match (c.pending_player_entity_id.take(), c.pending_world_entry.take()) {
                (Some(eid), Some(entry)) => Some((eid, entry)),
                _ => None,
            }
        } else {
            None
        }
    };

    // Also retrieve the pending player load data
    let pending_load = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_player_load_data.take()
        } else {
            None
        }
    };

    if let Some((_eid, entry_info)) = pending {
        // -- Create player step: CREATE_BASE_PLAYER + onClientMapLoad --
        // Send only the base entity and terrain load notification. The client
        // will load geometry and respond with `mapLoaded` (cell method index 25,
        // msg_id 0x99). The enter-world step (viewport + cell + position + entity data)
        // is sent in response to that message.
        let (acks, seq) = drain_acks_and_seq(connected, addr)?;

        tracing::info!(
            %addr,
            player_entity_id = entry_info.player_entity_id,
            space_id = entry_info.space_id,
            seq,
            "Create player: sending CREATE_BASE_PLAYER + onClientMapLoad (waiting for mapLoaded)"
        );

        let pkt = build_create_player(&key, seq, &acks, &entry_info);
        socket.send_to(&pkt, addr).await?;

        // Store world entry + player data for the enter-world step (triggered by mapLoaded)
        {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_map_loaded = Some(entry_info);
            c.pending_player_load_data = pending_load;
            c.pending_client_ready = None;
        }
    }

        tracing::info!(%addr, "Create player complete -- waiting for client mapLoaded");
        return Ok(());
    }

    // -- Phase 4: initial entity creation -- send Account entity + char list --
    // Account entity was already allocated in Phase 3 (handle_login).

    // Guard: only send once per connection.
    {
        let mut clients = connected
            .lock()
            .map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            if c.char_list_sent {
                return Ok(()); // already sent
            }
            c.char_list_sent = true;
        } else {
            tracing::warn!(%addr, "enable_entities: addr not in connected map");
            return Ok(());
        }
    }

    // Query characters from DB
    let characters = query_character_list(db_pool, account_id).await;
    let account_eid = get_account_entity_id(connected, addr)?;

    tracing::info!(
        %addr,
        account_entity_id = account_eid,
        count = characters.len(),
        "Phase 4: sending character list ({})",
        if characters.is_empty() { "creation screen" } else { "select screen" }
    );

    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    let pkt = build_char_list(&key, seq, &acks, &characters, account_eid);
    tracing::trace!(%addr, len = pkt.len(), seq, hex = %super::super::helpers::to_hex(&pkt), "UDP_OUT char_list");
    socket.send_to(&pkt, addr).await?;

    tracing::info!(%addr, "Phase 4 complete -- char list sent");

    Ok(())
}
