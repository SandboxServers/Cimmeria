//! GC1 (Escape escort) — chains 1171-1175 in `castle_cellblock_chains.sql`.
//! See `docs/analysis/castle-cellblock-rebuild/work-packets.md#gc1` for the
//! full evidence and scope split across GC1a (dialogs), GC1b-1 (reposition),
//! and GC1b-2 (follow).
//!
//! Chains 1173/1174/1175 touch NPC position and AI state directly, so
//! (per the "resolving is not enough for an executor arm" rule in
//! TESTING.md's chain-replay section) these push the resolved actions
//! through [`execute_actions`] and assert on the resulting `CellEntity`
//! state, not just on the resolved `Action` list. Chains 1171/1172 are
//! plain `display_dialog` chains with no new executor arm involved, so
//! they stay resolve-only, matching `mission_680.rs`'s style.
//!
//! GC1b-1/GC1b-2's tests load the real `data/spaces/castle_cellblock.nav`
//! fixture (self-skipping when absent, the repo's standard pattern — see
//! `crates/services/src/cell/space_manager/tests/movement_validation/navmesh.rs`)
//! because "Marsh's post-teleport_in position lands in the topside navmesh
//! component" and "Marsh's nav_path stays non-degenerate while following"
//! are both navmesh-shaped claims a fake/no-navmesh fixture cannot prove —
//! the `find_path` fallback under a missing navmesh always returns exactly
//! one straight-line waypoint (see `npc_ai::follow`'s own
//! `out_of_band_follow_with_no_navmesh_falls_back_to_straight_line_waypoint`
//! test), which would make a non-degenerate assertion pass for the wrong
//! reason.

use std::path::Path;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::navigation::NavMesh;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::executor::execute_actions;
use crate::cell::service::npc_ai_follow_for_test;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

const PLAYER_EID: u32 = 7101;
const MARSH_EID: u32 = 100_101;
const MARSH_TAG: &str = "Preparation_ColMarsh";

/// Marsh's actual `spawnlist.sql` spawn (`Preparation_ColMarsh`, world 12).
const PREP_COLMARSH_SPAWN: [f32; 3] = [-191.0, 54.719_997, -138.588];
/// GC1b-1's authored destination — 2 units off the exact `CellblockRing3`
/// ring-transport landing pad (`ring_transport_regions.sql` region_id=3),
/// matching chain 1173's `move_waypoint` params exactly.
const GC1B1_DESTINATION: [f32; 3] = [-91.689, 45.188, -161.533];
/// `MessHall_Guard1`'s spawn (`spawnlist.sql`) — a confirmed-connected
/// topside anchor, one hop into the escort route from the Ring 3 pad.
const MESSHALL_G1: [f32; 3] = [-96.25, 34.591, -91.59];
/// `Hallway01_Guard`'s spawn (`spawnlist.sql`) — a second confirmed-connected
/// topside anchor, further along the escort route.
const HALLWAY01: [f32; 3] = [-128.853, 39.552, -73.534];
/// Every Hallway guard spawn on the escort route (`spawnlist.sql`, world
/// 12), in route order. Mission 686 completes at Hallway05, so the escort
/// has to be able to walk all the way there.
const HALLWAY_GUARDS: [(&str, [f32; 3]); 6] = [
    ("Hallway01_Guard", HALLWAY01),
    ("Hallway02_Guard", [-113.485, 39.552, -63.042]),
    ("Hallway03_Guard", [-98.58, 39.55, -77.09]),
    ("Hallway04_Guard", [-61.349, 34.591, -69.032]),
    ("Hallway05_Guard1", [-100.16, 24.67, -43.895]),
    ("Hallway05_Guard2", [-101.5, 24.67, -51.3]),
];

/// Whether a `find_path` result's last waypoint actually reaches `goal`. A
/// goal on another mesh component still returns `Some` with a truncated
/// best-effort path, so "returns Some" or "more than one waypoint" is not a
/// connectivity proof; the end has to land on the goal.
///
/// "On the goal" is within 1 u horizontally: `Hallway02_Guard` is authored
/// against a wall 0.59 u off the mesh edge, and the route ends at the
/// nearest walkable point beside it. A route truncated at the edge of
/// another component ends metres away, not within a unit.
fn reaches(path: &[cimmeria_common::Vector3], goal: [f32; 3]) -> bool {
    path.last().is_some_and(|last| {
        let horizontal = ((last.x - goal[0]).powi(2) + (last.z - goal[2]).powi(2)).sqrt();
        horizontal < 1.0 && (last.y - goal[1]).abs() < 1.0
    })
}

