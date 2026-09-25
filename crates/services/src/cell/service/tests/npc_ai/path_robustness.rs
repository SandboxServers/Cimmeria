//! NA15: path robustness over the real `castle_cellblock.nav` (audit S8-S10,
//! S14). Each test reproduces the bug shape and names the revert that fails
//! it.
//!
//! The rebuilt Cellblock mesh has 17 components. The mess hall is on the main
//! interior island; the `y = 0.2` ground plane is another one, and a route
//! between them is a 340 u partial corridor ending at `(-148.3, 24.8, -144.1)`.

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::NavMesh;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use crate::cell::space_manager::SpaceManager;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};

const NPC: u32 = 200;
const PLAYER: u32 = 101;
const WORLD: &str = "Castle_CellBlock";
/// `MessHall_Guard1`'s spawn, on the main interior island.
const MESSHALL: [f32; 3] = [-96.25, 34.591, -91.59];
/// `Hallway01_Guard`'s spawn: the same island, about 37 u away.
const HALLWAY01: [f32; 3] = [-128.853, 39.552, -73.534];
/// The south-west corner of the ground plane: another island.
const OTHER_ISLAND: [f32; 3] = [-400.0, 0.2, -400.0];
/// Beside the mess hall's west wall, at floor height: more than 3 u from any
/// polygon (the pathfinder's destination box), under 8 u from the hallway.
const BESIDE_WALL: [f32; 3] = [-124.25, 34.6, -105.59];

fn v(p: [f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

fn cellblock_nav() -> NavMesh {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/spaces/castle_cellblock.nav");
    NavMesh::load(&p).unwrap_or_else(|e| panic!("load {}: {e}", p.display()))
}

/// A Fighting NPC at `npc`, spawn `spawn`, with a connected full-health
/// player at `target` on its threat list. Positions are used as given, so a
/// caller that wants a point on the floor passes it through [`on_mesh`].
fn fixture(npc: Vector3, spawn: Option<Vector3>, target: Vector3) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#,
    )
    .unwrap();
    let space_id = mgr.space_id_for_world(WORLD).unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(cellblock_nav());
    mgr.create_entity(NPC, WORLD, [npc.x, npc.y, npc.z], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(NPC).unwrap();
    e.is_player = false;
    e.class_id = 0x04;
    e.spawn_position = spawn;
    // Far enough that neither the leash radius nor target loss ends the
    // chase before the behaviour under test.
    e.leash.distance_override = Some(10_000.0);
    e.aoi_radius = 2_000.0;
    crate::cell::service::npc_ai::force_ai_state(e, AiState::Fighting);
    let h = e.stats.get_mut(HEALTH).unwrap();
    h.update(0, 100, 100);
    h.clear_dirty();
    mgr.create_entity(PLAYER, WORLD, [target.x, target.y, target.z], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER as i32);
    p.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
    mgr.connect_entity(PLAYER);
    let _ = mgr.compute_aoi_changes();
    mgr.get_entity_mut(NPC)
        .unwrap()
        .threat_list
        .insert(PLAYER, 10.0);
    mgr
}

fn on_mesh(p: [f32; 3]) -> Vector3 {
    cellblock_nav().get_nearest_point(&v(p))
}

async fn ai_tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(4096);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

fn movement_tick(mgr: &mut SpaceManager) {
    crate::cell::service::ticks::npc_movement_tick(mgr);
}

fn rows(logs: &LogCaptureGuard, target: &str, event: &str) -> Vec<Captured> {
    logs.all()
        .into_iter()
        .filter(|c| c.target == target && c.has_field("event", event))
        .collect()
}

fn horizontal(a: &Vector3, b: &Vector3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// Walk the installed route to its end, with an AI tick every 20 movement
/// ticks (the production 2 s cadence). Panics after `max` movement ticks.
async fn walk_route_out(mgr: &mut SpaceManager, max: usize) {
    for tick in 1..=max {
        if mgr.get_entity(NPC).unwrap().nav_path.is_empty() {
            return;
        }
        movement_tick(mgr);
        if tick % 20 == 0 {
            ai_tick(mgr).await;
        }
    }
    panic!("the route did not end within {max} movement ticks");
}

/// **S8.** A target on another mesh island: the NPC walks the partial route
/// to the island edge, holds there with zero velocity and no new route
/// requests, and after the grace period walks home.
///
/// Revert-proof: without the hold, every AI tick at the edge requests a new
/// (partial, near-zero) route, and the request count grows with the ticks.
#[tokio::test]
async fn a_chase_to_another_island_walks_to_the_edge_holds_then_gives_up() {
    let mut mgr = fixture(on_mesh(MESSHALL), None, on_mesh(OTHER_ISLAND));
    let logs = LogCapture::install();

    ai_tick(&mut mgr).await;
    assert!(!mgr.get_entity(NPC).unwrap().nav_path.is_empty());
    walk_route_out(&mut mgr, 1_000).await;

    let edge = mgr.get_entity(NPC).unwrap().position;
    for _ in 0..6 {
        ai_tick(&mut mgr).await;
        for _ in 0..20 {
            movement_tick(&mut mgr);
        }
        let npc = mgr.get_entity(NPC).unwrap();
        assert_eq!(npc.ai_state(), AiState::Fighting);
        assert_eq!(npc.position, edge, "the NPC holds at the island edge");
        assert_eq!(npc.velocity, [0.0; 3], "holding, not running in place");
        assert!(npc.nav_path.is_empty());
    }
    assert!(mgr
        .get_entity(NPC)
        .unwrap()
        .leash
        .unreachable_since
        .is_some());
    let requests = rows(&logs, "npc_ai.path", "request").len();
    assert_eq!(
        requests, 1,
        "one route for the whole chase, none while holding: {requests}"
    );

    mgr.get_entity_mut(NPC).unwrap().leash.unreachable_since =
        Some(Instant::now() - Duration::from_secs(9));
    ai_tick(&mut mgr).await;
    assert!(
        logs.all()
            .into_iter()
            .any(|c| c.target == "npc_ai.transition"
                && c.has_field("to", "leashing")
                && c.has_field("reason", "unreachable")),
        "the NPC gives up on the unreachable target"
    );
    assert!(mgr.get_entity(NPC).unwrap().threat_list.is_empty());
}

/// **S8 on the walk home.** Spawn is on another island: the route home is
/// partial. The NPC walks it to its end and then snaps home, instead of
/// replanning from the island edge until the 20 s walk timeout.
///
/// Revert-proof: without the partial flag the leash tick replans at the edge
/// (a second `state=leash` request) and ends `snap_no_path`.
#[tokio::test]
async fn a_partial_route_home_walks_to_its_end_then_snaps() {
    let home = on_mesh(OTHER_ISLAND);
    let mut mgr = fixture(on_mesh(MESSHALL), Some(home), on_mesh(HALLWAY01));
    {
        let npc = mgr.get_entity_mut(NPC).unwrap();
        npc.threat_list.clear();
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Leashing);
    }
    let logs = LogCapture::install();

    ai_tick(&mut mgr).await;
    assert!(mgr.get_entity(NPC).unwrap().leash.home_route_partial);
    walk_route_out(&mut mgr, 1_000).await;
    ai_tick(&mut mgr).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Idle);
    assert_eq!(npc.position, home, "snapped home from the route's end");
    assert!(!npc.leash.home_route_partial);
    let leash_requests = rows(&logs, "npc_ai.path", "request")
        .into_iter()
        .filter(|r| r.has_field("state", "leash"))
        .count();
    assert_eq!(leash_requests, 1, "planned once, never replanned");
    assert!(
        logs.all()
            .into_iter()
            .any(|c| c.has_field("arrival", "snap_partial_route")),
        "{:#?}",
        logs.all()
    );
}

