//! Space management for the CellService.
//!
//! Loads world definitions from `entities/spaces.xml` and creates startup
//! spaces from `entities/cell_spaces.xml`. Manages the lifecycle of space
//! instances and the cell entities within them.

use std::collections::{HashMap, HashSet};

use cimmeria_common::{EntityId, SpaceId, Vector3};
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::navigation::NavMesh;
use cimmeria_entity::space::Space;

use super::messages::CellToBaseMsg;

/// Grid cell size for spatial hashing (world units).
const GRID_CELL_SIZE: f32 = 50.0;

/// Flag indicating this region should be sent to the client for client-side
/// hit testing. Matches `Atrea.enums.REGION_FLAG_ClientHinted`.
pub const REGION_FLAG_CLIENT_HINTED: i32 = 1;

/// A registered generic region from the database.
///
/// Loaded from `resources.point_sets` (type='AreaSet') + `resources.point_set_points`.
/// Each region is assigned a server-side runtime ID (auto-incrementing from 1)
/// that the client sends back in `triggerClientHintedGenericRegion` calls.
///
/// Reference: `python/cell/GenericRegion.py`
#[derive(Debug, Clone)]
pub struct RegionData {
    pub runtime_id: u32,
    pub db_set_id: i32,
    /// Region tag from `point_sets.name` — used as the content engine event key
    /// (e.g., "Castle_Cellblock.Region2"). This IS the key the content engine
    /// matches on, NOT a constructed `{world}.Region{id}` string.
    pub tag: String,
    pub world_name: String,
    pub height: f32,
    pub radius: f32,
    pub flags: i32,
    /// Polygon vertices from `point_set_points`. After the cylinder→bbox workaround,
    /// all regions should have exactly 4 points.
    pub points: Vec<[f32; 3]>,
}

/// World definition parsed from `entities/spaces.xml`.
#[derive(Debug, Clone)]
pub struct WorldDef {
    pub world_name: String,
    pub instanced: bool,
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

/// A live space instance with its entity population.
pub struct SpaceInstance {
    pub space_id: u32,
    pub world_name: String,
    pub space: Space,
    pub entities: HashMap<u32, CellEntity>,
    /// Entity IDs that have a client controller (players).
    pub players: HashSet<u32>,
    /// Navigation mesh for this space (if loaded).
    pub navmesh: Option<NavMesh>,
}

/// Manages spaces and cell entities for one CellApp.
pub struct SpaceManager {
    /// This cell's ID (used in space ID scheme: `(cell_id << 16) | local_index`).
    cell_id: u16,
    /// World definitions keyed by WorldName (from spaces.xml).
    worlds: HashMap<String, WorldDef>,
    /// Active space instances keyed by space_id.
    spaces: HashMap<u32, SpaceInstance>,
    /// Non-instanced world name → space_id (one instance per world).
    world_spaces: HashMap<String, u32>,
    /// Entity ID → space_id lookup for quick entity → space resolution.
    entity_space: HashMap<u32, u32>,
    /// Next local index for space ID allocation.
    next_local_id: u32,
    /// Next NPC entity ID (starts at 100_000 to avoid player ID collision).
    next_npc_id: u32,
    /// Cached dialog_set_maps: dialog_set_map_id → (dialog_id, interaction_flags).
    /// Populated at startup from `resources.dialog_set_maps`.
    pub dialog_set_maps: HashMap<i32, super::spawner::DialogSetMapEntry>,
    /// Cached mission definitions: mission_id → (first step_id, objectives).
    /// Populated at startup from `resources.mission_steps` + `resources.mission_objectives`.
    pub mission_defs: HashMap<i32, super::spawner::MissionDefEntry>,
    /// Cached stargate destinations: stargate_id → (world_name, position, yaw).
    /// Populated at startup from `resources.stargates` + `resources.worlds`.
    pub stargates: HashMap<i32, super::spawner::StargateEntry>,
    /// Cached step objectives: step_id → objectives for that step.
    /// Populated at startup from `resources.mission_objectives`.
    /// Used by `advance_step` to load new objectives when advancing a mission step.
    pub step_objectives: HashMap<i32, Vec<super::spawner::MissionObjectiveDef>>,
    /// Registered generic regions keyed by runtime_id (auto-incrementing from 1).
    /// Loaded from `resources.point_sets` (type='AreaSet') at startup.
    pub regions: HashMap<u32, RegionData>,
    /// Next runtime region ID (auto-incrementing, starts at 1).
    /// Matches Python `GenericRegionManager.lastRegionId`.
    pub next_region_id: u32,
    /// Ability definitions: ability_id → AbilityDef.
    /// Loaded from `resources.abilities` at startup.
    pub ability_defs: HashMap<i32, cimmeria_entity::abilities::AbilityDef>,
    /// Effect definitions: effect_id → EffectDef.
    /// Loaded from `resources.effects` + `resources.effect_nvps` at startup.
    pub effect_defs: HashMap<i32, cimmeria_entity::abilities::EffectDef>,
    /// Event set sequence lookup: (event_set_id, event_id) → sequence_id.
    /// Used to resolve the correct KismetEventSetSeqID for onSequence calls.
    /// Loaded from `resources.event_sets_sequences` + `resources.sequences` at startup.
    pub sequence_map: HashMap<(i32, i32), i32>,
    /// Item → preferred container mapping from `resources.items.container_sets`.
    /// Loaded at startup so runtime item grants go into the correct inventory bag
    /// (e.g. mission items into INV_Mission, weapons into bandolier).
    pub item_containers: HashMap<i32, i32>,
    /// Weapon defs (clip_size + default_ammo_type) keyed by item_id.
    /// Loaded from `resources.items` at startup. Used by the content engine's
    /// GrantItem path to seed bandolier slots when a weapon is granted at
    /// runtime, so the client renders the correct empty magazine.
    pub item_defs: HashMap<i32, super::spawner::WeaponDef>,
    /// Loot tables: loot_table_id → entries.
    /// Loaded from `resources.loot` at startup for NPC death loot generation.
    pub loot_tables: HashMap<i32, Vec<super::spawner::LootTableEntry>>,
    /// Respawner definitions loaded from `resources.respawners`.
    /// Used to populate the Defeat Window and look up respawn positions.
    pub respawners: Vec<super::spawner::RespawnerDef>,
}

impl SpaceManager {
    /// Create a new SpaceManager with the given cell ID.
    pub fn new(cell_id: u16) -> Self {
        Self {
            cell_id,
            worlds: HashMap::new(),
            spaces: HashMap::new(),
            world_spaces: HashMap::new(),
            entity_space: HashMap::new(),
            next_local_id: 0,
            next_npc_id: 100_000,
            dialog_set_maps: HashMap::new(),
            mission_defs: HashMap::new(),
            stargates: HashMap::new(),
            step_objectives: HashMap::new(),
            regions: HashMap::new(),
            next_region_id: 1,
            ability_defs: HashMap::new(),
            effect_defs: HashMap::new(),
            sequence_map: HashMap::new(),
            item_containers: HashMap::new(),
            item_defs: HashMap::new(),
            loot_tables: HashMap::new(),
            respawners: Vec::new(),
        }
    }

