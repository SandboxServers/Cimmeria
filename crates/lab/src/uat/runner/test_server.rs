//! The fake `cimmeria-lab-mcp` the packet and ability tests drive: canned
//! tap reads, scripted replies per tool, and a switch that makes every call
//! fail like the colo's HTTP 403.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use serde_json::{json, Value};

use super::tests::{canned_outcome, next_canned};
use crate::uat::invoke::{BoxedOutcome, ServerInvoker, ToolOutcome};

/// A pretend lab endpoint: two sessions, a canned tap read.
pub(super) struct FakeServer {
    pub(super) calls: Mutex<Vec<(String, Value)>>,
    tap: Value,
    /// Every call fails like the colo's 2026-09-29 HTTP 403.
    pub(super) unreachable: bool,
    /// How many `server_packet_tap_stop` calls fail before one succeeds.
    pub(super) stop_failures: Mutex<u32>,
    /// Scripted replies per tool, popped in order (see `answer`).
    canned: Mutex<HashMap<String, VecDeque<Value>>>,
}

impl FakeServer {
    pub(super) fn new(tap: Value) -> Self {
        Self {
            calls: Mutex::new(vec![]),
            tap,
            unreachable: false,
            stop_failures: Mutex::new(0),
            canned: Mutex::new(HashMap::new()),
        }
    }

    /// Script the replies `tool` gives, in order; the last one repeats.
    pub(super) fn answer(self, tool: &str, replies: Vec<Value>) -> Self {
        self.canned
            .lock()
            .unwrap()
            .insert(tool.to_string(), VecDeque::from(replies));
        self
    }

    pub(super) fn names(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.0.clone())
            .collect()
    }
}

impl ServerInvoker for FakeServer {
    fn url(&self) -> &str {
        "http://127.0.0.1:1/mcp"
    }

    fn call<'a>(&'a self, name: &'a str, args: Value) -> BoxedOutcome<'a> {
        self.calls.lock().unwrap().push((name.to_string(), args));
        if let Some(reply) = next_canned(&self.canned, name) {
            let out = canned_outcome(reply);
            return Box::pin(async move { out });
        }
        let ok = |json: Value| ToolOutcome {
            ok: true,
            json,
            ..Default::default()
        };
        let out = if self.unreachable {
            ToolOutcome::err("lab-mcp HTTP 403 Forbidden: Host header is not allowed")
        } else {
            match name {
                "server_sessions" => ok(json!({ "count": 2, "sessions": [
                    { "entity_id": 7, "name": "Labone", "status": "in_world" },
                    { "entity_id": 9, "name": "Labtwo", "status": "in_world" },
                ]})),
                "server_packet_tap_start" => {
                    ok(json!({ "entity_id": 7, "replaced_existing": false }))
                }
                "server_packet_tap_read" => ok(self.tap.clone()),
                "server_packet_tap_stop" => {
                    let mut left = self.stop_failures.lock().unwrap();
                    if *left > 0 {
                        *left -= 1;
                        ToolOutcome::err("lab-mcp: operation timed out")
                    } else {
                        ok(json!({ "entity_id": 7, "stopped": true }))
                    }
                }
                _ => ToolOutcome::err(format!("unknown tool {name}")),
            }
        };
        Box::pin(async move { out })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_scripted_server_reply_pops_then_repeats() {
        let server = FakeServer::new(json!({})).answer(
            "server_sessions",
            vec![json!({ "count": 1 }), json!({ "count": 2 })],
        );
        assert_eq!(
            server.call("server_sessions", json!({})).await.json["count"],
            1
        );
        assert_eq!(
            server.call("server_sessions", json!({})).await.json["count"],
            2
        );
        assert_eq!(
            server.call("server_sessions", json!({})).await.json["count"],
            2
        );
    }
}
