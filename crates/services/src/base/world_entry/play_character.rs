//! `playCharacter` -> entity teardown step.
//!
//! When the client picks a character and calls `playCharacter`, we tear down
//! its previous entity tree by sending RESET_ENTITIES. The next packet from
//! the client is ENABLE_ENTITIES, which drives the create-player step in
//! `enable_entities.rs`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use cimmeria_entity::manager::EntityManager;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::build_reset_entities;

use super::super::ConnectedClientState;
use super::methods::{query_player_load_data, query_world_entry};

/// Send RESET_ENTITIES when the client calls `playCharacter` to begin world entry.
pub(crate) async fn handle_play_character(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    player_id: i32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Guard: only send once per connection.
    let arcs = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            if !c.world_entry_sent {
                c.world_entry_sent = true;
                Some((Arc::clone(&c.pending_acks), Arc::clone(&c.next_seq)))
            } else {
                None
            }
        } else {
            tracing::warn!(%addr, "play_character: addr not in connected map");
            None
        }
    };

    let (pending_acks_arc, next_seq) = match arcs {
        Some(a) => a,
        None => return Ok(()),
    };

    // Query character data from DB and resolve space via CellService
    let entry_info =
        query_world_entry(db_pool, account_id, player_id, entity_manager, cell_tx).await;

    // query_world_entry returns NO_ENTITY_ID as a "world entry failed"
    // sentinel (DB error or character not found). Bail before dispatching any
    // packets that would target an entity the cell never registered, AND
    // clear the world_entry_sent latch so a subsequent playCharacter retry on
    // the same connection isn't silently dropped.
    if entry_info.player_entity_id == super::methods::world_entry_db::NO_ENTITY_ID {
        tracing::error!(
            %addr, player_id, account_id,
            "World entry aborted: query_world_entry returned NO_ENTITY_ID sentinel"
        );
        if let Ok(mut clients) = connected.lock() {
            if let Some(c) = clients.get_mut(&addr) {
                c.world_entry_sent = false;
            }
        }
        return Ok(());
    }

    // Also query the full player data needed for mapLoaded
    let player_load_data = query_player_load_data(db_pool, account_id, player_id).await;

    // NOTE: C++ Account.py:293-296 uses SGWGmPlayer (0x03) for access_level > 0,
    // but SGWGmPlayer adds 6 ClientMethods and 80+ CellMethods that shift ALL
    // flattened method indices. Our hardcoded method_idx constants (BeingAppearance=26,
    // etc.) only work for SGWPlayer. Until we build a separate SGWGmPlayer index
    // table, always use SGWPlayer (0x02) regardless of access_level.
    // TODO: Build SGWGmPlayer method index table to enable GM entity type.

    tracing::info!(
        %addr,
        player_id,
        entity_id = entry_info.player_entity_id,
        space_id = entry_info.space_id,
        pos = ?entry_info.pos,
        class_id = entry_info.class_id,
        "World entry: sending RESET_ENTITIES (entity teardown)"
    );

    let acks: Vec<u32> = {
        let mut pending = pending_acks_arc.lock().unwrap();
        pending.drain(..).collect()
    };

    // Entity teardown: Send ONLY RESET_ENTITIES.
    // The C++ server sends RESET_ENTITIES in its own flushed bundle. The client
    // tears down all entities, then sends ENABLE_ENTITIES, which triggers
    // the create-player step (CREATE_BASE_PLAYER + viewport + cell + forced position).
    let seq = next_seq.fetch_add(1, Ordering::Relaxed);
    let pkt = build_reset_entities(&key, seq, &acks);
    tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT RESET_ENTITIES (entity teardown)");
    socket.send_to(&pkt, addr).await?;

    // Store the world entry info and player load data for the create-player step.
    {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_player_entity_id = Some(entry_info.player_entity_id);
            c.player_entity_id = Some(entry_info.player_entity_id);
            c.player_name = Some(player_load_data.player_name.clone());
            c.player_level = Some(player_load_data.level);
            c.player_archetype = Some(player_load_data.archetype);
            c.world_name = Some(entry_info.world_name.clone());
            c.player_xp = Some(player_load_data.exp as u64);
            c.player_training_points = Some(player_load_data.training_points as u32);
            // Cache the selected player's DB id so gate-travel/respawn can use
            // it directly instead of falling back to a "lowest player_id for
            // this account" lookup that would target the wrong character on
            // multi-character accounts.
            c.active_player_id = Some(player_id);
            c.pending_world_entry = Some(entry_info);
            c.pending_player_load_data = Some(player_load_data);
            c.pending_client_ready = None;
        }
    }

    tracing::info!(%addr, "Entity teardown sent -- waiting for ENABLE_ENTITIES from client");

    Ok(())
}

