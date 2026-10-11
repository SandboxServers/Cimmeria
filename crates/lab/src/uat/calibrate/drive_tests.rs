//! SV-10 sweep tests over a client fake with geometry: `/gmgotoxyz` moves
//! the player, `client_camera` sets the view, and `client_hover_probe`
//! answers from where the player stands and how the camera points, so the
//! order of candidates, re-stands and confirmations is what the sweep
//! really did, not a canned reply queue. Time is paused: each stand costs
//! one 250 ms poll of virtual time.
//!
//! This file holds the fake and the harness; the tests are in
//! `drive_sweep_tests.rs`, `drive_confirm_tests.rs` and
//! `drive_bounds_tests.rs`. Every sweep runs through [`run_spec`], which
//! checks after it that the client saw only the sweep's own tools (no
//! click) and only well-formed hover probes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::watch;

use super::super::candidate::JSONL;
use super::*;
use crate::uat::invoke::{BoxedOutcome, ServerInvoker, ToolInvoker, ToolOutcome};
use crate::uat::runner::{LoadedSpec, RunRequest, Runner};
use crate::uat::spec;

#[path = "drive_bounds_tests.rs"]
mod bounds;
#[path = "drive_confirm_tests.rs"]
mod confirms;
#[path = "drive_sweep_tests.rs"]
mod sweep;

const TAG: &str = "ArmYourself_FrostBody";
const FROST: [f64; 3] = [-328.30, 73.472, -210.27];
const FROST_ID: u32 = 100_751;
const PLAYER: u32 = 8;
/// The pitch the face lands on when a candidate keeps it.
const FACE_PITCH: f64 = -18.0;
const SECTION: &str = "first-session";

/// The only client tools a sweep may call.
const SWEEP_TOOLS: [&str; 7] = [
    "lab_client_status",
    "client_input_focus",
    "client_input_key",
    "client_type_text",
    "client_entity_find",
    "client_camera",
    "client_hover_probe",
];

/// Where the player stands and how the camera points, from the target.
#[derive(Debug, Clone, Copy, Default)]
struct View {
    r: f64,
    b: f64,
    yaw: f64,
    pitch: f64,
}

impl View {
    fn is(&self, r: f64, b: f64) -> bool {
        let db = (self.b - b).rem_euclid(360.0);
        (self.r - r).abs() < 0.01 && db.min(360.0 - db) < 0.1
    }
}

/// What the probe answers for a view, given how often that view was
/// probed before.
type Hover = Box<dyn Fn(&View, u32) -> Value + Send + Sync>;

fn hits(n: u32) -> Value {
    json!({ "verdict": "ok", "pixel": [640, 360], "hover": "entity", "jitter_hits": n,
            "jitter": 8, "avatar_px": 150.0, "edge_px": 120.0, "cursor_restored": true })
}

/// As the real probe answers when the avatar is off-screen: `avatar_px`
/// null, which scores as the 400 px cap.
fn far(n: u32) -> Value {
    let mut v = hits(n);
    v["avatar_px"] = Value::Null;
    v
}

struct Client {
    log: Mutex<Vec<(String, Value)>>,
    typed: Mutex<String>,
    at: Mutex<[f64; 3]>,
    view: Mutex<View>,
    visits: Mutex<HashMap<String, u32>>,
    hover: Hover,
    login: &'static str,
    /// Stand points (radius, bearing) a `/gmgotoxyz` never reaches.
    blocked: Vec<(f64, f64)>,
    /// Hover arguments `HoverRequest` would refuse.
    bad_hover: Mutex<Vec<String>>,
    /// Revoke the lease once this many probes have answered.
    revoke_after: Option<u32>,
    probes: Mutex<u32>,
    revoke: watch::Sender<Option<String>>,
}

impl Client {
    fn new(hover: Hover) -> Self {
        Self {
            log: Mutex::new(vec![]),
            typed: Mutex::new(String::new()),
            at: Mutex::new([0.0, 0.0, 0.0]),
            view: Mutex::new(View::default()),
            visits: Mutex::new(HashMap::new()),
            hover,
            login: "in_world",
            blocked: vec![],
            bad_hover: Mutex::new(vec![]),
            revoke_after: None,
            probes: Mutex::new(0),
            revoke: watch::channel(None).0,
        }
    }

