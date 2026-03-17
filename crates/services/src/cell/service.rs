//! CellService lifecycle — construction, startup, message loop, AoI tick.

use std::net::SocketAddr;
use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_common::ServerConfig;
use cimmeria_content_engine::chain::ChainEngine;

use super::content;
use super::messages::{BaseToCellMsg, CellToBaseMsg};
use super::space_manager::SpaceManager;
use super::{CellError, chat, dispatch, spawner};

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

    /// Wire in the Base<->Cell channels. Called by the orchestrator before `start()`.
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

        // Load spawn records from DB and populate startup spaces
        let spawn_records = if let Some(ref pool) = self.db_pool {
            match spawner::load_spawns_from_db(pool).await {
                Ok(records) => {
                    tracing::info!(count = records.len(), "Loaded spawn records from database");
                    records
                }
                Err(e) => {
                    tracing::warn!("Failed to load spawn records: {e} — using hardcoded fallback");
                    vec![]
                }
            }
        } else {
            vec![]
        };

        let npc_count = spawner::spawn_npcs_from_records(&spawn_records, &mut space_mgr);
        tracing::info!(npc_count, "NPC population initialized");

        // Load dialog_set_maps cache for per-player interaction system
        if let Some(ref pool) = self.db_pool {
            match spawner::load_dialog_set_maps(pool).await {
                Ok(maps) => {
                    space_mgr.dialog_set_maps = maps;
                }
                Err(e) => {
                    tracing::warn!("Failed to load dialog_set_maps: {e}");
                }
            }
        }

        // Load mission definitions cache for AcceptMission content actions
        if let Some(ref pool) = self.db_pool {
            match spawner::load_mission_defs(pool).await {
                Ok(defs) => {
                    space_mgr.mission_defs = defs;
                }
                Err(e) => {
                    tracing::warn!("Failed to load mission_defs: {e}");
                }
            }
        }

        // Load stargate destinations cache for gate travel
        if let Some(ref pool) = self.db_pool {
            match spawner::load_stargates(pool).await {
                Ok(gates) => {
                    space_mgr.stargates = gates;
                }
                Err(e) => {
                    tracing::warn!("Failed to load stargates: {e}");
                }
            }
        }

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
                run_cell_loop(&mut rx, &tx, space_mgr, engine, db_pool, spawn_records).await;
            });
        } else {
            tracing::warn!("Cell service started without channels — operating in stub mode");
        }

        self.is_running = true;
        Ok(())
    }

    /// Stop the cell service gracefully.
    pub async fn stop(&mut self) {
        tracing::info!("Stopping cell service");
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
    spawn_records: Vec<spawner::SpawnRecord>,
) {
    tracing::debug!("Cell service message loop started");

    let mut tick_interval = tokio::time::interval(std::time::Duration::from_millis(100));
    let mut aoi_tick_counter: u32 = 0;

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Some(BaseToCellMsg::ReloadContentEngine) => {
                        tracing::info!("Hot-reloading content engine from database");
                        engine = content::build_engine(db_pool.as_deref()).await;
                        tracing::info!(chains = engine.chain_count(), "Content engine reloaded");
                    }
                    Some(msg) => handle_base_message(msg, tx, &mut space_mgr, &engine, &spawn_records).await,
                    None => {
                        tracing::info!("Cell service channel closed — shutting down");
                        break;
                    }
                }
            }
            _ = tick_interval.tick() => {
                run_aoi_tick(tx, &mut space_mgr).await;

                // NPC AI runs every 20th AoI tick (2 seconds at 100ms intervals)
                aoi_tick_counter = aoi_tick_counter.wrapping_add(1);
                if aoi_tick_counter % 20 == 0 {
                    npc_ai_tick(tx, &mut space_mgr).await;
                }
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
    spawn_records: &[spawner::SpawnRecord],
) {
    match msg {
        BaseToCellMsg::CreateEntity { entity_id, world_name, position, rotation, reply_tx } => {
            tracing::debug!(entity_id, %world_name, ?position, "CreateEntity");

            let is_new_space = !space_mgr.has_space_for_world(&world_name);

            match space_mgr.create_entity(entity_id, &world_name, position, rotation) {
                Ok(space_id) => {
                    if is_new_space {
                        let npc_count = spawner::spawn_instance_npcs_from_records(spawn_records, &world_name, space_mgr);
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

        BaseToCellMsg::InitPlayerState { entity_id, player_id, world_name, saved_missions } => {
            tracing::debug!(entity_id, player_id, %world_name, saved_count = saved_missions.len(), "InitPlayerState");
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                entity.player_id = Some(player_id);

                // Restore saved missions BEFORE content engine fires, so that
                // chain conditions correctly see existing mission state and
                // don't re-trigger already-active or completed missions.
                for saved in &saved_missions {
                    use cimmeria_entity::missions::{MissionInstance, MissionObjective, STATUS_ACTIVE, STATUS_COMPLETED};
                    let objectives: Vec<MissionObjective> = saved.active_objective_ids.iter()
                        .map(|&oid| {
                            let status = if saved.completed_objective_ids.contains(&oid) {
                                STATUS_COMPLETED
                            } else {
                                STATUS_ACTIVE
                            };
                            MissionObjective {
                                objective_id: oid,
                                status,
                                hidden: false,
                                optional: false,
                            }
                        })
                        .collect();

                    let mut mission = MissionInstance::new(
                        saved.mission_id,
                        saved.current_step_id.unwrap_or(0),
                        objectives,
                    );
                    mission.status = saved.status;
                    mission.completed_steps = saved.completed_step_ids.clone();
                    mission.completed_objectives = saved.completed_objective_ids.clone();

                    entity.missions.add_mission(mission);
                    tracing::debug!(
                        entity_id, mission_id = saved.mission_id,
                        status = saved.status, "Restored saved mission"
                    );
                }
                entity.saved_missions_loaded = true;
            }
            content::fire_player_loaded(entity_id, player_id, &world_name, engine, tx, space_mgr).await;
        }

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

/// NPC AI tick — runs every 2 seconds (every 20th AoI tick).
///
/// For each NPC in Fighting state: find top threat target, check leash
/// distance, and attack with default ability. For NPCs in Leashing state:
/// reset to Idle when close enough to spawn point, restore health.
async fn npc_ai_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::AiState;

    // Snapshot NPC IDs and their AI state so we don't hold a borrow on space_mgr
    // while calling handle_use_ability (which needs &mut SpaceManager).
    let npc_snapshot: Vec<(u32, AiState)> = space_mgr.all_npc_entity_ids()
        .iter()
        .filter_map(|&eid| {
            space_mgr.get_entity(eid).map(|e| (eid, e.ai_state))
        })
        .filter(|(_, state)| *state == AiState::Fighting || *state == AiState::Leashing)
        .collect();

    for (npc_id, ai_state) in npc_snapshot {
        match ai_state {
            AiState::Fighting => {
                npc_ai_fight(npc_id, tx, space_mgr).await;
            }
            AiState::Leashing => {
                npc_ai_leash(npc_id, tx, space_mgr).await;
            }
            _ => {}
        }
    }
}

/// NPC fighting behavior: attack top-threat target or leash if too far from spawn.
async fn npc_ai_fight(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::AiState;
    use super::combat;

    // Read NPC state (immutable borrow)
    let (top_target, spawn_pos, _npc_pos) = {
        let npc = match space_mgr.get_entity(npc_id) {
            Some(e) => e,
            None => return,
        };

        // Find highest-threat target
        let top = npc.threat_list.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&eid, _)| eid);

        (top, npc.spawn_position, npc.position)
    };

    let target_id = match top_target {
        Some(tid) => tid,
        None => {
            // No threat targets left — reset to idle
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.ai_state = AiState::Idle;
                npc.threat_list.clear();
                tracing::debug!(npc_id, "NPC AI: no threat targets, resetting to Idle");
            }
            return;
        }
    };

    // Check if target still exists
    let target_pos = match space_mgr.get_entity(target_id) {
        Some(t) => t.position,
        None => {
            // Target gone (disconnected), remove from threat and re-evaluate
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.threat_list.remove(&target_id);
            }
            return;
        }
    };

    // Leash check: if target is too far from NPC's spawn point, disengage
    if let Some(spawn) = spawn_pos {
        let dist_to_spawn = spawn.distance_to(&target_pos);
        if dist_to_spawn > combat::LEASH_DISTANCE {
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.ai_state = AiState::Leashing;
                npc.threat_list.clear();
                tracing::info!(
                    npc_id, target = target_id,
                    distance = dist_to_spawn,
                    "NPC AI: target too far from spawn, leashing"
                );
            }
            return;
        }
    }

    // Attack the target with the default ability
    tracing::debug!(npc_id, target = target_id, "NPC AI: attacking top threat target");
    super::abilities::handle_use_ability(
        npc_id,
        combat::NPC_DEFAULT_ABILITY,
        target_id as i32,
        tx,
        space_mgr,
    ).await;
}

