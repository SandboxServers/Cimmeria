//! Guards for the shared NPC path-failure emitter, and for each AI
//! state actually being wired to it.
//!
//! The production gap: before this module, `patrol`, `investigate` and
//! `wander` turned `find_path(..) -> None` into an empty `Vec` via
//! `unwrap_or_default()`, pushed the raw destination as a single
//! waypoint, and let the NPC walk through geometry toward it with **no
//! log at any level**. The 2026-09-18 Castle playtest saw NPCs cutting
//! through walls; the only evidence in telemetry was the follow legs,
//! because follow was the one state somebody had instrumented.
//!
//! Every test below runs in a navmesh-less fixture, which is the
//! `no_mesh` arm. That is deliberate: it is the arm that reproduces
//! without a mesh fixture, and the `no_path` arm differs only in the
//! `reason` token (`PathFailReason::for_missing_path` is the only
//! branch, and it is covered directly).

use std::time::{Duration, Instant};

use cimmeria_common::Vector3;
use cimmeria_entity::navigation::PathStatus;
use tokio::sync::mpsc;

use super::{report_path_failure, PathFailReason, PathFailure, PathFallback};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

/// Navmesh-less "Agnos", the same fixture the follow tests use.
fn make_space_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr
}

fn failure(npc_id: u32, state: &'static str, outcome: &'static str) -> PathFailure {
    PathFailure {
        npc_id,
        state,
        decision_outcome: outcome,
        from: Vector3::new(0.0, 0.0, 0.0),
        to: Vector3::new(10.0, 4.0, 0.0),
        reason: PathFailReason::NoMesh,
        fallback: PathFallback::DirectWaypoint,
        target_id: None,
    }
}

fn rows(capture: &crate::test_support::LogCaptureGuard) -> Vec<crate::test_support::Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.message_contains("npc_ai.path_fail"))
        .collect()
}

/// The row shape every state shares.
#[test]
fn the_row_carries_state_world_geometry_and_reason() {
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let capture = LogCapture::install();

    report_path_failure(
        &mut mgr,
        failure(101, "patrol", "patrol_no_path"),
        Instant::now(),
    );

    let rows = rows(&capture);
    assert_eq!(rows.len(), 1);
    let ev = &rows[0];
    assert_eq!(ev.level, tracing::Level::WARN);
    assert!(ev.has_field("npc_id", "101"), "{ev:#?}");
    assert!(ev.has_field("state", "patrol"), "{ev:#?}");
    assert!(ev.has_field("reason", "no_mesh"), "{ev:#?}");
    assert!(
        ev.has_field("decision_outcome", "patrol_no_path"),
        "the row must carry the decision_outcome vocabulary value so a \
         SigNoz groupBy over the enum sees path failures: {ev:#?}"
    );
    assert!(
        ev.has_field("world", "Agnos"),
        "a path failure is a per-zone content problem — without `world` \
         it cannot be triaged to a map: {ev:#?}"
    );
    assert!(ev.fields.contains_key("dist"), "{ev:#?}");
    assert!(
        ev.fields.contains_key("dy"),
        "dy is the air-climb signature: a large positive dy means the \
         straight-line fallback will drag the NPC upward through \
         geometry: {ev:#?}"
    );
    assert!(
        !ev.fields.contains_key("navmesh_hash"),
        "there is no mesh in this space, so there is no hash to name: {ev:#?}"
    );
}

/// `no_mesh` (the zone has no `.nav` at all) and `no_path` (there is a
/// mesh and it has a hole or a split) are different problems with
/// different owners, and the classifier is the only thing that tells
/// them apart.
#[test]
fn reason_distinguishes_a_missing_mesh_from_a_missing_route() {
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    assert_eq!(
        PathFailReason::for_missing_path(&mgr, 101, None),
        PathFailReason::NoMesh,
        "Agnos has no navmesh loaded"
    );

    let nav_path = std::path::Path::new("../../data/spaces/castle_cellblock.nav");
    if !nav_path.exists() {
        return; // fixture-less CI — the no_path arm needs a real mesh
    }
    let navmesh = cimmeria_entity::navigation::NavMesh::load(nav_path).unwrap();
    let space_id = mgr.get_entity_space_id(101).unwrap();
    mgr.spaces.get_mut(&space_id).unwrap().navmesh = Some(navmesh);
    assert_eq!(
        PathFailReason::for_missing_path(&mgr, 101, None),
        PathFailReason::NoPath,
        "with a mesh loaded, a failed route is a hole in the mesh, not a \
         missing mesh — reporting no_mesh here would send an operator to \
         build a .nav that already exists"
    );
}

