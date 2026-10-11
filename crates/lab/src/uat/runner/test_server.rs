//! The fake `cimmeria-lab-mcp` the packet, ability and fingerprint tests
//! drive: canned tap reads, entity snapshots, scripted replies per tool, a
//! flag that keeps the character out of the world, and a switch that makes
//! every call fail like the colo's HTTP 403.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

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
    /// While this is false `server_sessions` lists nobody: the character
    /// is not in the world yet (see `in_world_when`).
    in_world: Option<Arc<AtomicBool>>,
}

impl FakeServer {
    pub(super) fn new(tap: Value) -> Self {
        Self {
            calls: Mutex::new(vec![]),
            tap,
            unreachable: false,
            stop_failures: Mutex::new(0),
            canned: Mutex::new(HashMap::new()),
            in_world: None,
        }
    }

    /// List the sessions only once `flag` is set (a test's client sets it
    /// when the row's "play" action runs).
    pub(super) fn in_world_when(mut self, flag: Arc<AtomicBool>) -> Self {
        self.in_world = Some(flag);
        self
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
        self.calls
            .lock()
            .unwrap()
            .push((name.to_string(), args.clone()));
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
            let in_world = self
                .in_world
                .as_ref()
                .is_none_or(|f| f.load(Ordering::SeqCst));
            match name {
                "server_sessions" if !in_world => ok(json!({ "count": 0, "sessions": [] })),
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
                "server_entity_get" => ok(json!({ "entity": {
                    "entity_id": args["entity_id"], "space_id": 3, "is_player": true,
                    "world": "Castle_CellBlock",
                }})),
                // The player, Frost (tagged) and a spawn with no tag; the
                // extra fields are the ones a snapshot drops.
                "server_entity_query" => ok(json!({
                    "returned": 3, "total_matched": 3, "capped": false,
                    "entities": [
                        { "entity_id": 7, "space_id": 3, "is_player": true, "tag": null,
                          "spawn_id": null, "template_id": null, "position": [0.0, 0.0, 0.0] },
                        { "entity_id": 100751, "space_id": 3, "is_player": false,
                          "tag": "ArmYourself_FrostBody", "spawn_id": null, "template_id": 14,
                          "entity_name": "Frost" },
                        { "entity_id": 100752, "space_id": 3, "is_player": false, "tag": null,
                          "spawn_id": 412, "template_id": 15 },
                    ]
                })),
                // lab-chaos (CH-05's replies; `chaos_tests` scripts refusals).
                "server_chaos_set" => ok(json!({ "set": true, "expires_in_s": args["ttl_s"] })),
                "server_chaos_clear" => ok(json!({ "cleared": 1 })),
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
