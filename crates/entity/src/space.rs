//! Space (world/zone) management.
//!
//! A `Space` represents a single game world instance -- a zone, map, or
//! instanced area. It tracks all entities currently present in the space
//! and owns a [`WorldGrid`] for spatial queries.
//!
//! In the original BigWorld engine, spaces are created by the CellApp and
//! correspond to entries in `entities/cell_spaces.xml`. Entity positions
//! are meaningful only within the coordinate system of their containing space.

use std::collections::HashSet;

use tracing::{debug, warn};

use cimmeria_common::{EntityId, SpaceId, Vector3};

use crate::world_grid::WorldGrid;

/// A game world instance containing entities and a spatial index.
pub struct Space {
    /// Unique identifier for this space.
    pub space_id: SpaceId,

    /// Human-readable name (e.g. "SGC", "P3X-888", "Chulak").
    pub name: String,

    /// Set of all entity IDs currently present in this space.
    entities: HashSet<EntityId>,

    /// Spatial partitioning grid for proximity queries.
    pub grid: WorldGrid,
}

impl Space {
    /// Create a new space with the given ID, name, and grid cell size.
    ///
    /// The grid cell size should match the `grid_chunk_size` configuration
    /// for this space type (typically 50.0-100.0 world units).
    pub fn new(space_id: SpaceId, name: String, grid_cell_size: f32) -> Self {
        debug!("Creating space {} ('{}')", space_id, name);
        Self {
            space_id,
            name,
            entities: HashSet::new(),
            grid: WorldGrid::new(grid_cell_size),
        }
    }

    /// Add an entity to this space at the given position.
    ///
    /// The entity is inserted into both the entity set and the spatial grid.
    pub fn add_entity(&mut self, entity_id: EntityId, position: &Vector3) {
        if self.entities.insert(entity_id) {
            self.grid.insert(entity_id, position);
            debug!("Entity {} entered space {}", entity_id, self.space_id);
        } else {
            warn!(
                "Entity {} is already in space {}",
                entity_id, self.space_id
            );
        }
    }

    /// Remove an entity from this space.
    ///
    /// `position` must be the entity's last known position so it can be
    /// removed from the correct grid cell.
    pub fn remove_entity(&mut self, entity_id: EntityId, position: &Vector3) {
        if self.entities.remove(&entity_id) {
            self.grid.remove(entity_id, position);
            debug!("Entity {} left space {}", entity_id, self.space_id);
        } else {
            warn!(
                "Attempted to remove entity {} from space {} but it was not present",
                entity_id, self.space_id
            );
        }
    }

    /// Find all entities within `radius` world units of `pos`.
    ///
    /// Delegates to the underlying [`WorldGrid`] for candidate selection.
    /// Results may include entities slightly outside the radius due to
    /// cell-granularity -- callers should apply a secondary exact-distance
    /// filter if precision is required.
    pub fn get_entities_in_range(&self, pos: &Vector3, radius: f32) -> Vec<EntityId> {
        self.grid.query_radius(pos, radius)
    }

    /// Return the number of entities currently in this space.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Returns `true` if the given entity is present in this space.
    pub fn contains_entity(&self, entity_id: EntityId) -> bool {
        self.entities.contains(&entity_id)
    }
}

impl std::fmt::Debug for Space {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Space")
            .field("space_id", &self.space_id)
            .field("name", &self.name)
            .field("entity_count", &self.entities.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_space() -> Space {
        Space::new(SpaceId(1), "TestWorld".to_string(), 50.0)
    }

    #[test]
    fn new_space_is_empty() {
        let space = make_space();
        assert_eq!(space.entity_count(), 0);
        assert_eq!(space.space_id, SpaceId(1));
        assert_eq!(space.name, "TestWorld");
    }

    #[test]
    fn add_and_count_entities() {
        let mut space = make_space();
        space.add_entity(EntityId(1), &Vector3::new(10.0, 0.0, 20.0));
        space.add_entity(EntityId(2), &Vector3::new(30.0, 0.0, 40.0));
        assert_eq!(space.entity_count(), 2);
    }

    #[test]
    fn contains_entity() {
        let mut space = make_space();
        space.add_entity(EntityId(1), &Vector3::new(0.0, 0.0, 0.0));
        assert!(space.contains_entity(EntityId(1)));
        assert!(!space.contains_entity(EntityId(2)));
    }

    #[test]
    fn remove_entity() {
        let mut space = make_space();
        let pos = Vector3::new(10.0, 0.0, 20.0);
        space.add_entity(EntityId(1), &pos);
        assert_eq!(space.entity_count(), 1);

        space.remove_entity(EntityId(1), &pos);
        assert_eq!(space.entity_count(), 0);
        assert!(!space.contains_entity(EntityId(1)));
    }

    #[test]
    fn duplicate_add_is_idempotent() {
        let mut space = make_space();
        let pos = Vector3::new(10.0, 0.0, 20.0);
        space.add_entity(EntityId(1), &pos);
        space.add_entity(EntityId(1), &pos); // duplicate, should warn but not double-count
        assert_eq!(space.entity_count(), 1);
    }

    #[test]
    fn remove_absent_entity_is_safe() {
        let mut space = make_space();
        space.remove_entity(EntityId(99), &Vector3::zero()); // should not panic
    }

    #[test]
    fn get_entities_in_range() {
        let mut space = make_space();
        space.add_entity(EntityId(1), &Vector3::new(10.0, 0.0, 10.0));
        space.add_entity(EntityId(2), &Vector3::new(15.0, 0.0, 15.0));
        space.add_entity(EntityId(3), &Vector3::new(500.0, 0.0, 500.0));

        let nearby = space.get_entities_in_range(&Vector3::new(10.0, 0.0, 10.0), 60.0);
        assert!(nearby.contains(&EntityId(1)));
        assert!(nearby.contains(&EntityId(2)));
        assert!(!nearby.contains(&EntityId(3)));
    }
}