/// NPC leashing behavior: reset to Idle and restore health.
///
/// In a full implementation this would pathfind the NPC back to spawn.
/// For now we snap back instantly and restore health.
async fn npc_ai_leash(
    npc_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    use cimmeria_entity::cell_entity::AiState;

    let npc = match space_mgr.get_entity_mut(npc_id) {
        Some(e) => e,
        None => return,
    };

    // Snap back to spawn position
    if let Some(spawn_pos) = npc.spawn_position {
        npc.position = spawn_pos;
    }

    // Restore health to max
    if let Some(health) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
        health.set_current(health.max);
    }

    // Clear dead state flag
    npc.ai_state = AiState::Idle;
    npc.threat_list.clear();
    npc.abilities.clear_all_cooldowns();

    tracing::info!(npc_id, "NPC AI: leash complete, reset to Idle with full health");

    // Send stat update to witnesses so they see health restored
    let stat_update = npc.stats.serialize_dirty();
    npc.stats.clear_dirty();

    // State field update (clear dead flag)
    let mut state_field = 0u32;
    super::combat::clear_dead_state(&mut state_field);

    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id: npc_id,
        method_index: 20, // onStatUpdate
        args: stat_update,
    }).await;

    let mut state_args = Vec::with_capacity(4);
    state_args.extend_from_slice(&state_field.to_le_bytes());
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id: npc_id,
        method_index: 19, // onStateFieldUpdate
        args: state_args,
    }).await;
}
