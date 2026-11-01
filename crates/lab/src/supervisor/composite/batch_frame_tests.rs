use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::*;
use crate::supervisor::composite::batch::parse_steps;
use crate::supervisor::events::fake_bridge::{self, lua_ok, Responder};

fn steps(raw: Vec<Value>) -> Vec<Step> {
    parse_steps(&raw).expect("steps parse")
}

/// A fake bridge that records each method it is asked, answered by `answer`.
async fn recording(
    answer: fn(&str, &Value) -> Result<Value, String>,
) -> (Supervisor, Arc<Mutex<Vec<String>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    let responder: Responder = Arc::new(move |m: &str, p: &Value| {
        log.lock().unwrap().push(m.to_string());
        answer(m, p)
    });
    (fake_bridge::supervisor(responder).await, seen)
}

fn calls(seen: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    seen.lock().unwrap().clone()
}

#[test]
fn literal_reads_are_batchable() {
    let lit = steps(vec![
        json!({ "op": "mem_read", "addr": "0x1000" }),
        json!({ "op": "lua", "chunk": "return 7" }),
    ]);
    assert!(dll_batchable(&lit));
    let refs = steps(vec![
        json!({ "id": "a", "op": "mem_read", "addr": "0x1000" }),
        json!({ "op": "mem_read", "addr": "$a" }),
    ]);
    assert!(!dll_batchable(&refs), "a reference takes the step path");
    assert!(!dll_batchable(&steps(vec![
        json!({ "op": "wait", "ms": 5 })
    ])));
    let many: Vec<Value> = (0..17)
        .map(|_| json!({ "op": "mem_read", "addr": "0x1000" }))
        .collect();
    assert!(!dll_batchable(&steps(many)), "17 steps is over the DLL cap");
    assert!(!dll_batchable(&[]));
}

#[tokio::test]
async fn batch_goes_to_the_dll_once() {
    let (s, seen) = recording(|m: &str, _: &Value| match m {
        "batch" => Ok(json!({
            "results": [
                { "ok": { "hex": "2a000000" } },
                { "ok": { "ok": true, "results": ["7"] } },
            ],
            "frame": 900,
        })),
        other => Err(format!("unexpected {other}")),
    })
    .await;
    let out = s
        .batch(
            &steps(vec![
                json!({ "id": "a", "op": "mem_read", "addr": "0x1000", "as": "u32" }),
                json!({ "id": "b", "op": "lua", "chunk": "return 7" }),
            ]),
            true,
        )
        .await;
    assert_eq!(out.values["a"], json!(42));
    assert_eq!(out.values["b"], json!("7"));
    assert_eq!(out.frame, Some(900));
    assert_eq!(out.to_json()["frame"], json!(900));
    assert_eq!(calls(&seen), ["batch"]);
}

#[tokio::test]
async fn old_dll_falls_back_and_remembers() {
    let (s, seen) = recording(|m: &str, _: &Value| match m {
        "batch" => Err("method not found (-32601)".to_string()),
        "mem_read" => Ok(json!({ "hex": "2a000000" })),
        "lua_eval" => Ok(lua_ok(&["7"])),
        other => Err(format!("unexpected {other}")),
    })
    .await;
    let raw = vec![
        json!({ "id": "a", "op": "mem_read", "addr": "0x1000", "as": "u32" }),
        json!({ "id": "b", "op": "lua", "chunk": "return 7" }),
    ];
    let out = s.batch(&steps(raw.clone()), false).await;
    assert_eq!(out.values["a"], json!(42));
    assert_eq!(out.values["b"], json!("7"));
    assert_eq!(out.frame, None, "the step path has no frame");
    assert_eq!(calls(&seen), ["batch", "mem_read", "lua_eval"]);
    // The flag is set: the next batch goes straight to the step path.
    s.batch(&steps(raw), false).await;
    assert_eq!(
        calls(&seen),
        ["batch", "mem_read", "lua_eval", "mem_read", "lua_eval"]
    );
}

#[tokio::test]
async fn short_dll_answer_falls_back() {
    let (s, seen) = recording(|m: &str, _: &Value| match m {
        "batch" => Ok(json!({ "results": [], "frame": 3 })),
        "mem_read" => Ok(json!({ "hex": "2a000000" })),
        other => Err(format!("unexpected {other}")),
    })
    .await;
    let raw = vec![json!({ "id": "a", "op": "mem_read", "addr": "0x1000", "as": "u32" })];
    let out = s.batch(&steps(raw), false).await;
    assert_eq!(out.values["a"], json!(42));
    assert_eq!(out.frame, None, "a short DLL answer is not a frame");
    assert_eq!(calls(&seen), ["batch", "mem_read"]);
}

#[tokio::test]
async fn refs_use_the_step_path() {
    let (s, seen) = recording(|m: &str, _: &Value| match m {
        "mem_read" => Ok(json!({ "hex": "2a000000" })),
        other => Err(format!("unexpected {other}")),
    })
    .await;
    let out = s
        .batch(
            &steps(vec![
                json!({ "id": "a", "op": "mem_read", "addr": "0x1000", "as": "u32" }),
                json!({ "id": "b", "op": "mem_read", "addr": "$a", "as": "u32" }),
            ]),
            false,
        )
        .await;
    assert_eq!(out.values["b"], json!(42));
    assert_eq!(calls(&seen), ["mem_read", "mem_read"], "no DLL batch");
}

#[tokio::test]
async fn per_step_error_is_reported() {
    let (s, _) = recording(|m: &str, _: &Value| match m {
        "batch" => Ok(json!({
            "results": [
                { "error": { "code": -32000, "message": "bad addr" } },
                { "ok": { "ok": true, "results": ["1"] } },
            ],
            "frame": 5,
        })),
        other => Err(format!("unexpected {other}")),
    })
    .await;
    let out = s
        .batch(
            &steps(vec![
                json!({ "id": "a", "op": "mem_read", "addr": "0x1" }),
                json!({ "id": "b", "op": "lua", "chunk": "return 1" }),
            ]),
            false,
        )
        .await;
    assert_eq!(out.values["a"], json!({ "error": "bad addr" }));
    assert_eq!(out.values["b"], json!("1"));
    assert_eq!(out.frame, Some(5));
}
