use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::*;
use crate::supervisor::events::fake_bridge::supervisor_with_meta;

/// What the fake bridge answers: a result with an optional meta stamp, or an error.
type Reply = Result<(Value, Option<Value>), String>;

#[tokio::test]
async fn no_hit_gives_none_and_sends_the_span_and_flags() {
    let seen: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    let sup = supervisor_with_meta(Arc::new(move |_: &str, p: &Value| -> Reply {
        log.lock().unwrap().push(p.clone());
        Ok((json!({ "hit": false, "time": 1.0 }), None))
    }))
    .await;
    let out = live_trace(
        &sup,
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        0x2086,
    )
    .await;
    assert_eq!(out, Ok(None));
    let params = seen.lock().unwrap();
    assert_eq!(params.len(), 1);
    assert_eq!(params[0]["start"], json!([0.0, 0.0, 0.0]));
    assert_eq!(params[0]["flags"].as_u64(), Some(0x2086));
}

#[tokio::test]
async fn a_hit_decodes_time_location_item_and_actor() {
    let sup = supervisor_with_meta(Arc::new(|_: &str, _: &Value| -> Reply {
        Ok((
            json!({
                "hit": true,
                "time": 0.5,
                "location": [1.0, 2.0, 3.0],
                "item": 7,
                "actor": 9,
            }),
            None,
        ))
    }))
    .await;
    let out = live_trace(
        &sup,
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        0x2086,
    )
    .await;
    assert_eq!(
        out,
        Ok(Some(TraceHit {
            time: 0.5,
            location: Vec3::new(1.0, 2.0, 3.0),
            item: 7,
            actor: 9,
        }))
    );
}

#[tokio::test]
async fn an_old_dll_without_trace_line_is_unsupported() {
    let sup = supervisor_with_meta(Arc::new(|_: &str, _: &Value| -> Reply {
        Err("method not found -32601".to_string())
    }))
    .await;
    let out = live_trace(
        &sup,
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        0x2086,
    )
    .await;
    assert_eq!(out, Err("trace_line unsupported".to_string()));
}

#[tokio::test]
async fn other_bridge_errors_keep_their_text() {
    let sup = supervisor_with_meta(Arc::new(|_: &str, _: &Value| -> Reply {
        Err("boom".to_string())
    }))
    .await;
    let out = live_trace(
        &sup,
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        0x2086,
    )
    .await;
    let err = out.expect_err("a bridge error must be an Err");
    assert!(err.starts_with("trace_line:"), "{err}");
    assert_ne!(err, "trace_line unsupported");
}
