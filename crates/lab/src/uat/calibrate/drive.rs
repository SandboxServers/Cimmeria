//! The calibration sweep on a live client (SV-10, `lab_uat_calibrate`).
//!
//! Per pose: resolve its tag, then for each [`grid::candidates`] stand at
//! the candidate's point (`/gmgotoxyz`, only when the point changed), turn
//! the camera onto the target and hover-probe it; the best score is then
//! confirmed [`CONFIRM_RUNS`] times from a fresh teleport (D-SV6). The
//! winner becomes a pose line rewritten in a copy of the spec, and the
//! change a diff, both in the run folder.
//!
//! Nothing here clicks: the only client tools are `lab_client_status` and
//! those of [`super::candidate`] (`client_camera`, `client_hover_probe`,
//! the chat keys of `/gmgotoxyz` and the position reads). The spec file
//! itself is never written; a person applies the diff.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Map, Value};

use super::candidate::{
    confirm, log_line, look_and_probe, round, sample_line, scratch, stand, stand_key, Sample,
    Stood, Target,
};
use super::grid::{self, Candidate, Grid};
use super::score::{self, CONFIRM_RUNS};
use super::{edit, CalibrateOutcome};
use crate::uat::evidence::path_safe;
use crate::uat::invoke::ToolInvoker;
use crate::uat::runner::{LoadedSpec, Runner, Who};
use crate::uat::spec::{PoseSpec, RowSpec};

/// `budget_minutes` when the caller gives none ...
pub const DEFAULT_BUDGET: Duration = Duration::from_secs(15 * 60);
/// ... and the most it may ask for.
pub const MAX_BUDGET: Duration = Duration::from_secs(30 * 60);
/// Camera distance for a pose that names none (the pose then records it).
pub const DEFAULT_ZOOM: f64 = 250.0;
/// Winners tried before the sweep gives up confirming (D-SV6).
const MAX_WINNERS: usize = 3;
const CLICK_TOOL: &str = "client_world_click";
const STATUS_TOOL: &str = "lab_client_status";

/// One `lab_uat_calibrate` call.
#[derive(Debug, Clone)]
pub struct CalibrateRequest {
    pub spec: LoadedSpec,
    pub row: String,
    /// None: every pose a `client_world_click` step of the row names, in step order.
    pub pose: Option<String>,
    pub grid: Grid,
    /// For the whole call, checked between candidates; confirmations of a
    /// winner already found still run. Capped at [`MAX_BUDGET`].
    pub budget: Duration,
    /// The calibrate run folder.
    pub out_dir: PathBuf,
    /// The `calibrated` provenance's prefix: the date, plus the server
    /// version when known.
    pub stamp: String,
}

/// The spec's path from the repo root (`docs/guides/uat-specs/<file>`),
/// for the diff header; its file name when it is not under such a folder.
fn repo_relative(path: &str) -> String {
    let p = path.replace('\\', "/");
    const DIR: &str = "docs/guides/uat-specs/";
    if p.starts_with(DIR) {
        return p;
    }
    match p.find(&format!("/{DIR}")) {
        Some(i) => p[i + 1..].to_string(),
        None => Path::new(path)
            .file_name()
            .map_or(p.clone(), |f| f.to_string_lossy().into_owned()),
    }
}

/// The poses to calibrate: the one asked for, else every pose a
/// `client_world_click` step names, in step order, each once.
fn poses_of(row: &RowSpec, only: Option<&str>) -> Result<Vec<PoseSpec>, String> {
    if let Some(id) = only {
        return row
            .pose
            .iter()
            .find(|p| p.id == id)
            .map(|p| vec![p.clone()])
            .ok_or_else(|| format!("row {} has no pose {id}", row.id));
    }
    let mut out: Vec<PoseSpec> = Vec::new();
    for s in row
        .steps
        .iter()
        .filter(|s| s.tool.as_deref() == Some(CLICK_TOOL))
    {
        let id = s
            .args
            .as_ref()
            .and_then(|a| a.get("pose"))
            .and_then(Value::as_str);
        if let Some(p) = id.and_then(|id| row.pose.iter().find(|p| p.id == id)) {
            if !out.iter().any(|o| o.id == p.id) {
                out.push(p.clone());
            }
        }
    }
    if out.is_empty() {
        return Err(format!(
            "row {} names no pose in a {CLICK_TOOL} step",
            row.id
        ));
    }
    Ok(out)
}

