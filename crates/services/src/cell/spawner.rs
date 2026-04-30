//! NPC spawn system for the CellService.
//!
//! Loads spawn data from the database (`resources.spawnlist` joined with
//! `resources.entity_templates` and `resources.worlds`) and populates world
//! spaces with NPC entities at the correct positions with full template data.
//!
//! Reference: `python/base/SGWSpawnSet.py`, `python/cell/SGWMob.py`,
//!            `python/cell/SGWSpawnableEntity.py`

use sqlx::PgPool;

use super::space_manager::SpaceManager;

// ── Mission definition cache ─────────────────────────────────────────────────

/// Cached mission definition: first step + its objectives.
///
/// Loaded at startup from `resources.mission_steps` + `resources.mission_objectives`
/// so that `AcceptMission` content actions can look up step/objective data without
/// per-action DB queries.
#[derive(Debug, Clone)]
pub struct MissionDefEntry {
    pub step_id: i32,
    pub objectives: Vec<MissionObjectiveDef>,
}

/// A single objective within a mission step.
#[derive(Debug, Clone)]
pub struct MissionObjectiveDef {
    pub objective_id: i32,
    pub is_hidden: bool,
    pub is_optional: bool,
}

/// Load mission definitions (first step + objectives) from the database.
///
/// Maps `mission_id → MissionDefEntry` for all missions that have at least one step.
/// Only loads the first step (lowest `index`) per mission, matching the behavior
/// of `AcceptMission` which starts at step 0.
pub async fn load_mission_defs(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, MissionDefEntry>, sqlx::Error> {
    use sqlx::Row;

    // Get the first step per mission (lowest index)
    let step_rows = sqlx::query(
        "SELECT DISTINCT ON (mission_id) mission_id, step_id \
         FROM resources.mission_steps \
         ORDER BY mission_id, index ASC"
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(step_rows.len());
    for r in &step_rows {
        let mission_id: i32 = r.get("mission_id");
        let step_id: i32 = r.get("step_id");
        map.insert(mission_id, MissionDefEntry {
            step_id,
            objectives: Vec::new(),
        });
    }

    // Load objectives for all steps we just loaded
    let step_ids: Vec<i32> = map.values().map(|e| e.step_id).collect();
    if !step_ids.is_empty() {
        let obj_rows = sqlx::query(
            "SELECT step_id, objective_id, is_hidden, is_optional \
             FROM resources.mission_objectives \
             WHERE step_id = ANY($1)"
        )
        .bind(&step_ids)
        .fetch_all(pool)
        .await?;

        // Build a step_id → objectives lookup
        let mut obj_by_step: std::collections::HashMap<i32, Vec<MissionObjectiveDef>> =
            std::collections::HashMap::new();
        for r in &obj_rows {
            let step_id: i32 = r.get("step_id");
            let obj = MissionObjectiveDef {
                objective_id: r.get("objective_id"),
                is_hidden: r.get("is_hidden"),
                is_optional: r.get("is_optional"),
            };
            obj_by_step.entry(step_id).or_default().push(obj);
        }

        // Attach objectives to their mission entries
        for entry in map.values_mut() {
            if let Some(objs) = obj_by_step.remove(&entry.step_id) {
                entry.objectives = objs;
            }
        }
    }

    tracing::info!(count = map.len(), "Loaded mission_defs cache");
    Ok(map)
}

/// Load step objectives for all steps from the database.
///
/// Maps `step_id → Vec<MissionObjectiveDef>` so that `AdvanceStep` can
/// look up the objectives for a new step without per-action DB queries.
pub async fn load_step_objectives(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, Vec<MissionObjectiveDef>>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT step_id, objective_id, is_hidden, is_optional \
         FROM resources.mission_objectives ORDER BY step_id, objective_id"
    )
    .fetch_all(pool)
    .await?;

    let mut map: std::collections::HashMap<i32, Vec<MissionObjectiveDef>> =
        std::collections::HashMap::new();
    for r in &rows {
        let step_id: i32 = r.get("step_id");
        let obj = MissionObjectiveDef {
            objective_id: r.get("objective_id"),
            is_hidden: r.get("is_hidden"),
            is_optional: r.get("is_optional"),
        };
        map.entry(step_id).or_default().push(obj);
    }

    tracing::info!(steps = map.len(), "Loaded step_objectives cache");
    Ok(map)
}

// ── Dialog set map cache ─────────────────────────────────────────────────────

/// Cached row from `resources.dialog_set_maps`, used by `add_dialog_set` content actions.
#[derive(Debug, Clone)]
pub struct DialogSetMapEntry {
    pub dialog_id: i32,
    pub interaction_flags: i64,
}

/// Load the `dialog_set_maps` lookup table from the database.
///
/// Maps `dialog_set_map_id → (dialog_id, interaction_flags)` so that
/// `add_dialog_set` actions can resolve at runtime without per-action DB queries.
pub async fn load_dialog_set_maps(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, DialogSetMapEntry>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT dialog_set_map_id, dialog_id, interaction_flags \
         FROM resources.dialog_set_maps"
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in &rows {
        let id: i32 = r.get("dialog_set_map_id");
        let dialog_id: Option<i32> = r.get("dialog_id");
        let interaction_flags: i64 = r.get("interaction_flags");
        if let Some(dialog_id) = dialog_id {
            map.insert(id, DialogSetMapEntry { dialog_id, interaction_flags });
        }
    }

    tracing::info!(count = map.len(), "Loaded dialog_set_maps cache");
    Ok(map)
}

// ── Database-driven spawning ─────────────────────────────────────────────────

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
}