/// Whether Marsh's follow route ends by his leader. The Follow handler aims
/// one `follow_min_distance` (2 u) short of the leader, so the end must be
/// within that plus a margin, horizontally, and on the leader's floor. A
/// route truncated at an island edge ends far from the leader.
fn follow_leg_reaches(
    nav_path: &std::collections::VecDeque<cimmeria_common::Vector3>,
    leader: [f32; 3],
) -> bool {
    nav_path.back().is_some_and(|end| {
        let horizontal = ((end.x - leader[0]).powi(2) + (end.z - leader[2]).powi(2)).sqrt();
        horizontal <= 3.0 && (end.y - leader[1]).abs() < 1.5
    })
}

fn nav_fixture_path() -> &'static Path {
    Path::new("../../data/spaces/castle_cellblock.nav")
}

/// Wide-bounds `Castle_CellBlock` space, real navmesh injected. Bounds
/// mirror the fixture's own `bmin`/`bmax` (roughly ±400 on X/Z) so every
/// coordinate used in this file's tests sits inside the spatial grid.
fn make_castle_cellblock_mgr_with_navmesh() -> Option<SpaceManager> {
    let nav_path = nav_fixture_path();
    if !nav_path.exists() {
        return None;
    }
    let navmesh = NavMesh::load(nav_path).expect("load castle_cellblock.nav");

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-450" MaxX="450" MinY="-450" MaxY="450" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();

    let space_id = mgr
        .create_entity(PLAYER_EID, "Castle_CellBlock", GC1B1_DESTINATION, [0.0; 3])
        .expect("Castle_CellBlock startup space must accept the player entity");
    if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
        p.is_player = true;
        p.player_id = Some(42);
    }
    mgr.connect_entity(PLAYER_EID);

    mgr.spawn_npc(MARSH_EID, "Castle_CellBlock", PREP_COLMARSH_SPAWN, [0.0; 3])
        .expect("Castle_CellBlock space must accept the Marsh NPC entity");
    mgr.get_entity_mut(MARSH_EID)
        .expect("Marsh entity must exist immediately after spawn_npc")
        .tag = Some(MARSH_TAG.to_string());

    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(navmesh);
    Some(mgr)
}

fn assert_close(actual: [f32; 3], expected: [f32; 3], what: &str) {
    for axis in 0..3 {
        assert!(
            (actual[axis] - expected[axis]).abs() < 0.001,
            "{what}: axis {axis} must be {} but was {} (full vector {actual:?} vs {expected:?})",
            expected[axis],
            actual[axis],
        );
    }
}

// ── GC1a: dialog chains (resolve-only, no new executor arm) ────────────

/// Chain 1171 positive: interacting with Marsh while mission-680 step 2344
/// is active (Preparation room, before ring travel) resolves exactly one
/// `DisplayDialog(2309)`.
#[tokio::test]
async fn chain_1171_interact_marsh_while_step_2344_active_shows_dialog_2309() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1171)
        .await
        .expect("DB query for chain 1171 must succeed")
        .expect("chain 1171 must exist in seeded content_chains and assemble successfully");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(MARSH_TAG));
    ctx.set_param(
        "mission_680_step_2344_status".to_string(),
        serde_json::json!("active"),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    let shows = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1171 && matches!(action, Action::DisplayDialog { dialog_id: 2309 })
        })
        .count();
    assert_eq!(
        shows, 1,
        "chain 1171 must resolve exactly one DisplayDialog(2309) while step \
         2344 is active; got {shows}. Resolved: {:?}",
        resolved.actions,
    );
}

