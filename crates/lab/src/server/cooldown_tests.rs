//! Tests for the quick-fail cooldown (C9); the clock is injected.

use rmcp::{model::*, ErrorData as McpError};
use serde_json::json;

use super::{exempt, failed, gate, key, note, Cooldown};
use crate::daemon::http_tests::{spawn_daemon, test_server, McpSession};

#[test]
fn backoff_doubles_and_caps() {
    let mut c = Cooldown::default();
    let k = key("client_world_click", &json!({}));
    let mut now = 0;
    for window in [500u64, 1000, 2000, 4000, 8000, 8000] {
        c.record(k, true, now);
        let r = c.check(k, now).expect("backoff running");
        assert_eq!(r["retry_after_ms"], window);
        // One ms before the window ends is still refused; the end is not.
        assert!(c.check(k, now + window - 1).is_some());
        assert!(c.check(k, now + window).is_none());
        now += window;
    }
}

#[test]
fn refusal_shape() {
    let mut c = Cooldown::default();
    let k = key("t", &json!({}));
    c.record(k, true, 100);
    let r = c.check(k, 300).unwrap();
    assert_eq!(r["error"], "cooling down");
    assert_eq!(r["why"], "cooling_down");
    assert_eq!(r["hint"], crate::proof::Why::CoolingDown.hint());
    assert_eq!(r["retry_after_ms"], 300);
    assert_eq!(r["failures"], 1);
}

#[test]
fn success_clears() {
    let mut c = Cooldown::default();
    let k = key("t", &json!({}));
    c.record(k, true, 0);
    c.record(k, false, 10);
    assert!(c.check(k, 20).is_none());
}

#[test]
fn lease_and_verbose_do_not_change_the_key() {
    let base = key("t", &json!({"entity_id": 5}));
    let noisy =
        json!({"entity_id": 5, "lease_id": "x", "instance": "b", "verbose": true, "fields": ["a"]});
    assert_eq!(base, key("t", &noisy));
    assert_ne!(base, key("t", &json!({"entity_id": 6})));
    assert_ne!(base, key("u", &json!({"entity_id": 5})));
}

#[test]
fn key_order_does_not_change_the_key() {
    assert_eq!(
        key("t", &json!({"a": 1, "b": {"x": 1, "y": 2}})),
        key("t", &json!({"b": {"y": 2, "x": 1}, "a": 1}))
    );
}

#[test]
fn idle_keys_are_forgotten() {
    let mut c = Cooldown::default();
    let k = key("t", &json!({}));
    c.record(k, true, 0);
    assert!(c.check(k, 61_000).is_none());
    assert!(c.keys.is_empty() && c.order.is_empty());
}

#[test]
fn oldest_key_is_evicted_past_the_cap() {
    let mut c = Cooldown::default();
    for i in 0..257u64 {
        c.record(i, true, i);
    }
    assert_eq!(c.keys.len(), 256);
    assert!(!c.keys.contains_key(&0));
}

#[test]
fn exempt_tools() {
    for t in [
        "lab_lease_acquire",
        "client_ui_state",
        "lab_uat_run",
        "lab_recover",
        "lab_client_start",
    ] {
        assert!(exempt(t), "{t}");
    }
    assert!(!exempt("client_world_click"));
}

#[test]
fn denied_evidence_counts_as_failure() {
    let denied = json!({"evidence": {"verdict": "denied"}}).to_string();
    let ok = json!({"evidence": {"verdict": "confirmed"}}).to_string();
    let wrap = |t: String| -> Result<CallToolResponse, McpError> {
        Ok(CallToolResponse::Complete(CallToolResult::success(vec![
            ContentBlock::text(t),
        ])))
    };
    assert!(failed(&wrap(denied)));
    assert!(!failed(&wrap(ok)));
    assert!(failed(&Ok(CallToolResponse::Complete(
        CallToolResult::error(vec![ContentBlock::text("x")])
    ))));
    assert!(failed(&Err(McpError::internal_error("boom", None))));
}

#[test]
fn same_call_on_another_instance_is_not_cooled() {
    // A tool and argument value no other test uses: the state is process-wide.
    let args = json!({ "entity_id": 90_036 });
    let args = args.as_object().unwrap();
    let failed_response = Ok(CallToolResponse::Complete(CallToolResult::error(vec![
        ContentBlock::text("denied"),
    ])));
    let (k_a, _) = gate("a", "client_world_click", Some(args)).expect("keyed");
    note(k_a, &failed_response);
    let (_, refused_a) = gate("a", "client_world_click", Some(args)).expect("keyed");
    let (_, refused_b) = gate("b", "client_world_click", Some(args)).expect("keyed");
    assert!(refused_a.is_some(), "instance a is cooling down");
    assert!(
        refused_b.is_none(),
        "instance b is not cooled by a's failure"
    );
}

#[tokio::test]
async fn handler_refuses_a_repeat_of_a_failed_call() {
    let url = spawn_daemon(test_server()).await;
    let mut s = McpSession::open(&url).await;
    let grant = s
        .call(
            "lab_lease_acquire",
            json!({ "owner": "cooldown-test", "purpose": "cooldown test" }),
        )
        .await;
    let text = grant["result"]["content"][0]["text"]
        .as_str()
        .expect("grant text");
    let id = serde_json::from_str::<serde_json::Value>(text).unwrap()["lease_id"]
        .as_str()
        .expect("lease_id")
        .to_string();
    // A chunk no other test uses: the cooldown state is process-wide.
    let args = json!({ "chunk": "return 'ax36-cooldown'", "lease_id": id });
    let first = s.call("client_lua_eval", args.clone()).await;
    let msg = first["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("bridge"),
        "the first call reaches the absent bridge: {first}"
    );
    let second = s.call("client_lua_eval", args).await;
    assert_eq!(second["result"]["isError"], true, "{second}");
    let body = second["result"]["content"][0]["text"]
        .as_str()
        .expect("refusal text");
    let v: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(v["why"], "cooling_down", "{second}");
}
