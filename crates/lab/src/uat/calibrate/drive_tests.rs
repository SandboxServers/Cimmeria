//! SV-10 sweep tests over a client fake with geometry: `/gmgotoxyz` moves
//! the player, `client_camera` sets the view, and `client_hover_probe`
//! answers from where the player stands and how the camera points, so the
//! order of candidates, re-stands and confirmations is what the sweep
//! really did, not a canned reply queue. Time is paused: each stand costs
//! one 250 ms poll of virtual time.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};

use super::super::candidate::JSONL;
use super::*;
use crate::uat::invoke::{BoxedOutcome, ServerInvoker, ToolInvoker, ToolOutcome};
use crate::uat::runner::{LoadedSpec, RunRequest, Runner};
use crate::uat::spec;

const TAG: &str = "ArmYourself_FrostBody";
const FROST: [f64; 3] = [-328.30, 73.472, -210.27];
const FROST_ID: u32 = 100_751;
const PLAYER: u32 = 8;
/// The pitch the face lands on when a candidate keeps it.
const FACE_PITCH: f64 = -18.0;

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

struct Client {
    log: Mutex<Vec<(String, Value)>>,
    typed: Mutex<String>,
    at: Mutex<[f64; 3]>,
    view: Mutex<View>,
    visits: Mutex<HashMap<String, u32>>,
    hover: Hover,
    login: &'static str,
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

