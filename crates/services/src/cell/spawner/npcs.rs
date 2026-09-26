//! Database-driven NPC spawning.
//!
//! `SpawnRecord` materializes the join of `resources.spawnlist` +
//! `resources.entity_templates` + `resources.worlds` into a single row.
//! `spawn_npcs_from_records` populates startup spaces; instanced spaces
//! go through `spawn_instance_npcs_from_records` so they don't recreate
//! the space.

use sqlx::PgPool;

use super::super::space_manager::SpaceManager;

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

/// Load all spawn records from the database.
///
/// Joins `resources.spawnlist` with `resources.entity_templates` and
/// `resources.worlds` to get position, template data, and world name
/// in a single query.
pub async fn load_spawns_from_db(pool: &PgPool) -> Result<Vec<SpawnRecord>, sqlx::Error> {
    use sqlx::Row;

    // Correlated subquery + `COALESCE(..., ARRAY[]::int[])` pulls the
    // per-template ability bucket alongside the spawn row in one round-trip.
    // When the template's `ability_set_id` is NULL (no matching rows in
    // `ability_set_abilities`), the inner `array_agg` returns NULL and
    // COALESCE substitutes an empty Postgres array so Rust always sees
    // `Vec<i32>` (possibly empty), never `None`. Templates without an
    // ability set fall back to NPC_DEFAULT_ABILITY in
    // `spawn_npc_from_record_into`.
    let rows = sqlx::query(
        "SELECT s.spawn_id, w.world AS world_name, s.x, s.y, s.z, s.heading, s.tag, \
               s.is_stationary, \
               t.template_id, t.template_name, t.class, t.static_mesh, t.body_set, \
               t.components, t.flags, t.interaction_type, t.event_set_id, t.level, \
               t.alignment, t.faction, t.name_id, t.speaker_id, \
               t.static_interaction_sets, t.has_dynamic_properties, \
               t.loot_table_id, \
               COALESCE(s.patrol_path_id, t.patrol_path_id) AS patrol_path_id, \
               COALESCE(s.patrol_point_delay, t.patrol_point_delay, 2.0) AS patrol_point_delay, \
               COALESCE(t.wander_radius, 0.0) AS wander_radius, \
               COALESCE(t.wander_min_dwell_secs, 3.0) AS wander_min_dwell_secs, \
               COALESCE(t.wander_max_dwell_secs, 8.0) AS wander_max_dwell_secs, \
               COALESCE(t.follow_min_distance, 2.0) AS follow_min_distance, \
               COALESCE(t.follow_max_distance, 5.0) AS follow_max_distance, \
               COALESCE(t.move_speed, 0.6) AS move_speed, \
               t.leash_distance, t.aggro_radius, t.assist_radius, s.aggression_override, \
               t.use_cover, \
               COALESCE(s.respawn_secs, t.respawn_secs) AS respawn_secs, \
               COALESCE( \
                 (SELECT array_agg(asa.ability_id ORDER BY asa.ability_id) \
                  FROM resources.ability_set_abilities asa \
                  WHERE asa.ability_set_id = t.ability_set_id), \
                 ARRAY[]::int[] \
               ) AS ability_ids \
        FROM resources.spawnlist s \
        JOIN resources.entity_templates t ON s.template_id = t.template_id \
        JOIN resources.worlds w ON s.world_id = w.world_id \
        ORDER BY s.spawn_id",
    )
    .fetch_all(pool)
    .await?;

    // Load patrol points for every patrol_path_id referenced by the
    // spawn list, in a single follow-up query. Building a HashMap
    // keyed by set_id then lookup per-record is cheaper than the
    // alternative (correlated subquery + jsonb_agg) and keeps the
    // primary spawn query readable. Empty patrol_path_ids → empty
    // hashmap → every spawn gets `patrol_path: vec![]`.
    let patrol_path_ids: Vec<i32> = rows
        .iter()
        .filter_map(|r| r.try_get::<Option<i32>, _>("patrol_path_id").ok().flatten())
        .collect();
    let patrol_points = load_patrol_points(pool, &patrol_path_ids).await?;

    let records = rows
        .iter()
        .map(|r| SpawnRecord {
            spawn_id: r.get("spawn_id"),
            world_name: r.get("world_name"),
            x: r.get("x"),
            y: r.get("y"),
            z: r.get("z"),
            heading: r.get("heading"),
            tag: r.get("tag"),
            template_id: r.get("template_id"),
            template_name: r.get("template_name"),
            class: r.get("class"),
            static_mesh: r.get("static_mesh"),
            body_set: r.get("body_set"),
            components: r.get("components"),
            flags: r.get("flags"),
            interaction_type: r.get("interaction_type"),
            event_set_id: r.get("event_set_id"),
            level: r.get("level"),
            alignment: r.get("alignment"),
            faction: r.get("faction"),
            name_id: r.get("name_id"),
            speaker_id: r.get("speaker_id"),
            static_interaction_sets: r.get("static_interaction_sets"),
            has_dynamic_properties: r.get("has_dynamic_properties"),
            loot_table_id: r.get("loot_table_id"),
            is_stationary: r.get("is_stationary"),
            ability_ids: r.get::<Vec<i32>, _>("ability_ids"),
            respawn_secs: normalize_respawn_secs(
                r.try_get::<Option<i32>, _>("respawn_secs").ok().flatten(),
            ),
            patrol_path: r
                .try_get::<Option<i32>, _>("patrol_path_id")
                .ok()
                .flatten()
                .and_then(|id| patrol_points.get(&id).cloned())
                .unwrap_or_default(),
            patrol_point_delay_secs: r.get::<f32, _>("patrol_point_delay"),
            wander_radius: r.get::<f32, _>("wander_radius"),
            wander_min_dwell_secs: r.get::<f32, _>("wander_min_dwell_secs"),
            wander_max_dwell_secs: r.get::<f32, _>("wander_max_dwell_secs"),
            follow_min_distance: r.get::<f32, _>("follow_min_distance"),
            follow_max_distance: r.get::<f32, _>("follow_max_distance"),
            move_speed: r.get::<f32, _>("move_speed"),
            leash_distance: normalize_leash_distance(r.get::<Option<f32>, _>("leash_distance")),
            aggro_radius: normalize_aggro_radius(r.get::<Option<f32>, _>("aggro_radius")),
            assist_radius: normalize_aggro_radius(r.get::<Option<f32>, _>("assist_radius")),
            aggression_override: normalize_aggression_override(
                r.get::<Option<i16>, _>("aggression_override"),
            ),
            use_cover: r.get::<Option<bool>, _>("use_cover"),
        })
        .collect();

    Ok(records)
}

