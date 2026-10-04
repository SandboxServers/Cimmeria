//! The UAT tools: `lab_uat_run`, `lab_uat_report`, `lab_uat_attest`.
//!
//! `lab_uat_run` drives the client through this server's own tool router
//! *by name* ([`RouterInvoker`]), so a spec can name a tool another change
//! adds and the runner picks it up with no code change here; a name that
//! is not routed BLOCKs the row with that name.

use std::collections::HashSet;
use std::path::PathBuf;

use rmcp::{
    handler::server::{tool::ToolCallContext, wrapper::Parameters},
    model::*,
    schemars,
    service::RequestContext,
    tool, tool_router, ErrorData as McpError, RoleServer,
};
use serde_json::{json, Map, Value};

use super::LabServer;
use crate::supervisor::{instance, session_file};
use crate::uat::attest::{attest, AttestRequest};
use crate::uat::evidence::{default_root, RunDir, Verdict};
use crate::uat::invoke::{normalize_result, ServerInvoker, ServerTools, ToolInvoker, ToolOutcome};
use crate::uat::runner::{client_fingerprint, RunRequest, Runner};
use crate::uat::{latest_run, ledger, load_sections, specs_dir};

/// Args for `lab_uat_run`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct UatRunArgs {
    /// Section ids to run (`gm-parity`, `chat`, ...). Omit for all.
    #[serde(default)]
    pub sections: Option<Vec<String>>,
    /// Only these row ids (e.g. `["M1-1", "M4-1b"]`).
    #[serde(default)]
    pub rows: Option<Vec<String>>,
    /// Spec directory (default: CIMMERIA_LAB_UAT_SPECS, else the repo's
    /// docs/guides/uat-specs found from the working directory).
    #[serde(default)]
    pub specs_dir: Option<String>,
    /// Append to an existing run directory instead of starting a run.
    #[serde(default)]
    pub run_dir: Option<String>,
    /// The server's `service.version` (git SHA), when known (SigNoz).
    #[serde(default)]
    pub server_version: Option<String>,
    /// Colo rule-6 actions the owner approved in this session:
    /// `announce`, `bm_seed`, `mute`, `gmshout`, `content_reload`.
    #[serde(default)]
    pub owner_approvals: Vec<String>,
    /// Extra `${var}` values for the specs (`player_id`, `account_id`).
    #[serde(default)]
    pub vars: Option<Map<String, Value>>,
    /// The lab character (default: lab-account.json's `character`).
    #[serde(default)]
    pub character: Option<String>,
    /// Check specs and tool availability only; drive nothing.
    #[serde(default)]
    pub plan_only: bool,
}

/// Args for `lab_uat_report`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct UatReportArgs {
    /// Run directory (default: the newest under the UAT root).
    #[serde(default)]
    pub run_dir: Option<String>,
    /// Include every ledger block (default false: the summary only).
    #[serde(default)]
    pub ledger: bool,
}

/// Args for `lab_uat_attest`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct UatAttestArgs {
    /// Run directory (default: the newest).
    #[serde(default)]
    pub run_dir: Option<String>,
    #[serde(default)]
    pub section: Option<String>,
    /// Row id to attest a clause of.
    #[serde(default)]
    pub row: Option<String>,
    /// Clause id within the row.
    #[serde(default)]
    pub clause: Option<String>,
    /// SigNoz: rows the clause's filter returned in its window.
    #[serde(default)]
    pub row_count: Option<u64>,
    /// SigNoz: key rows as flat attribute objects.
    #[serde(default)]
    pub rows: Vec<Value>,
    /// SigNoz: the filter actually run.
    #[serde(default)]
    pub query_ran: Option<String>,
    /// Human or server clause: `pass` or `fail`.
    #[serde(default)]
    pub verdict: Option<String>,
    /// What the person (or the other read) saw.
    #[serde(default)]
    pub answer: Option<String>,
    #[serde(default)]
    pub by: Option<String>,
    /// The server's `service.version` for the whole run.
    #[serde(default)]
    pub server_version: Option<String>,
}

