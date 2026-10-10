//! `client_drag_drop` against a fake client ([`cegui_fake`]): the pure
//! helpers, then the native sequence (cursor, button down, one step per
//! frame, drop decision, button up) and how each outcome is labelled.
//! A whole `split` drag needs the game window for its posted Ctrl, so it is
//! live-only; the Ctrl hold and release are tested on their own.

use std::sync::{Arc, Mutex};

use super::*;
use crate::lease::permit::{scope, Permit};
use crate::lease::{AcquireRequest, LeaseBook};
use crate::supervisor::cegui_fake::{self, FakeCegui, CONTAINER, TARGET};
use crate::supervisor::events::fake_bridge;

/// A window handle no window has: posting to it fails, as a dead client's
/// window does.
const NO_WINDOW: isize = 0x7ead_bee0;

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

fn change(container: &str, slot: i64, before: Value, after: Value) -> Value {
    json!({ "container": container, "slot": slot, "before": before, "after": after })
}

fn item(name: &str, qty: i64) -> Value {
    json!({ "item_id": 7, "name": name, "qty": qty })
}

fn key(c: &str, s: i64) -> SlotKey {
    (c.to_string(), s)
}

/// `moved` is about the two slots of the drag, not about any change.
#[test]
fn landed_needs_the_source_item_in_the_target_slot() {
    let (from, to) = (key("Main", 1), key("Main", 2));
    let moved = json!({ "slot_changes": [
        change("Main", 1, item("Zat", 1), Value::Null),
        change("Main", 2, Value::Null, item("Zat", 1)),
    ]});
    assert!(landed(&moved, &from, &to));
    // A swap and a split (the source keeps part of its stack) both land.
    let swap = json!({ "slot_changes": [
        change("Main", 1, item("Zat", 1), item("Medkit", 2)),
        change("Main", 2, item("Medkit", 2), item("Zat", 1)),
    ]});
    assert!(landed(&swap, &from, &to));
    let split = json!({ "slot_changes": [
        change("Main", 1, item("Ammo", 10), item("Ammo", 9)),
        change("Main", 2, Value::Null, item("Ammo", 1)),
    ]});
    assert!(landed(&split, &from, &to));
    // The item went somewhere else.
    let elsewhere = json!({ "slot_changes": [
        change("Main", 1, item("Zat", 1), Value::Null),
        change("Main", 3, Value::Null, item("Zat", 1)),
    ]});
    assert!(!landed(&elsewhere, &from, &to));
    // Some other item arrived in the target; the source never changed.
    let other = json!({ "slot_changes": [
        change("Main", 5, item("Zat", 1), Value::Null),
        change("Main", 2, Value::Null, item("Zat", 1)),
    ]});
    assert!(!landed(&other, &from, &to));
    // The same slot numbers in another container are not these slots.
    let other_bag = json!({ "slot_changes": [
        change("Mission", 1, item("Zat", 1), Value::Null),
        change("Mission", 2, Value::Null, item("Zat", 1)),
    ]});
    assert!(!landed(&other_bag, &from, &to));
}

async fn drag_between(
    state: FakeCegui,
    from: DragEnd,
    to: DragEnd,
    steps: u32,
    allow_notify: bool,
) -> (Result<Value, String>, Arc<Mutex<FakeCegui>>) {
    let s = Arc::new(Mutex::new(state));
    let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
    let out = sup
        .drag_drop(&from, &to, false, steps, allow_notify, Duration::ZERO)
        .await;
    (out, s)
}

async fn drag(state: FakeCegui, steps: u32, allow_notify: bool) -> (Value, Arc<Mutex<FakeCegui>>) {
    let (out, s) = drag_between(
        state,
        DragEnd::Window("SrcSlot".into()),
        DragEnd::Window("DstSlot".into()),
        steps,
        allow_notify,
    )
    .await;
    (out.unwrap(), s)
}

