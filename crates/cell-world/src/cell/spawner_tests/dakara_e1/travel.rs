//! DK-04 seed guards: the four flap spawns, four evidence-backed named
//! areas, both travel arrivals and the two Dakara respawners. The shipped
//! navmeshes decide standability and component reachability; a missing mesh
//! is a test failure, never a silent skip.

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::{NavMesh, PathStatus};

use super::*;

const PLAZA: [f32; 3] = [100.0, -17.4, 230.0];
const INTERIOR: [f32; 3] = [71.0, 0.05, 30.0];
const COURT: [f32; 3] = [141.0, -20.8, 288.0];

const FLAPS: [(i32, i32, i32, &str, [f32; 3]); 4] = [
    (
        8000,
        61,
        445,
        "Dakara_E1_TentFlap_ToCommand",
        [121.5, -19.04, 265.2],
    ),
    (
        8001,
        61,
        446,
        "Dakara_E1_TentFlap_ToMohkatan",
        [164.0, -21.33, 276.0],
    ),
    (
        8002,
        62,
        447,
        "Dakara_E1_StoryRm_TentFlap_FromCommand",
        [70.74, 0.0, 39.4],
    ),
    (
        8003,
        62,
        448,
        "Dakara_E1_StoryRm_TentFlap_FromMohkatan",
        [70.52, 0.05, 34.5],
    ),
];

const AREAS: [(i32, &str, [f32; 3]); 4] = [
    (2123, "Dakara_E1.CommandTent", [121.5, -19.04, 265.2]),
    (2124, "Dakara_E1.MohkatanTent", [164.0, -21.33, 276.0]),
    (2125, "Dakara_E1.StargatePlaza", PLAZA),
    (
        2126,
        "Dakara_E1.SuperweaponCourtyard",
        [129.4, -21.53, -66.5],
    ),
];

fn mesh(name: &str) -> NavMesh {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/spaces")
        .join(name);
    NavMesh::load(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn vector(p: [f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

fn reaches(mesh: &NavMesh, from: [f32; 3], to: [f32; 3]) -> bool {
    let path = mesh.find_path(&vector(from), &vector(to));
    matches!(path.status, PathStatus::Ok)
        && path
            .waypoints
            .last()
            .is_some_and(|end| end.distance_to(&vector(to)) < 2.0)
}

fn standable_and_connected(mesh: &NavMesh, point: [f32; 3], anchor: [f32; 3], label: &str) {
    assert!(
        mesh.is_point_valid(&vector(point)),
        "{label} is off the shipped navmesh"
    );
    assert!(
        reaches(mesh, point, anchor),
        "{label} is on a different navmesh component than {anchor:?}"
    );
}

#[tokio::test]
async fn live_db_dakara_tent_flaps_areas_and_arrivals_match_the_placement_ledger() {
    let pool = require_db_or_skip!();
    let exterior = mesh("dakara_e1.nav");
    let interior = mesh("dakara_e1_storyrm.nav");

    for (spawn_id, world_id, template_id, tag, expected) in FLAPS {
        let row: (i32, i32, String, f32, f32, f32, bool) = sqlx::query_as(
            "SELECT world_id, template_id, tag::text, x, y, z, is_stationary \
             FROM resources.spawnlist WHERE spawn_id = $1",
        )
        .bind(spawn_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("flap spawn {spawn_id}: {e}"));
        assert_eq!(
            (row.0, row.1, row.2.as_str(), row.6),
            (world_id, template_id, tag, true)
        );
        let actual = [row.3, row.4, row.5];
        assert_eq!(
            actual, expected,
            "flap {spawn_id} moved: update the DK-02 ledger"
        );
        if world_id == 61 {
            standable_and_connected(&exterior, actual, PLAZA, tag);
        } else {
            standable_and_connected(&interior, actual, INTERIOR, tag);
        }
    }

    for (set_id, name, expected) in AREAS {
        let row: (String, i32, i32, f32, f32, f32) = sqlx::query_as(
            "SELECT s.name::text, s.world_id, s.flags, p.x, p.y, p.z \
             FROM resources.point_sets s JOIN resources.point_set_points p USING (set_id) \
             WHERE s.set_id = $1",
        )
        .bind(set_id)
        .fetch_one(&pool)
        .await
        .unwrap_or_else(|e| panic!("area {set_id}: {e}"));
        assert_eq!((row.0.as_str(), row.1, row.2), (name, 61, 1));
        let actual = [row.3, row.4, row.5];
        assert_eq!(
            actual, expected,
            "area {set_id} moved: update the DK-02 ledger"
        );
        standable_and_connected(&exterior, actual, PLAZA, name);
    }

    // Both chains into the room use respawner 25's front-room floor. Both
    // exits use the one open court DK-02 placed between adjacent tents.
    standable_and_connected(&interior, INTERIOR, [70.74, 0.0, 39.4], "interior arrival");
    standable_and_connected(&exterior, COURT, PLAZA, "shared exterior return");
}

#[tokio::test]
async fn live_db_dakara_respawners_are_in_their_tents_and_none_is_at_origin() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, i32, String, f32, f32, f32)> = sqlx::query_as(
        "SELECT respawner_id, world_id, name::text, pos_x, pos_y, pos_z \
         FROM resources.respawners WHERE world_id IN (61, 62) ORDER BY respawner_id",
    )
    .fetch_all(&pool)
    .await
    .expect("Dakara respawners");
    assert_eq!(
        rows,
        vec![
            (25, 62, "Dakara Story Room Respawn".into(), 71.0, 0.05, 30.0),
            (610, 61, "Dakara Gate Plaza".into(), 100.0, -17.4, 230.0),
            (611, 61, "Med Tent".into(), 121.6, -21.08, 283.2),
        ],
        "the story-room and Med Tent respawners must match DK-02's floor points"
    );
    for (id, world, _, x, y, z) in rows {
        assert_ne!(
            [x, y, z],
            [0.0; 3],
            "world {world} respawner {id} is at origin"
        );
    }
    let exterior = mesh("dakara_e1.nav");
    let interior = mesh("dakara_e1_storyrm.nav");
    standable_and_connected(
        &exterior,
        [121.6, -21.08, 283.2],
        PLAZA,
        "Med Tent respawner",
    );
    standable_and_connected(&interior, INTERIOR, [70.52, 0.05, 34.5], "respawner 25");
}
