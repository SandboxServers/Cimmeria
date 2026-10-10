use super::super::*; // gm module: dispatch + GM_* constants
use super::*; // shared helpers from tests/mod.rs
use crate::cell::messages::CellToBaseMsg;
use tokio::sync::mpsc;

/// A mission *action* is any client method push other than the feedback line
/// (method 28): the onMissionUpdate / onStepUpdate / onObjectiveUpdate burst.
fn is_mission_action(m: &CellToBaseMsg) -> bool {
    matches!(
        m,
        CellToBaseMsg::EntityMethodCall { method_index, .. } if *method_index != 28
    )
}

#[tokio::test]
async fn gm_mission_clear_rejects_non_numeric_design_id() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "FindAmbernol");
    assert!(dispatch(1, GM_MISSION_CLEAR, &args, &tx, &mut mgr, &test_engine()).await);
    let msgs = drain(&mut rx);
    assert!(
        !msgs.iter().any(is_mission_action),
        "non-numeric DesignID must not emit a mission update"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "non-numeric DesignID must feed back a rejection"
    );
}

#[tokio::test]
async fn gm_mission_advance_truncated_step_is_noop() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    // Numeric DesignID but missing the INT32 step → no panic, no emit.
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "1001");
    assert!(dispatch(1, GM_MISSION_ADVANCE, &args, &tx, &mut mgr, &test_engine()).await);
    let msgs = drain(&mut rx);
    assert!(
        !msgs.iter().any(is_mission_action),
        "missing step must not advance"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "missing step must feed back a rejection"
    );
}

#[tokio::test]
async fn mission_handlers_reject_malformed_design_id() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    // Empty args → WSTRING parse fails for both.
    assert!(dispatch(1, GM_MISSION_CLEAR, &[], &tx, &mut mgr, &test_engine()).await);
    assert!(dispatch(1, GM_MISSION_ADVANCE, &[], &tx, &mut mgr, &test_engine()).await);
    // Non-numeric design id on advance.
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "QuestName");
    args.extend_from_slice(&2i32.to_le_bytes());
    assert!(dispatch(1, GM_MISSION_ADVANCE, &args, &tx, &mut mgr, &test_engine()).await);
    let msgs = drain(&mut rx);
    assert!(
        !msgs.iter().any(is_mission_action),
        "malformed/non-numeric mission id must not emit a mission update"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "malformed/non-numeric mission id must feed back a rejection"
    );
}

