//! Ground-clamp guards for `npc_movement_tick` on the real
//! `castle_cellblock.nav` (NA11).

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::NavMesh;

use super::super::npc_movement::npc_movement_tick;
use crate::cell::service::npc_ai::replace_nav_path_on;
use crate::cell::space_manager::SpaceManager;

const FIXTURE: &str = "../../data/spaces/castle_cellblock.nav";
const NPC: u32 = 200;

/// A navmesh-backed Castle_CellBlock space with one walking NPC at `start`,
/// or `None` when the fixture is not checked out.
fn cellblock_with_npc(start: Vector3) -> Option<SpaceManager> {
    let path = std::path::Path::new(FIXTURE);
    if !path.exists() {
        return None;
    }
    let navmesh = NavMesh::load(path).expect("load castle_cellblock.nav");
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    let space_id = mgr
        .create_entity(
            NPC,
            "Castle_CellBlock",
            [start.x, start.y, start.z],
            [0.0; 3],
        )
        .unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(navmesh);
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.is_player = false;
    npc.class_id = 0x04;
    npc.move_speed = 0.6;
    Some(mgr)
}

/// The audit's ramp column (NPC 100574 on colo): the Cellblock guard spawn
/// on the floor at Y 68.6, and the tutorial staging spot at the top of the
/// ramp. Detour returns this as a short flat leg and then ONE 42.8 u leg
/// from Y 68.6 to 73.6, whose chord floats up to ~1.95 u over the flat part.
const RAMP_FOOT: Vector3 = Vector3 {
    x: -289.465,
    y: 68.542,
    z: -154.276,
};
const RAMP_TOP: Vector3 = Vector3 {
    x: -315.6,
    y: 73.6,
    z: -191.4,
};

/// Route the NPC from `RAMP_FOOT` to `RAMP_TOP` the way the fight chase
/// does (the path minus its start point).
fn route_up_the_ramp(mgr: &mut SpaceManager) {
    let path = mgr
        .find_path(NPC, &RAMP_FOOT, &RAMP_TOP)
        .expect("the guard column must route to the ramp top");
    assert!(
        path.windows(2)
            .any(|w| w[1].y - w[0].y > 4.0 && w[0].distance_to(&w[1]) > 30.0),
        "precondition: the route must contain the long floor-then-ramp leg, \
         or this test no longer exercises M1: {path:?}"
    );
    let npc = mgr.get_entity_mut(NPC).unwrap();
    replace_nav_path_on(npc, path.into_iter().skip(1));
}

/// **M1 regression guard.** Every tick of the walk up the ramp leaves the
/// NPC within 0.3 u of the floor under it. On the old chord lerp the NPC is
/// ~1.9 u in the air over the flat part of the long leg, so this fails.
#[test]
fn an_npc_walking_floor_then_ramp_stays_on_the_floor() {
    let Some(mut mgr) = cellblock_with_npc(RAMP_FOOT) else {
        return;
    };
    route_up_the_ramp(&mut mgr);

    let mut ticks = 0;
    let mut worst = 0.0f32;
    while !mgr.get_entity(NPC).unwrap().nav_path.is_empty() {
        npc_movement_tick(&mut mgr);
        ticks += 1;
        assert!(ticks < 500, "the NPC must finish a ~46 u route");
        let p = mgr.get_entity(NPC).unwrap().position;
        let floor = mgr
            .get_navmesh_height(NPC, p.x, p.y, p.z)
            .unwrap_or_else(|| panic!("tick {ticks}: no floor within 4 u of {p:?}"));
        let off = p.y - floor;
        if off.abs() > worst.abs() {
            worst = off;
        }
        assert!(
            off.abs() <= 0.3,
            "tick {ticks}: NPC at {p:?} is {off:+.2} u off the floor at {floor:.2}"
        );
    }
    let end = mgr.get_entity(NPC).unwrap().position;
    assert!(
        end.distance_to(&RAMP_TOP) < 0.5,
        "the NPC must still arrive at the ramp top, got {end:?} (worst {worst:+.2})"
    );
}

