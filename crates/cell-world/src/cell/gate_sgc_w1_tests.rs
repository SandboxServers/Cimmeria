//! Seed guards for the SGC_W1 end of real gate travel (Dakara rebuild
//! DK-SGC-01): stargate 27's arrival pin on the Gate Room floor.
//!
//! Same pairing as `gate_dakara_e1_tests`: the row comes from the seeded
//! database and the real `data/spaces/sgc_w1.nav` judges the coordinate,
//! loaded by path because the space loader resolves `.nav` files against the
//! process CWD. `is_point_valid` alone passes on the wrong mesh component, so
//! the pin guard also walks a path from the pin to the Control Room, with a
//! control point the mesh accepts but the Gate Room cannot reach.

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::{NavMesh, PathStatus};

use crate::cell::space_manager::REGION_FLAG_STARGATE;
use crate::cell::spawner::{is_point_in_region, load_regions_from_db, load_stargates};
use crate::test_support::require_db_or_skip;

/// Stargate 27, `SGC W1`.
const SGC_GATE: i32 = 27;
/// The arrival pin as seeded: the Union SGC campaign's sealed P13.
const PIN: [f32; 3] = [167.00, -10.80, 18.00];
const PIN_YAW: f32 = 0.0;
/// The Gate Room and Requisitions floor (placement.md, P10 and P13).
const GATE_ROOM_FLOOR_Y: f32 = -10.85;
/// P01, Harriman's Control Room spot, beside the Gate Control screen.
const CONTROL_ROOM: [f32; 3] = [172.08, -6.40, 46.70];
/// P05, the Failsafe: on the mesh (component 3), not reachable from the Gate Room (47).
const CONTROL: [f32; 3] = [147.00, 1.30, -197.00];

fn sgc_mesh() -> NavMesh {
    let path = std::path::Path::new("../../data/spaces/sgc_w1.nav");
    NavMesh::load(path).expect("sgc_w1.nav must be present and load for the arrival pin guard")
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

/// **Guard (DK-SGC-01): gate 27's arrival pin.** The pin is the seeded point,
/// on the Gate Room floor, in front of the gate and off its axis, facing into
/// the room, outside every gate volume on its world, standing on
/// `sgc_w1.nav` on the Control Room's component, and it walks to the Control
/// Room.
///
/// Drop the pin and the traveller arrives on the gate row, inside the gate
/// volume: the "arrival pin" assertion fails. Move it into the volume, off the
/// floor, off the mesh or onto another component and its own assertion fails.
#[tokio::test]
async fn live_db_sgc_w1_gate_arrival_pin_is_on_the_gate_room_floor_outside_the_gate_volume() {
    let pool = require_db_or_skip!();
    let gates = load_stargates(&pool).await.expect("load_stargates");
    let gate = gates
        .get(&SGC_GATE)
        .expect("stargate 27 (SGC W1) is seeded");
    assert_eq!(gate.world_name, "SGC_W1");
    let row = [gate.x, gate.y, gate.z];

    assert!(
        gate.arrival.is_some(),
        "gate 27 lost its arrival pin: travellers land on the gate row again, \
         inside the gate's own volume"
    );
    let (arrival, yaw) = gate.desired_arrival();
    assert_eq!(arrival, PIN, "the pin moved: re-measure it and update PIN");
    assert_eq!(yaw, PIN_YAW, "the pin's yaw moved: update PIN_YAW");

    // In front of the gate, and facing into the room. The gate row's yaw is
    // its facing: (sin, cos) in (x, z), 0 = +Z. The axis strip has no floor
    // polygon, so the pin is 26 degrees off it: the threshold is 0.85, not 0.99.
    let out = [arrival[0] - row[0], arrival[2] - row[2]];
    let out_len = dist_xz(arrival, row);
    let along = |facing: f32| (out[0] * facing.sin() + out[1] * facing.cos()) / out_len;
    assert!(
        along(gate.yaw) > 0.85,
        "the pin is not in front of the gate (gate yaw {})",
        gate.yaw
    );
    assert!(
        along(yaw) > 0.85,
        "arrival yaw {yaw} does not face away from the gate"
    );
    assert!(
        (5.0..=15.0).contains(&out_len),
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

    let mesh = sgc_mesh();
    assert!(
        (arrival[1] - GATE_ROOM_FLOOR_Y).abs() <= 0.25,
        "the pin's y {} is not on the Gate Room floor ({GATE_ROOM_FLOOR_Y})",
        arrival[1]
    );
    for radius in [0.6, 1.2] {
        let off = disc_off_mesh(&mesh, arrival, radius);
        assert!(
            off.is_empty(),
            "the pin {arrival:?} is not standable at r = {radius}: {off:?}"
        );
    }
    assert!(
        reaches(&mesh, arrival, CONTROL_ROOM),
        "the pin must be on the Control Room's component: it does not walk to Harriman's spot"
    );
    // Control: the guard can still say no.
    assert!(mesh.is_point_valid(&v(CONTROL)), "control is on the mesh");
    assert!(
        !reaches(&mesh, arrival, CONTROL),
        "control: a point on another component must not count as reachable"
    );
}
