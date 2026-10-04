//! The evidence bundle: one directory per run, one JSON per row, plus
//! attachments.
//!
//! ```text
//! <root>/<YYYY-MM-DD>-<build8>-<run id>/
//!   run.json                       RunManifest: builds, clocks, account, tools
//!   rows/<section>/<row>.json      RowEvidence
//!   rows/<section>/<row>/          screenshots (.png), tool reads (.json)
//!   ledger.md                      "Recording results" blocks + summary table
//! ```
//!
//! `<root>` is `CIMMERIA_LAB_UAT_DIR`, else
//! `%LOCALAPPDATA%\cimmeria-lab\uat-runs`. Bundles are local evidence, not
//! repo content: the ledger text is what goes into a campaign ledger.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::spec::Source;
use super::tier::{Role, Tier};

/// Bundle schema version (bump when a field changes meaning).
pub const BUNDLE_SCHEMA: u32 = 1;

/// One clause's outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail,
    /// Waiting for an attestation (a SigNoz query the agent runs).
    Pending,
    /// Could not be checked (a reader or endpoint was unreachable).
    Unverified,
    /// Waiting for a person's answer.
    NeedsHuman,
    /// Its tool is not routed.
    Blocked,
}

/// A row's result. The first four are the guide's own words; the last
/// three are the automation's, and none of them is a pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RowResult {
    Pass,
    Fail,
    /// Could not reach the step (missing tool, second player, owner files).
    Blocked,
    Skipped,
    /// Every automatic clause passed; a person must answer the rest.
    NeedsHuman,
    /// A required clause is pending or could not be checked.
    Unverified,
    /// Everything held, but a step ran below the row's required native
    /// level (an N3 fallback, a server shortcut).
    NativeShortfall,
}

impl RowResult {
    pub fn as_str(self) -> &'static str {
        match self {
            RowResult::Pass => "PASS",
            RowResult::Fail => "FAIL",
            RowResult::Blocked => "BLOCKED",
            RowResult::Skipped => "SKIPPED",
            RowResult::NeedsHuman => "NEEDS_HUMAN",
            RowResult::Unverified => "UNVERIFIED",
            RowResult::NativeShortfall => "NATIVE_SHORTFALL",
        }
    }
}

/// What ran for one action (a chat line expands into several calls).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    pub index: usize,
    pub role: Role,
    pub label: Option<String>,
    /// `tool`, `chat`, `wait`, `capture`.
    pub kind: String,
    /// What the spec asked for (tool name or the chat line).
    pub requested: String,
    /// The tool that actually ran (differs when a fallback was used).
    pub tool: Option<String>,
    pub args: Value,
    pub tier: Option<Tier>,
    pub tier_source: Option<String>,
    pub fallback_used: bool,
    pub host_started_ms: i64,
    pub elapsed_ms: u64,
    pub ok: bool,
    pub error: Option<String>,
    /// The tool's JSON result, truncated for size.
    pub result: Value,
    /// The individual tool calls a chat line or reader expanded into.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<Value>,
    /// `p2` when the second lab client ran it (two-player rows).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
}

/// One expected clause's outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClauseResult {
    pub id: String,
    pub text: String,
    pub source: Source,
    pub required: bool,
    pub expected: String,
    pub verdict: Verdict,
    pub observed: Value,
    #[serde(default)]
    pub detail: Option<String>,
    /// Host clock (ms) when it was evaluated or attested.
    pub evaluated_ms: i64,
    /// For SigNoz clauses: the filter with variables filled in and the
    /// time window, ready to paste into the SigNoz MCP or Logs Explorer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
    /// `p2` when the clause read the second lab client.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
}

/// An attachment written next to the row JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    /// Path relative to the run directory.
    pub path: String,
    pub tool: String,
    pub host_ms: i64,
}

/// The `.bug uat <row>` anchor typed at the row's start.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Anchor {
    pub note: String,
    /// `Bookmark <id> recorded`: the server's epoch ms at capture, which
    /// doubles as the server clock sample.
    pub bookmark_id: Option<u64>,
    pub host_sent_ms: i64,
    pub host_seen_ms: Option<i64>,
    /// Server minus host clock, from the bookmark (ms).
    pub server_offset_ms: Option<i64>,
}

