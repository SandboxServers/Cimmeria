//! World seeds for SGC_W1 Union (US-11): the Control Room, Carter's terminal,
//! the Failsafe Device, the six requisitions crates, the route Jaffa tags,
//! the two cylinder regions and templates 5800-5803. Seeds live in
//! `db/resources/`; the positions are the sealed points in
//! `docs/analysis/union-sgc-continuation/placement.md`.

use super::rig::*;
use crate::test_support::require_db_or_skip;

/// The nine spawns of `spawnlist_sgc_w1_union.sql` and their seeded
/// positions (world 58).
const EXPECTED: [(&str, [f32; 3]); 9] = [
    ("SGC_W1_Harriman", [172.08, -6.40, 46.70]),
    ("SGC_W1_CarterTerminal", [-49.48, 2.32, 36.38]),
    ("SGC_W1_FailsafeDevice", [151.184, 1.28, -193.376]),
    ("SGC_W1_Crate1", [257.157, -10.85, 41.74]),
    ("SGC_W1_Crate2", [259.131, -10.85, 42.326]),
    ("SGC_W1_Crate3", [260.237, -10.85, 43.42]),
    ("SGC_W1_Crate4", [255.923, -10.85, 43.406]),
    ("SGC_W1_Crate5", [257.726, -10.85, 43.679]),
    ("SGC_W1_Crate6", [254.369, -10.85, 43.338]),
];

/// The seven route Jaffa of `spawnlist.sql` rows 57-63 with their story tags
/// (D-US10 option B).
const JAFFA: [(&str, [f32; 3]); 7] = [
    ("SGC_W1_TacticsJaffa1", [-52.056, 1.311, -44.99]),
    ("SGC_W1_TacticsJaffa2", [-69.456, 1.311, -44.085]),
    ("SGC_W1_ClearJaffa1", [210.781, 1.311, -273.39]),
    ("SGC_W1_ClearJaffa2", [209.334, 1.311, -280.5]),
    ("SGC_W1_ClearJaffa3", [188.105, 1.311, -258.789]),
    ("SGC_W1_ClearJaffa4", [179.442, 1.311, -271.59]),
    ("SGC_W1_ClearJaffa5", [123.702, 1.311, -215.982]),
];

/// Whether `actual` is within 0.01 on every axis of `expected`.
fn near(actual: [f32; 3], expected: [f32; 3]) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(a, e)| (a - e).abs() < 0.01)
}

/// **Guard: every Union world object spawns where placement.md puts it.**
/// Revert proof: delete `spawnlist_sgc_w1_union.sql` (or its `\ir` line in
/// `db/database.sql`) and `SGC_W1_Harriman` is not live.
#[tokio::test]
async fn live_db_union_sgc_world_objects_spawn_at_their_points() {
    let pool = require_db_or_skip!();
    let rig = Rig::new(&pool, Some(SOLDIER)).await;
    for (tag, expected) in EXPECTED {
        assert!(rig.live(tag), "{tag} is a seeded world-58 spawn");
        let actual = rig.pos(tag);
        assert!(
            near(actual, expected),
            "{tag} at {actual:?}, seeded at {expected:?}"
        );
    }
    // Spawn 64 keeps its prop: chains 3041-3044 are about it.
    assert!(rig.live("SGC_W1_Crate"), "SGC_W1_Crate (spawn 64) is live");
}

/// **Guard: the static route Jaffa carry the story tags (D-US10 B).**
/// Revert proof: put row 57's tag back to NULL and `SGC_W1_TacticsJaffa1` is
/// not found (`rig.pos` panics with `is spawned`).
#[tokio::test]
async fn live_db_union_sgc_route_jaffa_are_tagged() {
    let pool = require_db_or_skip!();
    let rig = Rig::new(&pool, Some(SOLDIER)).await;
    for (tag, expected) in JAFFA {
        assert!(rig.live(tag), "{tag} is a tagged world-58 spawn");
        let actual = rig.pos(tag);
        assert!(
            near(actual, expected),
            "{tag} at {actual:?}, seeded at {expected:?}"
        );
    }
}

/// **Guard: both regions are seeded as hinted cylinders on world 58, and the
/// SGC templates 5800-5803 are the world-object rows.**
/// Revert proof: delete the 15800 row in `point_sets.sql` and the query
/// returns no row for 15800.
#[tokio::test]
async fn live_db_union_sgc_regions_are_hinted_world_58_cylinders() {
    let pool = require_db_or_skip!();
    let sets: Vec<(String, i32, String, i64, f64, f64)> = sqlx::query_as(
        "SELECT name, world_id, shape::text, flags::int8, radius::float8, height::float8 \
         FROM resources.point_sets WHERE set_id = ANY($1) ORDER BY set_id",
    )
    .bind(vec![15800_i32, 15801])
    .fetch_all(&pool)
    .await
    .expect("point_sets");
    assert_eq!(sets.len(), 2, "both SGC_W1 region rows exist");
    assert_eq!(sets[0].0, "SGC_W1.FailsafeRoom");
    assert_eq!(sets[1].0, "SGC_W1.GateControl");
    for (name, world, shape, flags, _, _) in &sets {
        assert_eq!(*world, 58, "{name} is on SGC_W1");
        assert_eq!(shape.as_str(), "Cylinder", "{name} is a cylinder");
        assert_eq!(flags & 1, 1, "{name} is hinted to the client");
    }

    let points: Vec<(i32, f64, f64, f64)> = sqlx::query_as(
        "SELECT set_id, x::float8, y::float8, z::float8 FROM resources.point_set_points \
         WHERE set_id = ANY($1) ORDER BY set_id",
    )
    .bind(vec![15800_i32, 15801])
    .fetch_all(&pool)
    .await
    .expect("point_set_points");
    let centres = [
        (15800, [150.0_f32, 1.3, -196.0]),
        (15801, [173.0_f32, -6.4, 55.0]),
    ];
    assert_eq!(points.len(), 2, "one centre point per region");
    for ((set, x, y, z), (want_set, want)) in points.iter().zip(centres) {
        assert_eq!(*set, want_set);
        let got = [*x as f32, *y as f32, *z as f32];
        assert!(near(got, want), "region {set} centre at {got:?}");
    }

    let templates: Vec<(i32, i64, String)> = sqlx::query_as(
        "SELECT template_id, flags::int8, class::text FROM resources.entity_templates \
         WHERE template_id = ANY($1) ORDER BY template_id",
    )
    .bind(vec![5800_i32, 5802, 5803])
    .fetch_all(&pool)
    .await
    .expect("entity_templates");
    let want: Vec<(i32, i64, String)> = [5800, 5802, 5803]
        .into_iter()
        .map(|id| (id, 4, "being".to_string()))
        .collect();
    assert_eq!(templates, want, "5800, 5802 and 5803 are flags 4 beings");
    let reserved: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resources.entity_templates WHERE template_id = 5801",
    )
    .fetch_one(&pool)
    .await
    .expect("5801 count");
    assert_eq!(reserved, 0, "5801 is reserved (F12), not seeded");
}
