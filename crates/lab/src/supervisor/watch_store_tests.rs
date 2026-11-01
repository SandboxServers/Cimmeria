use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};

use super::*;
use crate::proof::claims::{claim, Claim};
use crate::supervisor::events::fake_bridge::supervisor_with_meta;

fn sample(i: u64, ms: u64) -> WatchSample {
    WatchSample {
        i,
        f: i,
        ms,
        wall_ms: 0,
        t: None,
        pos: None,
        rot: None,
        hover: None,
        mode: None,
        buttons: None,
        actors: None,
    }
}

fn wire(i: u64, ms: u64) -> Value {
    json!({ "i": i, "f": i, "ms": ms })
}

#[test]
fn ingest_maps_mono_to_wall() {
    let mut store = WatchStore::default();
    let kept = store.ingest(vec![sample(0, 1000), sample(1, 1500)], 50_000, Some(2000));
    assert_eq!(kept, 2);
    let walls: Vec<i64> = store.samples.iter().map(|s| s.wall_ms).collect();
    assert_eq!(walls, vec![49_000, 49_500]);
}

#[test]
fn ingest_without_meta_uses_read_time() {
    let mut store = WatchStore::default();
    store.ingest(vec![sample(0, 1000), sample(1, 1500)], 50_000, None);
    let walls: Vec<i64> = store.samples.iter().map(|s| s.wall_ms).collect();
    assert_eq!(walls, vec![50_000, 50_000]);
}

#[test]
fn ingest_skips_seen_indexes() {
    let mut store = WatchStore::default();
    let first: Vec<WatchSample> = (0..=4).map(|i| sample(i, i)).collect();
    assert_eq!(store.ingest(first, 0, None), 5);
    let second: Vec<WatchSample> = (3..=7).map(|i| sample(i, i)).collect();
    assert_eq!(store.ingest(second, 0, None), 3);
    assert_eq!(store.samples.len(), 8);
}

#[test]
fn store_caps_at_4096() {
    let mut store = WatchStore::default();
    let all: Vec<WatchSample> = (0..5000).map(|i| sample(i, i)).collect();
    store.ingest(all, 0, None);
    assert_eq!(store.samples.len(), STORE_CAP);
    assert_eq!(store.samples.front().map(|s| s.i), Some(904));
}

#[tokio::test]
async fn read_pulls_and_reports_pulled() {
    let sup = supervisor_with_meta(Arc::new(|method: &str, _p: &Value| {
        if method != "watch_read" {
            return Err("unexpected".to_string());
        }
        let result = json!({
            "samples": [wire(0, 1000), wire(1, 1500), wire(2, 1800)],
            "next": 3, "dropped": 0, "running": true
        });
        Ok((result, Some(json!({ "frame": 5, "mono_ms": 2000 }))))
    }))
    .await;
    let out = sup.watch("read", json!({})).await.unwrap();
    assert_eq!(out["samples"].as_array().map(Vec::len), Some(3));
    assert_eq!(out["pulled"], json!(3));
    let (running, samples) = sup.watch_samples_since(0).await;
    assert!(running);
    assert_eq!(samples.len(), 3);
    assert!(samples[0].wall_ms < samples[2].wall_ms);
}

#[tokio::test]
async fn start_clears_the_store() {
    let sup = supervisor_with_meta(Arc::new(|method: &str, _p: &Value| match method {
        "watch_read" => {
            let result = json!({
                "samples": [wire(0, 1000), wire(1, 1500), wire(2, 1800)],
                "next": 3, "dropped": 0, "running": false
            });
            Ok((result, None))
        }
        "watch_start" => Ok((json!({ "started_frame": 1, "replaced": false }), None)),
        _ => Err("unexpected".to_string()),
    }))
    .await;
    sup.watch("read", json!({})).await.unwrap();
    assert_eq!(sup.watch_samples_since(0).await.1.len(), 3);
    sup.watch("start", json!({})).await.unwrap();
    let (running, samples) = sup.watch_samples_since(0).await;
    assert!(running);
    assert!(samples.is_empty());
}

#[tokio::test]
async fn old_dll_gets_the_c15_error() {
    let sup = supervisor_with_meta(Arc::new(|_: &str, _: &Value| {
        Err("method not found -32601".to_string())
    }))
    .await;
    let err = sup.watch("start", json!({})).await.unwrap_err();
    assert_eq!(err, "watch needs a lab DLL with watch_start (AX-24)");
}

#[tokio::test]
async fn bad_action_is_refused() {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let sup = supervisor_with_meta(Arc::new(move |_: &str, _: &Value| {
        counter.fetch_add(1, Ordering::SeqCst);
        Ok((json!({}), None))
    }))
    .await;
    let err = sup.watch("pause", json!({})).await.unwrap_err();
    assert_eq!(err, "action must be start, read or stop");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn client_watch_is_classified() {
    assert_eq!(claim("client_watch"), Claim::NoClaim);
    assert!(crate::lease::policy::LEASED.contains(&"client_watch"));
}
