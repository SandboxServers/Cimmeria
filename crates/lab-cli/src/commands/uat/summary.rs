//! The summary of a batch: totals, per-row and per-lane pass counts, failures
//! grouped by signature. `summary.md` and `batch.json` are written from it.
//! LX-17 adds the `golden`, `chaos_divergence` and `heal` parts.

use std::fs;
use std::io;
use std::path::Path;

use chrono::{DateTime, Local};
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};

use crate::batch::{PlanLane, RunRecord};

const NL: &str = "\r\n";

fn eqi(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Passed and total counts of one row, `pass` first as in the JSON.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct RowStat {
    pub pass: u32,
    pub total: u32,
}

/// Per-row counts in the order each row was first seen. Serialises as a JSON
/// object in that order (a `serde_json::Map` would sort the keys).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowStats(pub Vec<(String, RowStat)>);

impl RowStats {
    /// The counts of `id` (ignoring case).
    pub fn get(&self, id: &str) -> Option<&RowStat> {
        self.0.iter().find(|(k, _)| eqi(k, id)).map(|(_, v)| v)
    }

    fn entry(&mut self, id: &str) -> &mut RowStat {
        let at = match self.0.iter().position(|(k, _)| eqi(k, id)) {
            Some(i) => i,
            None => {
                self.0.push((id.to_string(), RowStat::default()));
                self.0.len() - 1
            }
        };
        &mut self.0[at].1
    }
}

impl Serialize for RowStats {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (id, stat) in &self.0 {
            map.serialize_entry(id, stat)?;
        }
        map.end()
    }
}

/// One lane of the plan with its totals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LaneStat {
    pub lane: u32,
    pub instance: String,
    pub runs: u32,
    pub passed: u32,
    pub braked: bool,
}

/// Failed runs with the same signature: `<row> <result>: <reason>` or
/// `error: <error>`. `issue` is the known issue of the first such run, never set
/// for an error. `runs` are `<instance>#<index>` in record order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FailureGroup {
    pub signature: String,
    pub count: u32,
    pub issue: Option<String>,
    pub runs: Vec<String>,
}

/// The batch summary (`Merge-UatResults`). `Default` is for LX-17's tests.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Summary {
    pub ok: bool,
    pub runs: u32,
    pub passed: u32,
    pub failed: u32,
    pub failed_known: u32,
    pub failed_new: u32,
    pub rows: RowStats,
    pub lanes: Vec<LaneStat>,
    pub failures: Vec<FailureGroup>,
}

/// Aggregates run records into the summary. Lane deaths (`Index` 0) are records
/// like any other here, as in the PowerShell. `braked` are the instances of the
/// records marked `Braked`.
pub fn merge(runs: &[RunRecord], lanes: &[PlanLane], braked: &[String]) -> Summary {
    let mut rows = RowStats::default();
    for run in runs {
        for r in &run.rows {
            let stat = rows.entry(&r.row);
            stat.total += 1;
            if eqi(&r.result, "PASS") {
                stat.pass += 1;
            }
        }
    }
    let mut failures: Vec<FailureGroup> = Vec::new();
    for run in runs.iter().filter(|r| !r.ok) {
        let error = run.error.as_deref().filter(|e| !e.is_empty());
        let verdict = run.verdict.clone().unwrap_or_default();
        let signature = match error {
            Some(e) => format!("error: {e}"),
            None => format!(
                "{} {}: {}",
                verdict.row.as_deref().unwrap_or(""),
                verdict.result.as_deref().unwrap_or(""),
                verdict.reason.as_deref().unwrap_or("")
            ),
        };
        let at = match failures.iter().position(|g| eqi(&g.signature, &signature)) {
            Some(i) => i,
            None => {
                let issue = if error.is_some() {
                    None
                } else {
                    verdict.issue.clone().filter(|i| !i.is_empty())
                };
                failures.push(FailureGroup {
                    signature,
                    count: 0,
                    issue,
                    runs: Vec::new(),
                });
                failures.len() - 1
            }
        };
        failures[at].count += 1;
        failures[at]
            .runs
            .push(format!("{}#{}", run.instance, run.index));
    }
    let lane_stats: Vec<LaneStat> = lanes
        .iter()
        .map(|l| {
            let mine: Vec<&RunRecord> = runs
                .iter()
                .filter(|r| eqi(&r.instance, &l.instance))
                .collect();
            LaneStat {
                lane: l.lane,
                instance: l.instance.clone(),
                runs: mine.len() as u32,
                passed: mine.iter().filter(|r| r.ok).count() as u32,
                braked: braked.iter().any(|b| eqi(b, &l.instance)),
            }
        })
        .collect();
    let total = runs.len() as u32;
    let passed = runs.iter().filter(|r| r.ok).count() as u32;
    let known = runs
        .iter()
        .filter(|r| {
            !r.ok
                && r.error.as_deref().is_none_or(str::is_empty)
                && r.verdict
                    .as_ref()
                    .and_then(|v| v.issue.as_deref())
                    .is_some_and(|i| !i.is_empty())
        })
        .count() as u32;
    Summary {
        ok: total > 0 && passed == total,
        runs: total,
        passed,
        failed: total - passed,
        failed_known: known,
        failed_new: total - passed - known,
        rows,
        lanes: lane_stats,
        failures,
    }
}