/// Map the DB `entity_templates.class` column to the wire class_id.
///
/// The class_id is the entity type index from `entities/entities.xml`:
///   0 = SGWSpawnableEntity, 1 = SGWBeing, 2 = SGWPlayer, 3 = SGWGmPlayer,
///   4 = SGWMob, 5 = SGWPet, 6 = SGWDuelMarker, 7 = SGWBlackMarket
pub fn class_id_for_class(class: &str) -> u8 {
    match class {
        "spawnable" => 0x00, // SGWSpawnableEntity
        "being"     => 0x01, // SGWBeing
        "mob"       => 0x04, // SGWMob
        _           => 0x04, // Default to SGWMob
    }
}

/// Load all spawn records from the database.
///
/// Joins `resources.spawnlist` with `resources.entity_templates` and
/// `resources.worlds` to get position, template data, and world name
/// in a single query.
pub async fn load_spawns_from_db(pool: &PgPool) -> Result<Vec<SpawnRecord>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT s.spawn_id, w.world AS world_name, s.x, s.y, s.z, s.heading, s.tag, \
               t.template_id, t.template_name, t.class, t.static_mesh, t.body_set, \
               t.components, t.flags, t.interaction_type, t.event_set_id, t.level, \
               t.alignment, t.faction, t.name_id, t.speaker_id, \
               t.static_interaction_sets, t.has_dynamic_properties, \
               t.loot_table_id \
        FROM resources.spawnlist s \
        JOIN resources.entity_templates t ON s.template_id = t.template_id \
        JOIN resources.worlds w ON s.world_id = w.world_id \
        ORDER BY s.spawn_id"
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

// ── Respawner definitions ────────────────────────────────────────────────────

/// A respawner location loaded from the database.
///
/// Players see a list of these in the Defeat Window (onBeginAidWait).
/// The client sends back the chosen `respawner_id` via `callForAid`.
#[derive(Debug, Clone)]
pub struct RespawnerDef {
    pub respawner_id: i32,
    pub world_name: String,
    pub name: String,
    pub pos: [f32; 3],
}