fn main_slot(slot: u32) -> DragEnd {
    DragEnd::Slot {
        container: ContainerRef::Id(1),
        slot,
    }
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
    assert_eq!(out["dragging_at_drop"], true);
    assert_eq!(out["drop_target_resolved"], false);
    assert_eq!(out["drop_notified"], true);
    assert_eq!(out["moved"], true);
    assert_eq!(out["moved_check"], "any_change");
    assert_eq!(out["effect_ok"], true);
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
    assert_eq!(out["effect_ok"], true);
    assert_eq!(out["native_level"], "native_cegui");
    assert_eq!(out["native_tier"], "N1");
    assert_eq!(out["native_pass"], true);
}

/// `allow_fallback: false` forbids the explicit drop: the drag runs, drops
/// nothing, and reports a snap-back. It drove at N1 but moved nothing, so
/// it is no native pass and `effect_ok` is false.
#[tokio::test]
async fn without_fallback_a_null_target_snaps_back_and_is_no_pass() {
    let (out, s) = drag(FakeCegui::default(), 4, false).await;
    assert_eq!(s.lock().unwrap().edges(), vec!["down 0", "up 0"]);
    assert_eq!(out["drag_started"], true);
    assert_eq!(out["drop_notified"], false);
    assert_eq!(out["moved"], false);
    assert_eq!(out["snap_back"], true);
    assert_eq!(out["native_tier"], "N1");
    assert_eq!(out["native_pass"], false);
    assert_eq!(out["effect_ok"], false);
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
    let (out, s) = drag_between(
        FakeCegui {
            container_vtable: 0x01aa_0000,
            ..FakeCegui::default()
        },
        DragEnd::Window("SrcSlot".into()),
        DragEnd::Window("DstSlot".into()),
        4,
        true,
    )
    .await;
    let e = out.unwrap_err();
    assert!(e.contains("SrcSlot is not a CEGUI DragContainer"), "{e}");
    assert!(s.lock().unwrap().calls.is_empty());
}

/// A press CEGUI did not take drags nothing of ours: refused with no drop
/// fired, and the button still goes up.
#[tokio::test]
async fn a_press_cegui_did_not_take_is_refused_and_released() {
    let (out, s) = drag_between(
        FakeCegui {
            down_handled: false,
            ..FakeCegui::default()
        },
        DragEnd::Window("SrcSlot".into()),
        DragEnd::Window("DstSlot".into()),
        4,
        true,
    )
    .await;
    let e = out.unwrap_err();
    assert!(e.contains("CEGUI did not take the press on SrcSlot"), "{e}");
    assert_eq!(s.lock().unwrap().edges(), vec!["down 0", "up 0"]);
}

/// Regression guard for the wrong-item drop: `getDragInfo` shows a drag
/// (of some other item) the whole time, but our container never drags.
/// Nothing is dropped, and the result says the drag never started.
#[tokio::test]
async fn another_items_drag_is_never_dropped() {
    let (out, s) = drag(
        FakeCegui {
            container_drags: false,
            other_drag: true,
            ..FakeCegui::default()
        },
        4,
        true,
    )
    .await;
    assert_eq!(s.lock().unwrap().edges(), vec!["down 0", "up 0"]);
    assert_eq!(out["drag_info_seen"], true);
    assert_eq!(out["drag_started"], false);
    assert_eq!(out["dragging_at_drop"], false);
    assert_eq!(out["drop_notified"], false);
    assert_eq!(out["moved"], false);
    assert_eq!(out["effect_ok"], false);
    assert_eq!(out["native_pass"], false);
}

/// A failure mid-drag (the second step's drag-state read) still lets go:
/// `up 0` is the last call, and no drop was fired.
#[tokio::test]
async fn the_button_is_released_after_a_mid_drag_failure() {
    let (out, s) = drag_between(
        FakeCegui {
            fail_drag_state_on: Some(2),
            ..FakeCegui::default()
        },
        DragEnd::Window("SrcSlot".into()),
        DragEnd::Window("DstSlot".into()),
        4,
        true,
    )
    .await;
    let e = out.unwrap_err();
    assert!(e.contains("scripted drag-state failure"), "{e}");
    let s = s.lock().unwrap();
    assert_eq!(s.calls.last().map(String::as_str), Some("up 0"));
    assert_eq!(s.edges(), vec!["down 0", "up 0"]);
}

