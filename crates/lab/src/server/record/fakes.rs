//! Fakes for the record tool tests: a scripted `cimmeria-lab-mcp` and a
//! client probe that checks the lease permit like the supervisor's bridge
//! calls, and [`Lab`], one test's instance.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use super::run::{start, Failure, Recorder, StartReq, RECORD_FILE};
use super::stop::{status, stop, StopReq};
use crate::lease::{permit, LeaseBook};
use crate::supervisor::record_slot::RecordSlot;
use crate::uat::invoke::{BoxedOutcome, ServerInvoker, ToolOutcome};
use crate::uat::record::emit::transform;
use crate::uat::record::log::{read_log, RecordLog};
use crate::uat::record::sampler::{BoxedJson, ClientProbe, SamplerConfig};

/// 2026-10-12 roughly; any fixed time works.
pub(super) const T0: i64 = 1_791_800_000_000;
/// A tap batch that makes the fake server panic when it is read.
pub(super) const PANIC: &str = "panic";

pub(super) fn ok(json: Value) -> ToolOutcome {
    ToolOutcome {
        ok: true,
        json,
        ..Default::default()
    }
}

/// The server: one player (`Labone`, player 42, entity 7) and a tap that
/// hands out `batches` one per read, then nothing.
#[derive(Default)]
pub(super) struct FakeServer {
    pub(super) calls: Mutex<Vec<String>>,
    pub(super) batches: Mutex<VecDeque<Value>>,
}

impl FakeServer {
    pub(super) fn with_tap(batches: Vec<Value>) -> Self {
        Self {
            batches: Mutex::new(batches.into()),
            ..Default::default()
        }
    }

    pub(super) fn count(&self, name: &str) -> usize {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|n| *n == name)
            .count()
    }

    pub(super) fn reply(&self, name: &str, args: &Value) -> ToolOutcome {
        match name {
            "server_sessions" => ok(json!({ "sessions": [
                { "entity_id": 7, "name": "Labone", "player_id": 42 },
            ]})),
            "server_db_query" => {
                let sql = args["sql"].as_str().unwrap_or_default();
                let rows = if sql.starts_with("SELECT player_id") {
                    if sql.ends_with("'Labone'") {
                        json!([{ "player_id": 42 }])
                    } else {
                        json!([])
                    }
                } else if sql.starts_with("SELECT mission_id") {
                    json!([{ "mission_id": 622, "status": 1, "step": 2113, "repeats": 0 }])
                } else if sql.starts_with("SELECT level") {
                    json!([{ "level": 1, "exp": 0, "naquadah": 0, "world_location": "Castle_CellBlock" }])
                } else {
                    json!([{ "type_id": 55, "container_id": 1, "count": 2, "name": "Medkit" }])
                };
                ok(json!({ "rows": rows }))
            }
            "server_packet_tap_start" => ok(json!({ "replaced_existing": false })),
            "server_packet_tap_read" => {
                let rows = self.batches.lock().unwrap().pop_front();
                // A scripted fault in the recorder's task (lock released).
                if rows.as_ref().is_some_and(|r| r == PANIC) {
                    panic!("fake server: scripted panic");
                }
                ok(json!({ "dropped": 0, "messages": rows.unwrap_or(json!([])) }))
            }
            "server_packet_tap_stop" => ok(json!({ "stopped": true })),
            "server_entity_get" => {
                let id = args["entity_id"].as_u64().unwrap();
                let (name, tag) = if id == 7 {
                    ("Labone", Value::Null)
                } else {
                    ("Frost", json!("ArmYourself_FrostBody"))
                };
                ok(json!({ "entity": {
                    "entity_id": id, "space_id": 3, "position": [-328.3, 73.472, -210.27],
                    "tag": tag, "template_id": 14, "entity_name": name,
                }}))
            }
            "server_log_tail" => ok(json!({ "entries": [], "newest_ms": 1000 })),
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
        self.calls.lock().unwrap().push(name.to_string());
        Box::pin(async move { out })
    }
}

/// Each read checks the task's lease permit first, as
/// `Supervisor::bridge_call` does, and counts the refusals.
#[derive(Default)]
pub(super) struct FakeClient {
    pub(super) refused: AtomicU32,
}

impl FakeClient {
    pub(super) fn read(&self, what: &str, v: Value) -> BoxedJson<'_> {
        let out = match permit::ensure(&format!("bridge {what}")) {
            Ok(()) => Ok(v),
            Err(e) => {
                self.refused.fetch_add(1, Ordering::SeqCst);
                Err(e)
            }
        };
        Box::pin(async move { out })
    }
}

impl ClientProbe for FakeClient {
    fn camera(&self) -> BoxedJson<'_> {
        self.read(
            "camera",
            json!({
                "zoom": 250.0, "pitch_offset_deg": -28.8, "yaw_gain": -20.0,
                "pose": { "server": { "x": -325.0, "y": 73.6, "z": -212.8 }, "yaw_deg": 12.5 },
            }),
        )
    }

    fn ui(&self) -> BoxedJson<'_> {
        self.read("ui", json!({ "windows": ["SelfStatusWin"] }))
    }

    fn pump(&self) -> BoxedJson<'_> {
        self.read("pump", json!({}))
    }
}