/// Load respawner definitions from the database.
///
/// Joins `resources.respawners` with `resources.worlds` to get world names.
pub async fn load_respawners(
    pool: &PgPool,
) -> Result<Vec<RespawnerDef>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT r.respawner_id, w.world AS world_name, r.name, \
                r.pos_x, r.pos_y, r.pos_z \
         FROM resources.respawners r \
         JOIN resources.worlds w ON w.world_id = r.world_id"
    )
    .fetch_all(pool)
    .await?;

    let respawners: Vec<RespawnerDef> = rows.iter().map(|r| {
        RespawnerDef {
            respawner_id: r.get("respawner_id"),
            world_name: r.get("world_name"),
            name: r.get("name"),
            pos: [
                r.get::<f32, _>("pos_x"),
                r.get::<f32, _>("pos_y"),
                r.get::<f32, _>("pos_z"),
            ],
        }
    }).collect();

    tracing::info!(count = respawners.len(), "Loaded respawner definitions");
    Ok(respawners)
}

// ── Stargate destination cache ───────────────────────────────────────────────

/// Cached stargate destination from `resources.stargates` + `resources.worlds`.
#[derive(Debug, Clone)]
pub struct StargateEntry {
    pub world_name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
}

/// Load stargate destinations from the database.
///
/// Maps `stargate_id → StargateEntry` for gate travel lookups.
pub async fn load_stargates(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, StargateEntry>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT s.stargate_id, w.world AS world_name, \
                s.x_pos, s.y_pos, s.z_pos, s.yaw \
         FROM resources.stargates s \
         JOIN resources.worlds w ON s.world_id = w.world_id"
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in &rows {
        let id: i32 = r.get("stargate_id");
        map.insert(id, StargateEntry {
            world_name: r.get("world_name"),
            x: r.get::<f64, _>("x_pos") as f32,
            y: r.get::<f64, _>("y_pos") as f32,
            z: r.get::<f64, _>("z_pos") as f32,
            yaw: r.get::<f64, _>("yaw") as f32,
        });
    }

    tracing::info!(count = map.len(), "Loaded stargates cache");
    Ok(map)
}

// ── Generic region loading ──────────────────────────────────────────────────

/// Intermediate structure for loading region data before runtime ID assignment.
#[derive(Debug, Clone)]
pub struct RegionLoadData {
    pub set_id: i32,
    pub name: String,
    pub world_name: String,
    pub radius: f32,
    pub height: f32,
    pub flags: i32,
    pub points: Vec<[f32; 3]>,
}

