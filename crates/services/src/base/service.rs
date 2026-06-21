//! BaseService lifecycle — construction, startup, shutdown, admin API.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use cimmeria_common::{EntityId, ServerConfig};
use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::encryption::EncryptionVersion;
use cimmeria_mercury::transport::{Transport, UdpTransport};

use super::organization::authority::OrgAuthority;
use crate::auth::PendingLogin;
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::minigame::SessionRegistry;

use super::{
    archetype_name, connect_loop::run_connect_loop, outbox, resources::ResourceCache,
    world_entry::handle_cell_message, BaseError, ConnectedClientState, OnlinePlayer,
};

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

    /// Minigame session registry (shared with minigame TCP server).
    pub minigame_registry: SessionRegistry,

    /// Minigame server external host/port for URL construction.
    minigame_external_host: String,
    minigame_external_port: u16,

    /// Wire-encryption version every session on this service speaks. Pinned
    /// from server config at construction; threaded into each session's
    /// `ConnectedClientState` at login. Server-wide today — no per-client
    /// negotiation yet.
    enc_version: EncryptionVersion,
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
            minigame_registry: SessionRegistry::new(),
            minigame_external_host: config.base_external_host.clone(),
            minigame_external_port: config.minigame_port,
            enc_version: EncryptionVersion::from_config_u8(config.mercury_encryption_version),
        }
    }

    /// Snapshot of all connected players for the admin API.
    pub fn online_players(&self) -> Vec<OnlinePlayer> {
        let clients = self.connected_clients.lock().unwrap();
        clients
            .iter()
            .filter(|(_, c)| c.world_entry_sent)
            .map(|(addr, c)| OnlinePlayer {
                id: c.player_entity_id.unwrap_or(0),
                name: c.player_name.clone().unwrap_or_default(),
                archetype: archetype_name(c.player_archetype.unwrap_or(0)),
                level: c.player_level.unwrap_or(1),
                zone: c.world_name.clone().unwrap_or_default(),
                ping: None,
                status: if c.pending_map_loaded.is_some() || c.pending_client_ready.is_some() {
                    "loading"
                } else {
                    "in_world"
                },
                session: format!("{addr}"),
            })
            .collect()
    }

    /// Wire in the `pending_logins` Arc from `AuthService`.
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

        let resource_cache = match ResourceCache::load_all(&self.data_dir) {
            Ok(cache) => Some(Arc::new(cache)),
            Err(e) => {
                tracing::warn!("Failed to load resource cache: {e}");
                None
            }
        };

        let cell_tx = self.cell_tx.clone();
        let cell_rx = self.cell_rx.take();

        let connected = Arc::clone(&self.connected_clients);
        let entity_manager: Arc<Mutex<EntityManager>> = Arc::new(Mutex::new(EntityManager::new()));
        let entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Wrap the recv socket as a `BidirectionalTransport` once. The
        // recv loop owns it for `recv_from`; the cell→base handler gets
        // a clone projected to the send-only `Transport` super-trait.
        // Chaos integration tests substitute a `LossyTransport` wrapping
        // the same UDP socket without touching call sites.
        let bidi_transport: Arc<dyn cimmeria_mercury::transport::BidirectionalTransport> =
            Arc::new(UdpTransport::new(Arc::clone(&socket)));
        let transport_for_cell: Arc<dyn Transport> = bidi_transport.clone();
        let connected_for_cell = Arc::clone(&connected);
        let entity_to_addr_for_cell = Arc::clone(&entity_to_addr);
        let cell_tx_for_cell = cell_tx.clone();
        let db_pool_for_cell = self.db_pool.clone();
        let mg_registry_for_cell = Some(self.minigame_registry.clone());
        let mg_host_for_cell = self.minigame_external_host.clone();
        let mg_port_for_cell = self.minigame_external_port;

        // Build the OrgAuthority. If there's no DB pool we start empty and skip
        // persistence; org ops will be no-ops but won't crash.
        let org_authority_for_cell: Option<Arc<tokio::sync::Mutex<OrgAuthority>>> =
            if let Some(ref pool) = self.db_pool {
                match OrgAuthority::load_all(pool).await {
                    Ok(authority) => {
                        tracing::info!("OrgAuthority loaded from DB");
                        Some(Arc::new(tokio::sync::Mutex::new(authority)))
                    }
                    Err(e) => {
                        tracing::warn!(
                            "OrgAuthority load failed (DB error): {e}. Orgs will be empty."
                        );
                        Some(Arc::new(tokio::sync::Mutex::new(OrgAuthority::empty())))
                    }
                }
            } else {
                tracing::debug!("No DB pool — OrgAuthority disabled");
                None
            };

        tracing::trace!("Spawning base service UDP receive loop");
        let cell_tx_for_loop = cell_tx.clone();
        let connected_for_loop = Arc::clone(&connected);
        let entity_manager_for_loop = Arc::clone(&entity_manager);
        let entity_to_addr_for_loop = Arc::clone(&entity_to_addr);
        let enc_version = self.enc_version;
        // Clone the Arc so both the connect loop (login/logout tracking)
        // and the cell-message handler (persistent org mutations) share the
        // same OrgAuthority instance.
        let org_authority_for_loop = org_authority_for_cell.clone();
        tokio::spawn(async move {
            tracing::trace!("Base service UDP receive loop started");
            run_connect_loop(
                bidi_transport,
                pending_logins,
                db_pool,
                resource_cache,
                cell_tx_for_loop,
                connected_for_loop,
                entity_manager_for_loop,
                entity_to_addr_for_loop,
                enc_version,
                org_authority_for_loop,
            )
            .await;
            tracing::trace!("Base service UDP receive loop exited");
        });

        if let Some(mut cell_rx) = cell_rx {
            tokio::spawn(async move {
                tracing::debug!("Base service CellToBase message handler started");
                while let Some(msg) = cell_rx.recv().await {
                    handle_cell_message(
                        msg,
                        &transport_for_cell,
                        &connected_for_cell,
                        &entity_to_addr_for_cell,
                        &cell_tx_for_cell,
                        &db_pool_for_cell,
                        &mg_registry_for_cell,
                        &mg_host_for_cell,
                        mg_port_for_cell,
                        &org_authority_for_cell,
                    )
                    .await;
                }
                tracing::debug!("Base service CellToBase message handler exited");
            });
        }

        // Spawn the cell_event_outbox drainer. The startup pass
        // replays any rows orphaned by the previous shutdown; the periodic
        // ticker covers transient channel failures during steady-state.
        // Requires both a DB pool and a cell channel — gated on both.
        if let (Some(pool), Some(tx)) = (self.db_pool.clone(), cell_tx.clone()) {
            outbox::spawn_drainer(pool, tx);
        } else {
            tracing::debug!(
                "Skipping cell_event_outbox drainer: db_pool or cell_tx not configured"
            );
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
