//! The UAT runner: walks row specs, drives the client through the tool
//! router by name, evaluates every expected clause, and writes the
//! evidence bundle and the ledger blocks as each row finishes.
//!
//! One row runs as: static checks (standing `blocked`, players, colo
//! rule 6, tool availability) → reach the row's state (client running,
//! logged in, in world; and p2 in world for a two-player row) → the
//! `.bug uat <row>` anchor → packet tap and client-event marks → setup →
//! steps (a press also captures `${cast_id}`, see [`cast_id`]; clauses and
//! evidence tied to a step label run right after it) → end
//! clauses and evidence → teardown → grade → write. Rows are independent:
//! a failed row never stops the section.

mod actions;
mod cast_id;
mod checks;
mod clauses;
mod client_events;
mod drive;
mod fingerprint;
mod fixture;
mod fixture_verify;
mod lab_commands;
mod packet;
mod players;
mod revocation;
mod server_wait;
mod session;
mod snapshot;
mod targets;

use std::collections::HashMap;
use std::path::PathBuf;

use rmcp::schemars;
use serde_json::{json, Map, Value};

use super::evidence::{
    ActionRecord, Anchor, Attachment, ClauseResult, RowEvidence, RunDir, RunManifest, BUNDLE_SCHEMA,
};
use super::grade::grade;
use super::invoke::{ServerInvoker, ToolInvoker};
use super::ledger;
use super::spec::{PoseSpec, RowSpec, SectionSpec};

pub use players::SecondPlayer;
pub use session::client_fingerprint;

/// A parsed spec file and where it came from.
#[derive(Debug, Clone)]
pub struct LoadedSpec {
    pub path: String,
    pub sha256: String,
    pub spec: SectionSpec,
}

/// Everything one `lab_uat_run` call asks for.
#[derive(Debug, Clone, Default)]
pub struct RunRequest {
    pub sections: Vec<LoadedSpec>,
    /// Only these row ids (across the chosen sections); `None` = all.
    pub rows: Option<Vec<String>>,
    /// Append to this existing run instead of starting one.
    pub run_dir: Option<PathBuf>,
    pub root: PathBuf,
    /// `service.version` of the server build, when the caller knows it.
    pub server_version: Option<String>,
    /// Colo rule-6 actions the owner approved for this run
    /// (`announce`, `bm_seed`, `mute`, `gmshout`, `content_reload`).
    pub owner_approvals: Vec<String>,
    pub vars: Map<String, Value>,
    /// The lab account's character (lab-account.json `character`).
    pub lab_character: Option<String>,
    pub account_name: Option<String>,
    /// Check specs and tool availability only; drive nothing.
    pub plan_only: bool,
    pub operator: String,
    /// Client build fingerprint (see [`client_fingerprint`]).
    pub client: Value,
    /// Skip the UI settle sleeps between chat keys (tests only: the live
    /// client needs them to open the chat box and show the reply).
    pub no_settle: bool,
    /// The lab instance label this run drives (`default`, `p2`, ...).
    pub instance: Option<String>,
    /// When a row's fixture is applied (D-FX10). Default `Auto`.
    pub fixtures: FixtureMode,
    /// Capture what a golden fingerprint needs ([`fingerprint`]); never
    /// changes a row's grade.
    pub fingerprint: bool,
}

/// When a row's fixture is applied: `auto` (the row has one), `always`,
/// or `never` (D-FX10).
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    serde::Deserialize,
    serde::Serialize,
    schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum FixtureMode {
    /// Apply a row's fixture unless the previous row of the section passed
    /// and left the client in world (D-FX10).
    #[default]
    Auto,
    /// Apply a row's fixture even when the previous row left the client in
    /// world.
    Always,
    /// Never apply a fixture; the row runs from wherever the client is.
    Never,
}

/// A finished row, as the tool reports it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RowSummary {
    pub section: String,
    pub row: String,
    pub result: String,
    pub reasons: Vec<String>,
}

/// What a run returns.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RunOutcome {
    pub run_dir: String,
    pub rows: Vec<RowSummary>,
    /// The ledger blocks for the rows this call ran, ready to paste.
    pub ledger: String,
}

pub(crate) fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub(crate) fn utc_of(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_default()
}

/// A run id made of lowercase letters (base 26 of the epoch tenth of a
/// second), so it can also be a fresh character's name.
pub fn new_run_id(epoch_ds: i64) -> String {
    let mut n = epoch_ds.max(1) as u64;
    let mut s = Vec::new();
    while n > 0 {
        s.push(b'a' + (n % 26) as u8);
        n /= 26;
    }
    s.reverse();
    String::from_utf8(s).unwrap_or_default()
}

