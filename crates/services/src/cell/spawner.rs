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
               t.static_interaction_sets, t.has_dynamic_properties \
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

/// Spawn NPCs from DB records for a specific instanced world.
///
/// Called when an instanced space is first created (e.g., Castle_CellBlock, SGC_W1).
pub fn spawn_instance_npcs_from_records(
    records: &[SpawnRecord],
    world_name: &str,
    space_mgr: &mut SpaceManager,
) -> usize {
    let mut count = 0;
    for record in records {
        if record.world_name != world_name {
            continue;
        }
        let npc_id = space_mgr.allocate_npc_id();
        match space_mgr.spawn_npc_from_record(npc_id, record) {
            Ok(space_id) => {
                tracing::debug!(
                    npc_id, space_id, spawn_id = record.spawn_id,
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
        let x: f64 = r.get("x");
        let y: f64 = r.get("y");
        let z: f64 = r.get("z");
        points_by_set.entry(set_id)
            .or_default()
            .push([x as f32, y as f32, z as f32]);
    }

    let mut regions = Vec::with_capacity(region_rows.len());
    for r in &region_rows {
        let set_id: i32 = r.get("set_id");
        let radius: f64 = r.try_get::<f64, _>("radius").unwrap_or(0.0);
        let height: f64 = r.try_get::<f64, _>("height").unwrap_or(0.0);
        let mut points = points_by_set.remove(&set_id).unwrap_or_default();

        // Python workaround: single-point cylinder → 4-point bounding box
        // Reference: GenericRegion.workaround() — if 1 point and radius > 0,
        // expand to an axis-aligned box centered on the point.
        if points.len() == 1 && radius > 0.0 {
            let [px, py, pz] = points[0];
            let r = radius as f32;
            let h = height as f32;
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
        // Create the instanced space first
        mgr.find_or_create_space("Castle_CellBlock").unwrap();

        let records = vec![
            make_test_record("Castle_CellBlock", Some("Tag1"), "being"),
            make_test_record("Castle_CellBlock", Some("Tag2"), "spawnable"),
            make_test_record("Agnos", Some("Tag3"), "mob"), // wrong world
        ];

        let count = spawn_instance_npcs_from_records(&records, "Castle_CellBlock", &mut mgr);
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
