//! `lab uat --diff-golden`: compare each passing run with the section's
//! committed golden through `lab_golden_diff`, and fold the per-run values into
//! the batch's `golden` block (D-GD7 of the lab-golden ledger: the verdict of a
//! run never changes, a divergence is its own exit code).

use std::path::Path;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use crate::batch::RunRecord;
use crate::clip::clip_text;
use crate::commands::calibrate::call_error_text;
use crate::daemon::mcp::{hide_lease_ids, McpClient, McpError};
use crate::daemon::status::Endpoint;

/// A stored golden value is cut to this many characters.
pub const VALUE_MAX: usize = 300;
/// A group names at most this many runs.
pub const RUN_REFS_MAX: usize = 5;
/// One `lab_golden_diff` call may take this long.
pub const DIFF_TIMEOUT: Duration = Duration::from_secs(120);

/// One distinct non-match line with how many runs had it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GoldenGroup {
    pub line: String,
    pub count: u32,
    pub runs: Vec<String>,
}

/// The golden comparison of a batch: key order `match`, `diverged`, `errors`,
/// `groups` (most common first).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GoldenSummary {
    #[serde(rename = "match")]
    pub matched: u32,
    pub diverged: u32,
    pub errors: u32,
    pub groups: Vec<GoldenGroup>,
}

/// A run's name in a group: `<instance>#<index>`.
pub fn default_run_ref(run: &RunRecord) -> String {
    format!("{}#{}", run.instance, run.index)
}

/// The comparison of the runs that carry a `Golden` value (`Merge-UatGolden`):
/// counts of matches, divergences and errors (a diff that could not be made),
/// and every non-match line grouped with its count and up to
/// [`RUN_REFS_MAX`] run refs, most common first (ties keep the order first
/// seen). `None` when no run has a value (a batch without `--diff-golden`).
/// `run_ref` names a run in a group.
pub fn merge_golden(
    runs: &[RunRecord],
    run_ref: &dyn Fn(&RunRecord) -> String,
) -> Option<GoldenSummary> {
    let with_golden: Vec<&RunRecord> = runs
        .iter()
        .filter(|r| r.golden.as_deref().is_some_and(|g| !g.is_empty()))
        .collect();
    if with_golden.is_empty() {
        return None;
    }
    let (mut matched, mut diverged, mut errors) = (0, 0, 0);
    let mut groups: Vec<GoldenGroup> = Vec::new();
    for run in with_golden {
        let line = run.golden.as_deref().unwrap_or("");
        if line == "match" {
            matched += 1;
            continue;
        }
        if line.starts_with("error: ") {
            errors += 1;
        } else {
            diverged += 1;
        }
        let at = match groups.iter().position(|g| g.line == line) {
            Some(i) => i,
            None => {
                groups.push(GoldenGroup {
                    line: line.to_string(),
                    count: 0,
                    runs: Vec::new(),
                });
                groups.len() - 1
            }
        };
        groups[at].count += 1;
        if groups[at].runs.len() < RUN_REFS_MAX {
            groups[at].runs.push(run_ref(run));
        }
    }
    groups.sort_by_key(|g| std::cmp::Reverse(g.count)); // stable: ties keep first-seen order
    Some(GoldenSummary {
        matched,
        diverged,
        errors,
        groups,
    })
}

/// The first line of a message, without its CR.
fn first_line(text: &str) -> &str {
    text.split('\n').next().unwrap_or("").trim_end_matches('\r')
}

/// Each run of CR and LF characters becomes one space.
fn collapse_newlines(text: &str) -> String {
    let mut out = String::new();
    let mut in_run = false;
    for c in text.chars() {
        if c == '\r' || c == '\n' {
            if !in_run {
                out.push(' ');
            }
            in_run = true;
        } else {
            in_run = false;
            out.push(c);
        }
    }
    out
}

/// The value `--diff-golden` stores in a run's `Golden` field, from a
/// `lab_golden_diff` reply or the message of a call that failed: `match`, the
/// first divergence line, or `error: <why>`. A refusal (`no_golden`,
/// `cannot_diff`) or a reply without a verdict is an error, never a match.
/// Newlines become spaces, lease ids are hidden, and the value is cut to
/// [`VALUE_MAX`] characters (`...` at the end when cut).
pub fn golden_value(reply: Option<&Value>, call_error: Option<&str>) -> String {
    let field = |key: &str| reply.and_then(|r| r.get(key));
    let text = |key: &str| field(key).and_then(Value::as_str).unwrap_or("");
    let value = if let Some(why) = call_error.filter(|e| !e.is_empty()) {
        format!("error: {why}")
    } else if field("ok").and_then(Value::as_bool) != Some(true) {
        let code = text("code");
        let prefix = if code.is_empty() {
            String::new()
        } else {
            format!("{code}: ")
        };
        format!("error: {prefix}{}", text("error"))
    } else {
        match (field("match").and_then(Value::as_bool), text("first")) {
            (Some(true), _) => "match".to_string(),
            (Some(false), first) if !first.is_empty() => first.to_string(),
            _ => "error: lab_golden_diff gave no verdict".to_string(),
        }
    };
    clip_text(&hide_lease_ids(&collapse_newlines(&value)), VALUE_MAX)
}

/// The arguments of `lab_golden_diff` less `run_dir` (`$diffArgs` of
/// `uat.ps1`): the section, and the spec folder when there is a local one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffArgs {
    pub section: String,
    pub specs_dir: Option<String>,
}

impl DiffArgs {
    pub fn new(section: &str, specs_dir: Option<&Path>) -> DiffArgs {
        DiffArgs {
            section: section.to_string(),
            specs_dir: specs_dir.map(|p| p.display().to_string()),
        }
    }
}

#[derive(Serialize)]
struct DiffCall<'a> {
    section: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    specs_dir: Option<&'a str>,
    run_dir: &'a str,
}

/// The `Golden` value `--diff-golden` gives one finished run, or `None` for
/// none: no `--diff-golden` (`diff` is `None`), or a run that did not pass
/// (only passing runs are compared). A passing run that named no folder is an
/// error, with no call. `invoke` is the call itself, so the lane loop passes
/// the MCP call and a test passes a stub.
pub fn run_golden(
    rec: &RunRecord,
    diff: Option<&DiffArgs>,
    invoke: &mut dyn FnMut(Value) -> Result<Value, McpError>,
) -> Option<String> {
    let diff = diff?;
    if !rec.ok {
        return None;
    }
    let run_dir = rec.run_dir.as_deref().unwrap_or("");
    if run_dir.is_empty() {
        return Some("error: the run named no run folder".to_string());
    }
    let call = DiffCall {
        section: &diff.section,
        specs_dir: diff.specs_dir.as_deref(),
        run_dir,
    };
    let args = serde_json::to_value(&call).expect("a plain struct serialises");
    Some(match invoke(args) {
        Ok(reply) => golden_value(Some(&reply), None),
        Err(e) => golden_value(
            None,
            Some(first_line(&call_error_text("lab_golden_diff", &e))),
        ),
    })
}

/// One `lab_golden_diff` call on its own MCP session (the lane loop's
/// `Invoke` of the PowerShell): the session is closed whatever the answer.
pub fn call_golden_diff(ep: &Endpoint, args: Value) -> Result<Value, McpError> {
    let mut client = McpClient::connect(ep)?;
    let reply = client.call_tool("lab_golden_diff", args, DIFF_TIMEOUT);
    client.close();
    reply
}
