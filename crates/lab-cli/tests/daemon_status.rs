//! The /status client against a stub server. Ports 'Get-LabStatus is null
//! when nothing listens' of the retired tools/lab/cli/test-common.ps1, plus the fetch
//! cases the packet lists.

mod support;

use std::time::{Duration, Instant};

use cimmeria_lab_cli::daemon::status::{self, Endpoint};
use support::{closed_port_endpoint, env_lock, EnvGuard, HangServer, StubServer, STUB_TOKEN};
use tempfile::TempDir;

/// What the real daemon sends (crates/lab/src/daemon/status.rs), two rows.
const FULL: &str = r#"{
  "daemon": {"pid": 4242, "version": "0.1.0", "started_at": "2026-10-11T01:02:03+00:00", "uptime_s": 7380},
  "instances": [
    {"instance": "default", "account": "lab1", "bridge_port": 8790, "client_pid": 12044,
     "lease": {"held": false, "lease": null, "recent": [], "rules": {"default_ttl_s": 600}}, "uat": null},
    {"instance": "p3", "account": null, "bridge_port": 8792, "client_pid": null,
     "lease": {"held": false}, "uat": {"current_row": "FS-P3"}}
  ]
}"#;

fn serve(status: u16, body: &str) -> (StubServer, Endpoint) {
    StubServer::start(vec![("GET", "/status", status, body)])
}

// fetch: 200 with the right shape -> Some
#[test]
fn fetch_returns_the_reply_for_the_daemons_shape() {
    let (_server, ep) = serve(200, FULL);
    let reply = status::fetch(&ep).expect("a reply");
    assert_eq!(reply.daemon.pid, 4242);
    assert_eq!(reply.daemon.version, "0.1.0");
    assert_eq!(
        reply.daemon.started_at.as_deref(),
        Some("2026-10-11T01:02:03+00:00")
    );
    assert_eq!(reply.daemon.uptime_s, Some(7380));
    assert_eq!(reply.instances.len(), 2);
    let a = &reply.instances[0];
    assert_eq!(a.instance, "default");
    assert_eq!(a.account.as_deref(), Some("lab1"));
    assert_eq!(a.bridge_port, Some(8790));
    assert_eq!(a.client_pid, Some(12044));
    assert_eq!(a.lease.as_ref().unwrap()["held"], false);
    assert!(a.uat.is_none());
    let b = &reply.instances[1];
    assert_eq!(b.account, None);
    assert_eq!(b.client_pid, None);
    assert_eq!(b.uat.as_ref().unwrap()["current_row"], "FS-P3");
}

// Fields the daemon may leave out read as None.
#[test]
fn fetch_accepts_omitted_optional_fields() {
    let body = r#"{"daemon":{"pid":1,"version":"x"},"instances":[{"instance":"a"}]}"#;
    let (_server, ep) = serve(200, body);
    let reply = status::fetch(&ep).expect("a reply");
    assert_eq!(reply.daemon.started_at, None);
    assert_eq!(reply.daemon.uptime_s, None);
    let row = &reply.instances[0];
    assert_eq!(row.instance, "a");
    assert!(row.account.is_none() && row.bridge_port.is_none() && row.client_pid.is_none());
    assert!(row.lease.is_none() && row.uat.is_none());
}

// fetch: 401 -> None
#[test]
fn fetch_is_none_on_401() {
    let (_server, ep) = serve(401, "{}");
    assert!(status::fetch(&ep).is_none());
}

// fetch: wrong shape -> None (each of these is an answer from something else)
#[test]
fn fetch_is_none_for_the_wrong_shape() {
    for body in [
        r#"{"hello":"world"}"#,
        r#"{"daemon":null,"instances":[]}"#,
        r#"{"daemon":{"pid":1,"version":"x"}}"#,
        r#"{"instances":[]}"#,
        r#"[1,2]"#,
        "<html>not json</html>",
    ] {
        let (_server, ep) = serve(200, body);
        assert!(status::fetch(&ep).is_none(), "body {body}");
    }
}

