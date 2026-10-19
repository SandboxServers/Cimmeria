//! Seed guards for the Beta_Site_Evo_1 gate round trip (UB-01a): stargate 6's
//! arrival pin, and the two World 23 respawners that replaced the origin
//! placeholders.
//!
//! Same pairing as `gate_dakara_e1_tests`: the rows come from the seeded
//! database and the real `data/spaces/beta_site_evo_1.nav` judges the
//! coordinate, loaded by path because the space loader resolves `.nav` files
//! against the process CWD. `is_point_valid` alone passes on the wrong mesh
//! component, so the guards also walk paths, with a control point the mesh
//! accepts but the pin cannot reach.

use cimmeria_common::Vector3;
use cimmeria_entity::interaction_flags::INT_DHD;
use cimmeria_entity::navigation::{NavMesh, PathStatus};

use crate::cell::space_manager::REGION_FLAG_STARGATE;
use crate::cell::spawner::{
    is_point_in_region, load_regions_from_db, load_respawners, load_stargates,
};
use crate::test_support::require_db_or_skip;

/// Stargate 6, `Beta Site E1`.
const BETA_GATE: i32 = 6;
/// The arrival pin, which is the map's only PlayerStart (UB-00b seal
/// ARRIVAL, nav component 251, y floats 0.70 m above the mesh).
const PIN: [f32; 3] = [635.761, -60.134, 388.448];
/// A walkable point of `beta_site_evo_1.nav` on the west valley, component
/// 70 (RAS_NOR from UB-00b). There is no walk route from component 251.
const CONTROL: [f32; 3] = [-545.100, 6.77, 141.500];
/// The DHD prop of spawn 40 (`Beta_Site_Evo_1_DHD`), the one the arrival
/// walks to next.
const DHD_SPAWN: [f32; 3] = [629.596, -60.913, 381.289];
/// The World 23 respawner rows: `Respawner` and `Respawner_bet_Infirmary`.
const BETA_RESPAWNERS: [i32; 2] = [6, 7];

fn beta_mesh() -> NavMesh {
    let path = std::path::Path::new("../../data/spaces/beta_site_evo_1.nav");
    NavMesh::load(path).expect("beta_site_evo_1.nav must be present and load for the Beta guards")
}

