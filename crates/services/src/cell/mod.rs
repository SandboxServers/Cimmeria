//! CellApp service.
//!
//! Manages spatial entity simulation, world cells, movement, and Area of
//! Interest calculations. Mirrors the C++ CellApp that partitions the game
//! world into spatial cells and simulates entity interactions within them.

pub mod abilities;
pub mod chat;
pub mod combat;
pub mod content;
pub mod dispatch;
pub mod gate_travel;
pub mod interactions;
pub mod mail;
pub mod messages;
pub mod missions;
pub mod space_manager;
pub mod spawner;

use std::net::SocketAddr;
use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_common::{EntityId, ServerConfig, SpaceId};
use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;

/// Errors specific to the cell service.
#[derive(Debug, thiserror::Error)]
pub enum CellError {
    #[error("Space {0} not found")]
    SpaceNotFound(SpaceId),

    #[error("Entity {0} not found in any cell")]
    EntityNotFound(EntityId),

    #[error("Failed to create space: {0}")]
    SpaceCreationFailed(String),

    #[error("Service not running")]
    NotRunning,

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
}

/// CellApp service managing spatial entity simulation.
///
/// In the original C++ architecture, this was the `CellApp` process that:
/// - Managed game spaces (world zones/instances)
/// - Simulated cell entity halves (spatial state, movement, AoI)
/// - Processed entity interactions within spatial proximity
/// - Ran the game tick loop for entity updates
pub struct CellService {
    /// Address the cell service binds to for BaseApp communication.
    pub listener_addr: SocketAddr,

    /// Whether the service is currently running.
    pub is_running: bool,

    /// Receiver for messages from BaseApp (set by orchestrator before start).
    base_to_cell_rx: Option<mpsc::Receiver<BaseToCellMsg>>,

    /// Sender for messages to BaseApp (set by orchestrator before start).
    cell_to_base_tx: Option<mpsc::Sender<CellToBaseMsg>>,

    /// Path to the entities directory for loading space XML files.
    entities_dir: String,

    /// Database pool for content engine loading (set by orchestrator).
    db_pool: Option<Arc<PgPool>>,
}

impl CellService {
    /// Create a new cell service from server configuration.
    pub fn new(config: &ServerConfig) -> Self {
        let listener_addr = format!("{}:{}", config.cell_host, config.cell_port)
            .parse()
            .unwrap_or_else(|_| {
                SocketAddr::from(([127, 0, 0, 1], config.cell_port))
            });

        Self {
            listener_addr,
            is_running: false,
            base_to_cell_rx: None,
            cell_to_base_tx: None,
            entities_dir: "entities".to_string(),
            db_pool: None,
        }
    }

    /// Set the database pool for content engine loading.
    pub fn set_db_pool(&mut self, pool: Arc<PgPool>) {
        self.db_pool = Some(pool);
    }

    /// Wire in the Base↔Cell channels. Called by the orchestrator before `start()`.
    pub fn set_channels(
        &mut self,
        rx: mpsc::Receiver<BaseToCellMsg>,
        tx: mpsc::Sender<CellToBaseMsg>,
    ) {
        self.base_to_cell_rx = Some(rx);
        self.cell_to_base_tx = Some(tx);
    }

    /// Start the cell service.
    ///
    /// Loads space definitions from XML, creates startup spaces, and begins
    /// processing messages from BaseApp.
    pub async fn start(&mut self) -> Result<(), CellError> {
        tracing::info!(addr = %self.listener_addr, "Starting cell service");

        // Load space definitions from XML
        let mut space_mgr = SpaceManager::new(1); // cell_id = 1
        match space_mgr.load_from_xml(&self.entities_dir) {
            Ok(()) => {
                tracing::info!(
                    worlds = space_mgr.world_count(),
                    startup_spaces = space_mgr.space_count(),
                    "Cell service loaded space definitions"
                );
            }
            Err(e) => {
                tracing::warn!("Failed to load space definitions: {e} — continuing with empty space set");
            }
        }

        // Spawn initial NPC population
        let npc_count = spawner::spawn_initial_npcs(&mut space_mgr);
        tracing::info!(npc_count, "NPC population initialized");

        // Send SpaceData for all startup spaces to BaseApp
        if let Some(ref tx) = self.cell_to_base_tx {
            for (space_id, world_name) in space_mgr.all_spaces() {
                let msg = CellToBaseMsg::SpaceData {
                    space_id,
                    world_name,
                };
                if tx.send(msg).await.is_err() {
                    tracing::warn!("Failed to send SpaceData to BaseApp (channel closed)");
                    break;
                }
            }
        }

        // Build the content engine — load from DB if available, else fallback
        let engine = content::build_engine(self.db_pool.as_deref()).await;
        let db_pool = self.db_pool.clone();

        // Take ownership of channels for the message processing loop
        let rx = self.base_to_cell_rx.take();
        let tx = self.cell_to_base_tx.clone();

        if let (Some(mut rx), Some(tx)) = (rx, tx) {
            tokio::spawn(async move {
                run_cell_loop(&mut rx, &tx, space_mgr, engine, db_pool).await;
            });
        } else {
            tracing::warn!("Cell service started without channels — operating in stub mode");
        }

        self.is_running = true;
        Ok(())
    }