    fn names(&self) -> Vec<String> {
        self.log
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.0.clone())
            .collect()
    }

    /// Every call to `tool`, its arguments in order.
    fn calls(&self, tool: &str) -> Vec<Value> {
        self.log
            .lock()
            .unwrap()
            .iter()
            .filter(|(n, _)| n == tool)
            .map(|(_, a)| a.clone())
            .collect()
    }

    /// Every `/gmgotoxyz` typed, as (radius, bearing) from the target.
    fn stands(&self) -> Vec<(f64, f64)> {
        self.calls("client_type_text")
            .iter()
            .filter_map(|a| a["text"].as_str().and_then(parse_goto))
            .map(polar)
            .collect()
    }

    /// Run after every sweep: nothing but the sweep's own tools, so no
    /// click, and no hover probe `HoverRequest` would have refused.
    fn assert_only_sweep_tools(&self) {
        for n in self.names() {
            assert!(SWEEP_TOOLS.contains(&n.as_str()), "the sweep called {n}");
        }
        let bad = self.bad_hover.lock().unwrap();
        assert!(bad.is_empty(), "bad hover args: {bad:?}");
    }

    /// `client_hover_probe`'s own argument rules (`HoverRequest`): exactly
    /// one of `entity_id` and `point`, and `expect_entity` only with `point`.
    fn hover_args_error(args: &Value) -> Option<&'static str> {
        let (e, p) = (args.get("entity_id"), args.get("point"));
        if e.is_some() && args.get("expect_entity").is_some() {
            return Some("expect_entity goes with point only");
        }
        (e.is_some() == p.is_some()).then_some("give exactly one of entity_id and point")
    }
}

fn parse_goto(line: &str) -> Option<[f64; 3]> {
    let rest = line.strip_prefix("/gmgotoxyz ")?;
    let v: Vec<f64> = rest.split(' ').filter_map(|s| s.parse().ok()).collect();
    (v.len() == 3).then(|| [v[0], v[1], v[2]])
}

/// (radius, bearing) of a server point from the target, 2 dp and 1 dp.
fn polar(p: [f64; 3]) -> (f64, f64) {
    let (dx, dz) = (p[0] - FROST[0], p[2] - FROST[2]);
    let b = dz.atan2(dx).to_degrees().rem_euclid(360.0);
    let b = if (b * 10.0).round() / 10.0 >= 360.0 {
        0.0
    } else {
        b
    };
    (
        (dx.hypot(dz) * 100.0).round() / 100.0,
        (b * 10.0).round() / 10.0,
    )
}

impl ToolInvoker for Client {
    fn has_tool(&self, name: &str) -> bool {
        // The click tools are routed, so "never called" is the sweep's
        // choice, not a missing tool.
        SWEEP_TOOLS.contains(&name)
            || ["client_world_click", "client_target", "client_input_mouse"].contains(&name)
    }

    fn tool_names(&self) -> Vec<String> {
        vec![]
    }