/// One test's instance: its slot, lease book, fakes, clock and folders.
pub(super) struct Lab {
    pub(super) slot: Arc<RecordSlot>,
    pub(super) book: Arc<LeaseBook>,
    pub(super) server: Arc<FakeServer>,
    pub(super) client: Arc<FakeClient>,
    pub(super) clock: Arc<AtomicI64>,
    /// The instance label (`p2`); a test may lengthen it.
    pub(super) instance: String,
    /// `<root>/cimmeria-lab/uat-runs/records`, laid out like the real UAT
    /// root so path lengths are realistic; a test may deepen it.
    pub(super) records: PathBuf,
    /// `<root>/docs/guides/uat-specs`, created (start checks it exists).
    pub(super) specs: PathBuf,
    pub(super) root: tempfile::TempDir,
}

impl Lab {
    pub(super) fn new(server: FakeServer) -> Self {
        let root = tempfile::tempdir().unwrap();
        let records = root
            .path()
            .join("cimmeria-lab")
            .join("uat-runs")
            .join("records");
        let specs = root.path().join("docs").join("guides").join("uat-specs");
        std::fs::create_dir_all(&specs).unwrap();
        Self {
            slot: Arc::default(),
            book: Arc::default(),
            server: Arc::new(server),
            client: Arc::default(),
            clock: Arc::new(AtomicI64::new(T0)),
            instance: "p2".into(),
            records,
            // As a caller passes it: start canonicalizes it (on Windows to a
            // `\\?\` path, which a caller may not pass).
            specs,
            root,
        }
    }

    pub(super) fn records(&self) -> PathBuf {
        self.records.clone()
    }

    pub(super) fn specs(&self) -> PathBuf {
        self.specs.clone()
    }

    /// A recording's folder.
    pub(super) fn raw(&self, id: &str) -> PathBuf {
        self.records.join(id)
    }

    /// A recording's log.
    pub(super) fn log(&self, id: &str) -> RecordLog {
        read_log(&std::fs::read_to_string(self.raw(id).join(RECORD_FILE)).unwrap()).unwrap()
    }

    pub(super) fn recorder(&self) -> Recorder {
        let clock = self.clock.clone();
        Recorder {
            slot: self.slot.clone(),
            book: self.book.clone(),
            server: self.server.clone(),
            client: self.client.clone(),
            instance: self.instance.clone(),
            records: self.records(),
            clock: Arc::new(move || clock.load(Ordering::SeqCst)),
            tick: Duration::from_millis(10),
            cfg: SamplerConfig::default(),
        }
    }

    /// The start this lab makes unless a test changes it.
    pub(super) fn start_req(&self) -> StartReq {
        StartReq {
            section: "first-session".into(),
            character: "Labone".into(),
            specs_dir: Some(self.specs()),
            max_minutes: None,
        }
    }

    pub(super) async fn start(&self, max_minutes: Option<u32>) -> Result<Value, Failure> {
        let req = StartReq {
            max_minutes,
            ..self.start_req()
        };
        start(self.recorder(), req).await
    }

    pub(super) async fn start_with(&self, req: StartReq) -> Result<Value, Failure> {
        start(self.recorder(), req).await
    }

    /// The stop this lab makes unless a test changes it.
    pub(super) fn stop_req(&self, id: &str) -> StopReq {
        StopReq {
            recording_id: id.into(),
            character_expr: None,
            profile: None,
            no_draft: false,
            specs_dir: None,
            instance: self.instance.clone(),
            wait: Duration::from_secs(5),
            transform,
        }
    }

    pub(super) async fn stop(&self, id: &str, no_draft: bool) -> Result<Value, Failure> {
        let req = StopReq {
            no_draft,
            ..self.stop_req(id)
        };
        stop(&self.slot, req).await
    }

    pub(super) async fn stop_with(&self, req: StopReq) -> Result<Value, Failure> {
        stop(&self.slot, req).await
    }

    pub(super) fn status(&self) -> Value {
        status(&self.slot, &self.instance, T0)
    }

    /// Wait (at most 5 s) until `f` holds.
    pub(super) async fn until(&self, what: &str, f: impl Fn(&Value) -> bool) {
        for _ in 0..1000 {
            if f(&self.status()) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("timed out waiting for {what}: {}", self.status());
    }
}

pub(super) fn id_of(v: &Value) -> String {
    v["recording_id"].as_str().unwrap().to_string()
}

/// Praxis record 19's `interact(Frost)`, an anchor.
pub(super) fn interact() -> Value {
    json!({ "dir": "in", "msg_id": 189, "method_index": 74, "msg_name": "interact",
            "args_hex": "000000000d8f890100" })
}
