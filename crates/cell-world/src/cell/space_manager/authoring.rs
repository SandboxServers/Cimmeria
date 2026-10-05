//! Types for the `.`-console authoring buffer ([`SpaceManager::authoring_changes`]).
//!
//! The buffer holds what a GM has authored but not yet confirmed. Nothing in
//! it has touched the database: `.seedconfirm` writes the live DB and emits
//! the telemetry, `.seedcancel` throws it away. The logic lives in
//! `cimmeria_cell_console::cell::console::seed`; this module only defines the
//! shapes, because `SpaceManager` owns the buffer.
//!
//! [`SpaceManager::authoring_changes`]: super::SpaceManager::authoring_changes

/// One authored change waiting for `.seedconfirm`.
#[derive(Debug, Clone, PartialEq)]
pub struct AuthoringChange {
    /// The `db/resources/` seed file the statement belongs in.
    pub seed_file: String,
    /// Short command tag (`"savespawn"`, `"path_add"`, …).
    pub label: String,
    /// One server-generated SQL statement. It is valid both against the live
    /// DB and appended to `seed_file`.
    pub sql: String,
    /// The structured spawnlist row behind a `.savespawn` / `.delspawn`, so a
    /// confirmed spawn can be rebuilt from telemetry without parsing `sql`.
    /// `None` for every other authoring command.
    pub spawn: Option<SpawnRowChange>,
}

/// What a spawn authoring change does to `resources.spawnlist`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnRowOp {
    /// A new row for an NPC that has no `spawn_id` yet (`.spawn`ed this session).
    Insert,
    /// Re-place an existing row, keyed on its `spawn_id`.
    Update,
    /// Delete an existing row, keyed on its `spawn_id`.
    Delete,
}

impl SpawnRowOp {
    /// Lower-case name used in telemetry (`op=insert`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Insert => "insert",
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }
}

/// Every column a spawn authoring change sets, captured when the GM ran the
/// command.
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnRowChange {
    pub op: SpawnRowOp,
    /// The NPC's cell entity id. Later saves of the same NPC replace this
    /// change in the buffer instead of adding another.
    pub entity_id: u32,
    /// The NPC's name when the GM queued the change (Rule 6). Captured then,
    /// never at `.seedconfirm`: by confirm time a `.delspawn`ed NPC is gone
    /// and its slot may hold another entity.
    pub entity_name: Option<&'static str>,
    /// `None` for [`SpawnRowOp::Insert`]: the id comes from the sequence.
    pub spawn_id: Option<i32>,
    pub world: String,
    /// `resources.worlds.world_id`, when the world has a row.
    pub world_id: Option<i32>,
    pub template_id: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    /// Yaw in radians (`direction.y`), the `spawnlist.heading` column.
    pub heading: f32,
    pub tag: Option<String>,
}
