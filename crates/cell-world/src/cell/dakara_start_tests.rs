//! Seed-vs-navmesh guard for the Free Jaffa start on Dakara_E1 (Class Start
//! v6, CS-02): the `SGU_FREE_JAFFA` start profile (char_defs 8 and 18) and
//! respawner 610, both on the gate plaza.
//!
//! Same pairing as `room_placement_tests` and `debug_area_gate_tests`: the
//! coordinates come from the seeded database and the real
//! `data/spaces/dakara_e1.nav` judges them, so the test fails if a row is
//! dropped or moved off the plaza. `is_point_valid` alone passes on the
//! wrong mesh component (agent memory `navmesh-onmesh-assertions-are-weak`),
//! so the guard also walks a path from the start to the gate and checks it
//! arrives, with a control point the mesh accepts but cannot reach.

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::{NavMesh, PathStatus};

use crate::cell::spawner::{is_point_in_region, load_regions_from_db, load_respawners};
use crate::test_support::require_db_or_skip;

/// `respawners` row for the plaza.
const PLAZA_RESPAWNER: i32 = 610;
/// The Free Jaffa char_defs.
const FREE_JAFFA: [i32; 2] = [8, 18];
/// Stargate 25, the Dakara E1 gate prop.
const GATE: [f32; 3] = [96.174, -15.164, 253.206];
/// Spawn 38, the Dakara E1 DHD prop.
const DHD: [f32; 3] = [98.087, -16.769, 237.335];
/// `point_sets` row for the gate volume.
const GATE_VOLUME: i32 = 1005;

fn dakara_mesh() -> Option<NavMesh> {
    let path = std::path::Path::new("../../data/spaces/dakara_e1.nav");
    path.exists()
        .then(|| NavMesh::load(path).expect("dakara_e1.nav loads"))
}

/// The centre plus twelve points on the agent-radius ring that are off the
/// mesh (empty when the disc is standable).
fn disc_off_mesh(mesh: &NavMesh, c: [f32; 3], radius: f32) -> Vec<[f32; 3]> {
    let mut off = Vec::new();
    let mut probe = |p: [f32; 3]| {
        if !mesh.is_point_valid(&Vector3::new(p[0], p[1], p[2])) {
            off.push(p);
        }
    };
    probe(c);
    for i in 0..12 {
        let a = i as f32 * std::f32::consts::TAU / 12.0;
        probe([c[0] + radius * a.cos(), c[1], c[2] + radius * a.sin()]);
    }
    off
}

/// Whether a path from `from` really ends at `to` (within 2 m), not a
/// partial corridor stopping at the nearest point of another component.
fn reaches(mesh: &NavMesh, from: [f32; 3], to: [f32; 3]) -> bool {
    let a = Vector3::new(from[0], from[1], from[2]);
    let b = Vector3::new(to[0], to[1], to[2]);
    let outcome = mesh.find_path(&a, &b);
    if !matches!(outcome.status, PathStatus::Ok) {
        return false;
    }
    outcome
        .waypoints
        .last()
        .is_some_and(|end| end.distance_to(&b) < 2.0)
}

fn dist_xz(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// **Guard (CS-02).** Both Free Jaffa profiles start on Dakara_E1 at the
/// plaza respawner's point, which stands on `dakara_e1.nav`, walks to the
/// gate, keeps clear of the DHD prop and lies outside the gate volume.
/// Moving the start back to SGC_W1, dropping respawner 610, or nudging the
/// point off the plaza each fail it.
#[tokio::test]
async fn dakara_free_jaffa_start_is_on_the_gate_plaza_live_db() {
    let pool = require_db_or_skip!();

    let respawners = load_respawners(&pool).await.expect("load_respawners");
    let plaza = respawners
        .iter()
        .find(|r| r.respawner_id == PLAZA_RESPAWNER)
        .expect("respawner 610 (Dakara Gate Plaza) is seeded");
    assert_eq!(plaza.world_name, "Dakara_E1");
    let start = plaza.pos;
    assert_ne!(start, [0.0; 3]);

    for char_def in FREE_JAFFA {
        let (world, x, y, z): (String, f32, f32, f32) = sqlx::query_as(
            "SELECT starting_world::text, starting_x, starting_y, starting_z \
               FROM resources.char_creation WHERE char_def_id = $1",
        )
        .bind(char_def)
        .fetch_one(&pool)
        .await
        .expect("Free Jaffa start profile");
        assert_eq!(world, "Dakara_E1", "char_def {char_def}");
        assert_eq!([x, y, z], start, "char_def {char_def} starts at the plaza");
    }

    assert!(dist_xz(start, DHD) > 3.0, "clear of the DHD prop");
    let regions = load_regions_from_db(&pool).await.expect("load regions");
    let volume = regions
        .iter()
        .find(|r| r.set_id == GATE_VOLUME)
        .expect("point set 1005 Dakara_E1.Stargate is seeded");
    assert!(
        is_point_in_region(&volume.points, GATE),
        "control: the gate is inside its own volume"
    );
    assert!(
        !is_point_in_region(&volume.points, start),
        "the start must not be inside the gate volume"
    );

    let Some(mesh) = dakara_mesh() else {
        return;
    };
    let off = disc_off_mesh(&mesh, start, 0.6);
    assert!(off.is_empty(), "start {start:?} is not standable: {off:?}");
    assert!(
        reaches(&mesh, start, DHD) && reaches(&mesh, start, GATE),
        "the start must be on the gate plaza's component: it walks to the DHD and the gate"
    );
    // Control: the guard can still say no. CONTROL is on the mesh (component
    // 473 by nav_inspect, 250 m west of the plaza's component 279), so
    // `is_point_valid` accepts it, but the plaza cannot walk there.
    assert!(
        mesh.is_point_valid(&Vector3::new(CONTROL[0], CONTROL[1], CONTROL[2])),
        "control: {CONTROL:?} is on the mesh"
    );
    assert!(
        !reaches(&mesh, start, CONTROL),
        "control: a point on another component must not count as reachable"
    );
}

/// A walkable point of `dakara_e1.nav` on another component than the plaza.
const CONTROL: [f32; 3] = [-152.0, -20.55, 300.0];