/// Load patrol waypoints for a set of `patrol_path_id` values from
/// `point_set_points`. Returns a `HashMap<set_id, ordered waypoints>`.
///
/// Waypoints are ordered by `point_id` — the canonical authoring order
/// of patrol points in a set. Empty `ids` short-circuits without a DB
/// round-trip; sets with zero points produce an entry mapping to
/// `vec![]` (caller treats as "no patrol").
pub(crate) async fn load_patrol_points(
    pool: &PgPool,
    ids: &[i32],
) -> Result<std::collections::HashMap<i32, Vec<cimmeria_common::Vector3>>, sqlx::Error> {
    use sqlx::Row;

    if ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let rows = sqlx::query(
        "SELECT set_id, x, y, z \
         FROM resources.point_set_points \
         WHERE set_id = ANY($1) \
         ORDER BY set_id, point_id",
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;

    let mut out: std::collections::HashMap<i32, Vec<cimmeria_common::Vector3>> =
        std::collections::HashMap::new();
    for r in &rows {
        let set_id: i32 = r.get("set_id");
        let x: f32 = r.get("x");
        let y: f32 = r.get("y");
        let z: f32 = r.get("z");
        out.entry(set_id)
            .or_default()
            .push(cimmeria_common::Vector3::new(x, y, z));
    }
    Ok(out)
}

/// Keep a template's `leash_distance` only when it is a positive, finite
/// radius. The DB CHECK already rejects `<= 0`; this is the runtime's
/// belt-and-suspenders against a hand-edited row, the same shape as
/// [`normalize_respawn_secs`]. Anything else falls back to the server
/// default by returning `None`.
pub(crate) fn normalize_leash_distance(raw: Option<f32>) -> Option<f32> {
    raw.filter(|d| d.is_finite() && *d > 0.0)
}

/// Keep a template's `aggro_radius` (or `assist_radius`, NA14) only when it is a positive, finite
/// radius; the same belt-and-suspenders as [`normalize_leash_distance`]
/// behind the DB CHECK. `None` means the server default applies.
pub(crate) fn normalize_aggro_radius(raw: Option<f32>) -> Option<f32> {
    raw.filter(|d| d.is_finite() && *d > 0.0)
}

/// A seeded `spawnlist.aggression_override` as an `EMobAggressionLevel`.
/// The DB CHECK already limits it to 1-5; anything else is dropped (the
/// faction reaction then decides) rather than read as hostile.
pub(crate) fn normalize_aggression_override(
    raw: Option<i16>,
) -> Option<cimmeria_entity::cell_entity::MobAggression> {
    raw.and_then(|v| cimmeria_entity::cell_entity::MobAggression::from_level(v as i32))
}

/// Downgrade a raw `respawn_secs` value from the DB to the runtime's
/// `Option<u32>` shape. Zero and negative values become `None` —
/// they would schedule an instant-or-past respawn deadline which the
/// tick would fire on the same frame the NPC died, producing a
/// runaway loop. Positive values pass through unchanged.
///
/// Belt-and-suspenders against the DB-level `CHECK respawn_secs > 0`
/// constraint: even if a future migration relaxes the check, the
/// runtime stays defensive.
pub(crate) fn normalize_respawn_secs(raw: Option<i32>) -> Option<u32> {
    raw.and_then(|v| if v > 0 { Some(v as u32) } else { None })
}

/// Spawn NPCs from DB records into all currently-loaded startup spaces.
///
/// Only spawns records whose `world_name` matches a space that already exists
/// in the SpaceManager (i.e., non-instanced startup spaces). Instanced spaces
/// are handled by `spawn_instance_npcs_from_records`.
#[tracing::instrument(
    name = "spawner.spawn_startup",
    level = "info",
    skip_all,
    fields(record_count = records.len(), spawned = tracing::field::Empty),
)]
pub fn spawn_npcs_from_records(records: &[SpawnRecord], space_mgr: &mut SpaceManager) -> usize {
    let mut count = 0;
    for record in records {
        // Only spawn in spaces that already exist (startup/non-instanced spaces)
        if !space_mgr.has_space_for_world(&record.world_name) {
            continue;
        }

        let npc_id = space_mgr.allocate_npc_id();
        match space_mgr.spawn_npc_from_record(npc_id, record) {
            Ok(space_id) => {
                tracing::debug!(
                    npc_id, space_id, spawn_id = record.spawn_id,
                    world = %record.world_name, name = %record.template_name,
                    class = %record.class, tag = ?record.tag,
                    "Spawned NPC from DB"
                );
                log_spawn_behaviour(space_mgr, npc_id);
                count += 1;
            }
            Err(e) => {
                tracing::warn!(
                    spawn_id = record.spawn_id, world = %record.world_name,
                    name = %record.template_name, "Failed to spawn NPC from DB: {e}"
                );
            }
        }
    }
    tracing::Span::current().record("spawned", count);
    tracing::info!(count, "DB-driven NPC population spawned (startup spaces)");
    count
}

