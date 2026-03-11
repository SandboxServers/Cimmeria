//! BaseApp service -- Mercury UDP listener.
//!
//! # Protocol phases
//!
//! ## Phase 3 -- initial connect (unencrypted)
//!
//! 1. Client sends `baseAppLogin` (flags=0x41, 41 bytes, **unencrypted**).
//! 2. Server validates the ticket against the pending-logins map.
//! 3. Server sends **encrypted** `BASEMSG_REPLY_MESSAGE` (seq=1) echoing the ticket.
//! 4. Server sends **encrypted** time-sync bundle (seq=2).
//! 5. Connection is registered; all further traffic on this channel is encrypted.
//!
//! ## Phase 4 -- character list
//!
//! 1. Client sends AUTHENTICATE (msg_id=0x01, **encrypted**) -- server ignores it
//!    (matches C++ reference: "Ignored msg" at line 131 of client_handler.cpp).
//! 2. Client sends ENABLE_ENTITIES (msg_id=0x08) -- server creates the Account
//!    entity and sends the character list.
//! 3. Server sends **encrypted** character list (seq=3): game-state (0x05) +
//!    character list (0x82, count=N from DB).
//! 4. Server spawns a per-connection tick-sync task that sends `BASEMSG_TICK_SYNC`
//!    every 100 ms (seq=4, 5, ...) to prevent REASON_INACTIVITY disconnects.
//!
//! ## Phase 5 -- world entry
//!
//! 1. Client sends `playCharacter` (msg_id=0xC4, base method index 4) inside an
//!    encrypted bundle (prefixed by authenticate msg_id=0x01).
//! 2. Server queries DB for character position, sends compound packet:
//!    `createBasePlayer` (SGWPlayer) + `spaceViewportInfo` + `createCellPlayer` +
//!    `forcedPosition`.
//! 3. Client creates player entity and enters the game world.
//!
//! ## Character creation
//!
//! 1. Client sends `createCharacter` (msg_id=0xC4) with name, archetype, visuals.
//! 2. Server INSERTs into `sgw_player`, sends updated character list on success.
//!
//! ## Cooked data serving
//!
//! 1. Client sends `versionInfoRequest` (0xC0) -- server responds with `onVersionInfo`.
//! 2. Client sends `elementDataRequest` (0xC1) -- server fragments and sends XML
//!    from CookedCharCreation.pak via `BASEMSG_RESOURCE_FRAGMENT`.
//!
//! See `docs/protocol/login-handshake.md` for the full wire-level spec.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use cimmeria_common::{EntityId, ServerConfig};
use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::encryption::MercuryEncryption;

use serde::Serialize;

use crate::auth::PendingLogin;
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::mercury::{PlayerLoadData, WorldEntryInfo};

// ── Submodules ───────────────────────────────────────────────────────────────

pub(crate) mod chardef;
pub(crate) mod resources;
pub(crate) mod helpers;
pub(crate) mod connect_loop;
pub(crate) mod login;
pub(crate) mod character;
pub(crate) mod cooked_data;
pub(crate) mod world_entry;
pub(crate) mod dispatch;
pub(crate) mod tick_sync;

use connect_loop::run_connect_loop;
use resources::ResourceCache;
use world_entry::handle_cell_message;

// ── Error types ───────────────────────────────────────────────────────────────

/// Errors specific to the base service.
#[derive(Debug, thiserror::Error)]
pub enum BaseError {
    #[error("Entity {0} not found")]
    EntityNotFound(EntityId),

    #[error("Entity creation failed: {0}")]
    CreationFailed(String),

    #[error("Service not running")]
    NotRunning,

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}

// ── Public snapshot for admin API ─────────────────────────────────────────────

/// A snapshot of one connected player, safe to serialize for the admin API.
#[derive(Debug, Clone, Serialize)]
pub struct OnlinePlayer {
    pub id: u32,
    pub name: String,
    pub archetype: &'static str,
    pub level: i32,
    pub zone: String,
    pub ping: Option<u32>,
    pub status: &'static str,
    pub session: String,
}