/// A handler that enqueued nothing says something different to the
/// operator: the NPC keeps its *previous* path, so it walks toward
/// where the target used to be — or does not move at all.
#[test]
fn a_path_unchanged_fallback_gets_its_own_message() {
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let capture = LogCapture::install();

    let mut f = failure(101, "fight", "repath_degenerate");
    f.reason = PathFailReason::DegeneratePath;
    f.fallback = PathFallback::PathUnchanged;
    f.target_id = Some(202);
    report_path_failure(&mut mgr, f, Instant::now());

    let ev = &rows(&capture)[0];
    assert!(ev.has_field("reason", "degenerate_path"), "{ev:#?}");
    assert!(ev.has_field("fallback", "path_unchanged"), "{ev:#?}");
    assert!(ev.has_field("target_id", "202"), "{ev:#?}");
    assert!(
        ev.message_contains("enqueued nothing"),
        "leaving a stale route running is a different symptom from a \
         straight-line fallback: {ev:#?}"
    );
    assert!(
        !ev.message_contains("straight line"),
        "this NPC is not walking a straight line through geometry — it \
         is not walking anywhere new at all: {ev:#?}"
    );
}

/// **The message-accuracy guard.** The shared emitter used to pick the
/// message from `reason`, which made every non-degenerate failure claim
/// "falling back to a straight line through geometry". `fight`'s
/// `no_path` branch enqueues nothing, so that row sent operators
/// looking for a wall-clipping NPC that was in fact standing still.
///
/// `reason` and `fallback` are independent: the same `no_path` reason
/// must produce the straight-line message from a state that pushes a
/// direct waypoint and the stale-path message from one that does not.
#[test]
fn the_message_follows_the_fallback_not_the_reason() {
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.spawn_npc(102, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let capture = LogCapture::install();

    let mut standing = failure(101, "fight", "no_path");
    standing.fallback = PathFallback::PathUnchanged;
    report_path_failure(&mut mgr, standing, Instant::now());

    let mut walking = failure(102, "patrol", "patrol_no_path");
    walking.fallback = PathFallback::DirectWaypoint;
    report_path_failure(&mut mgr, walking, Instant::now());

    let rows = rows(&capture);
    let fight = rows
        .iter()
        .find(|r| r.has_field("state", "fight"))
        .expect("fight row");
    let patrol = rows
        .iter()
        .find(|r| r.has_field("state", "patrol"))
        .expect("patrol row");

    assert!(fight.has_field("reason", "no_mesh") && patrol.has_field("reason", "no_mesh"));
    assert!(
        fight.message_contains("enqueued nothing") && !fight.message_contains("straight line"),
        "fight's no-path branch does not enqueue a direct waypoint, so \
         the row must not promise a straight-line fallback: {fight:#?}"
    );
    assert!(
        patrol.message_contains("straight line"),
        "patrol does push the raw waypoint, which is the wall-clipping \
         shape the 2026-09-18 Castle playtest saw: {patrol:#?}"
    );
}

/// `classify` must keep `None` and a returned one-waypoint path apart.
/// Collapsing the result to a `Vec` first — which every caller but
/// `fight` used to do — reported a degenerate answer as `no_mesh` /
/// `no_path`, so "the mesh is missing" and "the mesh answered with
/// something unwalkable" looked identical in a SigNoz `group by
/// reason`.
#[test]
fn classify_separates_a_missing_path_from_a_degenerate_one() {
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    assert_eq!(
        PathFailReason::classify(&mgr, 101, None, None),
        PathFailReason::NoMesh,
        "a meshless space declining to route is a zone-level content gap"
    );
    let one = [Vector3::new(0.0, 0.0, 0.0)];
    assert_eq!(
        PathFailReason::classify(&mgr, 101, Some(PathStatus::Ok), Some(&one)),
        PathFailReason::DegeneratePath,
        "a pathfinder that answered with one waypoint did not fail to \
         find the mesh — it found it and returned something unwalkable"
    );
}

/// NA02: a failed route names the Detour stage that declined, instead of
/// collapsing all three into `no_path` (audit T6).
#[test]
fn a_missing_route_names_the_detour_stage_that_failed() {
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    for (status, reason, label) in [
        (
            PathStatus::NoStartPoly,
            PathFailReason::NoStartPoly,
            "no_start_poly",
        ),
        (
            PathStatus::NoEndPoly,
            PathFailReason::NoEndPoly,
            "no_end_poly",
        ),
        (
            PathStatus::NoCorridor,
            PathFailReason::NoCorridor,
            "no_corridor",
        ),
        (PathStatus::Partial, PathFailReason::Partial, "partial"),
    ] {
        let got = PathFailReason::classify(&mgr, 101, Some(status), None);
        assert_eq!(got, reason, "{status:?}");
        assert_eq!(got.label(), label);
    }
}

/// **The throttle guard.** These fire per AI tick; a stuck NPC is a
/// standing condition, not an event. Reverting the `admit` gate makes
/// this emit 6 rows instead of 2.
#[test]
fn a_stuck_npc_is_throttled_and_the_count_is_reported() {
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let capture = LogCapture::install();
    let t0 = Instant::now();

    for _ in 0..5 {
        report_path_failure(&mut mgr, failure(101, "wander", "wander_no_path"), t0);
    }
    assert_eq!(
        rows(&capture).len(),
        1,
        "five ticks of the same stuck NPC must produce one row — at the \
         ~100ms retry-sweep cadence an unthrottled emitter writes a row \
         every tick for as long as the zone is up"
    );

    report_path_failure(
        &mut mgr,
        failure(101, "wander", "wander_no_path"),
        t0 + Duration::from_secs(6),
    );
    let rows = rows(&capture);
    assert_eq!(rows.len(), 2);
    assert!(rows[1].has_field("suppressed", "4"), "{:#?}", rows[1]);
}

/// One stuck NPC must not hide another's first failure.
#[test]
fn one_npcs_throttle_does_not_silence_another() {
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.spawn_npc(102, "Agnos", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let capture = LogCapture::install();
    let t0 = Instant::now();

    for _ in 0..3 {
        report_path_failure(&mut mgr, failure(101, "patrol", "patrol_no_path"), t0);
    }
    report_path_failure(&mut mgr, failure(102, "patrol", "patrol_no_path"), t0);

    let rows = rows(&capture);
    assert_eq!(rows.len(), 2);
    assert!(
        rows.iter().any(|r| r.has_field("npc_id", "102")),
        "{rows:#?}"
    );
}

/// Throttle state dies with the NPC, so a despawn/respawn cycle (or an
/// id reuse) starts with a clean window.
#[test]
fn throttle_state_is_released_when_the_npc_is_destroyed() {
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    report_path_failure(
        &mut mgr,
        failure(101, "patrol", "patrol_no_path"),
        Instant::now(),
    );
    assert!(mgr.movement_telemetry.tracked() > 0);

    mgr.destroy_entity(101);
    assert_eq!(
        mgr.movement_telemetry.tracked(),
        0,
        "an NPC's path-failure window must not outlive it"
    );
}

// ── Per-state wiring ──────────────────────────────────────────────────
//
// One test per AI state, driving the real handler. These are what fail
// if a handler's call to `report_path_failure` is removed — the shared
// emitter tests above would all still pass.

/// Patrol used to route through a wall in silence.
#[tokio::test]
async fn patrol_reports_its_path_failure() {
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        crate::cell::service::npc_ai::force_ai_state(
            npc,
            cimmeria_entity::cell_entity::AiState::Patrol,
        );
        npc.patrol_path = vec![Vector3::new(40.0, 0.0, 40.0), Vector3::new(60.0, 0.0, 60.0)];
        npc.patrol_next_index = 0;
    }
    let capture = LogCapture::install();
    let (tx, _rx) = mpsc::channel(8);
    super::super::patrol::npc_ai_patrol(101, &tx, &mut mgr).await;

    let rows = rows(&capture);
    assert_eq!(
        rows.len(),
        1,
        "a patrol leg with no route must say so — it silently pushed the \
         raw waypoint and walked through geometry before this: {rows:#?}"
    );
    assert!(rows[0].has_field("state", "patrol"), "{:#?}", rows[0]);
    // The behaviour itself is unchanged: the fallback waypoint is still
    // queued. This is observability, not a movement change.
    assert_eq!(
        mgr.get_entity(101).unwrap().nav_path.len(),
        1,
        "logging the failure must not change what the NPC does"
    );
}

/// Wander used to route through a wall in silence.
#[tokio::test]
async fn wander_reports_its_path_failure() {
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        crate::cell::service::npc_ai::force_ai_state(
            npc,
            cimmeria_entity::cell_entity::AiState::Wander,
        );
        npc.wander_radius = 20.0;
        npc.spawn_position = Some(Vector3::new(10.0, 0.0, 10.0));
        // `None` means "just arrived" and only stamps a dwell; the
        // pathfinding branch is the elapsed-dwell arm.
        npc.wander_next_at = Some(Instant::now() - Duration::from_secs(60));
    }
    let capture = LogCapture::install();
    let (tx, _rx) = mpsc::channel(8);
    super::super::wander::npc_ai_wander(101, &tx, &mut mgr).await;

    let rows = rows(&capture);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert!(rows[0].has_field("state", "wander"), "{:#?}", rows[0]);
}

/// Investigate used to route through a wall in silence.
#[tokio::test]
async fn investigate_reports_its_path_failure() {
    let mut mgr = make_space_mgr();
    mgr.spawn_npc(101, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        crate::cell::service::npc_ai::force_ai_state(
            npc,
            cimmeria_entity::cell_entity::AiState::Investigating,
        );
        npc.poi = Some(Vector3::new(40.0, 0.0, 40.0));
        npc.investigate_until = None;
    }
    let capture = LogCapture::install();
    let (tx, _rx) = mpsc::channel(8);
    super::super::investigate::npc_ai_investigate(101, &tx, &mut mgr).await;

    let rows = rows(&capture);
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert!(rows[0].has_field("state", "investigate"), "{:#?}", rows[0]);
}