    /// Load world definitions from `spaces.xml` and create startup spaces from
    /// `cell_spaces.xml`. Both files are expected in `entities_dir`.
    pub fn load_from_xml(&mut self, entities_dir: &str) -> Result<(), String> {
        // Load world definitions from spaces.xml
        let spaces_path = format!("{}/spaces.xml", entities_dir);
        let spaces_xml = std::fs::read_to_string(&spaces_path)
            .map_err(|e| format!("Failed to read {spaces_path}: {e}"))?;
        self.parse_spaces_xml(&spaces_xml)?;

        // Load startup space list from cell_spaces.xml
        let cell_spaces_path = format!("{}/cell_spaces.xml", entities_dir);
        let cell_spaces_xml = std::fs::read_to_string(&cell_spaces_path)
            .map_err(|e| format!("Failed to read {cell_spaces_path}: {e}"))?;
        self.create_startup_spaces(&cell_spaces_xml)?;

        Ok(())
    }

    /// Parse `spaces.xml` and populate the `worlds` map.
    pub(crate) fn parse_spaces_xml(&mut self, xml: &str) -> Result<(), String> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut reader = Reader::from_str(xml);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() == b"Space" => {
                    let mut world_name = String::new();
                    let mut instanced = false;
                    let mut min_x: i32 = 0;
                    let mut max_x: i32 = 0;
                    let mut min_y: i32 = 0;
                    let mut max_y: i32 = 0;

                    for attr in e.attributes().flatten() {
                        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                        let val = std::str::from_utf8(&attr.value).unwrap_or("");
                        match key {
                            "WorldName" => world_name = val.to_string(),
                            "Instanced" => instanced = val == "true",
                            "MinX" => min_x = val.parse().unwrap_or(0),
                            "MaxX" => max_x = val.parse().unwrap_or(0),
                            "MinY" => min_y = val.parse().unwrap_or(0),
                            "MaxY" => max_y = val.parse().unwrap_or(0),
                            _ => {}
                        }
                    }

                    if !world_name.is_empty() {
                        tracing::trace!(
                            world = %world_name,
                            instanced,
                            "Loaded world definition"
                        );
                        self.worlds.insert(world_name.clone(), WorldDef {
                            world_name,
                            instanced,
                            min_x,
                            max_x,
                            min_y,
                            max_y,
                        });
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("XML parse error: {e}")),
                _ => {}
            }
            buf.clear();
        }

