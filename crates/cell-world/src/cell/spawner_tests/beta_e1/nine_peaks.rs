//! UB-02d production seed guards for the Nine Peaks camp (spawns 23110-23114),
//! the SW ring spawn (23115), the Nine Peaks region and the ring pair 2300/2301
//! that links the hub ring to the valley ring. The positions are the UB-00b
//! placement seal's snapped stand points. Nav component 70 has no walk route
//! to ARRIVAL (seal Q1), so the camp is reached by beam or by the ring pair.

use super::nav::{mesh, standable, standable_and_connected, ARRIVAL};
use super::*;
use cimmeria_common::Vector3;
use cimmeria_entity::navigation::{NavMesh, PathStatus};

/// Ras'nor, the camp anchor (spawn 23110). The other camp rows must reach it.
const RAS_NOR: [f32; 3] = [-545.100, 6.77, 141.500];

/// Spawn id, template, tag, seal position and heading of each camp spawn.
const CAMP: [(i32, i32, &str, [f32; 3], f32); 5] = [
    (23110, 2306, "BetaE1_RasNor", RAS_NOR, 4.957),
    (
        23111,
        2307,
        "BetaE1_SickJaffa_1",
        [-547.100, 6.80, 138.500],
        0.588,
    ),
    (
        23112,
        2307,
        "BetaE1_SickJaffa_2",
        [-554.100, 6.77, 142.500],
        1.681,
    ),
    (
        23113,
        2307,
        "BetaE1_SickJaffa_3",
        [-552.100, 6.77, 138.500],
        1.166,
    ),
    (
        23114,
        2326,
        "BetaE1_JaffaBox",
        [-546.100, 6.77, 146.500],
        2.944,
    ),
];

/// The SW ring spawn (23115) and its ring, which share a position.
const SW_RING: [f32; 3] = [-791.600, -51.43, -92.600];

/// The hub ring centre (ring 148, nav 251), the pair's first end.
const HUB_RING: [f32; 3] = [146.716, 8.77, 403.273];

/// A standable pocket on nav 251, 12.73 m from the hub ring and not walkable
/// from it (UB-00b-2). The negative control: a walk from the ring must not
/// reach it, so the leg test cannot pass by accident.
const ISLAND_POCKET: [f32; 3] = [144.916, 7.811, 415.873];

