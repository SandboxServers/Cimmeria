//! Seed and navmesh guards for Union Omega (worlds 18 `Omega_Site` and 80
//! `Omega_Site_CmdCenter`), chains and spawns 63800-63999.
//!
//! - [`hub`]: UO-03, the hub's gate plaza respawner and placements (world 18).
//! - [`cmdcenter`]: UO-05, the Command Center's cast and placements (world 80).
//! - [`ambient`]: UO-14, the droppable ambient spawns (63880-63959).
//!
//! The helpers below are copied from `dakara_e1/travel.rs`, not imported:
//! that module's items are private to `dakara_e1`.

mod ambient;
mod cmdcenter;
mod hub;

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::{NavMesh, PathStatus};

use crate::test_support::require_db_or_skip;

const HUB_MESH: &str = "omega_site.nav";
const CMD_MESH: &str = "omega_site_cmdcenter.nav";
/// Modi (spawn 78), the hub's shipped anchor.
const MODI: [f32; 3] = [39.544, -51.914, -129.92];
/// Respawner 26, the Command Center's shipped anchor.
const RESPAWNER_26: [f32; 3] = [10.0, 35.23, 9.987];

pub(super) fn mesh(name: &str) -> NavMesh {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/spaces")
        .join(name);
    NavMesh::load(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn vector(p: [f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

/// Most corridor hops [`reaches`] follows. The hub is about 400 m across and
/// one Detour corridor holds 256 polygons, so a far point needs a few hops.
const MAX_HOPS: usize = 16;

/// Whether `to` is reachable from `from` on `mesh`.
///
/// One `find_path` corridor is capped at 256 polygons, so a goal on the same
/// component but far away (Omega's Jaffa wing is about 260 m from Modi)
/// comes back `Partial`, ending where the corridor ran out. A `Partial` path
/// is followed from its last waypoint, up to [`MAX_HOPS`] times. A goal on
/// another component makes no progress between hops, so it still fails.
fn reaches(mesh: &NavMesh, from: [f32; 3], to: [f32; 3]) -> bool {
    let goal = vector(to);
    let mut at = vector(from);
    for _ in 0..MAX_HOPS {
        let path = mesh.find_path(&at, &goal);
        let Some(end) = path.waypoints.last().copied() else {
            return false;
        };
        match path.status {
            PathStatus::Ok => return end.distance_to(&goal) < 2.0,
            PathStatus::Partial if end.distance_to(&at) > 1.0 => at = end,
            _ => return false,
        }
    }
    false
}

/// On the shipped navmesh, with no claim about which component it is on.
pub(super) fn standable(mesh: &NavMesh, point: [f32; 3], label: &str) {
    assert!(
        mesh.is_point_valid(&vector(point)),
        "{label} is off the shipped navmesh"
    );
}

pub(super) fn standable_and_connected(
    mesh: &NavMesh,
    point: [f32; 3],
    anchor: [f32; 3],
    label: &str,
) {
    standable(mesh, point, label);
    assert!(
        reaches(mesh, point, anchor),
        "{label} is on a different navmesh component than {anchor:?}"
    );
}

/// Both shipped Omega meshes load, and the two shipped anchors every later
/// placement test reaches stand on them. A missing mesh is a failure, never
/// a skip.
#[test]
fn omega_meshes_load_and_the_shipped_anchors_stand_on_them() {
    let hub = mesh(HUB_MESH);
    standable_and_connected(&hub, MODI, MODI, "Modi (spawn 78)");
    let cmd = mesh(CMD_MESH);
    standable_and_connected(&cmd, RESPAWNER_26, RESPAWNER_26, "respawner 26");
}

/// `reaches` follows a long corridor across the hub, and still refuses a
/// point on another component. The Jaffa wing is about 260 m from Modi, past
/// one 256-polygon corridor. Bridge E in world 80 stands on component 12,
/// with no route from the pad's component 11 (UO-01 worknote). The hub island
/// point checks the same refusal in the world that needs hops.
#[test]
fn reaches_follows_long_corridors_and_refuses_other_components() {
    let hub = mesh(HUB_MESH);
    let market = [297.22, -61.16, -106.85];
    standable(&hub, market, "Jaffa market point");
    assert!(reaches(&hub, MODI, market), "Modi reaches the Jaffa market");
    // A hub point beside the arena that is on the mesh but on another component:
    // the hop loop must stop, not creep to it.
    let island = [200.0, -62.0, -100.0];
    standable(&hub, island, "hub island point");
    assert!(
        !reaches(&hub, MODI, island),
        "the hub island point is on another component than Modi"
    );
    let cmd = mesh(CMD_MESH);
    let bridge_e = [144.78, 30.0, 10.22];
    standable(&cmd, bridge_e, "bridge E");
    assert!(
        !reaches(&cmd, RESPAWNER_26, bridge_e),
        "bridge E is on another component than respawner 26"
    );
}

/// Omega's footers lift every sequence it reserves past its block, so a
/// GM-made row never takes an Omega id.
#[tokio::test]
async fn live_db_omega_sequences_clear_the_block() {
    let pool = require_db_or_skip!();
    // One literal query: the sqlx SQL-safety lint rejects a `format!`ed name.
    let (templates, spawns, lists, list_rows): (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT last_value FROM resources.entity_templates_template_id_seq), \
                (SELECT last_value FROM resources.spawnlist_spawn_id_seq), \
                (SELECT last_value FROM resources.item_lists_item_list_id_seq), \
                (SELECT last_value FROM resources.item_list_items_item_id_seq)",
    )
    .fetch_one(&pool)
    .await
    .expect("sequence query");
    for (seq, last, floor) in [
        ("entity_templates_template_id_seq", templates, 63_999_i64),
        ("spawnlist_spawn_id_seq", spawns, 63_999),
        ("item_lists_item_list_id_seq", lists, 63_999),
        ("item_list_items_item_id_seq", list_rows, 639_999),
    ] {
        assert!(
            last >= floor,
            "{seq} stands at {last}, inside Omega's block up to {floor}"
        );
    }
}
/// Every Omega spawn (63800-63999) stands on its world's mesh, on the same
/// component as that world's shipped anchor, and keeps 1.5 m (x/z) from
/// every other spawn in its world. UO-05, UO-10, UO-11, UO-12 and UO-14 add
/// rows; this one test guards all of them, so none of them copies the nav
/// helpers.
#[tokio::test]
async fn live_db_every_omega_spawn_stands_on_its_mesh() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, i32, f32, f32, f32)> = sqlx::query_as(
        "SELECT spawn_id, world_id, x, y, z FROM resources.spawnlist \
          WHERE world_id IN (18, 80) ORDER BY spawn_id",
    )
    .fetch_all(&pool)
    .await
    .expect("spawnlist query");
    let hub = mesh(HUB_MESH);
    let cmd = mesh(CMD_MESH);
    for &(id, world, x, y, z) in rows.iter().filter(|r| (63_800..=63_999).contains(&r.0)) {
        let (nav, anchor) = if world == 18 {
            (&hub, MODI)
        } else {
            (&cmd, RESPAWNER_26)
        };
        standable_and_connected(nav, [x, y, z], anchor, &format!("spawn {id}"));
        for &(other, other_world, ox, _, oz) in &rows {
            if other != id && other_world == world {
                let d = ((x - ox).powi(2) + (z - oz).powi(2)).sqrt();
                assert!(d >= 1.5, "spawn {id} stands {d:.2} m from spawn {other}");
            }
        }
    }
}
