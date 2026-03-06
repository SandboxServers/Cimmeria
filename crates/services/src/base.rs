//! BaseApp service — Mercury UDP listener.
//!
//! # Protocol phases
//!
//! ## Phase 3 — initial connect (unencrypted)
//!
//! 1. Client sends `baseAppLogin` (flags=0x41, 41 bytes, **unencrypted**).
//! 2. Server validates the ticket against the pending-logins map.
//! 3. Server sends **encrypted** `BASEMSG_REPLY_MESSAGE` (seq=1) echoing the ticket.
//! 4. Server sends **encrypted** time-sync bundle (seq=2).
//! 5. Connection is registered; all further traffic on this channel is encrypted.
//!
//! ## Phase 4 — character list
//!
//! 1. Client sends AUTHENTICATE (msg_id=0x01, **encrypted**) — server ignores it
//!    (matches C++ reference: "Ignored msg" at line 131 of client_handler.cpp).
//! 2. Client sends ENABLE_ENTITIES (msg_id=0x08) — server creates the Account
//!    entity and sends the character list.
//! 3. Server sends **encrypted** character list (seq=3): game-state (0x05) +
//!    character list (0x82, count=N from DB).
//! 4. Server spawns a per-connection tick-sync task that sends `BASEMSG_TICK_SYNC`
//!    every 100 ms (seq=4, 5, …) to prevent REASON_INACTIVITY disconnects.
//!
//! ## Phase 5 — world entry
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
//! 1. Client sends `versionInfoRequest` (0xC0) — server responds with `onVersionInfo`.
//! 2. Client sends `elementDataRequest` (0xC1) — server fragments and sends XML
//!    from CookedCharCreation.pak via `BASEMSG_RESOURCE_FRAGMENT`.
//!
//! See `docs/protocol/login-handshake.md` for the full wire-level spec.

use std::collections::HashMap;
use std::io::Read as IoRead;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use cimmeria_common::{EntityId, ServerConfig};
use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::{parse_incoming, FLAG_HAS_REQUESTS, FLAG_HAS_SEQUENCE, FLAG_RELIABLE};

use tokio::sync::mpsc;

use crate::auth::PendingLogin;
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::mercury_ext::{
    build_avatar_update, build_char_list, build_char_create_failed,
    build_character_visuals, build_connect_reply, build_create_entity,
    build_entity_leave, build_entity_method_packet, build_logged_off,
    build_map_loaded, build_on_character_list, build_ongoing_tick_sync,
    build_reset_entities, build_resource_fragment,
    build_time_sync, build_version_info,
    build_world_entry, read_wstring,
    archetype_ability_tree, CharacterInfo, PlayerLoadData, WorldEntryInfo,
    DEFAULT_SPACE_ID, SKIN_TINTS,
    FRAG_FIRST, FRAG_FIRST_AND_LAST, FRAG_LAST, FRAG_MIDDLE,
};

// ── CharDef lookup ───────────────────────────────────────────────────────────

