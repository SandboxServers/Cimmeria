//! `lab_uat_calibrate` (lab-spec-vocab SV-10): sweep one row's poses on the
//! live client by hover and propose the winners as a spec diff
//! ([`crate::uat::calibrate::drive`]).
//!
//! It drives like `lab_uat_run`: in-process through this server's router
//! ([`RouterInvoker`]), under the caller's lease or one of its own (owner
//! `lab_uat_calibrate`) that is released whatever happens. D-SV7 (a): no
//! server guard, so it runs on the colo for the lab account and on any
//! local server.

use std::path::{Path, PathBuf};
use std::time::Duration;

use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, service::RequestContext, tool,
    tool_router, ErrorData as McpError, RoleServer,
};
use serde_json::json;

use super::uat::RouterInvoker;
use super::LabServer;
use crate::lease::run::CALIBRATE_OWNER;
use crate::lease::RunLease;
use crate::uat::calibrate::drive::{calibrate, CalibrateRequest, DEFAULT_BUDGET, MAX_BUDGET};
use crate::uat::calibrate::grid::{self, Grid, MAX_CANDIDATES, MIN_BEARING_STEP_DEG};
use crate::uat::calibrate::CalibrateOutcome;
use crate::uat::evidence::default_root;
use crate::uat::invoke::{ServerInvoker, ServerTools};
use crate::uat::runner::{now_ms, utc_of, RunRequest, Runner};
use crate::uat::{load_sections, specs_dir};

/// Args for `lab_uat_calibrate`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct UatCalibrateArgs {
    /// The section id (`first-session`).
    pub section: String,
    /// The row id (`FS-P3`).
    pub row: String,
    /// One pose id; omit for every pose a client_world_click step of the
    /// row names.
    #[serde(default)]
    pub pose: Option<String>,
    /// Spec directory (default: CIMMERIA_LAB_UAT_SPECS, else the repo's
    /// docs/guides/uat-specs).
    #[serde(default)]
    pub specs_dir: Option<String>,
    /// Minutes the sweep may take, for the whole call (default 15, at most
    /// 30). Checked between candidates; a winner already found is still
    /// confirmed.
    #[serde(default)]
    pub budget_minutes: Option<f64>,
    /// Stand-off radii to try, metres, in order (default [3.75, 3.0, 4.5]).
    #[serde(default)]
    pub radii_m: Option<Vec<f64>>,
    /// Bearing step, degrees, 1 to 360 (default 30: 12 bearings). The grid
    /// may hold at most 1999 candidates a pose.
    #[serde(default)]
    pub bearing_step_deg: Option<f64>,
    /// Your lab lease; omit it and the sweep takes one of its own (owner
    /// `lab_uat_calibrate`), refused while someone else holds the lab, and
    /// releases it when it ends.
    #[serde(default)]
    pub lease_id: Option<String>,
}

fn invalid(msg: impl Into<String>) -> McpError {
    McpError::invalid_params(msg.into(), None)
}

/// The sweep's budget: minutes above 0, at most [`MAX_BUDGET`]. A value
/// too large for a `Duration` is the cap, not a panic.
fn budget_of(minutes: Option<f64>) -> Result<Duration, McpError> {
    match minutes {
        None => Ok(DEFAULT_BUDGET),
        Some(m) if m.is_finite() && m > 0.0 => Ok(Duration::try_from_secs_f64(m * 60.0)
            .unwrap_or(MAX_BUDGET)
            .min(MAX_BUDGET)),
        Some(m) => Err(invalid(format!(
            "budget_minutes {m}: give more than 0 and at most 30 minutes"
        ))),
    }
}

