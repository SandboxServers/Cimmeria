//! The golden tools: `lab_golden_record` and `lab_golden_diff` (README
//! D-GD9). Both only read run bundles; record also writes the one golden
//! file under `<specs_dir>/golden/`. Neither touches the client, so neither
//! needs the lease (F19).
//!
//! Replies are compact JSON with stable `code`s, because `lab golden` and
//! `lab uat -DiffGolden` map them to exit codes (GD-08, GD-09).

use std::path::PathBuf;

use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_router,
    ErrorData as McpError,
};
use serde_json::{json, Map, Value};

use super::LabServer;
use crate::uat::evidence::RunDir;
use crate::uat::golden::agree::{agree, check_runs, AgreeError};
use crate::uat::golden::build::fingerprint_run;
use crate::uat::golden::diff::diff_run_dir;
use crate::uat::golden::file::{self, golden_path, GoldenBuild, GoldenEvidence, GoldenFile};
use crate::uat::{load_sections, specs_dir};

/// Diverged rows a diff reply lists, the first included: enough to see a
/// pattern, few enough that the reply stays a few hundred characters.
const MAX_DIFF_ROWS: usize = 5;
/// An error is one line of at most this many characters. A disagreement is
/// a 300-character divergence line plus its run prefix, so it fits whole.
const MAX_ERROR_CHARS: usize = 400;

/// Args for `lab_golden_record`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct GoldenRecordArgs {
    /// The section id (`first-session`, ...).
    pub section: String,
    /// Only these row ids; omit for every row the runs reached.
    #[serde(default)]
    pub rows: Option<Vec<String>>,
    /// The fresh fingerprint-mode run directories, at least five.
    pub run_dirs: Vec<String>,
    /// Spec directory (default: CIMMERIA_LAB_UAT_SPECS, else the repo's
    /// docs/guides/uat-specs). The golden is written to its `golden/`.
    #[serde(default)]
    pub specs_dir: Option<String>,
    /// Replace an existing golden.
    #[serde(default)]
    pub rebless: bool,
}

/// Args for `lab_golden_diff`.
#[derive(Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct GoldenDiffArgs {
    /// A fingerprint-mode run directory.
    pub run_dir: String,
    /// The section id whose golden to compare against.
    pub section: String,
    /// Spec directory holding `golden/<section>.json` (default as for
    /// lab_golden_record).
    #[serde(default)]
    pub specs_dir: Option<String>,
}

/// `{ ok: false, code, error }` with `error` as one capped line.
fn refuse(code: &str, error: impl AsRef<str>) -> Value {
    json!({ "ok": false, "code": code, "error": one_line(error.as_ref()) })
}

/// Line breaks and other control characters become spaces, then the text
/// is cut, so a tool reply never carries a multi-line error.
fn one_line(s: &str) -> String {
    let flat: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if flat.chars().count() <= MAX_ERROR_CHARS {
        return flat;
    }
    let mut out: String = flat.chars().take(MAX_ERROR_CHARS - 3).collect();
    out.push_str("...");
    out
}

fn resolve_specs_dir(arg: Option<&str>) -> Option<PathBuf> {
    arg.filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(specs_dir)
}

/// The GD-08 exit code families: a disagreement (6) is a re-run or a
/// variance edit, a failing row (1) is a bug, everything else (7) is a run
/// set that cannot be recorded as given.
fn agree_code(e: &AgreeError) -> &'static str {
    match e {
        AgreeError::Disagree(_) | AgreeError::ExtraRow { .. } => "disagree",
        AgreeError::NotPassing { .. } => "not_passing",
        AgreeError::TooFew { .. }
        | AgreeError::NoRows
        | AgreeError::Mismatch { .. }
        | AgreeError::TapUnreliable { .. } => "cannot_record",
    }
}

