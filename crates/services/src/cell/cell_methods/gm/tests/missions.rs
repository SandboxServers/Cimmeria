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
