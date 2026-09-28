//! #844: GM target resolution acts only on a stored target that is in the
//! caller's view (itself, or an entity in its witness set). Same-space alone
//! let a 19-minute-old target 216 m away steer a command.
//!
//! Fails without the `target_in_view` checks in `dispatch::resolve_target`
//! and `gm::query::subject_or_self`.

use super::*;

/// The GM's stored target is in its space but not in its view.
fn setup_out_of_view() -> (SpaceManager, u32, u32) {
    let (mut mgr, gm, npc) = setup();
    mgr.get_entity_mut(gm).unwrap().witnesses.clear();
    assert_eq!(
        mgr.get_entity(gm).unwrap().current_target_id,
        Some(npc as i32),
        "precondition: the target is still stored"
    );
    (mgr, gm, npc)
}

fn feedback(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<String> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let Some(text) = decode_feedback(&msg) {
            out.push(text);
        }
    }
    out
}

/// A target-typed command refuses a target outside the caller's view.
#[tokio::test]
async fn a_typed_command_refuses_a_target_out_of_view() {
    let (mut mgr, gm, npc) = setup_out_of_view();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".primarystats", &tx, &mut mgr, &ChainEngine::new()).await;
    let lines = feedback(&mut rx);
    assert!(
        lines
            .iter()
            .any(|l| l.contains(&format!("targeted entity {npc} is not in view"))),
        "got {lines:?}"
    );
}

/// The same command with the target in view goes through (control: the
/// refusal above is the view check, not something else in the fixture).
#[tokio::test]
async fn a_typed_command_accepts_a_target_in_view() {
    let (mut mgr, gm, _npc) = setup();
    let (tx, mut rx) = mpsc::channel(64);
    handle_console_command(gm, ".primarystats", &tx, &mut mgr, &ChainEngine::new()).await;
    let lines = feedback(&mut rx);
    assert!(
        !lines.iter().any(|l| l.contains("not in view")),
        "got {lines:?}"
    );
}

/// An out-of-view stored target is not used as an inspection subject:
/// `gmShowTargetLocation` falls back to the caller.
#[tokio::test]
async fn an_inspection_falls_back_to_self_for_a_target_out_of_view() {
    use crate::cell::console::gm::dispatch as gm_dispatch;
    use crate::cell::console::gm::GM_SHOW_TARGET_LOCATION;

    let (mut mgr, gm, npc) = setup_out_of_view();
    let (tx, mut rx) = mpsc::channel(16);
    gm_dispatch(
        gm,
        GM_SHOW_TARGET_LOCATION,
        &[],
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;
    let lines = feedback(&mut rx);
    assert!(
        lines.iter().any(|l| l.contains(&format!("[{gm}]"))),
        "must report the caller, got {lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l.contains(&format!("[{npc}]"))),
        "must not report the out-of-view target, got {lines:?}"
    );
}
