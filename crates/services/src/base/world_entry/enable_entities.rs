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

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_char_list, build_create_player};

use super::super::character::query_character_list;
use super::super::helpers::{drain_acks_and_seq, get_account_entity_id};
use super::super::ConnectedClientState;

/// Handle `ENABLE_ENTITIES` (0x08) -- dispatches char list or create-player step.
///
/// - **Char list** (no `pending_player_entity_id`): First ENABLE_ENTITIES after connect.
///   Creates the Account entity and sends the character list, then starts tick-sync.
/// - **Create player** (has `pending_player_entity_id`): After world entry RESET_ENTITIES.
///   Sends `CREATE_BASE_PLAYER` + `onClientMapLoad`. The client loads terrain and
///   then sends `mapLoaded`, which triggers the enter-world step.
pub(crate) async fn handle_enable_entities(
    transport: &Arc<dyn Transport>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    _entity_manager: &Arc<Mutex<EntityManager>>,
    _cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    _entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Peek at pending world entry without consuming it: we only commit (take)
    // after the create-player send_to succeeds, so a transient send failure
    // leaves login state intact for the next ENABLE_ENTITIES retry. Cloning
    // `pending_player_load_data` here so we can pre-warm the body model in
    // the same packet. The cost (a few hundred bytes of inventory +
    // abilities data clone) only fires once per world-entry.
    let pending = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get(&addr) {
            match (c.pending_player_entity_id, c.pending_world_entry.clone()) {
                (Some(eid), Some(entry)) => Some((eid, entry, c.pending_player_load_data.clone())),
                _ => None,
            }
        } else {
            None
        }
    };

    if let Some((_eid, entry_info, load_data)) = pending {
        // -- Create player step: CREATE_BASE_PLAYER + appearance pre-warm + onClientMapLoad --
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
            appearance_prewarm = load_data.is_some(),
            "Create player: sending CREATE_BASE_PLAYER + onClientMapLoad (waiting for mapLoaded)"
        );

        let pkt = build_create_player(&key, seq, &acks, &entry_info, load_data.as_ref());
        if let Err(e) = transport.send_to(&pkt, addr).await {
            // Issue #304: surface the failure before the early-return so
            // ops can correlate against the staged `pending_map_loaded`
            // state that won't be reached. Without this, a packet-send
            // failure looked identical to a client that simply never
            // ack'd `mapLoaded`.
            tracing::error!(
                %addr,
                player_entity_id = entry_info.player_entity_id,
                space_id = entry_info.space_id,
                seq,
                "Create player: socket.send_to failed before staging pending_map_loaded: {e}"
            );
            return Err(e.into());
        }
        // Register this reliable send with the per-session Channel's TX
        // window so it retransmits if the ACK doesn't land.
        super::super::helpers::shadow_register_reliable_send(
            connected,
            addr,
            seq,
            cimmeria_mercury::packet::Bytes::copy_from_slice(&pkt),
        );

        // Send succeeded — now commit: take pending state and stage map_loaded data.
        {
            let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
            if let Some(c) = clients.get_mut(&addr) {
                c.pending_player_entity_id.take();
                let entry = c.pending_world_entry.take().unwrap_or(entry_info);
                let load = c.pending_player_load_data.take();
                c.pending_map_loaded = Some(entry);
                c.pending_player_load_data = load;
                c.pending_client_ready = None;
            }
        }

        tracing::info!(%addr, "Create player complete -- waiting for client mapLoaded");
        return Ok(());
    }

    // -- Phase 4: initial entity creation -- send Account entity + char list --
    // Account entity was already allocated in Phase 3 (handle_login).

    // Guard: only proceed if we haven't already sent the char list.
    // Note: char_list_sent is flipped only after send_to succeeds below, so a
    // transient failure permits retry on the next ENABLE_ENTITIES.
    {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        match clients.get(&addr) {
            Some(c) if c.char_list_sent => return Ok(()),
            Some(_) => {}
            None => {
                tracing::warn!(%addr, "enable_entities: addr not in connected map");
                return Ok(());
            }
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
    transport.send_to(&pkt, addr).await?;
    // Char list is a one-shot state delivery; retransmit on loss.
    super::super::helpers::shadow_register_reliable_send(
        connected,
        addr,
        seq,
        cimmeria_mercury::packet::Bytes::copy_from_slice(&pkt),
    );

    // Send succeeded -- mark char list as sent so we don't re-send on subsequent ENABLE_ENTITIES.
    {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            c.char_list_sent = true;
        }
    }

    tracing::info!(%addr, "Phase 4 complete -- char list sent");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestTransport;
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn enable_entities_returns_ok_when_addr_not_in_connected_map() {
        // Typed handle for the no-send assertion; dyn handle for the call.
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
        let key = [0u8; 32];

        let result = handle_enable_entities(
            &dyn_transport,
            addr,
            key,
            1,
            &connected,
            &None,
            &entity_manager,
            &None,
            &entity_to_addr,
        )
        .await;
        assert!(result.is_ok());
        assert!(connected.lock().unwrap().is_empty());
        assert_eq!(entity_manager.lock().unwrap().entity_count(), 0);
        assert!(entity_to_addr.lock().unwrap().is_empty());
        assert!(transport.is_empty(), "early return must not send UDP");
    }

    #[tokio::test]
    async fn enable_entities_returns_ok_when_char_list_already_sent() {
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
        let mut client = crate::test_support::test_default_connected_client_state();
        client.char_list_sent = true;
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new({
                let mut m = HashMap::new();
                m.insert(addr, client);
                m
            }));
        let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
        let key = [0u8; 32];

        let result = handle_enable_entities(
            &dyn_transport,
            addr,
            key,
            1,
            &connected,
            &None,
            &entity_manager,
            &None,
            &entity_to_addr,
        )
        .await;
        assert!(
            result.is_ok(),
            "must short-circuit when char_list_sent is true"
        );
        let clients = connected.lock().unwrap();
        assert_eq!(clients.len(), 1);
        assert!(clients.get(&addr).unwrap().char_list_sent);
        drop(clients);
        assert_eq!(entity_manager.lock().unwrap().entity_count(), 0);
        assert!(entity_to_addr.lock().unwrap().is_empty());
        assert!(transport.is_empty(), "early return must not send UDP");
    }
}