/// Given a CharDefId (1–23), return (alignment, archetype, gender, bodyset, world_location, pos_x, pos_y, pos_z).
/// Returns None for unknown IDs.
///
/// Derived from `db/resources/Archetypes/Seed/char_creation.sql`.
///
/// Starting coordinates:
/// - Praxis (Castle_CellBlock): (-334.231, 73.472, -228.026)
/// - SGU (SGC_W1): (201.5, 1.31, 49.724)
fn chardef_lookup(id: i32) -> Option<(i32, i32, i32, &'static str, &'static str, f32, f32, f32)> {
    // (alignment, archetype, gender, bodyset, starting_world, pos_x, pos_y, pos_z)
    // Alignment: 1=Praxis, 2=SGU
    // Gender: 0=Male, 1=Female (EGender enum: GENDER_Male=0, GENDER_Female=1)
    //   DB constraint requires 1-3, so we store gender+1: 1=Male, 2=Female
    // Bodyset uses doubled format: "BS_X.BS_X" (matches DB varchar(64) column)
    const PRAXIS_POS: (f32, f32, f32) = (-334.231, 73.472, -228.026);
    const SGU_POS: (f32, f32, f32) = (201.5, 1.31, 49.724);

    match id {
        1  => Some((1, 1, 1, "BS_HumanMale.BS_HumanMale",     "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Soldier Male
        2  => Some((2, 1, 1, "BS_HumanMale.BS_HumanMale",     "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Soldier Male
        3  => Some((1, 2, 1, "BS_HumanMale.BS_HumanMale",     "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Commando Male
        4  => Some((2, 2, 1, "BS_HumanMale.BS_HumanMale",     "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Commando Male
        5  => Some((1, 4, 1, "BS_HumanMale.BS_HumanMale",     "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Archeologist Male
        6  => Some((2, 4, 1, "BS_HumanMale.BS_HumanMale",     "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Archeologist Male
        7  => Some((1, 8, 1, "BS_JaffaMale.BS_JaffaMale",     "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Jaffa Male
        8  => Some((2, 7, 1, "BS_JaffaMale.BS_JaffaMale",     "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Shol'va Male
        9  => Some((2, 5, 1, "BS_Asgard.BS_Asgard",           "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Asgard Male
        10 => Some((1, 6, 1, "BS_GoauldMale.BS_GoauldMale",   "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Goa'uld Male
        11 => Some((1, 1, 2, "BS_HumanFemale.BS_HumanFemale", "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Soldier Female
        12 => Some((2, 1, 2, "BS_HumanFemale.BS_HumanFemale", "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Soldier Female
        13 => Some((1, 2, 2, "BS_HumanFemale.BS_HumanFemale", "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Commando Female
        14 => Some((2, 2, 2, "BS_HumanFemale.BS_HumanFemale", "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Commando Female
        15 => Some((1, 4, 2, "BS_HumanFemale.BS_HumanFemale", "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Archeologist Female
        16 => Some((2, 4, 2, "BS_HumanFemale.BS_HumanFemale", "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Archeologist Female
        17 => Some((1, 8, 2, "BS_JaffaFemale.BS_JaffaFemale", "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Jaffa Female
        18 => Some((2, 7, 2, "BS_JaffaFemale.BS_JaffaFemale", "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Shol'va Female
        19 => Some((1, 6, 2, "BS_GoauldFemale.BS_GoauldFemale","Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Goa'uld Female
        20 => Some((1, 3, 1, "BS_HumanMale.BS_HumanMale",     "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Scientist Male
        21 => Some((2, 3, 1, "BS_HumanMale.BS_HumanMale",     "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Scientist Male
        22 => Some((1, 3, 2, "BS_HumanFemale.BS_HumanFemale", "Castle_CellBlock", PRAXIS_POS.0, PRAXIS_POS.1, PRAXIS_POS.2)), // Praxis Scientist Female
        23 => Some((2, 3, 2, "BS_HumanFemale.BS_HumanFemale", "SGC_W1",           SGU_POS.0,    SGU_POS.1,    SGU_POS.2)),    // SGU Scientist Female
        _  => None,
    }
}

// ── Inventory constants (from python/Atrea/enums.py + Account.py) ────────────

const INV_BANDOLIER: i32 = 3;

/// Order in which starter items fill inventory bags (Account.py:12-31).
/// Equipment slots first so items get equipped and show on the char select screen.
const BAG_FILL_ORDER: &[i32] = &[
    4,  // Head
    5,  // Face
    6,  // Neck
    7,  // Chest
    8,  // Hands
    9,  // Waist
    10, // Back
    11, // Legs
    12, // Feet
    13, // Artifact1
    14, // Artifact2
    3,  // Bandolier
    2,  // Mission
    1,  // Main
    15, // Crafting
];

/// Max items per container (Constants.py:142-162).
fn bag_max_slots(container_id: i32) -> i32 {
    match container_id {
        1 => 40,   // Main
        2 => 100,  // Mission
        3 => 4,    // Bandolier
        4..=14 => 1,  // Equipment slots
        15 => 100, // Crafting
        _ => 0,
    }
}

// ── Hex dump helper ──────────────────────────────────────────────────────────

/// Format a byte slice as a hex string for trace logging.
fn to_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")
}

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

// ── Resource cache ───────────────────────────────────────────────────────────

/// In-memory cache of cooked character creation data loaded from CookedCharCreation.pak.
/// Per-category cooked data loaded from a PAK file.
struct CategoryData {
    /// MetaData value (u32 from the PAK's MetaData entry).
    metadata: u32,
    /// elementId → raw XML bytes.
    elements: HashMap<u32, Vec<u8>>,
}

/// All cooked game data, loaded from `data/cache/*.pak` at startup.
///
/// Maps category_id → { elementId → raw XML bytes }.
#[derive(Clone)]
struct ResourceCache {
    categories: Arc<HashMap<u32, CategoryData>>,
}

/// Category ID → PAK filename mapping (from `resource.cpp`).
const CATEGORY_PAKS: &[(u32, &str)] = &[
    (1, "CookedDataKismetSeqEvent.pak"),
    (2, "CookedDataAbilities.pak"),
    (3, "CookedDataMissions.pak"),
    (4, "CookedDataItems.pak"),
    (5, "CookedDataDialogs.pak"),
    (6, "CookedDataKismetSetEvent.pak"),
    (7, "CookedCharCreation.pak"),
    (8, "CookedInteractionSet.pak"),
    (9, "CookedDataEffects.pak"),
    (10, "TextStrings.pak"),
    (11, "ErrorStrings.pak"),
    (12, "CookedWorldInfo.pak"),
    (13, "CookedDataStargates.pak"),
    (14, "CookedDataContainers.pak"),
    (15, "CookedBlueprints.pak"),
    (16, "CookedSciences.pak"),
    (17, "CookedDisciplines.pak"),
    (18, "CookedParadigm.pak"),
    (19, "SpecialWords.pak"),
    (20, "CookedInteractions.pak"),
];

impl ResourceCache {
    /// Load all PAK files from the given directory.
    fn load_all(data_dir: &str) -> Result<Self, String> {
        let mut categories = HashMap::new();

        for &(cat_id, filename) in CATEGORY_PAKS {
            let pak_path = format!("{}/{}", data_dir, filename);
            match Self::load_pak(&pak_path) {
                Ok(cat_data) => {
                    tracing::info!(
                        category = cat_id,
                        file = filename,
                        elements = cat_data.elements.len(),
                        metadata = cat_data.metadata,
                        "Loaded PAK"
                    );
                    categories.insert(cat_id, cat_data);
                }
                Err(e) => {
                    tracing::warn!(category = cat_id, file = filename, "Failed to load PAK: {e}");
                }
            }
        }

        tracing::info!(
            categories = categories.len(),
            total_elements = categories.values().map(|c| c.elements.len()).sum::<usize>(),
            "Resource cache loaded"
        );

        Ok(Self {
            categories: Arc::new(categories),
        })
    }

    /// Load a single PAK file (ZIP archive) into a CategoryData.
    fn load_pak(pak_path: &str) -> Result<CategoryData, String> {
        let file = std::fs::File::open(pak_path)
            .map_err(|e| format!("Failed to open {pak_path}: {e}"))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Failed to read ZIP {pak_path}: {e}"))?;

        let mut elements = HashMap::new();
        let mut metadata: u32 = 0;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)
                .map_err(|e| format!("ZIP entry {i}: {e}"))?;
            let name = entry.name().to_string();

            if name == "MetaData" {
                let mut buf = [0u8; 4];
                if entry.read_exact(&mut buf).is_ok() {
                    metadata = u32::from_le_bytes(buf);
                }
            } else if let Some(id_str) = name.strip_prefix('_') {
                if let Ok(id) = id_str.parse::<u32>() {
                    let mut data = Vec::with_capacity(entry.size() as usize);
                    entry.read_to_end(&mut data)
                        .map_err(|e| format!("Failed to read entry {name}: {e}"))?;
                    elements.insert(id, data);
                }
            }
        }

        Ok(CategoryData { metadata, elements })
    }

    /// Get a category's data.
    fn category(&self, category_id: u32) -> Option<&CategoryData> {
        self.categories.get(&category_id)
    }

    /// Get XML data for a given category + element.
    fn get(&self, category_id: u32, element_id: u32) -> Option<&Vec<u8>> {
        self.categories.get(&category_id)?.elements.get(&element_id)
    }
}

// ── Per-connection state ──────────────────────────────────────────────────────

/// State held for each client that has completed the Phase 3 handshake.
struct ConnectedClientState {
    /// Encryption context (for decrypting client packets).
    enc: MercuryEncryption,
    /// Raw 32-byte AES-256 key (for spawning the tick-sync task).
    key: [u8; 32],
    /// Account ID from the pending login (FK to account table).
    account_id: u32,
    /// `true` once the Phase 4 character list has been sent to this client.
    char_list_sent: bool,
    /// `true` once the Phase 5a world entry packet (viewport+cell+position+reset) has been sent.
    world_entry_sent: bool,
    /// Player entity ID for Phase 5b (CREATE_BASE_PLAYER), set after Phase 5a.
    /// Consumed by `handle_enable_entities` Phase 5b via `.take()`.
    pending_player_entity_id: Option<u32>,
    /// Allocated player entity ID (persisted for cleanup on disconnect).
    player_entity_id: Option<u32>,
    /// Shared outgoing sequence counter.  Used by both the tick-sync loop
    /// and one-shot senders (Phase 4/5) to avoid seq collisions.
    next_seq: Arc<AtomicU32>,
    /// Client reliable-message sequence IDs that need to be ACKed.
    /// Drained by the tick-sync loop (or the char_list response) and
    /// piggybacked on the next outgoing packet.
    pending_acks: Arc<Mutex<Vec<u32>>>,
    /// Timestamp of the last received packet from this client.
    /// Used by the tick-sync loop to detect dead clients.
    last_recv: Arc<Mutex<Instant>>,
    /// Dynamically allocated Account entity ID for this session.
    account_entity_id: u32,
    /// Monotonic counter for resource fragment dataId values.
    next_data_id: u16,
    /// World entry info for Phase 5b (viewport + cell + position), set after Phase 5a.
    /// Consumed by `handle_enable_entities` Phase 5b via `.take()`.
    pending_world_entry: Option<WorldEntryInfo>,
    /// Full player load data for the mapLoaded sequence, set after Phase 5a.
    /// Consumed by `handle_enable_entities` Phase 5b via `.take()`.
    pending_player_load_data: Option<PlayerLoadData>,
    /// Set to `true` by `handle_log_off` to signal the tick-sync loop to stop.
    cancelled: Arc<AtomicBool>,
    /// Player character name, set during world entry for chat routing.
    player_name: Option<String>,
}

// ── BaseService ───────────────────────────────────────────────────────────────

/// BaseApp service — manages persistent entity state and client connections.
pub struct BaseService {
    /// Address the Mercury UDP listener binds to.
    pub listener_addr: SocketAddr,

    /// Whether the service is currently running.
    pub is_running: bool,

    /// Pending login handoffs shared with AuthService (ticket → PendingLogin).
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
        }
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

    /// Wire in the Base↔Cell channels. Called by the orchestrator before `start()`.
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

        // Create shared state accessible by both the connect loop and cell message handler.
        let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let entity_manager: Arc<Mutex<EntityManager>> =
            Arc::new(Mutex::new(EntityManager::new()));
        // Reverse index: entity_id → SocketAddr (populated during Phase 5b).
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

// ── UDP receive loop ──────────────────────────────────────────────────────────

/// Main receive loop — one per running `BaseService`.
async fn run_connect_loop(
    socket: Arc<UdpSocket>,
    pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
    db_pool: Option<Arc<PgPool>>,
    resource_cache: Option<Arc<ResourceCache>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: Arc<Mutex<EntityManager>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {

    let mut buf = [0u8; 4096];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, addr)) => {
                tracing::trace!(%addr, len, hex = %to_hex(&buf[..len]), "UDP_IN");
                if let Err(e) = handle_datagram(
                    &socket,
                    addr,
                    &buf[..len],
                    &pending_logins,
                    &connected,
                    &db_pool,
                    &resource_cache,
                    &entity_manager,
                    &cell_tx,
                    &entity_to_addr,
                )
                .await
                {
                    tracing::warn!(%addr, "Datagram handler error: {e}");
                }
            }
            Err(e) => {
                // On Windows, WSAECONNRESET (10054) arrives on the UDP socket
                // when a previous send_to targeted a closed port (ICMP port
                // unreachable).  This is per-destination, NOT a socket failure.
                if e.raw_os_error() == Some(10054) {
                    tracing::debug!("UDP recv: WSAECONNRESET (10054) — remote closed, continuing");
                    continue;
                }
                tracing::error!("Base service UDP recv error (fatal): {e}");
                break;
            }
        }
    }
}

/// Dispatch a single incoming UDP datagram.
async fn handle_datagram(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    raw: &[u8],
    pending_logins: &Arc<Mutex<HashMap<String, PendingLogin>>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    resource_cache: &Option<Arc<ResourceCache>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if raw.is_empty() {
        return Ok(());
    }

    // Check for an established encrypted channel first.
    let channel_key: Option<(MercuryEncryption, [u8; 32], u32, Arc<Mutex<Vec<u32>>>, Arc<Mutex<Instant>>)> = {
        let clients = connected
            .lock()
            .map_err(|_| "connected lock poisoned")?;
        clients.get(&addr).map(|c| {
            (
                c.enc.clone(),
                c.key,
                c.account_id,
                Arc::clone(&c.pending_acks),
                Arc::clone(&c.last_recv),
            )
        })
    };

    if let Some((enc, key, account_id, pending_acks, last_recv)) = channel_key {
        // Update last-recv timestamp on every packet from this client.
        *last_recv.lock().unwrap() = Instant::now();
        return handle_encrypted_datagram(
            socket, addr, raw, enc, key, account_id,
            &pending_acks, connected, db_pool, resource_cache, entity_manager, cell_tx,
            entity_to_addr,
        ).await;
    }

    // Not yet connected — only accept the unencrypted Phase 3 login (flags=0x41).
    let login_flags = FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE; // 0x41
    if raw[0] != login_flags {
        tracing::trace!(%addr, flags = raw[0], "Ignoring packet from unknown addr (flags={:#04x})", raw[0]);
        return Ok(());
    }

    match parse_baseapp_login(raw) {
        Ok((request_id, ticket_str)) => {
            tracing::info!(%addr, ticket = %ticket_str, "baseAppLogin received");
            handle_login(socket, addr, request_id, &ticket_str, pending_logins, connected, entity_manager, cell_tx, entity_to_addr).await
        }
        Err(e) => {
            tracing::warn!(%addr, "Failed to parse baseAppLogin: {e}");
            Ok(())
        }
    }
}

/// Handle an encrypted datagram from a known connected client.
async fn handle_encrypted_datagram(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    raw: &[u8],
    enc: MercuryEncryption,
    key: [u8; 32],
    account_id: u32,
    pending_acks: &Arc<Mutex<Vec<u32>>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    resource_cache: &Option<Arc<ResourceCache>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let plaintext = match enc.decrypt(raw) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(%addr, "Decryption failed (bad HMAC?): {e}");
            return Ok(());
        }
    };

    tracing::trace!(%addr, len = plaintext.len(), hex = %to_hex(&plaintext), "DECRYPT_OK");

    let pkt = match parse_incoming(&plaintext) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(%addr, "Packet parse failed after decrypt: {e}");
            return Ok(());
        }
    };

    tracing::debug!(
        %addr,
        flags = pkt.flags,
        body_len = pkt.body.len(),
        seq = ?pkt.seq_id,
        acks = ?pkt.acks,
        "Decrypted packet received"
    );

    // Queue an ACK for any reliable message the client sends.
    if pkt.flags & FLAG_RELIABLE != 0 {
        if let Some(seq) = pkt.seq_id {
            tracing::trace!(%addr, client_seq = seq, "Queueing ACK for client reliable message");
            pending_acks.lock().unwrap().push(seq);
        }
    }

    // Parse the client bundle.
    let body = &pkt.body;
    if body.is_empty() {
        return Ok(());
    }

    let mut offset = 0;

    // First message may be authenticate (0x01, WORD_LENGTH).
    // The C++ reference server ignores this message — entity creation happens
    // on ENABLE_ENTITIES (0x08) so the client's entity system is ready.
    if body[offset] == 0x01 {
        offset += 1; // skip msg_id
        if offset + 2 <= body.len() {
            let word_len = u16::from_le_bytes([body[offset], body[offset + 1]]) as usize;
            offset += 2 + word_len;
        }
        tracing::debug!(%addr, "AUTHENTICATE received — ignored (entity created on ENABLE_ENTITIES)");

        if offset >= body.len() {
            return Ok(());
        }
    }

    // Scan remaining messages in the bundle.
    //
    // Client messages come in two flavours:
    //   - System messages (0x00–0x0D): use CONSTANT_LENGTH or WORD_LENGTH per the
    //     ClientMessageList table in messages.cpp.
    //   - Entity method calls (0xC0+): always WORD_LENGTH (u16 prefix).
    while offset < body.len() {
        let msg_id = body[offset];
        offset += 1;

        // Determine payload length based on message format.
        // System messages (0x00-0x0D) have defined formats; entity methods use WORD_LENGTH.
        let payload_result = match msg_id {
            // --- System messages with CONSTANT_LENGTH ---
            // 0x02: AVATAR_UPD_IMPLICIT (CONSTANT_LENGTH = 36)
            0x02 => read_constant_payload(body, &mut offset, 36),
            // 0x03: AVATAR_UPDATE_EXPLICIT (CONSTANT_LENGTH = 40)
            0x03 => read_constant_payload(body, &mut offset, 40),
            // 0x04: AVATAR_UPDW_IMPLICIT (CONSTANT_LENGTH = 36)
            0x04 => read_constant_payload(body, &mut offset, 36),
            // 0x05: AVATAR_UPDW_EXPLICIT (CONSTANT_LENGTH = 40)
            0x05 => read_constant_payload(body, &mut offset, 40),
            // 0x06: SWITCH_INTERFACE (CONSTANT_LENGTH = 0)
            0x06 => read_constant_payload(body, &mut offset, 0),
            // 0x08: ENABLE_ENTITIES (CONSTANT_LENGTH = 8)
            0x08 => read_constant_payload(body, &mut offset, 8),
            // 0x09: VIEWPORT_ACK (CONSTANT_LENGTH = 8)
            0x09 => read_constant_payload(body, &mut offset, 8),
            // 0x0A: VEHICLE_ACK (CONSTANT_LENGTH = 8)
            0x0A => read_constant_payload(body, &mut offset, 8),
            // 0x0C: DISCONNECT (CONSTANT_LENGTH = 1)
            0x0C => read_constant_payload(body, &mut offset, 1),

            // --- System messages with WORD_LENGTH ---
            // 0x07: REQUEST_ENTITY_UPDATE (WORD_LENGTH)
            0x07 => read_word_length_payload(body, &mut offset),
            // 0x0B: RESTORE_CLIENT_ACK (WORD_LENGTH)
            0x0B => read_word_length_payload(body, &mut offset),

            // --- Entity method calls (0xC0+): always WORD_LENGTH ---
            _ => read_word_length_payload(body, &mut offset),
        };

        let payload = match payload_result {
            Some(p) => p,
            None => {
                tracing::trace!(%addr, msg_id = format_args!("{:#04x}", msg_id), "Bundle truncated");
                break;
            }
        };

        tracing::debug!(%addr, msg_id = format_args!("{:#04x}", msg_id), payload_len = payload.len(), "Client bundle message");

        // Dispatch message.
        //
        // Entity method indices use EXPOSED-ONLY ordering from entity defs.
        // ClientCache interface: 0xC0=versionInfoRequest, 0xC1=elementDataRequest
        // Account base methods (non-exposed logOffInternal & onPlayerFailedToLoad SKIPPED):
        //   0xC2=logOff, 0xC3=createCharacter, 0xC4=playCharacter,
        //   0xC5=deleteCharacter, 0xC6=requestCharacterVisuals, 0xC7=onClientVersion
        match msg_id {
            // ── System messages ──
            // ENABLE_ENTITIES (0x08) — client re-enables entity system after RESET_ENTITIES
            0x08 => {
                tracing::info!(%addr, "Client sent ENABLE_ENTITIES");
                handle_enable_entities(socket, addr, key, account_id, connected, db_pool, entity_manager, cell_tx, entity_to_addr).await?;
            }
            // AVATAR_UPDATE_EXPLICIT (0x03) — client movement update (40 bytes)
            0x03 => {
                if payload.len() >= 40 {
                    if let Some(ref tx) = cell_tx {
                        let entity_id = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                        // payload[4..8] = viewport_id (unused here)
                        let pos = [
                            f32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]),
                            f32::from_le_bytes([payload[12], payload[13], payload[14], payload[15]]),
                            f32::from_le_bytes([payload[16], payload[17], payload[18], payload[19]]),
                        ];
                        let vel = [
                            f32::from_le_bytes([payload[20], payload[21], payload[22], payload[23]]),
                            f32::from_le_bytes([payload[24], payload[25], payload[26], payload[27]]),
                            f32::from_le_bytes([payload[28], payload[29], payload[30], payload[31]]),
                        ];
                        let dir = [payload[32] as i8, payload[33] as i8, payload[34] as i8];
                        tracing::trace!(entity_id, ?pos, "AVATAR_UPDATE_EXPLICIT → CellService");
                        let _ = tx.send(BaseToCellMsg::EntityMove {
                            entity_id,
                            position: pos,
                            direction: dir,
                            velocity: vel,
                        }).await;
                    }
                }
            }
            // DISCONNECT (0x0C)
            0x0C => {
                tracing::info!(%addr, "Client sent DISCONNECT");
                destroy_client_entities(connected, entity_manager, addr, cell_tx, entity_to_addr);
            }
            // VIEWPORT_ACK (0x09)
            0x09 => {
                tracing::trace!(%addr, "Client sent VIEWPORT_ACK");
            }

            // ── Entity base method calls (0xC0+) ──
            //
            // The dispatch table changes based on which entity type the client controls:
            //
            // ACCOUNT entity (character select):
            //   ClientCache: 0xC0=versionInfoRequest, 0xC1=elementDataRequest
            //   Account:     0xC2=logOff, 0xC3=createCharacter, 0xC4=playCharacter,
            //                0xC5=deleteCharacter, 0xC6=requestCharacterVisuals, 0xC7=onClientVersion
            //
            // SGWPLAYER entity (in-world):
            //   Communicator: 0xC0=chatJoin, 0xC1=chatLeave, 0xC2=sendPlayerCommunication,
            //                 0xC3=chatSetAFKMessage, 0xC4=chatSetDNDMessage, ...
            //   (Other interfaces and SGWPlayer own methods at higher indices)
            id if id >= 0xC0 => {
                let (in_world, player_name) = {
                    let clients = connected.lock().unwrap();
                    match clients.get(&addr) {
                        Some(c) => (c.player_entity_id.is_some(), c.player_name.clone()),
                        None => (false, None),
                    }
                };

                if in_world {
                    // SGWPlayer base method dispatch
                    dispatch_sgw_player_base_method(
                        id, payload, &player_name, addr, socket, key,
                        connected, entity_manager, cell_tx, entity_to_addr,
                    ).await?;
                } else {
                    // Account base method dispatch
                    match id {
                        0xC0 => {
                            handle_version_info_request(
                                socket, addr, key, payload, connected, resource_cache,
                            ).await?;
                        }
                        0xC1 => {
                            handle_element_data_request(
                                socket, addr, key, payload, connected, resource_cache,
                            ).await?;
                        }
                        0xC2 => {
                            handle_log_off(
                                socket, addr, key, connected, entity_manager, cell_tx, entity_to_addr,
                            ).await?;
                        }
                        0xC3 => {
                            tracing::info!(%addr, "Client requests createCharacter");
                            handle_create_character(
                                socket, addr, key, account_id, payload,
                                connected, db_pool,
                            ).await?;
                        }
                        0xC4 => {
                            let player_id = if payload.len() >= 4 {
                                i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]])
                            } else {
                                0
                            };
                            tracing::info!(%addr, player_id, "Client requests playCharacter");
                            handle_play_character(socket, addr, key, account_id, player_id, connected, db_pool, entity_manager, cell_tx).await?;
                        }
                        0xC5 => {
                            let player_id = if payload.len() >= 4 {
                                i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]])
                            } else { 0 };
                            tracing::info!(%addr, player_id, "Client requests deleteCharacter");
                            handle_delete_character(
                                socket, addr, key, account_id, player_id,
                                connected, db_pool,
                            ).await?;
                        }
                        0xC6 => {
                            let player_id = if payload.len() >= 4 {
                                i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]])
                            } else { 0 };
                            tracing::debug!(%addr, player_id, "Client sent requestCharacterVisuals");
                            handle_request_character_visuals(
                                socket, addr, key, player_id, connected, db_pool,
                            ).await?;
                        }
                        0xC7 => {
                            tracing::debug!(%addr, "Client sent onClientVersion — acknowledged");
                        }
                        _ => {
                            tracing::trace!(%addr, msg_id = format_args!("{:#04x}", id), "Unhandled Account base method");
                        }
                    }
                }
            }
            // ── Cell entity method calls (0x80-0xBF range) ──
            // After world entry, the client sends cell method calls as:
            //   Direct:   msg_id = cellMethodIndex | 0x80 (indices 0-60)
            //   Extended: msg_id = 0xBD, payload contains subIndex
            // These are forwarded to the CellService for dispatch.
            id if (0x80..=0xBF).contains(&id) => {
                let player_eid = {
                    let clients = connected.lock().unwrap();
                    clients.get(&addr).and_then(|c| c.player_entity_id)
                };
                if let Some(player_eid) = player_eid {
                    if id == 0xBD {
                        // Extended encoding: subIndex is first byte of payload
                        if !payload.is_empty() {
                            let sub_index = payload[0] as u16;
                            let args = if payload.len() > 1 { payload[1..].to_vec() } else { Vec::new() };
                            if let Some(ref tx) = cell_tx {
                                let _ = tx.send(BaseToCellMsg::CellMethodCall {
                                    entity_id: player_eid,
                                    method_index: sub_index + 61,
                                    args,
                                }).await;
                            }
                        }
                    } else {
                        let method_index = (id - 0x80) as u16;
                        if let Some(ref tx) = cell_tx {
                            let _ = tx.send(BaseToCellMsg::CellMethodCall {
                                entity_id: player_eid,
                                method_index,
                                args: payload.to_vec(),
                            }).await;
                        }
                    }
                } else {
                    tracing::trace!(%addr, msg_id = format_args!("{:#04x}", id), "Cell method before world entry — ignored");
                }
            }
            _ => {
                tracing::trace!(%addr, msg_id = format_args!("{:#04x}", msg_id), payload_len = payload.len(), "Unhandled client message");
            }
        }
    }

    Ok(())
}

/// Read a CONSTANT_LENGTH payload (no length prefix, fixed size).
fn read_constant_payload<'a>(body: &'a [u8], offset: &mut usize, size: usize) -> Option<&'a [u8]> {
    if *offset + size > body.len() {
        return None;
    }
    let payload = &body[*offset..*offset + size];
    *offset += size;
    Some(payload)
}

/// Read a WORD_LENGTH payload (u16 length prefix).
fn read_word_length_payload<'a>(body: &'a [u8], offset: &mut usize) -> Option<&'a [u8]> {
    if *offset + 2 > body.len() {
        return None;
    }
    let word_len = u16::from_le_bytes([body[*offset], body[*offset + 1]]) as usize;
    *offset += 2;
    if *offset + word_len > body.len() {
        return None;
    }
    let payload = &body[*offset..*offset + word_len];
    *offset += word_len;
    Some(payload)
}

// ── Phase 4: character list ───────────────────────────────────────────────────

/// Query the character list from the database.
async fn query_character_list(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
) -> Vec<CharacterInfo> {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!("No DB pool — returning empty character list");
            return Vec::new();
        }
    };

    #[derive(sqlx::FromRow)]
    struct CharRow {
        player_id: i32,
        player_name: String,
        extra_name: String,
        alignment: i32,
        level: i32,
        gender: i32,
        world_location: String,
        archetype: i32,
        title: i32,
    }

    tracing::debug!(account_id, "Querying sgw_player for character list");

    match sqlx::query_as::<_, CharRow>(
        "SELECT player_id, player_name, extra_name, alignment, level, gender, \
         world_location, archetype, title \
         FROM sgw_player WHERE account_id = $1 ORDER BY player_id",
    )
    .bind(account_id as i32)
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(rows) => {
            tracing::info!(account_id, count = rows.len(), "Character list query result");
            rows.into_iter()
                .map(|r| CharacterInfo {
                    player_id: r.player_id,
                    name: r.player_name,
                    extra_name: r.extra_name,
                    alignment: r.alignment as u8,
                    level: r.level as u8,
                    gender: r.gender as u8,
                    world_location: r.world_location,
                    archetype: r.archetype as u8,
                    title: r.title as u8,
                    player_type: 1,
                    playable: 1,
                })
                .collect()
        }
        Err(e) => {
            tracing::error!(account_id, "Failed to query character list: {e}");
            Vec::new()
        }
    }
}

// ── Character creation ───────────────────────────────────────────────────────

/// Handle `createCharacter` (0xC4) — parse args and INSERT into sgw_player.
async fn handle_create_character(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    payload: &[u8],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::warn!(%addr, "createCharacter: no DB pool");
            send_char_create_failed(socket, addr, key, connected, 3).await?;
            return Ok(());
        }
    };

    // Parse createCharacter args (from Account.def):
    // [WSTRING Name][WSTRING ExtraName][INT32 CharDefId][ARRAY<VisualChoices> VisualChoiceList][INT32 SkinTintColorID]
    let mut off = 0;

    let (name, consumed) = match read_wstring(payload, off) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(%addr, "createCharacter: failed to parse name: {e}");
            send_char_create_failed(socket, addr, key, connected, 2).await?;
            return Ok(());
        }
    };
    off += consumed;

    // Name length validation (matches C++ Account.py: rejects names < 3 chars).
    if name.len() < 3 {
        tracing::info!(%addr, name_len = name.len(), "createCharacter: name too short");
        send_char_create_failed(socket, addr, key, connected, 2).await?;
        return Ok(());
    }

    let (extra_name, consumed) = match read_wstring(payload, off) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(%addr, "createCharacter: failed to parse extraName: {e}");
            send_char_create_failed(socket, addr, key, connected, 2).await?;
            return Ok(());
        }
    };
    off += consumed;

    if off + 4 > payload.len() {
        tracing::warn!(%addr, "createCharacter: payload too short for CharDefId");
        send_char_create_failed(socket, addr, key, connected, 2).await?;
        return Ok(());
    }
    let char_def_id = i32::from_le_bytes([
        payload[off], payload[off + 1], payload[off + 2], payload[off + 3],
    ]);
    off += 4;

    // Parse ARRAY<VisualChoices> — count + entries
    if off + 4 > payload.len() {
        tracing::warn!(%addr, "createCharacter: payload too short for visuals count");
        send_char_create_failed(socket, addr, key, connected, 2).await?;
        return Ok(());
    }
    let visual_count = u32::from_le_bytes([
        payload[off], payload[off + 1], payload[off + 2], payload[off + 3],
    ]) as usize;
    off += 4;
    // Each VisualChoices = { VisGroupId: INT32, ChoiceId: INT32 } = 8 bytes
    if off + visual_count * 8 > payload.len() {
        tracing::warn!(%addr, "createCharacter: payload too short for visual choices");
        send_char_create_failed(socket, addr, key, connected, 2).await?;
        return Ok(());
    }
    let mut visual_choices: Vec<(i32, i32)> = Vec::with_capacity(visual_count);
    for _ in 0..visual_count {
        let vis_group_id = i32::from_le_bytes([
            payload[off], payload[off + 1], payload[off + 2], payload[off + 3],
        ]);
        off += 4;
        let choice_id = i32::from_le_bytes([
            payload[off], payload[off + 1], payload[off + 2], payload[off + 3],
        ]);
        off += 4;
        visual_choices.push((vis_group_id, choice_id));
    }

    if off + 4 > payload.len() {
        tracing::warn!(%addr, "createCharacter: payload too short for SkinTintColorID");
        send_char_create_failed(socket, addr, key, connected, 2).await?;
        return Ok(());
    }
    let skin_tint_color_id = i32::from_le_bytes([
        payload[off], payload[off + 1], payload[off + 2], payload[off + 3],
    ]);

    // Derive alignment, archetype, gender, bodyset, starting position from CharDefId.
    let (alignment, archetype, gender, bodyset, world_location, start_x, start_y, start_z) =
        match chardef_lookup(char_def_id) {
            Some(info) => info,
            None => {
                tracing::warn!(%addr, char_def_id, "createCharacter: unknown CharDefId");
                send_char_create_failed(socket, addr, key, connected, 2).await?;
                return Ok(());
            }
        };

    tracing::info!(
        %addr,
        name = %name,
        extra_name = %extra_name,
        char_def_id,
        alignment,
        archetype,
        gender,
        bodyset,
        visual_count = visual_choices.len(),
        skin_tint_color_id,
        "Creating character"
    );

    // ── Resolve visual choices (matches CharacterCreation.py:getAllChoices) ───

    // Query all visual groups and their choices for this char_def_id
    let vg_rows = sqlx::query_as::<_, (i32, String, Option<i32>, Option<String>, Option<i32>, Option<bool>, Option<i32>)>(
        "SELECT vg.vis_group_id, vg.vis_type::text, \
                c.choice_id, c.component, c.item_id, c.item_bound, c.item_durability \
         FROM resources.char_creation_visgroups vg \
         LEFT JOIN resources.char_creation_choices c ON c.vis_group_id = vg.vis_group_id \
         WHERE vg.char_def_id = $1 \
         ORDER BY vg.vis_group_id, c.choice_id",
    )
    .bind(char_def_id)
    .fetch_all(pool.as_ref())
    .await;

    let vg_rows = match vg_rows {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(%addr, error = %e, "createCharacter: failed to query visgroups");
            send_char_create_failed(socket, addr, key, connected, 3).await?;
            return Ok(());
        }
    };

    // Build visgroup map: vis_group_id → (vis_type, choices ordered by choice_id)
    struct ChoiceData {
        component: String,
        item_id: Option<i32>,
        item_bound: bool,
        item_durability: i32,
    }
    struct VisGroup {
        vis_type: String,
        choices: std::collections::BTreeMap<i32, ChoiceData>,
    }
    let mut visgroups: std::collections::BTreeMap<i32, VisGroup> = std::collections::BTreeMap::new();
    for (vg_id, vis_type, choice_id, component, item_id, item_bound, item_durability) in &vg_rows {
        let group = visgroups.entry(*vg_id).or_insert_with(|| VisGroup {
            vis_type: vis_type.clone(),
            choices: std::collections::BTreeMap::new(),
        });
        if let (Some(cid), Some(comp)) = (choice_id, component) {
            group.choices.insert(*cid, ChoiceData {
                component: comp.clone(),
                item_id: *item_id,
                item_bound: item_bound.unwrap_or(false),
                item_durability: item_durability.unwrap_or(-1),
            });
        }
    }

    // Validate client-provided choices and resolve forced groups
    struct ResolvedChoice {
        component: String,
        item_id: Option<i32>,
        item_bound: bool,
        item_durability: i32,
    }
    let mut resolved: HashMap<i32, ResolvedChoice> = HashMap::new();

    // Client choices must target VIS_Optional groups only
    for &(vg_id, choice_id) in &visual_choices {
        let group = match visgroups.get(&vg_id) {
            Some(g) => g,
            None => {
                tracing::warn!(%addr, vg_id, char_def_id, "Invalid visual group");
                send_char_create_failed(socket, addr, key, connected, 10003).await?;
                return Ok(());
            }
        };
        if group.vis_type != "VIS_Optional" {
            tracing::warn!(%addr, vg_id, "Choice not allowed for forced visual group");
            send_char_create_failed(socket, addr, key, connected, 10003).await?;
            return Ok(());
        }
        let choice = match group.choices.get(&choice_id) {
            Some(c) => c,
            None => {
                tracing::warn!(%addr, vg_id, choice_id, "Invalid choice for visual group");
                send_char_create_failed(socket, addr, key, connected, 10003).await?;
                return Ok(());
            }
        };
        resolved.insert(vg_id, ResolvedChoice {
            component: choice.component.clone(),
            item_id: choice.item_id,
            item_bound: choice.item_bound,
            item_durability: choice.item_durability,
        });
    }

    // Auto-select forced groups; reject missing optional groups
    for (&vg_id, group) in &visgroups {
        if !resolved.contains_key(&vg_id) {
            if group.vis_type == "VIS_Forced" {
                if let Some((_, choice)) = group.choices.iter().next() {
                    resolved.insert(vg_id, ResolvedChoice {
                        component: choice.component.clone(),
                        item_id: choice.item_id,
                        item_bound: choice.item_bound,
                        item_durability: choice.item_durability,
                    });
                }
            } else {
                tracing::warn!(%addr, vg_id, char_def_id, "Missing choice for optional visual group");
                send_char_create_failed(socket, addr, key, connected, 10000).await?;
                return Ok(());
            }
        }
    }

    // ── Separate body components from item components (Account.py:156-161) ───

    let mut body_components: Vec<String> = Vec::new();
    struct ItemChoice { item_id: i32, item_bound: bool, item_durability: i32 }
    let mut item_choices: Vec<ItemChoice> = Vec::new();

    for choice in resolved.values() {
        if let Some(item_id) = choice.item_id {
            item_choices.push(ItemChoice {
                item_id,
                item_bound: choice.item_bound,
                item_durability: choice.item_durability,
            });
        } else {
            body_components.push(choice.component.clone());
        }
    }

    // ── Look up world_id (Account.py:163) ───

    let world_id: Option<i32> = sqlx::query_scalar(
        "SELECT world_id FROM resources.worlds WHERE world = $1",
    )
    .bind(world_location)
    .fetch_optional(pool.as_ref())
    .await
    .unwrap_or(None);

    // ── Look up starting abilities (Account.py:166) ───

    let abilities: Vec<i32> = sqlx::query_scalar(
        "SELECT ability_id FROM resources.char_creation_abilities WHERE char_def_id = $1",
    )
    .bind(char_def_id)
    .fetch_all(pool.as_ref())
    .await
    .unwrap_or_default();

    tracing::debug!(
        %addr, char_def_id,
        components = ?body_components,
        item_count = item_choices.len(),
        world_id = ?world_id,
        ability_count = abilities.len(),
        "Resolved character creation visuals"
    );

    // ── INSERT into sgw_player with components, world_id, abilities ───

    let result = sqlx::query_scalar::<_, i32>(
        "INSERT INTO sgw_player \
         (account_id, player_name, extra_name, alignment, archetype, gender, \
          world_location, bodyset, level, title, pos_x, pos_y, pos_z, \
          skin_color_id, components, world_id, abilities) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1, 0, $9, $10, $11, $12, $13, $14, $15) \
         RETURNING player_id",
    )
    .bind(account_id as i32)
    .bind(&name)
    .bind(&extra_name)
    .bind(alignment)
    .bind(archetype)
    .bind(gender)
    .bind(world_location)
    .bind(bodyset)
    .bind(start_x)
    .bind(start_y)
    .bind(start_z)
    .bind(skin_tint_color_id)
    .bind(&body_components)
    .bind(world_id)
    .bind(&abilities)
    .fetch_one(pool.as_ref())
    .await;

    match result {
        Ok(player_id) => {
            // ── Insert starter items into sgw_inventory (Account.py:182-207) ───
            let mut slot_indices: HashMap<i32, i32> = HashMap::new();
            for item in &item_choices {
                // Look up which containers this item can go into
                let container_sets = sqlx::query_scalar::<_, Vec<i32>>(
                    "SELECT container_sets FROM resources.items WHERE item_id = $1",
                )
                .bind(item.item_id)
                .fetch_optional(pool.as_ref())
                .await
                .ok()
                .flatten()
                .unwrap_or_default();

                // Find the best container from BagFillOrder
                let bag_id = match BAG_FILL_ORDER.iter().find(|&&bag| container_sets.contains(&bag)) {
                    Some(&bag) => bag,
                    None => {
                        tracing::warn!(%addr, item_id = item.item_id, "No valid container for starter item");
                        continue;
                    }
                };

                let entry = slot_indices.entry(bag_id).or_insert_with(|| {
                    if bag_id == INV_BANDOLIER { 1 } else { 0 }
                });
                let current_slot = *entry;
                *entry += 1;

                if *entry > bag_max_slots(bag_id) {
                    tracing::warn!(%addr, bag_id, item_id = item.item_id, "Bag full, skipping starter item");
                    continue;
                }

                if let Err(e) = sqlx::query(
                    "INSERT INTO sgw_inventory \
                     (container_id, slot_id, type_id, character_id, durability, bound) \
                     VALUES ($1, $2, $3, $4, $5, $6)",
                )
                .bind(bag_id)
                .bind(current_slot)
                .bind(item.item_id)
                .bind(player_id)
                .bind(item.item_durability)
                .bind(item.item_bound)
                .execute(pool.as_ref())
                .await
                {
                    tracing::error!(%addr, item_id = item.item_id, error = %e, "Failed to insert starter item");
                }
            }

            tracing::info!(%addr, player_id, name = %name, "Character created successfully");

            // Send updated character list (Account entity already exists)
            let characters = query_character_list(db_pool, account_id).await;
            let account_eid = get_account_entity_id(connected, addr)?;
            let (acks, seq) = drain_acks_and_seq(connected, addr)?;
            let pkt = build_on_character_list(&key, seq, &acks, &characters, account_eid);
            tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT updated char_list");
            socket.send_to(&pkt, addr).await?;
        }
        Err(e) => {
            let error_str = e.to_string();
            let error_code = if error_str.contains("sgw_player_player_name_key") {
                tracing::info!(%addr, name = %name, "Character name already taken");
                1 // name taken
            } else {
                tracing::error!(%addr, error = %e, "Character creation DB error");
                3 // DB error
            };
            send_char_create_failed(socket, addr, key, connected, error_code).await?;
        }
    }

    Ok(())
}

/// Send `onCharacterCreateFailed`.
async fn send_char_create_failed(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    error_code: i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let account_eid = get_account_entity_id(connected, addr)?;
    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    let pkt = build_char_create_failed(&key, seq, &acks, error_code, account_eid);
    socket.send_to(&pkt, addr).await?;
    Ok(())
}

// ── Character deletion ───────────────────────────────────────────────────────

/// Handle `deleteCharacter` (0xC5) — delete a character and send updated list.
async fn handle_delete_character(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    player_id: i32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::warn!(%addr, "deleteCharacter: no DB pool");
            return Ok(());
        }
    };

    // Delete the character (only if it belongs to this account).
    let result = sqlx::query(
        "DELETE FROM sgw_player WHERE player_id = $1 AND account_id = $2",
    )
    .bind(player_id)
    .bind(account_id as i32)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(r) => {
            if r.rows_affected() > 0 {
                tracing::info!(%addr, player_id, account_id, "Character deleted");
            } else {
                tracing::warn!(%addr, player_id, account_id, "Character not found or not owned");
            }
        }
        Err(e) => {
            tracing::error!(%addr, player_id, "Failed to delete character: {e}");
            return Ok(());
        }
    }

    // Send updated character list (Account entity already exists).
    let characters = query_character_list(db_pool, account_id).await;
    let account_eid = get_account_entity_id(connected, addr)?;
    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    let pkt = build_on_character_list(&key, seq, &acks, &characters, account_eid);
    tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT updated char_list after delete");
    socket.send_to(&pkt, addr).await?;

    Ok(())
}

// ── Character visuals ────────────────────────────────────────────────────────

/// Handle `requestCharacterVisuals` (0xC6).
///
/// Queries the DB for the player's bodyset, components, and skin color,
/// then sends `onCharacterVisuals` (0x84) so the client can render the
/// character model on the select screen.
async fn handle_request_character_visuals(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    player_id: i32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::warn!(%addr, player_id, "requestCharacterVisuals: no DB pool");
            return Ok(());
        }
    };

    // Ownership check: only return visuals for characters belonging to this account.
    let account_id = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        clients.get(&addr).ok_or("addr not in connected map")?.account_id
    };

    let row = sqlx::query_as::<_, (String, Vec<String>, i32, i32)>(
        "SELECT bodyset, components, skin_color_id, bandolier_slot \
         FROM sgw_player WHERE player_id = $1 AND account_id = $2",
    )
    .bind(player_id)
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await;

    match row {
        Ok(Some((bodyset, mut components, skin_color_id, bandolier_slot))) => {
            // Load item visual components from equipped inventory (Account.py:246-258)
            let item_visuals: Vec<String> = sqlx::query_scalar(
                "SELECT ri.visual_component \
                 FROM sgw_inventory inv \
                 JOIN resources.items ri ON ri.item_id = inv.type_id \
                 WHERE inv.character_id = $1 \
                   AND ri.visual_component IS NOT NULL \
                   AND ( \
                     (inv.container_id IN (3,4,5,6,7,8,9,10,11,12,13,14) AND inv.slot_id = 0) \
                     OR (inv.container_id = 3 AND inv.slot_id = $2) \
                   )",
            )
            .bind(player_id)
            .bind(bandolier_slot)
            .fetch_all(pool.as_ref())
            .await
            .unwrap_or_default();

            components.extend(item_visuals);

            tracing::debug!(
                %addr, player_id, %bodyset,
                component_count = components.len(),
                skin_color_id,
                "Sending character visuals"
            );

            let skin_tint = SKIN_TINTS.get(skin_color_id as usize).copied().unwrap_or(0x2F1308FF);
            let account_eid = get_account_entity_id(connected, addr)?;
            let (acks, seq) = drain_acks_and_seq(connected, addr)?;
            let pkt = build_character_visuals(
                &key, seq, &acks,
                player_id,
                &bodyset,
                &components,
                0xFF, // primaryTint (matches C++ Account.py)
                0xFF, // secondaryTint (matches C++ Account.py)
                skin_tint, // skinTint — RGBA from SKIN_TINTS lookup
                account_eid,
            );
            tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT onCharacterVisuals");
            socket.send_to(&pkt, addr).await?;
        }
        Ok(None) => {
            tracing::warn!(%addr, player_id, "requestCharacterVisuals: player not found");
        }
        Err(e) => {
            tracing::error!(%addr, player_id, error = %e, "requestCharacterVisuals: DB error");
        }
    }

    Ok(())
}

// ── Cooked data serving ──────────────────────────────────────────────────────

/// Handle `versionInfoRequest` (0xC0).
///
/// Client payload: [categoryId: u32][version: u32]
/// Response: onVersionInfo — if we have data for this category, tell the client
/// to invalidate and re-fetch; otherwise echo the client's version (cache OK).
async fn handle_version_info_request(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    payload: &[u8],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    resource_cache: &Option<Arc<ResourceCache>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if payload.len() < 8 {
        tracing::warn!(%addr, "versionInfoRequest: payload too short");
        return Ok(());
    }

    let category_id = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let client_version = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);

    // Version negotiation: echo back the PAK metadata version so the client
    // sees a match and keeps its local cache. The C++ server does the same
    // for categories not in `categoryMaps` (including category 7 char_creation).
    // Only invalidate when there's an actual version mismatch.
    let (version, required_updates, invalidate_all) = match resource_cache {
        Some(cache) => match cache.category(category_id) {
            Some(cat) => {
                // Never invalidate — we don't implement proactive resource push,
                // so invalidation would leave the client with an empty cache.
                // The client's shipped PAK files have the correct data.
                (cat.metadata, 0u32, false)
            }
            None => (client_version, 0u32, false),
        },
        None => (client_version, 0u32, false),
    };

    tracing::info!(
        %addr, category_id, client_version, version, required_updates,
        "Responding to versionInfoRequest"
    );

    let account_eid = get_account_entity_id(connected, addr)?;
    let (acks, seq) = drain_acks_and_seq(connected, addr)?;
    let pkt = build_version_info(&key, seq, &acks, category_id, version, required_updates, invalidate_all, account_eid);
    socket.send_to(&pkt, addr).await?;

    Ok(())
}

