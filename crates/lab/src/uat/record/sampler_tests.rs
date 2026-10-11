//! Sampler tests over a scripted `cimmeria-lab-mcp` and a counting client
//! probe: no lab, no client. The server fake refuses any `client_*` name,
//! so a read routed through a tool router shows up as a failure.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::*;
use crate::uat::invoke::{BoxedOutcome, ToolOutcome};

const FROST: u32 = 100751;

#[derive(Default)]
struct FakeServer {
    calls: Mutex<Vec<(String, Value)>>,
    /// Scripted replies per tool, popped in order; the last one repeats.
    canned: Mutex<HashMap<String, VecDeque<ToolOutcome>>>,
    /// Entities `server_entity_get` reports as gone (null).
    gone: Mutex<HashSet<u32>>,
    /// Entities whose `server_entity_get` fails this many more times.
    flaky: Mutex<HashMap<u32, u32>>,
    /// `entity_name` overrides (a recycled player id).
    renamed: Mutex<HashMap<u32, String>>,
}

fn ok(json: Value) -> ToolOutcome {
    ToolOutcome {
        ok: true,
        json,
        ..Default::default()
    }
}

impl FakeServer {
    fn answer(self, tool: &str, replies: Vec<ToolOutcome>) -> Self {
        self.canned
            .lock()
            .unwrap()
            .insert(tool.to_string(), replies.into());
        self
    }

    fn calls(&self) -> Vec<(String, Value)> {
        self.calls.lock().unwrap().clone()
    }

    fn names(&self) -> Vec<String> {
        self.calls().into_iter().map(|c| c.0).collect()
    }

    fn count(&self, name: &str) -> usize {
        self.names().iter().filter(|n| *n == name).count()
    }

    fn reply(&self, name: &str, args: &Value) -> ToolOutcome {
        if name.starts_with("client_") {
            return ToolOutcome::err(format!("the sampler called client tool {name}"));
        }
        if let Some(q) = self.canned.lock().unwrap().get_mut(name) {
            if let Some(r) = if q.len() > 1 {
                q.pop_front()
            } else {
                q.front().cloned()
            } {
                return r;
            }
        }
        match name {
            "server_sessions" => ok(json!({ "count": 2, "sessions": [
                { "entity_id": 7, "name": "Labone", "player_id": 42 },
                { "entity_id": 9, "name": "Labtwo", "player_id": 43 },
            ]})),
            "server_db_query" => {
                let sql = args["sql"].as_str().unwrap_or_default();
                let rows = if sql.starts_with("SELECT player_id") {
                    json!([{ "player_id": 42 }])
                } else if sql.starts_with("SELECT mission_id") {
                    json!([{ "mission_id": 622, "status": 1, "step": 2113, "repeats": 0 }])
                } else if sql.starts_with("SELECT level") {
                    json!([{ "level": 1, "exp": 0, "naquadah": 0, "world_location": "Castle_CellBlock" }])
                } else {
                    json!([
                        { "type_id": 55, "container_id": 1, "count": 2, "name": "Medkit" },
                        { "type_id": null, "container_id": 1, "count": 1, "name": null },
                    ])
                };
                ok(json!({ "row_count": 1, "rows": rows }))
            }
            "server_packet_tap_start" => ok(json!({
                "entity_id": args["entity_id"], "replaced_existing": false,
            })),
            "server_packet_tap_read" => ok(json!({ "messages": [], "dropped": 0 })),
            "server_packet_tap_stop" => ok(json!({ "stopped": true })),
            "server_entity_get" => {
                let id = args["entity_id"].as_u64().unwrap() as u32;
                if let Some(n) = self.flaky.lock().unwrap().get_mut(&id).filter(|n| **n > 0) {
                    *n -= 1;
                    return ToolOutcome::err("lab-mcp: operation timed out");
                }
                if self.gone.lock().unwrap().contains(&id) {
                    return ok(json!({ "entity": null }));
                }
                let tag = (id == FROST).then_some("ArmYourself_FrostBody");
                // The player's entities (7, then 11 after a relog) carry
                // the character's name; everyone else is Frost.
                let default = if matches!(id, 7 | 11) {
                    "Labone"
                } else {
                    "Frost"
                };
                let name = self
                    .renamed
                    .lock()
                    .unwrap()
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| default.to_string());
                ok(json!({ "entity": {
                    "entity_id": id, "space_id": 3, "position": [-328.3, 73.472, -210.27],
                    "tag": tag, "template_id": 14, "entity_name": name,
                }}))
            }
            "server_log_tail" => ok(json!({ "count": 0, "entries": [], "newest_ms": 1000 })),
            _ => ToolOutcome::err(format!("unknown tool {name}")),
        }
    }
}