/// **M3 regression guard.** Mid-leg on flat floor the broadcast velocity
/// has no vertical component. The old chord velocity carried the leg's
/// average climb (5 u over 42.8 u, ~0.7 u/s at 6 u/s), which the client's
/// filter extrapolated into the air between updates.
#[test]
fn broadcast_vy_is_zero_mid_leg_on_flat_floor() {
    let Some(mut mgr) = cellblock_with_npc(RAMP_FOOT) else {
        return;
    };
    route_up_the_ramp(&mut mgr);

    // Walk onto the long leg (one waypoint left). Its first six steps are on
    // level floor at Y 68.6; a small bump in the detail mesh starts at the
    // seventh, well short of the ramp.
    while mgr.get_entity(NPC).unwrap().nav_path.len() > 1 {
        npc_movement_tick(&mut mgr);
    }
    for step in 1..=6 {
        npc_movement_tick(&mut mgr);
        let npc = mgr.get_entity(NPC).unwrap();
        let [vx, vy, vz] = npc.velocity;
        assert!(
            vy.abs() < 0.01,
            "step {step}: flat-floor broadcast vy = {vy}"
        );
        assert!(
            (npc.position.y - FLAT_FLOOR_Y).abs() < 0.01,
            "step {step} must be on the level floor, got {:?}",
            npc.position
        );
        let horizontal = (vx * vx + vz * vz).sqrt();
        assert!(
            (horizontal - 6.0).abs() < 0.01,
            "step {step}: horizontal speed must be the full 6 u/s, got {horizontal}"
        );
    }
}

/// The guard's floor under the first part of the long leg.
const FLAT_FLOOR_Y: f32 = 68.6;

/// Random on-mesh points, deterministic, so a sweep is repeatable.
fn random_mesh_points(mesh: &NavMesh, n: usize) -> Vec<Vector3> {
    let (bmin, bmax) = (mesh.bmin, mesh.bmax);
    let mut seed: u64 = 0x1234_5678;
    let mut unit = || {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((seed >> 33) as f32) / ((1u64 << 31) as f32)
    };
    let mut pts = Vec::with_capacity(n);
    while pts.len() < n {
        let q = Vector3::new(
            bmin[0] + (bmax[0] - bmin[0]) * unit(),
            bmin[1] + (bmax[1] - bmin[1]) * unit(),
            bmin[2] + (bmax[2] - bmin[2]) * unit(),
        );
        if let Some((_, p)) = mesh.find_nearest_poly(&q) {
            pts.push(p);
        }
    }
    pts
}

/// The clamp across the whole mesh, not just one ramp: 60 random routes
/// (about 34,000 ticks) with no tick more than 0.3 u off the floor. On the
/// old chord lerp 234 ticks are, the worst 4.7 u *under* a floor, and
/// another ~1,300 have no floor within 4 u at all.
///
/// This sweep is also the measurement behind not adopting
/// `DT_STRAIGHTPATH_ALL_CROSSINGS`: with the clamp, crossings leave the Y
/// error at zero either way and the XZ track is identical (the crossing
/// vertices lie on the same string-pulled segments), but every extra corner
/// is an arrival snap that forfeits the rest of that tick's step, so the
/// same routes took 35,617 ticks instead of 34,412 (3.5% slower NPCs).
#[test]
fn random_routes_across_the_cellblock_never_leave_the_floor() {
    let path = std::path::Path::new(FIXTURE);
    if !path.exists() {
        return;
    }
    let mesh = NavMesh::load(path).expect("load castle_cellblock.nav");
    let pts = random_mesh_points(&mesh, 120);

    let (mut routes, mut off_floor) = (0, Vec::new());
    for pair in pts.chunks(2) {
        let (a, b) = (pair[0], pair[1]);
        let mut mgr = cellblock_with_npc(a).unwrap();
        let Some(path) = mgr.find_path(NPC, &a, &b).filter(|p| p.len() >= 2) else {
            continue;
        };
        routes += 1;
        replace_nav_path_on(mgr.get_entity_mut(NPC).unwrap(), path.into_iter().skip(1));
        for _ in 0..3000 {
            if mgr.get_entity(NPC).unwrap().nav_path.is_empty() {
                break;
            }
            npc_movement_tick(&mut mgr);
            let p = mgr.get_entity(NPC).unwrap().position;
            if let Some(floor) = mgr.get_navmesh_height(NPC, p.x, p.y, p.z) {
                if (p.y - floor).abs() > 0.3 {
                    off_floor.push((a, b, p, floor));
                }
            }
        }
    }
    assert!(
        routes >= 40,
        "precondition: most pairs must route, got {routes}"
    );
    assert!(
        off_floor.is_empty(),
        "{} ticks more than 0.3 u off the floor, first: {:?}",
        off_floor.len(),
        off_floor.first()
    );
}
