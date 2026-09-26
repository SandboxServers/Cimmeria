//! `SpawnRecord`: one NPC spawn, as the cell's spawner loads it and as the
//! base ships it back for a GM `.spawn` (`BaseToCellMsg::GmSpawnNpcReady`).
//!
//! It is a message payload, so it lives in the wire contract rather than in
//! the spawner's DB loaders (`cimmeria-services`' `cell::spawner`), which
//! re-export it at their old path. `class_id_for_class` maps its `class`
//! column to the wire entity class id.

/// A spawn record loaded from the database, joining spawnlist + entity_templates + worlds.
#[derive(Debug, Clone)]
pub struct SpawnRecord {
    pub spawn_id: i32,
    pub world_name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub heading: f32,
    pub tag: Option<String>,
    pub template_id: i32,
    pub template_name: String,
    pub class: String,
    pub static_mesh: Option<String>,
    pub body_set: String,
    pub components: Option<Vec<String>>,
    pub flags: i64,
    pub interaction_type: i64,
    pub event_set_id: Option<i32>,
    pub level: Option<i32>,
    pub alignment: Option<i32>,
    pub faction: Option<i32>,
    pub name_id: Option<i32>,
    pub speaker_id: Option<i32>,
    pub static_interaction_sets: Vec<i32>,
    pub has_dynamic_properties: bool,
    pub loot_table_id: Option<i32>,
    pub is_stationary: bool,
    /// Ability IDs the NPC starts with, loaded from the template's
    /// `ability_set_id` via `ability_set_abilities`. Empty when the
    /// template has no ability set — the spawn path falls back to
    /// `NPC_DEFAULT_ABILITY` so we never spawn a defenseless mob by
    /// accident (the Castle_CellBlock guards rely on this fallback).
    pub ability_ids: Vec<i32>,
    /// Resolved respawn delay in seconds.
    /// `COALESCE(spawnlist.respawn_secs, entity_templates.respawn_secs)`:
    /// per-spawn override beats template default. `None` on both → mob
    /// is one-shot (no respawn — the corpse stays in the world). Set
    /// to a positive integer to opt in. Zero and negative values from
    /// the DB are downgraded to `None` at load time so a misconfigured
    /// row doesn't trigger an instant respawn loop.
    pub respawn_secs: Option<u32>,
    /// Ordered waypoints loaded from
    /// `point_set_points` keyed by `entity_templates.patrol_path_id`.
    /// Empty when the template has no `patrol_path_id` or the
    /// referenced set has zero points. The runtime checks
    /// `patrol_path.is_empty()` to decide whether the NPC has a
    /// patrol behavior at all — non-empty paths transition the
    /// NPC out of Idle into `AiState::Patrol` once the AI tick
    /// runs.
    pub patrol_path: Vec<cimmeria_common::Vector3>,
    /// Seconds the NPC dwells at each patrol waypoint before
    /// moving on. Defaulted to `2.0` if the template has a NULL
    /// `patrol_point_delay` but a non-empty path (some content
    /// authors fill the path but leave the delay null). Ignored
    /// when `patrol_path.is_empty()`.
    pub patrol_point_delay_secs: f32,
    /// Wander radius in world units. `0.0` → no wander. Positive
    /// values opt the NPC into `AiState::Wander` from Idle (when
    /// it has no patrol_path and is not hostile on sight).
    pub wander_radius: f32,
    /// Lower bound of the random dwell duration drawn between
    /// successive wander hops, in seconds. Defaults to `3.0` when
    /// the template field is NULL.
    pub wander_min_dwell_secs: f32,
    /// Upper bound of the random dwell duration, in seconds.
    /// Defaults to `8.0` when NULL. The CHECK constraint enforces
    /// `min <= max` at the DB boundary so the runtime can sample
    /// without an extra guard.
    pub wander_max_dwell_secs: f32,
    /// Follow-state distance band lower bound, in world units. The
    /// NPC doesn't back away from the target inside this distance —
    /// just holds position. Defaulted to `2.0` when NULL.
    pub follow_min_distance: f32,
    /// Follow-state distance band upper bound, in world units. The
    /// NPC walks toward the target whenever the distance exceeds
    /// this. Defaulted to `5.0` when NULL.
    pub follow_max_distance: f32,
    /// Per-tick movement speed, in world units per 100ms tick.
    /// Defaulted to `0.6` (the historical hardcoded value from
    /// `CellEntity::new`) when the template's `move_speed` is NULL.
    ///
    /// `0.6` (6.0 units/sec) is 26% slower than World 12's player run
    /// speed (8.125 units/sec) — too slow for a follower NPC to ever
    /// close the follow-distance band against a moving player.
    /// Templates that need to keep pace (escort/companion NPCs) set
    /// this column explicitly; see `entity_templates.move_speed`.
    pub move_speed: f32,
    /// Per-template leash radius in world units, from
    /// `entity_templates.leash_distance`. `None` (the column is NULL) means
    /// the server default `combat::LEASH_DISTANCE` applies. Not COALESCEd in
    /// SQL so the runtime can tell "the template chose 50" from "the template
    /// said nothing" in its logs.
    pub leash_distance: Option<f32>,
    /// Per-template proximity-aggro radius in world units, from
    /// `entity_templates.aggro_radius` (NA13). `None` (NULL) means the
    /// server default `combat::DEFAULT_AGGRO_RADIUS` (18) applies.
    pub aggro_radius: Option<f32>,
    /// Per-template assist radius in world units, from
    /// `entity_templates.assist_radius` (NA14). `None` (NULL) means the
    /// server default `combat::DEFAULT_ASSIST_RADIUS` (10) applies.
    pub assist_radius: Option<f32>,
    /// Per-spawn aggression override, from `spawnlist.aggression_override`
    /// (`EMobAggressionLevel`, NA13). `None` (NULL) means the faction
    /// reaction decides. Seeded NEUTRAL on chain-armed spawns so their chain,
    /// not proximity, starts the fight. Always `None` on a template
    /// prototype: it is a placement property, like `is_stationary`.
    pub aggression_override: Option<cimmeria_entity::cell_entity::MobAggression>,
    /// `entity_templates.use_cover`. `None` (NULL) applies the default
    /// rule at spawn: a hostile (`faction = 10`) NPC takes cover. A
    /// stationary NPC or a prop never does, whatever the column says
    /// (NA22, `SGWMob.def` `useCover`).
    pub use_cover: Option<bool>,
}

/// Map the DB `entity_templates.class` column to the wire class_id.
///
/// The class_id is the entity type index from `entities/entities.xml`:
///   0 = SGWSpawnableEntity, 1 = SGWBeing, 2 = SGWPlayer, 3 = SGWGmPlayer,
///   4 = SGWMob, 5 = SGWPet, 6 = SGWDuelMarker, 7 = SGWBlackMarket
pub fn class_id_for_class(class: &str) -> u8 {
    match class {
        "spawnable" => 0x00, // SGWSpawnableEntity
        "being" => 0x01,     // SGWBeing
        "mob" => 0x04,       // SGWMob
        _ => 0x04,           // Default to SGWMob
    }
}