/// Per-row mutable state.
pub(crate) struct RowCtx {
    pub section: String,
    pub row_id: String,
    pub vars: Map<String, Value>,
    pub actions: Vec<ActionRecord>,
    pub blocked: Vec<String>,
    pub chat_marks: HashMap<String, Vec<String>>,
    pub attachments: Vec<Attachment>,
    pub character: Value,
    pub anchor: Option<Anchor>,
    pub started_ms: i64,
    /// The row's packet tap, when a clause reads one.
    pub tap: Option<packet::RowTap>,
    /// Event-store seqs client_event clauses read from, per client and
    /// label (`Who::mark`; `""` is the row start), or why there is none.
    pub event_marks: HashMap<String, Result<u64, String>>,
    /// Fingerprint-mode capture, when the run asked for it.
    pub fp: Option<fingerprint::FpCapture>,
    /// How a fixture row ran: `"fixture"` (its own staged session) or
    /// `"chain"` (continued the previous row's session); `None` without a
    /// fixture. Also the row's `fixture_mode` var.
    pub fixture_mode: Option<&'static str>,
    /// Errors from ending this row's fixture session. Each becomes a
    /// `session end: ...` flag: visible, but never the row's result.
    pub session_end: Vec<String>,
    /// A chained fixture row's parity with its fixture (FX-10).
    pub parity: Option<super::snapshot::Parity>,
    /// The row's poses, for `pose = "<id>"` arguments ([`targets`]).
    pub poses: Vec<PoseSpec>,
    /// lab-spec-vocab (SV-08): the server log watermark the next
    /// `wait_server` waits from, read before the action in front of it.
    pub log_mark: Option<server_wait::LogMark>,
}

/// The last row the run finished, for fixture mode `auto` (D-FX10).
#[derive(Debug, Clone)]
pub(crate) struct LastRow {
    pub(crate) section: String,
    pub(crate) row_id: String,
    pub(crate) passed: bool,
}

/// The runner. Holds the invokers, the run directory and its manifest.
pub struct Runner<'a, I: ToolInvoker> {
    pub(crate) inv: &'a I,
    pub(crate) server: Option<&'a dyn ServerInvoker>,
    pub(crate) run: RunDir,
    pub(crate) manifest: RunManifest,
    pub(crate) req: RunRequest,
    /// Section id → the fresh character made for it this run.
    pub(crate) fresh: HashMap<String, String>,
    /// The latest character-select list any flow returned.
    pub(crate) characters: Vec<Value>,
    /// The character the runner last entered the world as.
    pub(crate) in_world_as: Option<String>,
    /// The second lab client for two-player rows, or why there is none.
    pub(crate) p2: Result<SecondPlayer<'a, I>, String>,
    /// The character p2 last entered the world as.
    pub(crate) p2_in_world_as: Option<String>,
    /// The lease-loss signal that stops the run ([`revocation`]).
    pub(crate) revoked: Option<revocation::Revocation>,
    /// The row this run finished last.
    pub(crate) last_row: Option<LastRow>,
    /// A character an `end_session` could not drain; the next one drains
    /// it before its own (F24).
    pub(crate) pending_drain: Option<String>,
}

impl<'a, I: ToolInvoker> Runner<'a, I> {
    /// Open (or create) the run directory and write the manifest.
    pub fn new(
        inv: &'a I,
        server: Option<&'a dyn ServerInvoker>,
        req: RunRequest,
    ) -> Result<Self, String> {
        let t = now_ms();
        let (run, manifest) = match &req.run_dir {
            Some(dir) => {
                let run = RunDir::open(dir)?;
                let m: RunManifest = run.read_json(&run.run_json())?;
                (run, m)
            }
            None => {
                let run_id = new_run_id(t / 100);
                let date = utc_of(t).get(..10).unwrap_or("0000-00-00").to_string();
                let run = RunDir::create(&req.root, &date, req.server_version.as_deref(), &run_id)?;
                let m = RunManifest {
                    schema: BUNDLE_SCHEMA,
                    run_id,
                    started_utc: utc_of(t),
                    ended_utc: None,
                    operator: req.operator.clone(),
                    server: json!({
                        "service_version": req.server_version,
                        "source": if req.server_version.is_some() { "arg" } else { "unknown" },
                        "lab_mcp": server.map(|s| s.url()),
                    }),
                    client: req.client.clone(),
                    account: json!({ "name": req.account_name, "kind": null }),
                    characters: vec![],
                    clocks: json!({ "host_start_ms": t, "host_start_utc": utc_of(t) }),
                    tools: inv.tool_names(),
                    specs: vec![],
                    owner_approvals: req.owner_approvals.clone(),
                    vars: Value::Object(req.vars.clone()),
                    rows: vec![],
                };
                (run, m)
            }
        };
        let mut runner = Self {
            inv,
            server,
            run,
            manifest,
            req,
            fresh: HashMap::new(),
            characters: vec![],
            in_world_as: None,
            p2: Err(players::NO_P2.into()),
            p2_in_world_as: None,
            revoked: None,
            last_row: None,
            pending_drain: None,
        };
        for s in &runner.req.sections {
            let entry = json!({ "path": s.path, "sha256": s.sha256, "section": s.spec.section.id });
            if !runner.manifest.specs.contains(&entry) {
                runner.manifest.specs.push(entry);
            }
        }
        runner.save_manifest()?;
        Ok(runner)
    }