/// Calls this server's tools in-process through its router.
struct RouterInvoker<'a> {
    server: &'a LabServer,
    ctx: RequestContext<RoleServer>,
    names: HashSet<String>,
}

/// Tools the runner must never call: itself, re-entrantly.
fn forbidden(name: &str) -> bool {
    name.starts_with("lab_uat_")
}

impl ToolInvoker for RouterInvoker<'_> {
    fn has_tool(&self, name: &str) -> bool {
        !forbidden(name) && self.names.contains(name)
    }

    fn tool_names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.names.iter().cloned().collect();
        v.sort();
        v
    }

    async fn call(&self, name: &str, args: Value) -> ToolOutcome {
        if !self.has_tool(name) {
            return ToolOutcome::err(format!("tool {name} is not routed"));
        }
        let obj = match args {
            Value::Object(o) => o,
            Value::Null => Map::new(),
            other => return ToolOutcome::err(format!("arguments must be an object, got {other}")),
        };
        let params = CallToolRequestParams::new(name.to_string()).with_arguments(obj);
        let tcc = ToolCallContext::new(self.server, params, self.ctx.clone());
        match self.server.tool_router.call(tcc).await {
            Ok(CallToolResponse::Complete(r)) => {
                normalize_result(&serde_json::to_value(&r).unwrap_or(Value::Null))
            }
            Ok(_) => ToolOutcome::err(format!(
                "{name} asked for client input; not supported in a run"
            )),
            Err(e) => ToolOutcome {
                ok: false,
                error: Some(e.message.to_string()),
                error_data: e.data,
                ..Default::default()
            },
        }
    }
}

fn install_dir() -> Option<PathBuf> {
    std::env::var("CIMMERIA_LAB_INSTALL_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

/// lab-account.json of this instance: (username, character).
fn lab_account() -> (Option<String>, Option<String>) {
    let Some(dir) = install_dir() else {
        return (None, None);
    };
    let inst = instance::from_env().ok().flatten();
    match session_file::read_lab_account_at(&instance::account_path(&dir, inst.as_deref())) {
        Ok(a) => (
            Some(a.username),
            Some(a.character).filter(|c| !c.is_empty()),
        ),
        Err(_) => (None, None),
    }
}

fn text(v: &Value) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(
        serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string()),
    )])
}

fn run_dir_or_latest(arg: Option<&String>) -> Result<PathBuf, McpError> {
    match arg {
        Some(d) => Ok(PathBuf::from(d)),
        None => latest_run(&default_root())
            .ok_or_else(|| McpError::invalid_params("no UAT run found; pass run_dir", None)),
    }
}

