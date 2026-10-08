//! DK-05 production seed guards for the four static story actors and the
//! owner-approved speculative Repository location.

use super::*;

const CAST: [(i32, i32, i32, &str, [f32; 3]); 4] = [
    (8004, 62, 59, "Dakara_E1_StoryRm_Bratac", [69.5, 0.0, 27.6]),
    (
        8005,
        62,
        54,
        "Dakara_E1_StoryRm_Mohkatan",
        [73.6, 0.0, 27.6],
    ),
    (8006, 61, 441, "Dakara_E1_Raknor", [103.5, -16.8, 238.5]),
    (8007, 61, 440, "Dakara_E1_Lothta", [290.0, -17.0, 95.0]),
];

#[tokio::test]
async fn live_db_dakara_cast_and_repository_match_approved_estimates() {
    let pool = require_db_or_skip!();
    let exterior = travel::mesh("dakara_e1.nav");
    let interior = travel::mesh("dakara_e1_storyrm.nav");

    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.spawnlist WHERE spawn_id BETWEEN 8004 AND 8007",
    )
    .fetch_one(&pool)
    .await
    .expect("DK-05 spawn count");
    assert_eq!(count, 4);

    for (spawn_id, world_id, template_id, tag, expected) in CAST {
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
            (world_id, template_id, tag, true, Some(30))
        );
        assert!(tag.starts_with("Dakara_E1_"));
        let actual = [row.3, row.4, row.5];
        assert_eq!(
            actual, expected,
            "spawn {spawn_id} moved: update the ledger"
        );
        if world_id == 61 {
            travel::standable_and_connected(&exterior, actual, [100.0, -17.4, 230.0], tag);
        } else {
            travel::standable_and_connected(&interior, actual, [71.0, 0.05, 30.0], tag);
        }
    }

    let repository: (String, i32, i32, f32, f32, f32) = sqlx::query_as(
        "SELECT s.name::text, s.world_id, s.flags, p.x, p.y, p.z \
         FROM resources.point_sets s JOIN resources.point_set_points p USING (set_id) \
         WHERE s.set_id = 2127",
    )
    .fetch_one(&pool)
    .await
    .expect("speculative Repository region");
    assert_eq!(
        (repository.0.as_str(), repository.1, repository.2),
        ("Dakara_E1.NaqRepository", 61, 1)
    );
    assert_eq!(
        [repository.3, repository.4, repository.5],
        [290.0, -17.0, 95.0]
    );
    travel::standable_and_connected(
        &exterior,
        [repository.3, repository.4, repository.5],
        [100.0, -17.4, 230.0],
        "speculative Repository",
    );
}