/// Only the fields that changed, `{ field: [old, new] }` at the precision
/// the pose line writes them; an unset value shows as `"none"` (no nulls).
fn changed_fields(old: &PoseSpec, new: &PoseSpec) -> Map<String, Value> {
    let fields: [(&str, Option<f64>, Option<f64>, i32); 5] = [
        ("radius_m", Some(old.radius_m), Some(new.radius_m), 2),
        (
            "bearing_deg",
            Some(old.bearing_deg),
            Some(new.bearing_deg),
            1,
        ),
        ("pitch_deg", old.pitch_deg, new.pitch_deg, 1),
        ("yaw_offset_deg", old.yaw_offset_deg, new.yaw_offset_deg, 1),
        ("zoom", old.zoom, new.zoom, 1),
    ];
    let shown = |v: Option<f64>, places| v.map_or(json!("none"), |v| json!(round(v, places)));
    let mut out = Map::new();
    for (name, a, b, places) in fields {
        let (a, b) = (shown(a, places), shown(b, places));
        if a != b {
            out.insert(name.into(), json!([a, b]));
        }
    }
    out
}

/// Calibrate the request's poses, in order. Never errors: every problem is
/// a `reason` on that pose's outcome.
pub async fn calibrate<I: ToolInvoker>(
    runner: &Runner<'_, I>,
    req: &CalibrateRequest,
) -> Vec<CalibrateOutcome> {
    let section = req.spec.spec.section.id.clone();
    let refused = |pose: &str, reason: String| CalibrateOutcome {
        section: section.clone(),
        row: req.row.clone(),
        pose: pose.to_string(),
        reason: Some(reason),
        ..Default::default()
    };
    let asked = req.pose.as_deref().unwrap_or_default();
    let Some(row) = req.spec.spec.rows.iter().find(|r| r.id == req.row) else {
        return vec![refused(
            asked,
            format!("no row {} in section {section}", req.row),
        )];
    };
    let poses = match poses_of(row, req.pose.as_deref()) {
        Ok(p) => p,
        Err(e) => return vec![refused(asked, e)],
    };
    // Read the spec before anything moves: a sweep with nowhere to write
    // its proposal is lab time for nothing.
    let original = match refuse(runner, req).await {
        Some(why) => Err(why),
        None => std::fs::read_to_string(&req.spec.path)
            .map_err(|e| format!("read {}: {e}", req.spec.path)),
    };
    let original = match original {
        Ok(t) => t,
        Err(why) => return poses.iter().map(|p| refused(&p.id, why.clone())).collect(),
    };
    if let Err(e) = std::fs::create_dir_all(&req.out_dir) {
        let why = format!("run folder {}: {e}", req.out_dir.display());
        return poses.iter().map(|p| refused(&p.id, why.clone())).collect();
    }
    let deadline = tokio::time::Instant::now() + req.budget.min(MAX_BUDGET);
    let spec_rel = repo_relative(&req.spec.path);
    let mut text = original.clone();
    let mut out = Vec::with_capacity(poses.len());
    for pose in &poses {
        let (mut o, proposal) = calibrate_pose(runner, req, pose, deadline).await;
        if let Some(new) = proposal {
            // Each proposal builds on the last, so one diff carries them all.
            match propose(req, &spec_rel, &original, &text, &new) {
                Ok((t, diff)) => {
                    text = t;
                    o.diff_path = Some(diff.display().to_string());
                    o.spec_path = Some(spec_rel.clone());
                }
                Err(e) => {
                    o.ok = false;
                    o.reason = Some(format!("spec edit: {e}"));
                }
            }
        }
        tracing::info!(target: "lab.calibrate", event = "pose_calibrated",
            section = %o.section, row = %o.row, pose = %o.pose, ok = o.ok,
            score = ?o.score, confirmed = o.confirmed, probes = o.probes,
            reason = o.reason.as_deref().unwrap_or(""), "calibrate: pose done");
        out.push(o);
    }
    out
}