fn archetype_name(id: i32) -> &'static str {
    match id {
        1 => "Soldier",
        2 => "Commando",
        3 => "Scientist",
        4 => "Archaeologist",
        5 => "Asgard",
        6 => "Goa'uld",
        7 => "Jaffa",
        _ => "Unknown",
    }
}

// ── Per-connection state ──────────────────────────────────────────────────────

/// Deferred world-entry finalization that must wait for `SGWPlayer.onClientReady`.
pub(crate) struct PendingClientReadyInfo {
    /// Player entity ID to connect on the CellService.
    pub entity_id: u32,
    /// Persistent player ID used for DB-backed initialization.
    pub player_id: i32,
    /// World name used by content-engine `player.loaded` triggers.
    pub world_name: String,
}

/// State held for each client that has completed the Phase 3 handshake.
pub(crate) struct ConnectedClientState {
    /// Encryption context (for decrypting client packets).
    pub enc: MercuryEncryption,
    /// Raw 32-byte AES-256 key (for spawning the tick-sync task).
    pub key: [u8; 32],
    /// Account ID from the pending login (FK to account table).
    pub account_id: u32,
    /// Account privilege level (0 = normal, 2+ = admin/GM).
    /// Will be used for GM command authorization.
    #[allow(dead_code)]
    pub access_level: u32,
    /// `true` once the Phase 4 character list has been sent to this client.
    pub char_list_sent: bool,
    /// `true` once the Phase 5a world entry packet (viewport+cell+position+reset) has been sent.
    pub world_entry_sent: bool,
    /// Player entity ID for Phase 5b (CREATE_BASE_PLAYER), set after Phase 5a.
    /// Consumed by `handle_enable_entities` Phase 5b via `.take()`.
    pub pending_player_entity_id: Option<u32>,
    /// Allocated player entity ID (persisted for cleanup on disconnect).
    pub player_entity_id: Option<u32>,
    /// Shared outgoing sequence counter.  Used by both the tick-sync loop
    /// and one-shot senders (Phase 4/5) to avoid seq collisions.
    pub next_seq: Arc<AtomicU32>,
    /// Client reliable-message sequence IDs that need to be ACKed.
    /// Drained by the tick-sync loop (or the char_list response) and
    /// piggybacked on the next outgoing packet.
    pub pending_acks: Arc<Mutex<Vec<u32>>>,
    /// Timestamp of the last received packet from this client.
    /// Used by the tick-sync loop to detect dead clients.
    pub last_recv: Arc<Mutex<Instant>>,
    /// Dynamically allocated Account entity ID for this session.
    pub account_entity_id: u32,
    /// Monotonic counter for resource fragment dataId values.
    pub next_data_id: u16,
    /// World entry info for Phase 5b (viewport + cell + position), set after Phase 5a.
    /// Consumed by `handle_enable_entities` Phase 5b via `.take()`.
    pub pending_world_entry: Option<WorldEntryInfo>,
    /// Full player load data for the mapLoaded sequence, set after Phase 5a.
    /// In Phase 5b-A, stored here. Consumed in Phase 5b-B when client sends
    /// `mapLoaded` (cell method index 25).
    pub pending_player_load_data: Option<PlayerLoadData>,
    /// World entry info held between Phase 5b-A (CREATE_BASE_PLAYER + onClientMapLoad)
    /// and Phase 5b-B (VIEWPORT + CELL + POSITION), triggered by client `mapLoaded`.
    pub pending_world_entry_phase_b: Option<WorldEntryInfo>,
    /// Player init that must wait until the client sends `SGWPlayer.onClientReady`
    /// after receiving the full mapLoaded bundle.
    pub pending_client_ready: Option<PendingClientReadyInfo>,
    /// Set to `true` by `handle_log_off` to signal the tick-sync loop to stop.
    pub cancelled: Arc<AtomicBool>,
    /// Player character name, set during world entry for chat routing.
    pub player_name: Option<String>,
    /// Player level, set during world entry for admin display.
    pub player_level: Option<i32>,
    /// Archetype ID, set during world entry for admin display.
    pub player_archetype: Option<i32>,
    /// Current world name, set during world entry for admin display.
    pub world_name: Option<String>,
    /// Player XP, set during world entry and updated on GrantXP.
    pub player_xp: Option<u64>,
    /// Player training points, set during world entry and updated on GrantXP.
    pub player_training_points: Option<u32>,
}