/// Load generic regions from the database.
///
/// Queries `resources.point_sets` (type='AreaSet') joined with `resources.worlds`
/// for region metadata, then `resources.point_set_points` for polygon vertices.
///
/// Also applies the Python `GenericRegion.workaround()` — single-point cylinder
/// regions (radius > 0, 1 point) are expanded to a 4-point bounding box so the
/// client can hit-test them.
///
/// Reference: `python/cell/GenericRegion.py:GenericRegionManager.load()`
pub async fn load_regions_from_db(
    pool: &PgPool,
) -> Result<Vec<RegionLoadData>, sqlx::Error> {
    use sqlx::Row;

    let region_rows = sqlx::query(
        "SELECT ps.set_id, ps.name, ps.radius, ps.height, ps.flags, \
                w.world AS world_name \
         FROM resources.point_sets ps \
         JOIN resources.worlds w ON ps.world_id = w.world_id \
         WHERE ps.type = 'AreaSet' \
         ORDER BY ps.set_id"
    )
    .fetch_all(pool)
    .await?;

    if region_rows.is_empty() {
        return Ok(vec![]);
    }

    // Collect all set_ids to batch-fetch points
    let set_ids: Vec<i32> = region_rows.iter()
        .map(|r| r.get::<i32, _>("set_id"))
        .collect();

    let point_rows = sqlx::query(
        "SELECT set_id, x, y, z \
         FROM resources.point_set_points \
         WHERE set_id = ANY($1) \
         ORDER BY set_id, point_id"
    )
    .bind(&set_ids)
    .fetch_all(pool)
    .await?;

    // Group points by set_id
    let mut points_by_set: std::collections::HashMap<i32, Vec<[f32; 3]>> =
        std::collections::HashMap::new();
    for r in &point_rows {
        let set_id: i32 = r.get("set_id");
        let x: f32 = r.get("x");
        let y: f32 = r.get("y");
        let z: f32 = r.get("z");
        points_by_set.entry(set_id)
            .or_default()
            .push([x, y, z]);
    }

    let mut regions = Vec::with_capacity(region_rows.len());
    for r in &region_rows {
        let set_id: i32 = r.get("set_id");
        let radius: f32 = r.try_get::<f32, _>("radius").unwrap_or(0.0);
        let height: f32 = r.try_get::<f32, _>("height").unwrap_or(0.0);
        let mut points = points_by_set.remove(&set_id).unwrap_or_default();

        // Python workaround: single-point cylinder → 4-point bounding box
        // Reference: GenericRegion.workaround() — if 1 point and radius > 0,
        // expand to an axis-aligned box centered on the point.
        if points.len() == 1 && radius > 0.0 {
            let [px, py, pz] = points[0];
            let r = radius;
            let h = height;
            points = vec![
                [px - r, py,     pz - r],
                [px - r, py,     pz + r],
                [px + r, py,     pz + r],
                [px + r, py + h, pz - r],
            ];
        }

        regions.push(RegionLoadData {
            set_id,
            name: r.get("name"),
            world_name: r.get("world_name"),
            radius: radius as f32,
            height: height as f32,
            flags: r.get("flags"),
            points,
        });
    }

    tracing::info!(count = regions.len(), "Loaded generic regions from database");
    Ok(regions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager_with_worlds() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
    <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="2400" MinY="0" MaxY="2400" />
    <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
</Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" />
    <Space WorldName="Castle" />
</Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();
        mgr
    }

    fn make_test_record(world_name: &str, tag: Option<&str>, class: &str) -> SpawnRecord {
        SpawnRecord {
            spawn_id: 1,
            world_name: world_name.to_string(),
            x: 10.0, y: 0.0, z: 20.0,
            heading: 1.57,
            tag: tag.map(|t| t.to_string()),
            template_id: 14,
            template_name: "Test Entity".to_string(),
            class: class.to_string(),
            static_mesh: Some("Props.TestMesh".to_string()),
            body_set: "GLB_Components.WorldObject_Small".to_string(),
            components: None,
            flags: 0,
            interaction_type: 0,
            event_set_id: None,
            level: Some(5),
            alignment: Some(0),
            faction: Some(1),
            name_id: Some(7031),
            speaker_id: None,
            static_interaction_sets: vec![],
            has_dynamic_properties: true,
            loot_table_id: None,
        }
    }

    #[test]
    fn npc_ids_are_sequential() {
        let mut mgr = make_manager_with_worlds();
        let id1 = mgr.allocate_npc_id();
        let id2 = mgr.allocate_npc_id();
        assert_eq!(id1, 100_000);
        assert_eq!(id2, 100_001);
    }

    #[test]
    fn spawn_in_unknown_world_skipped() {
        let mut mgr = SpaceManager::new(1);
        let npc_id = mgr.allocate_npc_id();
        let result = mgr.spawn_npc(npc_id, "Nonexistent", [0.0; 3], [0.0; 3]);
        assert!(result.is_err());
    }

    #[test]
    fn class_id_mapping() {
        assert_eq!(class_id_for_class("spawnable"), 0x00);
        assert_eq!(class_id_for_class("being"), 0x01);
        assert_eq!(class_id_for_class("mob"), 0x04);
        assert_eq!(class_id_for_class("unknown"), 0x04); // fallback
    }

    #[test]
    fn spawn_npc_from_record_sets_template_fields() {
        let mut mgr = make_manager_with_worlds();
        let record = make_test_record("Agnos", Some("TestTag"), "being");

        let npc_id = mgr.allocate_npc_id();
        let space_id = mgr.spawn_npc_from_record(npc_id, &record).unwrap();
        assert!(space_id > 0);

        let npc = mgr.get_entity(npc_id).unwrap();
        assert_eq!(npc.class_id, 0x01); // SGWBeing
        assert_eq!(npc.template_id, Some(14));
        assert_eq!(npc.tag.as_deref(), Some("TestTag"));
        assert_eq!(npc.name_id, Some(7031));
        assert_eq!(npc.faction, 1);
        assert_eq!(npc.level, 5);
        assert_eq!(npc.npc_name.as_deref(), Some("Test Entity"));
        assert!(npc.has_dynamic_properties);
    }

    #[test]
    fn spawn_from_records_only_in_startup_spaces() {
        let mut mgr = make_manager_with_worlds();
        let records = vec![
            make_test_record("Agnos", Some("Tag1"), "mob"),
            make_test_record("Castle_CellBlock", Some("Tag2"), "being"), // instanced, not loaded
        ];

        let count = spawn_npcs_from_records(&records, &mut mgr);
        assert_eq!(count, 1); // Only Agnos (startup), not Castle_CellBlock (instanced)
    }

    #[test]
    fn spawn_instance_npcs_filters_by_world() {
        let mut mgr = make_manager_with_worlds();
        // Create the instanced space first — returns a new space_id each time
        let space_id = mgr.find_or_create_space("Castle_CellBlock").unwrap();

        let records = vec![
            make_test_record("Castle_CellBlock", Some("Tag1"), "being"),
            make_test_record("Castle_CellBlock", Some("Tag2"), "spawnable"),
            make_test_record("Agnos", Some("Tag3"), "mob"), // wrong world
        ];

        let count = spawn_instance_npcs_from_records(&records, "Castle_CellBlock", space_id, &mut mgr);
        assert_eq!(count, 2);
    }

    #[test]
    fn find_entity_by_tag_works() {
        let mut mgr = make_manager_with_worlds();
        let record = make_test_record("Agnos", Some("TestTag"), "being");
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record(npc_id, &record).unwrap();

        let found = mgr.find_entity_by_tag("Agnos", "TestTag");
        assert_eq!(found, Some(npc_id));

        let not_found = mgr.find_entity_by_tag("Agnos", "NonexistentTag");
        assert_eq!(not_found, None);
    }
}