/// Proactively send all resources for a category as BASEMSG_RESOURCE_FRAGMENT packets.
///
/// The C++ server does this immediately after onVersionInfo when the client's cache
/// is stale, rather than waiting for individual elementDataRequests.
/// Currently unused — on-demand delivery via handle_element_data_request is used instead.
/// Kept for future use when flow-controlled proactive push is implemented.
#[allow(dead_code)]
async fn send_category_resources(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    category_id: u32,
    cat: &CategoryData,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const MAX_CHUNK: usize = 1000;

    // Sort element IDs for deterministic ordering.
    let mut element_ids: Vec<u32> = cat.elements.keys().copied().collect();
    element_ids.sort();

    for &element_id in &element_ids {
        let xml_data = match cat.elements.get(&element_id) {
            Some(data) => data,
            None => continue,
        };

        // Allocate a data_id for this transfer.
        let data_id = {
            let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
            if let Some(c) = clients.get_mut(&addr) {
                let id = c.next_data_id;
                c.next_data_id = c.next_data_id.wrapping_add(1);
                id
            } else {
                return Ok(());
            }
        };

        let chunks: Vec<&[u8]> = xml_data.chunks(MAX_CHUNK).collect();
        let total_chunks = chunks.len();

        for (i, chunk) in chunks.iter().enumerate() {
            let frag_flags = match (i == 0, i == total_chunks - 1) {
                (true, true) => FRAG_FIRST_AND_LAST,
                (true, false) => FRAG_FIRST,
                (false, true) => FRAG_LAST,
                (false, false) => FRAG_MIDDLE,
            };

            let (mt, cat_id, elem) = if i == 0 {
                (Some(0u8), Some(category_id), Some(element_id))
            } else {
                (None, None, None)
            };

            let (acks, seq) = drain_acks_and_seq(connected, addr)?;
            let pkt = build_resource_fragment(
                &key, seq, &acks,
                data_id, i as u8, frag_flags,
                mt, cat_id, elem, chunk,
            );
            socket.send_to(&pkt, addr).await?;
        }
    }

    tracing::debug!(
        %addr, category_id, count = element_ids.len(),
        "Proactively sent all resource fragments"
    );

    Ok(())
}

