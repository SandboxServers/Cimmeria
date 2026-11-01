//! `lab_recover` recipes against the fake bridge. `recenter_camera` and
//! `unstick` read the world through memory reads the fake bridge does not
//! model, so their decisions are tested through the pure check helpers.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::{
    dialog_closed_check, input_released_check, moved_check, moved_m, pitch_level_check,
    server_moved_check,
};
use crate::proof::{EvVerdict, Why};
use crate::supervisor::events::fake_bridge::{self, Responder};
use crate::supervisor::world::geometry::Vec3;

type Calls = Arc<Mutex<Vec<(String, Value)>>>;

fn dialog_lines() -> Value {
    json!({ "ok": true, "results": ["dialog_title\tHello", "dialog_text\tBody"] })
}

fn no_dialog_lines() -> Value {
    json!({ "ok": true, "results": ["window\tHUD"] })
}

/// A bridge whose `input_status` is `status` (or an error) and whose
/// `lua_eval` answers come from `ui` in order (the last one repeats).
fn bridge(status: Result<Value, String>, ui: Vec<Value>) -> (Responder, Calls) {
    let calls: Calls = Arc::default();
    let log = calls.clone();
    let reads = Arc::new(Mutex::new(0usize));
    let r: Responder = Arc::new(move |m: &str, p: &Value| {
        log.lock().unwrap().push((m.to_string(), p.clone()));
        match m {
            "input_status" => status.clone(),
            "lua_eval" => {
                let mut n = reads.lock().unwrap();
                let i = (*n).min(ui.len().saturating_sub(1));
                *n += 1;
                Ok(ui.get(i).cloned().unwrap_or_else(no_dialog_lines))
            }
            "input_hold" => Ok(json!({ "posted": true, "held": [] })),
            _ => Ok(json!({})),
        }
    });
    (r, calls)
}

fn empty_status() -> Value {
    json!({ "held": { "keys": [], "buttons": [] }, "holds": [] })
}

fn verdict(out: &Value) -> (EvVerdict, Value) {
    let ev = &out["evidence"];
    let v: EvVerdict = serde_json::from_value(ev["verdict"].clone()).unwrap();
    (v, ev["why"].clone())
}

fn count(calls: &Calls, method: &str) -> usize {
    calls
        .lock()
        .unwrap()
        .iter()
        .filter(|(m, _)| m == method)
        .count()
}

/// The `down` presses of Escape (one `input_hold` with `down: true` each).
fn escapes(calls: &Calls) -> usize {
    calls
        .lock()
        .unwrap()
        .iter()
        .filter(|(m, p)| m == "input_hold" && p["down"] == true)
        .count()
}

#[tokio::test]
async fn release_input_confirms_on_empty_status() {
    let (r, calls) = bridge(Ok(empty_status()), vec![]);
    let sup = fake_bridge::supervisor(r).await;
    let out = sup.recover("release_input").await.unwrap();
    assert_eq!(verdict(&out).0, EvVerdict::Confirmed);
    assert_eq!(out["recipe"], "release_input");
    assert_eq!(count(&calls, "input_release"), 1);
}

#[tokio::test]
async fn release_input_with_a_stuck_key_is_denied() {
    let status = json!({ "held": { "keys": [87], "buttons": [] }, "holds": [] });
    let (r, _calls) = bridge(Ok(status), vec![]);
    let sup = fake_bridge::supervisor(r).await;
    let out = sup.recover("release_input").await.unwrap();
    assert_eq!(verdict(&out), (EvVerdict::Denied, json!("no_effect")));
}

#[tokio::test]
async fn close_dialogs_stops_when_closed() {
    let (r, calls) = bridge(Ok(empty_status()), vec![dialog_lines(), no_dialog_lines()]);
    let sup = fake_bridge::supervisor(r).await;
    let out = sup.recover_with("close_dialogs", Some(0)).await.unwrap();
    assert_eq!(verdict(&out).0, EvVerdict::Confirmed);
    assert_eq!(escapes(&calls), 2);
}

#[tokio::test]
async fn close_dialogs_gives_up_after_three() {
    let (r, calls) = bridge(Ok(empty_status()), vec![dialog_lines()]);
    let sup = fake_bridge::supervisor(r).await;
    let out = sup.recover_with("close_dialogs", Some(0)).await.unwrap();
    assert_eq!(verdict(&out), (EvVerdict::Denied, json!("no_effect")));
    assert_eq!(escapes(&calls), 3);
}

