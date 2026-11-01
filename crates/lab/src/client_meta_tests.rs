use std::sync::Arc;

use serde_json::json;

use crate::client::BridgeMeta;
use crate::supervisor::events::fake_bridge::{client_with_meta, MetaResponder};

fn responder() -> MetaResponder {
    Arc::new(|method, _| match method {
        "stamped" => Ok((
            json!({ "n": 1 }),
            Some(json!({ "frame": 81234, "mono_ms": 5 })),
        )),
        "plain" => Ok((json!({ "n": 2 }), None)),
        _ => Err("no".into()),
    })
}

#[tokio::test]
async fn call_meta_reads_meta_and_tolerates_absence() {
    let c = client_with_meta(responder()).await;
    let (v, m) = c.call_meta("stamped", json!({})).await.unwrap();
    assert_eq!(v, json!({ "n": 1 }));
    assert_eq!(
        m,
        Some(BridgeMeta {
            frame: 81234,
            mono_ms: 5,
            game_t: None
        })
    );
    let (v, m) = c.call_meta("plain", json!({})).await.unwrap();
    assert_eq!((v, m), (json!({ "n": 2 }), None));
}

#[tokio::test]
async fn call_still_returns_result_only() {
    let c = client_with_meta(responder()).await;
    let v = c.call("stamped", json!({})).await.unwrap();
    assert_eq!(v, json!({ "n": 1 }));
}
