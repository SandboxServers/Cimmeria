//! Cell-side entity representation.
//!
//! A `CellEntity` lives on the CellApp and owns the spatial state of a game
//! entity: its position, direction, area-of-interest radius, and the set of
//! other entities that can currently "see" it (witnesses). Property changes
//! on the cell entity may be broadcast to witnesses depending on distribution
//! flags.
//!
//! This corresponds to the C++ `CellEntity` / `Entity` classes in
//! `src/server/CellApp/`.

use std::collections::{HashMap, HashSet};

use cimmeria_common::{EntityId, SpaceId, Vector3};

use crate::base_entity::PropertyValue;

/// The cell-side half of a game entity.
///
/// Manages spatial position, orientation, area-of-interest, and the witness
/// set. Created when a base entity requests a cell presence via
/// `BaseApp::createCellEntity`.
pub struct CellEntity {
    /// Runtime entity ID (matches the corresponding `BaseEntity`).
    pub entity_id: EntityId,

    /// The space (world/zone) this entity currently inhabits.
    pub space_id: SpaceId,

    /// World-space position.
    pub position: Vector3,

    /// Facing direction (unit vector or yaw/pitch/roll encoded).
    pub direction: Vector3,

    /// Whether the entity is currently on the ground (affects movement mode).
    pub is_on_ground: bool,

    /// Cell-local property values (CELL_PUBLIC, CELL_PRIVATE, etc.).
    pub properties: HashMap<String, PropertyValue>,

    /// Set of entity IDs that currently have this entity in their AoI.
    ///
    /// When a property with `OTHER_CLIENTS` or `ALL_CLIENTS` distribution
    /// changes, updates are sent to all witnesses.
    pub witnesses: HashSet<EntityId>,

    /// Area-of-interest radius in world units. Other entities within this
    /// radius may become witnesses.
    pub aoi_radius: f32,
}

impl CellEntity {
    /// Create a new cell entity at the given position in the given space.
    pub fn new(entity_id: EntityId, space_id: SpaceId, position: Vector3) -> Self {
        Self {
            entity_id,
            space_id,
            position,
            direction: Vector3::zero(),
            is_on_ground: true,
            properties: HashMap::new(),
            witnesses: HashSet::new(),
            aoi_radius: 100.0, // Default AoI radius (matches grid_vision_distance)
        }
    }

    /// Update the entity's world-space position.
    pub fn set_position(&mut self, position: Vector3) {
        self.position = position;
    }

    /// Get the entity's current world-space position.
    pub fn get_position(&self) -> &Vector3 {
        &self.position
    }

    /// Add an entity to the witness set (it can now see this entity).
    pub fn add_witness(&mut self, entity_id: EntityId) {
        self.witnesses.insert(entity_id);
    }

    /// Remove an entity from the witness set (it can no longer see this entity).
    pub fn remove_witness(&mut self, entity_id: EntityId) {
        self.witnesses.remove(&entity_id);
    }

    /// Get the current set of witness entity IDs.
    pub fn get_witnesses(&self) -> &HashSet<EntityId> {
        &self.witnesses
    }

    /// Returns `true` if the given position is within this entity's AoI radius.
    ///
    /// Uses squared distance comparison to avoid a square root.
    pub fn is_in_aoi(&self, other_pos: &Vector3) -> bool {
        self.position.distance_squared_to(other_pos) <= self.aoi_radius * self.aoi_radius
    }
}

impl std::fmt::Debug for CellEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CellEntity")
            .field("entity_id", &self.entity_id)
            .field("space_id", &self.space_id)
            .field("position", &self.position)
            .field("is_on_ground", &self.is_on_ground)
            .field("witness_count", &self.witnesses.len())
            .field("aoi_radius", &self.aoi_radius)
            .field("property_count", &self.properties.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity() -> CellEntity {
        CellEntity::new(EntityId(1), SpaceId(100), Vector3::new(10.0, 0.0, 20.0))
    }

    #[test]
    fn new_entity_defaults() {
        let entity = make_entity();
        assert_eq!(entity.entity_id, EntityId(1));
        assert_eq!(entity.space_id, SpaceId(100));
        assert_eq!(entity.position, Vector3::new(10.0, 0.0, 20.0));
        assert_eq!(entity.direction, Vector3::zero());
        assert!(entity.is_on_ground);
        assert!(entity.witnesses.is_empty());
        assert_eq!(entity.aoi_radius, 100.0);
    }

    #[test]
    fn set_and_get_position() {
        let mut entity = make_entity();
        let new_pos = Vector3::new(50.0, 5.0, 60.0);
        entity.set_position(new_pos);
        assert_eq!(*entity.get_position(), new_pos);
    }

    #[test]
    fn add_and_remove_witness() {
        let mut entity = make_entity();
        entity.add_witness(EntityId(2));
        entity.add_witness(EntityId(3));
        assert_eq!(entity.get_witnesses().len(), 2);
        assert!(entity.get_witnesses().contains(&EntityId(2)));

        entity.remove_witness(EntityId(2));
        assert_eq!(entity.get_witnesses().len(), 1);
        assert!(!entity.get_witnesses().contains(&EntityId(2)));
    }

    #[test]
    fn duplicate_witness_is_idempotent() {
        let mut entity = make_entity();
        entity.add_witness(EntityId(2));
        entity.add_witness(EntityId(2));
        assert_eq!(entity.get_witnesses().len(), 1);
    }

    #[test]
    fn remove_absent_witness_is_noop() {
        let mut entity = make_entity();
        entity.remove_witness(EntityId(99)); // no panic
        assert!(entity.get_witnesses().is_empty());
    }

    #[test]
    fn is_in_aoi_within_radius() {
        let entity = make_entity(); // pos = (10, 0, 20), radius = 100
        let nearby = Vector3::new(20.0, 0.0, 25.0);
        assert!(entity.is_in_aoi(&nearby));
    }

    #[test]
    fn is_in_aoi_outside_radius() {
        let entity = make_entity(); // pos = (10, 0, 20), radius = 100
        let far_away = Vector3::new(500.0, 0.0, 500.0);
        assert!(!entity.is_in_aoi(&far_away));
    }

    #[test]
    fn is_in_aoi_at_exact_boundary() {
        let mut entity = make_entity();
        entity.aoi_radius = 10.0;
        // Point exactly 10 units away on the X axis
        let boundary = Vector3::new(20.0, 0.0, 20.0);
        assert!(entity.is_in_aoi(&boundary));
    }
}
