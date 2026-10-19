//! UB-02c production seed guards for the Asgard base, deck, infirmary and fire
//! site on world 23: spawns 23060-23071 and the Infirmary and FireSite regions
//! (sets 23030 and 23031). The positions are the UB-00b placement seal's
//! snapped stand points.

use super::nav::{mesh, standable, standable_and_connected, ARRIVAL};
use super::*;
use crate::cell::spawner::{
    is_point_in_region, load_regions_from_db, GENERIC_REGION_CHECK_THRESHOLD,
};
use cimmeria_common::Vector3;
use cimmeria_entity::navigation::{NavMesh, PathStatus};

/// Mimir's deck stand point (nav 355, beam only).
const MIMIR: [f32; 3] = [696.440, 157.77, 67.360];
/// The deck spawns, all on nav 355 and none walkable to `ARRIVAL` (F8).
const DECK: [i32; 3] = [23060, 23066, 23067];
const HOLOGRAM_EMITTER: [f32; 3] = [961.300, 15.97, 284.900];
const DISABLED_DRONE: [f32; 3] = [959.800, 15.97, 284.900];
const FIRE_SUPPRESSANT_MANAGER: [f32; 3] = [1019.327, 13.37, 271.180];
const FIRE_CONTROL_HUB: [f32; 3] = [1022.327, 13.37, 268.180];
const INFIRMARY_CENTRE: [f32; 3] = [961.300, 15.97, 284.900];
/// The FireSite centre and radius (placement seal, Regions table).
const FIRE_SITE_CENTRE: [f32; 3] = [1_022.3, 14.50, 271.2];
const FIRE_SITE_RADIUS: f32 = 12.0;

/// The beam pads that UB-30's beam chains use: the deck pad beside spawn 151
/// and the ground pad beside spawn 149.
pub(super) const BEAM_DECK_DEST: [f32; 3] = [698.980, 157.93, 70.360];
pub(super) const BEAM_GROUND_DEST: [f32; 3] = [745.050, -19.60, 68.750];

/// Spawn id, template, tag and seal position of each Asgard spawn.
const ASGARD: [(i32, i32, &str, [f32; 3]); 12] = [
    (23060, 2305, "BetaE1_Mimir", MIMIR),
    (
        23061,
        2330,
        "BetaE1_AsgardScanner_1",
        [740.480, -19.43, 68.640],
    ),
    (
        23062,
        2330,
        "BetaE1_AsgardScanner_2",
        [746.480, -19.54, 62.640],
    ),
    (
        23063,
        2330,
        "BetaE1_AsgardScanner_3",
        [746.480, -19.80, 74.640],
    ),
    (
        23064,
        2330,
        "BetaE1_AsgardScanner_4",
        [752.480, -20.20, 68.640],
    ),
    (
        23065,
        2330,
        "BetaE1_AsgardScanner_5",
        [740.480, -19.43, 62.640],
    ),
    (
        23066,
        2331,
        "BetaE1_AnalyticalMachine",
        [696.440, 157.77, 64.360],
    ),
    (
        23067,
        2334,
        "BetaE1_ConsciousnessSimulator",
        [699.440, 157.93, 67.360],
    ),
    (23068, 2332, "BetaE1_HologramEmitter", HOLOGRAM_EMITTER),
    (23069, 2333, "BetaE1_DisabledDrone", DISABLED_DRONE),
    (
        23070,
        2328,
        "BetaE1_FireSuppressantManager",
        FIRE_SUPPRESSANT_MANAGER,
    ),
    (23071, 2329, "BetaE1_FireControlHub", FIRE_CONTROL_HUB),
];

/// True when `find_path` walks from `from` to `to`. This is the test `reaches`
/// applies in `nav.rs`, which is private to that module.
fn walks_to(mesh: &NavMesh, from: [f32; 3], to: [f32; 3]) -> bool {
    let vector = |p: [f32; 3]| Vector3::new(p[0], p[1], p[2]);
    let path = mesh.find_path(&vector(from), &vector(to));
    matches!(path.status, PathStatus::Ok)
        && path
            .waypoints
            .last()
            .is_some_and(|end| end.distance_to(&vector(to)) < 2.0)
}

/// Horizontal (x, z) distance between two points, in metres.
fn ground_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The seeded position of a spawn, read from the live table.
async fn spawn_position(pool: &sqlx::PgPool, spawn_id: i32) -> [f32; 3] {
    let row: (f32, f32, f32) =
        sqlx::query_as("SELECT x, y, z FROM resources.spawnlist WHERE spawn_id = $1")
            .bind(spawn_id)
            .fetch_one(pool)
            .await
            .unwrap_or_else(|e| {
                panic!("spawn {spawn_id} is not seeded (23002 is UB-02b's Satterfield): {e}")
            });
    [row.0, row.1, row.2]
}

/// The expanded four-corner box of a named region, as the loader builds it.
async fn region_points(pool: &sqlx::PgPool, name: &str) -> Vec<[f32; 3]> {
    let regions = load_regions_from_db(pool).await.expect("region load");
    let region = regions
        .into_iter()
        .find(|r| r.name == name)
        .unwrap_or_else(|| panic!("region {name} is not loaded"));
    assert_eq!(region.points.len(), 4, "{name} must expand to four corners");
    region.points
}

