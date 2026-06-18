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

use cimmeria_entity::missions::{
    MissionInstance, MissionObjective, MISSION_ACTIVE, MISSION_COMPLETED, STATUS_ACTIVE,
};
use tokio::sync::mpsc;

use super::feedback::send_gm_feedback;
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

/// `gmMissionAssign(WSTRING DesignID, UINT8 popup)` — grant a mission to the
/// caller by numeric design id. Mirrors the content-engine accept path: resolve
/// the mission def's first step + objectives, then `accept_mission`. The `popup`
/// byte is a client UI hint and isn't needed server-side.
pub(super) async fn handle_mission_assign(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let Some((mission_id, _)) = parse_mission_id(entity_id, args, "gmMissionAssign") else {
        send_gm_feedback(
            entity_id,
            "gmMissionAssign: DesignID must be a positive numeric id",
            tx,
        )
        .await;
        return true;
    };
    let Some((step_id, objectives)) = space_mgr.mission_defs.get(&mission_id).map(|def| {
        let objectives: Vec<MissionObjective> = def
            .objectives
            .iter()
            .map(|o| MissionObjective {
                objective_id: o.objective_id,
                status: STATUS_ACTIVE,
                hidden: o.is_hidden,
                optional: o.is_optional,
            })
            .collect();
        (def.step_id, objectives)
    }) else {
        tracing::warn!(
            entity_id,
            mission_id,
            "gmMissionAssign: no mission def for id"
        );
        send_gm_feedback(
            entity_id,
            &format!("gmMissionAssign: no mission def for id {mission_id}"),
            tx,
        )
        .await;
        return true;
    };
    tracing::info!(
        entity_id,
        mission_id,
        step_id,
        "gmMissionAssign: assigning mission"
    );
    // accept_mission returns false when the offer guard refuses (e.g. already
    // held, or a non-repeatable mission past its repeat cap). Only report
    // success when the mutation actually happened.
    let accepted =
        missions::accept_mission(entity_id, mission_id, step_id, objectives, tx, space_mgr).await;
    let line = if accepted {
        format!("gmMissionAssign: assigned mission {mission_id} (step {step_id})")
    } else {
        format!(
            "gmMissionAssign: mission {mission_id} not assigned (already held or not offerable)"
        )
    };
    send_gm_feedback(entity_id, &line, tx).await;
    true
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
        send_gm_feedback(
            entity_id,
            "gmMissionClear: DesignID must be a positive numeric id",
            tx,
        )
        .await;
        return true;
    };
    tracing::info!(entity_id, mission_id, "gmMissionClear: abandoning mission");
    missions::abandon_mission(entity_id, mission_id, tx, space_mgr).await;
    send_gm_feedback(
        entity_id,
        &format!("gmMissionClear: abandoned mission {mission_id}"),
        tx,
    )
    .await;
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
        send_gm_feedback(
            entity_id,
            "gmMissionAdvance: DesignID must be a positive numeric id",
            tx,
        )
        .await;
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
            send_gm_feedback(
                entity_id,
                "gmMissionAdvance: missing INT32 StepToAdvanceTo",
                tx,
            )
            .await;
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
        send_gm_feedback(entity_id, "gmMissionAdvance: step must be positive", tx).await;
        return true;
    }
    tracing::info!(
        entity_id,
        mission_id,
        new_step_id,
        "gmMissionAdvance: advancing mission step"
    );
    missions::advance_step(entity_id, mission_id, new_step_id, tx, space_mgr).await;
    send_gm_feedback(
        entity_id,
        &format!("gmMissionAdvance: mission {mission_id} advanced to step {new_step_id}"),
        tx,
    )
    .await;
    true
}

// ── Mission query handlers (report via the feedback channel) ──────────────────

/// Format one mission instance into a compact line.
fn mission_line(m: &MissionInstance) -> String {
    let status = match m.status {
        MISSION_ACTIVE => "active",
        MISSION_COMPLETED => "done",
        _ => "other",
    };
    format!(
        "#{} {status} step {:?} obj {}/{}",
        m.mission_id,
        m.current_step_id,
        m.completed_objectives.len(),
        m.completed_objectives.len() + m.active_objectives.len()
    )
}

/// `gmMissionList()` — list the caller's active missions.
pub(super) async fn handle_mission_list(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let text = match space_mgr.get_entity(entity_id) {
        Some(e) => {
            let lines: Vec<String> = e
                .missions
                .active_missions()
                .iter()
                .map(|m| mission_line(m))
                .collect();
            if lines.is_empty() {
                "gmMissionList: no active missions.".to_string()
            } else {
                format!("gmMissionList ({}): {}", lines.len(), lines.join(" | "))
            }
        }
        None => "gmMissionList: no entity.".to_string(),
    };
    send_gm_feedback(entity_id, &text, tx).await;
    true
}

/// `gmMissionListFull()` — list ALL the caller's missions (incl. completed/hidden).
pub(super) async fn handle_mission_list_full(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let text = match space_mgr.get_entity(entity_id) {
        Some(e) => {
            let lines: Vec<String> = e.missions.all_missions().map(mission_line).collect();
            if lines.is_empty() {
                "gmMissionListFull: no missions.".to_string()
            } else {
                format!("gmMissionListFull ({}): {}", lines.len(), lines.join(" | "))
            }
        }
        None => "gmMissionListFull: no entity.".to_string(),
    };
    send_gm_feedback(entity_id, &text, tx).await;
    true
}

/// `gmMissionDetails(WSTRING DesignID)` — show one mission's detail (numeric id).
pub(super) async fn handle_mission_details(
    entity_id: u32,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let mission_id = match read_wstring(args, 0)
        .ok()
        .and_then(|(s, _)| s.trim().parse::<i32>().ok())
    {
        Some(id) if id > 0 => id,
        _ => {
            send_gm_feedback(
                entity_id,
                "gmMissionDetails: DesignID must be a positive numeric id.",
                tx,
            )
            .await;
            return true;
        }
    };
    let text = match space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.missions.get_mission(mission_id))
    {
        Some(m) => format!("gmMissionDetails {}", mission_line(m)),
        None => format!("gmMissionDetails: mission {mission_id} not found on you."),
    };
    send_gm_feedback(entity_id, &text, tx).await;
    true
}
