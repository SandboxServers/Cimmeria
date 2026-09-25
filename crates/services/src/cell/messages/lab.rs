//! Live-research-lab query contract (issue #688, phase 5).
//!
//! These types are the request/reply payload for
//! [`crate::cell::messages::BaseToCellMsg::LabQuery`] — the read-only snapshot
//! path the lab MCP endpoint (`cimmeria-lab-mcp`) uses to inspect live cell
//! state (`server_entity_get` / `server_entity_query` / `server_witnesses`).
//!
//! The cell loop owns `SpaceManager`; nothing outside the loop can read it. The
//! one precedent for pulling a value back out is
//! [`crate::cell::messages::BaseToCellMsg::CreateEntity`]'s `reply_tx`. `LabQuery`
//! follows that shape: the lab endpoint (base-side) builds a [`LabQuery`], sends
//! it with a `oneshot` reply channel, and the cell handler answers *between
//! ticks* by reading `SpaceManager` — never mutating it.
//!
//! # Cost discipline
//!
//! The handler runs on the cell loop thread, so every snapshot is built from
//! **copied-out primitives** — no references into `SpaceManager` escape, and a
//! query can never stall the tick unboundedly:
//! [`LabEntityFilter`] queries are capped at [`LAB_ENTITY_QUERY_CAP`] snapshots
//! (with `total_matched`/`capped` reported so the caller knows it was
//! truncated). The witness lists are naturally bounded by AoI size.

use serde::{Deserialize, Serialize};

/// Maximum number of entity snapshots a single [`LabQuery::EntityQuery`]
/// returns, regardless of how many entities match the filter.
///
/// The snapshot build runs on the cell loop thread between ticks; an
/// unbounded materialization over a densely-populated space (a raid, a mob
/// pull, a stress test) would allocate a snapshot per entity and stall the
/// 100 ms tick. The cap keeps the worst-case work bounded no matter what the
/// caller asks for. The reply always reports the pre-cap `total_matched` and a
/// `capped` flag so the caller can tell a truncated answer from a complete one
/// and narrow the filter (by space / template / radius) to see the rest.
pub const LAB_ENTITY_QUERY_CAP: usize = 256;

/// Result of a [`crate::cell::messages::BaseToCellMsg::LabQuery`].
///
/// `Err(_)` is a human-readable reason the query could not be answered (today
/// only "unknown entity" for a radius-around-entity center whose anchor does
/// not exist). A successful query that simply matched nothing is
/// `Ok(LabQueryReply::…)` with an empty payload, never `Err`.
pub type LabQueryResult = Result<LabQueryReply, String>;

/// A read-only live-state query answered by the cell loop.
#[derive(Debug, Clone)]
pub enum LabQuery {
    /// One entity snapshot by id, or `None` if no such entity exists.
    EntityGet { entity_id: u32 },
    /// Every entity matching [`LabEntityFilter`], capped at
    /// [`LAB_ENTITY_QUERY_CAP`].
    EntityQuery { filter: LabEntityFilter },
    /// The bidirectional witness relationship for one entity.
    Witnesses { entity_id: u32 },
}

/// Filter for [`LabQuery::EntityQuery`]. All fields are AND-combined; a `None`
/// field imposes no constraint. An all-`None` filter matches every entity on
/// the cell (still capped at [`LAB_ENTITY_QUERY_CAP`]).
#[derive(Debug, Clone, Default)]
pub struct LabEntityFilter {
    /// Restrict to a single space instance id.
    pub space_id: Option<u32>,
    /// Restrict to entities whose `template_id` matches (`entity_templates.template_id`).
    pub template_id: Option<i32>,
    /// Restrict to entities of a given wire class id (`0x02` = SGWPlayer,
    /// `0x04` = SGWMob).
    pub class_id: Option<u8>,
    /// Restrict to entities within a radius of a point or another entity.
    pub radius: Option<LabRadius>,
}

/// Spatial constraint for [`LabEntityFilter`]: match only entities within
/// `radius` world units of `center`.
#[derive(Debug, Clone)]
pub struct LabRadius {
    pub center: LabRadiusCenter,
    /// Radius in world units. Compared against 3D Euclidean distance.
    pub radius: f32,
}

