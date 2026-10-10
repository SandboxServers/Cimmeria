//! Transport tests for the daemon: a real `axum::serve` on an ephemeral
//! loopback port, driven with raw MCP JSON-RPC over HTTP (reqwest).

use std::sync::Arc;

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, HOST, ORIGIN};
use serde_json::{json, Value};

use crate::client::BridgeClient;
use crate::server::LabServer;
use crate::supervisor::{Supervisor, SupervisorConfig};

pub(crate) const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef";

pub(crate) fn test_server() -> LabServer {
    // Nothing listens on the bridge port: the tests never reach the client.
    let bridge = Arc::new(BridgeClient::new("127.0.0.1:1", String::new()));
    let mut config = SupervisorConfig::from_env();
    config.install_dir = None;
    // A fresh lease book, so the test owns its lease state.
    let sup =
        Supervisor::new(bridge, config).with_leases(Arc::new(crate::lease::LeaseBook::default()));
    LabServer::new(Arc::new(sup))
}

/// Serve the daemon router on 127.0.0.1:0; returns the `/mcp` URL.
pub(crate) async fn spawn_daemon(server: LabServer) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = super::build_router(server, TOKEN);
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    format!("http://{addr}/mcp")
}

fn headers(token: Option<&str>) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    h.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/event-stream"),
    );
    if let Some(t) = token {
        h.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {t}")).unwrap(),
        );
    }
    h
}

/// The JSON-RPC message in a response body: plain JSON, or the first SSE
/// `data:` line that parses.
fn rpc_body(text: &str) -> Value {
    if let Ok(v) = serde_json::from_str(text) {
        return v;
    }
    text.lines()
        .filter_map(|l| l.strip_prefix("data:"))
        .filter_map(|d| serde_json::from_str::<Value>(d.trim()).ok())
        .find(|v| v.get("id").is_some())
        .unwrap_or_else(|| panic!("no JSON-RPC response in {text:?}"))
}

fn initialize_body() -> Value {
    json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "labd-test", "version": "0" }
        }
    })
}

/// An MCP session over HTTP: initialize, the initialized notification,
/// then requests carrying `mcp-session-id`.
pub(crate) struct McpSession {
    http: reqwest::Client,
    url: String,
    session: Option<String>,
    next_id: u64,
}

impl McpSession {
    pub(crate) async fn open(url: &str) -> Self {
        let http = reqwest::Client::new();
        let r = http
            .post(url)
            .headers(headers(Some(TOKEN)))
            .json(&initialize_body())
            .send()
            .await
            .unwrap();
        assert!(r.status().is_success(), "initialize: {}", r.status());
        let session = r
            .headers()
            .get("mcp-session-id")
            .map(|v| v.to_str().unwrap().to_string());
        let init = rpc_body(&r.text().await.unwrap());
        assert!(init["result"]["serverInfo"].is_object(), "{init}");
        let s = Self {
            http,
            url: url.to_string(),
            session,
            next_id: 2,
        };
        let r = s
            .post(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .await;
        assert!(r.status().is_success(), "initialized: {}", r.status());
        s
    }

    async fn post(&self, body: Value) -> reqwest::Response {
        let mut req = self
            .http
            .post(&self.url)
            .headers(headers(Some(TOKEN)))
            .json(&body);
        if let Some(id) = &self.session {
            req = req.header("mcp-session-id", id);
        }
        req.send().await.unwrap()
    }

    pub(crate) async fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let r = self
            .post(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
            .await;
        assert!(r.status().is_success(), "{method}: {}", r.status());
        rpc_body(&r.text().await.unwrap())
    }

    /// `tools/call`; returns the JSON-RPC message (result or error).
    pub(crate) async fn call(&mut self, name: &str, args: Value) -> Value {
        self.request("tools/call", json!({ "name": name, "arguments": args }))
            .await
    }
}

/// Transport smoke: initialize and list the tools over HTTP.
#[tokio::test]
async fn initialize_and_list_tools_over_http() {
    let url = spawn_daemon(test_server()).await;
    let mut s = McpSession::open(&url).await;
    let list = s.request("tools/list", json!({})).await;
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    for want in ["lab_client_status", "client_lua_eval", "lab_uat_run"] {
        assert!(names.contains(&want), "{want} missing from {names:?}");
    }
    // A tool call round-trips too (no client: the status says so).
    let status = s.call("lab_client_status", json!({})).await;
    assert!(status["result"]["content"].is_array(), "{status}");
}

/// Regression guard: no token, or the wrong one, never reaches MCP.
#[tokio::test]
async fn requests_without_the_token_are_refused() {
    let url = spawn_daemon(test_server()).await;
    let http = reqwest::Client::new();
    for token in [None, Some("wrong-token-wrong-token-wrong-token!!")] {
        let r = http
            .post(&url)
            .headers(headers(token))
            .json(&initialize_body())
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), reqwest::StatusCode::UNAUTHORIZED, "{token:?}");
    }
}

/// `/status` is behind the same bearer gate as `/mcp`.
#[tokio::test]
async fn status_needs_the_bearer() {
    let url = spawn_daemon(test_server()).await.replace("/mcp", "/status");
    let http = reqwest::Client::new();
    let r = http.get(&url).headers(headers(None)).send().await.unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::UNAUTHORIZED);
    let r = http
        .get(&url)
        .headers(headers(Some(TOKEN)))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::OK);
}

/// `/status` lists each instance with its lease state, never the lease id.
#[tokio::test]
async fn status_lists_instances_without_lease_ids() {
    let server = test_server();
    let lease = server
        .instances()
        .first()
        .supervisor
        .leases()
        .acquire(crate::lease::AcquireRequest {
            owner: "status-test".into(),
            purpose: "status test".into(),
            ..Default::default()
        })
        .unwrap();
    let url = spawn_daemon(server).await.replace("/mcp", "/status");
    let r = reqwest::Client::new()
        .get(&url)
        .headers(headers(Some(TOKEN)))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::OK);
    let text = r.text().await.unwrap();
    assert!(!text.contains(&lease.lease_id), "lease id leaked: {text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    let rows = body["instances"].as_array().expect("instances array");
    assert_eq!(rows.len(), 1, "{body}");
    assert_eq!(rows[0]["lease"]["held"], true, "{body}");
}

/// Regression guard (DNS rebinding): a valid token from a page that resolved
/// some other name to 127.0.0.1 is still refused by its `Host`, and any
/// browser `Origin` is refused.
#[tokio::test]
async fn foreign_host_and_any_origin_are_refused() {
    let url = spawn_daemon(test_server()).await;
    let http = reqwest::Client::new();
    let mut h = headers(Some(TOKEN));
    h.insert(HOST, HeaderValue::from_static("evil.example"));
    let r = http
        .post(&url)
        .headers(h)
        .json(&initialize_body())
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::FORBIDDEN);

    let mut h = headers(Some(TOKEN));
    h.insert(ORIGIN, HeaderValue::from_static("http://127.0.0.1:8779"));
    let r = http
        .post(&url)
        .headers(h)
        .json(&initialize_body())
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::FORBIDDEN);
}
