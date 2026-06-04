//! CellService startup: load XML world defs, hydrate DB caches, spawn the message loop.

use std::sync::Arc;

use tokio::sync::Notify;

use super::super::content;
use super::super::cover;
use super::super::messages::CellToBaseMsg;
use super::super::space_manager::SpaceManager;
use super::super::{spawner, CellError};
use super::CellService;

impl CellService {
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
                tracing::warn!(
                    "Failed to load space definitions: {e} — continuing with empty space set"
                );
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

        // Load monologue dialog ids for the DisplayDialog executor.
        // ERROR (not WARN) on failure: an empty cache silently strips
        // ~42% of authored dialog screens (the inner-thought ones)
        // from world entry. The executor falls through to bail-and-
        // warn so behavior stays safe, but operators need to see this.
        if let Some(ref pool) = self.db_pool {
            match spawner::load_monologue_dialog_ids(pool).await {
                Ok(ids) => {
                    space_mgr.monologue_dialog_ids = ids;
                }
                Err(e) => {
                    tracing::error!("Failed to load monologue dialog ids: {e}");
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
                        space_mgr.regions.insert(
                            runtime_id,
                            super::super::space_manager::RegionData {
                                runtime_id,
                                db_set_id: rd.set_id,
                                tag: rd.name,
                                world_name: rd.world_name,
                                height: rd.height,
                                radius: rd.radius,
                                flags: rd.flags,
                                points: rd.points,
                            },
                        );
                    }
                    tracing::info!(
                        count = space_mgr.regions.len(),
                        "Registered generic regions"
                    );
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
                Ok(defs) => {
                    space_mgr.ability_defs = defs;
                }
                Err(e) => {
                    tracing::warn!("Failed to load ability defs: {e}");
                }
            }
            match spawner::load_effect_defs(pool).await {
                Ok(defs) => {
                    space_mgr.effect_defs = defs;
                }
                Err(e) => {
                    tracing::warn!("Failed to load effect defs: {e}");
                }
            }
            match spawner::load_event_set_sequences(pool).await {
                Ok(map) => {
                    space_mgr.sequence_map = map;
                }
                Err(e) => {
                    tracing::warn!("Failed to load event_set sequences: {e}");
                }
            }
            // Cover-system data. Builds the per-process spatial index from
            // `resources.cover_sets` + `resources.cover_nodes`. Initialises
            // if either load returns any rows; stays on `Cover::empty()`
            // only when both come back empty (load failure or fresh DB).
            // The rest of the cell service still functions; NPCs just
            // won't use cover until the load is repaired.
            let cover_sets = match cover::load_cover_sets(pool).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("Failed to load cover sets: {e}");
                    Vec::new()
                }
            };
            let cover_nodes = match cover::load_cover_nodes(pool).await {
                Ok(n) => n,
                Err(e) => {
                    tracing::error!("Failed to load cover nodes: {e}");
                    Vec::new()
                }
            };
            if !cover_sets.is_empty() || !cover_nodes.is_empty() {
                space_mgr.cover = cover::Cover::from_loaded(cover_sets, cover_nodes);
                tracing::info!(
                    sets = space_mgr.cover.set_count(),
                    nodes = space_mgr.cover.node_count(),
                    "Cover service loaded"
                );
            }
            match spawner::load_item_containers(pool).await {
                Ok(map) => {
                    space_mgr.item_containers = map;
                }
                Err(e) => {
                    tracing::warn!("Failed to load item containers: {e}");
                }
            }
            match spawner::load_item_event_set_abilities(pool).await {
                Ok(map) => {
                    space_mgr.item_event_set_abilities = map;
                }
                Err(e) => {
                    tracing::warn!("Failed to load items_event_sets abilities: {e}");
                }
            }
            match spawner::load_archetype_ability_trees(pool).await {
                Ok(map) => {
                    space_mgr.archetype_ability_trees = map;
                }
                Err(e) => {
                    tracing::warn!("Failed to load archetype ability trees: {e}");
                }
            }
            match spawner::load_trainer_abilities(pool).await {
                Ok(map) => {
                    space_mgr.trainer_abilities = map;
                }
                Err(e) => {
                    tracing::warn!("Failed to load trainer abilities: {e}");
                }
            }
            match spawner::load_template_trainer_lists(pool).await {
                Ok(map) => {
                    space_mgr.template_trainer_lists = map;
                }
                Err(e) => {
                    tracing::warn!("Failed to load template trainer lists: {e}");
                }
            }
            match spawner::load_item_defs(pool).await {
                Ok(map) => {
                    space_mgr.item_defs = map;
                }
                Err(e) => {
                    tracing::warn!("Failed to load item defs: {e}");
                }
            }
            match spawner::load_loot_tables(pool).await {
                Ok(tables) => {
                    space_mgr.loot_tables = tables;
                }
                Err(e) => {
                    tracing::warn!("Failed to load loot tables: {e}");
                }
            }
            match super::super::ring_transport::load_ring_regions(pool).await {
                Ok(regions) => {
                    space_mgr.ring_transporters.load(&regions);
                    // Multiple rings sharing a point_set_id would silently
                    // route everyone through whichever ring HashMap iteration
                    // landed last. Log the collision so the bad seed data
                    // surfaces at startup rather than as ghost-routing later.
                    let mut point_set_to_region =
                        std::collections::HashMap::with_capacity(regions.len());
                    for (rid, r) in &regions {
                        if let Some(existing) = point_set_to_region.insert(r.point_set_id, *rid) {
                            tracing::error!(
                                point_set_id = r.point_set_id,
                                first_region = existing, second_region = *rid,
                                "duplicate point_set_id across ring regions — routing will be non-deterministic"
                            );
                        }
                    }
                    space_mgr.ring_point_set_to_region = point_set_to_region;
                    space_mgr.ring_regions = regions;
                    tracing::info!(
                        count = space_mgr.ring_regions.len(),
                        "Initialized ring transporters"
                    );
                }
                Err(e) => {
                    tracing::warn!("Failed to load ring transport regions: {e}");
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
            // Stash a shutdown signal so `stop()` can ask the loop to exit
            // without dropping the channel out from under it.
            let shutdown = Arc::new(Notify::new());
            self.shutdown_signal = Some(shutdown.clone());
            let handle = tokio::spawn(async move {
                super::message_loop::run_cell_loop(
                    &mut rx,
                    &tx,
                    space_mgr,
                    engine,
                    db_pool,
                    spawn_records,
                    shutdown,
                )
                .await;
            });
            self.cell_loop_handle = Some(handle);
        } else {
            tracing::warn!("Cell service started without channels — operating in stub mode");
        }

        self.is_running = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `CellService::new` must produce a non-running service with no
    /// loop handle. This is the minimal surface we can exercise without
    /// real XML files / DB pools — larger startup tests are deferred to
    /// the integration harness.
    #[test]
    fn new_service_is_not_running() {
        let svc = CellService::new(&cimmeria_common::ServerConfig::default());
        assert!(!svc.is_running, "fresh CellService must not be running");
        assert!(
            svc.cell_loop_handle.is_none(),
            "no loop handle before start()"
        );
        assert!(
            svc.shutdown_signal.is_none(),
            "no shutdown signal before start()"
        );
    }
}