/// Step 1: why this call may not drive, if it may not.
async fn refuse<I: ToolInvoker>(runner: &Runner<'_, I>, req: &CalibrateRequest) -> Option<String> {
    let account = &req.spec.spec.section.account;
    if account != "gm" {
        return Some(format!(
            "calibration needs a gm section (it stands with /gmgotoxyz); this one is account = \"{account}\""
        ));
    }
    if runner.server.is_none() {
        return Some(
            "calibration needs the server lab MCP (CIMMERIA_LAB_MCP_URL/_TOKEN) to resolve tags"
                .into(),
        );
    }
    let st = runner.on(Who::P1).call(STATUS_TOOL, json!({})).await;
    let state = st.json.get("login_state").and_then(Value::as_str);
    match (st.ok, state) {
        (true, Some("in_world")) => None,
        (true, s) => Some(format!(
            "the client is not in world ({STATUS_TOOL}: {})",
            s.unwrap_or("no login_state")
        )),
        (false, _) => Some(format!("{STATUS_TOOL}: {}", st.error.unwrap_or_default())),
    }
}

/// Write the rewritten spec copy and the diff into the run folder; the
/// new text and the diff's path.
fn propose(
    req: &CalibrateRequest,
    spec_rel: &str,
    original: &str,
    text: &str,
    new: &PoseSpec,
) -> Result<(String, PathBuf), String> {
    let next = edit::rewrite_pose(text, &req.row, new)?;
    let name = Path::new(&req.spec.path)
        .file_name()
        .map_or_else(|| "spec.toml".into(), |f| f.to_string_lossy().into_owned());
    let copy = req.out_dir.join(name);
    std::fs::write(&copy, &next).map_err(|e| format!("write {}: {e}", copy.display()))?;
    let diff = req
        .out_dir
        .join(format!("{}.diff", path_safe(&req.spec.spec.section.id)));
    std::fs::write(&diff, edit::line_diff(spec_rel, original, &next))
        .map_err(|e| format!("write {}: {e}", diff.display()))?;
    Ok((next, diff))
}

