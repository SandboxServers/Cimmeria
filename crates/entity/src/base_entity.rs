//! Base-side entity representation.
//!
//! A `BaseEntity` lives on the BaseApp and owns the persistent state of a game
//! entity. It may optionally have a cell-side counterpart (managed by CellApp)
//! and a client connection. Communication with those remote halves happens
//! through mailboxes.
//!
//! This corresponds to the C++ `Base` class in `src/server/BaseApp/`.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use cimmeria_common::{EntityId, Vector3};

use crate::mailbox::{CellMailbox, ClientMailbox};

/// A value that can be stored in an entity property slot.
///
/// These correspond to the primitive types supported by the BigWorld entity
/// definition system. Complex types (ARRAY, FIXED_DICT, PYTHON) are stored
/// as `Blob` and decoded on demand.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PropertyValue {
    Int32(i32),
    Float32(f32),
    String(String),
    Bool(bool),
    Vector3(Vector3),
    Uint32(u32),
    Int64(i64),
    Blob(Vec<u8>),
}

impl fmt::Display for PropertyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropertyValue::Int32(v) => write!(f, "{}", v),
            PropertyValue::Float32(v) => write!(f, "{}", v),
            PropertyValue::String(v) => write!(f, "{}", v),
            PropertyValue::Bool(v) => write!(f, "{}", v),
            PropertyValue::Vector3(v) => write!(f, "({}, {}, {})", v.x, v.y, v.z),
            PropertyValue::Uint32(v) => write!(f, "{}", v),
            PropertyValue::Int64(v) => write!(f, "{}", v),
            PropertyValue::Blob(v) => write!(f, "<blob {} bytes>", v.len()),
        }
    }
}

/// The base-side half of a game entity.
///
/// Manages persistent state, database identity, and mailbox references to the
/// cell entity and client connection. Created and destroyed by the
/// [`EntityManager`](crate::manager::EntityManager).
pub struct BaseEntity {
    /// Unique runtime identifier assigned by the EntityManager.
    pub entity_id: EntityId,

    /// Entity type name as declared in `entities/entities.xml` (e.g. "SGWPlayer").
    pub entity_type: String,

    /// Property bag keyed by property name.
    pub properties: HashMap<String, PropertyValue>,

    /// Database row ID for persistent entities, or `None` for transient ones.
    pub db_id: Option<i64>,

    /// Mailbox to the cell-side counterpart (if this entity has a cell presence).
    pub cell_mailbox: Option<CellMailbox>,

    /// Mailbox to the owning client (if a player is connected).
    pub client_mailbox: Option<ClientMailbox>,

    /// Whether this entity should be saved to the database on destruction.
    pub is_persistent: bool,
}

impl BaseEntity {
    /// Create a new base entity with the given ID and type name.
    ///
    /// The entity starts with no properties, no database identity, no cell
    /// presence, and no client connection.
    pub fn new(entity_id: EntityId, entity_type: String) -> Self {
        Self {
            entity_id,
            entity_type,
            properties: HashMap::new(),
            db_id: None,
            cell_mailbox: None,
            client_mailbox: None,
            is_persistent: false,
        }
    }

    /// Retrieve a property value by name, if it exists.
    pub fn get_property(&self, name: &str) -> Option<&PropertyValue> {
        self.properties.get(name)
    }

    /// Set a property value, inserting it if it does not already exist.
    pub fn set_property(&mut self, name: String, value: PropertyValue) {
        self.properties.insert(name, value);
    }

    /// Persist the entity state to the database.
    ///
    /// # Panics
    ///
    /// Not yet implemented -- will serialize properties and write to the
    /// `sgw_player` / entity-specific table via the persistence layer.
    pub async fn save_to_db(&self) -> cimmeria_common::Result<()> {
        todo!("BaseEntity::save_to_db - serialize properties and write via persistence layer")
    }

    /// Load entity state from the database by `db_id`.
    ///
    /// # Panics
    ///
    /// Not yet implemented -- will query the appropriate table and deserialize
    /// properties into the property bag.
    pub async fn load_from_db(&mut self, db_id: i64) -> cimmeria_common::Result<()> {
        let _ = db_id;
        todo!("BaseEntity::load_from_db - query persistence layer and populate properties")
    }

    /// Returns `true` if this entity currently has a cell-side counterpart.
    pub fn has_cell_entity(&self) -> bool {
        self.cell_mailbox.is_some()
    }

    /// Tear down this entity, cleaning up mailbox references.
    ///
    /// # Panics
    ///
    /// Not yet implemented -- will notify the cell entity (if any) and client
    /// (if any) that the entity is being destroyed.
    pub fn destroy(&mut self) {
        todo!("BaseEntity::destroy - notify cell/client and clean up resources")
    }
}

impl fmt::Debug for BaseEntity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BaseEntity")
            .field("entity_id", &self.entity_id)
            .field("entity_type", &self.entity_type)
            .field("db_id", &self.db_id)
            .field("is_persistent", &self.is_persistent)
            .field("has_cell", &self.has_cell_entity())
            .field("has_client", &self.client_mailbox.is_some())
            .field("property_count", &self.properties.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_common::SpaceId;

    #[test]
    fn new_entity_has_no_cell() {
        let entity = BaseEntity::new(EntityId(1), "SGWPlayer".to_string());
        assert!(!entity.has_cell_entity());
    }

    #[test]
    fn new_entity_has_no_client() {
        let entity = BaseEntity::new(EntityId(1), "SGWPlayer".to_string());
        assert!(entity.client_mailbox.is_none());
    }

    #[test]
    fn set_and_get_property() {
        let mut entity = BaseEntity::new(EntityId(1), "SGWPlayer".to_string());
        entity.set_property("health".to_string(), PropertyValue::Int32(100));
        assert_eq!(
            entity.get_property("health"),
            Some(&PropertyValue::Int32(100))
        );
    }

    #[test]
    fn get_missing_property_returns_none() {
        let entity = BaseEntity::new(EntityId(1), "SGWPlayer".to_string());
        assert!(entity.get_property("nonexistent").is_none());
    }

    #[test]
    fn set_property_overwrites() {
        let mut entity = BaseEntity::new(EntityId(1), "SGWPlayer".to_string());
        entity.set_property("level".to_string(), PropertyValue::Int32(1));
        entity.set_property("level".to_string(), PropertyValue::Int32(50));
        assert_eq!(
            entity.get_property("level"),
            Some(&PropertyValue::Int32(50))
        );
    }

    #[test]
    fn property_value_display() {
        assert_eq!(format!("{}", PropertyValue::Int32(42)), "42");
        assert_eq!(format!("{}", PropertyValue::Bool(true)), "true");
        assert_eq!(
            format!("{}", PropertyValue::String("hello".to_string())),
            "hello"
        );
        assert_eq!(
            format!("{}", PropertyValue::Blob(vec![0; 16])),
            "<blob 16 bytes>"
        );
    }

    #[test]
    fn new_entity_is_not_persistent() {
        let entity = BaseEntity::new(EntityId(1), "SGWMob".to_string());
        assert!(!entity.is_persistent);
    }

    #[test]
    fn has_cell_entity_with_mailbox() {
        let mut entity = BaseEntity::new(EntityId(1), "SGWPlayer".to_string());
        entity.cell_mailbox = Some(CellMailbox {
            entity_id: EntityId(1),
            space_id: SpaceId(100),
        });
        assert!(entity.has_cell_entity());
    }
}