#[tokio::test]
/// Revert proof: remove Mimir's row (the count fails), or move Mimir onto the
/// ground so it walks to ARRIVAL (the deck check fails), or move a ground row
/// into a cliff (the standable check fails). Each names the spawn.
async fn live_db_beta_asgard_spawns_match_the_placement_seal() {
    let pool = require_db_or_skip!();
    let exterior = mesh("beta_site_evo_1.nav");

    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.spawnlist WHERE spawn_id BETWEEN 23060 AND 23109",
    )
    .fetch_one(&pool)
    .await
    .expect("UB-02c spawn count");
    assert_eq!(count, 12);

    for (spawn_id, template_id, tag, expected) in ASGARD {
        let row: (i32, i32, String, f32, f32, f32, bool, Option<i32>) = sqlx::query_as(
            "SELECT world_id, template_id, tag::text, x, y, z, is_stationary, respawn_secs \
             FROM resources.spawnlist WHERE spawn_id = $1",
        )
        .bind(spawn_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("Asgard spawn {spawn_id}: {e}"));
        assert_eq!(
            (row.0, row.1, row.2.as_str(), row.6, row.7),
            (23, template_id, tag, true, Some(30)),
            "spawn {spawn_id}"
        );
        let actual = [row.3, row.4, row.5];
        assert_eq!(
            actual, expected,
            "spawn {spawn_id} moved: update the placement seal"
        );
        if DECK.contains(&spawn_id) {
            // The deck is beam only (F8): standable, connected to Mimir, and no
            // walk to ARRIVAL. If it walked, the beam would be pointless.
            if spawn_id == 23060 {
                standable(&exterior, actual, tag);
            } else {
                standable_and_connected(&exterior, actual, MIMIR, tag);
            }
            assert!(
                !walks_to(&exterior, actual, ARRIVAL),
                "{tag} walks to ARRIVAL: the deck must be reachable only by beam (F8)"
            );
        } else {
            standable_and_connected(&exterior, actual, ARRIVAL, tag);
        }
    }
}

#[tokio::test]
/// Revert proof: move BEAM_DECK_DEST 20 m off the deck, or move
/// BEAM_GROUND_DEST into the cliff; this fails naming the pad.
async fn live_db_beta_beam_pads_stand_on_their_decks() {
    let pool = require_db_or_skip!();
    let exterior = mesh("beta_site_evo_1.nav");

    // The seal puts each pad 1.5 m from its spawn. The deck pair is 1.49999 m
    // apart in f32, so the deck floor has 0.05 m of slack.
    let deck_gap = ground_distance(BEAM_DECK_DEST, spawn_position(&pool, 151).await);
    assert!(
        (1.45..=2.5).contains(&deck_gap),
        "BEAM_DECK_DEST is {deck_gap:.3} m from spawn 151"
    );
    standable_and_connected(&exterior, BEAM_DECK_DEST, MIMIR, "BEAM_DECK_DEST");

    let ground_gap = ground_distance(BEAM_GROUND_DEST, spawn_position(&pool, 149).await);
    assert!(
        (1.5..=2.5).contains(&ground_gap),
        "BEAM_GROUND_DEST is {ground_gap:.3} m from spawn 149"
    );
    standable_and_connected(&exterior, BEAM_GROUND_DEST, ARRIVAL, "BEAM_GROUND_DEST");
}

#[tokio::test]
/// Revert proof: rename the Infirmary set, or move the emitter 20 m outside
/// the box; this fails naming the region or the object.
async fn live_db_beta_infirmary_region_covers_its_objects() {
    let pool = require_db_or_skip!();
    let exterior = mesh("beta_site_evo_1.nav");
    let points = region_points(&pool, "Beta_Site_Evo_1.Infirmary").await;

    for (label, object) in [
        ("HologramEmitter", HOLOGRAM_EMITTER),
        ("DisabledDrone", DISABLED_DRONE),
    ] {
        assert!(
            is_point_in_region(&points, object),
            "{label} at {object:?} is outside the Infirmary region"
        );
    }
    standable_and_connected(&exterior, INFIRMARY_CENTRE, ARRIVAL, "Infirmary centre");
}

#[tokio::test]
/// Revert proof: move Satterfield (23002) into the fire zone, or shrink
/// FireSite below the fire objects; this fails naming the object or spawn.
async fn live_db_beta_fire_site_holds_its_objects_but_not_satterfield() {
    let pool = require_db_or_skip!();
    let points = region_points(&pool, "Beta_Site_Evo_1.FireSite").await;

    for (label, object) in [
        ("FireSuppressantManager", FIRE_SUPPRESSANT_MANAGER),
        ("FireControlHub", FIRE_CONTROL_HUB),
    ] {
        assert!(
            is_point_in_region(&points, object),
            "{label} at {object:?} is outside the FireSite region"
        );
    }

    // UB-34 advances 443 S1484 on entering this region after the Satterfield
    // talk, so Satterfield must stand at least 2 m outside the region's edge.
    let satterfield = spawn_position(&pool, 23002).await;
    assert!(
        !is_point_in_region(&points, satterfield),
        "Satterfield (23002) is inside FireSite: UB-34 could not advance 443 S1484"
    );
    let gap = (satterfield[0] - FIRE_SITE_CENTRE[0])
        .abs()
        .max((satterfield[2] - FIRE_SITE_CENTRE[2]).abs())
        - (FIRE_SITE_RADIUS + GENERIC_REGION_CHECK_THRESHOLD);
    assert!(
        gap >= 2.0,
        "Satterfield is only {gap:.2} m outside FireSite (needs 2 m)"
    );
}