// ── Ability + Effect definitions ─────────────────────────────────────────────

/// Event IDs for ability sequence lookups (from Atrea.enums).
pub const EVENT_ABILITY_BEGIN: i32 = 1000;
pub const EVENT_ABILITY_END: i32 = 1001;

/// Load the event set → sequence mapping from the database.
///
/// Joins `resources.event_sets_sequences` with `resources.sequences` to build
/// a lookup from `(event_set_id, event_id) → sequence_id`. This resolves the
/// correct KismetEventSetSeqID to send in `onSequence` calls.
///
/// The lookup chain: ability has event_set_id → event_sets_sequences join →
/// sequences table has (sequence_id, event_id). The client expects sequence_id,
/// NOT event_set_id.
pub async fn load_event_set_sequences(
    pool: &PgPool,
) -> Result<std::collections::HashMap<(i32, i32), i32>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT ess.event_set_id, s.sequence_id, s.event_id \
         FROM resources.event_sets_sequences ess \
         JOIN resources.sequences s ON s.sequence_id = ess.sequence_id"
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in &rows {
        let event_set_id: i32 = r.get("event_set_id");
        let sequence_id: i32 = r.get("sequence_id");
        let event_id: i32 = r.get("event_id");
        map.insert((event_set_id, event_id), sequence_id);
    }

    tracing::info!(count = map.len(), "Loaded event_set sequence mappings");
    Ok(map)
}