/// **S9.** An NPC hovering 2 u over the floor fails the pathfinder's ±0.5
/// start box. It is snapped onto the floor and routed in the same tick.
///
/// Revert-proof: without the snap the NPC keeps its hover and gets no route.
#[tokio::test]
async fn an_off_mesh_start_is_snapped_onto_the_mesh_and_routed() {
    let floor = on_mesh(MESSHALL);
    let hover = Vector3::new(floor.x, floor.y + 2.0, floor.z);
    let mut mgr = fixture(hover, Some(floor), on_mesh(HALLWAY01));
    let logs = LogCapture::install();

    ai_tick(&mut mgr).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert!((npc.position.y - floor.y).abs() < 0.1, "{:?}", npc.position);
    assert!(!npc.nav_path.is_empty(), "routed from the floor");
    assert!(mgr.diagnose_point(NPC, &npc.position).unwrap().valid);
    assert_eq!(rows(&logs, "npc_ai.path", "off_mesh_snap").len(), 1);
    let fails = rows(&logs, "npc_ai.path_fail", "path_fail");
    assert!(
        fails[0].has_field("reason", "no_start_poly")
            && fails[0].has_field("fallback", "snapped_to_mesh"),
        "{fails:#?}"
    );
}

/// **S9, no floor near.** Beside a wall, more than 2 u from any polygon: no
/// snap. The NPC gives up, and the leash tick snaps it to spawn because it
/// cannot route from where it stands.
///
/// Revert-proof: the old handler logged `no_path` and left the NPC there.
#[tokio::test]
async fn an_off_mesh_start_with_no_floor_near_goes_home() {
    let home = on_mesh(MESSHALL);
    let mut mgr = fixture(v(BESIDE_WALL), Some(home), on_mesh(HALLWAY01));
    let logs = LogCapture::install();

    ai_tick(&mut mgr).await;
    assert_eq!(
        mgr.get_entity(NPC).unwrap().ai_state(),
        AiState::Leashing,
        "{:#?}",
        logs.all()
    );
    assert!(logs
        .all()
        .into_iter()
        .any(|c| c.target == "npc_ai.transition" && c.has_field("reason", "unreachable")));

    ai_tick(&mut mgr).await;
    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Idle);
    assert_eq!(npc.position, home);
}