fn vector(p: [f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

/// Whether `find_path` walks from `from` to `to`, as `reaches` in `nav.rs`
/// does. That helper is private to `nav.rs`, so this guard carries a copy.
fn walks(mesh: &NavMesh, from: [f32; 3], to: [f32; 3]) -> bool {
    let path = mesh.find_path(&vector(from), &vector(to));
    matches!(path.status, PathStatus::Ok)
        && path
            .waypoints
            .last()
            .is_some_and(|end| end.distance_to(&vector(to)) < 2.0)
}

/// Longest route one `find_path` call returns. `NavMesh::find_path` caps the
/// corridor at `MAX_POLY_PATH` (256 polygons), so a route of about 500 m ends
/// `Partial` short of its goal; the ring leg needs several calls.
const MAX_LEGS: usize = 8;

/// Walks `from` to `to` in legs: each leg restarts from the last waypoint of
/// the previous one while it is `Partial` and still making progress. Passes
/// only when a leg is `Ok` and ends within 2 m of `to`, the same bar `walks`
/// sets for a single leg.
fn walks_in_legs(mesh: &NavMesh, from: [f32; 3], to: [f32; 3]) -> bool {
    let goal = vector(to);
    let mut here = vector(from);
    for _ in 0..MAX_LEGS {
        let path = mesh.find_path(&here, &goal);
        let Some(&end) = path.waypoints.last() else {
            return false;
        };
        if matches!(path.status, PathStatus::Ok) {
            return end.distance_to(&goal) < 2.0;
        }
        if here.distance_to(&end) <= 0.5 {
            return false;
        }
        here = end;
    }
    false
}

#[tokio::test]
async fn live_db_beta_nine_peaks_spawns_match_the_placement_seal() {
    // Revert proof: remove the Ras'nor row from spawnlist_beta_e1_ninepeaks.sql;
    // the count fails. Move a SickJaffa 50 m into a cliff; the position check
    // fails naming the spawn.
    let pool = require_db_or_skip!();
    let exterior = mesh("beta_site_evo_1.nav");

    // The camp block is 23110-23114. The SW ring spawn (23115) is checked in
    // the ring test, so it is outside this count.
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.spawnlist WHERE spawn_id BETWEEN 23110 AND 23114",
    )
    .fetch_one(&pool)
    .await
    .expect("UB-02d camp spawn count");
    assert_eq!(count, 5);

    for (spawn_id, template_id, tag, expected, heading) in CAMP {
        let row: (i32, i32, String, f32, f32, f32, f32, bool, Option<i32>) = sqlx::query_as(
            "SELECT world_id, template_id, tag::text, x, y, z, heading, is_stationary, respawn_secs \
             FROM resources.spawnlist WHERE spawn_id = $1",
        )
        .bind(spawn_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("camp spawn {spawn_id}: {e}"));
        assert_eq!(
            (row.0, row.1, row.2.as_str(), row.7, row.8),
            (23, template_id, tag, true, Some(30)),
            "spawn {spawn_id}"
        );
        assert_eq!(
            [row.3, row.4, row.5],
            expected,
            "spawn {spawn_id} moved: update the placement seal"
        );
        assert_eq!(row.6, heading, "spawn {spawn_id} heading");
        if spawn_id == 23110 {
            standable(&exterior, expected, tag);
        } else {
            standable_and_connected(&exterior, expected, RAS_NOR, tag);
        }
        // Seal Q1: no foot route from the camp to ARRIVAL. The ring pair is
        // the only link, and the ring test below checks it.
        assert!(
            !walks(&exterior, expected, ARRIVAL),
            "{tag} walks to ARRIVAL: nav 70 must stay cut off (seal Q1)"
        );
    }
}

#[tokio::test]
async fn live_db_beta_nine_peaks_region_is_named_at_the_seal_centre() {
    // Revert proof: rename NinePeaks in point_sets_beta_e1_ninepeaks.sql; this
    // fails naming the region. The centre is a region centre, not a stand
    // point (the seal measures it 3.50 m off the mesh), so it is not walked.
    let pool = require_db_or_skip!();

    let row: (String, i32, i32, f32, f32, f32) = sqlx::query_as(
        "SELECT s.name::text, s.world_id, s.flags, p.x, p.y, p.z \
         FROM resources.point_sets s JOIN resources.point_set_points p USING (set_id) \
         WHERE s.set_id = 23060",
    )
    .fetch_one(&pool)
    .await
    .expect("UB-02d Nine Peaks region");
    assert_eq!(
        (row.0.as_str(), row.1, row.2),
        ("Beta_Site_Evo_1.NinePeaks", 23, 1)
    );
    assert_eq!(
        [row.3, row.4, row.5],
        [-549.100, 6.77, 142.500],
        "NinePeaks centre moved"
    );
}

#[tokio::test]
async fn live_db_beta_ring_pair_links_both_valleys() {
    // Revert proof: drop 2301 from ring_transport_regions_beta_e1.sql; this
    // fails on the missing row. Move 2300 off ring 148 and its centre fails
    // to reach ARRIVAL.
    let pool = require_db_or_skip!();
    let exterior = mesh("beta_site_evo_1.nav");

    let rows: Vec<(i32, i32, Vec<i32>, f32, f32, f32)> = sqlx::query_as(
        "SELECT region_id, world_id, destination_region_ids, x, y, z \
         FROM resources.ring_transport_regions WHERE region_id IN (2300, 2301) \
         ORDER BY region_id",
    )
    .fetch_all(&pool)
    .await
    .expect("UB-02d ring rows");
    assert_eq!(rows.len(), 2, "both ring rows 2300 and 2301 must exist");

    let (hub, sw) = (&rows[0], &rows[1]);
    assert_eq!(
        (hub.0, hub.1, hub.2.as_slice()),
        (2300, 23, &[2301][..]),
        "2300 names 2301"
    );
    assert_eq!(
        (sw.0, sw.1, sw.2.as_slice()),
        (2301, 23, &[2300][..]),
        "2301 names 2300"
    );
    assert_eq!([hub.3, hub.4, hub.5], HUB_RING, "ring 2300 moved");
    assert_eq!([sw.3, sw.4, sw.5], SW_RING, "ring 2301 moved");

    // 2300 (nav 251) walks to ARRIVAL in legs: one find_path call truncates
    // at 256 polygons and ends about 220 m short. Its reverse is not checked
    // (the corridor is not symmetric). 2301 (nav 70) reaches Ras'nor in one leg.
    standable(&exterior, HUB_RING, "ring 2300");
    assert!(
        walks_in_legs(&exterior, ARRIVAL, HUB_RING),
        "ring 2300 is not walkable from ARRIVAL in legs (nav 251)"
    );
    standable_and_connected(&exterior, SW_RING, RAS_NOR, "ring 2301");

    // Negative control: the island pocket is standable but a walk from the
    // ring does not reach it, so the legs above prove a real walk.
    standable(&exterior, ISLAND_POCKET, "island pocket");
    assert!(
        !walks_in_legs(&exterior, HUB_RING, ISLAND_POCKET),
        "the island pocket must not be walkable from ring 2300"
    );

    // The SW ring spawn sits on the same position as ring 2301.
    let spawn: (i32, String, f32, f32, f32) = sqlx::query_as(
        "SELECT template_id, tag::text, x, y, z FROM resources.spawnlist WHERE spawn_id = 23115",
    )
    .fetch_one(&pool)
    .await
    .expect("UB-02d SW ring spawn");
    assert_eq!((spawn.0, spawn.1.as_str()), (3, "BetaE1_Ring_SW"));
    assert_eq!([spawn.2, spawn.3, spawn.4], SW_RING, "spawn 23115 moved");
}
