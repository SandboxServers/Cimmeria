//! `lab_uat_calibrate` (lab-spec-vocab SV-10): sweep one row's poses on the
//! live client by hover and propose the winners as a spec diff
//! ([`crate::uat::calibrate::drive`]).
//!
//! It drives like `lab_uat_run`: in-process through this server's router
//! ([`RouterInvoker`]), under the caller's lease or one of its own (owner
//! `lab_uat_calibrate`) that is released whatever happens. D-SV7 (a): no
//! server guard, so it runs on the colo for the lab account and on any
//! local server.

use std::path::PathBuf;
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
use crate::uat::calibrate::grid::Grid;
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
    /// Bearing step, degrees (default 30: 12 bearings).
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

/// The sweep's budget: minutes, at most [`MAX_BUDGET`].
fn budget_of(minutes: Option<f64>) -> Result<Duration, McpError> {
    match minutes {
        None => Ok(DEFAULT_BUDGET),
        Some(m) if m.is_finite() && m >= 0.0 => {
            Ok(Duration::from_secs_f64(m * 60.0).min(MAX_BUDGET))
        }
        Some(m) => Err(invalid(format!("budget_minutes {m}: give 0 to 30 minutes"))),
    }
}

/// The default grid with the caller's radii and bearing step. Radii stay
/// inside a pose's valid range (1.0 to 8.0 m) so the winner always loads.
fn grid_of(a: &UatCalibrateArgs) -> Result<Grid, McpError> {
    let mut grid = Grid::default();
    if let Some(r) = &a.radii_m {
        if r.is_empty() || r.iter().any(|v| !(1.0..=8.0).contains(v)) {
            return Err(invalid("radii_m: give one or more radii from 1.0 to 8.0"));
        }
        grid.radii_m = r.clone();
    }
    if let Some(s) = a.bearing_step_deg {
        if !(s.is_finite() && s > 0.0 && s <= 360.0) {
            return Err(invalid(
                "bearing_step_deg: give a step above 0 and at most 360",
            ));
        }
        grid.bearing_step_deg = s;
    }
    Ok(grid)
}

#[tool_router(router = calibrate_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Calibrate a row's interaction poses on the live client by hover only, never a click (D-SV6): for each candidate stand-off and camera pose (3 radii x 12 bearings x 3 yaws x 3 pitches, the current pose first) teleport with /gmgotoxyz, face the tagged target with client_camera and score a client_hover_probe; confirm the best 3 times from a fresh teleport (falling to the next best, at most 3 winners). Writes calibrate.jsonl (every candidate), a rewritten copy of the spec and <section>.diff into a run folder under the UAT root, and never writes the spec itself. Returns { outcomes: [{ ok, section, row, pose, score, confirmed, changed, diff_path, spec_path, probes, reason }] }. Needs a gm section, the server lab MCP and an in-world client. Runs under the lab lease: pass your lease_id, or it takes one of its own (owner lab_uat_calibrate) and releases it when it ends."
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
        let body = serde_json::to_string(&json!({ "outcomes": outcomes })).unwrap_or_default();
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
        assert_eq!(budget_of(Some(0.0)).unwrap(), Duration::ZERO);
        assert!(budget_of(Some(-1.0)).is_err());
        assert!(budget_of(Some(f64::NAN)).is_err());
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