impl ServerInvoker for FakeServer {
    fn url(&self) -> &str {
        "http://127.0.0.1:1/mcp"
    }

    fn call<'a>(&'a self, name: &'a str, args: Value) -> BoxedOutcome<'a> {
        let out = self.reply(name, &args);
        self.calls.lock().unwrap().push((name.to_string(), args));
        Box::pin(async move { out })
    }
}

#[derive(Default)]
struct FakeClient {
    camera: AtomicU32,
    ui: AtomicU32,
    pump: AtomicU32,
    camera_fails: AtomicBool,
    /// The readout has only the unsigned `gain` (no SV-05 `yaw_gain`).
    no_yaw_gain: AtomicBool,
    ui_fails: AtomicBool,
    pump_fails: AtomicBool,
    pump_lua_error: AtomicBool,
    /// `clicks()` replies, in order; `{"clicks": []}` once used up.
    clicks: Mutex<VecDeque<Value>>,
}

impl ClientProbe for FakeClient {
    fn camera(&self) -> BoxedJson<'_> {
        self.camera.fetch_add(1, Ordering::SeqCst);
        let out = if self.camera_fails.load(Ordering::SeqCst) {
            Err("camera chain did not resolve: player controller not found in the actor list, which is long".repeat(2))
        } else if self.no_yaw_gain.load(Ordering::SeqCst) {
            Ok(json!({
                "zoom": 250.0, "pitch_offset_deg": -28.8, "gain": 20.0,
                "pose": { "server": { "x": -325.0, "y": 73.6, "z": -212.8 }, "yaw_deg": 12.5 },
            }))
        } else {
            Ok(json!({
                "zoom": 250.0, "pitch_offset_deg": -28.8, "gain": 20.0, "yaw_gain": -20.0,
                "pose": { "server": { "x": -325.0, "y": 73.6, "z": -212.8 }, "yaw_deg": 12.5 },
            }))
        };
        Box::pin(async move { out })
    }

    fn ui(&self) -> BoxedJson<'_> {
        self.ui.fetch_add(1, Ordering::SeqCst);
        let fails = self.ui_fails.load(Ordering::SeqCst);
        Box::pin(async move {
            if fails {
                return Err("bridge: not connected".to_string());
            }
            Ok(json!({
                "windows": ["SelfStatusWin", "DialogWin"],
                "dialog": { "title": "Frost", "buttons": [
                    { "window": "Dialog_AcceptButton", "label": "Accept" },
                    { "window": "Dialog_NextButton", "label": "" },
                ]},
                "errors": ["prompts: not loaded"],
            }))
        })
    }

    fn pump(&self) -> BoxedJson<'_> {
        self.pump.fetch_add(1, Ordering::SeqCst);
        let out = if self.pump_fails.load(Ordering::SeqCst) {
            Err("bridge: dispatch timeout".to_string())
        } else if self.pump_lua_error.load(Ordering::SeqCst) {
            Ok(json!({ "lua_error": "attempt to index a nil value" }))
        } else {
            Ok(json!({ "lua_error": null }))
        };
        Box::pin(async move { out })
    }

    fn clicks(&self) -> BoxedJson<'_> {
        let next = self.clicks.lock().unwrap().pop_front();
        Box::pin(async move { Ok(next.unwrap_or_else(|| json!({ "clicks": [] }))) })
    }
}

fn interact_tap() -> ToolOutcome {
    // Praxis record 19: interact(Frost) on sub-slot 0xBD, then the dialog.
    ok(json!({ "dropped": 0, "messages": [
        { "ts_ms": 1, "dir": "in", "msg_id": 189, "method_index": 74, "msg_name": "interact",
          "target_entity_id": null, "args_hex": "000000000d8f890100", "decoded": null },
        { "ts_ms": 2, "dir": "in", "msg_id": 189, "method_index": 5, "msg_name": "avatarUpdateExplicit",
          "target_entity_id": null, "args_hex": "00", "decoded": null },
        { "ts_ms": 3, "dir": "out", "msg_id": null, "method_index": 105, "msg_name": "onDialogDisplay",
          "target_entity_id": 8, "args_hex": "00", "decoded": { "dialog_id": 3995 } },
    ]}))
}

fn quiet_tap() -> ToolOutcome {
    ok(json!({ "dropped": 0, "messages": [
        { "ts_ms": 9, "dir": "in", "msg_name": "perfStats", "args_hex": "00" },
    ]}))
}

async fn begin<'a>(
    server: &'a FakeServer,
    client: &'a FakeClient,
) -> (Sampler<'a>, RecordStart, Tick) {
    begin_as(server, client, "Labone").await.unwrap()
}