/// Handle `elementDataRequest` (0xC1).
///
/// Client payload: [categoryId: u32][key: u32]
/// Response: fragment the XML data for the requested element.
async fn handle_element_data_request(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    payload: &[u8],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    resource_cache: &Option<Arc<ResourceCache>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if payload.len() < 8 {
        tracing::warn!(%addr, "elementDataRequest: payload too short");
        return Ok(());
    }

    let category_id = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
    let element_id = u32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);

    tracing::debug!(%addr, category_id, element_id, "elementDataRequest");

    let cache = match resource_cache {
        Some(c) => c,
        None => {
            tracing::warn!(%addr, "No resource cache loaded — cannot serve element data");
            return Ok(());
        }
    };

    let xml_data = match cache.get(category_id, element_id) {
        Some(data) => data,
        None => {
            tracing::warn!(%addr, category_id, element_id, "Element not found in resource cache");
            return Ok(());
        }
    };

    // Allocate a data_id for this transfer
    let data_id = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            let id = c.next_data_id;
            c.next_data_id = c.next_data_id.wrapping_add(1);
            id
        } else {
            return Ok(());
        }
    };

    // Fragment into 1000-byte chunks
    const MAX_CHUNK: usize = 1000;
    let chunks: Vec<&[u8]> = xml_data.chunks(MAX_CHUNK).collect();
    let total_chunks = chunks.len();

    tracing::debug!(
        %addr,
        element_id,
        total_size = xml_data.len(),
        total_chunks,
        data_id,
        "Fragmenting resource data"
    );

    for (i, chunk) in chunks.iter().enumerate() {
        let frag_flags = match (i == 0, i == total_chunks - 1) {
            (true, true) => FRAG_FIRST_AND_LAST,
            (true, false) => FRAG_FIRST,
            (false, true) => FRAG_LAST,
            (false, false) => FRAG_MIDDLE,
        };

        // First fragment includes msgType, categoryId, elementId
        let (mt, cat, elem) = if i == 0 {
            (Some(0u8), Some(category_id), Some(element_id))
        } else {
            (None, None, None)
        };

        let (acks, seq) = drain_acks_and_seq(connected, addr)?;
        let pkt = build_resource_fragment(
            &key, seq, &acks,
            data_id, i as u8, frag_flags,
            mt, cat, elem, chunk,
        );
        socket.send_to(&pkt, addr).await?;
    }

    tracing::debug!(%addr, element_id, total_chunks, "Resource fragments sent");

    Ok(())
}

