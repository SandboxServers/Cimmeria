//! LX-05: the MCP client against a private stub of the daemon's endpoint.
//! Ports the `Hide-LeaseIds` check of `tools/lab/cli/test-uat.ps1`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_lab_cli::daemon::mcp::{
    hide_lease_ids, wait_lease_free_at_every, wait_lease_free_with, McpClient, McpError,
};
use serde_json::{json, Value};

const TOKEN: &str = "tok-123";
const T: Duration = Duration::from_secs(5);

// ---- the stub: one request per connection, replies recorded, routes matched in order ----

#[derive(Clone)]
struct Route {
    method: &'static str,
    path: &'static str,
    /// The request body must contain this text.
    body_has: Option<&'static str>,
    status: u16,
    headers: Vec<(&'static str, &'static str)>,
    body: String,
    delay_ms: u64,
}

fn route(
    method: &'static str,
    path: &'static str,
    body_has: Option<&'static str>,
    status: u16,
    body: &str,
) -> Route {
    Route {
        method,
        path,
        body_has,
        status,
        headers: vec![],
        body: body.to_owned(),
        delay_ms: 0,
    }
}

#[derive(Clone, Debug)]
struct Recorded {
    method: String,
    path: String,
    headers: Vec<(String, String)>, // names lower-cased
    body: String,
}

impl Recorded {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

struct Stub {
    addr: SocketAddr,
    seen: Arc<Mutex<Vec<Recorded>>>,
    stop: Arc<AtomicBool>,
}

impl Stub {
    fn start(routes: Vec<Route>) -> Stub {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the stub");
        let addr = listener.local_addr().expect("stub address");
        let routes = Arc::new(routes);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (r, s, st) = (routes.clone(), seen.clone(), stop.clone());
        std::thread::spawn(move || {
            for conn in listener.incoming() {
                if st.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = conn else { continue };
                let (r, s) = (r.clone(), s.clone());
                std::thread::spawn(move || serve(stream, &r, &s));
            }
        });
        Stub { addr, seen, stop }
    }
    fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }
    fn requests(&self) -> Vec<Recorded> {
        self.seen.lock().unwrap().clone()
    }
    /// The recorded POST whose body contains `needle`.
    fn post_with(&self, needle: &str) -> Recorded {
        self.requests()
            .into_iter()
            .find(|r| r.method == "POST" && r.body.contains(needle))
            .unwrap_or_else(|| panic!("no POST containing {needle}"))
    }
}

impl Drop for Stub {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr); // wakes the accept loop
    }
}

fn serve(stream: TcpStream, routes: &[Route], seen: &Mutex<Vec<Recorded>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut first = String::new();
    if reader.read_line(&mut first).unwrap_or(0) == 0 {
        return;
    }
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("").to_owned();
    let path = parts.next().unwrap_or("").to_owned();
    let mut headers = Vec::new();
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let line = line.trim_end().to_owned();
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim().to_owned());
            if k == "content-length" {
                length = v.parse().unwrap_or(0);
            }
            headers.push((k, v));
        }
    }
    let mut body = vec![0u8; length];
    let _ = reader.read_exact(&mut body);
    let body = String::from_utf8_lossy(&body).into_owned();
    seen.lock().unwrap().push(Recorded {
        method: method.clone(),
        path: path.clone(),
        headers,
        body: body.clone(),
    });
    let hit = routes.iter().find(|r| {
        r.method == method && r.path == path && r.body_has.is_none_or(|n| body.contains(n))
    });
    let (status, extra, out, delay) = match hit {
        Some(r) => (r.status, r.headers.clone(), r.body.clone(), r.delay_ms),
        None => (500, vec![], "no route".to_owned(), 0),
    };
    std::thread::sleep(Duration::from_millis(delay));
    let mut head = format!(
        "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n",
        out.len()
    );
    for (k, v) in extra {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    let mut stream = stream;
    let _ = stream.write_all(format!("{head}\r\n{out}").as_bytes());
    let _ = stream.flush();
    let _ = stream.shutdown(std::net::Shutdown::Both);
}

// ---- reply builders ----

const INIT_REPLY: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{},"serverInfo":{"name":"stub","version":"1"}}}"#;

fn rpc_text(text: &str) -> String {
    json!({"jsonrpc":"2.0","id":2,"result":{"content":[{"type":"text","text":text}]}}).to_string()
}