/// Chain 1171 negative: once the player has passed step 2344 (into 2345,
/// topside), interacting with Marsh must not re-show the pre-departure
/// dialog.
#[tokio::test]
async fn chain_1171_does_not_fire_once_step_2344_advanced_past() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1171)
        .await
        .expect("DB query for chain 1171 must succeed")
        .expect("chain 1171 must exist in seeded content_chains and assemble successfully");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(MARSH_TAG));
    ctx.set_param(
        "mission_680_step_2344_status".to_string(),
        serde_json::json!("completed"),
    );
    ctx.set_param(
        "mission_680_step_2345_status".to_string(),
        serde_json::json!("active"),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1171_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1171)
        .count();
    assert_eq!(
        chain_1171_actions, 0,
        "chain 1171 must NOT fire once step 2344 has advanced past; got \
         {chain_1171_actions} actions",
    );
}

/// Chain 1172 positive: mission 686 completing (the Straegis scene)
/// resolves exactly one `DisplayDialog(5859)`, the v3-surfaced post-death
/// "find a way out without Marsh" beat, at `delay_ms = 10600` -- after
/// C08b's own chain 1161 plays the StraegisAttack Matinee (delay_ms 0)
/// and shows dialog 2516 (delay_ms 10100) on the same trigger. Pinning
/// the exact delay (found in review, 2026-09-18) guards against 5859
/// popping up while the Matinee is still playing.
#[tokio::test]
async fn chain_1172_mission_686_complete_shows_post_death_dialog_5859() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1172)
        .await
        .expect("DB query for chain 1172 must succeed")
        .expect("chain 1172 must exist in seeded content_chains and assemble successfully");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(686));
    let event = TriggerEvent {
        trigger_type: TriggerType::MissionCompleted,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    let shows: Vec<i32> = resolved
        .actions
        .iter()
        .zip(resolved.action_delays.iter())
        .filter_map(|((id, action), delay)| {
            if *id == 1172 && matches!(action, Action::DisplayDialog { dialog_id: 5859 }) {
                Some(*delay)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        shows,
        vec![10600],
        "chain 1172 must resolve exactly one DisplayDialog(5859) at \
         delay_ms=10600 on mission 686 completion (after C08b's Matinee + \
         dialog 2516); got {shows:?}. Resolved: {:?}",
        resolved.actions,
    );
}

/// Chain 1172 negative: a different mission completing must not show the
/// post-death Marsh beat.
#[tokio::test]
async fn chain_1172_does_not_fire_on_a_different_mission_completion() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1172)
        .await
        .expect("DB query for chain 1172 must succeed")
        .expect("chain 1172 must exist in seeded content_chains and assemble successfully");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(687));
    let event = TriggerEvent {
        trigger_type: TriggerType::MissionCompleted,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    let chain_1172_actions = resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == 1172)
        .count();
    assert_eq!(
        chain_1172_actions, 0,
        "chain 1172 must NOT fire on a mission_completed event for a \
         mission other than 686; got {chain_1172_actions} actions",
    );
}

// ── GC1b-1: reposition (executor-level, real navmesh) ───────────────────

