//! T-02a: the Tollana 568 cast, Copplemann (template 420) and the food-unit
//! prop (template 421), and their four spawns (19001-19004) in World 19.
//!
//! Every test reads the seed through the production loader or the DB, so none
//! passes with the seed files removed. The spawn coordinates are offline
//! navmesh candidates beside gate 4 (`spawnlist_tollana.sql`); the nav test
//! checks them against the shipped `tollana.nav`.

use cimmeria_common::Vector3;
use cimmeria_entity::interaction_flags::INT_MISSION_WORLD_OBJECT;
use cimmeria_entity::navigation::{NavMesh, PathStatus};

use crate::cell::spawner::load_spawn_templates;
use crate::test_support::require_db_or_skip;

/// World 19 `Tollana`.
const WORLD: i32 = 19;

/// The four seeded spawns: `(spawn id, template id, tag, [x, y, z])`. Copplemann
/// first, then the Core, Nest and Fungus food units.
const CAST: [(i32, i32, &str, [f32; 3]); 4] = [
    (19001, 420, "Tollana_Copplemann", [206.00, -0.58, -543.00]),
    (
        19002,
        421,
        "Tollana_FoodUnit_Core",
        [199.50, -0.93, -540.00],
    ),
    (
        19003,
        421,
        "Tollana_FoodUnit_Nest",
        [195.50, -1.01, -540.00],
    ),
    (
        19004,
        421,
        "Tollana_FoodUnit_Fungus",
        [197.50, -1.01, -536.54],
    ),
];

/// Gate 4 at Tollana (`stargates.sql`): the cast must stand within reach of it.
const GATE_4: [f32; 3] = [211.045, 0.606, -547.644];

/// Spawn one template into a fixture space beside a player, as a spawn row
/// would, and return the manager and the NPC's entity id.
fn spawned(
    record: &crate::cell::spawner::SpawnRecord,
) -> (crate::cell::space_manager::SpaceManager, /* npc */ u32) {
    const PLAYER: u32 = 1;
    let mut mgr = crate::test_support::make_space_manager_with_player(PLAYER);
    let mut record = record.clone();
    // The fixture space is `Agnos`; the spawn path takes no world from the
    // record beyond that name, as the Dakara guards do.
    record.world_name = "Agnos".to_string();
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(npc, &record)
        .unwrap_or_else(|e| panic!("template {} must spawn: {e}", record.template_id));
    (mgr, npc)
}

