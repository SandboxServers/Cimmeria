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

    /// Get a clone of the CellToBase sender (for minigame result routing).
    pub fn cell_to_base_tx(&self) -> Option<mpsc::Sender<CellToBaseMsg>> {
        self.cell_to_base_tx.clone()
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

        // Load step objectives cache for AdvanceStep content actions
        if let Some(ref pool) = self.db_pool {
            match spawner::load_step_objectives(pool).await {
                Ok(objs) => {
                    space_mgr.step_objectives = objs;
                }
                Err(e) => {
                    tracing::warn!("Failed to load step_objectives: {e}");
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

        // Load generic regions from DB and register with auto-incrementing runtime IDs.
        // Reference: python/cell/GenericRegion.py — GenericRegionManager.load() + registerRegion()
        if let Some(ref pool) = self.db_pool {
            match spawner::load_regions_from_db(pool).await {
                Ok(region_data) => {
                    for rd in region_data {
                        let runtime_id = space_mgr.next_region_id;
                        space_mgr.next_region_id += 1;
                        space_mgr.regions.insert(runtime_id, super::space_manager::RegionData {
                            runtime_id,
                            db_set_id: rd.set_id,
                            tag: rd.name,
                            world_name: rd.world_name,
                            height: rd.height,
                            radius: rd.radius,
                            flags: rd.flags,
                            points: rd.points,
                        });
                    }
                    tracing::info!(count = space_mgr.regions.len(), "Registered generic regions");
                }
                Err(e) => {
                    tracing::warn!("Failed to load generic regions: {e}");
                }
            }
        }

        // Load respawner definitions for death/respawn system
        if let Some(ref pool) = self.db_pool {
            match spawner::load_respawners(pool).await {
                Ok(defs) => {
                    space_mgr.respawners = defs;
                }
                Err(e) => {
                    tracing::warn!("Failed to load respawners: {e}");
                }
            }
        }

        // Load ability + effect definitions from DB
        if let Some(ref pool) = self.db_pool {
            match spawner::load_ability_defs(pool).await {
                Ok(defs) => { space_mgr.ability_defs = defs; }
                Err(e) => { tracing::warn!("Failed to load ability defs: {e}"); }
            }
            match spawner::load_effect_defs(pool).await {
                Ok(defs) => { space_mgr.effect_defs = defs; }
                Err(e) => { tracing::warn!("Failed to load effect defs: {e}"); }
            }
            match spawner::load_event_set_sequences(pool).await {
                Ok(map) => { space_mgr.sequence_map = map; }
                Err(e) => { tracing::warn!("Failed to load event_set sequences: {e}"); }
            }
            match spawner::load_item_containers(pool).await {
                Ok(map) => { space_mgr.item_containers = map; }
                Err(e) => { tracing::warn!("Failed to load item containers: {e}"); }
            }
            match spawner::load_loot_tables(pool).await {
                Ok(tables) => { space_mgr.loot_tables = tables; }
                Err(e) => { tracing::warn!("Failed to load loot tables: {e}"); }
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

                aoi_tick_counter = aoi_tick_counter.wrapping_add(1);

                // NPC movement runs every AoI tick (100ms) for smooth pathing
                npc_movement_tick(&mut space_mgr);

                // NPC AI runs every 20th AoI tick (2 seconds at 100ms intervals)
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

            // For instanced worlds, every CreateEntity gets a new space with its
            // own NPCs. For non-instanced worlds, the space already exists from
            // startup and NPCs were spawned then.
            let is_instanced = space_mgr.is_world_instanced(&world_name);

            match space_mgr.create_entity(entity_id, &world_name, position, rotation) {
                Ok(space_id) => {
                    if is_instanced {
                        // Notify BaseApp about the new instanced space so it can
                        // route entity messages to it
                        let _ = tx.send(CellToBaseMsg::SpaceData {
                            space_id,
                            world_name: world_name.clone(),
                        }).await;

                        let npc_count = spawner::spawn_instance_npcs_from_records(spawn_records, &world_name, space_id, space_mgr);
                        if npc_count > 0 {
                            tracing::info!(world = %world_name, space_id, npc_count, "Spawned instance NPCs");
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

        BaseToCellMsg::InitPlayerState { entity_id, player_id, world_name, saved_missions, abilities, active_bandolier_slot, bandolier_items } => {
            tracing::debug!(entity_id, player_id, %world_name, saved_count = saved_missions.len(), ability_count = abilities.len(), "InitPlayerState");
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                entity.player_id = Some(player_id);

                // Register player's known abilities on the server-side entity
                for &ability_id in &abilities {
                    entity.abilities.add_ability(ability_id);
                }
                tracing::debug!(entity_id, count = abilities.len(), "Registered player abilities on cell entity");

                // Apply bandolier state to entity (Bug #2: restore persisted bandolier slot and items)
                entity.active_bandolier_slot = active_bandolier_slot;
                entity.bandolier_items = bandolier_items.into_iter().collect();
                tracing::debug!(
                    entity_id,
                    active_bandolier_slot,
                    bandolier_item_count = entity.bandolier_items.len(),
                    "Applied bandolier state to cell entity"
                );

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

            // Send addClientHintedGenericRegion for each client-hinted region in
            // this world. Matches Python Space.playerEntered() → queryRegions():
            // clearClientHintedGenericRegions was already sent in mapLoaded body,
            // now register all regions so the client can fire triggerRegion events.
            {
                use super::space_manager::REGION_FLAG_CLIENT_HINTED;
                let world_regions: Vec<_> = space_mgr.regions_for_world(&world_name)
                    .iter()
                    .filter(|r| r.flags & REGION_FLAG_CLIENT_HINTED != 0)
                    .map(|r| (r.runtime_id, r.height, r.radius, r.flags, r.points.clone()))
                    .collect();

                let region_count = world_regions.len();
                for (rid, height, radius, flags, points) in world_regions {
                    let mut args = Vec::with_capacity(16 + points.len() * 12);
                    args.extend_from_slice(&(rid as i32).to_le_bytes());
                    args.extend_from_slice(&height.to_le_bytes());
                    args.extend_from_slice(&radius.to_le_bytes());
                    args.extend_from_slice(&flags.to_le_bytes());
                    args.extend_from_slice(&(points.len() as u32).to_le_bytes()); // ARRAY count
                    for p in &points {
                        args.extend_from_slice(&p[0].to_le_bytes()); // x
                        args.extend_from_slice(&p[1].to_le_bytes()); // y
                        args.extend_from_slice(&p[2].to_le_bytes()); // z
                    }
                    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                        entity_id,
                        method_index: 125, // addClientHintedGenericRegion
                        args,
                    }).await;
                }
                if region_count > 0 {
                    tracing::info!(
                        entity_id, player_id, world = %world_name,
                        count = region_count, "Sent region registrations"
                    );
                }
            }

            content::fire_player_loaded(entity_id, player_id, &world_name, engine, tx, space_mgr).await;
        }

        BaseToCellMsg::ReloadContentEngine => {}

        BaseToCellMsg::MinigameResult { entity_id, result_code, on_victory_chains } => {
            tracing::info!(entity_id, result_code, chains = ?on_victory_chains, "Minigame result");
            if result_code == 1 {
                // Victory — fire on_victory_chains through the content engine
                let player_id = space_mgr.get_entity(entity_id)
                    .and_then(|e| e.player_id)
                    .unwrap_or(0);
                for chain_id in &on_victory_chains {
                    content::fire_chain_by_id(
                        *chain_id as i64,
                        entity_id, player_id,
                        engine, tx, space_mgr,
                    ).await;
                }
            }
        }

        BaseToCellMsg::UpdateBandolierItem { entity_id, slot_id, item, make_active } => {
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                entity.bandolier_items.insert(slot_id, item);
                if make_active {
                    entity.active_bandolier_slot = slot_id;
                }
            }
        }

        BaseToCellMsg::SyncBandolierItems { entity_id, active_bandolier_slot, bandolier_items } => {
            if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                entity.active_bandolier_slot = active_bandolier_slot;
                entity.bandolier_items = bandolier_items.into_iter().collect();
            }
        }

        // Bandolier state is re-synced via SyncBandolierItems; these handlers are
        // logging-only — base owns the inventory mutation, cell only learns about it.
        BaseToCellMsg::InventoryItemMoveApplied { entity_id, item_id, source_container_id, target_container_id, swapped_item_id } => {
            tracing::debug!(entity_id, item_id, source = source_container_id, target = target_container_id, swapped_item_id = ?swapped_item_id, "Item moved in inventory");
        }

        BaseToCellMsg::InventoryItemRemoved { entity_id, item_id, source_container_id } => {
            tracing::debug!(entity_id, item_id, source = source_container_id, "Item removed from inventory");
        }

        BaseToCellMsg::InventoryItemGranted { entity_id, item_id, container_id, slot_id, quantity } => {
            tracing::debug!(entity_id, item_id, container_id, slot_id, quantity, "Item granted to player");
        }
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
/// Advance NPCs along their nav_path waypoints.
///
/// Each tick, if an NPC has a non-empty `nav_path`, move it toward the next
/// waypoint by `move_speed` units. When it reaches (or overshoots) a waypoint,
/// consume it and continue to the next. Position updates propagate to witnesses
/// via the AoI tick's `EntityMoved` messages.
fn npc_movement_tick(space_mgr: &mut SpaceManager) {
    // Collect NPCs that have active paths
    let moving_npcs: Vec<u32> = space_mgr.all_npc_entity_ids()
        .iter()
        .filter(|&&eid| {
            space_mgr.get_entity(eid)
                .map_or(false, |e| !e.nav_path.is_empty())
        })
        .copied()
        .collect();

    for npc_id in moving_npcs {
        // Read the next waypoint, move_speed, and remaining path length
        let (next_wp, move_speed, cur_pos, path_len) = {
            let npc = match space_mgr.get_entity(npc_id) {
                Some(e) if !e.nav_path.is_empty() => e,
                _ => continue,
            };
            (npc.nav_path[0], npc.move_speed, npc.position, npc.nav_path.len())
        };

        let dx = next_wp.x - cur_pos.x;
        let dy = next_wp.y - cur_pos.y;
        let dz = next_wp.z - cur_pos.z;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();

        // Speed in world units per second (tick is 100ms = 0.1s)
        let speed_per_sec = move_speed * 10.0;

        if dist <= move_speed {
            // Reached (or overshot) the waypoint — snap to it and consume
            // Waypoint Y comes from Detour's findStraightPath (already on navmesh surface)
            let snap_y = next_wp.y;

            // Peek at the NEXT waypoint (index 1) to compute velocity toward it
            let next_next_wp = if path_len > 1 {
                space_mgr.get_entity(npc_id).map(|e| e.nav_path[1])
            } else {
                None
            };

            let (velocity, yaw) = if let Some(nn) = next_next_wp {
                // Still more waypoints — compute velocity toward the next one
                let ndx = nn.x - next_wp.x;
                let ndz = nn.z - next_wp.z;
                let ndy = nn.y - next_wp.y;
                let nd = (ndx * ndx + ndy * ndy + ndz * ndz).sqrt();
                if nd > 0.001 {
                    (
                        [ndx / nd * speed_per_sec, ndy / nd * speed_per_sec, ndz / nd * speed_per_sec],
                        ndx.atan2(ndz),
                    )
                } else {
                    ([0.0; 3], 0.0)
                }
            } else {
                // Last waypoint — stopping, keep current facing
                ([0.0; 3], dx.atan2(dz))
            };

            space_mgr.update_entity_position(
                npc_id,
                [next_wp.x, snap_y, next_wp.z],
                [0, 0, 0],
                velocity,
            );
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.nav_path.remove(0);
                npc.direction = cimmeria_common::Vector3::new(0.0, yaw, 0.0);
            }
        } else {
            // Move toward waypoint by move_speed units
            let t = move_speed / dist;
            let new_x = cur_pos.x + dx * t;
            let new_z = cur_pos.z + dz * t;

            // Linearly interpolate Y between current position and waypoint.
            // Waypoints from Detour's findStraightPath are on the navmesh surface,
            // so linear interpolation between them stays close to the floor.
            let new_y = cur_pos.y + dy * t;

            // Face the direction of movement (yaw = atan2(dx, dz) in radians)
            // Direction is [pitch, yaw, roll] — only yaw matters for facing
            let yaw = dx.atan2(dz);

            // Velocity = direction * speed_per_sec
            let velocity = [
                dx / dist * speed_per_sec,
                dy / dist * speed_per_sec,
                dz / dist * speed_per_sec,
            ];

            if (npc_id % 10000) < 5 { // log a few NPCs
                tracing::debug!(
                    npc_id,
                    cur = format_args!("({:.1},{:.1},{:.1})", cur_pos.x, cur_pos.y, cur_pos.z),
                    new = format_args!("({:.1},{:.1},{:.1})", new_x, new_y, new_z),
                    wp = format_args!("({:.1},{:.1},{:.1})", next_wp.x, next_wp.y, next_wp.z),
                    "NPC movement step"
                );
            }

            space_mgr.update_entity_position(
                npc_id,
                [new_x, new_y, new_z],
                [0, 0, 0],
                velocity,
            );
            // Set yaw directly as radians (pack_angle reads direction.y)
            if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                npc.direction = cimmeria_common::Vector3::new(0.0, yaw, 0.0);
            }
        }
    }
}

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
    let (top_target, spawn_pos, npc_pos) = {
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

    // Check if target still exists and is alive
    let target_pos = match space_mgr.get_entity(target_id) {
        Some(t) => {
            // Don't attack dead targets
            let is_dead = t.stats.get(cimmeria_entity::stats::HEALTH)
                .map_or(true, |s| s.cur <= 0);
            if is_dead {
                if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                    npc.threat_list.remove(&target_id);
                    tracing::debug!(npc_id, target = target_id, "NPC AI: target is dead, removing from threat");
                }
                return;
            }
            t.position
        }
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

    // Range check: don't attack until target is within weapon range
    let dist_to_target = npc_pos.distance_to(&target_pos);
    let in_range = dist_to_target <= combat::NPC_ATTACK_RANGE;
    let has_los = space_mgr.has_line_of_sight(npc_id, target_id);

    if !in_range {
        // Out of range — pathfind toward target, but only recalculate if:
        // 1. NPC has no active path, OR
        // 2. Target has moved significantly from the path's destination
        let needs_repath = {
            let npc = space_mgr.get_entity(npc_id);
            match npc {
                Some(e) if !e.nav_path.is_empty() => {
                    // Check if target moved far from the last waypoint
                    let last_wp = e.nav_path[e.nav_path.len() - 1];
                    last_wp.distance_to(&target_pos) > 5.0
                }
                _ => true, // No path — need one
            }
        };

        if needs_repath {
            if let Some(path) = space_mgr.find_path(npc_id, &npc_pos, &target_pos) {
                if path.len() > 1 {
                    let waypoints: Vec<_> = path.into_iter().skip(1).collect();
                    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
                        npc.nav_path = waypoints;
                    }
                    tracing::debug!(
                        npc_id, target = target_id,
                        in_range, has_los,
                        "NPC AI: pathfinding toward target"
                    );
                }
            } else {
                tracing::debug!(
                    npc_id, target = target_id,
                    "NPC AI: no path to target"
                );
            }
        }
        return;
    }

    // In range — stop moving and attack (LoS check skipped to prevent jittering;
    // the ability's range check handles the actual validation)
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.nav_path.clear();
    }

    // Attack the target with the default ability
    tracing::debug!(npc_id, target = target_id, distance = dist_to_target, "NPC AI: attacking top threat target");
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

    let (stat_update, state_field) = {
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

        // Clear dead flag from the NPC's actual state_field
        super::combat::clear_dead_state(&mut npc.state_field);

        tracing::info!(npc_id, "NPC AI: leash complete, reset to Idle with full health");

        // Collect data before dropping the mutable borrow
        let stat_update = npc.stats.serialize_dirty();
        npc.stats.clear_dirty();
        let state_field = npc.state_field;
        (stat_update, state_field)
    };

    super::abilities::send_entity_method(npc_id, 20, stat_update, tx, space_mgr).await;

    let mut state_args = Vec::with_capacity(4);
    state_args.extend_from_slice(&state_field.to_le_bytes());
    super::abilities::send_entity_method(npc_id, 19, state_args, tx, space_mgr).await;
}
