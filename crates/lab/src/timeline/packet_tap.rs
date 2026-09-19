//! Fetch server packet-tap rows from `cimmeria-lab-mcp` over HTTP.
//!
//! The supervisor is an MCP *client* of the in-server lab endpoint
//! (ADR §3.5). It does not link `cimmeria-lab-mcp`; it calls the
//! `server_packet_tap_read` tool (issue #688 / PR #695) over the network
//! and merges the returned rows locally. SigNoz keeps the durable copy;
//! this path is the low-latency local read (ADR §5).
//!
//! Two halves, split so the logic is testable without a live server:
//!
//! - **Pure** — [`build_tap_request`] (the JSON-RPC `tools/call` body),
//!   [`extract_jsonrpc_result`] (peel the envelope out of a JSON *or*
//!   SSE response body), and [`parse_tap_result`] (rmcp `CallToolResult`
//!   → [`TapRow`]s, tolerant of the exact field names #688 settles on).
//! - **Transport** — [`PacketTapClient::fetch`], a thin `reqwest` POST.
//!
//! LIVE-VALIDATION SEAM: streamable-HTTP MCP servers may require an
//! `initialize` handshake and an `Mcp-Session-Id` header before
//! `tools/call`. That negotiation is confirmed against the running colo
//! endpoint (the #689 exit criterion); if #695 requires it, add it in
//! [`PacketTapClient::fetch`] — the pure request/response/parse helpers
//! do not change.

use serde::Serialize;
use serde_json::{json, Value};

use super::clock::PingSample;
use super::event::TimelineEvent;

/// Config for reaching the in-server lab endpoint. Absent env ⇒ no
/// endpoint (fail-closed; the timeline runs client-only).
#[derive(Debug, Clone)]
pub struct PacketTapConfig {
    /// Full MCP endpoint URL, e.g. `http://10.0.0.2:8444/mcp`.
    pub url: String,
    /// Bearer token (the server's `CIMMERIA_LAB_MCP_TOKEN`).
    pub token: String,
}

impl PacketTapConfig {
    /// Read from `CIMMERIA_LAB_MCP_URL` + `CIMMERIA_LAB_MCP_TOKEN`.
    /// Returns `None` if either is unset (endpoint absent by design).
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("CIMMERIA_LAB_MCP_URL").ok()?;
        let token = std::env::var("CIMMERIA_LAB_MCP_TOKEN").ok()?;
        if url.is_empty() || token.is_empty() {
            return None;
        }
        Some(Self { url, token })
    }
}

/// One decoded Mercury message from the server tap.
#[derive(Debug, Clone, Serialize)]
pub struct TapRow {
    /// Server-clock time of the packet (ms since epoch).
    pub ts_ms: i64,
    /// `"c2s"` / `"s2c"` when the tap reports it.
    pub direction: Option<String>,
    /// Message name/id when reported.
    pub name: Option<String>,
    /// The untouched row, for drill-down.
    pub raw: Value,
}

impl TapRow {
    /// Project onto the merged timeline. `kind` is `mercury.<name>` when
    /// a name is present, else `mercury.packet`.
    pub fn into_event(self) -> TimelineEvent {
        let kind = match &self.name {
            Some(n) => format!("mercury.{n}"),
            None => "mercury.packet".to_string(),
        };
        TimelineEvent::server(self.ts_ms, kind, self.direction.clone(), self.raw)
    }
}

/// Build the JSON-RPC `tools/call` body for `server_packet_tap_read`.
pub fn build_tap_request(id: i64, session_id: &str, since_ms: Option<i64>, limit: u32) -> Value {
    let mut args = json!({ "session_id": session_id, "limit": limit });
    if let Some(since) = since_ms {
        args["since_ms"] = json!(since);
    }
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": { "name": "server_packet_tap_read", "arguments": args },
    })
}