/// The Tollana mesh as the Dakara guards load theirs. A missing mesh panics.
fn tollana_mesh() -> NavMesh {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/spaces/tollana.nav");
    NavMesh::load(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn vector(p: [f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

/// Horizontal (x and z only) distance between two points.
fn horizontal(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The four seeded positions, read from the DB in spawn id order.
async fn seeded_points(pool: &sqlx::PgPool) -> Vec<[f32; 3]> {
    let rows: Vec<(f32, f32, f32)> = sqlx::query_as(
        "SELECT x, y, z FROM resources.spawnlist \
         WHERE spawn_id BETWEEN 19001 AND 19010 ORDER BY spawn_id",
    )
    .fetch_all(pool)
    .await
    .expect("cast position query must succeed");
    rows.into_iter().map(|(x, y, z)| [x, y, z]).collect()
}

/// Revert proof: drop the seed's `\ir` line for the templates, or change
/// Copplemann's `name_id`, and this names the field.
///
/// The loader's values for 420 and 421 match the frozen literals, and the
/// food-unit template spawns with `INT_MissionWorldObject` set, so the prop
/// is clickable from spawn.
#[tokio::test]
async fn live_db_tollana_templates_load_as_seeded() {
    let pool = require_db_or_skip!();
    let templates = load_spawn_templates(&pool)
        .await
        .expect("load_spawn_templates must succeed");
    let copp = templates.get(&420).expect("template 420 loads");
    let food = templates.get(&421).expect("template 421 loads");

    assert_eq!(
        (
            copp.class.as_str(),
            copp.interaction_type,
            copp.event_set_id,
            copp.flags,
            copp.name_id,
            copp.speaker_id,
            copp.has_dynamic_properties,
        ),
        ("mob", 0, Some(570), 0, Some(7035), Some(968), true),
        "template 420 (Copplemann)"
    );
    assert_eq!(
        copp.static_mesh, None,
        "template 420 draws a body, not a mesh"
    );
    assert_eq!(copp.body_set, "BS_HumanFemale.BS_HumanFemale");

    assert_eq!(
        (
            food.class.as_str(),
            food.interaction_type,
            food.event_set_id,
            food.flags,
            food.name_id,
            food.speaker_id,
            food.has_dynamic_properties,
        ),
        ("being", 1073741824, None, 4, Some(7558), None, true),
        "template 421 (food unit)"
    );
    assert_eq!(
        food.static_mesh.as_deref(),
        Some("Em-Props.EM-ViewScreen02"),
        "template 421 mesh"
    );
    assert_eq!(food.body_set, "GLB_Components.WorldObject_Small");

    let (mgr, npc) = spawned(food);
    let e = mgr.get_entity(npc).expect("spawned food unit");
    assert!(
        (e.interaction_type_flags & INT_MISSION_WORLD_OBJECT) != 0,
        "template 421 spawns with INT_MissionWorldObject, got {:#x}",
        e.interaction_type_flags
    );
}

/// Revert proof: comment out the `spawnlist_tollana.sql` include in
/// `database.sql`, or change one row's coordinates, and this fails.
#[tokio::test]
async fn live_db_tollana_cast_rows_are_seeded_exactly() {
    let pool = require_db_or_skip!();
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.spawnlist WHERE spawn_id BETWEEN 19001 AND 19010",
    )
    .fetch_one(&pool)
    .await
    .expect("T-02a spawn count");
    assert_eq!(count, 4, "spawns 19001-19010 hold the four cast rows");

    for (spawn_id, template_id, tag, [x, y, z]) in CAST {
        let row: (i32, i32, String, f32, f32, f32, bool, Option<i32>) = sqlx::query_as(
            "SELECT world_id, template_id, tag::text, x, y, z, is_stationary, respawn_secs \
             FROM resources.spawnlist WHERE spawn_id = $1",
        )
        .bind(spawn_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("cast spawn {spawn_id}: {e}"));
        assert_eq!(
            (row.0, row.1, row.2.as_str(), row.6, row.7),
            (WORLD, template_id, tag, true, Some(30)),
            "spawn {spawn_id}: world, template, tag, stationary, respawn"
        );
        for (got, want) in [(row.3, x), (row.4, y), (row.5, z)] {
            assert!(
                (got - want).abs() < 0.001,
                "spawn {spawn_id} moved: {got} vs {want}"
            );
        }
    }

    let (last_value,): (i64,) =
        sqlx::query_as("SELECT last_value FROM resources.spawnlist_spawn_id_seq")
            .fetch_one(&pool)
            .await
            .expect("spawnlist_spawn_id_seq");
    assert!(
        last_value >= 19010,
        "spawnlist_spawn_id_seq is {last_value}, below the reserved block"
    );
}

/// Revert proof: move one seeded prop 25 m from Copplemann, or into a wall,
/// and this names the failing bound.
#[tokio::test]
async fn live_db_tollana_cast_is_on_tollana_nav() {
    let pool = require_db_or_skip!();
    let points = seeded_points(&pool).await;
    assert_eq!(points.len(), 4, "four cast rows");
    let mesh = tollana_mesh();

    for (i, p) in points.iter().enumerate() {
        assert!(
            mesh.is_point_valid(&vector(*p)),
            "spawn {} at {p:?} is off the shipped navmesh",
            19001 + i
        );
    }

    let copp = points[0];
    for (i, prop) in points.iter().enumerate().skip(1) {
        let path = mesh.find_path(&vector(copp), &vector(*prop));
        assert!(
            matches!(path.status, PathStatus::Ok),
            "spawn {} has no path from Copplemann",
            19001 + i
        );
        assert!(
            horizontal(copp, *prop) <= 20.0,
            "spawn {} is more than 20 m from Copplemann",
            19001 + i
        );
    }

    let props = &points[1..];
    for (i, a) in props.iter().enumerate() {
        for b in &props[i + 1..] {
            assert!(
                horizontal(*a, *b) >= 3.0,
                "food units {a:?} and {b:?} are closer than 3 m"
            );
        }
    }

    assert!(
        horizontal(copp, GATE_4) <= 20.0,
        "Copplemann is more than 20 m from gate 4"
    );
}
