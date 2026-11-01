//! `hold_key` and friends against the fake bridge. The window handle is
//! passed in (`*_at` seams): the fake has no game window.

use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::*;
use crate::supervisor::events::fake_bridge::{self, Responder};

type Calls = Arc<Mutex<Vec<(String, Value)>>>;

fn recording(
    reply: impl Fn() -> Result<Value, String> + Send + Sync + 'static,
) -> (Responder, Calls) {
    let calls: Calls = Arc::default();
    let log = calls.clone();
    let r: Responder = Arc::new(move |m: &str, p: &Value| {
        if m == "input_hold" {
            log.lock().unwrap().push((m.to_string(), p.clone()));
            return reply();
        }
        Ok(json!({}))
    });
    (r, calls)
}

fn holds(calls: &Calls) -> Vec<Value> {
    calls
        .lock()
        .unwrap()
        .iter()
        .map(|(_, p)| p.clone())
        .collect()
}

fn w() -> keys::Key {
    keys::named("w").unwrap()
}

#[tokio::test]
async fn hold_key_uses_input_hold() {
    let (r, calls) = recording(|| Ok(json!({ "posted": true, "held": [] })));
    let sup = fake_bridge::supervisor(r).await;
    sup.hold_key_at(0x1234, "w", w(), true).await.unwrap();
    let sent = holds(&calls);
    assert_eq!(sent.len(), 1);
    let p = &sent[0];
    assert_eq!(p["down"], true);
    assert_eq!(p["lease_ms"], 10_000);
    assert_eq!(p["vk"], w().vk);
    assert_eq!(p["hwnd"], 0x1234);
    assert_eq!(p["lparam_down"], w().lparam_down() as u32);
    assert_eq!(p["lparam_up"], w().lparam_up() as u32);
    assert!(!sup.holds.lock().await.is_empty());
}

#[tokio::test]
async fn old_dll_falls_back_to_posts() {
    let (r, calls) = recording(|| Err("method not found -32601".into()));
    let sup = fake_bridge::supervisor(r).await;
    // The retry posts a real window message to a fake handle; it may fail,
    // which is not what this test is about.
    let _ = sup.hold_key_at(0, "w", w(), true).await;
    assert!(sup.holds.lock().await.is_fallback());
    assert_eq!(holds(&calls).len(), 1);
    let _ = sup.hold_key_at(0, "w", w(), true).await;
    assert_eq!(holds(&calls).len(), 1, "fallback must not call input_hold");
}

#[tokio::test]
async fn renew_logs_reassert() {
    let (r, calls) = recording(|| Ok(json!({ "posted": true, "held": [] })));
    let sup = fake_bridge::supervisor(r).await;
    {
        let mut b = sup.holds.lock().await;
        b.set_lease_ms(0);
        b.pressed(w().vk, "w", 0);
    }
    // `renew_holds_at` emits `lab.hold.reasserted` once per posted renewal
    // and returns that count, which is what we can observe deterministically.
    assert_eq!(sup.renew_holds_at(7).await, 1);
    let sent = holds(&calls);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0]["down"], true);
}

#[tokio::test]
async fn release_all_sends_ups() {
    let (r, calls) = recording(|| Ok(json!({ "posted": true, "held": [] })));
    let sup = fake_bridge::supervisor(r).await;
    sup.hold_key_at(7, "w", w(), true).await.unwrap();
    sup.hold_key_at(7, "a", keys::named("a").unwrap(), true)
        .await
        .unwrap();
    calls.lock().unwrap().clear();
    sup.release_all_holds_at(7).await;
    let sent = holds(&calls);
    assert_eq!(sent.len(), 2);
    assert!(sent.iter().all(|p| p["down"] == false));
    assert!(sup.holds.lock().await.is_empty());
}