/// The default grid with the caller's radii and bearing step. Radii stay
/// inside a pose's valid range (1.0 to 8.0 m) so the winner always loads;
/// the step is at least [`MIN_BEARING_STEP_DEG`], and the whole grid at
/// most [`MAX_CANDIDATES`].
fn grid_of(a: &UatCalibrateArgs) -> Result<Grid, McpError> {
    let mut grid = Grid::default();
    if let Some(r) = &a.radii_m {
        if r.is_empty() || r.iter().any(|v| !(1.0..=8.0).contains(v)) {
            return Err(invalid("radii_m: give one or more radii from 1.0 to 8.0"));
        }
        grid.radii_m = r.clone();
    }
    if let Some(s) = a.bearing_step_deg {
        if !(s.is_finite() && (MIN_BEARING_STEP_DEG..=360.0).contains(&s)) {
            return Err(invalid(format!(
                "bearing_step_deg: give a step from {MIN_BEARING_STEP_DEG} to 360"
            )));
        }
        grid.bearing_step_deg = s;
    }
    // The current pose is one more.
    let n = grid::count(&grid);
    if n >= MAX_CANDIDATES {
        return Err(invalid(format!(
            "radii_m x bearings gives {n} candidates a pose; give fewer radii or a coarser bearing_step_deg (at most {} candidates)",
            MAX_CANDIDATES - 1
        )));
    }
    Ok(grid)
}

/// The tool's result (D-SV12): the run folder once, then the outcomes.
fn body(run_dir: &Path, outcomes: &[CalibrateOutcome]) -> String {
    let v = json!({ "run_dir": run_dir.display().to_string(), "outcomes": outcomes });
    serde_json::to_string(&v).unwrap_or_default()
}

