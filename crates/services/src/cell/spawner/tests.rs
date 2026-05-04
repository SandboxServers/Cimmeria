use super::super::space_manager::SpaceManager;
use super::*;

fn make_manager_with_worlds() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
    <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="2400" MinY="0" MaxY="2400" />
    <Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
</Spaces>"#;
    let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
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
        x: 10.0,
        y: 0.0,
        z: 20.0,
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
        is_stationary: false,
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

    // Use the NPC itself as the source entity -- it lives in Agnos, so the
    // search runs against that single space.
    let found = mgr.find_entity_by_tag(npc_id, "TestTag");
    assert_eq!(found, Some(npc_id));

    let not_found = mgr.find_entity_by_tag(npc_id, "NonexistentTag");
    assert_eq!(not_found, None);
}

/// Live-DB sanity tests for the spawner loader functions.
///
/// Each loader is a thin sqlx query that maps `resources.*` rows into a
/// runtime cache. The tests below run each loader against the seeded DB
/// and assert (a) it doesn't error, (b) the cache is non-empty (the
/// seeded resources schema has rows for every loaded table), (c) sample
/// rows have plausible shape. These are byte-cheap regression guards
/// for column renames, type drift, and JOIN breakage that the rest of
/// the test suite wouldn't catch — sqlx surfaces those as `Err` from
/// the loader.
mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    #[tokio::test]
    async fn load_loot_tables_returns_seeded_data_with_non_empty_entries() {
        let pool = require_db_or_skip!();
        let map = load_loot_tables(&pool)
            .await
            .expect("load_loot_tables must succeed against seeded DB");
        assert!(!map.is_empty(), "seeded resources.loot has rows");
        for (id, entries) in &map {
            assert!(
                !entries.is_empty(),
                "loot_table {id} present in map but has no entries",
            );
        }
    }

    #[tokio::test]
    async fn load_item_defs_returns_seeded_weapons_with_clip_size_columns() {
        let pool = require_db_or_skip!();
        let map = load_item_defs(&pool)
            .await
            .expect("load_item_defs must succeed against seeded DB");
        // The loader filters `WHERE clip_size IS NOT NULL` — items with a
        // populated clip_size column. The actual values include 0 (placeholder
        // entries for non-loaded weapon templates), so we don't assert
        // positivity here. The load_item_defs invariant we CAN pin is that
        // the cache is non-empty (the seed has weapons) and that every cached
        // entry's clip_size is non-negative (clip_size is i32 in the
        // resources.items column; negative would indicate a sign-extend bug).
        assert!(
            !map.is_empty(),
            "seeded resources.items has weapons with clip_size populated"
        );
        for (item_id, def) in &map {
            assert!(
                def.clip_size >= 0,
                "item {item_id} surfaced from load_item_defs with negative clip_size {}",
                def.clip_size
            );
        }
    }

    #[tokio::test]
    async fn load_item_containers_projects_first_element_of_container_sets() {
        let pool = require_db_or_skip!();
        // Pick a seeded item with a non-empty `container_sets` and remember
        // its first element. The loader's `container_sets[1]` projection
        // (PostgreSQL is 1-indexed) must round-trip that exact value into
        // the cached HashMap. A regression that swaps to `container_sets[2]`
        // or aggregates the array would fail this assertion.
        let row: Option<(i32, i32)> = sqlx::query_as(
            "SELECT item_id, container_sets[1] AS first_container \
             FROM resources.items \
             WHERE array_length(container_sets, 1) > 0 \
             ORDER BY item_id \
             LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .expect("seed query must succeed");
        let (probe_item_id, expected_first) =
            row.expect("seed must have at least one row with non-empty container_sets");

        let map = load_item_containers(&pool)
            .await
            .expect("load_item_containers must succeed against seeded DB");
        assert!(!map.is_empty(), "non-empty seed → non-empty cache");
        assert_eq!(
            map.get(&probe_item_id).copied(),
            Some(expected_first),
            "loader must project the FIRST element of container_sets"
        );
    }

    #[tokio::test]
    async fn load_respawners_returns_seeded_rows_with_world_names() {
        let pool = require_db_or_skip!();
        let respawners = load_respawners(&pool)
            .await
            .expect("load_respawners must succeed");
        assert!(!respawners.is_empty());
        for r in &respawners {
            assert!(
                !r.world_name.is_empty(),
                "respawner {} has empty world_name — JOIN to resources.worlds broke",
                r.respawner_id
            );
        }
    }

    #[tokio::test]
    async fn load_spawns_returns_records_with_resolved_world_names() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");
        assert!(!records.is_empty(), "seeded resources.spawnlist has rows");
        for r in &records {
            assert!(
                !r.world_name.is_empty(),
                "spawn {} has empty world_name — JOIN to resources.worlds broke",
                r.spawn_id
            );
            assert!(
                !r.template_name.is_empty(),
                "spawn {} has empty template_name — JOIN to entity_templates broke",
                r.spawn_id
            );
        }
    }

    #[tokio::test]
    async fn load_mission_defs_only_includes_missions_with_a_step() {
        let pool = require_db_or_skip!();
        let map = load_mission_defs(&pool)
            .await
            .expect("load_mission_defs must succeed");
        assert!(!map.is_empty(), "seeded mission_steps has rows");
        for (mission_id, entry) in &map {
            assert!(
                entry.step_id > 0,
                "mission {mission_id} has non-positive step_id {}",
                entry.step_id
            );
        }
    }

    #[tokio::test]
    async fn load_step_objectives_groups_by_step_id() {
        let pool = require_db_or_skip!();
        let map = load_step_objectives(&pool)
            .await
            .expect("load_step_objectives must succeed");
        assert!(!map.is_empty());
        for (step_id, objs) in &map {
            assert!(
                !objs.is_empty(),
                "step {step_id} present in map with no objectives — should have been filtered out"
            );
        }
    }

    #[tokio::test]
    async fn load_dialog_set_maps_drops_rows_with_null_dialog_id() {
        let pool = require_db_or_skip!();
        let map = load_dialog_set_maps(&pool)
            .await
            .expect("load_dialog_set_maps must succeed");
        assert!(
            !map.is_empty(),
            "seeded dialog_set_maps has rows with non-null dialog_id"
        );
        // The loader explicitly drops rows where `dialog_id IS NULL` —
        // every cached entry must have a positive dialog_id by construction.
        for (set_map_id, entry) in &map {
            assert!(
                entry.dialog_id > 0,
                "dialog_set_map_id {set_map_id} surfaced with non-positive dialog_id {}",
                entry.dialog_id
            );
        }
        // Direct invariant pin: any row with NULL dialog_id in the seeded
        // table must be absent from the loaded cache. Catches a regression
        // that flips the `if let Some(dialog_id)` to a default-on-None.
        let null_set_map_id: Option<i32> = sqlx::query_scalar(
            "SELECT dialog_set_map_id FROM resources.dialog_set_maps WHERE dialog_id IS NULL LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .expect("query must succeed");
        if let Some(id) = null_set_map_id {
            assert!(
                !map.contains_key(&id),
                "dialog_set_map_id {id} has NULL dialog_id in DB but surfaced in the cache"
            );
        }
    }

    #[tokio::test]
    async fn load_stargates_resolves_world_join() {
        let pool = require_db_or_skip!();
        let map = load_stargates(&pool)
            .await
            .expect("load_stargates must succeed");
        assert!(!map.is_empty());
        for (id, entry) in &map {
            assert!(
                !entry.world_name.is_empty(),
                "stargate {id} has empty world_name — JOIN to resources.worlds broke"
            );
        }
    }

    #[tokio::test]
    async fn load_regions_applies_single_point_cylinder_workaround() {
        let pool = require_db_or_skip!();
        // Find a seeded AreaSet that the workaround SHOULD expand:
        // type='AreaSet', radius > 0, exactly one row in point_set_points.
        // Without this query, a seed with no qualifying region would let
        // a workaround-removed regression slip through (the conditional
        // `if let Some(r) = expanded` would be skipped entirely).
        let probe: Option<(i32, f32)> = sqlx::query_as(
            "SELECT ps.set_id, ps.radius FROM resources.point_sets ps \
             JOIN ( \
               SELECT set_id, COUNT(*) AS pts FROM resources.point_set_points GROUP BY set_id \
             ) c ON c.set_id = ps.set_id \
             WHERE ps.type = 'AreaSet' AND ps.radius > 0 AND c.pts = 1 \
             ORDER BY ps.set_id LIMIT 1",
        )
        .fetch_optional(&pool)
        .await
        .expect("seed probe query must succeed");
        let (probe_set_id, probe_radius) = probe.expect(
            "seed must contain at least one type='AreaSet' single-point cylinder \
             (radius > 0, exactly one point) so the workaround test isn't vacuous",
        );

        let regions = load_regions_from_db(&pool)
            .await
            .expect("load_regions_from_db must succeed");
        assert!(!regions.is_empty());

        let expanded = regions
            .iter()
            .find(|r| r.set_id == probe_set_id)
            .expect("probe region must surface from load_regions_from_db");
        // GenericRegion.workaround(): single-point input + radius > 0 →
        // 4-point bounding box. If a refactor drops the workaround,
        // expanded.points.len() stays at 1 and this assertion fails.
        assert_eq!(
            expanded.points.len(),
            4,
            "workaround must expand single-point cylinder set_id={probe_set_id} to 4 points"
        );
        // 4-point box: opposing-corner x distance == 2*radius.
        let dx = (expanded.points[2][0] - expanded.points[0][0]).abs();
        let dz = (expanded.points[2][2] - expanded.points[0][2]).abs();
        assert!(
            (dx - 2.0 * probe_radius).abs() < 1e-3,
            "expanded region {probe_set_id} x-extent {dx} should be 2*radius {}",
            2.0 * probe_radius
        );
        assert!(
            (dz - 2.0 * probe_radius).abs() < 1e-3,
            "expanded region {probe_set_id} z-extent {dz} should be 2*radius {}",
            2.0 * probe_radius
        );
    }
}
