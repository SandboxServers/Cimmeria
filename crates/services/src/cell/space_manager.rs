//! Space management for the CellService.
//!
//! Loads world definitions from `entities/spaces.xml` and creates startup
//! spaces from `entities/cell_spaces.xml`. Manages the lifecycle of space
//! instances and the cell entities within them.

use std::collections::{HashMap, HashSet};

use cimmeria_common::{EntityId, SpaceId, Vector3};
use cimmeria_entity::cell_entity::CellEntity;
use cimmeria_entity::space::Space;

use super::messages::CellToBaseMsg;

/// Grid cell size for spatial hashing (world units).
const GRID_CELL_SIZE: f32 = 50.0;

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
        let instance = SpaceInstance {
            space_id,
            world_name: world_name.to_string(),
            space,
            entities: HashMap::new(),
            players: HashSet::new(),
        };
        tracing::debug!(space_id, world = %world_name, "Created space instance");
        self.spaces.insert(space_id, instance);
    }

    /// Check if a space already exists for a world name.
    pub fn has_space_for_world(&self, world_name: &str) -> bool {
        self.world_spaces.contains_key(world_name)
    }

    /// Find or create a space for the given world name.
    ///
    /// - Non-instanced worlds: return the existing startup space.
    /// - Instanced worlds: return existing shared instance, or create a new one.
    pub fn find_or_create_space(&mut self, world_name: &str) -> Result<u32, String> {
        // Check if there's already a space for this world
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

        // Create a new instance for this instanced world
        let space_id = self.allocate_space_id();
        self.create_space_instance(space_id, world_name);
        self.world_spaces.insert(world_name.to_string(), space_id);

        tracing::info!(space_id, world = %world_name, "Created instanced space on demand");
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
    pub fn destroy_entity(&mut self, entity_id: u32) {
        if let Some(space_id) = self.entity_space.remove(&entity_id) {
            if let Some(space) = self.spaces.get_mut(&space_id) {
                if let Some(cell_entity) = space.entities.remove(&entity_id) {
                    space.space.remove_entity(
                        EntityId(entity_id as i32),
                        &cell_entity.position,
                    );
                }
                space.players.remove(&entity_id);
            }
        }
        tracing::debug!(entity_id, "Cell entity destroyed");
    }

    /// Mark an entity as having a client controller (player).
    pub fn connect_entity(&mut self, entity_id: u32) {
        if let Some(&space_id) = self.entity_space.get(&entity_id) {
            if let Some(space) = self.spaces.get_mut(&space_id) {
                space.players.insert(entity_id);
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
        _velocity: [f32; 3],
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
                let (player_pos, aoi_radius) = match space.entities.get(&player_id) {
                    Some(e) => (e.position, e.aoi_radius),
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
                            events.push(CellToBaseMsg::EnteredAoI {
                                witness_id: player_id,
                                entity_id: eid,
                                space_id: space.space_id,
                                class_id: other.class_id,
                                position: [other.position.x, other.position.y, other.position.z],
                                direction: [other.direction.x, other.direction.y, other.direction.z],
                                level: other.level,
                            });
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
                                velocity: [0.0; 3], // TODO: track velocity
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

        let id = mgr.find_or_create_space("Castle_CellBlock").unwrap();
        assert_eq!(id, 65538); // next after 65536, 65537

        // Second call reuses the same space
        let id2 = mgr.find_or_create_space("Castle_CellBlock").unwrap();
        assert_eq!(id, id2);
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
}
