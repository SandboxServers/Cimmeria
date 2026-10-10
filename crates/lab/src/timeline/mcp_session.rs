//! A streamable-HTTP MCP client session for the in-server lab endpoint
//! (`cimmeria-lab-mcp`).
//!
//! The endpoint is an rmcp `StreamableHttpService` in its default, stateful
//! mode. It answers a `tools/call` that carries no `Mcp-Session-Id` with
//! HTTP 422 "Unexpected message, expect initialize request", which is what
//! every `lab_uat_run` server clause got in the DA-06 run (#1243). So the
//! session does the handshake first: `initialize`, keep the
//! `Mcp-Session-Id` it returns, `notifications/initialized`, then each
//! `tools/call` carries the session id and the negotiated protocol version.
//! A server restart, or the session manager's idle eviction (five minutes
//! in rmcp 3.4's `LocalSessionManager`), forgets the session: the call gets
//! 404, before the tool runs, so it handshakes once more and retries once.
//!
//! The pure helpers ([`initialize_request`], [`tool_call_request`]) build
//! the bodies; [`McpHttpSession::call_tool`] is the transport.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};

use super::packet_tap::{extract_jsonrpc_result, PacketTapConfig};

/// The protocol version this client asks for. The server may answer with
/// another it supports; later requests send the one it chose.
pub const CLIENT_PROTOCOL_VERSION: &str = "2025-06-18";
const HEADER_SESSION_ID: &str = "Mcp-Session-Id";
const HEADER_PROTOCOL_VERSION: &str = "MCP-Protocol-Version";
const ACCEPT: &str = "application/json, text/event-stream";

/// The `initialize` request body.
pub fn initialize_request(id: i64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": CLIENT_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "cimmeria-lab", "version": env!("CARGO_PKG_VERSION") },
        },
    })
}

/// The `tools/call` request body.
pub fn tool_call_request(id: i64, name: &str, args: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": { "name": name, "arguments": args },
    })
}

/// What the handshake established.
#[derive(Debug, Clone)]
struct Session {
    /// `None` when the server runs stateless and sent no session id.
    id: Option<String>,
    protocol_version: String,
}

/// One MCP session with the endpoint, opened on first use.
pub struct McpHttpSession {
    config: PacketTapConfig,
    http: reqwest::Client,
    next_id: AtomicI64,
    session: Mutex<Option<Session>>,
}

/// A response that means the server no longer knows the session.
fn session_lost(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::UNPROCESSABLE_ENTITY
}

impl McpHttpSession {
    pub fn new(config: PacketTapConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            next_id: AtomicI64::new(1),
            session: Mutex::new(None),
        }
    }

    pub fn url(&self) -> &str {
        &self.config.url
    }

    fn next_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    fn cached(&self) -> Option<Session> {
        self.session.lock().ok().and_then(|s| s.clone())
    }

    /// Drop the cached session, unless another call already replaced the
    /// one that failed.
    fn forget(&self, failed: &Session) {
        if let Ok(mut s) = self.session.lock() {
            if s.as_ref().map(|c| &c.id) == Some(&failed.id) {
                *s = None;
            }
        }
    }

    fn post(
        &self,
        body: &Value,
        session: Option<&Session>,
        timeout: Duration,
    ) -> reqwest::RequestBuilder {
        let mut req = self
            .http
            .post(&self.config.url)
            .bearer_auth(&self.config.token)
            .header("Accept", ACCEPT)
            .timeout(timeout)
            .json(body);
        if let Some(s) = session {
            if let Some(id) = &s.id {
                req = req.header(HEADER_SESSION_ID, id);
            }
            req = req.header(HEADER_PROTOCOL_VERSION, &s.protocol_version);
        }
        req
    }

    /// `initialize` + `notifications/initialized`; caches the session.
    async fn handshake(&self, timeout: Duration) -> Result<Session, String> {
        let url = &self.config.url;
        let resp = self
            .post(&initialize_request(self.next_id()), None, timeout)
            .send()
            .await
            .map_err(|e| format!("lab-mcp {url} initialize: {e}"))?;
        let status = resp.status();
        let id = resp
            .headers()
            .get(HEADER_SESSION_ID)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let text = resp
            .text()
            .await
            .map_err(|e| format!("lab-mcp {url} initialize read body: {e}"))?;
        if !status.is_success() {
            let snippet: String = text.chars().take(200).collect();
            return Err(format!("lab-mcp initialize HTTP {status}: {snippet}"));
        }
        let result = extract_jsonrpc_result(&text)?;
        let session = Session {
            id,
            protocol_version: result
                .get("protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(CLIENT_PROTOCOL_VERSION)
                .to_string(),
        };
        let note = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        let resp = self
            .post(&note, Some(&session), timeout)
            .send()
            .await
            .map_err(|e| format!("lab-mcp {url} initialized: {e}"))?;
        if !resp.status().is_success() {
            return Err(format!(
                "lab-mcp notifications/initialized HTTP {}",
                resp.status()
            ));
        }
        if let Ok(mut s) = self.session.lock() {
            *s = Some(session.clone());
        }
        Ok(session)
    }

    /// Open the session now if none is cached, so a caller timing the
    /// tool call (the timeline's clock ping) does not time the handshake.
    pub async fn ensure_session(&self, timeout: Duration) -> Result<(), String> {
        if self.cached().is_none() {
            self.handshake(timeout).await?;
        }
        Ok(())
    }

    /// One `tools/call`; returns the JSON-RPC `result` (an rmcp
    /// `CallToolResult`). `timeout` bounds the whole call, handshake and
    /// one retry included. Transport, HTTP and JSON-RPC errors come back as
    /// `Err`, never a panic.
    pub async fn call_tool(
        &self,
        name: &str,
        args: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        match tokio::time::timeout(timeout, self.call_tool_inner(name, args, timeout)).await {
            Ok(r) => r,
            Err(_) => Err(format!(
                "lab-mcp {}: {name} timed out after {} s",
                self.config.url,
                timeout.as_secs()
            )),
        }
    }

    async fn call_tool_inner(
        &self,
        name: &str,
        args: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        let url = &self.config.url;
        let mut retried = false;
        loop {
            let session = match self.cached() {
                Some(s) => s,
                None => self.handshake(timeout).await?,
            };
            let body = tool_call_request(self.next_id(), name, args.clone());
            let resp = self
                .post(&body, Some(&session), timeout)
                .send()
                .await
                .map_err(|e| format!("lab-mcp {url}: {e}"))?;
            let status = resp.status();
            // rmcp answers 404 for an unknown session before running the tool,
            // so the one retry never runs a tool twice.
            if session_lost(status) && !retried {
                self.forget(&session);
                retried = true;
                continue;
            }
            let text = resp
                .text()
                .await
                .map_err(|e| format!("lab-mcp {url} {name} read body: {e}"))?;
            if !status.is_success() {
                let snippet: String = text.chars().take(200).collect();
                return Err(format!("lab-mcp HTTP {status}: {snippet}"));
            }
            return extract_jsonrpc_result(&text);
        }
    }
}