    async fn call(&self, name: &str, args: Value) -> ToolOutcome {
        self.log
            .lock()
            .unwrap()
            .push((name.to_string(), args.clone()));
        let ok = |json: Value| ToolOutcome {
            ok: true,
            json,
            ..Default::default()
        };
        match name {
            "lab_client_status" => ok(json!({ "running": true, "login_state": self.login })),
            "client_type_text" => {
                *self.typed.lock().unwrap() = args["text"].as_str().unwrap_or("").into();
                ok(json!({}))
            }
            "client_input_key" => {
                let line = std::mem::take(&mut *self.typed.lock().unwrap());
                if let Some(p) = parse_goto(&line) {
                    let (r, b) = polar(p);
                    let blocked = self.blocked.iter().any(|&(br, bb)| {
                        View {
                            r,
                            b,
                            ..View::default()
                        }
                        .is(br, bb)
                    });
                    if !blocked {
                        *self.at.lock().unwrap() = p;
                    }
                }
                ok(json!({}))
            }
            "client_input_focus" => ok(json!({})),
            "client_entity_find" => {
                let p = *self.at.lock().unwrap();
                ok(json!({ "player": { "id": PLAYER, "position": {
                    "server": { "x": p[0], "y": p[1], "z": p[2] } } } }))
            }
            "client_camera" => {
                let (r, b) = polar(*self.at.lock().unwrap());
                let pitch = args["pitch_deg"].as_f64();
                let view = View {
                    r,
                    b,
                    yaw: args["yaw_offset_deg"].as_f64().unwrap_or(0.0),
                    pitch: pitch.unwrap_or(FACE_PITCH),
                };
                *self.view.lock().unwrap() = view;
                let mut out = json!({ "view_after": { "pitch_offset_deg": view.pitch } });
                if let Some(p) = pitch {
                    // Within the camera's 0.5 degree tolerance, not exact.
                    out["absolute"] = json!({ "pitch": { "want": p, "got": p + 0.2, "looks": 1 } });
                }
                ok(out)
            }
            "client_hover_probe" => {
                if let Some(e) = Self::hover_args_error(&args) {
                    self.bad_hover.lock().unwrap().push(format!("{e}: {args}"));
                    return ToolOutcome::err(e);
                }
                let v = *self.view.lock().unwrap();
                let key = format!("{:.2}/{:.1}/{:.1}/{:.1}", v.r, v.b, v.yaw, v.pitch);
                let reply = {
                    let mut visits = self.visits.lock().unwrap();
                    let n = visits.entry(key).or_insert(0);
                    let reply = (self.hover)(&v, *n);
                    *n += 1;
                    reply
                };
                let mut probes = self.probes.lock().unwrap();
                *probes += 1;
                if self.revoke_after == Some(*probes) {
                    self.revoke
                        .send_replace(Some("taken over by session-b".into()));
                }
                ok(reply)
            }
            other => ToolOutcome::err(format!("the sweep must not call {other}")),
        }
    }
}

/// The lab endpoint: the player in space 3, Frost the one tagged body.
struct Server;

impl ServerInvoker for Server {
    fn url(&self) -> &str {
        "http://127.0.0.1:1/mcp"
    }

    fn call<'a>(&'a self, name: &'a str, _args: Value) -> BoxedOutcome<'a> {
        let json = match name {
            "server_entity_get" => json!({ "entity": { "entity_id": PLAYER, "space_id": 3 } }),
            "server_entity_query" => json!({ "capped": false, "entities": [
                { "entity_id": FROST_ID, "space_id": 3, "tag": TAG, "position": FROST },
            ]}),
            _ => return Box::pin(async { ToolOutcome::err("unknown") }),
        };
        Box::pin(async move {
            ToolOutcome {
                ok: true,
                json,
                ..Default::default()
            }
        })
    }
}

/// The frost pose line as the spec holds it, under another pose id.
fn frost_line(id: &str, extra: &str) -> String {
    format!(
        r#"  {{ id = "{id}", tag = "{TAG}", radius_m = 4.16, bearing_deg = 322.5, dy_m = 0.13, pitch_deg = -28.8, yaw_offset_deg = 25.3, zoom = 250.0{extra} }}, # FS-P3"#
    )
}