    pub(crate) fn save_manifest(&self) -> Result<(), String> {
        self.run.write_json(&self.run.run_json(), &self.manifest)
    }

    /// Run every selected row of every section, in file order.
    pub async fn run_all(mut self) -> Result<RunOutcome, String> {
        let mut done = Vec::new();
        let sections = self.req.sections.clone();
        for loaded in &sections {
            for row in &loaded.spec.rows {
                if let Some(only) = &self.req.rows {
                    if !only.iter().any(|r| r == &row.id) {
                        continue;
                    }
                }
                let ev = self.run_row(&loaded.spec, row).await;
                self.last_row = Some(LastRow {
                    section: ev.section.clone(),
                    row_id: ev.row_id.clone(),
                    passed: ev.result == super::evidence::RowResult::Pass,
                });
                self.finish_row(&ev)?;
                done.push(ev);
            }
        }
        self.manifest.ended_utc = Some(utc_of(now_ms()));
        self.save_manifest()?;
        self.render_ledger()?;
        let build = self.build_label();
        Ok(RunOutcome {
            run_dir: self.run.root.display().to_string(),
            rows: done
                .iter()
                .map(|r| RowSummary {
                    section: r.section.clone(),
                    row: r.row_id.clone(),
                    result: r.result.as_str().to_string(),
                    reasons: r.reasons.clone(),
                })
                .collect(),
            ledger: done
                .iter()
                .map(|r| ledger::block(r, &build))
                .collect::<Vec<_>>()
                .join("\n"),
        })
    }

    pub(crate) fn build_label(&self) -> String {
        self.manifest
            .server
            .get("service_version")
            .and_then(Value::as_str)
            .map_or_else(
                || "unknown (attest service.version)".to_string(),
                str::to_string,
            )
    }

    fn finish_row(&mut self, ev: &RowEvidence) -> Result<(), String> {
        self.run
            .write_json(&self.run.row_json(&ev.section, &ev.row_id), ev)?;
        let key = format!("{}/{}", ev.section, ev.row_id);
        self.manifest
            .rows
            .retain(|r| r.get("row").and_then(Value::as_str) != Some(key.as_str()));
        self.manifest
            .rows
            .push(json!({ "row": key, "result": ev.result.as_str() }));
        for c in &self.characters {
            if !self.manifest.characters.contains(c) {
                self.manifest.characters.push(c.clone());
            }
        }
        self.save_manifest()?;
        self.render_ledger()
    }

    /// Rewrite ledger.md from every row in the run.
    pub(crate) fn render_ledger(&self) -> Result<(), String> {
        let rows = self.run.rows()?;
        let text = ledger::ledger(&rows, &self.build_label(), &self.manifest.run_id);
        std::fs::write(self.run.ledger_md(), text).map_err(|e| e.to_string())
    }

