//! Seed guards for the Dakara_E1 gate round trip (Dakara rebuild DK-01):
//! stargate 25's arrival pin, and the way back from every address a
//! `player_loaded` chain hands out.
//!
//! Same pairing as `debug_area_gate_tests` and `harset_placement_tests`: the
//! rows come from the seeded database and the real
//! `data/spaces/dakara_e1.nav` judges the coordinate, loaded by path because
//! the space loader resolves `.nav` files against the process CWD.
//! `is_point_valid` alone passes on the wrong mesh component, so the pin
//! guard also walks a path to the gate, the DHD and the plaza start point,
//! with a control point the mesh accepts but the plaza cannot reach.
//!
//! Class Start v6's respawner 610 and the `SGU_FREE_JAFFA` start (CS-02) put
//! a new Free Jaffa at [`PLAZA_START`]. This file names that point as a
//! coordinate and reads no CS-02 row, so it holds before and after CS-02
//! lands.

use cimmeria_common::Vector3;
use cimmeria_entity::interaction_flags::INT_DHD;
use cimmeria_entity::navigation::{NavMesh, PathStatus};

use crate::cell::space_manager::{SpaceManager, REGION_FLAG_STARGATE};
use crate::cell::spawner::{is_point_in_region, load_regions_from_db, load_stargates};
use crate::test_support::require_db_or_skip;

/// Stargate 25, `Dakara E1`.
const DAKARA_GATE: i32 = 25;
/// The arrival pin as seeded (`db/resources/Worlds/Seed/stargates.sql`):
/// 10 m out of the gate on its own axis, on the plaza slab.
const PIN: [f32; 3] = [96.87, -16.75, 243.23];
/// The plaza floor under the pin, from `obj_slab` over the cooked chunk.
const PLAZA_FLOOR_Y: f32 = -16.80;
/// The plaza start point of Class Start v6 (respawner 610, CS-02).
const PLAZA_START: [f32; 3] = [100.0, -17.4, 230.0];
/// A walkable point of `dakara_e1.nav` on another component than the plaza
/// (component 473 by `nav_inspect`; the plaza is 279).
const CONTROL: [f32; 3] = [-152.0, -20.55, 300.0];
/// The chain DK-01 adds: `player_loaded Dakara_E1` grants Omega Site.
const DK01_GRANT: (i32, &str, i32) = (8001, "Dakara_E1", 5);

