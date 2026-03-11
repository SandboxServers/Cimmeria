use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for an entity in the game world.
///
/// Corresponds to the C++ `EntityID` (int32) used throughout the BigWorld engine.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub i32);

/// Unique identifier for a game space (world/zone instance).
///
/// Corresponds to the C++ `SpaceID` (int32) used in CellApp space management.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpaceId(pub i32);

/// Unique identifier for a client session on the BaseApp.
///
/// Assigned during login and used to route messages to the correct client channel.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub u32);

/// Unique identifier for a player account.
///
/// Corresponds to the `account_id` column in the `sgw.account` table.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub i32);

/// Mercury protocol message identifier.
///
/// Each message type in the Mercury protocol is identified by a single byte.
/// See `src/mercury/base_cell_messages.hpp` for the C++ definitions.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MessageId(pub u8);

/// Entity data distribution flags from the BigWorld entity definition system.
///
/// These flags control which parts of the distributed architecture receive
/// property updates. They map directly to `EntityDataFlags` in the C++ source
/// and the `<Flags>` element in `.def` files.
///
/// See `docs/protocol/entity-property-sync.md` for full documentation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DistributionFlags(pub u32);

impl DistributionFlags {
    // --- Primitive bit flags (from data_description.hpp EntityDataFlags) ---

    /// `DATA_GHOSTED` (0x01) - Synced to ghost entities on other CellApps.
    /// Used for properties visible to other cells (e.g., position, appearance).
    pub const CELL_PUBLIC: Self = Self(0x01);

    /// `DATA_GHOSTED + DATA_OTHER_CLIENT` (0x03) - Sent to other players
    /// who can see this entity, plus ghosted to other CellApps.
    pub const OTHER_CLIENTS: Self = Self(0x01 | 0x02);

    /// `DATA_OWN_CLIENT` (0x04) - Sent only to the owning client.
    pub const OWN_CLIENT: Self = Self(0x04);

    /// `DATA_GHOSTED + DATA_OTHER_CLIENT + DATA_OWN_CLIENT` (0x07) -
    /// Sent to all clients: the owner and everyone who can see the entity.
    pub const ALL_CLIENTS: Self = Self(0x01 | 0x02 | 0x04);

    /// `DATA_BASE + DATA_OWN_CLIENT` (0x0C) - Exists on the base entity
    /// and is also sent to the owning client.
    pub const BASE_AND_CLIENT: Self = Self(0x08 | 0x04);

    /// No distribution bits set - property exists only on the cell entity
    /// and is never synced to ghosts, clients, or the base.
    pub const CELL_PRIVATE: Self = Self(0x00);

    /// `DATA_BASE` (0x08) - Property exists on the base entity (BaseApp).
    pub const BASE: Self = Self(0x08);

    /// `DATA_EDITOR_ONLY` (0x40) - Editor-only data, not used at runtime.
    pub const EDITOR: Self = Self(0x40);

    /// Returns `true` if all bits in `other` are set in `self`.
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Returns the union of two flag sets.
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Returns the intersection of two flag sets.
    pub fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Returns `true` if no bits are set.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// High-level entity classification.
///
/// Maps to the entity type names defined in `entities/entities.xml`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Player,
    Mob,
    Npc,
    Item,
    Other(String),
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Entity({})", self.0)
    }
}

impl fmt::Display for SpaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Space({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_id_display() {
        let id = EntityId(42);
        assert_eq!(format!("{}", id), "Entity(42)");
    }

    #[test]
    fn space_id_display() {
        let id = SpaceId(7);
        assert_eq!(format!("{}", id), "Space(7)");
    }

    #[test]
    fn distribution_flags_contains() {
        assert!(DistributionFlags::ALL_CLIENTS.contains(DistributionFlags::OWN_CLIENT));
        assert!(DistributionFlags::ALL_CLIENTS.contains(DistributionFlags::CELL_PUBLIC));
        assert!(!DistributionFlags::OWN_CLIENT.contains(DistributionFlags::BASE));
    }

    #[test]
    fn distribution_flags_cell_private_is_empty() {
        assert!(DistributionFlags::CELL_PRIVATE.is_empty());
    }

    #[test]
    fn distribution_flags_union() {
        let combined = DistributionFlags::BASE.union(DistributionFlags::OWN_CLIENT);
        assert_eq!(combined, DistributionFlags::BASE_AND_CLIENT);
    }

    #[test]
    fn entity_id_equality() {
        assert_eq!(EntityId(1), EntityId(1));
        assert_ne!(EntityId(1), EntityId(2));
    }

    #[test]
    fn message_id_value() {
        let msg = MessageId(0xFF);
        assert_eq!(msg.0, 255);
    }
}
