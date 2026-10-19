//! UB-02b production seed guards for the hub cast and objects (spawns
//! 23000-23021) and the Storage and AdminBuilding regions on world 23. The
//! positions are the UB-00b placement seal's snapped stand points.

use super::nav::{mesh, standable, standable_and_connected, ARRIVAL};
use super::*;

/// Spawn id, template, tag and seal position of each hub spawn.
const HUB: [(i32, i32, &str, [f32; 3]); 16] = [
    (23000, 2300, "BetaE1_Carter", [1011.405, 13.57, 511.851]),
    (23001, 2301, "BetaE1_Felger", [1007.405, 13.57, 511.851]),
    (
        23002,
        2302,
        "BetaE1_Satterfield",
        [1004.144, 13.44, 276.690],
    ),
    (23003, 2304, "BetaE1_Debeaux", [1034.146, 13.38, 314.764]),
    (23010, 2320, "BetaE1_XenoCage_1", [712.389, -66.99, 411.698]),
    (23011, 2320, "BetaE1_XenoCage_2", [765.860, -63.85, 426.136]),
    (23012, 2320, "BetaE1_XenoCage_3", [826.331, -36.48, 446.574]),
    (23013, 2320, "BetaE1_XenoCage_4", [884.801, -16.69, 464.012]),
    (23014, 2320, "BetaE1_XenoCage_5", [938.272, -4.48, 481.450]),
    (
        23015,
        2325,
        "BetaE1_Crate_Arc42",
        [1007.015, 13.37, 275.820],
    ),
    (
        23016,
        2325,
        "BetaE1_Crate_Arc66",
        [1004.015, 13.43, 275.820],
    ),
    (
        23017,
        2322,
        "BetaE1_HubComputer",
        [1018.405, 13.90, 506.851],
    ),
    (
        23018,
        2323,
        "BetaE1_AdminWorkstation",
        [1018.405, 13.57, 509.851],
    ),
    (
        23019,
        2324,
        "BetaE1_ScienceComputer",
        [649.700, 19.77, 814.100],
    ),
    (
        23020,
        2321,
        "BetaE1_FourPiecesCase",
        [647.700, 19.77, 814.100],
    ),
    (23021, 2335, "BetaE1_RuinNote", [122.200, 27.17, 356.800]),
];

/// Region set id, name and centre (the seal's Storage and AdminBuilding rows).
const REGIONS: [(i32, &str, [f32; 3]); 2] = [
    (23000, "Beta_Site_Evo_1.Storage", [1022.327, 13.37, 271.180]),
    (
        23001,
        "Beta_Site_Evo_1.AdminBuilding",
        [1018.405, 13.90, 506.851],
    ),
];

#[tokio::test]
async fn live_db_beta_hub_spawns_match_the_placement_seal() {
    // Revert proof: comment out the spawnlist `\ir` line in database.sql, or
    // move Carter 50 m into a cliff, or delete the spawn footer; this fails
    // naming the spawn or the sequence.
    let pool = require_db_or_skip!();
    let exterior = mesh("beta_site_evo_1.nav");

    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.spawnlist WHERE spawn_id BETWEEN 23000 AND 23059",
    )
    .fetch_one(&pool)
    .await
    .expect("UB-02b spawn count");
    assert_eq!(count, 16);

    for (spawn_id, template_id, tag, expected) in HUB {
        let row: (i32, i32, String, f32, f32, f32, bool, Option<i32>) = sqlx::query_as(
            "SELECT world_id, template_id, tag::text, x, y, z, is_stationary, respawn_secs \
             FROM resources.spawnlist WHERE spawn_id = $1",
        )
        .bind(spawn_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("hub spawn {spawn_id}: {e}"));
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
        // F8: the two nav-890 interior rows sit behind a 0.90 m door. `reaches`
        // fails across it to ARRIVAL (placement seal, "gap-linked"), so these
        // two are checked for standability only. RuinNote (23021) is the same:
        // the seal's same-component proxy says reachable, but find_path finds
        // no route (the seal's own caveat), so it is standable-only until the
        // seal is re-probed.
        if matches!(spawn_id, 23019..=23021) {
            standable(&exterior, actual, tag);
        } else {
            standable_and_connected(&exterior, actual, ARRIVAL, tag);
        }
    }

    // The footer in spawnlist_beta_e1_hub.sql reserves Beta's block: without
    // it the sequence stops at the highest hub row and the next default id
    // lands below the block top (the seeded-sequence guard's floor is only
    // 13869, so it does not catch this).
    let last: i64 = sqlx::query_scalar("SELECT last_value FROM resources.spawnlist_spawn_id_seq")
        .fetch_one(&pool)
        .await
        .expect("spawn sequence");
    assert!(
        last >= 23199,
        "spawnlist_spawn_id_seq is at {last}: the UB-02b spawn footer is missing"
    );
}

#[tokio::test]
async fn live_db_beta_hub_regions_are_named_and_reachable() {
    // Revert proof: rename Storage in point_sets_beta_e1_hub.sql, or move the
    // AdminBuilding centre into a cliff; this fails naming the region.
    let pool = require_db_or_skip!();
    let exterior = mesh("beta_site_evo_1.nav");

    for (set_id, name, expected) in REGIONS {
        let row: (String, i32, i32, f32, f32, f32) = sqlx::query_as(
            "SELECT s.name::text, s.world_id, s.flags, p.x, p.y, p.z \
             FROM resources.point_sets s JOIN resources.point_set_points p USING (set_id) \
             WHERE s.set_id = $1",
        )
        .bind(set_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("region {set_id}: {e}"));
        assert_eq!((row.0.as_str(), row.1, row.2), (name, 23, 1));
        let centre = [row.3, row.4, row.5];
        assert_eq!(centre, expected, "{name} centre moved");
        standable_and_connected(&exterior, centre, ARRIVAL, name);
    }
}

#[tokio::test]
async fn live_db_beta_hub_tags_are_unique_across_worlds() {
    // Revert proof: give two hub spawns the same tag (for example, copy
    // `BetaE1_Carter` onto spawn 23001); this fails naming the duplicate.
    let pool = require_db_or_skip!();

    let distinct: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT tag) FROM resources.spawnlist WHERE spawn_id BETWEEN 23000 AND 23059",
    )
    .fetch_one(&pool)
    .await
    .expect("UB-02b distinct tags");
    assert_eq!(distinct, 16);

    let duplicates: Vec<(String, i64)> = sqlx::query_as(
        "SELECT tag::text, count(*) FROM resources.spawnlist \
         WHERE tag IN (SELECT tag FROM resources.spawnlist WHERE spawn_id BETWEEN 23000 AND 23059) \
         GROUP BY tag HAVING count(*) > 1",
    )
    .fetch_all(&pool)
    .await
    .expect("UB-02b duplicate tags");
    assert!(duplicates.is_empty(), "duplicate tags: {duplicates:?}");
}