#[cfg(test)]
mod tests {
    //! World-entry latch tests for the auth → base → cell handoff seam.
    //!
    //! `world_entry_sent` is a one-shot latch on ConnectedClientState — it
    //! prevents duplicate RESET_ENTITIES dispatches when the client retries
    //! `playCharacter`. Cases covered:
    //!   * already latched → second call is a no-op (no overwrite of
    //!     `pending_player_entity_id`).
    //!   * no-DB success path → CombatSim default routes through, latch
    //!     flips, world_name + active_player_id get cached.
    //!   * unknown addr (ConnectedClientState gone) → no panic, no latch
    //!     change.
    //!
    //! Not covered here: the NO_ENTITY_ID failure path (latch reset on
    //! a failed query_world_entry) — that path requires a real DB pool
    //! that returns no rows, which is out of scope for unit tests.
    //! The DB-success branch (live-DB seeded) is exercised by the
    //! integration tests under `methods/world_entry_db.rs::tests`.

    use super::*;
    use cimmeria_mercury::encryption::MercuryEncryption;
    use std::sync::atomic::{AtomicBool, AtomicU32};
    use std::time::Instant;

    fn make_connected_state(account_id: u32) -> ConnectedClientState {
        ConnectedClientState {
            enc: MercuryEncryption::from_session_key([0xCDu8; 32]),
            key: [0xCDu8; 32],
            account_id,
            access_level: 0,
            char_list_sent: false,
            world_entry_sent: false,
            pending_player_entity_id: None,
            player_entity_id: None,
            next_seq: Arc::new(AtomicU32::new(3)),
            pending_acks: Arc::new(Mutex::new(Vec::new())),
            last_recv: Arc::new(Mutex::new(Instant::now())),
            account_entity_id: 1,
            next_data_id: 0,
            pending_world_entry: None,
            pending_player_load_data: None,
            pending_map_loaded: None,
            pending_client_ready: None,
            cached_appearance_args: None,
            cached_tint_args: None,
            cancelled: Arc::new(AtomicBool::new(false)),
            player_name: None,
            player_level: None,
            player_archetype: None,
            world_name: None,
            player_xp: None,
            player_training_points: None,
            active_player_id: None,
        }
    }

    async fn make_socket() -> Arc<UdpSocket> {
        Arc::new(UdpSocket::bind("127.0.0.1:0").await.expect("bind UDP"))
    }

    /// Second `playCharacter` on an already-latched session must be a
    /// no-op — no UDP send, no state change. Without this, the client
    /// could trigger redundant entity teardowns mid-world-entry.
    #[tokio::test]
    async fn play_character_is_idempotent_when_world_entry_already_sent() {
        let socket = make_socket().await;
        let addr: SocketAddr = "127.0.0.1:55600".parse().unwrap();
        let connected = Arc::new(Mutex::new(HashMap::new()));
        let mut state = make_connected_state(0xAABB);
        // Pre-latch: world_entry_sent is already true. Also seed an
        // entity_id so we can detect any unwanted overwrite.
        state.world_entry_sent = true;
        state.player_entity_id = Some(0x77_88_99_AA);
        state.active_player_id = Some(42);
        connected.lock().unwrap().insert(addr, state);

        let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
        let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;

        handle_play_character(
            &socket,
            addr,
            [0xCDu8; 32],
            0xAABB,
            42,
            &connected,
            &None,
            &entity_manager,
            &cell_tx,
        )
        .await
        .expect("idempotent path returns Ok");

        // Latch unchanged, prior entity_id intact (no second-call clobber).
        let map = connected.lock().unwrap();
        let c = map.get(&addr).unwrap();
        assert!(
            c.world_entry_sent,
            "latch must remain true on idempotent re-call"
        );
        assert_eq!(
            c.player_entity_id,
            Some(0x77_88_99_AA),
            "idempotent path must NOT overwrite the player_entity_id from the first call"
        );
    }

