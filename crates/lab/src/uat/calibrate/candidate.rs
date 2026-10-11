//! Driving one calibration candidate (SV-10): stand at its point, turn the
//! camera onto the target, hover-probe it, and the D-SV6 re-stand
//! confirmations; each probe becomes one line of the candidate log.
//!
//! The only client tools here are the `/gmgotoxyz` chat keys and position
//! reads (through the runner's `stand_at`), `client_camera` and
//! `client_hover_probe`. None of them presses a mouse button.

use std::io::Write as _;
use std::path::Path;

use serde_json::{json, Value};

use super::grid::Candidate;
use super::score::{self, CONFIRM_MIN_HITS, CONFIRM_RUNS};
use crate::uat::evidence::ActionRecord;
use crate::uat::invoke::ToolInvoker;
use crate::uat::runner::{ResolvedTag, Runner, Who};
use crate::uat::spec::PoseSpec;
use crate::uat::tier::Role;

/// The candidate log in the run folder (D-SV12: never in the result).
pub(super) const JSONL: &str = "calibrate.jsonl";
const CAMERA_TOOL: &str = "client_camera";
const HOVER_TOOL: &str = "client_hover_probe";

/// What one probe of one candidate gave.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Sample {
    pub(super) ok: bool,
    pub(super) hits: u32,
    pub(super) score: Option<u32>,
    /// The camera's pitch after the call (the face's, when the candidate
    /// kept it).
    pub(super) pitch: Option<f64>,
    /// Which step failed, for a miss that never reached a probe verdict.
    pub(super) miss: Option<&'static str>,
}

impl Sample {
    pub(super) fn missed(at: &'static str) -> Self {
        Sample {
            miss: Some(at),
            ..Sample::default()
        }
    }
}

/// The pose's target, as the camera and the probe aim at it.
pub(super) struct Target {
    pub(super) resolved: ResolvedTag,
    /// `aim = "point"`: the world point; else the entity itself.
    pub(super) point: Option<[f64; 3]>,
    pub(super) zoom: f64,
}

/// The stand point last reached (true) or failed (false), keyed by
/// [`stand_key`].
pub(super) type Stood = Option<((u64, u64), bool)>;

/// A candidate's stand point: its radius and bearing, bit for bit.
pub(super) fn stand_key(c: &Candidate) -> (u64, u64) {
    (c.radius_m.to_bits(), c.bearing_deg.to_bits())
}

fn point_json(p: [f64; 3]) -> Value {
    json!({ "x": p[0], "y": p[1], "z": p[2] })
}

/// A record the runner's target helpers write their calls into; the sweep
/// keeps its own log ([`JSONL`]) instead.
pub(super) fn scratch() -> ActionRecord {
    ActionRecord {
        index: 0,
        role: Role::Step,
        label: None,
        kind: "calibrate".into(),
        requested: String::new(),
        tool: None,
        args: Value::Null,
        tier: None,
        tier_source: None,
        fallback_used: false,
        host_started_ms: 0,
        elapsed_ms: 0,
        ok: true,
        error: None,
        result: Value::Null,
        calls: vec![],
        client: None,
    }
}

/// `v` to `places` decimals, as the pose line and the log show it.
pub(super) fn round(v: f64, places: i32) -> f64 {
    let k = 10f64.powi(places);
    (v * k).round() / k
}

/// Append one compact line to the candidate log. A log that cannot be
/// written loses only the log, not the sweep.
pub(super) fn log_line(out_dir: &Path, line: &Value) {
    let path = out_dir.join(JSONL);
    let res = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| writeln!(f, "{line}"));
    if let Err(e) = res {
        tracing::warn!(target: "lab.calibrate", path = %path.display(), error = %e,
            "calibrate: candidate log not written");
    }
}

