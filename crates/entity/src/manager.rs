//! Entity pool and lifecycle management.
//!
//! The `EntityManager` owns all base entities in the current process, assigns
//! entity IDs, and provides O(1) lookup by ID. It implements an ID recycling
//! strategy: destroyed entity IDs are placed into a free list and reused
//! before the monotonic counter is advanced.
//!
//! This corresponds to the entity management portions of the C++ `BaseApp`
//! and `CellApp` entity tables.

use std::collections::{HashMap, VecDeque};

use tracing::{debug, warn};

use cimmeria_common::EntityId;

use crate::base_entity::BaseEntity;

/// Central registry for all base entities in the current service process.
pub struct EntityManager {
    /// Active entities keyed by their runtime ID.
    entities: HashMap<EntityId, BaseEntity>,

    /// Monotonically increasing counter for new entity IDs.
    next_id: i32,

    /// Previously-used IDs that have been freed and can be recycled.
    free_ids: VecDeque<EntityId>,
}

impl EntityManager {
    /// Create an empty entity manager.
    ///
    /// Entity IDs start at 1 (ID 0 is reserved as "no entity").
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_id: 1,
            free_ids: VecDeque::new(),
        }
    }

    /// Allocate the next available entity ID.
    ///
    /// Prefers recycling IDs from the free list; falls back to the monotonic
    /// counter when the free list is empty.
    pub fn allocate_id(&mut self) -> EntityId {
        if let Some(id) = self.free_ids.pop_front() {
            debug!("Recycling entity ID {}", id);
            id
        } else {
            let id = EntityId(self.next_id);
            self.next_id += 1;
            id
        }
    }

    /// Create a new entity of the given type and return its assigned ID.
    ///
    /// The entity is inserted into the manager's entity table and can be
    /// retrieved via [`get_entity`](Self::get_entity).
    pub fn create_entity(&mut self, entity_type: &str) -> EntityId {
        let id = self.allocate_id();
        let entity = BaseEntity::new(id, entity_type.to_string());
        debug!("Created entity {} of type '{}'", id, entity_type);
        self.entities.insert(id, entity);
        id
    }

    /// Destroy an entity and return its ID to the free list.
    ///
    /// If the entity does not exist, a warning is logged and no action is taken.
    pub fn destroy_entity(&mut self, id: EntityId) {
        if self.entities.remove(&id).is_some() {
            debug!("Destroyed entity {}", id);
            self.free_ids.push_back(id);
        } else {
            warn!("Attempted to destroy non-existent entity {}", id);
        }
    }

    /// Get an immutable reference to an entity by ID.
    pub fn get_entity(&self, id: EntityId) -> Option<&BaseEntity> {
        self.entities.get(&id)
    }

    /// Get a mutable reference to an entity by ID.
    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut BaseEntity> {
        self.entities.get_mut(&id)
    }

    /// Return the number of currently active entities.
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
}

impl Default for EntityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for EntityManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityManager")
            .field("entity_count", &self.entities.len())
            .field("next_id", &self.next_id)
            .field("free_ids", &self.free_ids.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_manager_is_empty() {
        let mgr = EntityManager::new();
        assert_eq!(mgr.entity_count(), 0);
    }

    #[test]
    fn create_entity_assigns_sequential_ids() {
        let mut mgr = EntityManager::new();
        let id1 = mgr.create_entity("SGWPlayer");
        let id2 = mgr.create_entity("SGWMob");
        assert_eq!(id1, EntityId(1));
        assert_eq!(id2, EntityId(2));
        assert_eq!(mgr.entity_count(), 2);
    }

    #[test]
    fn get_entity_returns_created_entity() {
        let mut mgr = EntityManager::new();
        let id = mgr.create_entity("SGWPlayer");
        let entity = mgr.get_entity(id).unwrap();
        assert_eq!(entity.entity_id, id);
        assert_eq!(entity.entity_type, "SGWPlayer");
    }

    #[test]
    fn get_missing_entity_returns_none() {
        let mgr = EntityManager::new();
        assert!(mgr.get_entity(EntityId(999)).is_none());
    }

    #[test]
    fn destroy_entity_removes_it() {
        let mut mgr = EntityManager::new();
        let id = mgr.create_entity("SGWMob");
        assert_eq!(mgr.entity_count(), 1);
        mgr.destroy_entity(id);
        assert_eq!(mgr.entity_count(), 0);
        assert!(mgr.get_entity(id).is_none());
    }

    #[test]
    fn destroy_entity_recycles_id() {
        let mut mgr = EntityManager::new();
        let id1 = mgr.create_entity("SGWMob");
        mgr.destroy_entity(id1);
        let id2 = mgr.create_entity("SGWPlayer");
        // Should recycle the destroyed ID.
        assert_eq!(id2, id1);
    }

    #[test]
    fn get_entity_mut_allows_modification() {
        let mut mgr = EntityManager::new();
        let id = mgr.create_entity("SGWPlayer");
        {
            let entity = mgr.get_entity_mut(id).unwrap();
            entity.is_persistent = true;
        }
        let entity = mgr.get_entity(id).unwrap();
        assert!(entity.is_persistent);
    }

    #[test]
    fn destroy_nonexistent_entity_is_safe() {
        let mut mgr = EntityManager::new();
        mgr.destroy_entity(EntityId(42)); // should not panic
        assert_eq!(mgr.entity_count(), 0);
    }

    #[test]
    fn allocate_id_starts_at_one() {
        let mut mgr = EntityManager::new();
        let id = mgr.allocate_id();
        assert_eq!(id, EntityId(1));
    }

    #[test]
    fn default_is_same_as_new() {
        let mgr = EntityManager::default();
        assert_eq!(mgr.entity_count(), 0);
    }
}
