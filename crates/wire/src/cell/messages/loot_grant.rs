//! The loot-grant round trip: where a looted item came from, and why the
//! base refused to grant it.
//!
//! The cell takes an item off a corpse before the base has written it to
//! the looter's inventory. [`LootGrantSource`] rides on
//! `CellToBaseMsg::GrantItem` so that a grant the base refuses can be put
//! back on the corpse (`BaseToCellMsg::LootGrantRefused`) instead of being
//! destroyed.

use std::time::Instant;

/// The corpse and loot entry a granted item was taken from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootGrantSource {
    /// The corpse's entity id (the looter's `looting_entity`).
    pub corpse_id: u32,
    /// The loot entry's index on that corpse.
    pub index: i32,
    /// The corpse's `respawn_at` when the item was taken. A corpse that
    /// respawned and died again while the grant was in flight has a new
    /// one, so the item is not put onto the wrong body.
    pub corpse_respawn_at: Option<Instant>,
    /// The corpse's template when the item was taken, checked with
    /// `corpse_respawn_at` for the same reason.
    pub corpse_template_id: Option<i32>,
}

/// Why the base refused a grant. Every variant means no inventory write
/// committed, so the item may be returned to where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantRefusal {
    /// The item lists only storage containers, which a grant never writes.
    StorageOnly,
    /// The grant resolved to the buyback list, which a grant never writes.
    NotGrantable,
    /// The container the item goes to has no free slot.
    ContainerFull,
    /// The base has no database pool.
    NoDatabase,
    /// A database error before the commit; the transaction rolled back.
    DatabaseError,
}

impl GrantRefusal {
    /// The `reason` value logged for this refusal.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StorageOnly => "storage_only",
            Self::NotGrantable => "not_grantable_container",
            Self::ContainerFull => "container_full",
            Self::NoDatabase => "no_database",
            Self::DatabaseError => "database_error",
        }
    }
}