    /// One row, start to finish. Never returns an error: every problem
    /// becomes a reason on the row.
    async fn run_row(&mut self, spec: &SectionSpec, row: &RowSpec) -> RowEvidence {
        let mut ctx = RowCtx {
            section: spec.section.id.clone(),
            row_id: row.id.clone(),
            vars: self.base_vars(spec, row),
            actions: vec![],
            blocked: vec![],
            chat_marks: HashMap::new(),
            attachments: vec![],
            character: json!({ "name": self.character_name(spec) }),
            anchor: None,
            started_ms: now_ms(),
            tap: None,
            event_marks: HashMap::new(),
            fp: None,
            fixture_mode: None,
            session_end: vec![],
            parity: None,
            poses: row.pose.clone(),
            log_mark: None,
        };
        ctx.blocked = self.static_blocks(spec, row);
        if let Some(r) = self.revocation_reason() {
            // The lease went before this row started: drive nothing.
            ctx.blocked.push(format!("{}: {r}", revocation::REVOKED));
        }
        let mut results: Vec<Option<ClauseResult>> = vec![None; row.expect.len()];

        let planned = self.req.plan_only && ctx.blocked.is_empty();
        if ctx.blocked.is_empty() && !self.req.plan_only {
            // Race the row against the lease: a takeover cuts it off at its
            // next await (tool call, wait_ms sleep, server read).
            let rx = self.revoked.clone();
            let lost = tokio::select! {
                _ = self.drive_row(spec, row, &mut ctx, &mut results) => None,
                r = revocation::revoked(rx) => Some(r),
            };
            if let Some(r) = lost {
                ctx.blocked.push(format!("{}: {r}", revocation::REVOKED));
                // The cut-off row's tap and capture are still in `ctx`.
                self.tap_abandon(&mut ctx).await;
            }
        }

        let clauses: Vec<ClauseResult> = results.into_iter().flatten().collect();
        let mut g = grade(&ctx.blocked, &ctx.actions, &clauses, row.required_native);
        // A failed session end does not change what the row proved, but the
        // next login may meet the character still online: show it.
        g.flags
            .extend(ctx.session_end.iter().map(|e| format!("session end: {e}")));
        super::snapshot::apply_parity(&mut g, ctx.parity.as_ref());
        // Plan only: a row that could run is SKIPPED, not graded.
        let result = if planned {
            g.reasons = vec!["plan only: ready to run, nothing was driven".into()];
            super::evidence::RowResult::Skipped
        } else {
            g.result
        };
        let ended = now_ms();
        RowEvidence {
            schema: BUNDLE_SCHEMA,
            run_id: self.manifest.run_id.clone(),
            section: spec.section.id.clone(),
            system: spec.section.system.clone(),
            row_id: row.id.clone(),
            title: row.title.clone(),
            guide_ref: spec.section.guide.clone(),
            ledger_ref: spec.section.ledger.clone(),
            expected: row.expected.trim().to_string(),
            result,
            reasons: g.reasons,
            flags: g.flags,
            required_native: row.required_native,
            native_used: g.native_used,
            character: ctx.character,
            account_kind: spec.section.account.clone(),
            started_utc: utc_of(ctx.started_ms),
            ended_utc: utc_of(ended),
            host_started_ms: ctx.started_ms,
            host_ended_ms: ended,
            anchor: ctx.anchor,
            actions: ctx.actions,
            clauses,
            attachments: ctx.attachments,
            known_issues: row.known_issues.clone(),
            relog: row.relog.clone(),
            notes: row.notes.clone(),
            vars: Value::Object(ctx.vars),
            attestations: vec![],
            blocked: ctx.blocked,
            fixture_parity: ctx.parity,
        }
    }

    /// The `${var}` values a row's specs see. Pub(crate) so the fixture
    /// tests can read the instance var without a whole run.
    pub(crate) fn base_vars(&self, spec: &SectionSpec, row: &RowSpec) -> Map<String, Value> {
        let mut v = self.req.vars.clone();
        v.insert("run_id".into(), json!(self.manifest.run_id));
        v.insert("row_id".into(), json!(row.id));
        v.insert("section".into(), json!(spec.section.id));
        if let Some(i) = &self.req.instance {
            v.insert("instance".into(), json!(i));
        }
        if let Some(c) = self.character_name(spec) {
            v.insert("character".into(), json!(c));
        }
        if let (2, Ok(p)) = (row.players, &self.p2) {
            v.insert(players::P2_CHARACTER_VAR.into(), json!(p.character));
        }
        v
    }
}

#[cfg(test)]
mod ability_tests;
#[cfg(test)]
mod fingerprint_tests;
#[cfg(test)]
mod fixture_session_tests;
#[cfg(test)]
mod fixture_tests;
#[cfg(test)]
mod fixture_verify_check_tests;
#[cfg(test)]
mod fixture_verify_tests;
#[cfg(test)]
mod packet_tests;
#[cfg(test)]
mod players_tests;
#[cfg(test)]
mod revocation_tests;
#[cfg(test)]
mod snapshot_mode_tests;
#[cfg(test)]
mod snapshot_tests;
#[cfg(test)]
mod server_wait_tests;
#[cfg(test)]
mod targets_tests;
#[cfg(test)]
mod test_server;
#[cfg(test)]
mod tests;