#[tokio::test]
async fn mission_list_reports_no_missions() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    assert!(dispatch(1, GM_MISSION_LIST, &[], &tx, &mut mgr, &test_engine()).await);
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
    // Both steps belong to 1001, so `gmMissionAdvance` accepts them.
    mgr.step_missions.insert(200, 1001);
    mgr.step_missions.insert(201, 1001);
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
    assert!(
        dispatch(
            1,
            GM_MISSION_ASSIGN,
            &assign_args("1001", 1),
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
    // Drain the onMissionUpdate/onStepUpdate/onObjectiveUpdate burst, but first
    // confirm the cell-local success fed back what happened.
    let assign_msgs = drain(&mut rx);
    let fb = feedback_text(&assign_msgs, 1).expect("gmMissionAssign success must feed back");
    assert!(
        fb.contains("assigned") && fb.contains("1001"),
        "assign feedback must report the assigned mission, got: {fb}"
    );
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
    assert!(dispatch(1, GM_MISSION_LIST, &[], &tx, &mut mgr, &test_engine()).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("gmMissionList must feed back");
    assert!(
        fb.contains("#1001") && fb.contains("active"),
        "list must show the assigned mission as active, got: {fb}"
    );

    // ── List-full (all missions) ──────────────────────────────────────────────
    assert!(dispatch(1, GM_MISSION_LIST_FULL, &[], &tx, &mut mgr, &test_engine()).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("gmMissionListFull must feed back");
    assert!(
        fb.contains("#1001"),
        "list-full must include the assigned mission, got: {fb}"
    );

    // ── Details (one mission by numeric DesignID) ─────────────────────────────
    let mut details_args = Vec::new();
    write_wstring_arg(&mut details_args, "1001");
    assert!(
        dispatch(
            1,
            GM_MISSION_DETAILS,
            &details_args,
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
    let fb = feedback_text(&drain(&mut rx), 1).expect("gmMissionDetails must feed back");
    assert!(
        fb.contains("gmMissionDetails") && fb.contains("#1001"),
        "details must report the requested mission, got: {fb}"
    );

    // ── Advance to step 201 ───────────────────────────────────────────────────
    let mut advance_args = Vec::new();
    write_wstring_arg(&mut advance_args, "1001");
    advance_args.extend_from_slice(&201i32.to_le_bytes());
    assert!(
        dispatch(
            1,
            GM_MISSION_ADVANCE,
            &advance_args,
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
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
    assert!(
        dispatch(
            1,
            GM_MISSION_CLEAR,
            &clear_args,
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
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
    assert!(dispatch(1, GM_MISSION_DETAILS, &args, &tx, &mut mgr, &test_engine()).await);
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
    assert!(dispatch(1, GM_MISSION_DETAILS, &args, &tx, &mut mgr, &test_engine()).await);
    let fb = feedback_text(&drain(&mut rx), 1).expect("must feed back");
    assert!(
        fb.contains("positive numeric id"),
        "non-numeric details must report guidance, got: {fb}"
    );
}

/// The `MissionUpdate`s (the base's `sgw_mission` UPSERT) in `msgs`, as
/// `(player_id, mission_id, status, current_step_id, active_objective_ids)`.
fn mission_updates(msgs: &[CellToBaseMsg]) -> Vec<(i32, i32, i8, Option<i32>, Vec<i32>)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::MissionUpdate {
                player_id,
                mission_id,
                status,
                current_step_id,
                active_objective_ids,
                ..
            } => Some((
                *player_id,
                *mission_id,
                *status,
                *current_step_id,
                active_objective_ids.clone(),
            )),
            _ => None,
        })
        .collect()
}

/// **Regression guard (CS-08 F9).** `gmMissionAssign` must persist the
/// mission like a content accept: exactly one `MissionUpdate` for the caller's
/// player id, read back from the live instance (status active, the def's
/// first step, its objective). Before the fix the handler changed cell memory
/// only, so the assigned mission was gone after a relog. Removing the
/// `send_mission_update` call fails this.
#[tokio::test]
async fn gm_mission_assign_persists_the_mission() {
    let mut mgr = mgr_with_player(1, "Castle");
    seed_mission_1001(&mut mgr);
    let (tx, mut rx) = mpsc::channel(32);

    assert!(
        dispatch(
            1,
            GM_MISSION_ASSIGN,
            &assign_args("1001", 1),
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
    assert_eq!(
        mission_updates(&drain(&mut rx)),
        vec![(
            100,
            1001,
            cimmeria_entity::missions::MISSION_ACTIVE,
            Some(200),
            vec![300]
        )],
        "the assign must persist one active row at the first step"
    );

    // A second assign is refused by the offer guard: nothing changed, so
    // nothing is persisted (a refused accept that persisted is #411).
    assert!(
        dispatch(
            1,
            GM_MISSION_ASSIGN,
            &assign_args("1001", 1),
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
    assert!(
        mission_updates(&drain(&mut rx)).is_empty(),
        "a refused assign must not persist"
    );
}

/// **Regression guard (CS-08 F9).** `gmMissionAdvance` persists the new step
/// as the content `advance_step` action does; an advance of a mission the
/// caller does not hold persists nothing. Removing its
/// `send_mission_update` call fails this.
#[tokio::test]
async fn gm_mission_advance_persists_the_new_step() {
    let mut mgr = mgr_with_player(1, "Castle");
    seed_mission_1001(&mut mgr);
    let (tx, mut rx) = mpsc::channel(32);

    let mut advance_args = Vec::new();
    write_wstring_arg(&mut advance_args, "1001");
    advance_args.extend_from_slice(&201i32.to_le_bytes());

    // Not held yet: no row.
    assert!(
        dispatch(
            1,
            GM_MISSION_ADVANCE,
            &advance_args,
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
    assert!(
        mission_updates(&drain(&mut rx)).is_empty(),
        "advancing a mission the caller does not hold must not persist"
    );

    assert!(
        dispatch(
            1,
            GM_MISSION_ASSIGN,
            &assign_args("1001", 1),
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
    drain(&mut rx);
    assert!(
        dispatch(
            1,
            GM_MISSION_ADVANCE,
            &advance_args,
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
    assert_eq!(
        mission_updates(&drain(&mut rx)),
        vec![(
            100,
            1001,
            cimmeria_entity::missions::MISSION_ACTIVE,
            Some(201),
            vec![301]
        )],
        "the advance must persist the mission at the new step"
    );
}

/// **Regression guard (CS-08 review S1).** Since the advance is saved, a step
/// that is not one of the mission's steps must be refused with feedback and
/// change nothing: not the cell's step, and no `MissionUpdate`. Both a step of
/// another mission and an unknown step are covered. Removing the ownership
/// check saves the wrong step and fails this.
#[tokio::test]
async fn gm_mission_advance_refuses_a_step_of_another_mission() {
    let mut mgr = mgr_with_player(1, "Castle");
    seed_mission_1001(&mut mgr);
    mgr.step_missions.insert(4642, 2002);
    let (tx, mut rx) = mpsc::channel(32);
    assert!(
        dispatch(
            1,
            GM_MISSION_ASSIGN,
            &assign_args("1001", 1),
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
    drain(&mut rx);

    for (step, expect) in [
        (4642_i32, "belongs to mission 2002"),
        (999_999, "not a known"),
    ] {
        let mut args = Vec::new();
        write_wstring_arg(&mut args, "1001");
        args.extend_from_slice(&step.to_le_bytes());
        assert!(dispatch(1, GM_MISSION_ADVANCE, &args, &tx, &mut mgr, &test_engine()).await);
        let msgs = drain(&mut rx);
        assert!(
            mission_updates(&msgs).is_empty(),
            "step {step}: a refused advance saves nothing"
        );
        let fb = feedback_text(&msgs, 1).expect("a refused advance must feed back");
        assert!(fb.contains(expect), "step {step}: {fb}");
        assert_eq!(
            mgr.get_entity(1)
                .unwrap()
                .missions
                .get_mission(1001)
                .unwrap()
                .current_step_id,
            Some(200),
            "step {step}: the mission stays at its step"
        );
    }
}

/// **Regression guard (CS-08 review S3).** `gmMissionAssign` runs the
/// mission's `mission_accepted` chains, as a content accept does. A one-chain
/// engine bumps a counter on the accept; removing the
/// `fire_mission_accepted` call leaves it at 0.
#[tokio::test]
async fn gm_mission_assign_fires_mission_accepted() {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::{Chain, ChainEngine};
    use cimmeria_content_engine::triggers::Trigger;

    let mut mgr = mgr_with_player(1, "Castle");
    mgr.connect_entity(1);
    seed_mission_1001(&mut mgr);
    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 0x7005_1314,
        name: "test: count mission_accepted 1001".to_string(),
        enabled: true,
        trigger: Trigger::OnMissionAccepted { mission_id: 1001 },
        conditions: vec![],
        actions: vec![Action::IncrementCounter {
            counter_name: "accepted_1001".to_string(),
            amount: 1,
        }],
        action_delays: Vec::new(),
        priority: 0,
        once: false,
    });
    let (tx, _rx) = mpsc::channel(64);

    assert!(
        dispatch(
            1,
            GM_MISSION_ASSIGN,
            &assign_args("1001", 1),
            &tx,
            &mut mgr,
            &engine
        )
        .await
    );
    assert_eq!(
        mgr.get_entity(1)
            .and_then(|e| e.counters.get("accepted_1001").copied()),
        Some(1),
        "the assign must fire mission_accepted once"
    );
}

/// **Regression guard (#1315).** `gmMissionClear` saves the abandon: one
/// `MissionUpdate` at not active, so an assign then a clear no longer leaves
/// an active row that returns at the next login.
#[tokio::test]
async fn gm_mission_clear_saves_the_abandon() {
    let mut mgr = mgr_with_player(1, "Castle");
    seed_mission_1001(&mut mgr);
    let (tx, mut rx) = mpsc::channel(32);
    assert!(
        dispatch(
            1,
            GM_MISSION_ASSIGN,
            &assign_args("1001", 1),
            &tx,
            &mut mgr,
            &test_engine()
        )
        .await
    );
    drain(&mut rx);

    let mut args = Vec::new();
    write_wstring_arg(&mut args, "1001");
    assert!(dispatch(1, GM_MISSION_CLEAR, &args, &tx, &mut mgr, &test_engine()).await);
    assert_eq!(
        mission_updates(&drain(&mut rx)),
        vec![(
            100,
            1001,
            cimmeria_entity::missions::MISSION_NOT_ACTIVE,
            None,
            vec![]
        )]
    );
}
