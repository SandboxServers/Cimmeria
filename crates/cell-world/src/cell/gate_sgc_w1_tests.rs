//! Seed guards for the SGC_W1 end of real gate travel (Dakara rebuild
//! DK-SGC-01 and DK-SGC-02): stargate 27's arrival pin on the Gate Room
//! floor, the SGC Gate Control screen that opens the dial window, and the way
//! back from every address a content chain grants.
//!
//! Same pairing as `gate_dakara_e1_tests`: the rows come from the seeded
//! database and the real `data/spaces/sgc_w1.nav` judges the coordinate,
//! loaded by path because the space loader resolves `.nav` files against the
//! process CWD. `is_point_valid` alone passes on the wrong mesh component, so
//! the pin guard also walks a path from the pin to the Control Room, with a
//! control point the mesh accepts but the Gate Room cannot reach.

use cimmeria_common::Vector3;
use cimmeria_entity::interaction_flags::INT_DHD;
use cimmeria_entity::navigation::{NavMesh, PathStatus};

use crate::cell::space_manager::{SpaceManager, MAX_INTERACT_DISTANCE, REGION_FLAG_STARGATE};
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

fn repo_file(rel: &str) -> String {
    let path = format!("{}/../../{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
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

/// **Guard (DK-SGC-02, D-DK40): SGC_W1 has one dial terminal, beside
/// Harriman.** The Gate Control screen (spawn 68, template 37) is the only
/// always-spawned prop on world 58 that carries `INT_DHD`, so a right-click
/// there opens the dial window. The press is within interact range of
/// Harriman's Control Room spot, and the arrival pin walks there.
///
/// Clear the seed bit and the list is empty. Move the screen, or add a second
/// prop that carries the bit, and its assertion fails.
#[tokio::test]
async fn live_db_sgc_w1_has_one_dial_terminal_beside_harriman() {
    let pool = require_db_or_skip!();

    let terminals: Vec<(i32, String, i32, f32, f32, f32)> = sqlx::query_as(
        "SELECT s.spawn_id, s.tag::text, s.template_id, s.x, s.y, s.z \
           FROM resources.spawnlist s \
           JOIN resources.entity_templates t ON t.template_id = s.template_id \
          WHERE s.world_id = 58 AND t.interaction_type & $1 <> 0 AND s.set_name IS NULL \
          ORDER BY s.spawn_id",
    )
    .bind(INT_DHD)
    .fetch_all(&pool)
    .await
    .expect("SGC_W1 dial terminals");
    assert_eq!(
        terminals.len(),
        1,
        "SGC_W1 has no dial terminal: a player who dials gate 27 cannot dial home (D-DK40): {terminals:?}"
    );
    let (spawn, tag, template, x, y, z) = &terminals[0];
    assert_eq!(
        (*spawn, tag.as_str(), *template),
        (68, "SGC_W1_GateTerminal1", 37),
        "SGC_W1 has no dial terminal: a player who dials gate 27 cannot dial home (D-DK40)"
    );

    let d = ((x - CONTROL_ROOM[0]).powi(2)
        + (y - CONTROL_ROOM[1]).powi(2)
        + (z - CONTROL_ROOM[2]).powi(2))
    .sqrt();
    assert!(
        d <= MAX_INTERACT_DISTANCE,
        "the dial terminal is {d} m from the Control Room spot, beyond the {MAX_INTERACT_DISTANCE} m interact range"
    );
    assert!(
        reaches(&sgc_mesh(), PIN, CONTROL_ROOM),
        "the arrival pin does not walk to the Control Room, where the terminal is"
    );

    // Gate 27 is the only gate a dial from this world can show: one glyph.
    let gates = load_stargates(&pool).await.expect("load_stargates");
    let mut offered: Vec<i32> = gates
        .iter()
        .filter(|(_, g)| g.world_name == "SGC_W1" && !g.debug_dial_hub)
        .map(|(id, _)| *id)
        .collect();
    offered.sort_unstable();
    assert_eq!(
        offered,
        [SGC_GATE],
        "SGC_W1's non-hub gates changed: the dial window's glyph is gate 27's"
    );
    let gate = gates
        .get(&SGC_GATE)
        .expect("stargate 27 (SGC W1) is seeded");
    assert!(
        (1..=38).contains(&gate.address_origin),
        "gate 27's address origin {} is not a gate glyph (1 to 38): the window would have none",
        gate.address_origin
    );
}

/// **Guard (DK-SGC-02, D-DK40): no address grant is a one-way trip.** For
/// every enabled content chain that grants a stargate address, on any
/// trigger:
///
/// * the granted gate exists, is not a dial hub, and is on a world this
///   server can enter;
/// * that world has a DHD (a dial window) to dial home from.
///
/// This is the general form of the `player_loaded` guard in
/// `gate_dakara_e1_tests`. It also covers the click and offer grants of gate
/// 27 (chains 8313, 8432 and 8433), which that guard does not see (F44).
/// Point chain 8001 at gate 27 and this fails, with "which has no DHD: a
/// one-way trip".
#[tokio::test]
async fn live_db_every_granted_address_has_a_way_back() {
    let pool = require_db_or_skip!();

    let grants: Vec<(i32, Option<i32>)> = sqlx::query_as(
        "SELECT DISTINCT a.chain_id, a.target_id \
           FROM resources.content_actions a \
           JOIN resources.content_chains c ON c.chain_id = a.chain_id \
          WHERE a.action_type::text = 'grant_stargate_address' AND c.enabled \
          ORDER BY a.chain_id",
    )
    .fetch_all(&pool)
    .await
    .expect("enabled address grants");

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(&repo_file("entities/spaces.xml"))
        .expect("spaces.xml parses");
    mgr.create_startup_spaces(&repo_file("entities/cell_spaces.xml"))
        .expect("cell_spaces.xml creates its spaces");
    let gates = load_stargates(&pool).await.expect("load_stargates");
    let dhds = dhd_spawns(&pool).await;
    let has_dhd = |world: &str| dhds.iter().any(|(w, _)| w == world);

    for (chain_id, target) in &grants {
        let target =
            target.unwrap_or_else(|| panic!("chain {chain_id} grants no stargate (NULL target)"));
        let gate = gates.get(&target).unwrap_or_else(|| {
            panic!("chain {chain_id} grants stargate {target}, which has no row")
        });
        assert!(
            !gate.debug_dial_hub,
            "chain {chain_id} grants a dial hub, which nobody may hold"
        );
        let world = gate.world_name.as_str();
        assert!(
            mgr.world_is_enterable(world),
            "chain {chain_id} grants stargate {target} on {world}, which this server cannot enter"
        );
        assert!(
            has_dhd(world),
            "chain {chain_id} grants stargate {target} on {world}, which has no DHD: a one-way trip"
        );
    }

    // Control, after the loop: the loop must fail first when chain 8001 changes.
    assert!(
        grants.contains(&(8001, Some(5))),
        "control: chain 8001 must grant Omega Site (DK-01), got {grants:?}"
    );
}