/// Slot ends: `moved` checks the two slots, and the result names them.
#[tokio::test]
async fn slot_ends_check_the_item_reached_the_target_slot() {
    let (out, _) = drag_between(FakeCegui::default(), main_slot(1), main_slot(2), 4, true).await;
    let out = out.unwrap();
    assert_eq!(out["from"]["container"], "Main");
    assert_eq!(out["to"]["slot"], 2);
    assert_eq!(out["moved_check"], "slots");
    assert_eq!(out["moved"], true);
    assert_eq!(out["effect_ok"], true);
}

/// The item left the source but landed in another slot: the inventory
/// changed, the raw diff shows where, but the drag did not move it to its
/// target.
#[tokio::test]
async fn an_item_that_lands_elsewhere_is_not_moved() {
    let (out, _) = drag_between(
        FakeCegui {
            lands_in: 3,
            ..FakeCegui::default()
        },
        main_slot(1),
        main_slot(2),
        4,
        true,
    )
    .await;
    let out = out.unwrap();
    assert_eq!(out["moved_check"], "slots");
    assert_eq!(out["inventory_changed"], true);
    assert_eq!(out["moved"], false);
    assert_eq!(out["effect_ok"], false);
    assert_eq!(out["native_pass"], false);
    let slots: Vec<i64> = out["diff"]["slot_changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["slot"].as_i64().unwrap())
        .collect();
    assert_eq!(slots, vec![1, 3]);
}

/// Letting go of Ctrl clears the virtual modifier even when the key-up
/// cannot be posted, and reports the key-up error.
#[tokio::test]
async fn ctrl_release_clears_the_modifier_even_when_the_key_up_fails() {
    let s = Arc::new(Mutex::new(FakeCegui::default()));
    let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
    let e = sup.release_ctrl(NO_WINDOW).await.unwrap_err();
    assert!(e.contains("ctrl up"), "{e}");
    assert!(!e.contains("clearing input modifiers"), "{e}");
    assert_eq!(s.lock().unwrap().modifiers, vec![json!({})]);
}

/// A Ctrl press that cannot be posted takes the modifier back off before
/// it returns, so a failed split never leaves Ctrl held.
#[tokio::test]
async fn a_failed_ctrl_press_takes_the_modifier_back_off() {
    let s = Arc::new(Mutex::new(FakeCegui::default()));
    let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
    let e = sup.hold_ctrl(NO_WINDOW).await.unwrap_err();
    assert!(e.contains("ctrl down"), "{e}");
    assert_eq!(
        s.lock().unwrap().modifiers,
        vec![json!({ "ctrl": true }), json!({})]
    );
}

/// After the lease is taken over mid-drag, new input is refused, but the
/// button-up and the modifier clear still go through.
#[tokio::test]
async fn a_revoked_lease_still_lets_go_of_the_button_and_ctrl() {
    let s = Arc::new(Mutex::new(FakeCegui::default()));
    let sup = fake_bridge::supervisor(cegui_fake::responder(s.clone())).await;
    let book = Arc::new(LeaseBook::default());
    let req = |owner: &str, force: bool| AcquireRequest {
        owner: owner.into(),
        purpose: "drag test".into(),
        force,
        reason: force.then(|| "test takeover".into()),
        ..Default::default()
    };
    let a = book.acquire(req("a", false)).unwrap();
    let permit = Permit::Lease {
        book: book.clone(),
        id: a.lease_id,
    };
    scope(permit, async {
        let cegui = sup.cegui().await.unwrap();
        book.acquire(req("b", true)).unwrap();
        let e = cegui.button_down(LEFT_BUTTON).await.unwrap_err();
        assert!(e.contains("lease revoked"), "{e}");
        cegui.button_up(LEFT_BUTTON).await.unwrap();
        let e = sup.release_ctrl(NO_WINDOW).await.unwrap_err();
        assert!(!e.contains("clearing input modifiers"), "{e}");
    })
    .await;
    let s = s.lock().unwrap();
    assert_eq!(s.edges(), vec!["up 0"]);
    assert_eq!(s.modifiers, vec![json!({})]);
}
