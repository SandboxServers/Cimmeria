//! NA11: a seeded spawn a little off the navmesh floor is moved onto it, and
//! the moved point is what the leash and the respawn tick return to.

use cimmeria_entity::navigation::NavMesh;

use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::SpawnRecord;

const FIXTURE: &str = "../../data/spaces/castle_cellblock.nav";

/// The Cellblock guard's seeded XZ; the floor under it is at ~68.6.
const GUARD_X: f32 = -289.465;
const GUARD_Z: f32 = -154.276;
const GUARD_FLOOR_Y: f32 = 68.6;

/// Castle_CellBlock as a startup space with the real navmesh, or `None`
/// without the fixture.
fn cellblock() -> Option<SpaceManager> {
    let path = std::path::Path::new(FIXTURE);
    if !path.exists() {
        return None;
    }
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    let space_id = mgr.find_or_create_space("Castle_CellBlock").unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh =
        Some(NavMesh::load(path).expect("load castle_cellblock.nav"));
    Some(mgr)
}

fn guard_record(y: f32) -> SpawnRecord {
    SpawnRecord {
        spawn_id: 1,
        world_name: "Castle_CellBlock".to_string(),
        x: GUARD_X,
        y,
        z: GUARD_Z,
        heading: 0.0,
        tag: None,
        template_id: 15,
        template_name: "Cellblock Guard".to_string(),
        class: "mob".to_string(),
        static_mesh: None,
        body_set: "BS_HumanMale.BS_HumanMale".to_string(),
        components: None,
        flags: 0,
        interaction_type: 0,
        event_set_id: None,
        level: Some(1),
        alignment: Some(0),
        faction: Some(10),
        name_id: None,
        speaker_id: None,
        static_interaction_sets: vec![],
        has_dynamic_properties: true,
        loot_table_id: None,
        is_stationary: false,
        ability_ids: vec![],
        respawn_secs: None,
        leash_distance: None,
        aggro_radius: None,
        assist_radius: None,
        aggression_override: None,
        patrol_path: vec![],
        patrol_point_delay_secs: 2.0,
        wander_radius: 0.0,
        wander_min_dwell_secs: 3.0,
        wander_max_dwell_secs: 8.0,
        follow_min_distance: 2.0,
        follow_max_distance: 5.0,
        move_speed: 0.6,
    }
}

/// Spawn `record` and return `(position.y, spawn_position.y)`.
fn spawned_y(mgr: &mut SpaceManager, record: &SpawnRecord) -> (f32, f32) {
    let id = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(id, record).unwrap();
    let e = mgr.get_entity(id).unwrap();
    (e.position.y, e.spawn_position.unwrap().y)
}

/// **S9 regression guard.** A guard seeded 1 u over its floor stands on the
/// floor, and its `spawn_position` (the leash and respawn target) is the
/// same grounded point. Unsnapped, the start-polygon lookup's ±0.5 box
/// misses it and the NPC can never path.
#[test]
fn a_spawn_seeded_just_above_the_floor_is_grounded() {
    let Some(mut mgr) = cellblock() else { return };
    let (y, spawn_y) = spawned_y(&mut mgr, &guard_record(GUARD_FLOOR_Y + 1.0));
    assert!(
        (y - GUARD_FLOOR_Y).abs() < 0.1,
        "the guard must stand on the floor, got Y {y}"
    );
    assert_eq!(
        y, spawn_y,
        "the leash and respawn point must be the grounded spawn, not the seed"
    );
}

/// Outside the `is_point_valid` band the seed is kept as authored, and
/// props keep their authored height anywhere.
#[test]
fn far_off_seeds_and_props_keep_their_authored_y() {
    let Some(mut mgr) = cellblock() else { return };

    let high = GUARD_FLOOR_Y + 6.0;
    let (y, spawn_y) = spawned_y(&mut mgr, &guard_record(high));
    assert_eq!((y, spawn_y), (high, high), "6 u up is outside the band");

    let mut prop = guard_record(GUARD_FLOOR_Y + 1.0);
    prop.static_mesh = Some("CA-Props.CA-GuardCorpse02".to_string());
    prop.class = "spawnable".to_string();
    let (y, _) = spawned_y(&mut mgr, &prop);
    assert_eq!(y, GUARD_FLOOR_Y + 1.0, "a prop keeps its authored height");

    let mut sentry = guard_record(GUARD_FLOOR_Y + 1.0);
    sentry.is_stationary = true;
    let (y, _) = spawned_y(&mut mgr, &sentry);
    assert_eq!(y, GUARD_FLOOR_Y + 1.0, "a stationary NPC keeps its height");
}