/// `{pose, i, r, b, yaw, pitch, ok, hits, score}` with no nulls; `confirm`
/// numbers a confirmation re-probe, `miss` names the step a miss died at.
pub(super) fn sample_line(
    pose: &str,
    i: usize,
    c: &Candidate,
    s: &Sample,
    confirm: Option<u32>,
) -> Value {
    let mut v = json!({
        "pose": pose, "i": i, "r": round(c.radius_m, 2), "b": round(c.bearing_deg, 1),
        "yaw": round(c.yaw_offset_deg, 1), "ok": s.ok, "hits": s.hits,
    });
    if let Some(p) = c.pitch_deg.or(s.pitch) {
        v["pitch"] = json!(round(p, 1));
    }
    if let Some(sc) = s.score {
        v["score"] = json!(sc);
    }
    if let Some(k) = confirm {
        v["confirm"] = json!(k);
    }
    if let Some(m) = s.miss {
        v["miss"] = json!(m);
    }
    v
}

/// `/gmgotoxyz` to the stand point of `pose` moved to `radius_m` / `bearing_deg`.
pub(super) async fn stand<I: ToolInvoker>(
    runner: &Runner<'_, I>,
    target: &Target,
    pose: &PoseSpec,
    radius_m: f64,
    bearing_deg: f64,
) -> Result<(), String> {
    let at = PoseSpec {
        radius_m,
        bearing_deg,
        ..pose.clone()
    }
    .stand_point(target.resolved.position);
    runner
        .stand_at(Who::P1, target.resolved.player, at, &mut scratch())
        .await
}

/// Camera onto the target with the candidate's yaw and pitch, then one
/// hover probe. A failed camera call is a miss.
pub(super) async fn look_and_probe<I: ToolInvoker>(
    runner: &Runner<'_, I>,
    target: &Target,
    c: &Candidate,
) -> Sample {
    let inv = runner.on(Who::P1);
    let id = target.resolved.entity_id;
    let mut cam = json!({ "yaw_offset_deg": c.yaw_offset_deg, "zoom": target.zoom });
    match target.point {
        Some(p) => cam["face_point"] = point_json(p),
        None => cam["face_entity_id"] = json!(id),
    }
    if let Some(p) = c.pitch_deg {
        cam["pitch_deg"] = json!(p);
    }
    let out = inv.call(CAMERA_TOOL, cam).await;
    if !out.ok {
        tracing::debug!(target: "lab.calibrate", error = out.error.as_deref().unwrap_or(""),
            "calibrate: camera failed, a miss");
        return Sample::missed("camera");
    }
    let pitch = match c.pitch_deg {
        Some(_) => out.json.pointer("/absolute/pitch/got"),
        None => out.json.pointer("/view_after/pitch_offset_deg"),
    }
    .and_then(Value::as_f64);
    let probe_args = match target.point {
        Some(p) => json!({ "point": point_json(p), "expect_entity": id }),
        None => json!({ "entity_id": id }),
    };
    let out = inv.call(HOVER_TOOL, probe_args).await;
    let probe = out.ok.then(|| score::parse_probe(&out.json)).flatten();
    match probe {
        Some(p) => Sample {
            ok: p.ok,
            hits: p.jitter_hits,
            score: score::score(&p),
            pitch,
            miss: None,
        },
        None => Sample {
            pitch,
            ..Sample::missed("probe")
        },
    }
}

/// D-SV6: up to [`CONFIRM_RUNS`] re-probes of candidate `i`, each from a
/// fresh teleport (the opposite bearing, then back). Stops at the first
/// that fails; returns how many passed.
pub(super) async fn confirm<I: ToolInvoker>(
    runner: &Runner<'_, I>,
    out_dir: &Path,
    target: &Target,
    pose: &PoseSpec,
    (i, c): (usize, &Candidate),
    probes: &mut u32,
) -> u32 {
    let away = (c.bearing_deg + 180.0).rem_euclid(360.0);
    for k in 0..CONFIRM_RUNS {
        let stood = match stand(runner, target, pose, c.radius_m, away).await {
            Ok(()) => stand(runner, target, pose, c.radius_m, c.bearing_deg).await,
            Err(e) => Err(e),
        };
        let s = match stood {
            Ok(()) => {
                *probes += 1;
                look_and_probe(runner, target, c).await
            }
            Err(_) => Sample::missed("stand"),
        };
        log_line(out_dir, &sample_line(&pose.id, i, c, &s, Some(k + 1)));
        if !(s.ok && s.hits >= CONFIRM_MIN_HITS) {
            return k;
        }
    }
    CONFIRM_RUNS
}