/// A section with one row whose `@world_click` steps name `poses`, in
/// order; each pose is the frost pose line under that id.
fn spec_with(account: &str, poses: &[&str], extra: &str) -> String {
    let lines: Vec<String> = poses.iter().map(|p| frost_line(p, extra)).collect();
    let steps: Vec<String> = poses
        .iter()
        .map(|p| {
            format!(r#"{{ tool = "@world_click", args = {{ pose = "{p}" }}, label = "go {p}" }}"#)
        })
        .collect();
    format!(
        r#"schema = 1
[section]
id = "{SECTION}"
system = "Calibration"
guide = "unified-uat.md#first-session"
ledger = "lab-spec-vocab/README.md"
account = "{account}"

[[row]]
id = "FS-P3"
title = "frost"
expected = "Clicks Frost."
pose = [
{}
]
step = [{}]
[[row.expect]]
id = "c"
text = "t"
source = "chat"
contains = "Welcome"
"#,
        lines.join("\n"),
        steps.join(", ")
    )
}

fn spec_text(account: &str) -> String {
    spec_with(account, &["frost"], "")
}

/// 2 radii x 4 bearings, one yaw, one pitch, after the current pose:
/// 0 is the current pose, 1-4 are 3 m at 0/90/180/270, 5-8 are 4 m.
fn small_grid() -> Grid {
    Grid {
        radii_m: vec![3.0, 4.0],
        bearing_step_deg: 90.0,
        yaw_offsets_deg: vec![0.0],
        pitches_deg: vec![Some(-30.0)],
    }
}

struct Run {
    outcomes: Vec<CalibrateOutcome>,
    out_dir: PathBuf,
    spec_path: PathBuf,
    original: String,
}

impl Run {
    /// The candidate log's lines.
    fn jsonl(&self) -> Vec<Value> {
        std::fs::read_to_string(self.out_dir.join(JSONL))
            .unwrap_or_default()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// The candidate log's sweep lines (no confirmations).
    fn sweep(&self) -> Vec<Value> {
        self.jsonl()
            .into_iter()
            .filter(|l| l.get("confirm").is_none())
            .collect()
    }

    /// The proposed diff's text.
    fn diff(&self) -> String {
        std::fs::read_to_string(self.out_dir.join(format!("{SECTION}.diff"))).unwrap()
    }

    /// The spec copy the sweep wrote, parsed.
    fn written(&self) -> spec::SectionSpec {
        let text = std::fs::read_to_string(self.out_dir.join(format!("{SECTION}.toml"))).unwrap();
        spec::parse(&text).unwrap()
    }
}

/// A sweep of `text`'s row FS-P3 on `client`, with the client's own
/// revocation signal; then the no-click and hover-argument checks.
async fn run_spec(client: &Client, text: String, grid: Grid, budget: Duration) -> Run {
    let tmp = tempfile::tempdir().unwrap().keep();
    let dir = tmp.join("docs").join("guides").join("uat-specs");
    std::fs::create_dir_all(&dir).unwrap();
    let spec_path = dir.join(format!("{SECTION}.toml"));
    std::fs::write(&spec_path, &text).unwrap();
    let loaded = LoadedSpec {
        path: spec_path.display().to_string(),
        sha256: "0".into(),
        spec: spec::parse(&text).unwrap(),
    };
    let server = Server;
    let req = RunRequest {
        sections: vec![loaded.clone()],
        root: tmp.join("runs"),
        operator: "test".into(),
        no_settle: true,
        ..Default::default()
    };
    let runner = Runner::new(client, Some(&server as &dyn ServerInvoker), req)
        .unwrap()
        .with_revocation(client.revoke.subscribe());
    // As long as a real run folder's path, so the D-SV12 size checks bite.
    let out_dir = tmp
        .join("AppData")
        .join("Local")
        .join("cimmeria-lab")
        .join("uat-runs")
        .join("2026-10-11-1489a731-kqzxwbt");
    let req = CalibrateRequest {
        spec: loaded,
        row: "FS-P3".into(),
        pose: None,
        grid,
        budget,
        out_dir: out_dir.clone(),
        stamp: "2026-10-12".into(),
    };
    let outcomes = calibrate(&runner, &req).await;
    client.assert_only_sweep_tools();
    Run {
        outcomes,
        out_dir,
        spec_path,
        original: text,
    }
}

async fn run_with(client: &Client, account: &str, grid: Grid, budget: Duration) -> Run {
    run_spec(client, spec_text(account), grid, budget).await
}

async fn run(client: &Client) -> Run {
    run_with(client, "gm", small_grid(), DEFAULT_BUDGET).await
}

/// The frost pose the written spec copy loads.
fn written_pose(run: &Run) -> PoseSpec {
    run.written().rows[0].pose[0].clone()
}

/// The count of added pose lines in a diff.
fn added_lines(diff: &str) -> usize {
    diff.lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .count()
}

/// The candidate log's (i, confirm) pairs, in order.
fn confirms(run: &Run) -> Vec<(u64, u64)> {
    run.jsonl()
        .iter()
        .filter_map(|l| Some((l["i"].as_u64()?, l.get("confirm")?.as_u64()?)))
        .collect()
}