// ── Phase 5: world entry ─────────────────────────────────────────────────────

/// Send the Phase 5 world entry packet when the client calls `playCharacter`.
async fn handle_play_character(
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
        let mut clients = connected
            .lock()
            .map_err(|_| "connected lock poisoned")?;
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
    let entry_info = query_world_entry(db_pool, account_id, player_id, entity_manager, cell_tx).await;

    // Also query the full player data needed for mapLoaded
    let player_load_data = query_player_load_data(db_pool, account_id, player_id).await;

    tracing::info!(
        %addr,
        player_id,
        entity_id = entry_info.player_entity_id,
        space_id = entry_info.space_id,
        pos = ?entry_info.pos,
        "Phase 5a: sending RESET_ENTITIES (entity teardown)"
    );

    let acks: Vec<u32> = {
        let mut pending = pending_acks_arc.lock().unwrap();
        pending.drain(..).collect()
    };

    // Phase 5a: Send ONLY RESET_ENTITIES.
    // The C++ server sends RESET_ENTITIES in its own flushed bundle. The client
    // tears down all entities, then sends ENABLE_ENTITIES, which triggers
    // Phase 5b (CREATE_BASE_PLAYER + viewport + cell + forced position).
    let seq = next_seq.fetch_add(1, Ordering::Relaxed);
    let pkt = build_reset_entities(&key, seq, &acks);
    tracing::trace!(%addr, len = pkt.len(), seq, "UDP_OUT RESET_ENTITIES (Phase 5a)");
    socket.send_to(&pkt, addr).await?;

    // Store the world entry info and player load data for Phase 5b.
    {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_player_entity_id = Some(entry_info.player_entity_id);
            c.player_entity_id = Some(entry_info.player_entity_id);
            c.pending_world_entry = Some(entry_info);
            c.player_name = Some(player_load_data.player_name.clone());
            c.pending_player_load_data = Some(player_load_data);
        }
    }

    tracing::info!(%addr, "Phase 5a sent — waiting for ENABLE_ENTITIES from client");

    Ok(())
}

/// Handle `ENABLE_ENTITIES` (0x08) — dispatches Phase 4 or Phase 5b.
///
/// - **Phase 4** (no `pending_player_entity_id`): First ENABLE_ENTITIES after connect.
///   Creates the Account entity and sends the character list, then starts tick-sync.
/// - **Phase 5b** (has `pending_player_entity_id`): After world entry RESET_ENTITIES.
///   Sends `CREATE_BASE_PLAYER` for the SGWPlayer entity.
async fn handle_enable_entities(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    account_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    db_pool: &Option<Arc<PgPool>>,
    _entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Check if we have a pending world entry (Phase 5b).
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
        // ── Phase 5b: CREATE_BASE_PLAYER + viewport + cell + forced position ──
        // Packet 1: spatial setup (world entry)
        let (acks, seq) = drain_acks_and_seq(connected, addr)?;

        tracing::info!(
            %addr,
            player_entity_id = entry_info.player_entity_id,
            space_id = entry_info.space_id,
            seq,
            "Phase 5b: sending world entry + mapLoaded sequence"
        );

        let pkt = build_world_entry(&key, seq, &acks, &entry_info);
        socket.send_to(&pkt, addr).await?;

        // Packet 2: complete mapLoaded sequence (setupWorldParameters, stats,
        // appearance, abilities, inventory, exp, and onPlayerDataLoaded).
        let player_data = pending_load.unwrap_or_else(|| default_player_load_data());
        let (acks2, seq2) = drain_acks_and_seq(connected, addr)?;
        let pkt2 = build_map_loaded(&key, seq2, &acks2, entry_info.player_entity_id, &player_data);
        tracing::debug!(%addr, len = pkt2.len(), seq = seq2, "UDP_OUT mapLoaded");
        socket.send_to(&pkt2, addr).await?;

        tracing::info!(%addr, player = %player_data.player_name,
            level = player_data.level, archetype = player_data.archetype,
            "Phase 5b: mapLoaded complete ({} bytes)", pkt2.len());

        // Notify CellService that this entity now has a client controller
        if let Some(ref tx) = cell_tx {
            let _ = tx.send(BaseToCellMsg::ConnectEntity {
                entity_id: entry_info.player_entity_id,
            }).await;
        }

        // Register entity_id → addr so AoI messages can find this client's socket.
        entity_to_addr.lock().unwrap().insert(entry_info.player_entity_id, addr);

        tracing::info!(%addr, "Phase 5b complete — player in world");
        return Ok(());
    }

    // ── Phase 4: initial entity creation — send Account entity + char list ──
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
    tracing::trace!(%addr, len = pkt.len(), seq, hex = %to_hex(&pkt), "UDP_OUT char_list");
    socket.send_to(&pkt, addr).await?;

    tracing::info!(%addr, "Phase 4 complete — char list sent");

    Ok(())
}

// ── Space registry (populated from CellService SpaceData messages) ───────────

/// Thread-safe space registry mapping world_name → space_id.
/// Populated at startup when CellService sends SpaceData for each space.
static SPACE_REGISTRY: std::sync::LazyLock<Mutex<HashMap<String, u32>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Register a space in the global registry (called from CellToBase message handler).
fn register_space(world_name: String, space_id: u32) {
    tracing::debug!(world = %world_name, space_id, "Registered space in BaseApp registry");
    SPACE_REGISTRY.lock().unwrap().insert(world_name, space_id);
}

/// Look up a space_id from the registry (populated by CellService).
fn resolve_space_id_from_registry(world_name: &str) -> u32 {
    SPACE_REGISTRY.lock().unwrap()
        .get(world_name)
        .copied()
        .unwrap_or_else(|| {
            tracing::warn!(world = %world_name, "Space not in registry — using fallback");
            resolve_space_id_fallback(world_name)
        })
}

/// Legacy hardcoded space ID fallback (used when CellService is unavailable).
fn resolve_space_id_fallback(world_name: &str) -> u32 {
    match world_name {
        "Castle_CellBlock" => DEFAULT_SPACE_ID,     // 65552
        "SGC_W1"           => DEFAULT_SPACE_ID + 1, // 65553
        "CombatSim"        => DEFAULT_SPACE_ID + 2, // 65554
        _ => {
            tracing::warn!("Unknown world_location: {world_name}, defaulting to Castle_CellBlock");
            DEFAULT_SPACE_ID
        }
    }
}

/// Handle a gate travel request from CellService.
///
/// This re-uses the Phase 5a/5b world entry flow:
/// 1. Send RESET_ENTITIES to tear down the client entity system.
/// 2. Set up pending world entry for the new world (reusing same entity_id).
/// 3. Client responds with ENABLE_ENTITIES → Phase 5b sends the new world packets.
///
/// The CellService has already removed the entity from its old space.
/// We tell it to create the entity in the new space, then send the client
/// the full world-entry + mapLoaded sequence for the destination.
async fn handle_gate_travel(
    entity_id: u32,
    target_world_name: &str,
    position: [f32; 3],
    rotation: [f32; 3],
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    db_pool: &Option<Arc<PgPool>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Look up client socket from entity_id
    let addr = entity_to_addr.lock().unwrap().get(&entity_id).copied()
        .ok_or("Gate travel: no client addr for entity")?;

    // Get client state
    let (key, account_id, pending_acks_arc, next_seq) = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get(&addr).ok_or("Gate travel: client state not found")?;
        (c.key, c.account_id, Arc::clone(&c.pending_acks), Arc::clone(&c.next_seq))
    };

    tracing::info!(
        entity_id, %addr, world = %target_world_name,
        "Gate travel: sending RESET_ENTITIES for world transition"
    );

    // Tell CellService to create the entity in the new space
    if let Some(tx) = cell_tx {
        let _ = tx.send(BaseToCellMsg::CreateEntity {
            entity_id,
            world_name: target_world_name.to_string(),
            position,
            rotation,
        }).await;
    }

    // Resolve space_id from the space registry
    let space_id = resolve_space_id_from_registry(target_world_name);

    // Build the world entry info for the new destination
    let entry_info = WorldEntryInfo {
        player_entity_id: entity_id,
        space_id,
        pos: position,
        rot: rotation,
    };

    // Query player load data from DB (same player, different world)
    let player_id = {
        let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let c = clients.get(&addr).ok_or("client not found")?;
        // We stored account_id but need player_id for the DB query.
        // The player_id isn't stored in ConnectedClientState, so we query by
        // account_id alone (the DB query uses account_id to find the active char).
        c.account_id
    };
    let player_load_data = query_player_load_data_by_account(db_pool, account_id).await;

    // Phase 5a: Send RESET_ENTITIES
    let acks: Vec<u32> = {
        let mut pending = pending_acks_arc.lock().unwrap();
        pending.drain(..).collect()
    };
    let seq = next_seq.fetch_add(1, Ordering::Relaxed);
    let pkt = build_reset_entities(&key, seq, &acks);
    socket.send_to(&pkt, addr).await?;

    // Store pending world entry for Phase 5b (ENABLE_ENTITIES handler)
    {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        if let Some(c) = clients.get_mut(&addr) {
            c.pending_player_entity_id = Some(entity_id);
            c.pending_world_entry = Some(entry_info);
            c.pending_player_load_data = Some(player_load_data);
            // world_entry_sent stays true — we don't reset it, since
            // handle_enable_entities checks pending_player_entity_id
        }
    }

    tracing::info!(
        entity_id, %addr, world = %target_world_name,
        "Gate travel: RESET_ENTITIES sent — awaiting ENABLE_ENTITIES"
    );

    let _ = player_id; // account_id used above
    Ok(())
}