    /// Stop the cell service gracefully.
    ///
    /// Stops the tick loop, saves spatial state, and notifies the BaseApp.
    pub async fn stop(&mut self) {
        tracing::info!("Stopping cell service");
        // Dropping the channel receiver will cause the loop to exit
        self.base_to_cell_rx = None;
        self.cell_to_base_tx = None;
        self.is_running = false;
        tracing::trace!("Cell service stopped");
    }
}

/// Main CellService message processing loop.
async fn run_cell_loop(
    rx: &mut mpsc::Receiver<BaseToCellMsg>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    mut space_mgr: SpaceManager,
    mut engine: ChainEngine,
    db_pool: Option<Arc<PgPool>>,
) {
    tracing::debug!("Cell service message loop started");

    // Tick loop: process messages and run AoI checks
    let mut tick_interval = tokio::time::interval(std::time::Duration::from_millis(100));

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Some(BaseToCellMsg::ReloadContentEngine) => {
                        tracing::info!("Hot-reloading content engine from database");
                        engine = content::build_engine(db_pool.as_deref()).await;
                        tracing::info!(chains = engine.chain_count(), "Content engine reloaded");
                    }
                    Some(msg) => handle_base_message(msg, tx, &mut space_mgr, &engine).await,
                    None => {
                        tracing::info!("Cell service channel closed — shutting down");
                        break;
                    }
                }
            }
            _ = tick_interval.tick() => {
                run_aoi_tick(tx, &mut space_mgr).await;
            }
        }
    }

    tracing::debug!("Cell service message loop exited");
}

/// Handle a single message from BaseApp.
async fn handle_base_message(
    msg: BaseToCellMsg,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    match msg {
        BaseToCellMsg::CreateEntity { entity_id, world_name, position, rotation, reply_tx } => {
            tracing::debug!(entity_id, %world_name, ?position, "CreateEntity");

            // Check if this is a new instanced space (no entities yet)
            let is_new_space = !space_mgr.has_space_for_world(&world_name);

            match space_mgr.create_entity(entity_id, &world_name, position, rotation) {
                Ok(space_id) => {
                    // Spawn instance NPCs if the space was newly created
                    if is_new_space {
                        let npc_count = spawner::spawn_npcs_for_world(&world_name, space_mgr);
                        if npc_count > 0 {
                            tracing::info!(world = %world_name, npc_count, "Spawned instance NPCs");
                        }
                    }

                    let _ = reply_tx.send(space_id);
                    let _ = tx.send(CellToBaseMsg::EntityCreated {
                        entity_id,
                        space_id,
                        position,
                    }).await;
                }
                Err(e) => {
                    tracing::error!(entity_id, %world_name, "Failed to create entity: {e}");
                }
            }
        }

        BaseToCellMsg::DestroyEntity { entity_id } => {
            tracing::debug!(entity_id, "DestroyEntity");
            space_mgr.destroy_entity(entity_id);
        }

        BaseToCellMsg::ConnectEntity { entity_id } => {
            tracing::debug!(entity_id, "ConnectEntity (player)");
            space_mgr.connect_entity(entity_id);
        }

        BaseToCellMsg::DisconnectEntity { entity_id } => {
            tracing::debug!(entity_id, "DisconnectEntity");
            space_mgr.disconnect_entity(entity_id, tx).await;
        }

        BaseToCellMsg::EntityMove { entity_id, position, direction, velocity } => {
            tracing::trace!(entity_id, ?position, "EntityMove");
            space_mgr.update_entity_position(entity_id, position, direction, velocity);
        }

        BaseToCellMsg::CellMethodCall { entity_id, method_index, args } => {
            dispatch::dispatch_cell_method(entity_id, method_index, &args, tx, space_mgr, engine).await;
        }

        BaseToCellMsg::ChatMessage { entity_id, speaker_name, speaker_flags, channel, text } => {
            chat::handle_chat_message(entity_id, &speaker_name, speaker_flags, channel, &text, tx, space_mgr).await;
        }

        BaseToCellMsg::InitPlayerState { entity_id, player_id, world_name } => {
            tracing::debug!(entity_id, player_id, %world_name, "InitPlayerState");
            // Set player_id on the cell entity for later persistence lookups
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                entity.player_id = Some(player_id);
            }
            content::fire_player_loaded(entity_id, player_id, &world_name, engine, tx, space_mgr).await;
        }

        // Handled in run_cell_loop before dispatch; included for exhaustiveness.
        BaseToCellMsg::ReloadContentEngine => {}
    }
}

/// Run one tick of AoI processing across all spaces.
async fn run_aoi_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let events = space_mgr.compute_aoi_changes();
    for event in events {
        if tx.send(event).await.is_err() {
            tracing::warn!("Failed to send AoI event to BaseApp (channel closed)");
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_service_is_not_running() {
        let config = ServerConfig::default();
        let svc = CellService::new(&config);
        assert!(!svc.is_running);
        assert_eq!(svc.listener_addr.port(), 50000);
    }

    #[tokio::test]
    async fn start_sets_running() {
        let config = ServerConfig::default();
        let mut svc = CellService::new(&config);
        svc.start().await.unwrap();
        assert!(svc.is_running);
    }
}