// fetch: the request carried Authorization: Bearer <token>
#[test]
fn fetch_sends_a_bearer_token_to_get_status() {
    let (server, ep) = serve(200, FULL);
    status::fetch(&ep).expect("a reply");
    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/status");
    assert_eq!(
        requests[0].header("authorization"),
        Some(format!("Bearer {STUB_TOKEN}").as_str())
    );
}

// New: no token means no request at all.
#[test]
fn fetch_without_a_token_makes_no_request() {
    let (server, mut ep) = serve(200, FULL);
    ep.token = None;
    assert!(status::fetch(&ep).is_none());
    assert!(server.requests().is_empty());
}

// 'Get-LabStatus is null when nothing listens'
#[test]
fn fetch_is_none_when_nothing_listens() {
    assert!(status::fetch(&closed_port_endpoint()).is_none());
}

// New: the token is not printed by Debug.
#[test]
fn endpoint_debug_hides_the_token() {
    let ep = Endpoint {
        base_url: "http://127.0.0.1:1".to_string(),
        token: Some("s3cr3t-token".to_string()),
    };
    let text = format!("{ep:?}");
    assert!(!text.contains("s3cr3t-token"), "{text}");
    assert!(text.contains("<redacted>"), "{text}");
}

// New (Get-LabStatus): the bind comes from labd.pid, else the default.
#[test]
fn discover_uses_the_bind_from_labd_pid() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    std::fs::write(
        tmp.path().join("labd.pid"),
        "{\"pid\":4242,\"bind\":\"127.0.0.1:9\"}",
    )
    .unwrap();
    let _env = EnvGuard::set(&[("CIMMERIA_LAB_HOME", Some(tmp.path().to_str().unwrap()))]);
    assert_eq!(Endpoint::discover().base_url, "http://127.0.0.1:9");
}

#[test]
fn discover_defaults_the_bind_without_a_pidfile() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[("CIMMERIA_LAB_HOME", Some(tmp.path().to_str().unwrap()))]);
    assert_eq!(Endpoint::discover().base_url, "http://127.0.0.1:8779");
}

// Off Windows the token is the process variable.
#[cfg(not(windows))]
#[test]
fn discover_carries_the_process_token() {
    let _lock = env_lock();
    let tmp = TempDir::new().unwrap();
    let _env = EnvGuard::set(&[
        ("CIMMERIA_LAB_HOME", Some(tmp.path().to_str().unwrap())),
        ("CIMMERIA_LAB_DAEMON_TOKEN", Some("test-token-not-real")),
    ]);
    assert_eq!(
        Endpoint::discover().token.as_deref(),
        Some("test-token-not-real")
    );
}

// Get-LabStatus -TimeoutSec 5: the constant is five seconds.
#[test]
fn fetch_timeout_is_five_seconds() {
    assert_eq!(status::FETCH_TIMEOUT, Duration::from_secs(5));
}

// The timeout really applies: a daemon that never answers is None, and soon.
#[test]
fn fetch_gives_up_on_a_daemon_that_never_answers() {
    let (_server, ep) = HangServer::start(false);
    let started = Instant::now();
    assert!(status::fetch_with_timeout(&ep, Duration::from_millis(300)).is_none());
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

// It covers the body too: headers and part of the body, then silence.
#[test]
fn fetch_gives_up_on_a_body_that_stalls() {
    let (_server, ep) = HangServer::start(true);
    let started = Instant::now();
    assert!(status::fetch_with_timeout(&ep, Duration::from_millis(300)).is_none());
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

// New: an empty token is no token.
#[test]
fn fetch_with_an_empty_token_makes_no_request() {
    let (server, mut ep) = serve(200, FULL);
    ep.token = Some(String::new());
    assert!(status::fetch(&ep).is_none());
    assert!(server.requests().is_empty());
}