#[tokio::test]
async fn unknown_recipe_errors() {
    let (r, _calls) = bridge(Ok(empty_status()), vec![]);
    let sup = fake_bridge::supervisor(r).await;
    let e = sup.recover("dance").await.unwrap_err();
    assert_eq!(
        e,
        "unknown recipe dance; one of release_input, close_dialogs, recenter_camera, unstick"
    );
}

#[tokio::test]
async fn bridge_failure_is_inconclusive() {
    let (r, _calls) = bridge(Err("bridge down".into()), vec![]);
    let sup = fake_bridge::supervisor(r).await;
    let out = sup.recover("release_input").await.unwrap();
    assert_eq!(
        verdict(&out),
        (EvVerdict::Inconclusive, json!("no_post_read"))
    );
}

#[test]
fn input_released_cases() {
    assert_eq!(input_released_check(None).ok, None);
    // No `held` object at all is unmeasured, not a pass.
    assert_eq!(input_released_check(Some(&json!({}))).ok, None);
    assert_eq!(
        input_released_check(Some(&json!({ "held": {} }))).ok,
        Some(true)
    );
    let button = json!({ "held": { "keys": [], "buttons": [1] } });
    assert_eq!(input_released_check(Some(&button)).ok, Some(false));
    let hold = json!({ "held": { "keys": [], "buttons": [] }, "holds": [{ "vk": 87 }] });
    assert_eq!(input_released_check(Some(&hold)).ok, Some(false));
}

#[test]
fn dialog_closed_cases() {
    assert_eq!(dialog_closed_check(None).ok, None);
    assert_eq!(
        dialog_closed_check(Some(&json!({ "dialog": null }))).ok,
        Some(true)
    );
    assert_eq!(dialog_closed_check(Some(&json!({}))).ok, Some(true));
    assert_eq!(
        dialog_closed_check(Some(&json!({ "dialog": { "title": "x" } }))).ok,
        Some(false)
    );
    // A dialog section that threw reads `dialog: null`; that is not closed.
    let threw = json!({"dialog": null, "errors": ["dialog: boom"]});
    assert_eq!(dialog_closed_check(Some(&threw)).ok, None);
    let other = json!({"dialog": null, "errors": ["windows: boom"]});
    assert_eq!(dialog_closed_check(Some(&other)).ok, Some(true));
}

#[test]
fn jump_in_place_is_not_moved() {
    // A jump lifts the pawn along Z (UE3 units); that is not walking.
    let origin = Vec3::new(0.0, 0.0, 0.0);
    assert!(moved_m(origin, Vec3::new(0.0, 0.0, 150.0)) < 0.01);
    assert_eq!(moved_m(origin, Vec3::new(100.0, 0.0, 0.0)), 1.0);
}

#[test]
fn server_vertical_move_is_not_moved() {
    // Server Y is up: a rise of 2 m is not a horizontal move.
    let a = Vec3::new(0.0, 0.0, 0.0);
    let up = Vec3::new(0.0, 2.0, 0.0);
    assert_eq!(
        server_moved_check(Some(a), Some(up)).unwrap().ok,
        Some(false)
    );
}

#[test]
fn recenter_camera_decides_on_pitch() {
    assert_eq!(pitch_level_check(Some(4.9)).ok, Some(true));
    assert_eq!(pitch_level_check(Some(-5.1)).ok, Some(false));
    let none = pitch_level_check(None);
    assert_eq!((none.ok, none.why), (None, Some(Why::NoPostRead)));
}

#[test]
fn unstick_decides_on_distance() {
    assert_eq!(moved_check(Some(0.5)).ok, Some(true));
    assert_eq!(moved_check(Some(0.49)).ok, Some(false));
    assert_eq!(moved_check(None).ok, None);
    let a = Vec3::new(0.0, 0.0, 0.0);
    let far = Vec3::new(1.0, 0.0, 0.0);
    // Optional: never required, and absent unless the server answered twice.
    let c = server_moved_check(Some(a), Some(far)).unwrap();
    assert!(!c.required);
    assert_eq!(c.ok, Some(true));
    assert!(server_moved_check(Some(a), None).is_none());
}

#[test]
fn lab_recover_is_classified() {
    use crate::lease::policy;
    use crate::proof::claims::{self, Claim};
    let server = crate::daemon::http_tests::test_server();
    assert!(server
        .tool_router
        .list_all()
        .iter()
        .any(|t| t.name == "lab_recover"));
    assert!(policy::LEASED.contains(&"lab_recover"));
    assert_eq!(policy::gate("lab_recover"), policy::Gate::Lease);
    assert!(claims::JUDGED.contains(&"lab_recover"));
    assert_eq!(claims::claim("lab_recover"), Claim::Judged);
}
