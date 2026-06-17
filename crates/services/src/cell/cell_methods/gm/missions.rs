//! GM mission handlers — `gmMissionClear` / `gmMissionAbandon` (same
//! primitive) and `gmMissionAdvance`. Each reuses the canonical cell mission
//! operations in [`crate::cell::missions`], which mutate per-player mission
//! state and emit the `onMissionUpdate` / `onStepUpdate` / `onObjectiveUpdate`
//! wire bursts.
//!
//! The def declares the mission key as a `WSTRING DesignID`. The cell mission
//! ops take a numeric `mission_id` and there is no DesignID→id reverse map in
//! the cell, so these handlers accept the numeric form only (parse the WSTRING
//! as an `i32`) and reject a non-numeric key rather than guess.

use tokio::sync::mpsc;

use super::read_i32;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::missions;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::read_wstring;

/// Parse the leading `WSTRING DesignID` as a positive numeric mission id.
/// Returns `None` (after a warn) on malformed/non-numeric input.
fn parse_mission_id(entity_id: u32, args: &[u8], cmd: &str) -> Option<(i32, usize)> {
    let (design_id_str, consumed) = match read_wstring(args, 0) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(entity_id, error = %e, cmd, "GM mission cmd: malformed DesignID WSTRING");
            return None;
        }
    };
    match design_id_str.trim().parse::<i32>() {
        Ok(id) if id > 0 => Some((id, consumed)),
        _ => {
            tracing::warn!(
                entity_id,
                design_id = %design_id_str,
                cmd,
                "GM mission cmd: DesignID is not a positive numeric mission id — \
                 name resolution is not wired in the cell; rejecting"
            );
            None
        }
    }
}

/// `gmMissionClear(WSTRING DesignID)` / `gmMissionAbandon(WSTRING DesignID)` —
/// abandon one mission by numeric design id. Both indices route here; the
/// underlying `abandon_mission` removes the mission and sends the removal to
/// the client.
pub(super) async fn handle_mission_clear(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some((mission_id, _)) = parse_mission_id(entity_id, args, "gmMissionClear") else {
        return true;
    };
    tracing::info!(entity_id, mission_id, "gmMissionClear: abandoning mission");
    missions::abandon_mission(entity_id, mission_id, tx, space_mgr).await;
    true
}

/// `gmMissionAdvance(WSTRING DesignID, INT32 StepToAdvanceTo)` — jump a
/// mission to a specific step. Reuses `advance_step`, which completes the old
/// step's objectives, sets the new step, and loads + broadcasts the new
/// objectives.
pub(super) async fn handle_mission_advance(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some((mission_id, consumed)) = parse_mission_id(entity_id, args, "gmMissionAdvance") else {
        return true;
    };
    let new_step_id = match read_i32(args, consumed) {
        Some(v) => v,
        None => {
            tracing::warn!(
                entity_id,
                args_len = args.len(),
                "gmMissionAdvance: truncated args (missing INT32 StepToAdvanceTo)"
            );
            return true;
        }
    };
    if new_step_id <= 0 {
        tracing::warn!(
            entity_id,
            mission_id,
            new_step_id,
            "gmMissionAdvance: non-positive step rejected"
        );
        return true;
    }
    tracing::info!(
        entity_id,
        mission_id,
        new_step_id,
        "gmMissionAdvance: advancing mission step"
    );
    missions::advance_step(entity_id, mission_id, new_step_id, tx, space_mgr).await;
    true
}