/// Query player load data using just the account_id (for gate travel where we
/// don't have the player_id readily available in ConnectedClientState).
async fn query_player_load_data_by_account(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
) -> PlayerLoadData {
    let pool = match db_pool {
        Some(p) => p,
        None => return default_player_load_data(),
    };

    #[derive(sqlx::FromRow)]
    struct PlayerRow {
        player_id: i32,
    }

    // Find the most recently used character for this account
    match sqlx::query_as::<_, PlayerRow>(
        "SELECT player_id FROM sgw_player WHERE account_id = $1 ORDER BY player_id LIMIT 1",
    )
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(row)) => query_player_load_data(db_pool, account_id, row.player_id).await,
        _ => default_player_load_data(),
    }
}

/// Handle a mail request from CellService by querying the DB and sending results to the client.
async fn handle_mail_request(
    entity_id: u32,
    op: crate::cell::messages::MailOp,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    use crate::cell::messages::MailOp;
    use crate::cell::mail;

    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(entity_id, "Mail request: no DB pool available");
            return;
        }
    };

    // Get the player's character_id (player_id) from their account via entity lookup
    let account_id = {
        let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
            Some(a) => a,
            None => { tracing::warn!(entity_id, "Mail: no client addr"); return; }
        };
        let clients = connected.lock().unwrap();
        match clients.get(&addr) {
            Some(c) => c.account_id,
            None => { tracing::warn!(entity_id, "Mail: client not found"); return; }
        }
    };

    // Resolve player_id from account_id
    let player_id = match sqlx::query_scalar::<_, i32>(
        "SELECT player_id FROM sgw_player WHERE account_id = $1 ORDER BY player_id LIMIT 1",
    )
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(pid)) => pid,
        _ => {
            tracing::warn!(entity_id, account_id, "Mail: could not resolve player_id");
            return;
        }
    };

    // Get player name for mail read responses
    let player_name = {
        let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
            Some(a) => a,
            None => { tracing::warn!(entity_id, "Mail: no addr for player name lookup"); return; }
        };
        let clients = connected.lock().unwrap();
        clients.get(&addr)
            .and_then(|c| c.player_name.clone())
            .unwrap_or_default()
    };

    match op {
        MailOp::RequestHeaders { b_archive } => {
            tracing::debug!(entity_id, player_id, b_archive, "Mail: querying headers");

            #[derive(sqlx::FromRow)]
            struct MailRow {
                mail_id: i32,
                sender_name: String,
                sender_id: Option<i32>,
                subject: String,
                cash: i64,
                sent_time: i32,
                read_time: i32,
                flags: i32,
            }

            let rows = sqlx::query_as::<_, MailRow>(
                "SELECT mail_id, sender_name, sender_id, subject, cash, sent_time, read_time, flags \
                 FROM sgw_gate_mail WHERE character_id = $1 ORDER BY mail_id DESC",
            )
            .bind(player_id)
            .fetch_all(pool.as_ref())
            .await
            .unwrap_or_default();

            let headers: Vec<mail::MailHeader> = rows.iter().map(|r| mail::MailHeader {
                id: r.mail_id,
                from_text: r.sender_name.clone(),
                from_id: r.sender_id.unwrap_or(0),
                subject_text: r.subject.clone(),
                cash: r.cash as i32,
                sent_time: r.sent_time as f32,
                read_time: r.read_time as f32,
                flags: r.flags,
            }).collect();

            tracing::debug!(entity_id, count = headers.len(), "Mail: sending headers to client");

            let args = mail::serialize_on_mail_header_info(b_archive, &headers);
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id,
                        crate::mercury_ext::method_idx::ON_MAIL_HEADER_INFO, &args)
                },
            ).await;
        }

        MailOp::RequestBody { mail_id } => {
            tracing::debug!(entity_id, mail_id, "Mail: querying body");

            #[derive(sqlx::FromRow)]
            struct BodyRow {
                message: String,
            }

            match sqlx::query_as::<_, BodyRow>(
                "SELECT message FROM sgw_gate_mail WHERE mail_id = $1 AND character_id = $2",
            )
            .bind(mail_id)
            .bind(player_id)
            .fetch_optional(pool.as_ref())
            .await
            {
                Ok(Some(row)) => {
                    // Mark as read (set read_time if not already set)
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i32;
                    let _ = sqlx::query(
                        "UPDATE sgw_gate_mail SET read_time = $1 WHERE mail_id = $2 AND read_time = 0",
                    )
                    .bind(now)
                    .bind(mail_id)
                    .execute(pool.as_ref())
                    .await;

                    let args = mail::serialize_on_mail_read(mail_id, &row.message, &player_name);
                    send_to_witness(
                        socket, connected, entity_to_addr, entity_id,
                        |key, seq, acks| {
                            build_entity_method_packet(key, seq, acks, entity_id,
                                crate::mercury_ext::method_idx::ON_MAIL_READ, &args)
                        },
                    ).await;
                }
                _ => {
                    tracing::warn!(entity_id, mail_id, "Mail body not found");
                }
            }
        }

        MailOp::Delete { mail_id } => {
            tracing::debug!(entity_id, mail_id, "Mail: deleting");
            let _ = sqlx::query(
                "DELETE FROM sgw_gate_mail WHERE mail_id = $1 AND character_id = $2",
            )
            .bind(mail_id)
            .bind(player_id)
            .execute(pool.as_ref())
            .await;

            // Notify client to remove the header
            let args = mail::serialize_on_mail_header_remove(mail_id);
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id,
                        crate::mercury_ext::method_idx::ON_MAIL_HEADER_REMOVE, &args)
                },
            ).await;
        }

        MailOp::Archive { mail_id } => {
            tracing::debug!(entity_id, mail_id, "Mail: archiving");
            // Set the MAIL_Archive flag (bit 0)
            let _ = sqlx::query(
                "UPDATE sgw_gate_mail SET flags = flags | 1 WHERE mail_id = $1 AND character_id = $2",
            )
            .bind(mail_id)
            .bind(player_id)
            .execute(pool.as_ref())
            .await;

            // Notify client to remove the header from inbox
            let args = mail::serialize_on_mail_header_remove(mail_id);
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id,
                        crate::mercury_ext::method_idx::ON_MAIL_HEADER_REMOVE, &args)
                },
            ).await;
        }
    }
}

/// Handle a message from CellService — dispatches AoI packets to witness clients.
async fn handle_cell_message(
    msg: CellToBaseMsg,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    match msg {
        CellToBaseMsg::SpaceData { space_id, world_name } => {
            register_space(world_name, space_id);
        }
        CellToBaseMsg::EntityCreated { entity_id, space_id, position } => {
            tracing::debug!(entity_id, space_id, ?position, "CellService: entity created");
        }
        CellToBaseMsg::EnteredAoI { witness_id, entity_id, space_id: _, class_id, position, direction } => {
            tracing::debug!(witness_id, entity_id, class_id, "AoI: entity entered witness range");
            send_to_witness(
                socket, connected, entity_to_addr, witness_id,
                |key, seq, acks| {
                    build_create_entity(
                        key, seq, acks, entity_id,
                        class_id, position, direction,
                    )
                },
            ).await;
        }
        CellToBaseMsg::LeftAoI { witness_id, entity_id } => {
            tracing::debug!(witness_id, entity_id, "AoI: entity left witness range");
            send_to_witness(
                socket, connected, entity_to_addr, witness_id,
                |key, seq, acks| {
                    build_entity_leave(key, seq, acks, entity_id)
                },
            ).await;
        }
        CellToBaseMsg::EntityMoved { witness_id, entity_id, space_id: _, position, direction, velocity } => {
            tracing::trace!(witness_id, entity_id, "AoI: entity position update");
            send_to_witness(
                socket, connected, entity_to_addr, witness_id,
                |key, seq, acks| {
                    build_avatar_update(
                        key, seq, acks, entity_id,
                        position, velocity, direction,
                    )
                },
            ).await;
        }
        CellToBaseMsg::EntityMethodCall { entity_id, method_index, args } => {
            tracing::debug!(entity_id, method_index, args_len = args.len(), "CellService→client entity method call");
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id, method_index, &args)
                },
            ).await;
        }
        CellToBaseMsg::GateTravel { entity_id, target_world_name, position, rotation } => {
            if let Err(e) = handle_gate_travel(
                entity_id, &target_world_name, position, rotation,
                socket, connected, entity_to_addr, cell_tx, db_pool,
            ).await {
                tracing::error!(entity_id, world = %target_world_name, "Gate travel failed: {e}");
            }
        }
        CellToBaseMsg::MailRequest { entity_id, op } => {
            handle_mail_request(entity_id, op, socket, connected, entity_to_addr, db_pool).await;
        }
    }
}

/// Send an AoI packet to a specific witness's client.
///
/// Looks up the witness entity_id → SocketAddr, then finds the client state
/// to get encryption key and sequence number. Calls the packet builder closure
/// and sends the result via UDP.
async fn send_to_witness<F>(
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    witness_id: u32,
    build_packet: F,
) where
    F: FnOnce(&[u8; 32], u32, &[u32]) -> Vec<u8>,
{
    // Extract all data from locks in a sync block so no MutexGuard crosses an await.
    let send_data = {
        let addr = match entity_to_addr.lock().unwrap().get(&witness_id).copied() {
            Some(a) => a,
            None => {
                tracing::trace!(witness_id, "AoI: no client addr for witness — skipping");
                return;
            }
        };

        let clients = connected.lock().unwrap();
        match clients.get(&addr) {
            Some(c) => {
                let key = c.key;
                let seq = c.next_seq.fetch_add(1, Ordering::Relaxed);
                let acks: Vec<u32> = c.pending_acks.lock().unwrap().drain(..).collect();
                Some((addr, key, seq, acks))
            }
            None => {
                tracing::trace!(witness_id, %addr, "AoI: client disconnected — skipping");
                None
            }
        }
    };

    if let Some((addr, key, seq, acks)) = send_data {
        let packet = build_packet(&key, seq, &acks);
        if let Err(e) = socket.send_to(&packet, addr).await {
            tracing::warn!(witness_id, %addr, "AoI: failed to send packet: {e}");
        }
    }
}

/// Query the character's world entry data from the database and allocate a player entity ID.
///
/// If a CellService channel is available, sends `CreateEntity` to resolve the space_id
/// dynamically. Otherwise falls back to the hardcoded space ID table.
async fn query_world_entry(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
    player_id: i32,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) -> WorldEntryInfo {
    let player_eid = entity_manager.lock().unwrap().create_entity("SGWPlayer").0 as u32;

    let default_entry = || WorldEntryInfo {
        player_entity_id: player_eid,
        space_id: DEFAULT_SPACE_ID,
        pos: [0.0; 3],
        rot: [0.0; 3],
    };

    let pool = match db_pool {
        Some(p) => p,
        None => return default_entry(),
    };

    #[derive(sqlx::FromRow)]
    struct EntryRow {
        world_location: String,
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
    }

    match sqlx::query_as::<_, EntryRow>(
        "SELECT world_location, pos_x, pos_y, pos_z \
         FROM sgw_player WHERE player_id = $1 AND account_id = $2",
    )
    .bind(player_id)
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(row)) => {
            let pos = [row.pos_x, row.pos_y, row.pos_z];

            // Resolve space_id via CellService if available, else fall back
            // to the legacy hardcoded table.
            let space_id = if let Some(tx) = cell_tx {
                // Send CreateEntity to CellService — it will find or create the
                // space and return the assigned space_id via EntityCreated.
                if tx.send(BaseToCellMsg::CreateEntity {
                    entity_id: player_eid,
                    world_name: row.world_location.clone(),
                    position: pos,
                    rotation: [0.0; 3],
                }).await.is_ok() {
                    // We don't block on the response here; the CellService will
                    // send EntityCreated asynchronously, but we still need a
                    // space_id for the wire packet NOW. Use the space registry
                    // (populated at startup from SpaceData messages) or fall back.
                    // TODO: In a future phase, use a oneshot channel for synchronous
                    // space_id resolution. For now, rely on the space registry.
                    resolve_space_id_from_registry(&row.world_location)
                } else {
                    resolve_space_id_fallback(&row.world_location)
                }
            } else {
                resolve_space_id_fallback(&row.world_location)
            };

            WorldEntryInfo {
                player_entity_id: player_eid,
                space_id,
                pos,
                rot: [0.0; 3],
            }
        }
        Ok(None) => {
            tracing::warn!(player_id, account_id, "Character not found for world entry");
            default_entry()
        }
        Err(e) => {
            tracing::error!("Failed to query world entry: {e}");
            default_entry()
        }
    }
}