    /// No-DB CombatSim path: `query_world_entry` allocates a real
    /// entity_id and returns a default entry (CombatSim is the
    /// smoke-test world). play_character must then populate
    /// ConnectedClientState with the allocated entity_id and the
    /// "CombatSim" world name. Pins the no-DB code path used by the
    /// orchestrator's bring-up smoke tests when no Postgres is reachable.
    #[tokio::test]
    async fn play_character_no_db_routes_to_combatsim_and_populates_state() {
        let socket = make_socket().await;
        let addr: SocketAddr = "127.0.0.1:55601".parse().unwrap();
        let connected = Arc::new(Mutex::new(HashMap::new()));
        connected
            .lock()
            .unwrap()
            .insert(addr, make_connected_state(0xCCDD));

        let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
        let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;
        const PLAYER_ID: i32 = 7;

        handle_play_character(
            &socket,
            addr,
            [0xCDu8; 32],
            0xCCDD,
            PLAYER_ID,
            &connected,
            &None, // no DB → CombatSim default entry (NOT a failure)
            &entity_manager,
            &cell_tx,
        )
        .await
        .expect("no-DB CombatSim path returns Ok");

        let map = connected.lock().unwrap();
        let c = map.get(&addr).unwrap();
        // Latch is set — first playCharacter on this connection.
        assert!(c.world_entry_sent, "latch flips on first call");
        // CombatSim default world is what query_world_entry's no-DB
        // branch hard-codes; pin it so a future smoke-test refactor
        // can't silently change which world the no-DB path maps to.
        assert_eq!(
            c.world_name.as_deref(),
            Some("CombatSim"),
            "no-DB path must route to CombatSim"
        );
        // active_player_id was cached from the call argument so
        // gate-travel / respawn target the right character on
        // multi-character accounts.
        assert_eq!(c.active_player_id, Some(PLAYER_ID));
        // Entity id was allocated by EntityManager — must be non-zero
        // (zero is the NO_ENTITY_ID sentinel for the failure paths).
        assert!(
            c.player_entity_id.unwrap_or(0) != 0,
            "no-DB success path allocates a real entity_id, not the NO_ENTITY_ID sentinel"
        );
    }

    /// Unknown addr (ConnectedClientState absent) is the racy case where
    /// the client disconnected between sending playCharacter and base
    /// processing it. Handler must NOT panic and must NOT create a
    /// stub ConnectedClientState — the connection lifecycle is the only
    /// gate that produces those rows.
    #[tokio::test]
    async fn play_character_for_unknown_addr_is_silent_no_op() {
        let socket = make_socket().await;
        let addr: SocketAddr = "127.0.0.1:55602".parse().unwrap();
        // connected is empty — addr is genuinely unknown.
        let connected = Arc::new(Mutex::new(HashMap::new()));
        let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
        let cell_tx: Option<mpsc::Sender<BaseToCellMsg>> = None;

        let result = handle_play_character(
            &socket,
            addr,
            [0xCDu8; 32],
            0xEEFF,
            1,
            &connected,
            &None,
            &entity_manager,
            &cell_tx,
        )
        .await;
        assert!(
            result.is_ok(),
            "unknown-addr path must not propagate an error"
        );
        assert!(
            connected.lock().unwrap().is_empty(),
            "unknown addr must NOT cause an entry to be created"
        );
    }
}