fn v(p: [f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

/// The centre plus twelve points on the ring of `radius` that are off the
/// mesh (empty when the disc is standable).
fn disc_off_mesh(mesh: &NavMesh, c: [f32; 3], radius: f32) -> Vec<[f32; 3]> {
    let mut ring = vec![c];
    for i in 0..12 {
        let a = i as f32 * std::f32::consts::TAU / 12.0;
        ring.push([c[0] + radius * a.cos(), c[1], c[2] + radius * a.sin()]);
    }
    ring.retain(|p| !mesh.is_point_valid(&v(*p)));
    ring
}

/// Whether a path from `from` really ends at `to` (within 2 m), not a
/// partial corridor stopping at the nearest point of another component.
fn reaches(mesh: &NavMesh, from: [f32; 3], to: [f32; 3]) -> bool {
    let outcome = mesh.find_path(&v(from), &v(to));
    matches!(outcome.status, PathStatus::Ok)
        && outcome
            .waypoints
            .last()
            .is_some_and(|end| end.distance_to(&v(to)) < 2.0)
}

fn dist_xz(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// `(world, position)` of every always-spawned prop whose template carries
/// `INT_DHD`: the things a player can dial from.
async fn dhd_spawns(pool: &sqlx::PgPool) -> Vec<(String, [f32; 3])> {
    let rows: Vec<(String, f32, f32, f32)> = sqlx::query_as(
        "SELECT w.world::text, s.x, s.y, s.z FROM resources.spawnlist s \
           JOIN resources.entity_templates t ON t.template_id = s.template_id \
           JOIN resources.worlds w ON w.world_id = s.world_id \
          WHERE t.interaction_type & $1 <> 0 AND s.set_name IS NULL",
    )
    .bind(INT_DHD)
    .fetch_all(pool)
    .await
    .expect("DHD spawns");
    rows.into_iter()
        .map(|(world, x, y, z)| (world, [x, y, z]))
        .collect()
}

/// **Guard (UB-01a): gate 6's arrival pin.** The pin is the seeded point,
/// in front of the gate and facing away from it, outside every gate volume
/// on its world, clear of the DHD prop, standing on `beta_site_evo_1.nav`
/// on component 251, and walkable to the gate row and the DHD but not to
/// the control point on the west valley.
///
/// Revert the UB-01a gate 6 row (drop the arrival columns) and the
/// "lost its arrival pin" assertion fails: travellers land on the gate row,
/// inside the gate's own volume. Move the pin into the volume, onto the DHD,
/// off the mesh or onto another component and its own assertion fails.
#[tokio::test]
async fn live_db_beta_gate_arrival_pin_is_on_component_251_outside_the_gate_volume() {
    let pool = require_db_or_skip!();
    let gates = load_stargates(&pool).await.expect("load_stargates");
    let gate = gates
        .get(&BETA_GATE)
        .expect("stargate 6 (Beta Site E1) is seeded");
    assert_eq!(gate.world_name, "Beta_Site_Evo_1");
    let row = [gate.x, gate.y, gate.z];

    assert!(
        gate.arrival.is_some(),
        "gate 6 lost its arrival pin: travellers land on the gate row again, \
         inside the gate's own volume"
    );
    let (arrival, yaw) = gate.desired_arrival();
    assert_eq!(arrival, PIN, "the pin moved: re-measure it and update PIN");

    // In front of the gate, on its axis, and facing on away from it. The
    // gate row's yaw is its facing: (sin, cos) in (x, z), 0 = +Z.
    let out = [arrival[0] - row[0], arrival[2] - row[2]];
    let out_len = (out[0] * out[0] + out[1] * out[1]).sqrt();
    let along = |facing: f32| (out[0] * facing.sin() + out[1] * facing.cos()) / out_len;
    assert!(
        along(gate.yaw) > 0.99,
        "the pin is not on the gate's facing axis (gate yaw {})",
        gate.yaw
    );
    assert!(
        along(yaw) > 0.99,
        "arrival yaw {yaw} does not face away from the gate"
    );
    assert!(
        (15.0..=20.0).contains(&out_len),
        "the pin is {out_len} m from the gate row"
    );

    // Outside every stargate volume on the world, by relationship.
    let regions = load_regions_from_db(&pool).await.expect("load regions");
    let volumes: Vec<_> = regions
        .iter()
        .filter(|r| r.world_name == gate.world_name && r.flags & REGION_FLAG_STARGATE != 0)
        .collect();
    assert!(
        volumes.iter().any(|r| is_point_in_region(&r.points, row)),
        "control: the gate row is inside a stargate volume on its own world"
    );
    for volume in &volumes {
        assert!(
            !is_point_in_region(&volume.points, arrival),
            "the pin is inside gate volume {} ({})",
            volume.set_id,
            volume.name
        );
    }

    // Clear of the DHD prop the traveller walks to next.
    let dhds: Vec<[f32; 3]> = dhd_spawns(&pool)
        .await
        .into_iter()
        .filter(|(world, _)| *world == gate.world_name)
        .map(|(_, pos)| pos)
        .collect();
    assert!(!dhds.is_empty(), "Beta_Site_Evo_1 has a DHD");
    assert!(
        dhds.iter().any(|d| dist_xz(*d, DHD_SPAWN) < 0.01),
        "DHD spawn 40 is no longer a DHD on Beta_Site_Evo_1: {dhds:?}"
    );
    for dhd in &dhds {
        assert!(dist_xz(arrival, *dhd) > 3.0, "the pin is on the DHD prop");
    }

    let mesh = beta_mesh();
    assert!(
        (arrival[1] - PIN[1]).abs() <= 0.25,
        "the pin's y {} is not on the mesh level ({})",
        arrival[1],
        PIN[1]
    );
    for radius in [0.6, 1.2] {
        let off = disc_off_mesh(&mesh, arrival, radius);
        assert!(
            off.is_empty(),
            "the pin {arrival:?} is not standable at r = {radius}: {off:?}"
        );
    }
    assert!(
        reaches(&mesh, arrival, row),
        "the pin must be on the gate's component: it walks to the gate row"
    );
    assert!(
        reaches(&mesh, arrival, DHD_SPAWN),
        "the pin walks to the DHD (spawn 40)"
    );
    // Control: the guard can still say no.
    assert!(mesh.is_point_valid(&v(CONTROL)), "control is on the mesh");
    assert!(
        !reaches(&mesh, arrival, CONTROL),
        "control: a point on another component must not count as reachable"
    );
}

/// **Guard (UB-01a): respawners 6 and 7 stand on the Beta mesh.** Both World
/// 23 rows are authored, off the world origin, standable at r = 0.6 and
/// walkable to the arrival pin.
///
/// Revert either row to (0, 0, 0) (the pre-UB-01a placeholder) and the
/// origin assertion fails; the respawner fallback then fires for every
/// death on Beta. Move a row off the mesh or onto another component and
/// the standable or reachability assertion fails.
#[tokio::test]
async fn live_db_beta_respawners_stand_on_component_251_and_reach_the_gate() {
    let pool = require_db_or_skip!();
    let respawners = load_respawners(&pool)
        .await
        .expect("load_respawners must succeed");
    let beta: Vec<_> = respawners
        .iter()
        .filter(|r| r.world_name == "Beta_Site_Evo_1")
        .collect();
    let mut ids: Vec<i32> = beta.iter().map(|r| r.respawner_id).collect();
    ids.sort_unstable();
    assert_eq!(
        ids, BETA_RESPAWNERS,
        "world 23 must ship exactly respawners 6 and 7"
    );

    let mesh = beta_mesh();
    for r in &beta {
        assert_ne!(
            r.pos,
            [0.0, 0.0, 0.0],
            "respawner {} '{}' is still at the world origin",
            r.respawner_id,
            r.name
        );
        let off = disc_off_mesh(&mesh, r.pos, 0.6);
        assert!(
            off.is_empty(),
            "respawner {} '{}' at {:?} is not standable at r = 0.6: {off:?}",
            r.respawner_id,
            r.name,
            r.pos
        );
        assert!(
            reaches(&mesh, r.pos, PIN),
            "respawner {} '{}' at {:?} does not reach the arrival pin",
            r.respawner_id,
            r.name,
            r.pos
        );
    }
}