/// Load all ability definitions from `resources.abilities`.
pub async fn load_ability_defs(pool: &PgPool) -> Result<std::collections::HashMap<i32, cimmeria_entity::abilities::AbilityDef>, sqlx::Error> {
    let rows = sqlx::query_as::<_, AbilityRow>(
        "SELECT ability_id, name, cooldown, warmup, flags, is_ranged, \
         min_range, max_range, target_type_id, effect_ids, \
         required_ammo, event_set_id, velocity \
         FROM resources.abilities"
    )
    .fetch_all(pool)
    .await?;

    let mut defs = std::collections::HashMap::with_capacity(rows.len());
    for r in rows {
        defs.insert(r.ability_id, cimmeria_entity::abilities::AbilityDef {
            ability_id: r.ability_id,
            name: r.name,
            cooldown: r.cooldown,
            warmup: r.warmup,
            flags: r.flags as u32,
            is_ranged: r.is_ranged,
            min_range: r.min_range,
            max_range: r.max_range,
            target_type_id: r.target_type_id,
            effect_ids: r.effect_ids,
            moniker_ids: vec![],
            required_ammo: r.required_ammo,
            event_set_id: r.event_set_id,
            velocity: r.velocity,
        });
    }

    tracing::info!(count = defs.len(), "Loaded ability definitions");
    Ok(defs)
}

#[derive(sqlx::FromRow)]
struct AbilityRow {
    ability_id: i32,
    name: String,
    cooldown: f32,
    warmup: f32,
    flags: i32,
    is_ranged: bool,
    min_range: i32,
    max_range: i32,
    target_type_id: i32,
    effect_ids: Vec<i32>,
    required_ammo: i32,
    event_set_id: Option<i32>,
    velocity: f32,
}

/// Load all effect definitions from `resources.effects` + `resources.effect_nvps`.
pub async fn load_effect_defs(pool: &PgPool) -> Result<std::collections::HashMap<i32, cimmeria_entity::abilities::EffectDef>, sqlx::Error> {
    // Load effects
    let rows = sqlx::query_as::<_, EffectRow>(
        "SELECT effect_id, ability_id, delay, effect_sequence, event_set_id, script_name \
         FROM resources.effects"
    )
    .fetch_all(pool)
    .await?;

    let mut defs: std::collections::HashMap<i32, cimmeria_entity::abilities::EffectDef> =
        std::collections::HashMap::with_capacity(rows.len());
    for r in rows {
        defs.insert(r.effect_id, cimmeria_entity::abilities::EffectDef {
            effect_id: r.effect_id,
            ability_id: r.ability_id,
            delay: r.delay,
            effect_sequence: r.effect_sequence,
            event_set_id: r.event_set_id,
            script_name: r.script_name,
            params: std::collections::HashMap::new(),
        });
    }

    // Load NVPs and attach to effects
    let nvps = sqlx::query_as::<_, EffectNvpRow>(
        "SELECT effect_id, name, value FROM resources.effect_nvps"
    )
    .fetch_all(pool)
    .await?;

    for nvp in nvps {
        if let Some(effect) = defs.get_mut(&nvp.effect_id) {
            effect.params.insert(nvp.name, nvp.value);
        }
    }

    tracing::info!(count = defs.len(), "Loaded effect definitions");
    Ok(defs)
}

#[derive(sqlx::FromRow)]
struct EffectRow {
    effect_id: i32,
    ability_id: i32,
    delay: i32,
    effect_sequence: i32,
    event_set_id: Option<i32>,
    script_name: Option<String>,
}

#[derive(sqlx::FromRow)]
struct EffectNvpRow {
    effect_id: i32,
    name: String,
    value: String,
}

// ── Loot table cache ─────────────────────────────────────────────────────────