#[cfg(test)]
impl McpHttpSession {
    /// Pretend the server forgot the session: keep a session id it never issued.
    fn poison_session(&self) {
        if let Ok(mut s) = self.session.lock() {
            if let Some(s) = s.as_mut() {
                s.id = Some("forgotten-by-the-server".into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::http_tests::{spawn_daemon, test_server, TOKEN};

    fn config(url: String) -> PacketTapConfig {
        PacketTapConfig {
            url,
            token: TOKEN.into(),
        }
    }

    /// The bug shape (#1243): a stateful rmcp endpoint refuses a `tools/call`
    /// that skipped the handshake with 422. Guards that the fixture is the
    /// kind of server the session exists for.
    #[tokio::test]
    async fn bare_tools_call_is_refused_with_422() {
        let url = spawn_daemon(test_server()).await;
        let resp = reqwest::Client::new()
            .post(&url)
            .bearer_auth(TOKEN)
            .header("Accept", ACCEPT)
            .json(&tool_call_request(1, "lab_lease_status", json!({})))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn call_tool_handshakes_first() {
        let url = spawn_daemon(test_server()).await;
        let s = McpHttpSession::new(config(url));
        let result = s
            .call_tool("lab_lease_status", json!({}), Duration::from_secs(10))
            .await
            .expect("tools/call after the handshake");
        assert_ne!(result.get("isError"), Some(&json!(true)), "{result}");
        // The session is reused, not renegotiated, on the next call.
        let id = s.cached().and_then(|x| x.id);
        assert!(id.is_some());
        s.call_tool("lab_lease_status", json!({}), Duration::from_secs(10))
            .await
            .unwrap();
        assert_eq!(s.cached().and_then(|x| x.id), id);
    }

    #[tokio::test]
    async fn a_forgotten_session_handshakes_again() {
        let url = spawn_daemon(test_server()).await;
        let s = McpHttpSession::new(config(url));
        s.call_tool("lab_lease_status", json!({}), Duration::from_secs(10))
            .await
            .unwrap();
        s.poison_session();
        s.call_tool("lab_lease_status", json!({}), Duration::from_secs(10))
            .await
            .expect("re-handshake after the server lost the session");
        assert_ne!(
            s.cached().and_then(|x| x.id).as_deref(),
            Some("forgotten-by-the-server")
        );
    }

    #[test]
    fn requests_are_jsonrpc() {
        let init = initialize_request(7);
        assert_eq!(init["method"], "initialize");
        assert_eq!(init["params"]["protocolVersion"], CLIENT_PROTOCOL_VERSION);
        let call = tool_call_request(8, "server_sessions", json!({ "a": 1 }));
        assert_eq!(call["params"]["name"], "server_sessions");
        assert_eq!(call["params"]["arguments"]["a"], 1);
    }
}