/// Peel the JSON-RPC `result` out of a response body that is either a
/// bare JSON object or an SSE stream (`event:`/`data:` lines). Returns an
/// error string on a JSON-RPC `error` or an unparseable body.
pub fn extract_jsonrpc_result(body: &str) -> Result<Value, String> {
    // SSE: find the last `data:` line and parse that as the envelope.
    // (A single tools/call reply is one data frame, but be liberal.)
    let envelope: Value = if let Some(data) = last_sse_data(body) {
        serde_json::from_str(&data).map_err(|e| format!("SSE data not JSON: {e}"))?
    } else {
        serde_json::from_str(body.trim()).map_err(|e| format!("body not JSON: {e}"))?
    };

    if let Some(err) = envelope.get("error") {
        let code = err.get("code").and_then(Value::as_i64).unwrap_or(0);
        let msg = err
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("(no message)");
        return Err(format!("lab-mcp error {code}: {msg}"));
    }
    envelope
        .get("result")
        .cloned()
        .ok_or_else(|| "lab-mcp reply had neither result nor error".to_string())
}

/// Return the payload of the last `data:` line in an SSE body, or `None`
/// if the body isn't SSE.
fn last_sse_data(body: &str) -> Option<String> {
    let mut found = None;
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            found = Some(rest.trim().to_string());
        }
    }
    found
}

/// Parse an rmcp `CallToolResult` (the `result` of the tools/call) into
/// tap rows. Tolerant of the shapes #688 might use:
/// `structuredContent` with a rows array, or a text content block holding
/// pretty JSON, or a bare array.
pub fn parse_tap_result(result: &Value) -> Vec<TapRow> {
    // 1. structuredContent (preferred, machine-readable).
    if let Some(sc) = result.get("structuredContent") {
        let rows = rows_from_value(sc);
        if !rows.is_empty() {
            return rows;
        }
    }
    // 2. text content blocks holding JSON.
    if let Some(content) = result.get("content").and_then(Value::as_array) {
        for block in content {
            if let Some(text) = block.get("text").and_then(Value::as_str) {
                if let Ok(v) = serde_json::from_str::<Value>(text) {
                    let rows = rows_from_value(&v);
                    if !rows.is_empty() {
                        return rows;
                    }
                }
            }
        }
    }
    // 3. the result itself might already be the rows container.
    rows_from_value(result)
}