/// One row's bundle entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowEvidence {
    pub schema: u32,
    pub run_id: String,
    pub section: String,
    pub system: String,
    pub row_id: String,
    pub title: String,
    pub guide_ref: String,
    pub ledger_ref: String,
    pub expected: String,
    pub result: RowResult,
    /// Why the result is not PASS (each a short sentence).
    pub reasons: Vec<String>,
    /// Informational notes (setup shortcuts, failed teardown).
    #[serde(default)]
    pub flags: Vec<String>,
    pub required_native: Tier,
    /// The least native tier any step action ran at (None: no step ran).
    pub native_used: Option<Tier>,
    pub character: Value,
    pub account_kind: String,
    pub started_utc: String,
    pub ended_utc: String,
    pub host_started_ms: i64,
    pub host_ended_ms: i64,
    pub anchor: Option<Anchor>,
    pub actions: Vec<ActionRecord>,
    pub clauses: Vec<ClauseResult>,
    pub attachments: Vec<Attachment>,
    pub known_issues: Vec<String>,
    pub relog: Option<String>,
    pub notes: Option<String>,
    /// Variables captured during the row (mail ids, bookmark id).
    pub vars: Value,
    /// Attestations appended by `lab_uat_attest`, in order.
    #[serde(default)]
    pub attestations: Vec<Value>,
    /// The runner's own blocking reasons (kept apart from `reasons` so a
    /// re-grade after an attestation starts from the same facts).
    #[serde(default)]
    pub blocked: Vec<String>,
}

/// The run-level manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunManifest {
    pub schema: u32,
    pub run_id: String,
    pub started_utc: String,
    pub ended_utc: Option<String>,
    pub operator: String,
    /// `service.version` (git SHA) of the server build, and where it came
    /// from (`arg`, `lab-mcp`, `attested`), or null when unknown.
    pub server: Value,
    /// SGW.exe and telemetry-DLL hashes, image base / ASLR slide.
    pub client: Value,
    pub account: Value,
    pub characters: Vec<Value>,
    /// Host clock at start, and the best server-offset estimate.
    pub clocks: Value,
    /// Every tool the router exposed when the run started.
    pub tools: Vec<String>,
    /// Spec files with their SHA-256.
    pub specs: Vec<Value>,
    pub owner_approvals: Vec<String>,
    pub vars: Value,
    /// `section/row` → result, updated as rows finish.
    pub rows: Vec<Value>,
}

/// A run directory.
#[derive(Debug, Clone)]
pub struct RunDir {
    pub root: PathBuf,
}