fn repo_file(rel: &str) -> String {
    let path = format!("{}/../../{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn dakara_mesh() -> NavMesh {
    let path = std::path::Path::new("../../data/spaces/dakara_e1.nav");
    NavMesh::load(path).expect("dakara_e1.nav must be present and load for the arrival pin guard")
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

/// **Guard (DK-01): gate 25's arrival pin.** The pin is the seeded point,
/// in front of the gate and facing away from it, outside every gate volume
/// on its world, clear of the DHD prop, standing on `dakara_e1.nav` on the
/// plaza's own component.
///
/// Drop the pin and the traveller arrives on the gate row, inside the gate
/// volume: the "arrival pin" assertion fails. Move it into the volume, onto
/// the DHD, off the mesh or onto another component and its own assertion
/// fails.
#[tokio::test]
async fn live_db_dakara_gate_arrival_pin_is_on_the_plaza_outside_the_gate_volume() {
    let pool = require_db_or_skip!();
    let gates = load_stargates(&pool).await.expect("load_stargates");
    let gate = gates
        .get(&DAKARA_GATE)
        .expect("stargate 25 (Dakara E1) is seeded");
    assert_eq!(gate.world_name, "Dakara_E1");
    let row = [gate.x, gate.y, gate.z];

    assert!(
        gate.arrival.is_some(),
        "gate 25 lost its arrival pin: travellers land on the gate row again, \
         inside the gate's own volume"
    );
    let (arrival, yaw) = gate.desired_arrival();
    assert_eq!(arrival, PIN, "the pin moved: re-measure it and update PIN");

    // In front of the gate, on its axis, and facing on down the plaza. The
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

    // Clear of the DHD prop the traveller walks to next.
    let dhds: Vec<[f32; 3]> = dhd_spawns(&pool)
        .await
        .into_iter()
        .filter(|(world, _)| *world == gate.world_name)
        .map(|(_, pos)| pos)
        .collect();
    assert!(!dhds.is_empty(), "Dakara_E1 has a DHD");
    for dhd in &dhds {
        assert!(dist_xz(arrival, *dhd) > 3.0, "the pin is on the DHD prop");
    }

    let mesh = dakara_mesh();
    assert!(
        (arrival[1] - PLAZA_FLOOR_Y).abs() <= 0.25,
        "the pin's y {} is not on the plaza floor ({PLAZA_FLOOR_Y})",
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
        reaches(&mesh, arrival, row) && reaches(&mesh, arrival, PLAZA_START),
        "the pin must be on the plaza's component: it walks to the gate and the start point"
    );
    for dhd in &dhds {
        assert!(reaches(&mesh, arrival, *dhd), "the pin walks to the DHD");
    }
    // Control: the guard can still say no.
    assert!(mesh.is_point_valid(&v(CONTROL)), "control is on the mesh");
    assert!(
        !reaches(&mesh, arrival, CONTROL),
        "control: a point on another component must not count as reachable"
    );
}

/// **Guard (DK-01): an address handed out on arrival always has a way
/// back.** For every enabled chain that grants a stargate address on
/// `player_loaded`:
///
/// * the trigger names its world (a wildcard would grant on every world);
/// * that world has a DHD to dial from and a gate that can be learned, so
///   the arrival unlock at the far end teaches the way home;
/// * the granted gate exists, is not a dial hub, is on another world, and
///   that world is one this server can enter and has a DHD of its own.
///
/// Point chain 8001 at the SGC (gate 27, no DHD on world 58) or at a world
/// with no map and this fails.
#[tokio::test]
async fn live_db_every_address_a_player_loaded_chain_grants_has_a_way_back() {
    let pool = require_db_or_skip!();

    let grants: Vec<(i32, Option<String>, Option<i32>)> = sqlx::query_as(
        "SELECT a.chain_id, t.event_key::text, a.target_id \
           FROM resources.content_actions a \
           JOIN resources.content_chains c ON c.chain_id = a.chain_id \
           JOIN resources.content_triggers t ON t.chain_id = a.chain_id \
          WHERE a.action_type::text = 'grant_stargate_address' \
            AND t.event_type::text = 'player_loaded' AND c.enabled \
          ORDER BY a.chain_id, a.sort_order",
    )
    .fetch_all(&pool)
    .await
    .expect("player_loaded address grants");
    let (chain, world, gate) = DK01_GRANT;
    assert!(
        grants.contains(&(chain, Some(world.to_string()), Some(gate))),
        "chain 8001 must grant Omega Site on player_loaded Dakara_E1, got {grants:?}"
    );

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(&repo_file("entities/spaces.xml"))
        .expect("spaces.xml parses");
    mgr.create_startup_spaces(&repo_file("entities/cell_spaces.xml"))
        .expect("cell_spaces.xml creates its spaces");
    let gates = load_stargates(&pool).await.expect("load_stargates");
    let dhds = dhd_spawns(&pool).await;
    let has_dhd = |world: &str| dhds.iter().any(|(w, _)| w == world);

    for (chain_id, origin, target) in &grants {
        let origin = origin
            .as_deref()
            .unwrap_or_else(|| panic!("chain {chain_id} grants an address on every world"));
        let target =
            target.unwrap_or_else(|| panic!("chain {chain_id} grants no stargate (NULL target)"));
        assert!(
            mgr.world_is_enterable(origin) && has_dhd(origin),
            "chain {chain_id}: {origin} must be enterable and have a DHD to dial from"
        );
        assert!(
            gates
                .values()
                .any(|g| g.world_name == origin && !g.debug_dial_hub),
            "chain {chain_id}: {origin} has no gate the far end's arrival unlock can teach"
        );

        let gate = gates.get(&target).unwrap_or_else(|| {
            panic!("chain {chain_id} grants stargate {target}, which has no row")
        });
        assert!(
            !gate.debug_dial_hub,
            "chain {chain_id} grants a dial hub, which nobody may hold"
        );
        assert_ne!(
            gate.world_name, origin,
            "chain {chain_id} grants the gate the player is standing at"
        );
        assert!(
            mgr.world_is_enterable(&gate.world_name),
            "chain {chain_id} grants stargate {target} on {}, which this server cannot enter",
            gate.world_name
        );
        assert!(
            has_dhd(&gate.world_name),
            "chain {chain_id} grants stargate {target} on {}, which has no DHD: a one-way trip",
            gate.world_name
        );
    }
}
