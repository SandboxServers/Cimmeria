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
        })
        .collect();

    Ok(records)
}

/// Spawn NPCs from DB records into all currently-loaded startup spaces.
///
/// Only spawns records whose `world_name` matches a space that already exists
/// in the SpaceManager (i.e., non-instanced startup spaces). Instanced spaces
/// are handled by `spawn_instance_npcs_from_records`.
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
    tracing::info!(count, "DB-driven NPC population spawned (startup spaces)");
    count
}

/// Spawn NPCs from DB records for a specific instanced world into a given space.
///
/// Called when a new instanced space is created for a player (e.g., Castle_CellBlock,
/// SGC_W1). Each instance gets its own set of NPCs. The `space_id` parameter is the
/// space that was just created — NPCs are spawned directly into it rather than going
/// through `find_or_create_space` (which would create yet another new instance).
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
    count
}