/// Steps 2 to 5 for one pose: the outcome, and the new pose when it
/// differs from the old one.
async fn calibrate_pose<I: ToolInvoker>(
    runner: &Runner<'_, I>,
    req: &CalibrateRequest,
    pose: &PoseSpec,
    deadline: tokio::time::Instant,
) -> (CalibrateOutcome, Option<PoseSpec>) {
    let mut o = CalibrateOutcome {
        section: req.spec.spec.section.id.clone(),
        row: req.row.clone(),
        pose: pose.id.clone(),
        ..Default::default()
    };
    let resolved = match runner.resolve_tag(Who::P1, &pose.tag, &mut scratch()).await {
        Ok(t) => t,
        Err(e) => {
            o.reason = Some(format!("resolve: {e}"));
            return (o, None);
        }
    };
    let target = Target {
        point: pose.aim_point(resolved.position),
        resolved,
        zoom: pose.zoom.unwrap_or(DEFAULT_ZOOM),
    };
    let cands = grid::candidates(Some(pose), &req.grid);
    let mut scores: Vec<Option<u32>> = Vec::with_capacity(cands.len());
    let mut pitches: Vec<Option<f64>> = Vec::with_capacity(cands.len());
    let mut stood: Stood = None;
    let mut stop: Option<String> = None;
    for (i, c) in cands.iter().enumerate() {
        // Between candidates, not only between stands: a stand point holds
        // up to 9 camera poses.
        if i > 0 {
            if let Some(r) = runner.revocation_reason() {
                stop = Some(format!("lease revoked: {r}"));
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                stop = Some(format!("budget: swept {i} of {} candidates", cands.len()));
                break;
            }
        }
        if stood.map(|(k, _)| k) != Some(stand_key(c)) {
            let ok = stand(runner, &target, pose, c.radius_m, c.bearing_deg)
                .await
                .is_ok();
            stood = Some((stand_key(c), ok));
        }
        // A failed stand misses every candidate at that point.
        let s = if stood.is_some_and(|(_, ok)| !ok) {
            Sample::missed("stand")
        } else {
            o.probes += 1;
            look_and_probe(runner, &target, c).await
        };
        log_line(&req.out_dir, &sample_line(&pose.id, i, c, &s, None));
        scores.push(s.score);
        pitches.push(s.pitch);
    }

    if let Some(r) = stop.as_ref().filter(|r| r.starts_with("lease")) {
        // A displaced sweep confirms and proposes nothing.
        o.reason = Some(r.clone());
        return (o, None);
    }

    // Step 4: confirm the best, falling to the next best on a failure.
    let mut excluded = vec![false; scores.len()];
    let mut winner: Option<usize> = None;
    for _ in 0..MAX_WINNERS {
        let open: Vec<Option<u32>> = scores
            .iter()
            .zip(&excluded)
            .map(|(s, x)| if *x { None } else { *s })
            .collect();
        let Some(w) = score::best(&open) else { break };
        o.confirmed = confirm(
            runner,
            &req.out_dir,
            &target,
            pose,
            (w, &cands[w]),
            &mut o.probes,
        )
        .await;
        if o.confirmed == CONFIRM_RUNS {
            winner = Some(w);
            break;
        }
        excluded[w] = true;
    }
    let Some(w) = winner else {
        let what = if scores.iter().any(Option::is_some) {
            format!("no candidate held {CONFIRM_RUNS} confirmations")
        } else {
            "no candidate hovered the target".into()
        };
        let tail = stop.map(|s| format!("; {s}")).unwrap_or_default();
        o.reason = Some(format!("{what} ({} probes{tail})", o.probes));
        return (o, None);
    };
    o.ok = true;
    o.score = scores[w];
    o.reason = stop;
    let new = new_pose(pose, &cands[w], pitches[w], target.zoom, &req.stamp);
    o.changed = changed_fields(pose, &new);
    let proposal = (!o.changed.is_empty()).then_some(new);
    (o, proposal)
}

/// Step 5: the winner as a pose. An explicit pitch is the one asked for
/// (the camera reached it within its 0.5 degree tolerance, so the readout
/// would only add noise to an unchanged pose); a kept face pitch is the
/// readout's. A pose with no yaw keeps none when the winner turned none.
fn new_pose(
    pose: &PoseSpec,
    c: &Candidate,
    face_pitch: Option<f64>,
    zoom: f64,
    stamp: &str,
) -> PoseSpec {
    let yaw = match (pose.yaw_offset_deg, c.yaw_offset_deg) {
        (None, 0.0) => None,
        (_, y) => Some(y),
    };
    PoseSpec {
        radius_m: c.radius_m,
        bearing_deg: c.bearing_deg,
        pitch_deg: c.pitch_deg.or(face_pitch),
        yaw_offset_deg: yaw,
        zoom: Some(zoom),
        calibrated: Some(format!("{stamp} {CONFIRM_RUNS}/{CONFIRM_RUNS}")),
        ..pose.clone()
    }
}

#[cfg(test)]
#[path = "drive_tests.rs"]
mod tests;