/// **S10.** A repath that comes back as a single point clears the stale
/// route instead of leaving the NPC walking it.
///
/// The target stands 1 u across and 2.5 u up (mid-jump), out of a 2 u
/// ability's reach: the goal, pulled 1 u short, is the NPC's own spot.
/// Revert-proof: the old handler kept the stale route (`path_unchanged`).
#[tokio::test]
async fn a_degenerate_repath_clears_the_stale_route() {
    let at = on_mesh(MESSHALL);
    let target = Vector3::new(at.x - 1.0, at.y + 2.5, at.z);
    let mut mgr = fixture(at, None, target);
    super::seed_default_ability(&mut mgr, 0, 2);
    crate::cell::service::npc_ai::replace_nav_path_on(
        mgr.get_entity_mut(NPC).unwrap(),
        [on_mesh(HALLWAY01)],
    );
    let logs = LogCapture::install();

    ai_tick(&mut mgr).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert!(npc.nav_path.is_empty(), "the stale route is cleared");
    assert_eq!(npc.velocity, [0.0; 3]);
    let fails = rows(&logs, "npc_ai.path_fail", "path_fail");
    assert!(
        fails
            .iter()
            .any(|f| f.has_field("reason", "degenerate_path")
                && f.has_field("fallback", "path_cleared")),
        "{fails:#?}"
    );
}

/// **S10.** A chaser stops short of its target, never inside it. One AI tick
/// plans the route; the movement tick walks all of it.
///
/// Revert-proof: routing to the raw target ends the walk at the target's own
/// point (0 u).
#[tokio::test]
async fn a_chaser_never_ends_inside_its_target() {
    let target = on_mesh(MESSHALL);
    let mut mgr = fixture(on_mesh(HALLWAY01), None, target);
    super::seed_default_ability(&mut mgr, 0, 2);

    ai_tick(&mut mgr).await;
    assert!(!mgr.get_entity(NPC).unwrap().nav_path.is_empty());
    let mut closest = f32::INFINITY;
    for _ in 0..200 {
        movement_tick(&mut mgr);
        let pos = mgr.get_entity(NPC).unwrap().position;
        closest = closest.min(horizontal(&pos, &target));
        if mgr.get_entity(NPC).unwrap().nav_path.is_empty() {
            break;
        }
    }
    let end = mgr.get_entity(NPC).unwrap().position;
    assert!(closest >= 0.9, "came within {closest} u of the target");
    assert!(
        end.distance_to(&target) <= 2.0,
        "ends inside the 2 u reach: {}",
        end.distance_to(&target)
    );
}

/// A player walking down a ramp toward the chaser, 3 u closer and 2 u lower,
/// gets a new route. Revert-proof: the old 3D test against the old endpoint
/// (3.6 u < 5) kept the route planned for the upper level.
#[tokio::test]
async fn a_target_dropping_a_level_triggers_a_repath() {
    let target = on_mesh(MESSHALL);
    let mut mgr = fixture(on_mesh(HALLWAY01), None, target);
    ai_tick(&mut mgr).await;
    let first = mgr.get_entity(NPC).unwrap().leash.chase_route.unwrap();

    let lower = [target.x - 3.0, target.y - 2.0, target.z];
    mgr.update_position_preserving_facing(PLAYER, lower, [0.0; 3]);
    ai_tick(&mut mgr).await;

    let second = mgr.get_entity(NPC).unwrap().leash.chase_route.unwrap();
    assert!(
        (second.goal.y - lower[1]).abs() < 1e-3,
        "replanned toward the lower target: {first:?} -> {second:?}"
    );
}

/// **S14.** A target standing off the mesh (a GM on unmeshed geometry): the
/// chaser routes to the nearest on-mesh point instead of failing its
/// end-polygon lookup every tick, and the rows say so.
///
/// Revert-proof: the old handler logged `no_path` and installed nothing.
#[tokio::test]
async fn an_off_mesh_target_routes_to_the_nearest_on_mesh_point() {
    let mut mgr = fixture(on_mesh(MESSHALL), None, v(BESIDE_WALL));
    let logs = LogCapture::install();

    ai_tick(&mut mgr).await;

    let statuses: Vec<String> = rows(&logs, "npc_ai.path", "request")
        .iter()
        .map(|r| r.fields["status"].clone())
        .collect();
    assert_eq!(statuses, ["no_end_poly", "ok"], "{:#?}", logs.all());
    let npc = mgr.get_entity(NPC).unwrap();
    let end = *npc.nav_path.back().expect("a route to the nearest point");
    assert!(horizontal(&end, &v(BESIDE_WALL)) <= 8.0 + 1.0, "{end:?}");
    assert!(!npc.leash.chase_route.unwrap().reaches_goal);
    let fails = rows(&logs, "npc_ai.path_fail", "path_fail");
    assert!(
        fails[0].has_field("reason", "no_end_poly")
            && fails[0].has_field("fallback", "nearest_on_mesh"),
        "{fails:#?}"
    );
}
