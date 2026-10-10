//! `client_drag_drop` against a fake client ([`cegui_fake`]): the pure
//! helpers, then the native sequence (cursor, button down, one step per
//! frame, drop decision, button up) and how each outcome is labelled.
//! `split` posts Ctrl to the game window, so it is live-only.

use std::sync::{Arc, Mutex};

use super::*;
use crate::supervisor::cegui_fake::{self, FakeCegui, CONTAINER, TARGET};
use crate::supervisor::events::fake_bridge;

#[test]
fn path_ends_on_the_target_in_even_steps() {
    let p = path((0, 0), (100, 50), 4);
    assert_eq!(p, vec![(25, 13), (50, 25), (75, 38), (100, 50)]);
    assert_eq!(path((5, 5), (9, 9), 0), vec![(9, 9)]);
}

#[test]
fn drag_state_reads_type_or_the_drag_icon() {
    assert!(!drag_active(
        &json!({ "drag_type": null, "drag_icon_visible": false })
    ));
    assert!(!drag_active(&json!({ "drag_type": 0 })));
    assert!(drag_active(
        &json!({ "drag_type": 1, "drag_icon_visible": false })
    ));
    assert!(drag_active(
        &json!({ "drag_type": null, "drag_icon_visible": true })
    ));
}

async fn drag(state: FakeCegui, steps: u32, allow_notify: bool) -> (Value, Arc<Mutex<FakeCegui>>) {
    let s = Arc::new(Mutex::new(state));
    let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
    let out = sup
        .drag_drop(
            &DragEnd::Window("SrcSlot".into()),
            &DragEnd::Window("DstSlot".into()),
            false,
            steps,
            allow_notify,
            Duration::ZERO,
        )
        .await
        .unwrap();
    (out, s)
}

/// The live client today: no drop target, so the tool fires the target's
/// Dropped itself, between the last move and the release, and labels the
/// drag N3.
#[tokio::test]
async fn a_null_drop_target_is_dropped_explicitly_and_labelled_n3() {
    let (out, s) = drag(FakeCegui::default(), 4, true).await;
    let s = s.lock().unwrap();
    let notify = format!("notify {TARGET:#x} {CONTAINER:#x}");
    assert_eq!(s.edges(), vec!["down 0", notify.as_str(), "up 0"]);
    // Cursor onto the source, then one move per step, ending on the target.
    let moves: Vec<&String> = s.calls.iter().filter(|c| c.starts_with("move")).collect();
    assert_eq!(moves.len(), 5);
    assert_eq!(moves[0], "move 10 10");
    assert_eq!(moves[4], "move 110 10");
    assert_eq!(out["drag_started"], true);
    assert_eq!(out["drag_started_at_step"], 1);
    assert_eq!(out["drop_target_resolved"], false);
    assert_eq!(out["drop_notified"], true);
    assert_eq!(out["moved"], true);
    assert_eq!(out["snap_back"], false);
    assert_eq!(out["native_level"], "native_call");
    assert_eq!(out["native_tier"], "N3");
    assert_eq!(out["native_pass"], false);
}

/// When CEGUI resolves the drop target, button-up drops by itself: no
/// explicit call, and the drag is a native pass.
#[tokio::test]
async fn a_resolved_drop_target_needs_no_explicit_drop() {
    let (out, s) = drag(
        FakeCegui {
            drop_target: TARGET,
            ..FakeCegui::default()
        },
        4,
        true,
    )
    .await;
    assert_eq!(s.lock().unwrap().edges(), vec!["down 0", "up 0"]);
    assert_eq!(out["drop_target_resolved"], true);
    assert_eq!(out["drop_notified"], false);
    assert_eq!(out["moved"], true);
    assert_eq!(out["native_level"], "native_cegui");
    assert_eq!(out["native_tier"], "N1");
    assert_eq!(out["native_pass"], true);
}

/// `allow_fallback: false` forbids the explicit drop: the drag runs, drops
/// nothing, and reports a snap-back at N1.
#[tokio::test]
async fn without_fallback_a_null_target_snaps_back() {
    let (out, s) = drag(FakeCegui::default(), 4, false).await;
    assert_eq!(s.lock().unwrap().edges(), vec!["down 0", "up 0"]);
    assert_eq!(out["drag_started"], true);
    assert_eq!(out["drop_notified"], false);
    assert_eq!(out["moved"], false);
    assert_eq!(out["snap_back"], true);
    assert_eq!(out["native_pass"], true);
}

/// A one-step drag only crosses the threshold, which never picks a
/// target: the tool walks at least `MIN_STEPS`.
#[tokio::test]
async fn a_drag_walks_at_least_the_minimum_steps() {
    let (out, s) = drag(FakeCegui::default(), 1, true).await;
    let moves = s
        .lock()
        .unwrap()
        .calls
        .iter()
        .filter(|c| c.starts_with("move"))
        .count();
    assert_eq!(moves, 1 + MIN_STEPS as usize);
    assert_eq!(out["steps"], MIN_STEPS);
}

/// A source that is not a DragContainer is refused before anything is
/// pressed or moved.
#[tokio::test]
async fn a_source_that_is_not_a_drag_container_is_refused_untouched() {
    let s = Arc::new(Mutex::new(FakeCegui {
        container_vtable: 0x01aa_0000,
        ..FakeCegui::default()
    }));
    let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
    let e = sup
        .drag_drop(
            &DragEnd::Window("SrcSlot".into()),
            &DragEnd::Window("DstSlot".into()),
            false,
            4,
            true,
            Duration::ZERO,
        )
        .await
        .unwrap_err();
    assert!(e.contains("SrcSlot is not a CEGUI DragContainer"), "{e}");
    assert!(s.lock().unwrap().calls.is_empty());
}