// ── Player load data query ───────────────────────────────────────────────────

/// Query full player data from the database for the mapLoaded sequence.
///
/// Returns all fields needed by [`build_map_loaded`]: level, name, archetype,
/// appearance, abilities, inventory stubs, experience, etc.
async fn query_player_load_data(
    db_pool: &Option<Arc<PgPool>>,
    account_id: u32,
    player_id: i32,
) -> PlayerLoadData {
    let pool = match db_pool {
        Some(p) => p,
        None => return default_player_load_data(),
    };

    #[derive(sqlx::FromRow)]
    struct PlayerRow {
        level: i32,
        player_name: String,
        extra_name: String,
        alignment: i32,
        archetype: i32,
        gender: i32,
        bodyset: String,
        components: Vec<String>,
        exp: i32,
        naquadah: i32,
        known_stargates: Vec<i32>,
        abilities: Vec<i32>,
        training_points: i32,
        applied_science_points: i32,
        blueprint_ids: Vec<i32>,
        first_login: i32,
        access_level: i32,
        skin_color_id: i32,
    }

    match sqlx::query_as::<_, PlayerRow>(
        "SELECT level, player_name, extra_name, alignment, archetype, gender, \
         bodyset, components, exp, naquadah, known_stargates, abilities, \
         training_points, applied_science_points, blueprint_ids, first_login, \
         access_level, skin_color_id \
         FROM sgw_player WHERE player_id = $1 AND account_id = $2",
    )
    .bind(player_id)
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(row)) => {
            tracing::debug!(
                player_id, level = row.level, archetype = row.archetype,
                name = %row.player_name, "Loaded player data for mapLoaded"
            );
            PlayerLoadData {
                level: row.level,
                player_name: row.player_name,
                extra_name: row.extra_name,
                alignment: row.alignment,
                archetype: row.archetype,
                gender: row.gender,
                bodyset: row.bodyset,
                components: row.components,
                exp: row.exp,
                naquadah: row.naquadah,
                known_stargates: row.known_stargates,
                abilities: row.abilities,
                training_points: row.training_points,
                applied_science_points: row.applied_science_points,
                blueprint_ids: row.blueprint_ids,
                first_login: row.first_login,
                access_level: row.access_level,
                skin_color_id: row.skin_color_id,
                ability_tree: archetype_ability_tree(row.archetype),
            }
        }
        Ok(None) => {
            tracing::warn!(player_id, account_id, "Player not found for mapLoaded");
            default_player_load_data()
        }
        Err(e) => {
            tracing::error!(player_id, "Failed to query player load data: {e}");
            default_player_load_data()
        }
    }
}

/// Default player load data when the DB is unavailable.
fn default_player_load_data() -> PlayerLoadData {
    PlayerLoadData {
        level: 1,
        player_name: "Unknown".into(),
        extra_name: String::new(),
        alignment: 1,
        archetype: 1,
        gender: 1,
        bodyset: "BS_HumanMale.BS_HumanMale".into(),
        components: vec![],
        exp: 0,
        naquadah: 0,
        known_stargates: vec![],
        abilities: vec![],
        training_points: 0,
        applied_science_points: 0,
        blueprint_ids: vec![],
        first_login: 1,
        access_level: 0,
        skin_color_id: 0,
        ability_tree: archetype_ability_tree(1),
    }
}

// ── Tick-sync heartbeat ──────────────────────────────────────────────────────

/// Per-connection tick-sync heartbeat task.
async fn run_tick_loop(
    socket: Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    next_seq: Arc<AtomicU32>,
    pending_acks: Arc<Mutex<Vec<u32>>>,
    last_recv: Arc<Mutex<Instant>>,
    cancelled: Arc<AtomicBool>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: Arc<Mutex<EntityManager>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(60);

    let mut tick: u32 = 0;

    tracing::debug!(%addr, "Tick-sync loop started");

    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check if the session was torn down by handle_log_off.
        if cancelled.load(Ordering::Relaxed) {
            tracing::info!(%addr, "Tick-sync stopping: session cancelled (logOff)");
            break;
        }

        let idle = last_recv.lock().unwrap().elapsed();
        if idle > INACTIVITY_TIMEOUT {
            tracing::info!(
                %addr,
                idle_secs = idle.as_secs(),
                "Tick-sync stopping: client inactive for {}s",
                idle.as_secs()
            );
            break;
        }

        let acks: Vec<u32> = {
            let mut pending = pending_acks.lock().unwrap();
            pending.drain(..).collect()
        };
        if !acks.is_empty() {
            tracing::trace!(%addr, ?acks, "Piggybacking ACKs on tick_sync");
        }

        let seq_id = next_seq.fetch_add(1, Ordering::Relaxed);
        let pkt = build_ongoing_tick_sync(&key, seq_id, tick, &acks);
        if let Err(e) = socket.send_to(&pkt, addr).await {
            tracing::debug!(%addr, "Tick-sync stopped (send error): {e}");
            break;
        }

        if tick % 100 == 0 {
            tracing::debug!(%addr, tick, seq_id, "Tick-sync heartbeat (every 100th)");
        }

        tick = tick.wrapping_add(1);
    }

    // Clean up entities for this disconnected client.
    destroy_client_entities(&connected, &entity_manager, addr, &cell_tx, &entity_to_addr);
}

// ── SGWPlayer base method dispatch ──────────────────────────────────────────

/// SGWPlayer flattened EXPOSED BaseMethod indices.
///
/// After world entry, the client controls an SGWPlayer entity. Base method calls
/// use `msg_id = flat_index + 0xC0`. The ordering is: interface methods first
/// (in Implements order from SGWPlayer.def), then own methods.
///
/// ## Communicator interface (indices 0–14, msg_id 0xC0–0xCE)
/// | Idx | Method | Args |
/// |-----|--------|------|
/// | 0 | chatJoin | WSTRING channelName, WSTRING password |
/// | 1 | chatLeave | UINT8 channelId |
/// | 2 | sendPlayerCommunication | UINT8 channel, WSTRING target, WSTRING text |
/// | 3 | chatSetAFKMessage | WSTRING message |
/// | 4 | chatSetDNDMessage | WSTRING message |
/// | 5–14 | chatIgnore..announcePetition | (various, stub) |
mod sgw_player_base {
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
async fn dispatch_sgw_player_base_method(
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
            tracing::debug!(%addr, channel_name, "chatJoin — acknowledged (channels auto-joined)");
        }

        sgw_player_base::CHAT_LEAVE => {
            // chatLeave(UINT8 channelId)
            let channel_id = if !payload.is_empty() { payload[0] } else { 0 };
            tracing::debug!(%addr, channel_id, "chatLeave — acknowledged");
        }

