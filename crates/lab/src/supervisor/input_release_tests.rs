//! `input_key` as DLL holds and the one release path, against the fake
//! bridge. The window handle is passed in (`*_at` seams).

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::supervisor::events::fake_bridge::{self, Responder};

type Calls = Arc<Mutex<Vec<(String, Value)>>>;

fn recording(
    hold_reply: impl Fn() -> Result<Value, String> + Send + Sync + 'static,
    release_fails: bool,
) -> (Responder, Calls) {
    let calls: Calls = Arc::default();
    let log = calls.clone();
    let r: Responder = Arc::new(move |m: &str, p: &Value| {
        log.lock().unwrap().push((m.to_string(), p.clone()));
        match m {
            "input_hold" => hold_reply(),
            "input_release" if release_fails => Err("boom".into()),
            _ => Ok(json!({})),
        }
    });
    (r, calls)
}

fn ok_hold() -> Result<Value, String> {
    Ok(json!({ "posted": true, "held": [] }))
}

fn methods(calls: &Calls) -> Vec<(String, Value)> {
    calls.lock().unwrap().clone()
}

#[tokio::test]
async fn input_key_down_is_a_dll_hold() {
    let (r, calls) = recording(ok_hold, false);
    let sup = fake_bridge::supervisor(r).await;
    sup.input_key_at(0x1234, "W", "down", None).await.unwrap();
    let sent = methods(&calls);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "input_hold");
    assert_eq!(sent[0].1["down"], true);
    assert!(!sup.holds.lock().await.is_empty());
}

#[tokio::test]
async fn input_key_tap_holds_then_lets_go() {
    let (r, calls) = recording(ok_hold, false);
    let sup = fake_bridge::supervisor(r).await;
    sup.input_key_at(0x1234, "W", "tap", Some(1)).await.unwrap();
    let sent = methods(&calls);
    assert_eq!(sent.len(), 2);
    assert!(sent.iter().all(|(m, _)| m == "input_hold"));
    assert_eq!(sent[0].1["down"], true);
    assert_eq!(sent[1].1["down"], false);
    assert!(sup.holds.lock().await.is_empty());
}

#[tokio::test]
async fn old_dll_key_still_posts() {
    let (r, calls) = recording(|| Err("method not found -32601".into()), false);
    let sup = fake_bridge::supervisor(r).await;
    // The fallback posts a real window message to a fake handle, which may
    // fail; the point is that the hold attempt fell back and stuck there.
    let _ = sup.input_key_at(0, "W", "down", None).await;
    assert!(sup.holds.lock().await.is_fallback());
    let _ = sup.input_key_at(0, "W", "up", None).await;
    assert_eq!(
        methods(&calls).len(),
        1,
        "fallback must not call input_hold"
    );
}

#[tokio::test]
async fn release_all_input_clears_the_book() {
    let (r, calls) = recording(ok_hold, false);
    let sup = fake_bridge::supervisor(r).await;
    sup.input_key_at(7, "W", "down", None).await.unwrap();
    sup.release_all_input("t").await.unwrap();
    assert!(methods(&calls).iter().any(|(m, _)| m == "input_release"));
    assert!(sup.holds.lock().await.is_empty());
}

#[tokio::test]
async fn release_all_input_clears_even_on_error() {
    let (r, _calls) = recording(ok_hold, true);
    let sup = fake_bridge::supervisor(r).await;
    sup.input_key_at(7, "W", "down", None).await.unwrap();
    assert!(sup.release_all_input("t").await.is_err());
    assert!(sup.holds.lock().await.is_empty());
}