/// Pull a rows array out of a container that is either a bare array or an
/// object with one of the expected row keys.
fn rows_from_value(v: &Value) -> Vec<TapRow> {
    let arr = if let Some(a) = v.as_array() {
        a.clone()
    } else if let Some(obj) = v.as_object() {
        ["rows", "packets", "messages", "events"]
            .iter()
            .find_map(|k| obj.get(*k).and_then(Value::as_array).cloned())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    arr.iter().filter_map(row_from_value).collect()
}

/// One row object → [`TapRow`], tolerant of alternate field names.
fn row_from_value(v: &Value) -> Option<TapRow> {
    let obj = v.as_object()?;
    let ts_ms = ["ts_ms", "server_ts_ms", "ts", "timestamp_ms", "time_ms"]
        .iter()
        .find_map(|k| obj.get(*k).and_then(Value::as_i64))?;
    let direction = ["direction", "dir"]
        .iter()
        .find_map(|k| obj.get(*k).and_then(Value::as_str))
        .map(str::to_string);
    let name = ["message", "name", "msg", "method", "msg_name"]
        .iter()
        .find_map(|k| obj.get(*k).and_then(Value::as_str))
        .map(str::to_string);
    Some(TapRow {
        ts_ms,
        direction,
        name,
        raw: v.clone(),
    })
}

/// The result of one live fetch: the rows plus, if any arrived, a clock
/// ping sample built from the round trip (newest row ts ≈ "server now").
pub struct TapFetch {
    pub rows: Vec<TapRow>,
    pub ping: Option<PingSample>,
}

/// HTTP client for the in-server lab endpoint.
pub struct PacketTapClient {
    config: PacketTapConfig,
    http: reqwest::Client,
    next_id: std::sync::atomic::AtomicI64,
}

impl PacketTapClient {
    pub fn new(config: PacketTapConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            next_id: std::sync::atomic::AtomicI64::new(1),
        }
    }

    /// Fetch tap rows for `session_id`, and build a coarse clock-ping
    /// sample from the round trip (see the [`super::clock`] note on why
    /// the newest row's server timestamp stands in for "server now").
    pub async fn fetch(
        &self,
        session_id: &str,
        since_ms: Option<i64>,
        limit: u32,
    ) -> Result<TapFetch, String> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let req = build_tap_request(id, session_id, since_ms, limit);

        let local_send_ms = now_ms();
        let resp = self
            .http
            .post(&self.config.url)
            .bearer_auth(&self.config.token)
            .header("Accept", "application/json, text/event-stream")
            .json(&req)
            .send()
            .await
            .map_err(|e| format!("lab-mcp POST failed: {e}"))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| format!("lab-mcp read body failed: {e}"))?;
        let local_recv_ms = now_ms();
        if !status.is_success() {
            return Err(format!("lab-mcp HTTP {status}: {body}"));
        }

        let result = extract_jsonrpc_result(&body)?;
        let rows = parse_tap_result(&result);

        let ping = rows
            .iter()
            .map(|r| r.ts_ms)
            .max()
            .map(|server_ms| PingSample {
                local_send_ms,
                server_ms,
                local_recv_ms,
            });
        Ok(TapFetch { rows, ping })
    }
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_shape_is_tools_call() {
        let req = build_tap_request(7, "sess-1", Some(1234), 500);
        assert_eq!(req["method"], "tools/call");
        assert_eq!(req["params"]["name"], "server_packet_tap_read");
        assert_eq!(req["params"]["arguments"]["session_id"], "sess-1");
        assert_eq!(req["params"]["arguments"]["since_ms"], 1234);
        assert_eq!(req["params"]["arguments"]["limit"], 500);
    }

    #[test]
    fn request_omits_since_when_none() {
        let req = build_tap_request(1, "s", None, 10);
        assert!(req["params"]["arguments"].get("since_ms").is_none());
    }

    #[test]
    fn extract_result_from_plain_json() {
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"rows":[]}}"#;
        let r = extract_jsonrpc_result(body).unwrap();
        assert!(r.get("rows").is_some());
    }

    #[test]
    fn extract_result_from_sse() {
        let body =
            "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"rows\":[]}}\n\n";
        let r = extract_jsonrpc_result(body).unwrap();
        assert!(r.get("rows").is_some());
    }

    #[test]
    fn extract_result_surfaces_jsonrpc_error() {
        let body =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"no such session"}}"#;
        let err = extract_jsonrpc_result(body).unwrap_err();
        assert!(err.contains("no such session"), "{err}");
    }

    #[test]
    fn parse_rows_from_text_content_block() {
        // The shape #695's tool most likely returns: a text block of
        // pretty JSON with a `rows` array (mirrors the client-side lab
        // server's `wrap`).
        let inner = json!({
            "rows": [
                { "ts_ms": 1000, "direction": "s2c", "message": "createEntity" },
                { "ts": 1005, "dir": "c2s", "name": "avatarUpdate" },
            ]
        });
        let result = json!({
            "content": [ { "type": "text", "text": inner.to_string() } ],
            "isError": false,
        });
        let rows = parse_tap_result(&result);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].ts_ms, 1000);
        assert_eq!(rows[0].direction.as_deref(), Some("s2c"));
        assert_eq!(rows[0].name.as_deref(), Some("createEntity"));
        // Alternate field names on the second row are picked up too.
        assert_eq!(rows[1].ts_ms, 1005);
        assert_eq!(rows[1].direction.as_deref(), Some("c2s"));
        assert_eq!(rows[1].name.as_deref(), Some("avatarUpdate"));
    }

    #[test]
    fn parse_rows_from_structured_content() {
        let result = json!({
            "structuredContent": { "packets": [ { "ts_ms": 42, "message": "ping" } ] },
        });
        let rows = parse_tap_result(&result);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name.as_deref(), Some("ping"));
    }

    #[test]
    fn row_without_timestamp_is_dropped() {
        let result = json!({ "rows": [ { "message": "no-ts" }, { "ts_ms": 5 } ] });
        let rows = parse_tap_result(&result);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].ts_ms, 5);
    }

    #[test]
    fn tap_row_maps_to_mercury_event_kind() {
        let row = TapRow {
            ts_ms: 100,
            direction: Some("s2c".into()),
            name: Some("createEntity".into()),
            raw: json!({}),
        };
        let ev = row.into_event();
        assert_eq!(ev.kind, "mercury.createEntity");
        assert_eq!(ev.server_ts_ms, 100);
    }
}