#[tool_router(router = uat_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Run automated UAT rows from the TOML specs (docs/guides/uat-specs): reach each row's state with the lab flows, type a `.bug uat <row>` anchor, run setup and steps through the lab tools by name, check every expected clause, capture screenshots, and write an evidence bundle (run.json, rows/<section>/<row>.json, attachments, ledger.md). Returns each row's result (PASS, FAIL, BLOCKED, SKIPPED, NEEDS_HUMAN, UNVERIFIED, NATIVE_SHORTFALL) and the unified-uat.md \"Recording results\" blocks. A row never passes when a step ran below its required native level, a required clause is pending (SigNoz: attest with lab_uat_attest), or a tool it names is not routed (BLOCKED, naming the tool)."
    )]
    async fn lab_uat_run(
        &self,
        Parameters(a): Parameters<UatRunArgs>,
        ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let dir = a
            .specs_dir
            .map(PathBuf::from)
            .or_else(specs_dir)
            .ok_or_else(|| {
                McpError::invalid_params(
                    "no spec directory: pass specs_dir or set CIMMERIA_LAB_UAT_SPECS",
                    None,
                )
            })?;
        let sections = load_sections(&dir, a.sections.as_deref())
            .map_err(|e| McpError::invalid_params(e, None))?;
        let (account, character) = lab_account();
        let dll = std::env::var("CIMMERIA_LAB_DLL").ok().map(PathBuf::from);
        let patches = std::env::var("CIMMERIA_LAB_PATCHES_DLL")
            .ok()
            .map(PathBuf::from);
        let client = client_fingerprint(
            install_dir().as_deref(),
            &[
                ("telemetry_dll", dll.as_deref()),
                ("patches_dll", patches.as_deref()),
            ],
        );
        let req = RunRequest {
            sections,
            rows: a.rows,
            run_dir: a.run_dir.map(PathBuf::from),
            root: default_root(),
            server_version: a.server_version,
            owner_approvals: a.owner_approvals,
            vars: a.vars.unwrap_or_default(),
            lab_character: a.character.or(character),
            account_name: account,
            plan_only: a.plan_only,
            operator: std::env::var("CIMMERIA_LAB_OPERATOR").unwrap_or_else(|_| "lab agent".into()),
            client,
            no_settle: false,
        };
        let names: HashSet<String> = self
            .tool_router
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect();
        let inv = RouterInvoker {
            server: self,
            ctx,
            names,
        };
        let server_tools = ServerTools::from_env();
        let server = server_tools.as_ref().map(|s| s as &dyn ServerInvoker);
        let runner =
            Runner::new(&inv, server, req).map_err(|e| McpError::internal_error(e, None))?;
        let out = runner
            .run_all()
            .await
            .map_err(|e| McpError::internal_error(e, None))?;
        let mut blocks = vec![ContentBlock::text(
            serde_json::to_string_pretty(&json!({ "run_dir": out.run_dir, "rows": out.rows }))
                .unwrap_or_default(),
        )];
        if !out.ledger.is_empty() {
            blocks.push(ContentBlock::text(out.ledger));
        }
        Ok(CallToolResult::success(blocks))
    }

    #[tool(
        description = "Summarize a UAT run: PASS / FAIL / BLOCKED / SKIPPED / NEEDS_HUMAN / UNVERIFIED / NATIVE_SHORTFALL counts per section, every row that did not pass with its first reason, and (ledger=true) every Recording-results block. Defaults to the newest run."
    )]
    async fn lab_uat_report(
        &self,
        Parameters(a): Parameters<UatReportArgs>,
    ) -> Result<CallToolResult, McpError> {
        let dir = run_dir_or_latest(a.run_dir.as_ref())?;
        let run = RunDir::open(&dir).map_err(|e| McpError::invalid_params(e, None))?;
        let rows = run.rows().map_err(|e| McpError::internal_error(e, None))?;
        let report = ledger::report(&rows);
        let mut out = vec![
            ContentBlock::text(
                serde_json::to_string_pretty(
                    &json!({ "run_dir": dir.display().to_string(), "report": report }),
                )
                .unwrap_or_default(),
            ),
            ContentBlock::text(ledger::report_markdown(&report)),
        ];
        if a.ledger {
            let text = std::fs::read_to_string(run.ledger_md()).unwrap_or_default();
            out.push(ContentBlock::text(text));
        }
        Ok(CallToolResult::success(out))
    }

    #[tool(
        description = "Attest what the runner cannot observe, then re-grade the row: a SigNoz clause's row count and key rows (graded by the spec's min_rows / max_rows / field rules), a human clause's pass/fail answer, a server clause checked another way, or the run's server service.version. Returns the row's new result and its ledger block."
    )]
    async fn lab_uat_attest(
        &self,
        Parameters(a): Parameters<UatAttestArgs>,
    ) -> Result<CallToolResult, McpError> {
        let run_dir = run_dir_or_latest(a.run_dir.as_ref())?;
        let verdict = match a.verdict.as_deref() {
            None => None,
            Some("pass") => Some(Verdict::Pass),
            Some("fail") => Some(Verdict::Fail),
            Some(other) => {
                return Err(McpError::invalid_params(
                    format!("verdict {other:?}: pass or fail"),
                    None,
                ))
            }
        };
        let req = AttestRequest {
            run_dir,
            section: a.section,
            row: a.row,
            clause: a.clause,
            row_count: a.row_count,
            rows: a.rows,
            query_ran: a.query_ran,
            verdict,
            answer: a.answer,
            by: a.by,
            server_version: a.server_version,
        };
        attest(&req)
            .map(|v| text(&v))
            .map_err(|e| McpError::invalid_params(e, None))
    }
}
