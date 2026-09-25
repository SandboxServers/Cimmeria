//! `npc_ai.path event=request` and the partial-path WARN (audit S8/T6).

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
    let Some((mut mgr, _)) = cellblock_mgr() else {
        return;
    };
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
    let Some((mut mgr, _)) = cellblock_mgr() else {
        return;
    };
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