/// Fingerprint each run, check they are comparable and agree under the
/// owner's five-run rule with every row PASS (README D-GD6 a), then write
/// the golden. Taps are checked inside `agree` (D-GD8): a dropped,
/// replaced, unstopped or unavailable tap is `cannot_record`.
pub(crate) fn record(args: GoldenRecordArgs, now_utc: &str) -> Value {
    let Some(dir) = resolve_specs_dir(args.specs_dir.as_deref()) else {
        return refuse(
            "cannot_record",
            "no spec directory: pass specs_dir or set CIMMERIA_LAB_UAT_SPECS",
        );
    };
    let loaded = match load_sections(&dir, Some(std::slice::from_ref(&args.section))) {
        Ok(mut v) if !v.is_empty() => v.swap_remove(0),
        Ok(_) => return refuse("cannot_record", format!("no spec section {}", args.section)),
        Err(e) => return refuse("cannot_record", e),
    };
    // An empty list would fingerprint no rows; it means "every row".
    let rows = args.rows.filter(|r| !r.is_empty());
    let mut fps = Vec::with_capacity(args.run_dirs.len());
    let mut metas = Vec::with_capacity(args.run_dirs.len());
    for d in &args.run_dirs {
        let fingerprinted = RunDir::open(std::path::Path::new(d))
            .and_then(|run| fingerprint_run(&run, &loaded.spec, rows.as_deref()));
        match fingerprinted {
            Ok((fp, meta)) => {
                fps.push(fp);
                metas.push(meta);
            }
            Err(e) => return refuse("cannot_record", format!("run {d}: {e}")),
        }
    }
    if let Err(e) = check_runs(&metas) {
        return refuse("cannot_record", e);
    }
    // The golden stores this spec's variance under the runs' spec hash, so
    // the two must be the same file: a spec edited after the runs would be
    // recorded under a hash that does not describe it.
    if let Some(m) = metas
        .iter()
        .find(|m| m.spec_sha256.as_deref() != Some(loaded.sha256.as_str()))
    {
        return refuse(
            "cannot_record",
            format!(
                "run {} drove spec {} but {} is now {}; record from fresh runs",
                m.run_id,
                m.spec_sha256.as_deref().unwrap_or("(none)"),
                loaded.path,
                loaded.sha256
            ),
        );
    }
    let golden = match agree(&fps, true) {
        Ok(g) => g,
        Err(e) => return refuse(agree_code(&e), e.to_string()),
    };
    let section = loaded.spec.section.id.clone();
    let path = golden_path(&dir, &section);
    if path.exists() && !args.rebless {
        return refuse(
            "exists",
            format!("{} exists; pass rebless to replace it", path.display()),
        );
    }
    // check_runs has proved these present, non-empty and equal in every run.
    let first = &metas[0];
    let g = GoldenFile {
        schema: file::GOLDEN_SCHEMA,
        section,
        rows: golden.rows.iter().map(|r| r.row.clone()).collect(),
        recorded_utc: now_utc.to_string(),
        spec_sha256: first.spec_sha256.clone().unwrap_or_default(),
        build: GoldenBuild {
            server: first.server_build.clone().unwrap_or_default(),
            sgw_sha256: first.sgw_sha256.clone(),
        },
        evidence: metas
            .iter()
            .map(|m| GoldenEvidence {
                run_id: m.run_id.clone(),
                run_dir: m.run_dir.clone(),
            })
            .collect(),
        fingerprint: golden,
    };
    let events: usize = g.fingerprint.rows.iter().map(|r| r.events.len()).sum();
    match file::save(&path, &g) {
        Ok(bytes) => json!({
            "ok": true,
            "path": path.display().to_string(),
            "rows": g.rows.len(),
            "events": events,
            "bytes": bytes,
            "runs": fps.len(),
        }),
        Err(e) => refuse("cannot_record", e),
    }
}

/// Compare one run with the section's golden, under the golden's stored
/// variance. `first` is the first divergence; `rows` the other diverged
/// rows, so the first line is not sent twice. A run's rows the golden does
/// not hold are not reported: `diff_run_dir` fingerprints only the golden's
/// rows (README Review outcomes, GD-04).
pub(crate) fn diff(args: GoldenDiffArgs) -> Value {
    let Some(dir) = resolve_specs_dir(args.specs_dir.as_deref()) else {
        return refuse(
            "no_golden",
            "no spec directory: pass specs_dir or set CIMMERIA_LAB_UAT_SPECS",
        );
    };
    let golden = match file::load(&golden_path(&dir, &args.section)) {
        Ok(g) => g,
        Err(e) => return refuse("no_golden", e),
    };
    let report = match RunDir::open(std::path::Path::new(&args.run_dir))
        .and_then(|run| diff_run_dir(&golden.fingerprint, &run))
    {
        Ok((report, _meta)) => report,
        Err(e) => return refuse("cannot_diff", format!("run {}: {e}", args.run_dir)),
    };
    let mut out = Map::new();
    out.insert("ok".into(), json!(true));
    let Some(first) = &report.first else {
        out.insert("match".into(), json!(true));
        return Value::Object(out);
    };
    out.insert("match".into(), json!(false));
    out.insert("first".into(), json!(first.line()));
    let diverged: Vec<_> = report.rows.iter().filter_map(|(_, d)| d.as_ref()).collect();
    if diverged.len() > 1 {
        out.insert("diverged".into(), json!(diverged.len()));
    }
    let others: Map<String, Value> = diverged
        .iter()
        .filter(|d| d.row != first.row)
        .take(MAX_DIFF_ROWS - 1)
        .map(|d| (d.row.clone(), json!(d.line())))
        .collect();
    if !others.is_empty() {
        out.insert("rows".into(), Value::Object(others));
    }
    if !report.not_in_golden.is_empty() {
        out.insert("not_in_golden".into(), json!(report.not_in_golden));
    }
    Value::Object(out)
}

fn compact(v: &Value) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(v.to_string())])
}

#[tool_router(router = golden_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Record a section's golden fingerprint from at least five fresh fingerprint-mode run directories that all agree, every row PASS, on one spec, server build and SGW.exe, writing <specs_dir>/golden/<section>.json; it reads bundles only and never drives the client. Reply {ok, path, rows, events, bytes, runs} or {ok: false, code: exists | disagree | not_passing | cannot_record, error}."
    )]
    async fn lab_golden_record(
        &self,
        Parameters(a): Parameters<GoldenRecordArgs>,
    ) -> Result<CallToolResult, McpError> {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        Ok(compact(&record(a, &now)))
    }

    #[tool(
        description = "Compare one fingerprint-mode run directory with its section's golden (recorded only when five runs agreed) under the golden's stored variance, returning {match} or the first divergence as one line plus up to four more diverged rows; it reads bundles only and never drives the client."
    )]
    async fn lab_golden_diff(
        &self,
        Parameters(a): Parameters<GoldenDiffArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(compact(&diff(a)))
    }
}

#[cfg(test)]
#[path = "golden_tests.rs"]
mod tests;