async fn begin_as<'a>(
    server: &'a FakeServer,
    client: &'a FakeClient,
    character: &str,
) -> Result<(Sampler<'a>, RecordStart, Tick), String> {
    Sampler::begin(
        server,
        client,
        SamplerConfig::default(),
        Arc::default(),
        "first-session",
        "p2",
        character,
        1_000,
        "2026-10-12",
    )
    .await
}

#[tokio::test]
async fn begin_resolves_the_player_and_starts_a_10000_tap() {
    let server = FakeServer::default();
    let client = FakeClient::default();
    let (s, start, t0) = begin(&server, &client).await;
    assert_eq!(start.player_id, 42);
    assert_eq!(start.player_entity, 7);
    assert_eq!(start.character, "Labone");
    assert!(
        start.recording_id.ends_with("-p2"),
        "{}",
        start.recording_id
    );
    assert_eq!(s.player_entity(), 7);

    let calls = server.calls();
    let names: Vec<&str> = calls.iter().map(|c| c.0.as_str()).collect();
    assert_eq!(
        names[..3],
        [
            "server_db_query",
            "server_sessions",
            "server_packet_tap_start"
        ]
    );
    assert_eq!(
        calls[0].1["sql"],
        "SELECT player_id FROM sgw_player WHERE player_name = 'Labone'"
    );
    assert_eq!(calls[2].1, json!({ "entity_id": 7, "capacity": 10000 }));

    // Tick 0 is a full sample with the first snapshot, pumped once.
    assert_eq!(t0.seq, 0);
    let snap = t0.snapshot.expect("tick 0 snapshots");
    assert_eq!(snap.missions[0].step, Some(2113));
    assert_eq!(snap.player.unwrap().world_location, "Castle_CellBlock");
    // The row with a null type_id is skipped, not a failed snapshot.
    assert_eq!(snap.items.len(), 1);
    assert_eq!(client.pump.load(Ordering::SeqCst), 1);
    // Every snapshot statement is keyed by the integer player_id.
    for (name, args) in &calls {
        if name == "server_db_query" && !args["sql"].as_str().unwrap().contains("player_name") {
            assert!(args["sql"].as_str().unwrap().contains("= 42"), "{args}");
        }
    }
}

#[tokio::test]
async fn a_tick_drains_samples_and_snapshots_after_activity() {
    let server = FakeServer::default().answer(
        "server_packet_tap_read",
        vec![ok(json!({ "messages": [] })), interact_tap(), quiet_tap()],
    );
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;

    let t = s.tick(1_500).await;
    assert_eq!(t.seq, 1);
    assert_eq!(t.tap.len(), 3);
    let cam = t.camera.as_ref().unwrap();
    assert_eq!(
        cam.yaw_gain, -20.0,
        "SV-05's signed yaw_gain wins over gain"
    );
    assert_eq!(cam.pos, [-325.0, 73.6, -212.8]);
    assert_eq!(t.player.as_ref().unwrap().entity_id, 7);
    assert_eq!(
        t.windows.as_deref().unwrap(),
        ["DialogWin".to_string(), "SelfStatusWin".to_string()]
    );
    assert_eq!(t.dialog.as_ref().unwrap().buttons, ["Accept"]);
    assert_eq!(t.entities.len(), 1);
    assert_eq!(t.entities[0].entity_id, FROST);
    assert_eq!(t.entities[0].tag.as_deref(), Some("ArmYourself_FrostBody"));
    assert!(t.snapshot.is_some());
    assert!(t.errors.is_empty(), "{:?}", t.errors);

    // Ten quiet ticks keep snapshotting (perfStats is noise), the eleventh
    // does not; Frost is never fetched again.
    for i in 0..10 {
        let q = s.tick(2_000 + i * 500).await;
        assert!(q.snapshot.is_some(), "quiet tick {i} is in the tail");
        assert!(q.entities.is_empty());
    }
    let q = s.tick(7_500).await;
    assert!(q.snapshot.is_none(), "the eleventh quiet tick");
    let frost_reads = server
        .calls()
        .iter()
        .filter(|c| c.0 == "server_entity_get" && c.1["entity_id"] == FROST)
        .count();
    assert_eq!(frost_reads, 1);
}