// ── BaseService ───────────────────────────────────────────────────────────────

/// BaseApp service -- manages persistent entity state and client connections.
pub struct BaseService {
    /// Address the Mercury UDP listener binds to.
    pub listener_addr: SocketAddr,

    /// Whether the service is currently running.
    pub is_running: bool,

    /// Pending login handoffs shared with AuthService (ticket -> PendingLogin).
    /// Wired by the orchestrator before `start()` is called.
    pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,

    /// Database connection pool (None if not connected).
    db_pool: Option<Arc<PgPool>>,

    /// Path to the data directory for loading .pak files.
    data_dir: String,

    /// Sender for messages to CellService (set by orchestrator).
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,

    /// Receiver for messages from CellService (set by orchestrator, taken at start).
    cell_rx: Option<mpsc::Receiver<CellToBaseMsg>>,

    /// Shared connected-clients map, exposed for admin API read access.
    connected_clients: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
}

impl BaseService {
    /// Create a new base service from server configuration.
    pub fn new(config: &ServerConfig) -> Self {
        let listener_addr = format!("{}:{}", config.base_host, config.base_port)
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], config.base_port)));

        Self {
            listener_addr,
            is_running: false,
            pending_logins: Arc::new(Mutex::new(HashMap::new())),
            db_pool: None,
            data_dir: "data/cache".to_string(),
            cell_tx: None,
            cell_rx: None,
            connected_clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Snapshot of all connected players for the admin API.
    pub fn online_players(&self) -> Vec<OnlinePlayer> {
        let clients = self.connected_clients.lock().unwrap();
        clients.iter()
            .filter(|(_, c)| c.world_entry_sent)
            .map(|(addr, c)| OnlinePlayer {
                id: c.player_entity_id.unwrap_or(0),
                name: c.player_name.clone().unwrap_or_default(),
                archetype: archetype_name(c.player_archetype.unwrap_or(0)),
                level: c.player_level.unwrap_or(1),
                zone: c.world_name.clone().unwrap_or_default(),
                ping: None,
                status: if c.pending_world_entry_phase_b.is_some() || c.pending_client_ready.is_some() {
                    "loading"
                } else {
                    "in_world"
                },
                session: format!("{addr}"),
            })
            .collect()
    }

    /// Wire in the `pending_logins` Arc from `AuthService`.
    ///
    /// Must be called before [`start`].
    pub fn set_pending_logins(
        &mut self,
        pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
    ) {
        self.pending_logins = pending_logins;
    }

    /// Set the database connection pool.
    pub fn set_db_pool(&mut self, pool: Arc<PgPool>) {
        self.db_pool = Some(pool);
    }

    /// Wire in the Base<->Cell channels. Called by the orchestrator before `start()`.
    pub fn set_cell_channel(
        &mut self,
        tx: mpsc::Sender<BaseToCellMsg>,
        rx: mpsc::Receiver<CellToBaseMsg>,
    ) {
        self.cell_tx = Some(tx);
        self.cell_rx = Some(rx);
    }

    /// Start the Mercury UDP listener on `listener_addr`.
    pub async fn start(&mut self) -> Result<(), BaseError> {
        tracing::info!(addr = %self.listener_addr, "Starting base service UDP listener");

        tracing::trace!(addr = %self.listener_addr, "Binding UDP socket for base service");
        let socket = Arc::new(UdpSocket::bind(self.listener_addr).await.map_err(|e| {
            tracing::error!(addr = %self.listener_addr, error = %e, "Failed to bind base UDP socket");
            e
        })?);
        tracing::info!(addr = %socket.local_addr().unwrap(), "Base service UDP socket bound");

        let pending_logins = Arc::clone(&self.pending_logins);
        let db_pool = self.db_pool.clone();

        // Load all cooked data PAK files
        let resource_cache = match ResourceCache::load_all(&self.data_dir) {
            Ok(cache) => Some(Arc::new(cache)),
            Err(e) => {
                tracing::warn!("Failed to load resource cache: {e}");
                None
            }
        };

        let cell_tx = self.cell_tx.clone();
        let cell_rx = self.cell_rx.take();

        // Shared state accessible by the connect loop, cell message handler, and admin API.
        let connected = Arc::clone(&self.connected_clients);
        let entity_manager: Arc<Mutex<EntityManager>> =
            Arc::new(Mutex::new(EntityManager::new()));
        // Reverse index: entity_id -> SocketAddr (populated during Phase 5b).
        let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Clone shared state for the cell message handler before moving into connect loop.
        let socket_for_cell = Arc::clone(&socket);
        let connected_for_cell = Arc::clone(&connected);
        let entity_to_addr_for_cell = Arc::clone(&entity_to_addr);
        let cell_tx_for_cell = cell_tx.clone();
        let db_pool_for_cell = self.db_pool.clone();

        tracing::trace!("Spawning base service UDP receive loop");
        let cell_tx_for_loop = cell_tx.clone();
        let connected_for_loop = Arc::clone(&connected);
        let entity_manager_for_loop = Arc::clone(&entity_manager);
        let entity_to_addr_for_loop = Arc::clone(&entity_to_addr);
        tokio::spawn(async move {
            tracing::trace!("Base service UDP receive loop started");
            run_connect_loop(
                socket, pending_logins, db_pool, resource_cache,
                cell_tx_for_loop, connected_for_loop,
                entity_manager_for_loop, entity_to_addr_for_loop,
            ).await;
            tracing::trace!("Base service UDP receive loop exited");
        });

        // Spawn a task to process messages from CellService
        if let Some(mut cell_rx) = cell_rx {
            tokio::spawn(async move {
                tracing::debug!("Base service CellToBase message handler started");
                while let Some(msg) = cell_rx.recv().await {
                    handle_cell_message(
                        msg, &socket_for_cell, &connected_for_cell,
                        &entity_to_addr_for_cell,
                        &cell_tx_for_cell, &db_pool_for_cell,
                    ).await;
                }
                tracing::debug!("Base service CellToBase message handler exited");
            });
        }

        self.is_running = true;
        Ok(())
    }

    /// Stop the base service gracefully.
    pub async fn stop(&mut self) {
        tracing::info!("Stopping base service");
        self.is_running = false;
        tracing::trace!("Base service stopped");
    }

    /// Create a new base entity (stub).
    pub async fn create_base_entity(&self) -> Result<EntityId, BaseError> {
        if !self.is_running {
            return Err(BaseError::NotRunning);
        }
        tracing::debug!("Creating base entity");
        Ok(EntityId(0))
    }

    /// Destroy a base entity (stub).
    pub async fn destroy_base_entity(&self, entity_id: EntityId) -> Result<(), BaseError> {
        if !self.is_running {
            return Err(BaseError::NotRunning);
        }
        tracing::debug!(%entity_id, "Destroying base entity");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mercury::SKIN_TINTS;

    #[test]
    fn new_service_is_not_running() {
        let config = ServerConfig::default();
        let svc = BaseService::new(&config);
        assert!(!svc.is_running);
        assert_eq!(svc.listener_addr.port(), 32832);
    }

    #[tokio::test]
    async fn start_sets_running() {
        let mut config = ServerConfig::default();
        config.base_port = 0; // OS-assigned port to avoid conflicts in tests
        let mut svc = BaseService::new(&config);
        svc.start().await.unwrap();
        assert!(svc.is_running);
    }

    #[tokio::test]
    async fn create_entity_fails_when_not_running() {
        let config = ServerConfig::default();
        let svc = BaseService::new(&config);
        let result = svc.create_base_entity().await;
        assert!(result.is_err());
    }

    // ── SKIN_TINTS tests ─────────────────────────────────────────────────────

    #[test]
    fn skin_tints_array_length() {
        assert_eq!(SKIN_TINTS.len(), 16);
    }

    #[test]
    fn skin_tints_all_nonzero() {
        for (i, &tint) in SKIN_TINTS.iter().enumerate() {
            assert_ne!(tint, 0, "SKIN_TINTS[{i}] should not be zero");
        }
    }

    #[test]
    fn skin_tints_index_0_matches_python() {
        // From python/common/Constants.py: SKIN_TINTS[0] = 0x2F1308FF
        assert_eq!(SKIN_TINTS[0], 0x2F1308FF);
    }
}
