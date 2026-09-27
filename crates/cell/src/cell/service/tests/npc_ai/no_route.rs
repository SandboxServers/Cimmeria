//! NA41: what every mover does when the pathfinder gives it no usable route
//! (handoff §9, §12, §22; §26 tests 10, 12 and 15).
//!
//! Follow, patrol, wander and investigate used to push the raw destination
//! as one waypoint on any routing failure, and the movement tick walked it in
//! a straight line through walls. On a meshed world they now slide across the
//! mesh (Detour `moveAlongSurface`) or hold. A chaser that stands still for
//! want of a route turns to face its target.
//!
//! The movers run on the real `castle_cellblock.nav`, as `path_robustness`
//! does. Two shapes:
//!
//! - **Component gap.** [`EDGE`] is where the main interior island's partial
//!   corridor toward the ground plane ends. From there a route to
//!   [`OTHER_ISLAND`] is a one-point partial (degenerate), and the slide
//!   gains nothing: the NPC must hold.
//! - **Across a wall.** [`BESIDE_WALL`] is behind the mess hall's west wall,
//!   more than 3 u from any polygon, so a route to it fails its end lookup.
//!   The slide from [`MESSHALL`] stops at the wall: the NPC walks there.
//!
//! Every movement tick asserts the NPC is on the mesh **and** still on the
//! component it started on (a route back to its start is `Ok`). The old
//! straight line fails that: at the edge it walks off the island, and in the
//! mess hall it walks into the wall.

use std::f32::consts::FRAC_PI_2;
use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::{NavMesh, PathStatus};
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

use super::*;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};

const NPC: u32 = 200;
const LEADER: u32 = 101;
const WORLD: &str = "Castle_CellBlock";
/// `MessHall_Guard1`'s spawn, on the main interior island.
const MESSHALL: [f32; 3] = [-96.25, 34.6, -91.59];
/// The end of the partial corridor from the main island toward the ground
/// plane: the island's edge.
const EDGE: [f32; 3] = [-148.3, 24.8, -144.1];
/// The south-west corner of the ground plane: another island.
const OTHER_ISLAND: [f32; 3] = [-399.1, 0.2, -399.1];
/// Behind the mess hall's west wall, off the mesh.
const BESIDE_WALL: [f32; 3] = [-124.25, 34.6, -105.59];