#[tokio::test]
async fn the_sampler_never_calls_a_client_tool() {
    let server = FakeServer::default().answer("server_packet_tap_read", vec![interact_tap()]);
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;
    for i in 0..4 {
        s.tick(1_500 + i * 500).await;
    }
    let (last, stop) = s.finish(4_000, "stop").await;
    assert_eq!(stop.ticks, 6);
    assert!(last.errors.is_empty(), "{:?}", last.errors);

    let names = server.names();
    assert!(
        names.iter().all(|n| n.starts_with("server_")),
        "only server_* tools: {names:?}"
    );
    // Every client read went through the probe: one of each per tick,
    // including tick 0 and the final one.
    for (what, n) in [
        ("camera", &client.camera),
        ("ui", &client.ui),
        ("pump", &client.pump),
    ] {
        assert_eq!(n.load(Ordering::SeqCst), 6, "{what}");
    }
    assert_eq!(server.count("server_packet_tap_stop"), 1);
}

#[tokio::test]
async fn a_failed_read_is_an_error_entry_not_a_stop() {
    let server = FakeServer::default()
        .answer(
            "server_packet_tap_read",
            vec![ToolOutcome::err("lab-mcp: operation timed out")],
        )
        .answer(
            "server_db_query",
            vec![
                ok(json!({ "rows": [{ "player_id": 42 }] })),
                ToolOutcome::err("database not connected"),
            ],
        );
    let client = FakeClient::default();
    let (mut s, _, t0) = begin(&server, &client).await;
    assert!(t0.snapshot.is_none());
    client.camera_fails.store(true, Ordering::SeqCst);

    let t = s.tick(1_500).await;
    assert!(t.camera.is_none());
    assert!(t.tap.is_empty());
    // Still sampled what it could.
    assert!(t.player.is_some());
    assert!(t.windows.is_some());
    assert!(t
        .errors
        .iter()
        .any(|e| e.starts_with("server_packet_tap_read:")));
    assert!(t.errors.iter().any(|e| e.starts_with("client: camera:")));
    assert!(t.errors.len() <= 4);
    assert!(t.errors.iter().all(|e| e.chars().count() <= 120));
    assert!(s.full().is_none());
    // The next tick runs as normal.
    assert_eq!(s.tick(2_000).await.seq, 2);
}

#[tokio::test]
async fn a_relog_restarts_the_tap() {
    let server = FakeServer::default().answer(
        "server_sessions",
        vec![
            ok(json!({ "sessions": [{ "entity_id": 7, "name": "Labone", "player_id": 42 }] })),
            ok(json!({ "sessions": [{ "entity_id": 11, "name": "Labone", "player_id": 42 }] })),
        ],
    );
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;
    server.gone.lock().unwrap().insert(7);

    let t = s.tick(1_500).await;
    assert!(t.player.is_none());
    assert!(
        t.errors.iter().any(|e| e == "relog: tap restarted on 11"),
        "{:?}",
        t.errors
    );
    assert_eq!(s.player_entity(), 11);
    let starts: Vec<Value> = server
        .calls()
        .into_iter()
        .filter(|c| c.0 == "server_packet_tap_start")
        .map(|c| c.1)
        .collect();
    assert_eq!(starts.last().unwrap()["entity_id"], 11);
    // The old entity's tap is stopped, best effort.
    let stops: Vec<Value> = server
        .calls()
        .into_iter()
        .filter(|c| c.0 == "server_packet_tap_stop")
        .map(|c| c.1)
        .collect();
    assert_eq!(stops, [json!({ "entity_id": 7 })]);

    let t = s.tick(2_000).await;
    assert_eq!(t.player.unwrap().entity_id, 11);
    let last_read = server
        .calls()
        .into_iter()
        .rfind(|c| c.0 == "server_packet_tap_read")
        .unwrap();
    assert_eq!(last_read.1["entity_id"], 11);
}

#[tokio::test]
async fn a_bad_character_name_is_refused_before_any_sql() {
    // The session list holds the hostile name, so only the guard stops
    // it reaching the player_id query.
    let server = FakeServer::default().answer(
        "server_sessions",
        vec![ok(
            json!({ "sessions": [{ "entity_id": 7, "name": "Px'; DROP" }] }),
        )],
    );
    let client = FakeClient::default();
    let err = begin_as(&server, &client, "Px'; DROP")
        .await
        .err()
        .expect("refused");
    assert!(err.contains("Px'; DROP"), "{err}");
    assert_eq!(server.count("server_db_query"), 0);
    assert!(server.names().is_empty(), "{:?}", server.names());

    assert!(check_character("Uat Dqzkfma").is_ok());
    assert!(check_character(&"A".repeat(64)).is_ok());
    for bad in ["", " Lead", "Px1", "A".repeat(65).as_str(), "Zoë"] {
        assert!(check_character(bad).is_err(), "{bad:?}");
    }
}

#[path = "sampler_budget_tests.rs"]
mod budget;
#[path = "sampler_poll_tests.rs"]
mod poll;
#[path = "sampler_relog_tests.rs"]
mod relog;
