use super::super::*; // gm module: dispatch + GM_* constants
use super::*; // shared helpers from tests/mod.rs
use tokio::sync::mpsc;

#[tokio::test]
async fn gm_mission_clear_rejects_non_numeric_design_id() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "FindAmbernol");
    assert!(dispatch(1, GM_MISSION_CLEAR, &args, &tx, &mut mgr).await);
    assert!(
        rx.try_recv().is_err(),
        "non-numeric DesignID must not emit a mission update"
    );
}

#[tokio::test]
async fn gm_mission_advance_truncated_step_is_noop() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    // Numeric DesignID but missing the INT32 step → no panic, no emit.
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "1001");
    assert!(dispatch(1, GM_MISSION_ADVANCE, &args, &tx, &mut mgr).await);
    assert!(rx.try_recv().is_err(), "missing step must not advance");
}

#[tokio::test]
async fn mission_handlers_reject_malformed_design_id() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    // Empty args → WSTRING parse fails for both.
    assert!(dispatch(1, GM_MISSION_CLEAR, &[], &tx, &mut mgr).await);
    assert!(dispatch(1, GM_MISSION_ADVANCE, &[], &tx, &mut mgr).await);
    // Non-numeric design id on advance.
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "QuestName");
    args.extend_from_slice(&2i32.to_le_bytes());
    assert!(dispatch(1, GM_MISSION_ADVANCE, &args, &tx, &mut mgr).await);
    assert!(
        drain(&mut rx).is_empty(),
        "malformed/non-numeric mission id must emit nothing"
    );
}

#[tokio::test]
async fn mission_list_reports_no_missions() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    assert!(dispatch(1, GM_MISSION_LIST, &[], &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(fb.contains("no active missions"), "got: {fb}");
}

use crate::cell::spawner::{MissionDefEntry, MissionObjectiveDef};

/// Seed a single mission def (id 1001 → step 200, one objective) so the
/// `gmMissionAssign` happy path can resolve it without a DB. `is_hidden=false`
/// is required for the assigned mission to appear in `active_missions()` (the
/// manager filters hidden instances out of the active list).
fn seed_mission_1001(mgr: &mut SpaceManager) {
    mgr.mission_defs.insert(
        1001,
        MissionDefEntry {
            step_id: 200,
            objectives: vec![MissionObjectiveDef {
                objective_id: 300,
                is_hidden: false,
                is_optional: false,
            }],
            is_hidden: false,
            num_repeats: 0,
            can_repeat_on_fail: false,
        },
    );
    // Objectives for the step we advance to, so `gmMissionAdvance` loads a
    // real objective set rather than an empty one.
    mgr.step_objectives.insert(
        201,
        vec![MissionObjectiveDef {
            objective_id: 301,
            is_hidden: false,
            is_optional: false,
        }],
    );
}

/// Build `(WSTRING "1001", UINT8 popup)` for `gmMissionAssign`.
fn assign_args(design_id: &str, popup: u8) -> Vec<u8> {
    let mut args = Vec::new();
    write_wstring_arg(&mut args, design_id);
    args.push(popup);
    args
}

/// Full GM mission lifecycle over one seeded def: assign → list → list-full →
/// details → advance → clear. This single flow drives the success branch of
/// all six handlers plus `mission_line`. Each step asserts on the resulting
/// per-player mission state or feedback text, not just "no panic" — so a
/// regression that drops the assign mutation, mis-formats `mission_line`, or
/// fails to remove on clear trips the corresponding assertion.
#[tokio::test]
async fn mission_lifecycle_assign_list_advance_clear() {
    let mut mgr = mgr_with_player(1, "Castle");
    seed_mission_1001(&mut mgr);
    let (tx, mut rx) = mpsc::channel(32);

    // ── Assign ──────────────────────────────────────────────────────────────
    assert!(dispatch(1, GM_MISSION_ASSIGN, &assign_args("1001", 1), &tx, &mut mgr).await);
    drain(&mut rx); // discard the onMissionUpdate/onStepUpdate/onObjectiveUpdate burst
    {
        let e = mgr.get_entity(1).expect("caller exists");
        let m = e
            .missions
            .get_mission(1001)
            .expect("gmMissionAssign must add mission 1001 to the caller");
        assert_eq!(
            m.current_step_id,
            Some(200),
            "assigned mission must start at the def's first step"
        );
        assert_eq!(
            e.missions.active_missions().len(),
            1,
            "the assigned mission must be active and visible"
        );
    }

    // ── List (active only) ────────────────────────────────────────────────────
    assert!(dispatch(1, GM_MISSION_LIST, &[], &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("gmMissionList must feed back");
    assert!(
        fb.contains("#1001") && fb.contains("active"),
        "list must show the assigned mission as active, got: {fb}"
    );

    // ── List-full (all missions) ──────────────────────────────────────────────
    assert!(dispatch(1, GM_MISSION_LIST_FULL, &[], &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("gmMissionListFull must feed back");
    assert!(
        fb.contains("#1001"),
        "list-full must include the assigned mission, got: {fb}"
    );

    // ── Details (one mission by numeric DesignID) ─────────────────────────────
    let mut details_args = Vec::new();
    write_wstring_arg(&mut details_args, "1001");
    assert!(dispatch(1, GM_MISSION_DETAILS, &details_args, &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("gmMissionDetails must feed back");
    assert!(
        fb.contains("gmMissionDetails") && fb.contains("#1001"),
        "details must report the requested mission, got: {fb}"
    );

    // ── Advance to step 201 ───────────────────────────────────────────────────
    let mut advance_args = Vec::new();
    write_wstring_arg(&mut advance_args, "1001");
    advance_args.extend_from_slice(&201i32.to_le_bytes());
    assert!(dispatch(1, GM_MISSION_ADVANCE, &advance_args, &tx, &mut mgr).await);
    drain(&mut rx);
    assert_eq!(
        mgr.get_entity(1)
            .unwrap()
            .missions
            .get_mission(1001)
            .expect("mission still present after advance")
            .current_step_id,
        Some(201),
        "gmMissionAdvance must move the mission to the requested step"
    );

    // ── Clear (abandon) ───────────────────────────────────────────────────────
    let mut clear_args = Vec::new();
    write_wstring_arg(&mut clear_args, "1001");
    assert!(dispatch(1, GM_MISSION_CLEAR, &clear_args, &tx, &mut mgr).await);
    drain(&mut rx);
    assert!(
        mgr.get_entity(1)
            .unwrap()
            .missions
            .get_mission(1001)
            .is_none(),
        "gmMissionClear must remove the mission from the caller"
    );
}

/// `gmMissionDetails` for a numeric id the caller doesn't hold must report a
/// "not found" line (not a panic, not silence) — the not-found feedback branch.
#[tokio::test]
async fn mission_details_unknown_id_reports_not_found() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "1001");
    assert!(dispatch(1, GM_MISSION_DETAILS, &args, &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(
        fb.contains("not found") && fb.contains("1001"),
        "details for an unheld mission must report not-found, got: {fb}"
    );
}

/// `gmMissionDetails` with a non-numeric DesignID must report the guidance
/// feedback (the early reject branch in the handler, distinct from the
/// parse-then-lookup path).
#[tokio::test]
async fn mission_details_non_numeric_design_id_reports_guidance() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "FindAmbernol");
    assert!(dispatch(1, GM_MISSION_DETAILS, &args, &tx, &mut mgr).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(
        fb.contains("positive numeric id"),
        "non-numeric details must report guidance, got: {fb}"
    );
}
