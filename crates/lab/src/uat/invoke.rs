//! The action layer: tools are called by *name*, so a spec can name a
//! tool that a sibling change adds later. A name the invoker does not
//! route makes the row BLOCKED, naming the tool; nothing is guessed.
//!
//! Two invokers exist: the client lab router (in-process, see
//! `server::uat`) and [`ServerTools`], an HTTP MCP client of the in-server
//! `cimmeria-lab-mcp` endpoint for `server_*` evidence reads. Tests use a
//! scripted fake.

use std::future::Future;

use base64::Engine as _;
use serde_json::{json, Value};

use crate::timeline::packet_tap::{extract_jsonrpc_result, PacketTapConfig};

/// A tool call's result, normalized from the MCP `CallToolResult`.
#[derive(Debug, Clone, Default)]
pub struct ToolOutcome {
    pub ok: bool,
    /// The text content parsed as JSON when it is JSON, else the text.
    pub json: Value,
    /// Decoded image blocks: (mime type, bytes).
    pub images: Vec<(String, Vec<u8>)>,
    pub error: Option<String>,
    /// The MCP error's `data` (flows put their step log there).
    pub error_data: Option<Value>,
}

impl ToolOutcome {
    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(msg.into()),
            ..Default::default()
        }
    }
}

/// Calls tools by name.
pub trait ToolInvoker: Send + Sync {
    /// Whether `name` is routed right now.
    fn has_tool(&self, name: &str) -> bool;
    /// Every routed tool name, sorted.
    fn tool_names(&self) -> Vec<String>;
    /// Call `name` with a JSON object of arguments.
    fn call(&self, name: &str, args: Value) -> impl Future<Output = ToolOutcome> + Send;
}

/// Normalize a serialized `CallToolResult` (`{content: [...], isError}`).
pub fn normalize_result(v: &Value) -> ToolOutcome {
    let mut texts = Vec::new();
    let mut images = Vec::new();
    for block in v
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(t) = block.get("text").and_then(Value::as_str) {
                    texts.push(t.to_string());
                }
            }
            Some("image") => {
                let mime = block
                    .get("mimeType")
                    .and_then(Value::as_str)
                    .unwrap_or("image/png")
                    .to_string();
                if let Some(data) = block.get("data").and_then(Value::as_str) {
                    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(data) {
                        images.push((mime, bytes));
                    }
                }
            }
            _ => {}
        }
    }
    let json = match (v.get("structuredContent"), texts.as_slice()) {
        (Some(sc), _) if !sc.is_null() => sc.clone(),
        (_, [one]) => serde_json::from_str(one).unwrap_or_else(|_| Value::String(one.clone())),
        (_, []) => Value::Null,
        (_, many) => Value::String(many.join("\n")),
    };
    let is_error = v.get("isError").and_then(Value::as_bool).unwrap_or(false);
    ToolOutcome {
        ok: !is_error,
        error: is_error.then(|| match &json {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        }),
        json,
        images,
        error_data: None,
    }
}

/// HTTP MCP client for the in-server lab endpoint (`server_*` tools).
/// Configured from `CIMMERIA_LAB_MCP_URL` + `CIMMERIA_LAB_MCP_TOKEN`,
/// like `lab_timeline`. When the endpoint refuses (the colo's HTTP 403
/// "Host header is not allowed" seen 2026-09-29) every server clause is
/// UNVERIFIED with that error, and the SigNoz clauses carry the row.
pub struct ServerTools {
    config: PacketTapConfig,
    http: reqwest::Client,
    next_id: std::sync::atomic::AtomicI64,
}

impl ServerTools {
    pub fn from_env() -> Option<Self> {
        PacketTapConfig::from_env().map(|config| Self {
            config,
            http: reqwest::Client::new(),
            next_id: std::sync::atomic::AtomicI64::new(1),
        })
    }

    pub fn url(&self) -> &str {
        &self.config.url
    }

    /// One `tools/call`. Transport and JSON-RPC errors come back as a
    /// failed outcome, never a panic.
    pub async fn call(&self, name: &str, args: Value) -> ToolOutcome {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": args },
        });
        let resp = self
            .http
            .post(&self.config.url)
            .bearer_auth(&self.config.token)
            .header("Accept", "application/json, text/event-stream")
            .timeout(std::time::Duration::from_secs(20))
            .json(&body)
            .send()
            .await;
        let resp = match resp {
            Ok(r) => r,
            Err(e) => return ToolOutcome::err(format!("lab-mcp {}: {e}", self.config.url)),
        };
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            let snippet: String = text.chars().take(200).collect();
            return ToolOutcome::err(format!("lab-mcp HTTP {status}: {snippet}"));
        }
        match extract_jsonrpc_result(&text) {
            Ok(result) => normalize_result(&result),
            Err(e) => ToolOutcome::err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_json_is_parsed_and_images_decoded() {
        let v = json!({
            "content": [
                { "type": "text", "text": "{\"met\": true}" },
                { "type": "image", "data": "iVBORw==", "mimeType": "image/png" }
            ]
        });
        let o = normalize_result(&v);
        assert!(o.ok);
        assert_eq!(o.json["met"], true);
        assert_eq!(o.images.len(), 1);
        assert_eq!(o.images[0].1, vec![0x89, b'P', b'N', b'G']);
    }

    #[test]
    fn plain_text_and_errors_survive() {
        let o = normalize_result(
            &json!({ "content": [{ "type": "text", "text": "client window 800x600" }] }),
        );
        assert_eq!(o.json, json!("client window 800x600"));
        let o = normalize_result(
            &json!({ "content": [{ "type": "text", "text": "nope" }], "isError": true }),
        );
        assert!(!o.ok);
        assert_eq!(o.error.as_deref(), Some("nope"));
    }
}
