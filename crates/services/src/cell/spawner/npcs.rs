//! Database-driven NPC spawn records.
//!
//! `load_spawns_from_db` materializes the join of `resources.spawnlist` +
//! `resources.entity_templates` + `resources.worlds` into one `SpawnRecord`
//! per row. Populating spaces from those records is `SpaceManager` work:
//! `spawn_npcs_from_records` and `spawn_instance_npcs_from_records` live in
//! `cell::space_manager` (`npc_population.rs`).

use sqlx::PgPool;

// The record is a Base<->Cell message payload, so it lives in the wire
// contract; re-exported here so `spawner::SpawnRecord` keeps resolving.
pub use cimmeria_wire::cell::spawn_record::{class_id_for_class, SpawnRecord};

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
