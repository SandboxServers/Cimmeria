//! Space lifecycle: ID allocation, creation, lookup, destruction.

use std::collections::{HashMap, HashSet};

use cimmeria_common::SpaceId;
use cimmeria_entity::navigation::NavMesh;
use cimmeria_entity::space::Space;

use super::{SpaceInstance, SpaceManager, GRID_CELL_SIZE};

impl SpaceManager {
    /// Allocate the next space ID using the `(cell_id << 16) | local_index` scheme.
    pub(crate) fn allocate_space_id(&mut self) -> u32 {
        let id = ((self.cell_id as u32) << 16) | self.next_local_id;
        self.next_local_id += 1;
        id
    }

    /// Create a `SpaceInstance` and insert it into the spaces map.
    pub(crate) fn create_space_instance(&mut self, space_id: u32, world_name: &str) {
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

    /// Destroy an instanced space, removing all entities and freeing resources.
    ///
    /// Only called for instanced spaces when the last player leaves. Removes
    /// all NPC entities, the space instance, and any entity_space entries.
    pub(crate) fn destroy_space(&mut self, space_id: u32) {
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
}