fn v(p: [f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

fn cellblock_nav() -> NavMesh {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/spaces/castle_cellblock.nav");
    NavMesh::load(&p).unwrap_or_else(|e| panic!("load {}: {e}", p.display()))
}

/// Cellblock with its navmesh, one SGWMob NPC at `npc` in `state`, and a
/// connected player at `leader`.
fn fixture(npc: [f32; 3], state: AiState, leader: [f32; 3]) -> SpaceManager {
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
    mgr.create_entity(NPC, WORLD, npc, [0.0; 3]).unwrap();
    let e = mgr.get_entity_mut(NPC).unwrap();
    e.is_player = false;
    e.class_id = 0x04;
    e.spawn_position = Some(v(npc));
    e.leash.distance_override = Some(10_000.0);
    e.aoi_radius = 2_000.0;
    crate::cell::service::npc_ai::force_ai_state(e, state);
    let h = e.stats.get_mut(HEALTH).unwrap();
    h.update(0, 100, 100);
    h.clear_dirty();
    mgr.create_entity(LEADER, WORLD, leader, [0.0; 3]).unwrap();
    let p = mgr.get_entity_mut(LEADER).unwrap();
    p.is_player = true;
    p.player_id = Some(LEADER as i32);
    p.stats.get_mut(HEALTH).unwrap().update(0, 100, 100);
    mgr.connect_entity(LEADER);
    let _ = mgr.compute_aoi_changes();
    mgr
}

async fn ai_tick(mgr: &mut SpaceManager) {
    let (tx, _rx) = mpsc::channel(4096);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &crate::cell::content::EngineEvents(&cimmeria_content_engine::chain::ChainEngine::new()),
    )
    .await;
}

fn path_fails(logs: &LogCaptureGuard, state: &str) -> Vec<Captured> {
    logs.all()
        .into_iter()
        .filter(|c| {
            c.target == "npc_ai.path_fail"
                && c.has_field("event", "path_fail")
                && c.has_field("state", state)
        })
        .collect()
}

fn horizontal(a: &Vector3, b: &Vector3) -> f32 {
    ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// `rounds` AI ticks, each followed by 20 movement ticks (the production 2 s
/// cadence). After every movement tick the NPC must stand on the mesh, on the
/// component it started on. `before_ai` runs before each AI tick.
async fn drive_on_mesh(
    mgr: &mut SpaceManager,
    rounds: usize,
    mut before_ai: impl FnMut(&mut SpaceManager),
) {
    let nav = cellblock_nav();
    let start = mgr.get_entity(NPC).unwrap().position;
    for round in 0..rounds {
        before_ai(mgr);
        ai_tick(mgr).await;
        for step in 0..20 {
            crate::cell::service::ticks::npc_movement_tick(mgr);
            let pos = mgr.get_entity(NPC).unwrap().position;
            assert!(
                mgr.is_position_valid(NPC, &pos),
                "round {round} step {step}: off the mesh at {pos:?}"
            );
            let back = nav.find_path(&pos, &start);
            assert_eq!(
                back.status,
                PathStatus::Ok,
                "round {round} step {step}: {pos:?} left the start's component"
            );
        }
    }
}

/// §26 test 15 on a component gap. A follower at the island edge whose leader
/// is on another island holds: on the mesh, zero velocity, still in Follow
/// with the same target, and the row says `held_no_route`.
///
/// Revert-proof: the old raw push walks the follower off the island edge.
#[tokio::test]
async fn a_follower_across_a_component_gap_holds_and_keeps_its_target() {
    let mut mgr = fixture(EDGE, AiState::Follow, OTHER_ISLAND);
    mgr.get_entity_mut(NPC).unwrap().follow_target_id = Some(LEADER);
    let logs = LogCapture::install();

    drive_on_mesh(&mut mgr, 5, |_| {}).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Follow, "still following");
    assert_eq!(npc.follow_target_id, Some(LEADER), "target kept");
    assert!(npc.nav_path.is_empty());
    assert_eq!(npc.velocity, [0.0; 3], "holding, not running in place");
    let fails = path_fails(&logs, "follow");
    assert!(!fails.is_empty(), "the hold is logged");
    assert!(
        fails
            .iter()
            .all(|f| f.has_field("fallback", "held_no_route")),
        "{fails:#?}"
    );
}

/// A patrol leg to a waypoint behind a wall slides to the wall and stops
/// there, on the mesh, instead of walking into it.
///
/// Revert-proof: the old raw push walks the NPC to the off-mesh waypoint.
#[tokio::test]
async fn a_patrol_leg_behind_a_wall_slides_to_the_wall() {
    let mut mgr = fixture(MESSHALL, AiState::Patrol, MESSHALL);
    mgr.get_entity_mut(NPC).unwrap().patrol_path = vec![v(BESIDE_WALL)];
    let logs = LogCapture::install();

    drive_on_mesh(&mut mgr, 4, |_| {}).await;

    let end = mgr.get_entity(NPC).unwrap().position;
    assert!(
        horizontal(&end, &v(MESSHALL)) >= 0.5,
        "the slide made progress toward the waypoint: {end:?}"
    );
    let fails = path_fails(&logs, "patrol");
    assert!(
        fails
            .first()
            .is_some_and(|f| f.has_field("fallback", "surface_clamped")),
        "{fails:#?}"
    );
    assert!(fails
        .iter()
        .all(|f| !f.has_field("fallback", "direct_waypoint")));
}

/// A wanderer at the island edge whose sampled point is on another island
/// holds and dwells where it is.
///
/// Revert-proof: the old raw push walks the NPC off the island edge.
#[tokio::test]
async fn a_wander_hop_across_a_component_gap_holds() {
    let mut mgr = fixture(EDGE, AiState::Wander, MESSHALL);
    {
        let npc = mgr.get_entity_mut(NPC).unwrap();
        npc.spawn_position = Some(v(OTHER_ISLAND));
        npc.wander_radius = 0.01;
    }
    let logs = LogCapture::install();

    // Force a fresh hop on every AI tick: the dwell has always elapsed.
    drive_on_mesh(&mut mgr, 4, |mgr| {
        mgr.get_entity_mut(NPC).unwrap().wander_next_at =
            Some(Instant::now() - Duration::from_secs(1));
    })
    .await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Wander);
    assert_eq!(npc.velocity, [0.0; 3]);
    let fails = path_fails(&logs, "wander");
    assert!(!fails.is_empty(), "the hold is logged");
    assert!(
        fails
            .iter()
            .all(|f| f.has_field("fallback", "held_no_route")),
        "{fails:#?}"
    );
}

/// An investigate POI behind a wall: the NPC slides to the wall, on the mesh.
///
/// Revert-proof: the old raw push walks the NPC to the off-mesh POI.
#[tokio::test]
async fn an_investigate_poi_behind_a_wall_slides_to_the_wall() {
    let mut mgr = fixture(MESSHALL, AiState::Investigating, MESSHALL);
    mgr.get_entity_mut(NPC).unwrap().poi = Some(v(BESIDE_WALL));
    let logs = LogCapture::install();

    drive_on_mesh(&mut mgr, 4, |_| {}).await;

    let fails = path_fails(&logs, "investigate");
    assert!(
        fails
            .first()
            .is_some_and(|f| f.has_field("fallback", "surface_clamped")),
        "{fails:#?}"
    );
}