        sgw_player_base::CHAT_SET_AFK | sgw_player_base::CHAT_SET_DND => {
            tracing::debug!(%addr, msg_id = format_args!("{:#04x}", msg_id), "Chat status update — acknowledged");
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

// ── LogOff handler ──────────────────────────────────────────────────────────

/// Handle `logOff` (0xC2) — the client is leaving the session.
///
/// Both "Change Server" and "Back" buttons in the character select UI call this.
/// The C++ reference (`Account.py:logOff`) was a no-op (`pass`), which meant the
/// server never cleaned up — leaking sessions indefinitely.
///
/// Client flow (from Ghidra analysis of FUN_00def9f0):
///   1. Both buttons emit `Event_NetOut_LogOff` → sends logOff (0xC2)
///   2. `relogin()` also sets a relogin flag before step 1
///   3. Client fires `gotoLogin` state transition
///   4. Client waits for Mercury channel to die (server closes or timeout)
///   5. `Event_Net_Disconnected` fires → disconnect callback (FUN_00de9bb0):
///      - If relogin flag set → `EntityManager::disconnected()` + SOAP re-login
///      - If not set → shows "disconnected from network" UI
///
/// We send `LOGGED_OFF` (0x37) to trigger an immediate channel teardown on the
/// client side (via `ServerConnection::loggedOff → Mercury::disconnect`).
/// Without it, the client waits 30s for the Mercury channel timeout.
async fn handle_log_off(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    key: [u8; 32],
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(%addr, "Client requests logOff — sending LOGGED_OFF and cleaning up");

    // Signal the tick-sync loop to stop and grab seq/acks for the final packet.
    let (acks, seq) = {
        let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
        let client = clients.get_mut(&addr).ok_or("no session for addr")?;
        client.cancelled.store(true, Ordering::Relaxed);
        let acks: Vec<u32> = client.pending_acks.lock().unwrap().drain(..).collect();
        let seq = client.next_seq.fetch_add(1, Ordering::Relaxed);
        (acks, seq)
    };

    // Send LOGGED_OFF (0x37) with reason=0 to trigger immediate client-side
    // channel teardown.  This fires ServerConnection::loggedOff → Event_Net_Disconnected
    // which triggers the relogin flow (Change Server) or disconnect UI (Back).
    let pkt = build_logged_off(&key, seq, &acks);
    socket.send_to(&pkt, addr).await?;
    tracing::debug!(%addr, seq, "Sent LOGGED_OFF (0x37)");

    // Destroy entities and remove from connected map.
    destroy_client_entities(connected, entity_manager, addr, cell_tx, entity_to_addr);

    Ok(())
}

// ── Shared helpers ───────────────────────────────────────────────────────────

/// Drain pending ACKs and allocate the next sequence number.
fn drain_acks_and_seq(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> Result<(Vec<u32>, u32), Box<dyn std::error::Error + Send + Sync>> {
    let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
    let c = clients
        .get_mut(&addr)
        .ok_or("addr not in connected map")?;
    let acks: Vec<u32> = c.pending_acks.lock().unwrap().drain(..).collect();
    let seq = c.next_seq.fetch_add(1, Ordering::Relaxed);
    Ok((acks, seq))
}

/// Read the dynamically allocated account entity ID for a connected client.
fn get_account_entity_id(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    addr: SocketAddr,
) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
    let clients = connected.lock().map_err(|_| "connected lock poisoned")?;
    let c = clients.get(&addr).ok_or("addr not in connected map")?;
    Ok(c.account_entity_id)
}

/// Destroy all entities associated with a disconnecting client and remove it from the map.
///
/// Safe to call multiple times for the same address — returns silently if the
/// session was already removed (e.g. DISCONNECT handler cleaned up, then the
/// tick-sync inactivity timeout fires on the now-absent session).
///
/// Always sets `cancelled` on the session before removal so the tick-sync loop
/// exits promptly instead of running until the 60-second inactivity timeout.
fn destroy_client_entities(
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    addr: SocketAddr,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let (account_eid, player_eid) = {
        let mut clients = match connected.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        let Some(c) = clients.get(&addr) else {
            tracing::debug!(%addr, "destroy_client_entities: no session, already cleaned up");
            return;
        };
        // Signal the tick-sync loop to exit before we remove the session.
        c.cancelled.store(true, Ordering::Relaxed);
        let account_eid = c.account_entity_id;
        let player_eid = c.player_entity_id;
        clients.remove(&addr);
        (account_eid, player_eid)
    };

    let mut mgr = entity_manager.lock().unwrap();
    if account_eid != 0 {
        tracing::debug!(%addr, account_entity_id = account_eid, "Destroying Account entity");
        mgr.destroy_entity(EntityId(account_eid as i32));
    }
    if let Some(player_eid) = player_eid {
        tracing::debug!(%addr, player_entity_id = player_eid, "Destroying Player entity");
        mgr.destroy_entity(EntityId(player_eid as i32));

        // Remove from entity→addr reverse index
        entity_to_addr.lock().unwrap().remove(&player_eid);

        // Notify CellService to disconnect and destroy the cell entity
        if let Some(tx) = cell_tx {
            let _ = tx.try_send(BaseToCellMsg::DisconnectEntity {
                entity_id: player_eid,
            });
        }
    }
    tracing::info!(%addr, "Client entities cleaned up");
}

// ── Phase 3 login helpers ─────────────────────────────────────────────────────

/// Validate ticket, send Phase 3 reply + time-sync, register the encrypted channel.
async fn handle_login(
    socket: &Arc<UdpSocket>,
    addr: SocketAddr,
    request_id: u32,
    ticket: &str,
    pending_logins: &Arc<Mutex<HashMap<String, PendingLogin>>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_manager: &Arc<Mutex<EntityManager>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let login = {
        let mut map = pending_logins
            .lock()
            .map_err(|_| "pending_logins lock poisoned")?;
        map.remove(ticket)
    };

    let login = match login {
        Some(l) => l,
        None => {
            tracing::warn!("Unknown or already-consumed ticket: {ticket}");
            return Ok(());
        }
    };

    let key = decode_session_key(&login.session_key)?;

    // ── Duplicate login detection (KI-7) ────────────────────────────────────
    // If this account already has an active session, evict the old one first.
    // C++ checks ChannelManager.isPlayerOnline() at play-character time
    // (Account.py:286-290), but we also guard at login to prevent stale sessions.
    {
        let evict_addr: Option<(SocketAddr, [u8; 32])> = {
            let clients = connected
                .lock()
                .map_err(|_| "connected lock poisoned")?;
            clients.iter().find_map(|(existing_addr, c)| {
                if c.account_id == login.account_id && *existing_addr != addr {
                    Some((*existing_addr, c.key))
                } else {
                    None
                }
            })
        };
        if let Some((old_addr, old_key)) = evict_addr {
            tracing::warn!(
                account_id = login.account_id,
                %old_addr,
                %addr,
                "Duplicate login — evicting old session"
            );
            // Send LOGGED_OFF to the old client so it gets an immediate teardown.
            let (acks, seq) = {
                let mut clients = connected.lock().map_err(|_| "connected lock poisoned")?;
                if let Some(c) = clients.get_mut(&old_addr) {
                    let acks: Vec<u32> = c.pending_acks.lock().unwrap().drain(..).collect();
                    let seq = c.next_seq.fetch_add(1, Ordering::Relaxed);
                    (acks, seq)
                } else {
                    (vec![], 0)
                }
            };
            let pkt = build_logged_off(&old_key, seq, &acks);
            let _ = socket.send_to(&pkt, old_addr).await;
            destroy_client_entities(connected, entity_manager, old_addr, cell_tx, entity_to_addr);
        }
    }

    tracing::info!(
        account_id = login.account_id,
        "Phase 3 authenticated; sending reply and time-sync"
    );

    // connect_reply at seq=1.
    let reply = build_connect_reply(request_id, ticket.as_bytes(), &key, 1);
    tracing::trace!(%addr, len = reply.len(), hex = %to_hex(&reply), "UDP_OUT connect_reply");
    socket.send_to(&reply, addr).await?;

    // time-sync bundle at seq=2.
    let sync = build_time_sync(&key, 2);
    tracing::trace!(%addr, len = sync.len(), hex = %to_hex(&sync), "UDP_OUT time_sync");
    socket.send_to(&sync, addr).await?;

    // Register for Phase 4 encrypted traffic.
    let (pending_acks_arc, last_recv_arc, next_seq_arc, cancelled_arc) = {
        let next_seq = Arc::new(AtomicU32::new(3));
        let pending_acks = Arc::new(Mutex::new(Vec::new()));
        let last_recv = Arc::new(Mutex::new(Instant::now()));
        let cancelled = Arc::new(AtomicBool::new(false));
        let arcs = (Arc::clone(&pending_acks), Arc::clone(&last_recv), Arc::clone(&next_seq), Arc::clone(&cancelled));

        let mut clients = connected
            .lock()
            .map_err(|_| "connected lock poisoned")?;
        clients.insert(
            addr,
            ConnectedClientState {
                enc: MercuryEncryption::from_session_key(key),
                key,
                account_id: login.account_id,
                char_list_sent: false,
                world_entry_sent: false,
                pending_player_entity_id: None,
                player_entity_id: None,
                next_seq,
                pending_acks,
                last_recv,
                account_entity_id: entity_manager.lock().unwrap().create_entity("Account").0 as u32,
                next_data_id: 0,
                pending_world_entry: None,
                pending_player_load_data: None,
                cancelled,
                player_name: None,
            },
        );
        arcs
    };

    tracing::info!(%addr, "Phase 3 complete — channel registered, starting tick-sync");

    // Start tick-sync immediately after Phase 3 (matches C++ onConnected() behavior).
    // The C++ server starts gameTick() as soon as the Account entity is created,
    // BEFORE the client sends ENABLE_ENTITIES. This provides:
    // 1. ACK delivery for client reliable messages (AUTHENTICATE, ENABLE_ENTITIES)
    // 2. A running game clock so the client can play the stargate transition animation
    tokio::spawn(run_tick_loop(
        Arc::clone(socket),
        addr,
        key,
        next_seq_arc,
        pending_acks_arc,
        last_recv_arc,
        cancelled_arc,
        Arc::clone(connected),
        Arc::clone(entity_manager),
        cell_tx.clone(),
        Arc::clone(entity_to_addr),
    ));

    Ok(())
}

/// Parse the `baseAppLogin` packet body.
fn parse_baseapp_login(
    raw: &[u8],
) -> Result<(u32, String), Box<dyn std::error::Error + Send + Sync>> {
    let pkt = parse_incoming(raw)?;

    if pkt.flags != (FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE) {
        return Err(format!("unexpected flags: {:#04x}", pkt.flags).into());
    }

    let body = &pkt.body;
    if body.len() < 34 {
        return Err(format!("body too short: {} bytes (expected ≥ 34)", body.len()).into());
    }

    let msg_id = body[0];
    if msg_id != 0x00 {
        return Err(format!("unexpected msg_id: {:#04x}", msg_id).into());
    }

    let word_len = u16::from_le_bytes([body[1], body[2]]);
    if word_len != 25 {
        return Err(format!("unexpected WORD_LENGTH: {}", word_len).into());
    }

    let request_id = u32::from_le_bytes([body[3], body[4], body[5], body[6]]);
    let _account_id = u32::from_le_bytes([body[9], body[10], body[11], body[12]]);
    let ticket_len = body[13] as usize;

    if ticket_len != 20 {
        return Err(format!("unexpected ticketLen: {}", ticket_len).into());
    }
    if body.len() < 14 + ticket_len {
        return Err("body truncated at ticket".into());
    }

    let ticket_bytes = &body[14..14 + ticket_len];
    let ticket_str = String::from_utf8(ticket_bytes.to_vec())?;

    Ok((request_id, ticket_str))
}

/// Decode a 64-character uppercase-hex session key to a 32-byte array.
fn decode_session_key(
    hex: &str,
) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
    if hex.len() != 64 {
        return Err(format!("session key must be 64 hex chars, got {}", hex.len()).into());
    }
    let mut key = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        key[i] = (hi << 4) | lo;
    }
    Ok(key)
}

fn hex_nibble(b: u8) -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(format!("invalid hex character: {}", b as char).into()),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn decode_session_key_valid() {
        let hex = "AABBCCDD".repeat(8); // 64 chars
        let key = decode_session_key(&hex).unwrap();
        assert_eq!(key[0], 0xAA);
        assert_eq!(key[1], 0xBB);
        assert_eq!(key[2], 0xCC);
        assert_eq!(key[3], 0xDD);
    }

    #[test]
    fn decode_session_key_too_short() {
        let result = decode_session_key("AABB");
        assert!(result.is_err());
    }

    #[test]
    fn parse_baseapp_login_valid() {
        let mut raw = Vec::new();
        raw.push(0x41u8); // flags
        raw.push(0x00u8); // msg_id
        raw.extend_from_slice(&25u16.to_le_bytes());
        raw.extend_from_slice(&0xCAFEBABEu32.to_le_bytes());
        raw.extend_from_slice(&0u16.to_le_bytes());
        raw.extend_from_slice(&1u32.to_le_bytes());
        raw.push(20u8);
        raw.extend_from_slice(b"ABCDEF1234567890ABCD");
        raw.extend_from_slice(&1u16.to_le_bytes());
        raw.extend_from_slice(&3u32.to_le_bytes());

        assert_eq!(raw.len(), 41);

        let (req_id, ticket) = parse_baseapp_login(&raw).unwrap();
        assert_eq!(req_id, 0xCAFEBABE);
        assert_eq!(ticket, "ABCDEF1234567890ABCD");
    }

    // ── chardef_lookup tests ─────────────────────────────────────────────────

    #[test]
    fn chardef_lookup_alignment_values() {
        // id 20 = Praxis Scientist Male → alignment 1
        let entry20 = chardef_lookup(20).unwrap();
        assert_eq!(entry20.0, 1, "chardef 20 should be Praxis (alignment=1)");

        // id 22 = Praxis Scientist Female → alignment 1
        let entry22 = chardef_lookup(22).unwrap();
        assert_eq!(entry22.0, 1, "chardef 22 should be Praxis (alignment=1)");

        // id 21 = SGU Scientist Male → alignment 2
        let entry21 = chardef_lookup(21).unwrap();
        assert_eq!(entry21.0, 2, "chardef 21 should be SGU (alignment=2)");

        // id 23 = SGU Scientist Female → alignment 2
        let entry23 = chardef_lookup(23).unwrap();
        assert_eq!(entry23.0, 2, "chardef 23 should be SGU (alignment=2)");
    }

    #[test]
    fn chardef_lookup_bodyset_doubled_format() {
        // All bodysets use "BS_X.BS_X" doubled format for the DB varchar(64) column.
        for id in [1, 9, 19] {
            let entry = chardef_lookup(id).unwrap();
            assert!(
                entry.3.contains('.'),
                "chardef {id} bodyset '{}' should contain a dot",
                entry.3
            );
        }
    }

    #[test]
    fn chardef_lookup_has_starting_coordinates() {
        // Every valid chardef should have at least one non-zero coordinate.
        for id in 1..=23 {
            let entry = chardef_lookup(id).unwrap();
            let (x, y, z) = (entry.5, entry.6, entry.7);
            assert!(
                x != 0.0 || y != 0.0 || z != 0.0,
                "chardef {id} should have non-zero coordinates, got ({x}, {y}, {z})"
            );
        }
    }

    #[test]
    fn chardef_lookup_praxis_starting_pos() {
        // Praxis entries spawn near (-334.2, 73.5, -228.0) in Castle_CellBlock.
        let entry = chardef_lookup(1).unwrap(); // Praxis Soldier Male
        assert_eq!(entry.0, 1, "should be Praxis alignment");
        assert!((entry.5 - (-334.231)).abs() < 1.0, "pos_x off: {}", entry.5);
        assert!((entry.6 - 73.472).abs() < 1.0, "pos_y off: {}", entry.6);
        assert!((entry.7 - (-228.026)).abs() < 1.0, "pos_z off: {}", entry.7);
    }

    #[test]
    fn chardef_lookup_sgu_starting_pos() {
        // SGU entries spawn near (201.5, 1.3, 49.7) in SGC_W1.
        let entry = chardef_lookup(2).unwrap(); // SGU Soldier Male
        assert_eq!(entry.0, 2, "should be SGU alignment");
        assert!((entry.5 - 201.5).abs() < 1.0, "pos_x off: {}", entry.5);
        assert!((entry.6 - 1.31).abs() < 1.0, "pos_y off: {}", entry.6);
        assert!((entry.7 - 49.724).abs() < 1.0, "pos_z off: {}", entry.7);
    }

    #[test]
    fn chardef_lookup_all_ids_valid() {
        for id in 1..=23 {
            assert!(
                chardef_lookup(id).is_some(),
                "chardef_lookup({id}) should return Some"
            );
        }
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

    // ── Space ID tests ───────────────────────────────────────────────────────

    #[test]
    fn space_id_mapping_known_worlds() {
        // Verify the three known space IDs are distinct and have high 16 bits == 1.
        let castle_cellblock: u32 = 65552; // (1 << 16) | 16
        let sgc_w1: u32 = 65553;           // (1 << 16) | 17
        let combat_sim: u32 = 65554;       // (1 << 16) | 18

        // All distinct
        assert_ne!(castle_cellblock, sgc_w1);
        assert_ne!(sgc_w1, combat_sim);
        assert_ne!(castle_cellblock, combat_sim);

        // High 16 bits == 1 for all three
        assert_eq!(castle_cellblock >> 16, 1);
        assert_eq!(sgc_w1 >> 16, 1);
        assert_eq!(combat_sim >> 16, 1);
    }

    // ── Scanner payload reader tests ─────────────────────────────────────────

    #[test]
    fn scanner_constant_length_0x02() {
        // AVATAR_UPD_IMPLICIT: CONSTANT_LENGTH = 36
        let buf = vec![0xAAu8; 36];
        let mut offset = 0usize;
        let payload = read_constant_payload(&buf, &mut offset, 36);
        assert!(payload.is_some());
        assert_eq!(offset, 36);
        assert_eq!(payload.unwrap().len(), 36);
    }

    #[test]
    fn scanner_constant_length_0x04() {
        // AVATAR_UPDW_IMPLICIT: CONSTANT_LENGTH = 36
        let buf = vec![0xBBu8; 36];
        let mut offset = 0usize;
        let payload = read_constant_payload(&buf, &mut offset, 36);
        assert!(payload.is_some());
        assert_eq!(offset, 36);
        assert_eq!(payload.unwrap().len(), 36);
    }

    #[test]
    fn scanner_constant_length_0x05() {
        // AVATAR_UPDW_EXPLICIT: CONSTANT_LENGTH = 40
        let buf = vec![0xCCu8; 40];
        let mut offset = 0usize;
        let payload = read_constant_payload(&buf, &mut offset, 40);
        assert!(payload.is_some());
        assert_eq!(offset, 40);
        assert_eq!(payload.unwrap().len(), 40);
    }
}