    /// Every `/gmgotoxyz` typed, as (radius, bearing) from the target.
    fn stands(&self) -> Vec<(f64, f64)> {
        self.log
            .lock()
            .unwrap()
            .iter()
            .filter(|(n, _)| n == "client_type_text")
            .filter_map(|(_, a)| a["text"].as_str().and_then(parse_goto))
            .map(polar)
            .collect()
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
        [
            "lab_client_status",
            "client_input_focus",
            "client_input_key",
            "client_type_text",
            "client_entity_find",
            "client_camera",
            "client_hover_probe",
            "client_world_click",
            "client_target",
            "client_input_mouse",
        ]
        .contains(&name)
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
                    *self.at.lock().unwrap() = p;
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
                let v = *self.view.lock().unwrap();
                let key = format!("{:.2}/{:.1}/{:.1}/{:.1}", v.r, v.b, v.yaw, v.pitch);
                let mut visits = self.visits.lock().unwrap();
                let n = visits.entry(key).or_insert(0);
                let reply = (self.hover)(&v, *n);
                *n += 1;
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

const FROST_LINE: &str = r#"  { id = "frost", tag = "ArmYourself_FrostBody", radius_m = 4.16, bearing_deg = 322.5, dy_m = 0.13, pitch_deg = -28.8, yaw_offset_deg = 25.3, zoom = 250.0 }, # FS-P3"#;

fn spec_text(account: &str) -> String {
    format!(
        r#"schema = 1
[section]
id = "cal"
system = "Calibration"
guide = "unified-uat.md#cal"
ledger = "lab-spec-vocab/README.md"
account = "{account}"

[[row]]
id = "FS-P3"
title = "frost"
expected = "Clicks Frost."
pose = [
{FROST_LINE}
]
step = [{{ tool = "@world_click", args = {{ pose = "frost" }}, label = "go" }}]
[[row.expect]]
id = "c"
text = "t"
source = "chat"
contains = "Welcome"
"#
    )
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
}

async fn run_with(client: &Client, account: &str, grid: Grid, budget: Duration) -> Run {
    let tmp = tempfile::tempdir().unwrap().keep();
    let dir = tmp.join("docs").join("guides").join("uat-specs");
    std::fs::create_dir_all(&dir).unwrap();
    let spec_path = dir.join("cal.toml");
    let original = spec_text(account);
    std::fs::write(&spec_path, &original).unwrap();
    let loaded = LoadedSpec {
        path: spec_path.display().to_string(),
        sha256: "0".into(),
        spec: spec::parse(&original).unwrap(),
    };
    let server = Server;
    let req = RunRequest {
        sections: vec![loaded.clone()],
        root: tmp.join("runs"),
        operator: "test".into(),
        no_settle: true,
        ..Default::default()
    };
    let runner = Runner::new(client, Some(&server as &dyn ServerInvoker), req).unwrap();
    let out_dir = tmp.join("out");
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
    Run {
        outcomes,
        out_dir,
        spec_path,
        original,
    }
}

async fn run(client: &Client) -> Run {
    run_with(client, "gm", small_grid(), DEFAULT_BUDGET).await
}

/// The frost pose the written spec copy loads.
fn written_pose(run: &Run) -> PoseSpec {
    let text = std::fs::read_to_string(run.out_dir.join("cal.toml")).unwrap();
    let spec = spec::parse(&text).unwrap();
    spec.rows[0].pose[0].clone()
}

#[tokio::test(start_paused = true)]
async fn the_best_hovering_candidate_wins_and_is_confirmed() {
    let client = Client::new(Box::new(
        |v, _| {
            if v.is(4.0, 0.0) {
                hits(8)
            } else {
                hits(5)
            }
        },
    ));
    let run = run(&client).await;
    let o = &run.outcomes[0];
    assert!(o.ok, "{o:?}");
    assert_eq!(o.confirmed, 3);
    // 8 * 1000 + 150 + 120 / 2.
    assert_eq!(o.score, Some(8210));
    assert_eq!(o.changed["bearing_deg"], json!([322.5, 0.0]));
    assert_eq!(o.changed["radius_m"], json!([4.16, 4.0]));
    assert_eq!(o.changed["pitch_deg"], json!([-28.8, -30.0]));
    assert_eq!(o.changed["yaw_offset_deg"], json!([25.3, 0.0]));
    assert!(o.changed.get("zoom").is_none(), "zoom did not change");
    // 9 sweep probes, then 3 confirmations.
    assert_eq!(o.probes, 12);
    let lines = run.jsonl();
    let sweep: Vec<_> = lines
        .iter()
        .filter(|l| l.get("confirm").is_none())
        .collect();
    let order: Vec<(f64, f64)> = sweep
        .iter()
        .map(|l| (l["r"].as_f64().unwrap(), l["b"].as_f64().unwrap()))
        .collect();
    assert_eq!(
        order,
        [
            (4.16, 322.5),
            (3.0, 0.0),
            (3.0, 90.0),
            (3.0, 180.0),
            (3.0, 270.0),
            (4.0, 0.0),
            (4.0, 90.0),
            (4.0, 180.0),
            (4.0, 270.0)
        ],
        "the grid's order"
    );
    assert!(lines.iter().all(|l| !l.to_string().contains("null")));
    // Each confirmation re-stands: away to the opposite bearing, then back.
    let stands = client.stands();
    assert_eq!(
        stands[9..],
        [
            (4.0, 180.0),
            (4.0, 0.0),
            (4.0, 180.0),
            (4.0, 0.0),
            (4.0, 180.0),
            (4.0, 0.0)
        ]
    );
    // The copy in the run folder loads with the winner; the spec is untouched.
    let p = written_pose(&run);
    assert_eq!((p.radius_m, p.bearing_deg), (4.0, 0.0));
    assert_eq!(
        (p.pitch_deg, p.yaw_offset_deg, p.zoom),
        (Some(-30.0), Some(0.0), Some(250.0))
    );
    assert_eq!(p.calibrated.as_deref(), Some("2026-10-12 3/3"));
    assert_eq!(
        std::fs::read_to_string(&run.spec_path).unwrap(),
        run.original
    );
    let diff_path = o.diff_path.as_deref().unwrap();
    assert!(
        Path::new(diff_path).starts_with(&run.out_dir),
        "{diff_path}"
    );
    let diff = std::fs::read_to_string(diff_path).unwrap();
    assert!(diff.starts_with(
        "--- a/docs/guides/uat-specs/cal.toml\n+++ b/docs/guides/uat-specs/cal.toml\n"
    ));
    assert_eq!(
        diff.lines()
            .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
            .count(),
        1
    );
    assert_eq!(
        o.spec_path.as_deref(),
        Some("docs/guides/uat-specs/cal.toml")
    );
    // D-SV12: one pose's outcome stays well under the 400-character budget.
    let text = serde_json::to_string(o).unwrap();
    assert!(text.len() < 400, "{} chars: {text}", text.len());
    assert!(!text.contains("null"), "{text}");
}

#[tokio::test(start_paused = true)]
async fn nothing_is_clicked() {
    let client = Client::new(Box::new(
        |v, _| {
            if v.is(4.0, 0.0) {
                hits(8)
            } else {
                hits(5)
            }
        },
    ));
    let run = run(&client).await;
    assert!(run.outcomes[0].ok);
    let names = client.names();
    for never in ["client_world_click", "client_target", "client_input_mouse"] {
        assert!(!names.iter().any(|n| n == never), "{never} was called");
    }
    // Only the sweep's own tools ran.
    for n in &names {
        assert!(
            [
                "lab_client_status",
                "client_input_focus",
                "client_input_key",
                "client_type_text",
                "client_entity_find",
                "client_camera",
                "client_hover_probe",
            ]
            .contains(&n.as_str()),
            "{n}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn a_failed_confirmation_falls_to_the_next_best() {
    // Candidate 5 hovers once in the sweep and then poorly; candidate 2 is
    // a steady 7.
    let client = Client::new(Box::new(|v, n| {
        if v.is(4.0, 0.0) {
            hits(if n == 0 { 8 } else { 3 })
        } else if v.is(3.0, 90.0) {
            hits(7)
        } else {
            hits(5)
        }
    }));
    let run = run(&client).await;
    let o = &run.outcomes[0];
    assert!(o.ok, "{o:?}");
    assert_eq!(o.confirmed, 3);
    assert_eq!(o.changed["bearing_deg"], json!([322.5, 90.0]));
    assert_eq!(o.changed["radius_m"], json!([4.16, 3.0]));
    let confirms: Vec<(u64, u64)> = run
        .jsonl()
        .iter()
        .filter_map(|l| Some((l["i"].as_u64()?, l.get("confirm")?.as_u64()?)))
        .collect();
    assert_eq!(confirms, [(5, 1), (2, 1), (2, 2), (2, 3)]);
    // 9 sweep probes, 1 failed confirmation, 3 passed.
    assert_eq!(o.probes, 13);
    let p = written_pose(&run);
    assert_eq!((p.radius_m, p.bearing_deg), (3.0, 90.0));
}

#[tokio::test(start_paused = true)]
async fn the_budget_stops_the_sweep_with_the_best_so_far() {
    // A zero budget: the first candidate (the current pose) runs, then the
    // sweep stops; the current pose hovers and is confirmed.
    let client = Client::new(Box::new(
        |v, _| {
            if v.is(4.16, 322.5) {
                hits(7)
            } else {
                hits(8)
            }
        },
    ));
    let run = run_with(&client, "gm", small_grid(), Duration::ZERO).await;
    let o = &run.outcomes[0];
    assert!(o.ok, "{o:?}");
    assert_eq!(o.score, Some(7210));
    assert_eq!(o.confirmed, 3);
    assert!(
        o.reason.as_deref().is_some_and(|r| r.starts_with("budget")),
        "{o:?}"
    );
    let sweep = run
        .jsonl()
        .into_iter()
        .filter(|l| l.get("confirm").is_none())
        .count();
    assert_eq!(sweep, 1, "only the first candidate was probed");
    // The winner is the pose as it stands: nothing to propose.
    assert!(o.changed.is_empty() && o.diff_path.is_none(), "{o:?}");
}

#[tokio::test(start_paused = true)]
async fn the_budget_is_checked_between_candidates_not_only_between_stands() {
    // Two pitches share each stand point: 0 is the current pose, 1 and 2
    // stand at (4, 0). Each stand costs one 250 ms poll, so 300 ms is spent
    // after candidate 1; candidate 2 needs no stand but must not run.
    let grid = Grid {
        radii_m: vec![4.0],
        bearing_step_deg: 360.0,
        yaw_offsets_deg: vec![0.0],
        pitches_deg: vec![Some(-30.0), Some(-20.0)],
    };
    let client = Client::new(Box::new(|_, _| hits(8)));
    let run = run_with(&client, "gm", grid, Duration::from_millis(300)).await;
    let o = &run.outcomes[0];
    let sweep: Vec<u64> = run
        .jsonl()
        .iter()
        .filter(|l| l.get("confirm").is_none())
        .filter_map(|l| l["i"].as_u64())
        .collect();
    assert_eq!(sweep, [0, 1], "{o:?}");
    assert_eq!(o.reason.as_deref(), Some("budget: swept 2 of 3 candidates"));
}

#[tokio::test(start_paused = true)]
async fn a_non_gm_section_is_refused_before_anything_moves() {
    let client = Client::new(Box::new(|_, _| hits(8)));
    let run = run_with(&client, "non-gm", small_grid(), DEFAULT_BUDGET).await;
    let o = &run.outcomes[0];
    assert!(!o.ok);
    assert_eq!(o.pose, "frost");
    assert!(
        o.reason
            .as_deref()
            .is_some_and(|r| r.contains("gm section")),
        "{o:?}"
    );
    assert!(client.names().is_empty(), "{:?}", client.names());
    assert!(!run.out_dir.join(JSONL).exists());
}

#[tokio::test(start_paused = true)]
async fn a_client_out_of_world_is_refused() {
    let mut client = Client::new(Box::new(|_, _| hits(8)));
    client.login = "character_select";
    let run = run(&client).await;
    let o = &run.outcomes[0];
    assert!(
        o.reason
            .as_deref()
            .is_some_and(|r| r.contains("not in world")),
        "{o:?}"
    );
    assert_eq!(client.names(), ["lab_client_status"]);
}

#[tokio::test(start_paused = true)]
async fn the_unchanged_pose_gives_no_diff() {
    // The current pose is the only one that hovers well.
    let client = Client::new(Box::new(
        |v, _| {
            if v.is(4.16, 322.5) {
                hits(8)
            } else {
                hits(4)
            }
        },
    ));
    let run = run(&client).await;
    let o = &run.outcomes[0];
    assert!(o.ok, "{o:?}");
    assert_eq!(o.confirmed, 3);
    assert!(o.changed.is_empty(), "{o:?}");
    assert!(o.diff_path.is_none() && o.spec_path.is_none());
    assert!(!run.out_dir.join("cal.diff").exists());
    assert!(!run.out_dir.join("cal.toml").exists());
}

#[tokio::test(start_paused = true)]
async fn nothing_hovering_says_so() {
    let client = Client::new(Box::new(
        |_, _| json!({ "verdict": "nothing", "pixel": [1, 1] }),
    ));
    let run = run(&client).await;
    let o = &run.outcomes[0];
    assert!(!o.ok);
    assert_eq!(
        o.reason.as_deref(),
        Some("no candidate hovered the target (9 probes)")
    );
    assert_eq!(o.confirmed, 0);
}

#[test]
fn the_diff_names_the_repo_relative_spec() {
    let win = r"C:\repo\docs\guides\uat-specs\first-session.toml";
    assert_eq!(
        repo_relative(win),
        "docs/guides/uat-specs/first-session.toml"
    );
    assert_eq!(
        repo_relative("docs/guides/uat-specs/a.toml"),
        "docs/guides/uat-specs/a.toml"
    );
    assert_eq!(repo_relative("/elsewhere/specs/a.toml"), "a.toml");
    assert_eq!(repo_relative("/x/mydocs/guides/uat-specs/a.toml"), "a.toml");
}