#[tool_router(router = calibrate_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Calibrate a row's interaction poses on the live client by hover only, never a click (D-SV6): for each candidate stand-off and camera pose (3 radii x 12 bearings x 3 yaws x 3 pitches, the current pose first) teleport with /gmgotoxyz, face the tagged target with client_camera and score a client_hover_probe; confirm the best 3 times from a fresh teleport (falling to the next best, at most 3 winners). Writes calibrate.jsonl (every candidate), a rewritten copy of the spec and <section>.diff into a run folder under the UAT root, and never writes the spec itself. Returns { run_dir, outcomes: [{ ok, section, row, pose, score, confirmed, changed, diff_path, probes, reason }] }; diff_path is on the first outcome that proposed, the one diff carries every proposal, and a confirmed unchanged pose still proposes its new calibrated stamp. Needs a gm section, the server lab MCP and an in-world client. Runs under the lab lease: pass your lease_id, or it takes one of its own (owner lab_uat_calibrate) and releases it when it ends."
    )]
    async fn lab_uat_calibrate(
        &self,
        Parameters(a): Parameters<UatCalibrateArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        // Held until this function returns; an own lease is released on drop.
        let book = self.supervisor.leases().clone();
        let lease = match a.lease_id.clone() {
            // The `call_tool` gate already checked it is current.
            Some(id) => RunLease::caller(book, id),
            None => RunLease::acquire_own_as(
                book,
                CALIBRATE_OWNER,
                format!("UAT calibrate {}/{}", a.section, a.row),
            )
            .map_err(|e| McpError::invalid_request(e, Some(self.supervisor.leases().status())))?,
        };
        let dir = a
            .specs_dir
            .clone()
            .map(PathBuf::from)
            .or_else(specs_dir)
            .ok_or_else(|| {
                invalid("no spec directory: pass specs_dir or set CIMMERIA_LAB_UAT_SPECS")
            })?;
        let spec = load_sections(&dir, Some(std::slice::from_ref(&a.section)))
            .map_err(invalid)?
            .pop()
            .ok_or_else(|| invalid(format!("no section {} in {}", a.section, dir.display())))?;
        let budget = budget_of(a.budget_minutes)?;
        let grid = grid_of(&a)?;
        let run = RunRequest {
            sections: vec![spec.clone()],
            rows: Some(vec![a.row.clone()]),
            root: default_root(),
            operator: std::env::var("CIMMERIA_LAB_OPERATOR").unwrap_or_else(|_| "lab agent".into()),
            instance: Some(self.supervisor.label().to_string()),
            ..Default::default()
        };
        let inv = RouterInvoker::new(self, ctx, Some(&lease));
        let server_tools = ServerTools::from_env();
        let server = server_tools.as_ref().map(|s| s as &dyn ServerInvoker);
        let mut runner =
            Runner::new(&inv, server, run).map_err(|e| McpError::internal_error(e, None))?;
        // Renewed for the whole sweep; a takeover stops it at the next
        // candidate. Declared after `lease`, so it stops first.
        let keep = lease.keep_alive();
        runner = runner.with_revocation(keep.revoked());
        let req = CalibrateRequest {
            spec,
            row: a.row,
            pose: a.pose,
            grid,
            budget,
            out_dir: runner.run.root.clone(),
            stamp: utc_of(now_ms())
                .get(..10)
                .unwrap_or("0000-00-00")
                .to_string(),
        };
        let outcomes = if lease.is_own() {
            crate::lease::permit::scope(lease.permit(), calibrate(&runner, &req)).await
        } else {
            calibrate(&runner, &req).await
        };
        runner.manifest.ended_utc = Some(utc_of(now_ms()));
        if let Err(e) = runner.save_manifest() {
            tracing::warn!(target: "lab.calibrate", error = %e, "calibrate: run.json not updated");
        }
        // D-SV12: compact, the candidates are in calibrate.jsonl.
        let body = body(&req.out_dir, &outcomes);
        Ok(CallToolResult::success(vec![ContentBlock::text(body)]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_budget_is_capped_and_a_bad_one_refused() {
        assert_eq!(budget_of(None).unwrap(), DEFAULT_BUDGET);
        assert_eq!(budget_of(Some(90.0)).unwrap(), MAX_BUDGET);
        assert_eq!(budget_of(Some(1.5)).unwrap(), Duration::from_secs(90));
        // Too large for a Duration: the cap, not a panic.
        assert_eq!(budget_of(Some(1e300)).unwrap(), MAX_BUDGET);
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(budget_of(Some(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_fine_bearing_step_or_a_huge_grid_is_refused() {
        let step = |s: f64| UatCalibrateArgs {
            bearing_step_deg: Some(s),
            ..Default::default()
        };
        // Below 1 degree: refused before any grid is built.
        assert!(grid_of(&step(0.5)).is_err());
        assert!(grid_of(&step(1e-9)).is_err());
        // 1 degree is allowed by itself, but 360 bearings x 3 radii x 9
        // camera poses is 9720 candidates: over the cap.
        let e = grid_of(&step(1.0)).unwrap_err();
        assert!(e.message.contains("9720 candidates"), "{e:?}");
        // One radius at 10 degrees: 36 x 9 = 324, allowed.
        let a = UatCalibrateArgs {
            radii_m: Some(vec![4.0]),
            bearing_step_deg: Some(10.0),
            ..Default::default()
        };
        assert_eq!(grid::count(&grid_of(&a).unwrap()), 324);
        // Many radii at the default step: 1000 x 12 x 9, refused.
        let a = UatCalibrateArgs {
            radii_m: Some(vec![4.0; 1000]),
            ..Default::default()
        };
        assert!(grid_of(&a).is_err());
    }

    #[test]
    fn the_result_names_the_run_folder_once_and_has_no_nulls() {
        let run_dir = Path::new(
            r"C:\Users\lab-operator\AppData\Local\cimmeria-lab\uat-runs\2026-10-11-1489a731-kqzxwbt",
        );
        let o = |pose: &str, diff: bool| CalibrateOutcome {
            ok: diff,
            section: "first-session".into(),
            row: "FS-P3".into(),
            pose: pose.into(),
            diff_path: diff.then(|| run_dir.join("first-session.diff").display().to_string()),
            ..Default::default()
        };
        let text = body(run_dir, &[o("frost", true), o("supply", false)]);
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["run_dir"], run_dir.display().to_string());
        assert_eq!(v["outcomes"].as_array().unwrap().len(), 2);
        assert!(!text.contains("null"), "{text}");
        assert!(!text.contains("spec_path"), "{text}");
    }

    #[test]
    fn grid_arguments_replace_the_defaults_and_stay_in_range() {
        let a = UatCalibrateArgs {
            radii_m: Some(vec![4.0]),
            bearing_step_deg: Some(90.0),
            ..Default::default()
        };
        let g = grid_of(&a).unwrap();
        assert_eq!((g.radii_m, g.bearing_step_deg), (vec![4.0], 90.0));
        for bad in [vec![], vec![0.5], vec![9.0]] {
            let a = UatCalibrateArgs {
                radii_m: Some(bad),
                ..Default::default()
            };
            assert!(grid_of(&a).is_err());
        }
        let a = UatCalibrateArgs {
            bearing_step_deg: Some(0.0),
            ..Default::default()
        };
        assert!(grid_of(&a).is_err());
    }
}