/// Spawn NPCs from DB records for a specific instanced world into a given space.
///
/// Called when a new instanced space is created for a player (e.g., Castle_CellBlock,
/// SGC_W1). Each instance gets its own set of NPCs. The `space_id` parameter is the
/// space that was just created — NPCs are spawned directly into it rather than going
/// through `find_or_create_space` (which would create yet another new instance).
#[tracing::instrument(
    name = "spawner.spawn_instance",
    level = "info",
    skip_all,
    fields(world_name, space_id, record_count = records.len(), spawned = tracing::field::Empty),
)]
pub fn spawn_instance_npcs_from_records(
    records: &[SpawnRecord],
    world_name: &str,
    space_id: u32,
    space_mgr: &mut SpaceManager,
) -> usize {
    let mut count = 0;
    for record in records {
        if record.world_name != world_name {
            continue;
        }
        let npc_id = space_mgr.allocate_npc_id();
        match space_mgr.spawn_npc_from_record_in_space(npc_id, record, space_id) {
            Ok(sid) => {
                tracing::debug!(
                    npc_id, space_id = sid, spawn_id = record.spawn_id,
                    world = %record.world_name, name = %record.template_name,
                    tag = ?record.tag, "Spawned instance NPC from DB"
                );
                log_spawn_behaviour(space_mgr, npc_id);
                count += 1;
            }
            Err(e) => {
                tracing::warn!(
                    spawn_id = record.spawn_id, name = %record.template_name,
                    "Failed to spawn instance NPC: {e}"
                );
            }
        }
    }
    tracing::Span::current().record("spawned", count);
    // `cover.coverage` for the new instance, now that its NPCs exist.
    space_mgr.log_cover_coverage(space_id);
    count
}