/// Chain 1173 executor-level: teleporting in to region 3 must snap Marsh
/// from his Preparation-room spawn onto the destination beside the Ring 3
/// pad, and that destination must be (a) on walkable navmesh and (b)
/// reachable from the topside route (MessHall_Guard1's spawn) — i.e. the
/// same connected navmesh component as the escort route, not the
/// Preparation room. Self-skips without the navmesh fixture.
#[tokio::test]
async fn chain_1173_teleport_in_region_3_repositions_marsh_onto_the_topside_component() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1173)
        .await
        .expect("DB query for chain 1173 must succeed")
        .expect("chain 1173 must exist in seeded content_chains and assemble successfully");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_id".to_string(), serde_json::json!(3));
    let event = TriggerEvent {
        trigger_type: TriggerType::TeleportIn,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    let moves = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1173
                && matches!(
                    action,
                    Action::MoveWaypoint { entity_tag, .. } if entity_tag == MARSH_TAG
                )
        })
        .count();
    assert_eq!(
        moves, 1,
        "chain 1173 must resolve exactly one MoveWaypoint targeting \
         {MARSH_TAG} on teleport-in to region 3; got {moves}. Resolved: \
         {:?}",
        resolved.actions,
    );

    let Some(mut mgr) = make_castle_cellblock_mgr_with_navmesh() else {
        return; // fixture-less CI — skip
    };
    // Precondition: Marsh starts at his real Preparation-room spawn, not
    // already at the destination.
    let before = mgr.get_entity(MARSH_EID).unwrap().position;
    assert_close(
        [before.x, before.y, before.z],
        PREP_COLMARSH_SPAWN,
        "precondition: Marsh's starting position",
    );

    let (tx, _rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, 42, &tx, &mut mgr, &exec_engine).await;

    let marsh = mgr
        .get_entity(MARSH_EID)
        .expect("Marsh must survive the reposition");
    assert_close(
        [marsh.position.x, marsh.position.y, marsh.position.z],
        GC1B1_DESTINATION,
        "chain 1173's move_waypoint destination",
    );

    // (a) On walkable navmesh.
    assert!(
        mgr.is_position_valid(MARSH_EID, &marsh.position),
        "GC1b-1 destination must be on walkable navmesh; got {:?}",
        marsh.position,
    );

    // (b) Reachable from — i.e. in the same connected component as — the
    // topside escort route. `find_path`'s last waypoint must land exactly
    // on the anchor; a disconnected component still returns `Some` with a
    // truncated best-effort path (confirmed directly against this
    // fixture — see chain 1173's seed comment), so "returns Some" alone
    // is not a valid connectivity proof.
    let messhall_anchor =
        cimmeria_common::Vector3::new(MESSHALL_G1[0], MESSHALL_G1[1], MESSHALL_G1[2]);
    let path = mgr
        .find_path(MARSH_EID, &marsh.position, &messhall_anchor)
        .expect("GC1b-1 destination must have a route to the topside escort route");
    let last = path
        .last()
        .expect("a non-empty path must have a last waypoint");
    assert!(
        (last.x - MESSHALL_G1[0]).abs() < 0.5
            && (last.y - MESSHALL_G1[1]).abs() < 1.0
            && (last.z - MESSHALL_G1[2]).abs() < 0.5,
        "path from the GC1b-1 destination must actually reach \
         MessHall_Guard1's spawn (same navmesh component as the topside \
         escort route), not stop short of it; last waypoint was {last:?}",
    );

    // Corroboration: the SAME query from Marsh's original Preparation-room
    // spawn must NOT reach this destination — proving the Preparation room
    // and the topside route are genuinely disconnected components, not
    // just "far apart."
    let prep_pos = cimmeria_common::Vector3::new(
        PREP_COLMARSH_SPAWN[0],
        PREP_COLMARSH_SPAWN[1],
        PREP_COLMARSH_SPAWN[2],
    );
    let dest_pos = cimmeria_common::Vector3::new(
        GC1B1_DESTINATION[0],
        GC1B1_DESTINATION[1],
        GC1B1_DESTINATION[2],
    );
    if let Some(prep_path) = mgr.find_path(MARSH_EID, &prep_pos, &dest_pos) {
        let prep_last = prep_path
            .last()
            .expect("a non-empty path must have a last waypoint");
        let reached_destination = (prep_last.x - GC1B1_DESTINATION[0]).abs() < 0.5
            && (prep_last.z - GC1B1_DESTINATION[2]).abs() < 0.5;
        assert!(
            !reached_destination,
            "the Preparation room must NOT have a real route to the GC1b-1 \
             destination -- if this fails, the two navmesh components have \
             merged (a re-bake changed the geometry) and the disconnection \
             this chain relies on no longer holds; got last waypoint \
             {prep_last:?}",
        );
    }
    // `None` (no path at all) is also an acceptable disconnected outcome.
}

// ── GC1b-2: follow start/stop (executor-level, real navmesh) ───────────

