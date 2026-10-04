//! The lease gate end to end, over the daemon's HTTP transport: what a
//! session sees when it calls a guarded tool with and without a lease, and
//! what a second session sees while the first holds it.

use std::collections::HashSet;

use serde_json::{json, Value};

use crate::daemon::http_tests::{spawn_daemon, test_server, McpSession};
use crate::lease::policy::{self, Gate};

fn error_text(resp: &Value) -> String {
    resp["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn lease_id(resp: &Value) -> String {
    let text = resp["result"]["content"][0]["text"].as_str().expect("text");
    let v: Value = serde_json::from_str(text).expect("grant json");
    v["lease_id"].as_str().expect("lease_id").to_string()
}

async fn acquire(s: &mut McpSession, owner: &str) -> Value {
    s.call(
        "lab_lease_acquire",
        json!({ "owner": owner, "purpose": "lease test" }),
    )
    .await
}

/// Regression guard: a guarded tool is refused without a lease, with the
/// message naming the fix; with the lease it passes the gate (and then fails
/// only because no client is attached).
#[tokio::test]
async fn a_guarded_tool_is_refused_without_a_lease() {
    let url = spawn_daemon(test_server()).await;
    let mut s = McpSession::open(&url).await;

    let r = s
        .call("client_lua_eval", json!({ "chunk": "return 1" }))
        .await;
    let e = error_text(&r);
    assert!(e.contains("needs a lease"), "{r}");
    assert!(e.contains("lab_lease_acquire"), "{r}");

    let id = lease_id(&acquire(&mut s, "session-a").await);
    let r = s
        .call(
            "client_lua_eval",
            json!({ "chunk": "return 1", "lease_id": id }),
        )
        .await;
    let e = error_text(&r);
    assert!(!e.contains("lease"), "the gate let it through: {r}");
    assert!(e.contains("bridge"), "it reached the (absent) bridge: {r}");

    // Read-only tools never need one.
    let r = s.call("lab_client_status", json!({})).await;
    assert!(r["result"].is_object(), "{r}");
}

/// Two sessions on one daemon: the second is refused while the first holds
/// the lease, sees who holds it, and can take it over with a reason.
#[tokio::test]
async fn a_second_session_is_refused_while_the_first_holds_the_lease() {
    let url = spawn_daemon(test_server()).await;
    let mut a = McpSession::open(&url).await;
    let mut b = McpSession::open(&url).await;
    let a_id = lease_id(&acquire(&mut a, "session-a").await);

    let r = acquire(&mut b, "session-b").await;
    let e = error_text(&r);
    assert!(e.contains("session-a") && e.contains("lease test"), "{r}");

    let status = b.call("lab_lease_status", json!({})).await;
    let text = status["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("session-a"), "{text}");
    assert!(!text.contains(&a_id), "status leaked the lease id");

    // b cannot drive with a's lease id guessed wrong...
    let r = b
        .call("client_input_release", json!({ "lease_id": "lease-guess" }))
        .await;
    assert!(error_text(&r).contains("not the current lease"), "{r}");

    // ...but can take over, and a is then refused.
    let r = b
        .call(
            "lab_lease_acquire",
            json!({ "owner": "session-b", "purpose": "takeover", "force": true, "reason": "a is idle" }),
        )
        .await;
    let b_id = lease_id(&r);
    let r = a
        .call("client_input_release", json!({ "lease_id": a_id }))
        .await;
    let e = error_text(&r);
    assert!(e.contains("taken over") && e.contains("session-b"), "{r}");
    let r = b
        .call("client_input_release", json!({ "lease_id": b_id }))
        .await;
    assert!(!error_text(&r).contains("lease"), "{r}");
}

/// Regression guard (review 2026-10-04): a force takeover in the middle of
/// an admitted flow stops its next action. `client_wait_event` polls the
/// bridge until its timeout; the fake bridge forces a takeover on the first
/// poll, and the flow must fail "lease revoked" at once and send nothing
/// more, instead of polling on to its 5 s timeout under a lease it lost.
#[tokio::test]
async fn a_takeover_mid_flow_stops_the_next_action() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use crate::lease::{AcquireRequest, LeaseBook};
    use crate::supervisor::events::fake_bridge::{
        self, empty_rings, events, is_ring_pump, lua_ok, Responder,
    };

    let book = Arc::new(LeaseBook::default());
    let calls = Arc::new(AtomicU32::new(0));
    let after_takeover = Arc::new(AtomicU32::new(0));
    let (b2, c2, a2) = (book.clone(), calls.clone(), after_takeover.clone());
    let responder: Responder = Arc::new(move |method, params| {
        if c2.fetch_add(1, Ordering::SeqCst) == 0 {
            b2.acquire(AcquireRequest {
                owner: "session-b".into(),
                purpose: "takeover".into(),
                force: true,
                reason: Some("mid-flow test".into()),
                ..Default::default()
            })
            .unwrap();
        } else {
            a2.fetch_add(1, Ordering::SeqCst);
        }
        Ok(match method {
            "events_read" => events(vec![]),
            "lua_eval" if is_ring_pump(params) => empty_rings(),
            _ => lua_ok(&["false"]),
        })
    });
    let sup = fake_bridge::supervisor(responder)
        .await
        .with_leases(book.clone());
    let url = spawn_daemon(crate::server::LabServer::new(Arc::new(sup))).await;
    let mut a = McpSession::open(&url).await;
    let a_id = lease_id(&acquire(&mut a, "session-a").await);

    let started = std::time::Instant::now();
    let r = a
        .call(
            "client_wait_event",
            json!({ "name": "never", "timeout_ms": 5000, "lease_id": a_id }),
        )
        .await;
    let e = error_text(&r);
    assert!(e.contains("lease revoked"), "{r}");
    assert!(e.contains("session-b"), "{r}");
    assert_eq!(
        after_takeover.load(Ordering::SeqCst),
        0,
        "no bridge call after the takeover"
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(4));
}

fn status_json(resp: &Value) -> Value {
    serde_json::from_str(resp["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

/// A run with no `lease_id` takes a lease of its own (owner `lab_uat_run`)
/// and releases it when it ends, here on a spec error.
#[tokio::test]
async fn a_uat_run_without_a_lease_takes_and_releases_its_own() {
    let url = spawn_daemon(test_server()).await;
    let mut s = McpSession::open(&url).await;
    let specs = tempfile::tempdir().unwrap();
    let _ = s
        .call(
            "lab_uat_run",
            json!({ "specs_dir": specs.path().display().to_string() }),
        )
        .await;
    let st = status_json(&s.call("lab_lease_status", json!({})).await);
    assert_eq!(st["held"], false, "{st}");
    assert_eq!(
        st["recent"][0]["owner"],
        crate::lease::run::UAT_RUN_OWNER,
        "{st}"
    );
    assert_eq!(st["recent"][0]["how"], "released", "{st}");
}

/// Regression guard: a run cannot start while another session holds the
/// lab; with the holder's own lease it runs and leaves the lease held; a
/// plan needs no lease at all.
#[tokio::test]
async fn a_uat_run_respects_the_lease() {
    let url = spawn_daemon(test_server()).await;
    let mut a = McpSession::open(&url).await;
    let mut b = McpSession::open(&url).await;
    let a_id = lease_id(&acquire(&mut a, "session-a").await);
    let specs = tempfile::tempdir().unwrap();
    let dir = specs.path().display().to_string();

    let r = b.call("lab_uat_run", json!({ "specs_dir": dir })).await;
    let e = error_text(&r);
    assert!(e.contains("session-a"), "{r}");

    let r = b
        .call(
            "lab_uat_run",
            json!({ "specs_dir": dir, "lease_id": "lease-guess" }),
        )
        .await;
    assert!(error_text(&r).contains("not the current lease"), "{r}");

    let r = b
        .call(
            "lab_uat_run",
            json!({ "specs_dir": dir, "plan_only": true }),
        )
        .await;
    assert!(!error_text(&r).contains("lease"), "{r}");

    let r = a
        .call("lab_uat_run", json!({ "specs_dir": dir, "lease_id": a_id }))
        .await;
    assert!(!error_text(&r).contains("lease"), "{r}");
    let st = status_json(&a.call("lab_lease_status", json!({})).await);
    assert_eq!(
        st["lease"]["owner"], "session-a",
        "a's lease survives the run"
    );
}

/// `tools/list` advertises `lease_id` as a required argument exactly on the
/// guarded tools.
#[tokio::test]
async fn guarded_tools_advertise_a_required_lease_id() {
    let url = spawn_daemon(test_server()).await;
    let mut s = McpSession::open(&url).await;
    let list = s.request("tools/list", json!({})).await;
    for t in list["result"]["tools"].as_array().unwrap() {
        let name = t["name"].as_str().unwrap();
        let required = t["inputSchema"]["required"]
            .as_array()
            .is_some_and(|r| r.iter().any(|v| v == "lease_id"));
        // Renew and release are open (they check their own id) but take
        // the lease id as their own argument.
        let guarded = policy::gate(name) == Gate::Lease
            || matches!(name, "lab_lease_renew" | "lab_lease_release");
        assert_eq!(required, guarded, "{name}");
        if guarded {
            assert_eq!(
                t["inputSchema"]["properties"]["lease_id"]["type"], "string",
                "{name}"
            );
        }
    }
}

/// Every routed tool is classified on purpose (open or leased), and no
/// classified name is stale. A new tool fails this until someone decides.
#[test]
fn every_routed_tool_is_classified() {
    let server = test_server();
    let routed: HashSet<String> = server
        .tool_router
        .list_all()
        .into_iter()
        .map(|t| t.name.to_string())
        .collect();
    let classified: HashSet<String> = policy::OPEN
        .iter()
        .chain(policy::LEASED)
        .chain(policy::OWN_LEASE)
        .map(|s| s.to_string())
        .collect();
    let unclassified: Vec<_> = routed.difference(&classified).collect();
    let stale: Vec<_> = classified.difference(&routed).collect();
    assert!(
        unclassified.is_empty(),
        "add to lease::policy OPEN or LEASED: {unclassified:?}"
    );
    assert!(stale.is_empty(), "not routed any more: {stale:?}");
}