/// The resolved behaviour of a freshly spawned NPC, so "why does this NPC act
/// like that" is one query instead of a seed read: an `aggression` other than
/// 1 (HOSTILE) means it will never notice a player on its own; `use_cover = false` means the
/// loaded cover nodes are irrelevant to it; `respawn_secs = None` is one-shot.
fn log_spawn_behaviour(space_mgr: &mut SpaceManager, npc_id: u32) {
    // NA02: a spawn that passes `is_point_valid` below but fails
    // `find_path`'s tight start box (audit S9) — WARN, once per spawn id.
    crate::cell::service::npc_ai::detectors::spawn::check_spawn(space_mgr, npc_id);
    let Some(e) = space_mgr.get_entity(npc_id) else {
        return;
    };
    // NA44: which world and instance, what it fights with, and whether each
    // ability can animate (`event_set_ids` 0 = no fire animation).
    let ability_ids = super::npc_identity::sorted_ability_ids(e);
    let event_set_ids = super::npc_identity::ability_event_set_ids(space_mgr, &ability_ids);
    let world = space_mgr.get_entity_world_name(npc_id).unwrap_or_default();
    tracing::debug!(
        target: "spawner.npc_behaviour",
        npc_id,
        world = %world,
        space_id = space_mgr.get_entity_space_id(npc_id).unwrap_or(0),
        npc_name = e.npc_name.as_deref().unwrap_or(""),
        tag = e.tag.as_deref().unwrap_or(""),
        template_id = e.template_id.unwrap_or(0),
        spawn_id = e.spawn_id.unwrap_or(0),
        x = e.position.x,
        y = e.position.y,
        z = e.position.z,
        spawn_yaw_rad = e.direction.y,
        on_navmesh = space_mgr.is_position_valid(npc_id, &e.position),
        navmesh_loaded = space_mgr.space_has_navmesh(npc_id),
        ground_y = ?space_mgr.get_navmesh_height(npc_id, e.position.x, e.position.y, e.position.z),
        level = e.level,
        faction = e.faction,
        // Effective level toward players (1 = hostile, NA13), whether it is
        // an override, and the radius the Idle scan uses.
        aggression = crate::cell::combat::aggression_toward_players(e).level(),
        aggression_override = ?e.aggro.override_level.map(|l| l.level()),
        aggro_radius = crate::cell::combat::aggro_radius(e),
        assist_radius = crate::cell::combat::assist_radius(e),
        use_cover = e.use_cover,
        is_stationary = e.is_stationary,
        move_speed = e.move_speed,
        respawn_secs = ?e.respawn_secs,
        follow_min_distance = e.follow_min_distance,
        follow_max_distance = e.follow_max_distance,
        patrol_len = e.patrol_path.len(),
        wander_radius = e.wander_radius,
        interaction_flags = e.interaction_type_flags,
        loot_table_id = ?e.loot_table_id,
        ability_ids = ?ability_ids,
        event_set_ids = ?event_set_ids,
        weapon_visual = %super::npc_identity::weapon_visual(e),
        "NPC spawned -- resolved behaviour"
    );
}

#[cfg(test)]
mod respawn_secs_tests {
    use super::normalize_respawn_secs;

    /// Positive values pass through unchanged. The DB CHECK constraint
    /// already enforces this at the boundary; the runtime fallback is
    /// belt-and-suspenders.
    #[test]
    fn positive_values_pass_through() {
        assert_eq!(normalize_respawn_secs(Some(1)), Some(1));
        assert_eq!(normalize_respawn_secs(Some(30)), Some(30));
        assert_eq!(
            normalize_respawn_secs(Some(i32::MAX)),
            Some(i32::MAX as u32)
        );
    }

    /// Zero must downgrade to None, NOT to `Some(0)`. `Some(0)` would
    /// stamp `respawn_at = now + 0s`, which the next 1 Hz tick would
    /// immediately consume — the corpse would respawn the same tick it
    /// died, looking like the kill never happened.
    #[test]
    fn zero_downgrades_to_none() {
        assert_eq!(normalize_respawn_secs(Some(0)), None);
    }

    /// Negative values are nonsensical for a duration. Downgrading to
    /// None matches the zero case so a misconfigured row is treated as
    /// "no respawn" instead of crashing on the `as u32` cast (which
    /// would wrap to a huge positive number and effectively make the
    /// mob never respawn anyway, but loudly).
    #[test]
    fn negative_values_downgrade_to_none() {
        assert_eq!(normalize_respawn_secs(Some(-1)), None);
        assert_eq!(normalize_respawn_secs(Some(i32::MIN)), None);
    }

    /// NULL in the DB (None) is the canonical "no respawn" signal and
    /// must round-trip cleanly.
    #[test]
    fn none_passes_through() {
        assert_eq!(normalize_respawn_secs(None), None);
    }
}