/// Chains 1173+1174 together (both fire on the SAME teleport-in-to-region-3
/// event, exactly as they will in production): after execution, Marsh must
/// be both repositioned AND following the triggering player, and a
/// subsequent AI follow tick with the player far down the escort route must
/// route Marsh via a real multi-waypoint navmesh path — not the
/// single-waypoint straight-line fallback `npc_ai::follow`'s own test pins
/// as the no-navmesh failure shape.
#[tokio::test]
async fn chain_1174_teleport_in_starts_follow_and_routes_a_real_multi_waypoint_path() {
    let pool = require_db_or_skip!();
    let chain_1173 = load_single_chain_for_test(&pool, 1173)
        .await
        .expect("DB query for chain 1173 must succeed")
        .expect("chain 1173 must exist in seeded content_chains and assemble successfully");
    let chain_1174 = load_single_chain_for_test(&pool, 1174)
        .await
        .expect("DB query for chain 1174 must succeed")
        .expect("chain 1174 must exist in seeded content_chains and assemble successfully");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain_1173);
    engine.register_chain(chain_1174);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_id".to_string(), serde_json::json!(3));
    let event = TriggerEvent {
        trigger_type: TriggerType::TeleportIn,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    let starts_follow = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1174
                && matches!(
                    action,
                    Action::SetFollowTarget { entity_tag, use_player: Some(true), .. }
                    if entity_tag == MARSH_TAG
                )
        })
        .count();
    assert_eq!(
        starts_follow, 1,
        "chain 1174 must resolve exactly one SetFollowTarget(use_player=true) \
         targeting {MARSH_TAG} on teleport-in to region 3; got \
         {starts_follow}. Resolved: {:?}",
        resolved.actions,
    );

    let Some(mut mgr) = make_castle_cellblock_mgr_with_navmesh() else {
        return; // fixture-less CI — skip
    };
    let (tx, _rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, 42, &tx, &mut mgr, &exec_engine).await;

    let marsh = mgr
        .get_entity(MARSH_EID)
        .expect("Marsh must survive follow-start");
    assert_eq!(
        marsh.follow_target_id,
        Some(PLAYER_EID),
        "Marsh's follow_target_id must resolve to the triggering player \
         after chain 1174 executes"
    );
    assert_eq!(
        marsh.ai_state(),
        AiState::Follow,
        "Marsh must transition to AiState::Follow after chain 1174 executes"
    );
    // chain 1173 (higher priority, resolves first) must have already
    // repositioned Marsh -- follow starts from the topside destination,
    // not the original Preparation-room spawn.
    assert_close(
        [marsh.position.x, marsh.position.y, marsh.position.z],
        GC1B1_DESTINATION,
        "Marsh's position when follow starts",
    );

    // Move the player far down the escort route (out of the follow band)
    // and drive one follow tick.
    if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
        p.position = cimmeria_common::Vector3::new(MESSHALL_G1[0], MESSHALL_G1[1], MESSHALL_G1[2]);
    }
    let (tx2, _rx2) = mpsc::channel(64);
    npc_ai_follow_for_test(MARSH_EID, &tx2, &mut mgr).await;

    let marsh = mgr.get_entity(MARSH_EID).unwrap();
    assert!(
        marsh.nav_path.len() > 1 && follow_leg_reaches(&marsh.nav_path, MESSHALL_G1),
        "follow must route Marsh via a real navmesh path (>1 waypoint) that \
         reaches MessHall_Guard1's spawn, not the degenerate single-waypoint \
         straight-line fallback or a route truncated at an island edge; got \
         nav_path {:?}",
        marsh.nav_path,
    );

    // Step further: player continues on to Hallway01_Guard's spawn. Snap
    // Marsh onto the mess-hall leg (simulating he closed the distance) and
    // clear the stale nav_path, then drive a second follow tick.
    if let Some(m) = mgr.get_entity_mut(MARSH_EID) {
        m.position = cimmeria_common::Vector3::new(MESSHALL_G1[0], MESSHALL_G1[1], MESSHALL_G1[2]);
        m.nav_path.clear();
    }
    if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
        p.position = cimmeria_common::Vector3::new(HALLWAY01[0], HALLWAY01[1], HALLWAY01[2]);
    }
    let (tx3, _rx3) = mpsc::channel(64);
    npc_ai_follow_for_test(MARSH_EID, &tx3, &mut mgr).await;

    let marsh = mgr.get_entity(MARSH_EID).unwrap();
    assert!(
        marsh.nav_path.len() > 1 && follow_leg_reaches(&marsh.nav_path, HALLWAY01),
        "follow must keep routing Marsh via a real navmesh path that reaches \
         the player as they continue along the hallway waypoints, not degrade \
         to a straight-line fallback or stop short; got nav_path {:?}",
        marsh.nav_path,
    );
}