        tracing::info!(count = self.worlds.len(), "Parsed world definitions from spaces.xml");
        Ok(())
    }

    /// Parse `cell_spaces.xml` and create a space instance for each listed world.
    pub(crate) fn create_startup_spaces(&mut self, xml: &str) -> Result<(), String> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut reader = Reader::from_str(xml);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() == b"Space" => {
                    let mut world_name = String::new();
                    for attr in e.attributes().flatten() {
                        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                        let val = std::str::from_utf8(&attr.value).unwrap_or("");
                        if key == "WorldName" {
                            world_name = val.to_string();
                        }
                    }

                    if !world_name.is_empty() {
                        if !self.worlds.contains_key(&world_name) {
                            tracing::warn!(world = %world_name, "Startup space references unknown world — skipping");
                            continue;
                        }
                        let space_id = self.allocate_space_id();
                        self.create_space_instance(space_id, &world_name);
                        self.world_spaces.insert(world_name, space_id);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("XML parse error: {e}")),
                _ => {}
            }
            buf.clear();
        }

        tracing::info!(
            count = self.spaces.len(),
            id_range = format_args!("{}..{}", (self.cell_id as u32) << 16, ((self.cell_id as u32) << 16) | (self.next_local_id.saturating_sub(1))),
            "Created startup spaces from cell_spaces.xml"
        );
        Ok(())
    }

    /// Allocate the next space ID using the `(cell_id << 16) | local_index` scheme.
    fn allocate_space_id(&mut self) -> u32 {
        let id = ((self.cell_id as u32) << 16) | self.next_local_id;
        self.next_local_id += 1;
        id
    }

    /// Create a `SpaceInstance` and insert it into the spaces map.
    fn create_space_instance(&mut self, space_id: u32, world_name: &str) {
        let space = Space::new(
            SpaceId(space_id as i32),
            world_name.to_string(),
            GRID_CELL_SIZE,
        );
        // Try to load navmesh for this space
        let nav_name = world_name.to_lowercase().replace(' ', "_");
        let nav_path = format!("data/spaces/{nav_name}.nav");
        let navmesh = match NavMesh::load(std::path::Path::new(&nav_path)) {
            Ok(nm) => {
                tracing::info!(space_id, world = %world_name, polys = nm.poly_count(),
                    "NavMesh loaded for space");
                Some(nm)
            }
            Err(e) => {
                tracing::debug!(space_id, world = %world_name, path = %nav_path,
                    error = %e, "No navmesh for space (optional)");
                None
            }
        };
        let instance = SpaceInstance {
            space_id,
            world_name: world_name.to_string(),
            space,
            entities: HashMap::new(),
            players: HashSet::new(),
            navmesh,
        };
        tracing::debug!(space_id, world = %world_name, "Created space instance");
        self.spaces.insert(space_id, instance);
    }

    /// Check if a non-instanced space already exists for a world name.
    ///
    /// Only checks `world_spaces` (non-instanced startup spaces). For instanced
    /// worlds this always returns false because instances are per-player and not
    /// cached in `world_spaces`.
    pub fn has_space_for_world(&self, world_name: &str) -> bool {
        self.world_spaces.contains_key(world_name)
    }

    /// Check if a world is marked as instanced in spaces.xml.
    pub fn is_world_instanced(&self, world_name: &str) -> bool {
        self.worlds.get(world_name).map_or(false, |w| w.instanced)
    }

    /// Find or create a space for the given world name.
    ///
    /// - Non-instanced worlds: return the existing startup space from `world_spaces`.
    /// - Instanced worlds: always create a NEW space. Each player gets their own
    ///   private instance (Castle_CellBlock, SGC_W1, etc.). The space is NOT cached
    ///   in `world_spaces` — it lives only in `spaces` and is destroyed when the
    ///   last player leaves.
    pub fn find_or_create_space(&mut self, world_name: &str) -> Result<u32, String> {
        // Non-instanced: return the shared startup space
        if let Some(&space_id) = self.world_spaces.get(world_name) {
            return Ok(space_id);
        }

        // Check if we know about this world at all
        let world_def = self.worlds.get(world_name)
            .ok_or_else(|| format!("Unknown world: {world_name}"))?;

        if !world_def.instanced {
            return Err(format!(
                "Non-instanced world '{world_name}' has no startup space — \
                 it should be listed in cell_spaces.xml"
            ));
        }

        // Instanced: always create a fresh space — do NOT cache in world_spaces
        let space_id = self.allocate_space_id();
        self.create_space_instance(space_id, world_name);

        tracing::info!(space_id, world = %world_name, "Created new instanced space");
        Ok(space_id)
    }

    /// Create a cell entity in the appropriate space.
    pub fn create_entity(
        &mut self,
        entity_id: u32,
        world_name: &str,
        position: [f32; 3],
        rotation: [f32; 3],
    ) -> Result<u32, String> {
        let space_id = self.find_or_create_space(world_name)?;

        let pos = Vector3::new(position[0], position[1], position[2]);
        let dir = Vector3::new(rotation[0], rotation[1], rotation[2]);

        let mut cell_entity = CellEntity::new(
            EntityId(entity_id as i32),
            SpaceId(space_id as i32),
            pos,
        );
        cell_entity.direction = dir;

        let space = self.spaces.get_mut(&space_id)
            .ok_or_else(|| format!("Space {space_id} disappeared"))?;

        space.space.add_entity(EntityId(entity_id as i32), &pos);
        space.entities.insert(entity_id, cell_entity);
        self.entity_space.insert(entity_id, space_id);

        tracing::debug!(entity_id, space_id, ?position, "Cell entity created");
        Ok(space_id)
    }

    /// Destroy a cell entity, removing it from its space.
    ///
    /// If the entity was in an instanced space and was the last player, the
    /// entire space instance is destroyed (all remaining NPCs removed).
    pub fn destroy_entity(&mut self, entity_id: u32) {
        if let Some(space_id) = self.entity_space.remove(&entity_id) {
            let mut should_destroy_space = false;

            if let Some(space) = self.spaces.get_mut(&space_id) {
                if let Some(cell_entity) = space.entities.remove(&entity_id) {
                    space.space.remove_entity(
                        EntityId(entity_id as i32),
                        &cell_entity.position,
                    );
                }
                space.players.remove(&entity_id);

                // Check if this was the last player in an instanced space
                if space.players.is_empty() {
                    let world_name = &space.world_name;
                    if self.worlds.get(world_name).map_or(false, |w| w.instanced) {
                        should_destroy_space = true;
                    }
                }
            }

            if should_destroy_space {
                self.destroy_space(space_id);
            }
        }
        tracing::debug!(entity_id, "Cell entity destroyed");
    }

    /// Destroy an instanced space, removing all entities and freeing resources.
    ///
    /// Only called for instanced spaces when the last player leaves. Removes
    /// all NPC entities, the space instance, and any entity_space entries.
    fn destroy_space(&mut self, space_id: u32) {
        if let Some(space) = self.spaces.remove(&space_id) {
            let entity_count = space.entities.len();

            // Remove all entity_space entries for entities in this space
            for &eid in space.entities.keys() {
                self.entity_space.remove(&eid);
            }

            tracing::info!(
                space_id,
                world = %space.world_name,
                entities_removed = entity_count,
                "Destroyed instanced space (last player left)"
            );
        }
    }

    /// Mark an entity as having a client controller (player).
    pub fn connect_entity(&mut self, entity_id: u32) {
        if let Some(&space_id) = self.entity_space.get(&entity_id) {
            if let Some(space) = self.spaces.get_mut(&space_id) {
                space.players.insert(entity_id);
                if let Some(entity) = space.entities.get_mut(&entity_id) {
                    entity.is_player = true;
                    entity.class_id = 0x02; // SGWPlayer
                }
                tracing::debug!(entity_id, space_id, "Entity connected (player)");
            }
        }
    }

    /// Remove client controller and clean up AoI witnesses.
    pub async fn disconnect_entity(
        &mut self,
        entity_id: u32,
        tx: &tokio::sync::mpsc::Sender<CellToBaseMsg>,
    ) {
        if let Some(&space_id) = self.entity_space.get(&entity_id) {
            if let Some(space) = self.spaces.get_mut(&space_id) {
                space.players.remove(&entity_id);

                // Notify all entities that had this one in their AoI
                if let Some(cell_entity) = space.entities.get(&entity_id) {
                    let witnesses: Vec<u32> = cell_entity.witnesses
                        .iter()
                        .map(|eid| eid.0 as u32)
                        .collect();
                    for witness_id in witnesses {
                        let _ = tx.send(CellToBaseMsg::LeftAoI {
                            witness_id,
                            entity_id,
                        }).await;
                    }
                }

                // Remove this entity from all other entities' witness sets
                let eid = EntityId(entity_id as i32);
                for other in space.entities.values_mut() {
                    other.witnesses.remove(&eid);
                }
            }
        }

        // Then destroy the cell entity
        self.destroy_entity(entity_id);
        tracing::debug!(entity_id, "Entity disconnected and destroyed");
    }

    /// Update an entity's position from a client movement packet.
    pub fn update_entity_position(
        &mut self,
        entity_id: u32,
        position: [f32; 3],
        direction: [i8; 3],
        velocity: [f32; 3],
    ) {
        let space_id = match self.entity_space.get(&entity_id) {
            Some(&id) => id,
            None => return,
        };

        let space = match self.spaces.get_mut(&space_id) {
            Some(s) => s,
            None => return,
        };

        if let Some(cell_entity) = space.entities.get_mut(&entity_id) {
            let old_pos = cell_entity.position;
            let new_pos = Vector3::new(position[0], position[1], position[2]);

            cell_entity.position = new_pos;
            cell_entity.direction = Vector3::new(
                direction[0] as f32,
                direction[1] as f32,
                direction[2] as f32,
            );
            cell_entity.velocity = velocity;

            // Update the spatial grid
            space.space.grid.update_position(
                EntityId(entity_id as i32),
                &old_pos,
                &new_pos,
            );
        }
    }

    /// Compute AoI changes for all players across all spaces.
    ///
    /// Returns a list of `CellToBaseMsg` events: `EnteredAoI`, `LeftAoI`, `EntityMoved`.
    pub fn compute_aoi_changes(&mut self) -> Vec<CellToBaseMsg> {
        let mut events = Vec::new();

        for space in self.spaces.values_mut() {
            if space.players.is_empty() {
                continue;
            }

            // Collect player IDs to iterate (can't borrow space mutably while iterating)
            let player_ids: Vec<u32> = space.players.iter().copied().collect();

            for &player_id in &player_ids {
                let (player_pos, aoi_radius, player_interactions) = match space.entities.get(&player_id) {
                    Some(e) => (e.position, e.aoi_radius, e.available_interactions.clone()),
                    None => continue,
                };

                // Query the grid for nearby entities
                let candidates = space.space.get_entities_in_range(&player_pos, aoi_radius);

                // Filter to actual AoI: all entities in range (players + NPCs)
                let mut current_aoi: HashSet<u32> = HashSet::new();
                for candidate_eid in &candidates {
                    let cid = candidate_eid.0 as u32;
                    if cid == player_id {
                        continue; // skip self
                    }
                    // Exact distance check
                    if let Some(other) = space.entities.get(&cid) {
                        let dist_sq = player_pos.distance_squared_to(&other.position);
                        if dist_sq <= aoi_radius * aoi_radius {
                            current_aoi.insert(cid);
                        }
                    }
                }

                // Get previous witness set
                let previous_aoi: HashSet<u32> = match space.entities.get(&player_id) {
                    Some(e) => e.witnesses.iter().map(|eid| eid.0 as u32).collect(),
                    None => continue,
                };

                // Entered AoI: in current but not in previous
                for &eid in &current_aoi {
                    if !previous_aoi.contains(&eid) {
                        if let Some(other) = space.entities.get(&eid) {
                            let npc_data = if !other.is_player {
                                Some(super::messages::NpcAoIData {
                                    name_id: other.name_id,
                                    faction: other.faction,
                                    alignment: other.alignment,
                                    entity_flags: other.entity_flags,
                                    // Send the BASE interaction type in the cascade (not merged).
                                    // Dynamic per-player flags are sent as a separate
                                    // InteractionType update below, matching the C++ server's
                                    // createOnClient(base) → dynamicUpdate(merged) flow.
                                    interaction_type: other.interaction_type_flags,
                                    speaker_id: other.speaker_id,
                                    event_set_id: other.event_set_id,
                                    static_mesh: other.static_mesh.clone(),
                                    body_set: other.body_set.clone(),
                                    components: other.components.clone(),
                                })
                            } else {
                                None
                            };
                            events.push(CellToBaseMsg::EnteredAoI {
                                witness_id: player_id,
                                entity_id: eid,
                                space_id: space.space_id,
                                class_id: other.class_id,
                                position: [other.position.x, other.position.y, other.position.z],
                                direction: [other.direction.x, other.direction.y, other.direction.z],
                                level: other.level,
                                npc_data,
                            });

                            // ── dynamicUpdate: standalone InteractionType update ──
                            //
                            // In the C++ server, createOnClient() sends InteractionType
                            // with the entity's BASE flags (often 0). Then dynamicUpdate()
                            // fires and sends InteractionType with the MERGED per-player
                            // flags as a separate message. The client treats this as a
                            // state change that enables right-click interaction.
                            //
                            // Reference: src/cellapp/base_client.cpp:455-458
                            if other.has_dynamic_properties {
                                if let Some(tmpl_id) = other.template_id {
                                    if let Some(entries) = player_interactions.get(&tmpl_id) {
                                        let base = other.interaction_type_flags;
                                        let merged = base | entries.iter().fold(0i64, |acc, &(_, _, f)| acc | f);
                                        if merged != base {
                                            tracing::info!(
                                                player_id, entity_id = eid,
                                                template_id = tmpl_id, base, merged,
                                                "AoI: dynamicUpdate InteractionType (base→merged)"
                                            );
                                        }
                                        events.push(CellToBaseMsg::WitnessEntityMethod {
                                            witness_id: player_id,
                                            entity_id: eid,
                                            method_index: 3, // InteractionType
                                            args: (merged as u64).to_le_bytes().to_vec(),
                                        });

                                        // Register the entity as interactable on the client.
                                        // GameBeing::isInteractable() checks player+0x16c;
                                        // onDuelEntitiesRemove (method 152) adds to this set.
                                        // Must arrive AFTER CREATE_ENTITY so the client can
                                        // find the entity and refresh its interaction state.
                                        events.push(CellToBaseMsg::EntityMethodCall {
                                            entity_id: player_id,
                                            method_index: 152, // onDuelEntitiesRemove
                                            args: (eid as i32).to_le_bytes().to_vec(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

                // Left AoI: in previous but not in current
                for &eid in &previous_aoi {
                    if !current_aoi.contains(&eid) {
                        events.push(CellToBaseMsg::LeftAoI {
                            witness_id: player_id,
                            entity_id: eid,
                        });
                    }
                }

                // Entity moved: in both, send position updates to this witness
                // (BaseApp can diff to skip no-ops if position unchanged)
                for &eid in &current_aoi {
                    if previous_aoi.contains(&eid) {
                        if let Some(other) = space.entities.get(&eid) {
                            events.push(CellToBaseMsg::EntityMoved {
                                witness_id: player_id,
                                entity_id: eid,
                                space_id: space.space_id,
                                position: [other.position.x, other.position.y, other.position.z],
                                direction: [other.direction.x, other.direction.y, other.direction.z],
                                velocity: other.velocity,
                            });
                        }
                    }
                }

                // Update the witness set
                if let Some(entity) = space.entities.get_mut(&player_id) {
                    entity.witnesses = current_aoi.iter()
                        .map(|&id| EntityId(id as i32))
                        .collect();
                }
            }
        }

        events
    }

    /// Return all active spaces as (space_id, world_name) pairs.
    pub fn all_spaces(&self) -> Vec<(u32, String)> {
        self.spaces.values()
            .map(|s| (s.space_id, s.world_name.clone()))
            .collect()
    }

    /// Number of loaded world definitions.
    pub fn world_count(&self) -> usize {
        self.worlds.len()
    }

    /// Number of active space instances.
    pub fn space_count(&self) -> usize {
        self.spaces.len()
    }

    /// Look up the space_id for a world name.
    pub fn space_id_for_world(&self, world_name: &str) -> Option<u32> {
        self.world_spaces.get(world_name).copied()
    }

    /// Get a mutable reference to a cell entity by its entity ID.
    ///
    /// Searches across all spaces using the entity→space index.
    pub fn get_entity_mut(&mut self, entity_id: u32) -> Option<&mut CellEntity> {
        let &space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get_mut(&space_id)?;
        space.entities.get_mut(&entity_id)
    }

    /// Get an immutable reference to a cell entity by its entity ID.
    pub fn get_entity(&self, entity_id: u32) -> Option<&CellEntity> {
        let &space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(&space_id)?;
        space.entities.get(&entity_id)
    }

    /// Get the world name for an entity's current space.
    pub fn get_entity_world_name(&self, entity_id: u32) -> Option<String> {
        let &space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(&space_id)?;
        Some(space.world_name.clone())
    }

    /// Get the objectives for a given step from the step_objectives cache.
    ///
    /// Returns an empty vec if the step has no objectives in the cache.
    pub fn get_step_objectives(&self, step_id: i32) -> Vec<super::spawner::MissionObjectiveDef> {
        self.step_objectives
            .get(&step_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Look up a registered region by its runtime ID.
    pub fn get_region(&self, runtime_id: u32) -> Option<&RegionData> {
        self.regions.get(&runtime_id)
    }

    /// Return all registered regions for a given world name.
    pub fn regions_for_world(&self, world_name: &str) -> Vec<&RegionData> {
        self.regions.values()
            .filter(|r| r.world_name == world_name)
            .collect()
    }

    /// Collect all NPC entity IDs (class_id=0x04, not players) across all spaces.
    pub fn all_npc_entity_ids(&self) -> Vec<u32> {
        let mut ids = Vec::new();
        for space in self.spaces.values() {
            for entity in space.entities.values() {
                if !entity.is_player && entity.class_id == 0x04 {
                    ids.push(entity.entity_id.0 as u32);
                }
            }
        }
        ids
    }

    /// Collect all player entity IDs (entries in each space's `players` set)
    /// across all spaces. Returned as a `Vec` so callers can iterate without
    /// holding a borrow on `SpaceManager`.
    pub fn all_player_entity_ids(&self) -> Vec<u32> {
        let mut ids = Vec::new();
        for space in self.spaces.values() {
            ids.extend(space.players.iter().copied());
        }
        ids
    }

    /// Spawn an NPC entity in the named world at the given position.
    ///
    /// Returns the space_id the NPC was placed in.
    /// NPC entities have `is_player = false` and `class_id = 0x04` (SGWMob).
    /// They participate in AoI but don't generate AoI queries themselves.
    pub fn spawn_npc(
        &mut self,
        entity_id: u32,
        world_name: &str,
        position: [f32; 3],
        direction: [f32; 3],
    ) -> Result<u32, String> {
        let space_id = self.find_or_create_space(world_name)?;
        let pos = Vector3::new(position[0], position[1], position[2]);
        let dir = Vector3::new(direction[0], direction[1], direction[2]);

        let mut cell_entity = CellEntity::new(
            EntityId(entity_id as i32),
            SpaceId(space_id as i32),
            pos,
        );
        cell_entity.direction = dir;
        cell_entity.class_id = 0x04; // SGWMob
        cell_entity.is_player = false;
        cell_entity.spawn_position = Some(pos);
        cell_entity.abilities.add_ability(super::combat::NPC_DEFAULT_ABILITY);

        let space = self.spaces.get_mut(&space_id)
            .ok_or_else(|| format!("Space {space_id} disappeared"))?;

        space.space.add_entity(EntityId(entity_id as i32), &pos);
        space.entities.insert(entity_id, cell_entity);
        self.entity_space.insert(entity_id, space_id);

        tracing::debug!(entity_id, space_id, ?position, "NPC entity spawned");
        Ok(space_id)
    }

    /// Get the next NPC entity ID from a reserved range.
    ///
    /// NPC IDs start at 100_000 to avoid collision with player entity IDs
    /// (which are allocated sequentially from 1 by the EntityManager).
    pub fn allocate_npc_id(&mut self) -> u32 {
        let id = self.next_npc_id;
        self.next_npc_id += 1;
        id
    }

    /// Spawn an NPC entity from a database spawn record with full template data.
    ///
    /// Sets class_id, interaction flags, name, faction, alignment, and all other
    /// template-driven fields. Returns the space_id the NPC was placed in.
    ///
    /// For non-instanced worlds, uses `find_or_create_space` to resolve the space.
    /// For instanced worlds, callers should use `spawn_npc_from_record_in_space`
    /// instead to target a specific space_id.
    pub fn spawn_npc_from_record(
        &mut self,
        entity_id: u32,
        record: &super::spawner::SpawnRecord,
    ) -> Result<u32, String> {
        let space_id = self.find_or_create_space(&record.world_name)?;
        self.spawn_npc_from_record_into(entity_id, record, space_id)
    }

    /// Spawn an NPC entity from a database spawn record into a specific space.
    ///
    /// Used for instanced spaces where each player gets their own space_id.
    /// The caller is responsible for providing the correct space_id.
    pub fn spawn_npc_from_record_in_space(
        &mut self,
        entity_id: u32,
        record: &super::spawner::SpawnRecord,
        space_id: u32,
    ) -> Result<u32, String> {
        self.spawn_npc_from_record_into(entity_id, record, space_id)
    }

    /// Internal: spawn an NPC from a record into a given space_id.
    fn spawn_npc_from_record_into(
        &mut self,
        entity_id: u32,
        record: &super::spawner::SpawnRecord,
        space_id: u32,
    ) -> Result<u32, String> {
        let pos = Vector3::new(record.x, record.y, record.z);
        // heading is yaw (rotation.y), x and z rotation are 0
        let dir = Vector3::new(0.0, record.heading, 0.0);

        let mut e = CellEntity::new(
            EntityId(entity_id as i32),
            SpaceId(space_id as i32),
            pos,
        );
        e.direction = dir;
        e.class_id = super::spawner::class_id_for_class(&record.class);
        e.is_player = false;
        e.level = record.level.unwrap_or(1) as u32;
        e.npc_name = Some(record.template_name.clone());

        // Template-driven fields
        e.template_id = Some(record.template_id);
        e.tag = record.tag.clone();
        e.name_id = record.name_id;
        e.speaker_id = record.speaker_id;
        e.event_set_id = record.event_set_id;
        e.interaction_type_flags = record.interaction_type;
        e.entity_flags = record.flags as u64;
        e.faction = record.faction.unwrap_or(0) as u8;
        e.alignment = record.alignment.unwrap_or(0) as u8;
        e.static_interaction_sets = record.static_interaction_sets.clone();
        e.has_dynamic_properties = record.has_dynamic_properties;
        e.static_mesh = record.static_mesh.clone();
        e.body_set = Some(record.body_set.clone());
        e.components = record.components.clone().unwrap_or_default();
        e.spawn_position = Some(pos);
        e.loot_table_id = record.loot_table_id;

        // Give NPCs a default combat ability and stats so they can fight back
        e.abilities.add_ability(super::combat::NPC_DEFAULT_ABILITY);
        // Initialize NPC health based on level (simple scaling)
        use cimmeria_entity::stats::{HEALTH, FOCUS};
        let hp = 200 + (e.level as i32 * 50);
        if let Some(stat) = e.stats.get_mut(HEALTH) {
            stat.max = hp;
            stat.set_current(hp);
        }
        if let Some(stat) = e.stats.get_mut(FOCUS) {
            stat.max = 200;
            stat.set_current(200);
        }

        let space = self.spaces.get_mut(&space_id)
            .ok_or_else(|| format!("Space {space_id} disappeared"))?;

        space.space.add_entity(EntityId(entity_id as i32), &pos);
        space.entities.insert(entity_id, e);
        self.entity_space.insert(entity_id, space_id);

        Ok(space_id)
    }

    /// Return all player entity IDs that currently have `target_entity_id` in their AoI.
    ///
    /// Used for broadcasting property updates (InteractionType, SetVisible, etc.)
    /// to players who can see the entity.
    pub fn get_witnesses_of(&self, target_entity_id: u32) -> Vec<u32> {
        let Some(&space_id) = self.entity_space.get(&target_entity_id) else {
            return vec![];
        };
        let Some(space) = self.spaces.get(&space_id) else {
            return vec![];
        };
        let target_eid = EntityId(target_entity_id as i32);
        space.players.iter().filter(|&&pid| {
            space.entities.get(&pid)
                .map_or(false, |p| p.witnesses.contains(&target_eid))
        }).copied().collect()
    }

    /// Find an NPC entity by its spawn tag within a specific world.
    ///
    /// Used by content chain actions (SetInteractionType, DestroyTaggedEntity, etc.)
    /// to locate entities by their `spawnlist.tag` value.
    ///
    /// Searches all spaces matching `world_name` — works for both non-instanced
    /// (one space in `world_spaces`) and instanced (multiple spaces) worlds.
    pub fn find_entity_by_tag(&self, world_name: &str, tag: &str) -> Option<u32> {
        for space in self.spaces.values() {
            if space.world_name != world_name {
                continue;
            }
            for (&eid, entity) in &space.entities {
                if entity.tag.as_deref() == Some(tag) {
                    return Some(eid);
                }
            }
        }
        None
    }

    /// Find all entities with a given `template_id` in a specific world.
    ///
    /// Used by `add_dialog_set` to locate NPC entities that match the slot
    /// (template_id) so per-player InteractionType updates can be sent.
    ///
    /// Searches all spaces matching `world_name` — works for both non-instanced
    /// and instanced worlds.
    pub fn find_entities_by_template(&self, world_name: &str, template_id: i32) -> Vec<u32> {
        let mut results = Vec::new();
        for space in self.spaces.values() {
            if space.world_name != world_name {
                continue;
            }
            for (&eid, e) in &space.entities {
                if e.template_id == Some(template_id) {
                    results.push(eid);
                }
            }
        }
        results
    }

    // ── NavMesh queries ──────────────────────────────────────────────────

    /// Line-of-sight check between two entities using the navmesh.
    /// Returns `true` if there is clear LoS (or if no navmesh is loaded).
    pub fn has_line_of_sight(&self, entity_a: u32, entity_b: u32) -> bool {
        let space_id = match self.entity_space.get(&entity_a) {
            Some(&sid) => sid,
            None => return true, // No space info — assume LoS
        };
        let space = match self.spaces.get(&space_id) {
            Some(s) => s,
            None => return true,
        };
        let navmesh = match &space.navmesh {
            Some(nm) => nm,
            None => return true, // No navmesh — can't check, assume LoS
        };
        let pos_a = match space.entities.get(&entity_a) {
            Some(e) => e.position,
            None => return true,
        };
        let pos_b = match space.entities.get(&entity_b) {
            Some(e) => e.position,
            None => return true,
        };
        navmesh.raycast(&pos_a, &pos_b)
    }

    /// Find a path between two positions within the space containing `entity_id`.
    /// Returns waypoints or `None` if no path exists or no navmesh is loaded.
    pub fn find_path(&self, entity_id: u32, start: &Vector3, end: &Vector3) -> Option<Vec<Vector3>> {
        let space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(space_id)?;
        let navmesh = space.navmesh.as_ref()?;
        navmesh.find_path(start, end)
    }

    /// Check if a position is on walkable navmesh in the space containing `entity_id`.
    pub fn is_position_valid(&self, entity_id: u32, pos: &Vector3) -> bool {
        let space_id = match self.entity_space.get(&entity_id) {
            Some(&sid) => sid,
            None => return true,
        };
        let space = match self.spaces.get(&space_id) {
            Some(s) => s,
            None => return true,
        };
        match &space.navmesh {
            Some(nm) => nm.is_point_valid(pos),
            None => true,
        }
    }

    /// Sample the navmesh surface height at (x, z) in the space containing `entity_id`.
    /// Returns `None` if no navmesh is loaded or the point is off-mesh.
    pub fn get_navmesh_height(&self, entity_id: u32, x: f32, z: f32) -> Option<f32> {
        let space_id = self.entity_space.get(&entity_id)?;
        let space = self.spaces.get(space_id)?;
        let navmesh = space.navmesh.as_ref()?;
        navmesh.get_height_at(x, z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SPACES_XML: &str = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
    <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="2400" MinY="0" MaxY="2400" />
    <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    <Space WorldName="SGC_W1" Instanced="true" MinX="-400" MaxX="400" MinY="-400" MaxY="800" />
</Spaces>"#;

    const TEST_CELL_SPACES_XML: &str = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" />
    <Space WorldName="Castle" />
</Spaces>"#;

    fn make_manager() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(TEST_SPACES_XML).unwrap();
        mgr.create_startup_spaces(TEST_CELL_SPACES_XML).unwrap();
        mgr
    }

    #[test]
    fn parse_spaces_xml_loads_all_worlds() {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(TEST_SPACES_XML).unwrap();
        assert_eq!(mgr.world_count(), 4);
        assert!(mgr.worlds.contains_key("Agnos"));
        assert!(mgr.worlds.contains_key("Castle_CellBlock"));
        assert!(mgr.worlds["Castle_CellBlock"].instanced);
        assert!(!mgr.worlds["Agnos"].instanced);
    }

    #[test]
    fn startup_spaces_get_correct_ids() {
        let mgr = make_manager();
        assert_eq!(mgr.space_count(), 2);
        // cell_id=1: first space = (1<<16)|0 = 65536, second = 65537
        assert_eq!(mgr.space_id_for_world("Agnos"), Some(65536));
        assert_eq!(mgr.space_id_for_world("Castle"), Some(65537));
    }

    #[test]
    fn instanced_space_created_on_demand() {
        let mut mgr = make_manager();
        assert_eq!(mgr.space_id_for_world("Castle_CellBlock"), None);

        let id1 = mgr.find_or_create_space("Castle_CellBlock").unwrap();
        assert_eq!(id1, 65538); // next after 65536, 65537

        // Each call creates a NEW instance — they should NOT share a space
        let id2 = mgr.find_or_create_space("Castle_CellBlock").unwrap();
        assert_eq!(id2, 65539);
        assert_ne!(id1, id2);

        // Instanced spaces are NOT cached in world_spaces
        assert_eq!(mgr.space_id_for_world("Castle_CellBlock"), None);
    }

    #[test]
    fn unknown_world_returns_error() {
        let mut mgr = make_manager();
        assert!(mgr.find_or_create_space("Narnia").is_err());
    }

    #[test]
    fn create_entity_in_startup_space() {
        let mut mgr = make_manager();
        let space_id = mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3]).unwrap();
        assert_eq!(space_id, 65536);
        assert!(mgr.spaces[&65536].entities.contains_key(&100));
    }

    #[test]
    fn create_entity_in_instanced_space() {
        let mut mgr = make_manager();
        let space_id = mgr.create_entity(200, "SGC_W1", [5.0, 0.0, 5.0], [0.0; 3]).unwrap();
        assert_eq!(space_id, 65538);
        assert!(mgr.spaces[&65538].entities.contains_key(&200));
    }

    #[test]
    fn destroy_entity_removes_from_space() {
        let mut mgr = make_manager();
        mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3]).unwrap();
        mgr.destroy_entity(100);
        assert!(!mgr.spaces[&65536].entities.contains_key(&100));
        assert!(!mgr.entity_space.contains_key(&100));
    }

    #[test]
    fn connect_entity_marks_as_player() {
        let mut mgr = make_manager();
        mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3]).unwrap();
        mgr.connect_entity(100);
        assert!(mgr.spaces[&65536].players.contains(&100));
    }

    #[test]
    fn update_entity_position() {
        let mut mgr = make_manager();
        mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3]).unwrap();
        mgr.update_entity_position(100, [50.0, 5.0, 60.0], [0, 0, 0], [0.0; 3]);
        let entity = &mgr.spaces[&65536].entities[&100];
        assert_eq!(entity.position, Vector3::new(50.0, 5.0, 60.0));
    }

    #[test]
    fn aoi_detects_nearby_players() {
        let mut mgr = make_manager();
        mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();
        mgr.create_entity(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3]).unwrap();
        mgr.connect_entity(100);
        mgr.connect_entity(200);

        let events = mgr.compute_aoi_changes();

        // Both players should see each other enter AoI
        let entered: Vec<_> = events.iter().filter(|e| matches!(e, CellToBaseMsg::EnteredAoI { .. })).collect();
        assert_eq!(entered.len(), 2);
    }

    #[test]
    fn aoi_detects_entity_leaving() {
        let mut mgr = make_manager();
        mgr.create_entity(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();
        mgr.create_entity(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3]).unwrap();
        mgr.connect_entity(100);
        mgr.connect_entity(200);

        // First tick: both enter AoI
        let _ = mgr.compute_aoi_changes();

        // Move entity 200 far away
        mgr.update_entity_position(200, [5000.0, 0.0, 5000.0], [0, 0, 0], [0.0; 3]);

        // Second tick: entity 200 should leave AoI of entity 100
        let events = mgr.compute_aoi_changes();
        let left: Vec<_> = events.iter().filter(|e| matches!(e, CellToBaseMsg::LeftAoI { .. })).collect();
        assert_eq!(left.len(), 2); // Both should lose sight of each other
    }

    #[test]
    fn space_id_scheme() {
        let mut mgr = SpaceManager::new(1);
        assert_eq!(mgr.allocate_space_id(), 65536); // (1 << 16) | 0
        assert_eq!(mgr.allocate_space_id(), 65537); // (1 << 16) | 1
        assert_eq!(mgr.allocate_space_id(), 65538); // (1 << 16) | 2
    }

    #[test]
    fn full_xml_file_loading() {
        // Test with the actual XML content (same structure as files)
        let mut mgr = SpaceManager::new(1);

        let spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
    <Space WorldName="Agnos_Library" Instanced="false" MinX="-600" MaxX="600" MinY="-600" MaxY="600" />
    <Space WorldName="Beta_Site_Evo_1" Instanced="false" MinX="-1600" MaxX="2600" MinY="-3000" MaxY="3000" />
    <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="2400" MinY="0" MaxY="2400" />
    <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
    <Space WorldName="SGC_W1" Instanced="true" MinX="-400" MaxX="400" MinY="-400" MaxY="800" />
</Spaces>"#;

        let cell_spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" />
    <Space WorldName="Agnos_Library" />
    <Space WorldName="Beta_Site_Evo_1" />
    <Space WorldName="Castle" />
</Spaces>"#;

        mgr.parse_spaces_xml(spaces_xml).unwrap();
        assert_eq!(mgr.world_count(), 6);

        mgr.create_startup_spaces(cell_spaces_xml).unwrap();
        assert_eq!(mgr.space_count(), 4);

        // Startup spaces get sequential IDs
        assert_eq!(mgr.space_id_for_world("Agnos"), Some(65536));
        assert_eq!(mgr.space_id_for_world("Agnos_Library"), Some(65537));
        assert_eq!(mgr.space_id_for_world("Beta_Site_Evo_1"), Some(65538));
        assert_eq!(mgr.space_id_for_world("Castle"), Some(65539));

        // Instanced worlds not yet created
        assert_eq!(mgr.space_id_for_world("Castle_CellBlock"), None);
        assert_eq!(mgr.space_id_for_world("SGC_W1"), None);
    }

    // ── NPC spawn and AI ─────────────────────────────────────────────────

    #[test]
    fn spawn_npc_sets_class_id_and_spawn_position() {
        let mut mgr = make_manager();
        let pos = [50.0, 0.0, 75.0];
        mgr.spawn_npc(500, "Agnos", pos, [0.0; 3]).unwrap();

        let npc = mgr.get_entity(500).unwrap();
        assert_eq!(npc.class_id, 0x04); // SGWMob
        assert!(!npc.is_player);
        assert_eq!(npc.spawn_position.unwrap(), Vector3::new(50.0, 0.0, 75.0));
    }

    #[test]
    fn spawn_npc_gets_default_ability() {
        let mut mgr = make_manager();
        mgr.spawn_npc(500, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

        let npc = mgr.get_entity(500).unwrap();
        assert!(npc.abilities.has_ability(crate::cell::combat::NPC_DEFAULT_ABILITY));
    }

    #[test]
    fn all_npc_entity_ids_returns_only_npcs() {
        let mut mgr = make_manager();
        // Add a player entity
        mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
        mgr.connect_entity(1);
        // Add two NPC entities
        mgr.spawn_npc(100, "Agnos", [10.0, 0.0, 10.0], [0.0; 3]).unwrap();
        mgr.spawn_npc(200, "Agnos", [20.0, 0.0, 20.0], [0.0; 3]).unwrap();

        let npc_ids = mgr.all_npc_entity_ids();
        assert_eq!(npc_ids.len(), 2);
        assert!(npc_ids.contains(&100));
        assert!(npc_ids.contains(&200));
        // Player should NOT be in the list
        assert!(!npc_ids.contains(&1));
    }

    #[test]
    fn all_npc_entity_ids_empty_when_no_npcs() {
        let mgr = make_manager();
        assert!(mgr.all_npc_entity_ids().is_empty());
    }

    #[test]
    fn get_entity_world_name_for_npc() {
        let mut mgr = make_manager();
        mgr.spawn_npc(500, "Agnos", [0.0; 3], [0.0; 3]).unwrap();

        assert_eq!(mgr.get_entity_world_name(500), Some("Agnos".to_string()));
    }

    #[test]
    fn spawn_npc_from_record_sets_template_fields() {
        use crate::cell::spawner::SpawnRecord;
        let mut mgr = make_manager();
        let record = SpawnRecord {
            spawn_id: 1,
            world_name: "Agnos".to_string(),
            x: 10.0, y: 0.0, z: 20.0, heading: 1.5,
            class: "SGWMob".to_string(),
            template_id: 42,
            template_name: "TestGuard".to_string(),
            tag: Some("Guard01".to_string()),
            name_id: Some(1001),
            speaker_id: None,
            event_set_id: None,
            interaction_type: 0,
            flags: 0,
            faction: Some(10),
            alignment: Some(1),
            level: Some(5),
            static_interaction_sets: vec![],
            has_dynamic_properties: false,
            static_mesh: None,
            body_set: "BS_NID_Soldier.BS_NID_Soldier".to_string(),
            components: Some(vec!["Comp1".to_string()]),
            loot_table_id: Some(2),
        };

        mgr.spawn_npc_from_record(600, &record).unwrap();

        let npc = mgr.get_entity(600).unwrap();
        assert_eq!(npc.template_id, Some(42));
        assert_eq!(npc.tag.as_deref(), Some("Guard01"));
        assert_eq!(npc.faction, 10);
        assert_eq!(npc.alignment, 1);
        assert_eq!(npc.level, 5);
        assert_eq!(npc.spawn_position.unwrap(), Vector3::new(10.0, 0.0, 20.0));
        // 592 = NPC_DEFAULT_ABILITY (Pistol Shot — was previously 597/Heal Focus).
        assert!(npc.abilities.has_ability(592));
        // Health should be scaled: 200 + (5 * 50) = 450
        assert_eq!(npc.stats.get(cimmeria_entity::stats::HEALTH).unwrap().max, 450);
    }

    #[test]
    fn instanced_space_destroyed_when_last_player_leaves() {
        let mut mgr = make_manager();

        // Create a player entity in an instanced space
        let space_id = mgr.create_entity(200, "Castle_CellBlock", [5.0, 0.0, 5.0], [0.0; 3]).unwrap();
        mgr.connect_entity(200);

        // Manually add an NPC into the same instanced space (simulates what
        // spawn_instance_npcs_from_records does with a specific space_id)
        let npc_pos = Vector3::new(10.0, 0.0, 10.0);
        let mut npc = CellEntity::new(
            EntityId(100_000),
            SpaceId(space_id as i32),
            npc_pos,
        );
        npc.class_id = 0x04;
        npc.is_player = false;
        let space = mgr.spaces.get_mut(&space_id).unwrap();
        space.space.add_entity(EntityId(100_000), &npc_pos);
        space.entities.insert(100_000, npc);
        mgr.entity_space.insert(100_000, space_id);

        assert!(mgr.spaces.contains_key(&space_id));
        assert_eq!(mgr.spaces[&space_id].entities.len(), 2); // player + NPC

        // Destroy the player — last player in the instance, should destroy the space
        mgr.destroy_entity(200);

        // Space should be fully cleaned up
        assert!(!mgr.spaces.contains_key(&space_id));
        assert!(!mgr.entity_space.contains_key(&200));
        assert!(!mgr.entity_space.contains_key(&100_000)); // NPC also cleaned up
    }

    #[test]
    fn non_instanced_space_survives_player_leaving() {
        let mut mgr = make_manager();

        // Create a player in a non-instanced startup space
        mgr.create_entity(100, "Agnos", [10.0, 0.0, 20.0], [0.0; 3]).unwrap();
        mgr.connect_entity(100);

        // Destroy the player — non-instanced space should NOT be destroyed
        mgr.destroy_entity(100);

        assert!(mgr.spaces.contains_key(&65536)); // Agnos space still exists
    }

    #[test]
    fn two_players_get_separate_instances() {
        let mut mgr = make_manager();

        let space1 = mgr.create_entity(100, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();
        let space2 = mgr.create_entity(200, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        assert_ne!(space1, space2);
        assert!(mgr.spaces.contains_key(&space1));
        assert!(mgr.spaces.contains_key(&space2));

        // Each space has exactly one entity
        assert_eq!(mgr.spaces[&space1].entities.len(), 1);
        assert_eq!(mgr.spaces[&space2].entities.len(), 1);
    }
}