/// The exit code of a batch with no golden comparison and no heal: 0 all passed,
/// 4 a lane stopped early, else 1 (`Get-UatExitCode`). LX-17a replaces this
/// whole function (it adds 5 and 8).
pub fn exit_code(summary: &Summary) -> u8 {
    if summary.ok {
        0
    } else if summary.lanes.iter().any(|l| l.braked) {
        4
    } else {
        1
    }
}

/// The summary as Markdown for people (`Format-UatSummary`), `\r\n` line ends,
/// the last line ended too.
pub fn format_markdown(summary: &Summary, title: &str, dir: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push(format!(
        "# {title}: {} ({}/{} runs passed)",
        if summary.ok { "PASS" } else { "FAIL" },
        summary.passed,
        summary.runs
    ));
    if summary.failed > 0 {
        lines.push(format!(
            "Failed: {} ({} known issue, {} new)",
            summary.failed, summary.failed_known, summary.failed_new
        ));
    }
    lines.push(String::new());
    lines.push("| Lane | Instance | Passed | Stopped early |".to_string());
    lines.push("|---|---|---|---|".to_string());
    for l in &summary.lanes {
        lines.push(format!(
            "| {} | {} | {}/{} | {} |",
            l.lane,
            l.instance,
            l.passed,
            l.runs,
            if l.braked { "yes" } else { "" }
        ));
    }
    lines.push(String::new());
    lines.push("| Row | Passed |".to_string());
    lines.push("|---|---|".to_string());
    for (id, stat) in &summary.rows.0 {
        lines.push(format!("| {id} | {}/{} |", stat.pass, stat.total));
    }
    if !summary.failures.is_empty() {
        lines.push(String::new());
        lines.push("Failures, grouped by the first failing row:".to_string());
        let mut sorted: Vec<&FailureGroup> = summary.failures.iter().collect();
        sorted.sort_by_key(|f| std::cmp::Reverse(f.count)); // stable: ties keep first-seen order
        for f in sorted {
            let tag = f
                .issue
                .as_deref()
                .filter(|i| !i.is_empty())
                .map(|i| format!(" (known: {i})"))
                .unwrap_or_default();
            lines.push(format!(
                "- {}x {}{}; runs: {}",
                f.count,
                f.signature,
                tag,
                f.runs.join(", ")
            ));
        }
    }
    lines.push(String::new());
    lines.push(format!("Batch: {dir}"));
    lines.iter().map(|l| format!("{l}{NL}")).collect()
}

/// `2026-10-11T12:39:06.1234567+02:00`: PowerShell's round-trip form of a local time.
pub fn local_stamp(at: DateTime<Local>) -> String {
    format!(
        "{}.{:07}{}",
        at.format("%Y-%m-%dT%H:%M:%S"),
        at.timestamp_subsec_nanos() / 100,
        at.format("%:z")
    )
}

/// What `batch.json` and `summary.md` need besides the summary.
pub struct BatchMeta<'a> {
    pub dir: &'a Path,
    pub title: &'a str,
    pub section: &'a str,
    pub row_ids: &'a [String],
    pub lane_count: u32,
    pub runs_per_lease: u32,
    pub started: DateTime<Local>,
}

/// A number that prints as an integer when it is whole (`3`, not `3.0`).
struct Minutes(f64);

impl Serialize for Minutes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.0.fract() == 0.0 {
            serializer.serialize_i64(self.0 as i64)
        } else {
            serializer.serialize_f64(self.0)
        }
    }
}

/// `batch.json`: keys in the PowerShell's order.
#[derive(Serialize)]
struct BatchFile<'a> {
    section: &'a str,
    rows: &'a [String],
    leases: u32,
    runs_per_lease: u32,
    started: String,
    minutes: Minutes,
    summary: &'a Summary,
}

/// Writes `summary.md` then `batch.json` into `meta.dir` and returns the
/// Markdown (`Write-UatBatchSummary`). `summary.md` is the Markdown plus one more
/// `\r\n`; `batch.json` is indented JSON plus `\r\n`. `minutes` is the time from
/// `meta.started` to `now`, rounded to one decimal (halves to even).
pub fn write_batch_summary(
    summary: &Summary,
    meta: &BatchMeta,
    now: DateTime<Local>,
) -> io::Result<String> {
    let md = format_markdown(summary, meta.title, &meta.dir.display().to_string());
    fs::write(meta.dir.join("summary.md"), format!("{md}{NL}"))?;
    let elapsed = (now - meta.started).num_milliseconds() as f64 / 60_000.0;
    let file = BatchFile {
        section: meta.section,
        rows: meta.row_ids,
        leases: meta.lane_count,
        runs_per_lease: meta.runs_per_lease,
        started: local_stamp(meta.started),
        minutes: Minutes((elapsed * 10.0).round_ties_even() / 10.0),
        summary,
    };
    let mut json = serde_json::to_string_pretty(&file).map_err(io::Error::other)?;
    json.push_str(NL);
    fs::write(meta.dir.join("batch.json"), json)?;
    Ok(md)
}