/// Where a [`LabRadius`] is centered.
#[derive(Debug, Clone)]
pub enum LabRadiusCenter {
    /// Around an existing entity's current position. If the anchor entity does
    /// not exist the query fails with `Err(_)` rather than silently matching
    /// nothing.
    Entity(u32),
    /// Around an explicit world-space point.
    Point([f32; 3]),
}

/// The payload of a successful [`LabQueryResult`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LabQueryReply {
    /// [`LabQuery::EntityGet`] — the snapshot, or `null` if absent.
    Entity { entity: Option<LabEntitySnapshot> },
    /// [`LabQuery::EntityQuery`] — the (capped) matching snapshots.
    Entities {
        /// The snapshots actually returned (`len() <= LAB_ENTITY_QUERY_CAP`).
        entities: Vec<LabEntitySnapshot>,
        /// How many entities matched the filter *before* the cap was applied.
        total_matched: usize,
        /// `true` when `total_matched > entities.len()` — the answer is
        /// truncated and the caller should narrow the filter.
        capped: bool,
    },
    /// [`LabQuery::Witnesses`] — the bidirectional witness report.
    Witnesses { report: LabWitnessReport },
}

/// A copied-out, allocation-bounded snapshot of one cell entity. Every field is
/// a primitive or a small owned value — no references into `SpaceManager`.
///
/// Deliberately not a full `CellEntity` mirror: it carries the fields an AoI /
/// visibility / spawn investigation actually needs (identity, position,
/// class/faction, health, AI state, witness counts). Combat timers, bandolier
/// contents, trade proposals, and the like are intentionally omitted — add a
/// field here only when a lab question needs it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabEntitySnapshot {
    pub entity_id: u32,
    pub space_id: u32,
    pub world_name: String,
    pub position: [f32; 3],
    /// `[pitch, yaw, roll]` in radians (players and NPCs both — see
    /// `CellEntity::direction`).
    pub direction: [f32; 3],
    pub velocity: [f32; 3],
    pub is_on_ground: bool,
    pub is_player: bool,
    /// Wire class id: `0x02` SGWPlayer, `0x04` SGWMob.
    pub class_id: u8,
    pub faction: u8,
    pub alignment: u8,
    pub level: u32,
    /// `character_name` for players, `npc_name` for NPCs; `None` if unset.
    pub name: Option<String>,
    pub template_id: Option<i32>,
    pub spawn_id: Option<i32>,
    pub tag: Option<String>,
    pub name_id: Option<i32>,
    pub archetype_id: Option<i32>,
    pub access_level: u32,
    /// Debug label of the NPC AI state (`Idle`, `Fighting`, `Dead`, …).
    /// Meaningful only for NPCs; present for all entities.
    pub ai_state: String,
    pub current_target_id: Option<i32>,
    pub aoi_radius: f32,
    pub state_field: u32,
    pub interaction_type_flags: i64,
    pub weapon_holstered: bool,
    /// Whether this entity renders as a static mesh (corpse / prop) rather than
    /// a composited body. Directly relevant to the invisible-corpse AoI class
    /// of bug this phase targets.
    pub has_static_mesh: bool,
    /// Number of visual components composited into this entity's appearance.
    pub component_count: usize,
    /// Number of entities that currently have this entity in their AoI
    /// (i.e. `CellEntity::witnesses.len()`). For a player this is what it
    /// *sees*; see [`LabQuery::Witnesses`] for the resolved bidirectional view.
    pub witness_count: usize,
    /// Current / max HEALTH stat, if the entity carries one.
    pub health_cur: Option<i32>,
    pub health_max: Option<i32>,
}

/// The bidirectional witness relationship for one entity — the direct answer to
/// "who can see X, and whom does X see", the core AoI question this phase
/// exists to make answerable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabWitnessReport {
    pub entity_id: u32,
    pub space_id: u32,
    /// Player entity ids that currently have `entity_id` in their AoI —
    /// the observers of X. Resolved via `SpaceManager::get_witnesses_of`.
    pub witnessed_by: Vec<u32>,
    /// Entity ids `entity_id` currently sees. Populated only when `entity_id`
    /// is a player (only players carry a witness set); empty for NPCs.
    pub witnesses: Vec<u32>,
}