fn handshake() -> Vec<Route> {
    let mut init = route(
        "POST",
        "/mcp",
        Some(r#""method":"initialize""#),
        200,
        INIT_REPLY,
    );
    init.headers = vec![("Mcp-Session-Id", "sess-1")];
    vec![
        init,
        route(
            "POST",
            "/mcp",
            Some(r#""method":"notifications/initialized""#),
            202,
            "",
        ),
    ]
}

fn call_route(status: u16, body: &str) -> Route {
    route(
        "POST",
        "/mcp",
        Some(r#""method":"tools/call""#),
        status,
        body,
    )
}

/// A stub with the handshake plus one `tools/call` reply, and a client already open on it.
fn open_with(call: Route) -> (Stub, McpClient) {
    let mut routes = handshake();
    routes.push(call);
    routes.push(route("DELETE", "/mcp", None, 200, ""));
    let stub = Stub::start(routes);
    let client = McpClient::open(&stub.base_url(), Some(TOKEN)).expect("open");
    (stub, client)
}

// ---- call_tool results ----

#[test]
fn json_reply_text_is_parsed() {
    let (_stub, mut c) = open_with(call_route(200, &rpc_text(r#"{"ok":true,"n":3}"#)));
    let got = c.call_tool("lab_lease_status", json!({}), T).expect("call");
    assert_eq!(got, json!({"ok": true, "n": 3}));
}

#[test]
fn sse_reply_uses_the_last_data_line() {
    let first = rpc_text(r#"{"v":1}"#);
    let last = rpc_text(r#"{"v":2}"#);
    let sse = format!("event: message\ndata: {first}\n\nevent: message\ndata: {last}\n\n");
    let (_stub, mut c) = open_with(call_route(200, &sse));
    let got = c.call_tool("t", json!({}), T).expect("call");
    assert_eq!(got, json!({"v": 2}));
}

#[test]
fn is_error_becomes_a_tool_error_with_the_text() {
    let body = json!({"jsonrpc":"2.0","id":2,"result":{"isError":true,"content":[{"type":"text","text":"lease-abc is not held"}]}}).to_string();
    let (_stub, mut c) = open_with(call_route(200, &body));
    match c.call_tool("t", json!({}), T) {
        Err(McpError::Tool(m)) => assert_eq!(m, "lease-abc is not held"),
        other => panic!("expected Tool, got {other:?}"),
    }
}

#[test]
fn json_rpc_error_becomes_a_tool_error_with_the_message() {
    let body = r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32602,"message":"unknown tool"}}"#;
    let (_stub, mut c) = open_with(call_route(200, body));
    match c.call_tool("t", json!({}), T) {
        Err(McpError::Tool(m)) => assert_eq!(m, "unknown tool"),
        other => panic!("expected Tool, got {other:?}"),
    }
}

#[test]
fn non_json_text_comes_back_as_a_json_string() {
    let (_stub, mut c) = open_with(call_route(200, &rpc_text("plain words, not json")));
    let got = c.call_tool("t", json!({}), T).expect("call");
    assert_eq!(got, Value::String("plain words, not json".to_owned()));
}

#[test]
fn empty_content_is_a_protocol_error() {
    let body = r#"{"jsonrpc":"2.0","id":2,"result":{"content":[]}}"#;
    let (_stub, mut c) = open_with(call_route(200, body));
    assert!(matches!(
        c.call_tool("t", json!({}), T),
        Err(McpError::Protocol(_))
    ));
}

// ---- transport and HTTP errors ----

#[test]
fn http_401_on_tools_call_is_an_http_error() {
    let (_stub, mut c) = open_with(call_route(401, "bad token"));
    match c.call_tool("t", json!({}), T) {
        Err(McpError::Http(401, body)) => assert_eq!(body, "bad token"),
        other => panic!("expected Http(401), got {other:?}"),
    }
}

#[test]
fn http_403_on_initialize_is_an_http_error() {
    let stub = Stub::start(vec![route("POST", "/mcp", None, 403, "forbidden")]);
    match McpClient::open(&stub.base_url(), Some(TOKEN)) {
        Err(McpError::Http(403, body)) => assert_eq!(body, "forbidden"),
        other => panic!("expected Http(403), got {:?}", other.err()),
    }
}

#[test]
fn nothing_listening_is_unreachable() {
    // Bind to learn a free port, then close it: connecting there is refused.
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    match McpClient::open(&format!("http://127.0.0.1:{port}"), None) {
        Err(McpError::Unreachable(_)) => {}
        other => panic!("expected Unreachable, got {:?}", other.err()),
    }
}

#[test]
fn a_slow_reply_past_the_timeout_is_unreachable() {
    let mut slow = call_route(200, &rpc_text("{}"));
    slow.delay_ms = 1500;
    let (_stub, mut c) = open_with(slow);
    match c.call_tool("t", json!({}), Duration::from_millis(200)) {
        Err(McpError::Unreachable(_)) => {}
        other => panic!("expected Unreachable, got {other:?}"),
    }
}

// ---- what the client sends ----

#[test]
fn initialize_body_and_ids_follow_the_powershell() {
    let (stub, mut c) = open_with(call_route(200, &rpc_text("{}")));
    c.call_tool("lab_lease_status", json!({"a": 1}), T).unwrap();
    c.call_tool("lab_lease_status", json!({}), T).unwrap();
    let init: Value =
        serde_json::from_str(&stub.post_with(r#""method":"initialize""#).body).unwrap();
    assert_eq!(init["id"], 1);
    assert_eq!(init["params"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["params"]["clientInfo"]["name"], "lab-cli");
    let calls: Vec<Value> = stub
        .requests()
        .iter()
        .filter(|r| r.body.contains(r#""method":"tools/call""#))
        .map(|r| serde_json::from_str(&r.body).unwrap())
        .collect();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        (calls[0]["id"].as_u64(), calls[1]["id"].as_u64()),
        (Some(2), Some(3))
    );
    assert_eq!(calls[0]["params"]["name"], "lab_lease_status");
    assert_eq!(calls[0]["params"]["arguments"], json!({"a": 1}));
    let notice: Value =
        serde_json::from_str(&stub.post_with("notifications/initialized").body).unwrap();
    assert!(notice.get("id").is_none(), "a notification carries no id");
}

#[test]
fn session_id_from_initialize_is_sent_on_later_requests() {
    let (stub, mut c) = open_with(call_route(200, &rpc_text("{}")));
    c.call_tool("t", json!({}), T).unwrap();
    assert_eq!(
        stub.post_with(r#""method":"initialize""#)
            .header("mcp-session-id"),
        None
    );
    assert_eq!(
        stub.post_with("notifications/initialized")
            .header("mcp-session-id"),
        Some("sess-1")
    );
    assert_eq!(
        stub.post_with(r#""method":"tools/call""#)
            .header("mcp-session-id"),
        Some("sess-1")
    );
}

#[test]
fn every_request_carries_the_bearer_token_and_accept() {
    let (stub, mut c) = open_with(call_route(200, &rpc_text("{}")));
    c.call_tool("t", json!({}), T).unwrap();
    c.close();
    let seen = stub.requests();
    assert_eq!(seen.len(), 4, "initialize, notice, call, delete");
    for r in &seen {
        assert_eq!(
            r.header("authorization"),
            Some("Bearer tok-123"),
            "{} {}",
            r.method,
            r.body
        );
        assert_eq!(
            r.header("accept"),
            Some("application/json, text/event-stream")
        );
    }
}

#[test]
fn no_token_means_no_authorization_header() {
    let stub = Stub::start(handshake());
    McpClient::open(&stub.base_url(), None).expect("open");
    assert!(stub
        .requests()
        .iter()
        .all(|r| r.header("authorization").is_none()));
}

#[test]
fn close_sends_delete_with_the_session_id() {
    let (stub, c) = open_with(call_route(200, &rpc_text("{}")));
    c.close();
    let del = stub
        .requests()
        .into_iter()
        .find(|r| r.method == "DELETE")
        .expect("a DELETE");
    assert_eq!(del.path, "/mcp");
    assert_eq!(del.header("mcp-session-id"), Some("sess-1"));
}

#[test]
fn close_without_a_session_id_sends_nothing() {
    // A stateless server: initialize answers with no Mcp-Session-Id header.
    let stub = Stub::start(vec![
        route(
            "POST",
            "/mcp",
            Some(r#""method":"initialize""#),
            200,
            INIT_REPLY,
        ),
        route("POST", "/mcp", Some("notifications/initialized"), 202, ""),
    ]);
    McpClient::open(&stub.base_url(), Some(TOKEN))
        .expect("open")
        .close();
    assert!(stub.requests().iter().all(|r| r.method != "DELETE"));
}

#[test]
fn close_ignores_a_dead_daemon() {
    let (stub, c) = open_with(call_route(200, &rpc_text("{}")));
    drop(stub);
    std::thread::sleep(Duration::from_millis(50));
    c.close(); // must return, not panic
}

// ---- hide_lease_ids (test-uat.ps1 'lease ids are redacted' and the golden/heal samples) ----

#[test]
fn lease_ids_are_redacted() {
    // Check: 'lease ids are redacted'
    assert_eq!(
        hide_lease_ids("lease lease-0a1b2c3d4e5f is not the current lease"),
        "lease lease-<redacted> is not the current lease"
    );
    // Check: 'a failing reason has its lease id redacted'
    let r = hide_lease_ids("lease-0123456789abcdef is not the current lease");
    assert!(!r.contains("0123456789abcdef"), "{r}");
    // Check: 'a failed call proposes nothing' (the heal line)
    assert_eq!(
        hide_lease_ids("lab_uat_calibrate failed: lease-0123456789abcdef held"),
        "lab_uat_calibrate failed: lease-<redacted> held"
    );
}

#[test]
fn lease_redaction_edges_follow_the_regex() {
    // 8 hex digits is the shortest id; 7 is not an id.
    assert_eq!(hide_lease_ids("lease-0a1b2c3d"), "lease-<redacted>");
    assert_eq!(hide_lease_ids("lease-0a1b2c3"), "lease-0a1b2c3");
    // PowerShell's -replace ignores case.
    assert_eq!(hide_lease_ids("LEASE-0A1B2C3D4E"), "lease-<redacted>");
    assert_eq!(hide_lease_ids("no ids here"), "no ids here");
}

// ---- the lease wait ----

fn status_with(instance: &str, lease: Value) -> Option<Value> {
    Some(
        json!({"daemon": {"pid": 1, "version": "x"}, "instances": [{"instance": instance, "lease": lease}]}),
    )
}

#[test]
fn wait_returns_true_once_the_lease_clears() {
    let mut polls = 0;
    let mut poll = || {
        polls += 1;
        match polls {
            1 => None, // daemon not answering yet: keep waiting
            2 | 3 => status_with("i1", json!({"held": true})),
            _ => status_with("i1", json!({"held": false})),
        }
    };
    let ok = wait_lease_free_with(
        &mut poll,
        "i1",
        Duration::from_secs(5),
        Duration::from_millis(10),
    );
    assert!(ok);
    assert_eq!(polls, 4, "polled until the lease cleared, no more");
}

#[test]
fn wait_returns_false_when_the_lease_stays_held() {
    let mut poll = || status_with("i1", json!({"held": true}));
    let t = std::time::Instant::now();
    assert!(!wait_lease_free_with(
        &mut poll,
        "i1",
        Duration::from_millis(100),
        Duration::from_millis(10)
    ));
    assert!(
        t.elapsed() >= Duration::from_millis(100),
        "waited the full time"
    );
}

#[test]
fn wait_returns_false_when_the_daemon_never_answers() {
    let mut poll = || None;
    assert!(!wait_lease_free_with(
        &mut poll,
        "i1",
        Duration::from_millis(60),
        Duration::from_millis(10)
    ));
}

#[test]
fn wait_ignores_other_instances_and_treats_a_missing_one_as_free() {
    let mut other_held = || status_with("i2", json!({"held": true}));
    assert!(wait_lease_free_with(
        &mut other_held,
        "i1",
        Duration::from_secs(1),
        Duration::from_millis(10)
    ));
    let mut no_lease = || status_with("i1", Value::Null);
    assert!(wait_lease_free_with(
        &mut no_lease,
        "i1",
        Duration::from_secs(1),
        Duration::from_millis(10)
    ));
}

fn status_route(held: bool) -> Route {
    let body = json!({"daemon": {"pid": 1, "version": "x"}, "instances": [{"instance": "i1", "lease": {"held": held}}]}).to_string();
    route("GET", "/status", None, 200, &body)
}

#[test]
fn wait_reads_status_over_http_with_the_token() {
    let stub = Stub::start(vec![status_route(false)]);
    let ok = wait_lease_free_at_every(
        &stub.base_url(),
        Some(TOKEN),
        "i1",
        Duration::from_secs(5),
        Duration::from_millis(20),
    );
    assert!(ok);
    let req = stub.requests().remove(0);
    assert_eq!((req.method.as_str(), req.path.as_str()), ("GET", "/status"));
    assert_eq!(req.header("authorization"), Some("Bearer tok-123"));
}

#[test]
fn wait_over_http_stays_false_while_held_or_unauthorized() {
    let held = Stub::start(vec![status_route(true)]);
    assert!(!wait_lease_free_at_every(
        &held.base_url(),
        None,
        "i1",
        Duration::from_millis(80),
        Duration::from_millis(20)
    ));
    let denied = Stub::start(vec![route("GET", "/status", None, 401, "no")]);
    assert!(!wait_lease_free_at_every(
        &denied.base_url(),
        None,
        "i1",
        Duration::from_millis(80),
        Duration::from_millis(20)
    ));
    let wrong_shape = Stub::start(vec![route("GET", "/status", None, 200, r#"{"hello":1}"#)]);
    assert!(!wait_lease_free_at_every(
        &wrong_shape.base_url(),
        None,
        "i1",
        Duration::from_millis(80),
        Duration::from_millis(20)
    ));
}