/// Replace path-hostile characters in an id (`M1-1`, `9a` are fine).
pub fn path_safe(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Where run directories go.
pub fn default_root() -> PathBuf {
    if let Ok(d) = std::env::var("CIMMERIA_LAB_UAT_DIR") {
        if !d.is_empty() {
            return PathBuf::from(d);
        }
    }
    let base = std::env::var("LOCALAPPDATA").map_or_else(|_| std::env::temp_dir(), PathBuf::from);
    base.join("cimmeria-lab").join("uat-runs")
}

impl RunDir {
    /// `<root>/<date>-<build8>-<run_id>`.
    pub fn create(
        root: &Path,
        date: &str,
        build: Option<&str>,
        run_id: &str,
    ) -> Result<Self, String> {
        let build8: String = build.unwrap_or("unknown").chars().take(8).collect();
        let dir = root.join(format!(
            "{date}-{}-{}",
            path_safe(&build8),
            path_safe(run_id)
        ));
        std::fs::create_dir_all(dir.join("rows"))
            .map_err(|e| format!("create {}: {e}", dir.display()))?;
        Ok(Self { root: dir })
    }

    pub fn open(path: &Path) -> Result<Self, String> {
        if path.join("run.json").is_file() {
            Ok(Self {
                root: path.to_path_buf(),
            })
        } else {
            Err(format!("{} has no run.json", path.display()))
        }
    }

    pub fn run_json(&self) -> PathBuf {
        self.root.join("run.json")
    }

    pub fn ledger_md(&self) -> PathBuf {
        self.root.join("ledger.md")
    }

    pub fn row_json(&self, section: &str, row: &str) -> PathBuf {
        self.root
            .join("rows")
            .join(path_safe(section))
            .join(format!("{}.json", path_safe(row)))
    }

    pub fn attach_dir(&self, section: &str, row: &str) -> PathBuf {
        self.root
            .join("rows")
            .join(path_safe(section))
            .join(path_safe(row))
    }

    pub fn write_json<T: Serialize>(&self, path: &Path, v: &T) -> Result<(), String> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(|e| format!("mkdir {}: {e}", p.display()))?;
        }
        let text = serde_json::to_string_pretty(v).map_err(|e| e.to_string())?;
        std::fs::write(path, text).map_err(|e| format!("write {}: {e}", path.display()))
    }

    pub fn read_json<T: for<'de> Deserialize<'de>>(&self, path: &Path) -> Result<T, String> {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
    }

    /// Every row JSON in the run, sorted by path (section, then row).
    pub fn rows(&self) -> Result<Vec<RowEvidence>, String> {
        let mut paths = Vec::new();
        let rows_dir = self.root.join("rows");
        let sections = std::fs::read_dir(&rows_dir)
            .map_err(|e| format!("read {}: {e}", rows_dir.display()))?;
        for s in sections.flatten() {
            if !s.path().is_dir() {
                continue;
            }
            for f in std::fs::read_dir(s.path()).into_iter().flatten().flatten() {
                let p = f.path();
                if p.extension().is_some_and(|x| x == "json") {
                    paths.push(p);
                }
            }
        }
        paths.sort();
        paths.iter().map(|p| self.read_json(p)).collect()
    }
}

/// Clip a JSON value for the bundle: long strings are cut, and huge
/// arrays keep their head and tail. Screenshots never land here (image
/// blocks become attachments).
pub fn clip(v: &Value, max_str: usize) -> Value {
    match v {
        Value::String(s) if s.chars().count() > max_str => {
            let head: String = s.chars().take(max_str).collect();
            Value::String(format!("{head}... [{} chars]", s.chars().count()))
        }
        Value::Array(a) if a.len() > 200 => {
            let mut out: Vec<Value> = a[..100].iter().map(|x| clip(x, max_str)).collect();
            out.push(Value::String(format!(
                "... [{} items elided]",
                a.len() - 200
            )));
            out.extend(a[a.len() - 100..].iter().map(|x| clip(x, max_str)));
            Value::Array(out)
        }
        Value::Array(a) => Value::Array(a.iter().map(|x| clip(x, max_str)).collect()),
        Value::Object(o) => Value::Object(
            o.iter()
                .map(|(k, x)| (k.clone(), clip(x, max_str)))
                .collect(),
        ),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn run_dir_layout_and_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let run = RunDir::create(tmp.path(), "2026-09-29", Some("abcdef0123456"), "r1").unwrap();
        assert!(run.root.ends_with("2026-09-29-abcdef01-r1"));
        let p = run.row_json("gm-parity", "M1-1");
        assert!(
            p.ends_with("rows/gm-parity/M1-1.json") || p.ends_with("rows\\gm-parity\\M1-1.json")
        );
        run.write_json(&p, &json!({"a": 1})).unwrap();
        let back: Value = run.read_json(&p).unwrap();
        assert_eq!(back["a"], 1);
        assert_eq!(path_safe("a/b c"), "a_b_c");
    }

    #[test]
    fn clip_bounds_strings_and_arrays() {
        let long = "x".repeat(5000);
        let c = clip(&json!({"s": long}), 100);
        assert!(c["s"].as_str().unwrap().ends_with("[5000 chars]"));
        let arr: Vec<u32> = (0..500).collect();
        let c = clip(&json!(arr), 100);
        assert_eq!(c.as_array().unwrap().len(), 201);
    }
}