/// A patrol waypoint on another island is skipped after the hold, so the
/// patrol moves on instead of retrying that waypoint forever.
///
/// Revert-proof: without the skip the patrol stays on waypoint 0.
#[tokio::test]
async fn a_patrol_waypoint_across_a_component_gap_is_skipped() {
    let mut mgr = fixture(EDGE, AiState::Patrol, MESSHALL);
    mgr.get_entity_mut(NPC).unwrap().patrol_path = vec![v(OTHER_ISLAND), v(MESSHALL)];

    ai_tick(&mut mgr).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state(), AiState::Patrol);
    assert!(npc.nav_path.is_empty(), "control: held, nothing to walk");
    assert_eq!(npc.patrol_next_index, 1, "moved on to the next waypoint");
}

/// An investigate POI on another island: the NPC investigates from where it
/// holds, dwells and goes back to Idle, instead of retrying the POI forever.
///
/// Revert-proof: without the settle the POI stays on the other island and the
/// dwell never starts.
#[tokio::test]
async fn an_investigate_poi_across_a_component_gap_ends_where_the_npc_holds() {
    let mut mgr = fixture(EDGE, AiState::Investigating, MESSHALL);
    mgr.get_entity_mut(NPC).unwrap().poi = Some(v(OTHER_ISLAND));

    ai_tick(&mut mgr).await;
    assert_eq!(mgr.get_entity(NPC).unwrap().poi, Some(v(EDGE)));

    ai_tick(&mut mgr).await;
    let npc = mgr.get_entity_mut(NPC).unwrap();
    assert!(npc.investigate_until.is_some(), "dwelling where it holds");
    npc.investigate_until = Some(Instant::now() - Duration::from_secs(1));

    ai_tick(&mut mgr).await;
    assert_eq!(mgr.get_entity(NPC).unwrap().ai_state(), AiState::Idle);
}

/// §26 test 12 for a mobile chaser with no route (no navmesh, so the chase
/// gets `None` every tick): it turns to face its target, and keeps turning as
/// the target moves between ticks.
///
/// Revert-proof: without the `face_target` call in the no-route arm the NPC
/// keeps its stale +PI/2 heading.
#[tokio::test]
async fn a_chaser_with_no_route_turns_to_face_a_moving_target() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    seed_target_with_threat(&mut mgr, NPC, LEADER, [-40.0, 0.0, 0.0]);
    mgr.get_entity_mut(NPC).unwrap().direction = Vector3::new(0.0, FRAC_PI_2, 0.0);

    ai_tick(&mut mgr).await;
    let npc = mgr.get_entity(NPC).unwrap();
    assert!(!npc.is_stationary, "control: a mobile NPC");
    assert!(npc.nav_path.is_empty(), "control: no route was installed");
    let yaw = npc.direction.y;
    assert!((yaw + FRAC_PI_2).abs() < 1e-4, "faces west; got {yaw}");

    mgr.update_position_preserving_facing(LEADER, [0.0, 0.0, 40.0], [0.0; 3]);
    ai_tick(&mut mgr).await;
    let yaw = mgr.get_entity(NPC).unwrap().direction.y;
    assert!(yaw.abs() < 1e-4, "turned to face north; got {yaw}");
}

/// The degenerate-repath hold faces the target too. The target stands 1 u
/// west and 2.5 u up (mid-jump), out of a 2 u ability's reach, so the chase
/// goal is the NPC's own spot (the `path_robustness` S10 shape).
///
/// Revert-proof: without the `face_target` call the NPC keeps +PI/2.
#[tokio::test]
async fn a_degenerate_repath_hold_faces_the_target() {
    let at = cellblock_nav().get_nearest_point(&v(MESSHALL));
    let target = [at.x - 1.0, at.y + 2.5, at.z];
    let mut mgr = fixture([at.x, at.y, at.z], AiState::Fighting, target);
    seed_default_ability(&mut mgr, 0, 2);
    {
        let npc = mgr.get_entity_mut(NPC).unwrap();
        npc.threat_list.insert(LEADER, 10.0);
        npc.direction = Vector3::new(0.0, FRAC_PI_2, 0.0);
    }
    let logs = LogCapture::install();

    ai_tick(&mut mgr).await;

    assert!(
        path_fails(&logs, "fight")
            .iter()
            .any(|f| f.has_field("reason", "degenerate_path")),
        "control: the degenerate arm ran: {:#?}",
        logs.all()
    );
    let yaw = mgr.get_entity(NPC).unwrap().direction.y;
    assert!((yaw + FRAC_PI_2).abs() < 1e-3, "faces west; got {yaw}");
}