/// A single entry in a loot table, loaded from `resources.loot`.
#[derive(Debug, Clone)]
pub struct LootTableEntry {
    pub design_id: Option<i32>,
    pub min_quantity: i32,
    pub max_quantity: i32,
    pub probability: f32,
}

/// Load loot tables from the database.
///
/// Returns `loot_table_id → Vec<LootTableEntry>` so that loot generation at
/// NPC death can roll drops without per-kill DB queries.
///
/// Reference: `python/cell/interactions/Lootable.py:randomizeLoot()`
pub async fn load_loot_tables(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, Vec<LootTableEntry>>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT loot_table_id, design_id, min_quantity, max_quantity, probability \
         FROM resources.loot \
         ORDER BY loot_table_id, loot_id"
    )
    .fetch_all(pool)
    .await?;

    let mut map: std::collections::HashMap<i32, Vec<LootTableEntry>> =
        std::collections::HashMap::new();
    for r in &rows {
        let table_id: i32 = r.get("loot_table_id");
        let entry = LootTableEntry {
            design_id: r.get("design_id"),
            min_quantity: r.get("min_quantity"),
            max_quantity: r.get("max_quantity"),
            probability: r.get("probability"),
        };
        map.entry(table_id).or_default().push(entry);
    }

    tracing::info!(
        tables = map.len(),
        entries = map.values().map(|v| v.len()).sum::<usize>(),
        "Loaded loot tables"
    );
    Ok(map)
}

/// Load item → preferred container mappings from `resources.items.container_sets`.
///
/// The `container_sets` column is a PostgreSQL `integer[]`. We pick the first
/// element as the preferred container for runtime grants (mission items → 2,
/// weapons → 3, etc.). Items with an empty array are omitted — they default
/// to INV_Main (1) at the call site.
pub async fn load_item_containers(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, i32>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT item_id, container_sets[1] AS container_id \
         FROM resources.items \
         WHERE array_length(container_sets, 1) > 0",
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in &rows {
        let item_id: i32 = r.get("item_id");
        let container_id: i32 = r.get("container_id");
        map.insert(item_id, container_id);
    }

    tracing::info!(count = map.len(), "Loaded item container mappings");
    Ok(map)
}

/// Cached weapon stats for runtime item grants.
///
/// The content engine's `GrantItem` action seeds bandolier slots and AmmoSlot
/// stats from this cache so the client renders the correct empty magazine for
/// the new weapon. Mirrors the per-row clip_size + default_ammo_type that
/// `BANDOLIER_ITEMS_QUERY` reads for player_load.
#[derive(Debug, Clone, Copy)]
pub struct WeaponDef {
    pub clip_size: i32,
    /// 0-based EAmmoType index, matching the wire format used in
    /// `bandolier_items` and `onEntityProperty(AmmoTypeId)`.
    pub default_ammo_type: i32,
}

/// Load weapon stats (clip_size + default_ammo_type) from `resources.items`.
///
/// Skips items with `clip_size IS NULL` (non-weapons). The `default_ammo_type`
/// conversion mirrors `BANDOLIER_ITEMS_QUERY` exactly so that values seeded
/// here for runtime grants match the wire format the player_load path uses.
pub async fn load_item_defs(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, WeaponDef>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT item_id, clip_size, \
                CASE WHEN default_ammo_type IS NULL THEN 0 \
                     ELSE array_position(enum_range(NULL::resources.\"EAmmoType\"), default_ammo_type) - 1 \
                END AS default_ammo_type_id \
         FROM resources.items \
         WHERE clip_size IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in &rows {
        let item_id: i32 = r.get("item_id");
        let clip_size: i32 = r.get("clip_size");
        let default_ammo_type: i32 = r.get("default_ammo_type_id");
        map.insert(item_id, WeaponDef { clip_size, default_ammo_type });
    }

    tracing::info!(count = map.len(), "Loaded weapon defs");
    Ok(map)
}
