//! `lab uat --json`: the one compact object an agent reads. No nulls, failure
//! groups capped (with a count of the rest), reasons cut to 200 characters,
//! golden lines to 120; a clean 7-row 10-run batch is about 340 characters and a
//! 5-run batch with two divergences stays under 500.

use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;

use crate::clip::clip_text;

use super::golden::GoldenSummary;
use super::summary::{FailureGroup, RowStats, Summary};

/// Failure groups listed (the rest are counted in `more_failure_groups`).
pub const MAX_FAILURES: usize = 8;
/// Heal proposals listed (the rest are counted in `more_heal`).
pub const MAX_HEAL: usize = 3;
const WHY_MAX: usize = 200;
const GOLDEN_LINE_MAX: usize = 120;
const RUNS_PER_GROUP: usize = 5;

fn is_zero(n: &u32) -> bool {
    *n == 0
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// `{"FS-P2":"1/3"}` in row order.
struct Rows<'a>(&'a RowStats);

impl Serialize for Rows<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0 .0.len()))?;
        for (id, r) in &self.0 .0 {
            map.serialize_entry(id, &format!("{}/{}", r.pass, r.total))?;
        }
        map.end()
    }
}

#[derive(Serialize)]
struct Lane<'a> {
    instance: &'a str,
    passed: String,
    #[serde(skip_serializing_if = "is_false")]
    braked: bool,
}

#[derive(Serialize)]
struct Failure {
    n: u32,
    why: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<String>,
    runs: Vec<String>,
}

#[derive(Serialize)]
struct CompactGolden {
    #[serde(rename = "match")]
    matched: u32,
    diverged: u32,
    #[serde(skip_serializing_if = "is_zero")]
    errors: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    first: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize)]
struct HealOut<'a> {
    row: &'a str,
    pose: &'a str,
    diff: &'a str,
}

#[derive(Serialize)]
struct Compact<'a> {
    ok: bool,
    runs: u32,
    passed: u32,
    failed: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    failed_known: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failed_new: Option<u32>,
    rows: Rows<'a>,
    lanes: Vec<Lane<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    failures: Vec<Failure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    more_failure_groups: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    golden: Option<CompactGolden>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chaos_divergence: Option<CompactGolden>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    heal: Vec<HealOut<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    more_heal: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chaos: Option<&'a str>,
    batch: &'a str,
}

/// A path's file name, splitting on `\` and `/` (`Split-Path -Leaf`).
fn leaf(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// A golden comparison for `--json`: counts, `errors` when some, then `first`
/// (the most common divergence, never an error line) or, only when no run
/// diverged, `error` (the most common error line); lines cut to `max_line`.
fn compact_golden(g: &GoldenSummary, max_line: usize) -> CompactGolden {
    let mut lines = g.groups.iter().map(|x| x.line.as_str());
    let div = lines.clone().find(|l| !l.starts_with("error: "));
    let err = lines.find(|l| l.starts_with("error: "));
    CompactGolden {
        matched: g.matched,
        diverged: g.diverged,
        errors: g.errors,
        first: div.map(|l| clip_text(l, max_line)),
        error: if div.is_none() {
            err.map(|l| clip_text(l, max_line))
        } else {
            None
        },
    }
}

/// The summary as the compact `--json` line (the caps of the packet:
/// [`MAX_FAILURES`] failure groups, [`MAX_HEAL`] heal entries). `batch_dir` is
/// the batch folder as printed.
pub fn compact_json(summary: &Summary, batch_dir: &str) -> String {
    compact_json_with(summary, batch_dir, MAX_FAILURES, MAX_HEAL)
}

/// [`compact_json`] with explicit caps (the tests use small ones).
pub fn compact_json_with(
    summary: &Summary,
    batch_dir: &str,
    max_failures: usize,
    max_heal: usize,
) -> String {
    let mut fails: Vec<&FailureGroup> = summary.failures.iter().collect();
    fails.sort_by_key(|f| std::cmp::Reverse(f.count)); // stable: ties keep their order
    let failures: Vec<Failure> = fails
        .iter()
        .take(max_failures)
        .map(|f| Failure {
            n: f.count,
            why: clip_text(&f.signature, WHY_MAX),
            issue: f.issue.clone().filter(|i| !i.is_empty()),
            runs: f.runs.iter().take(RUNS_PER_GROUP).cloned().collect(),
        })
        .collect();
    let out = Compact {
        ok: summary.ok,
        runs: summary.runs,
        passed: summary.passed,
        failed: summary.failed,
        failed_known: (summary.failed > 0).then_some(summary.failed_known),
        failed_new: (summary.failed > 0).then_some(summary.failed_new),
        rows: Rows(&summary.rows),
        lanes: summary
            .lanes
            .iter()
            .map(|l| Lane {
                instance: &l.instance,
                passed: format!("{}/{}", l.passed, l.runs),
                braked: l.braked,
            })
            .collect(),
        failures,
        more_failure_groups: (fails.len() > max_failures).then(|| fails.len() - max_failures),
        golden: summary
            .golden
            .as_ref()
            .map(|g| compact_golden(g, GOLDEN_LINE_MAX)),
        chaos_divergence: summary
            .chaos_divergence
            .as_ref()
            .map(|g| compact_golden(g, GOLDEN_LINE_MAX)),
        heal: summary
            .heal
            .iter()
            .take(max_heal)
            .map(|h| HealOut {
                row: &h.row,
                pose: &h.pose,
                diff: leaf(&h.diff),
            })
            .collect(),
        more_heal: (summary.heal.len() > max_heal).then(|| summary.heal.len() - max_heal),
        chaos: summary.chaos.as_deref().filter(|c| !c.is_empty()),
        batch: batch_dir,
    };
    serde_json::to_string(&out).expect("a plain struct serialises")
}