/// §10 connectivity: the escort route from the Ring 3 pad reaches every
/// Hallway guard spawn, through Hallway05 where mission 686 completes. Only
/// the first hop (MessHall_Guard1) was pinned before NA42. The last
/// waypoint of `find_path` has to land on each spawn; a spawn on another
/// mesh component still returns a truncated path, which this rejects.
/// Needs only the navmesh fixture, no database.
#[test]
fn ring_3_pad_route_reaches_every_hallway_guard_spawn() {
    let Some(mgr) = make_castle_cellblock_mgr_with_navmesh() else {
        return; // fixture-less CI — skip
    };
    let pad = cimmeria_common::Vector3::new(
        GC1B1_DESTINATION[0],
        GC1B1_DESTINATION[1],
        GC1B1_DESTINATION[2],
    );
    let mut unreachable = Vec::new();
    for (tag, spawn) in HALLWAY_GUARDS {
        let goal = cimmeria_common::Vector3::new(spawn[0], spawn[1], spawn[2]);
        let path = mgr.find_path(MARSH_EID, &pad, &goal);
        if !path.as_deref().is_some_and(|p| reaches(p, spawn)) {
            unreachable.push(format!(
                "{tag}: last waypoint {:?}",
                path.and_then(|p| p.last().copied())
            ));
        }
    }
    assert!(
        unreachable.is_empty(),
        "the escort route from the Ring 3 pad must reach every Hallway guard \
         spawn; unreachable: {unreachable:#?}",
    );

    // Control: the same predicate rejects a goal on another component. The
    // Preparation room is disconnected from the topside route (chain 1173's
    // test), so its route toward Hallway05 must not count as reaching it.
    let prep = cimmeria_common::Vector3::new(
        PREP_COLMARSH_SPAWN[0],
        PREP_COLMARSH_SPAWN[1],
        PREP_COLMARSH_SPAWN[2],
    );
    let (_, hallway05) = HALLWAY_GUARDS[4];
    let goal = cimmeria_common::Vector3::new(hallway05[0], hallway05[1], hallway05[2]);
    let from_prep = mgr.find_path(MARSH_EID, &prep, &goal);
    assert!(
        !from_prep.as_deref().is_some_and(|p| reaches(p, hallway05)),
        "`reaches` must reject a truncated route from another component; \
         got last waypoint {:?}",
        from_prep.and_then(|p| p.last().copied()),
    );
}

/// Chain 1175 executor-level: mission 686 completing (the Straegis scene)
/// must clear Marsh's follow state — `follow_target_id` back to `None` and
/// `ai_state` back to `Idle`.
#[tokio::test]
async fn chain_1175_mission_686_complete_clears_marsh_follow_target() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1175)
        .await
        .expect("DB query for chain 1175 must succeed")
        .expect("chain 1175 must exist in seeded content_chains and assemble successfully");

    let mut engine = ChainEngine::new();
    engine.register_chain(chain);

    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(686));
    let event = TriggerEvent {
        trigger_type: TriggerType::MissionCompleted,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let resolved = engine.resolve_event(&event, &ctx);
    let clears = resolved
        .actions
        .iter()
        .filter(|(id, action)| {
            *id == 1175
                && matches!(
                    action,
                    Action::SetFollowTarget { entity_tag, target_tag: None, use_player: None }
                    if entity_tag == MARSH_TAG
                )
        })
        .count();
    assert_eq!(
        clears, 1,
        "chain 1175 must resolve exactly one clearing SetFollowTarget (no \
         target_tag, no use_player) targeting {MARSH_TAG} on mission 686 \
         completion; got {clears}. Resolved: {:?}",
        resolved.actions,
    );

    let Some(mut mgr) = make_castle_cellblock_mgr_with_navmesh() else {
        return; // fixture-less CI — skip
    };
    // Establish the "currently following" precondition directly (this
    // test's own concern is the CLEAR, not the start — chain 1174 already
    // covers the start).
    if let Some(m) = mgr.get_entity_mut(MARSH_EID) {
        m.follow_target_id = Some(PLAYER_EID);
        crate::cell::service::npc_ai::force_ai_state(m, AiState::Follow);
    }

    let (tx, _rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, 42, &tx, &mut mgr, &exec_engine).await;

    let marsh = mgr.get_entity(MARSH_EID).unwrap();
    assert_eq!(
        marsh.follow_target_id, None,
        "Marsh's follow_target_id must be cleared once mission 686 completes"
    );
    assert_eq!(
        marsh.ai_state(),
        AiState::Idle,
        "Marsh must drop back to AiState::Idle once mission 686 completes"
    );
}
