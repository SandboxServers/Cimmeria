//! `npc_ai.path event=request` and the partial-path WARN (audit S8/T6).

use std::time::Instant;

use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use tracing::Level;

use super::{add_npc, add_threat_player, ai_tick, cellblock_mgr, rows, NPC};
use crate::test_support::LogCapture;

/// `MessHall_Guard1`'s spawn: the mess hall is on the main interior island.
const MESSHALL: [f32; 3] = [-96.25, 34.591, -91.59];
/// The south-west corner of the rebuilt mesh's `y ≈ 0.2` ground plane: a
/// different island from the mess hall (the mesh has 17 components).
const OTHER_ISLAND: [f32; 3] = [-400.0, 0.2, -400.0];

/// **Acceptance: a partial path across two mesh islands.** The guard
/// chases a target on another island. Detour answers
/// `DT_SUCCESS | DT_PARTIAL_RESULT`; the fight handler walks it exactly as
/// before (the path is installed), and now says so. Revert-proof: dropping
/// the `DT_PARTIAL_RESULT` / last-poly check in
/// `PathStatus::classify_corridor` reports `ok` and no `path_fail` row.
#[tokio::test]
async fn a_chase_across_two_mesh_islands_is_a_partial_path() {
    let (mut mgr, _) = cellblock_mgr();
    // No spawn anchor: the leash would otherwise end the chase first.
    add_npc(
        &mut mgr,
        "Castle_CellBlock",
        MESSHALL,
        None,
        AiState::Fighting,
    );
    add_threat_player(&mut mgr, "Castle_CellBlock", OTHER_ISLAND);

    let logs = LogCapture::install();
    ai_tick(&mut mgr).await;

    let requests = rows(&logs, "npc_ai.path", "request");
    assert_eq!(requests.len(), 1, "{:#?}", logs.all());
    let req = &requests[0];
    assert_eq!(req.level, Level::DEBUG);
    for (k, v) in [
        ("status", "partial"),
        ("state", "fight"),
        ("target_id", "101"),
        ("target_is_gm", "false"),
    ] {
        assert!(req.has_field(k, v), "{k}={v}: {req:?}");
    }
    let fails = rows(&logs, "npc_ai.path_fail", "path_fail");
    assert_eq!(fails.len(), 1, "{fails:#?}");
    assert_eq!(fails[0].level, Level::WARN);
    assert!(fails[0].has_field("reason", "partial"), "{:?}", fails[0]);
    assert!(fails[0].has_field("fallback", "partial_route"));

    // Behaviour unchanged: the partial corridor is still walked.
    assert!(
        !mgr.get_entity(NPC).unwrap().nav_path.is_empty(),
        "the partial path must still be installed (NA02 changes no decision)"
    );
}

/// A guard hovering over its floor fails `find_path`'s ±0.5 start box
/// (audit S9); the row names the stage instead of a bare `no_path`.
#[tokio::test]
async fn a_hovering_chaser_reports_no_start_poly() {
    let (mut mgr, _) = cellblock_mgr();
    add_npc(
        &mut mgr,
        "Castle_CellBlock",
        [MESSHALL[0], MESSHALL[1] + 2.0, MESSHALL[2]],
        None,
        AiState::Fighting,
    );
    add_threat_player(&mut mgr, "Castle_CellBlock", [-128.853, 39.552, -73.534]);
    let logs = LogCapture::install();
    ai_tick(&mut mgr).await;
    let req = rows(&logs, "npc_ai.path", "request");
    assert!(req[0].has_field("status", "no_start_poly"), "{:?}", req[0]);
    let fails = rows(&logs, "npc_ai.path_fail", "path_fail");
    assert!(
        fails[0].has_field("reason", "no_start_poly"),
        "{:?}",
        fails[0]
    );
}

/// `Hallway01_Guard`'s spawn: the same island as the mess hall.
const HALLWAY01: [f32; 3] = [-128.853, 39.552, -73.534];

fn request(mgr: &mut crate::cell::space_manager::SpaceManager, to: [f32; 3], now: Instant) {
    use crate::cell::service::npc_ai::path_request::{request_path, PathRequest};
    request_path(
        mgr,
        PathRequest {
            npc_id: NPC,
            state: "patrol",
            from: v(MESSHALL),
            to: v(to),
            target_id: None,
            partial_outcome: "patrol_partial",
        },
        now,
    );
}

fn v(p: [f32; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

/// An `ok` route is sampled per NPC; any other status is logged every time.
/// Revert-proof: dropping the `admit_sample` gate on `ok` in `log_request`
/// logs both healthy requests.
#[test]
fn ok_requests_are_sampled_and_failures_are_not() {
    let (mut mgr, _) = cellblock_mgr();
    add_npc(
        &mut mgr,
        "Castle_CellBlock",
        MESSHALL,
        None,
        AiState::Patrol,
    );
    let now = Instant::now();
    let logs = LogCapture::install();
    request(&mut mgr, HALLWAY01, now);
    request(&mut mgr, HALLWAY01, now);
    let unmeshed = [MESSHALL[0], MESSHALL[1] + 300.0, MESSHALL[2]];
    request(&mut mgr, unmeshed, now);
    request(&mut mgr, unmeshed, now);
    let reqs = rows(&logs, "npc_ai.path", "request");
    let statuses: Vec<&str> = reqs.iter().map(|r| r.fields["status"].as_str()).collect();
    assert_eq!(statuses, ["ok", "no_end_poly", "no_end_poly"], "{reqs:#?}");
}

/// A partial route has its own throttle window: an NPC repathing into an
/// island edge must not hold back a real routing failure. Revert-proof:
/// sharing the `path_fail` kind again suppresses the second row.
#[test]
fn a_partial_route_does_not_suppress_a_real_path_failure() {
    use crate::cell::service::npc_ai::path_failure::{
        report_path_failure, PathFailReason, PathFailure, PathFallback,
    };
    let (mut mgr, _) = cellblock_mgr();
    add_npc(
        &mut mgr,
        "Castle_CellBlock",
        MESSHALL,
        None,
        AiState::Patrol,
    );
    let now = Instant::now();
    let failure = |reason, fallback| PathFailure {
        npc_id: NPC,
        state: "patrol",
        decision_outcome: "patrol_no_path",
        from: v(MESSHALL),
        to: v(OTHER_ISLAND),
        reason,
        fallback,
        target_id: None,
    };
    let logs = LogCapture::install();
    report_path_failure(
        &mut mgr,
        failure(PathFailReason::Partial, PathFallback::PartialRoute),
        now,
    );
    report_path_failure(
        &mut mgr,
        failure(PathFailReason::NoStartPoly, PathFallback::DirectWaypoint),
        now,
    );
    let fails = rows(&logs, "npc_ai.path_fail", "path_fail");
    let reasons: Vec<&str> = fails.iter().map(|r| r.fields["reason"].as_str()).collect();
    assert_eq!(reasons, ["partial", "no_start_poly"], "{fails:#?}");
}
