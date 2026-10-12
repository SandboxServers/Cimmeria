//! The `lab watch` model: one snapshot of a batch folder, the daemon's
//! `/status`, the `labd.log` tail and the SGW pids. Pure: the caller passes the
//! clock, the processes and the pid check, so tests drive it from fixtures.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::OnceLock;
use std::time::SystemTime;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use serde_json::Value;

use crate::batch::{
    format_progress_line, read_plan, read_records, Plan, RunRecord, BATCH_JSON, BATCH_JSONL,
    PLAN_JSON,
};
use crate::clip::clip_text;
use crate::daemon::mcp::hide_lease_ids;
use crate::known_issues;

/// Whether the batch is still running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchState {
    /// `plan.json`'s pid is alive.
    Live,
    /// `batch.json` exists.
    Done,
    /// A plan whose process is gone, and no `batch.json`.
    Interrupted,
    /// No `plan.json` (a batch from before plan.json existed) and no `batch.json`.
    Unknown,
}

impl BatchState {
    /// The word shown and written to JSON.
    pub fn as_str(self) -> &'static str {
        match self {
            BatchState::Live => "live",
            BatchState::Done => "done",
            BatchState::Interrupted => "interrupted",
            BatchState::Unknown => "unknown",
        }
    }
}

/// The last failed record of a batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastFailure {
    /// `<instance>#<index>`.
    pub run: String,
    /// `<row> <result>: <reason>` or `error: <error>`, lease ids hidden, cut to 72.
    pub why: String,
    pub issue: Option<String>,
}

/// The batch line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchView {
    /// The folder's name.
    pub name: String,
    pub state: BatchState,
    pub section: Option<String>,
    /// Finished runs (index 0 records, lane deaths, are not runs).
    pub runs: u32,
    /// Lanes times runs per lease; `None` without a plan.
    pub total: Option<u32>,
    pub passed: u32,
    pub failed: u32,
    /// Failed runs that match a known issue.
    pub known: u32,
    pub last_failure: Option<LastFailure>,
}

impl BatchView {
    /// `<runs>/<total>`, or `<runs>` without a plan.
    pub fn runs_text(&self) -> String {
        match self.total {
            Some(total) => format!("{}/{total}", self.runs),
            None => self.runs.to_string(),
        }
    }
}

/// One instance row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceView {
    pub instance: String,
    pub client_pid: Option<u32>,
    /// `free`, `<owner>` or `<owner> (<purpose>)`; `None` when /status has no row.
    pub lease: Option<String>,
    /// `-` outside the batch, else `<n>/<total>`, `<n>/<total> braked`, or `<n>`.
    pub run: String,
    /// The row under way, `-` between rows, `?` when the daemon has no `uat` field.
    pub row: String,
    /// Seconds the row under way has run.
    pub row_s: Option<u64>,
    /// Seconds since the run's newest screenshot.
    pub shot_s: Option<u64>,
    /// The lane's last progress line.
    pub last: Option<String>,
}

impl InstanceView {
    /// The ROW cell: `<row> <s>s`, or just the row.
    pub fn row_cell(&self) -> String {
        match self.row_s {
            Some(s) => format!("{} {s}s", self.row),
            None => self.row.clone(),
        }
    }
}

/// Relaunches and login retries counted in `labd.log`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogCounts {
    /// `(instance, count)` in order of first appearance.
    pub relaunches: Vec<(String, u32)>,
    pub login_retries: u32,
}

/// Everything one frame shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub daemon_up: bool,
    /// Some /status row has no `uat` key.
    pub old_daemon: bool,
    pub batch: Option<BatchView>,
    pub instances: Vec<InstanceView>,
    /// SGW pids no instance owns (empty when the daemon is down).
    pub orphans: Vec<u32>,
    /// `None` without a plan.
    pub logs: Option<LogCounts>,
}

/// What a snapshot reads.
pub struct Inputs<'a> {
    /// The `/status` body; `None` when the daemon is down.
    pub status: Option<&'a Value>,
    /// The batch folder, if any.
    pub batch_dir: Option<&'a Path>,
    /// The tail of `labd.log`.
    pub log_lines: &'a [String],
    /// Pids of every `SGW.exe`.
    pub sgw_pids: &'a [u32],
    pub now: DateTime<Utc>,
    /// Whether the process with this pid is the one running a batch.
    pub is_alive: &'a dyn Fn(u32) -> bool,
}

/// A batch folder read from disk.
#[derive(Debug, Clone)]
pub struct BatchFiles {
    pub name: String,
    pub plan: Option<Plan>,
    pub records: Vec<RunRecord>,
    pub state: BatchState,
}

/// One lane of the batch.
#[derive(Debug, Clone)]
pub struct LaneState {
    pub name: String,
    /// Runs finished (index above 0).
    pub count: u32,
    /// Runs per lease; `None` without a plan.
    pub total: Option<u32>,
    pub braked: bool,
    /// The lane's newest record (a lane death too).
    pub last: Option<RunRecord>,
}

/// A JSON time as UTC. `Z` and offset forms parse; a time with no offset reads
/// as UTC. Anything else is `None`.
pub fn parse_time(value: &str) -> Option<DateTime<Utc>> {
    let text = value.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(t) = DateTime::parse_from_rfc3339(text) {
        return Some(t.with_timezone(&Utc));
    }
    for format in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(text, format) {
            return Some(Utc.from_utc_datetime(&naive));
        }
    }
    None
}

fn seconds_since(now: DateTime<Utc>, then: DateTime<Utc>) -> u64 {
    u64::try_from((now - then).num_seconds().max(0)).unwrap_or(0)
}

/// Reads a batch folder: `plan.json`, `batch.jsonl` and the state (D-LW5):
/// `Done` when `batch.json` exists, else `Unknown` without a plan, else `Live`
/// when the plan's pid passes `is_alive`, else `Interrupted`. A missing folder
/// reads as `Unknown` with no records; an unreadable `batch.jsonl` is an error.
pub fn read_batch(dir: &Path, is_alive: &dyn Fn(u32) -> bool) -> io::Result<BatchFiles> {
    let plan = read_plan(&dir.join(PLAN_JSON));
    let jsonl = dir.join(BATCH_JSONL);
    let records = read_records(&jsonl)
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", jsonl.display())))?;
    let state = if dir.join(BATCH_JSON).exists() {
        BatchState::Done
    } else {
        match &plan {
            None => BatchState::Unknown,
            Some(p) => match p.pid {
                Some(pid) if is_alive(pid) => BatchState::Live,
                _ => BatchState::Interrupted,
            },
        }
    };
    Ok(BatchFiles {
        name: dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        plan,
        records,
        state,
    })
}

/// A record's known issue from the one table: `known_issues::tag` first, the
/// record's own `Verdict.Issue` only as a fallback; `None` for an error record.
pub fn issue_of(record: &RunRecord) -> Option<String> {
    if record.error.as_deref().is_some_and(|e| !e.is_empty()) {
        return None;
    }
    let verdict = record.verdict.as_ref();
    let row = verdict.and_then(|v| v.row.as_deref()).unwrap_or("");
    let reason = verdict.and_then(|v| v.reason.as_deref()).unwrap_or("");
    if let Some(tag) = known_issues::tag(row, reason) {
        return Some(tag.to_string());
    }
    verdict
        .and_then(|v| v.issue.clone())
        .filter(|issue| !issue.is_empty())
}

/// A record as a failure line: `<row> <result>: <reason>` or `error: <error>`,
/// lease ids hidden, cut to 72 characters.
pub fn why_of(record: &RunRecord) -> String {
    let why = match record.error.as_deref().filter(|e| !e.is_empty()) {
        Some(error) => format!("error: {error}"),
        None => {
            let verdict = record.verdict.as_ref();
            let row = verdict.and_then(|v| v.row.as_deref()).unwrap_or("");
            let result = verdict.and_then(|v| v.result.as_deref()).unwrap_or("");
            let reason = verdict.and_then(|v| v.reason.as_deref()).unwrap_or("");
            format!("{row} {result}: {reason}")
        }
    };
    clip_text(&hide_lease_ids(&why), 72)
}

fn unknown_total_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\[(\S+) (\d+)/0\]").expect("progress prefix regex"))
}

/// A record's progress line as `lab uat` prints it, but tagged with the
/// re-classified issue so it agrees with the counts. Without a plan (`total`
/// 0) the runs per lease are unknown: `<i>/?`.
pub fn last_line(record: &RunRecord, total: u32) -> String {
    let mut copy = record.clone();
    let mut verdict = copy.verdict.take().unwrap_or_default();
    verdict.issue = issue_of(record);
    copy.verdict = Some(verdict);
    let mut line = format_progress_line(&copy, total);
    if total == 0 {
        line = unknown_total_re()
            .replace(&line, "[${1} ${2}/?]")
            .into_owned();
    }
    hide_lease_ids(&line)
}

fn lane_index(lanes: &mut Vec<LaneState>, name: &str, total: Option<u32>) -> usize {
    if let Some(i) = lanes.iter().position(|l| l.name == name) {
        return i;
    }
    lanes.push(LaneState {
        name: name.to_string(),
        count: 0,
        total,
        braked: false,
        last: None,
    });
    lanes.len() - 1
}

/// The batch's counts and lanes. Index 0 records are lane deaths: they brake
/// their lane and can be the last failure, but are not counted as runs.
/// Lanes are the plan's, then any instance only the records name.
pub fn panel(files: &BatchFiles) -> (BatchView, Vec<LaneState>) {
    let per_lane = files.plan.as_ref().map(|p| p.runs_per_lease);
    let mut lanes: Vec<LaneState> = Vec::new();
    if let Some(plan) = &files.plan {
        for lane in &plan.lanes {
            if !lane.instance.is_empty() {
                lane_index(&mut lanes, &lane.instance, per_lane);
            }
        }
    }
    let (mut runs, mut passed, mut failed, mut known) = (0u32, 0u32, 0u32, 0u32);
    let mut last_failure = None;
    for record in &files.records {
        let i = lane_index(&mut lanes, &record.instance, per_lane);
        if record.index > 0 {
            runs += 1;
            lanes[i].count += 1;
            if record.ok {
                passed += 1;
            } else {
                failed += 1;
                if issue_of(record).is_some() {
                    known += 1;
                }
            }
        }
        if record.braked {
            lanes[i].braked = true;
        }
        lanes[i].last = Some(record.clone());
        if !record.ok {
            last_failure = Some(LastFailure {
                run: format!("{}#{}", record.instance, record.index),
                why: why_of(record),
                issue: issue_of(record),
            });
        }
    }
    let section = files
        .plan
        .as_ref()
        .map(|p| p.section.clone())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            files
                .records
                .first()
                .and_then(|r| r.rows.first())
                .and_then(|row| row.section.clone())
                .filter(|s| !s.is_empty())
        });
    let total = files
        .plan
        .as_ref()
        .map(|p| p.lanes.len() as u32 * p.runs_per_lease);
    let view = BatchView {
        name: files.name.clone(),
        state: files.state,
        section,
        runs,
        total,
        passed,
        failed,
        known,
        last_failure,
    };
    (view, lanes)
}

fn newest_png(dir: &Path) -> Option<SystemTime> {
    let mut newest: Option<SystemTime> = None;
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file()
                && path
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("png"))
            {
                if let Ok(modified) = entry.metadata().and_then(|m| m.modified()) {
                    if newest.is_none_or(|n| modified > n) {
                        newest = Some(modified);
                    }
                }
            }
        }
    }
    newest
}

/// Whole seconds since the newest `*.png` under `<run_dir>\rows`, or `None`
/// (an empty name, no `rows` folder, no screenshot). Errors under `rows` read
/// as no screenshot.
pub fn shot_age(run_dir: &str, now: DateTime<Utc>) -> Option<u64> {
    if run_dir.is_empty() {
        return None;
    }
    let rows = Path::new(run_dir).join("rows");
    if !rows.is_dir() {
        return None;
    }
    let modified = newest_png(&rows)?;
    Some(
        SystemTime::from(now)
            .duration_since(modified)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    )
}

/// `2026-10-11T01:39:06.0000000Z`: the form of the `labd.log` stamps that
/// `log_counts` compares against (7 fractional digits, ordinal compare).
pub fn since_stamp(since: DateTime<Utc>) -> String {
    format!(
        "{}.{:07}Z",
        since.format("%Y-%m-%dT%H:%M:%S"),
        since.timestamp_subsec_nanos() / 100
    )
}

fn stamp_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(\d{4}-\d\d-\d\dT\S+)").expect("stamp regex"))
}

fn instance_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"instance="([^"]+)""#).expect("instance span regex"))
}

/// Relaunches per instance and login retries in `labd.log` lines stamped at or
/// after `since` (an ordinal string compare of the ISO prefix; `""` keeps every
/// stamped line). A line without a leading stamp is skipped. A `relaunched
/// after crash` line counts for its first `instance="..."` span, or `default`
/// when it has none (a one-instance daemon); a `credential typing missed` line
/// is a login retry.
pub fn log_counts(lines: &[String], since: &str) -> LogCounts {
    let mut counts = LogCounts::default();
    for line in lines {
        let Some(caps) = stamp_re().captures(line) else {
            continue;
        };
        if !since.is_empty() && &caps[1] < since {
            continue;
        }
        if line.contains("relaunched after crash") {
            let instance = instance_re()
                .captures(line)
                .map(|c| c[1].to_string())
                .unwrap_or_else(|| "default".to_string());
            if let Some(entry) = counts.relaunches.iter_mut().find(|e| e.0 == instance) {
                entry.1 += 1;
            } else {
                counts.relaunches.push((instance, 1));
            }
        } else if line.contains("credential typing missed") {
            counts.login_retries += 1;
        }
    }
    counts
}

fn str_field<'a>(value: Option<&'a Value>, key: &str) -> &'a str {
    value
        .and_then(|v| v.get(key))
        .and_then(Value::as_str)
        .unwrap_or("")
}

/// An instance's LEASE cell: `free`, or `<owner> (<purpose>)` with the purpose
/// redacted and then cut to 40 characters (`...` after the cut). The time left
/// is never shown. Without a purpose, the owner alone.
pub fn lease_cell(lease: Option<&Value>) -> String {
    let held = lease
        .and_then(|l| l.get("held"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !held {
        return "free".to_string();
    }
    let inner = lease.and_then(|l| l.get("lease"));
    let owner = hide_lease_ids(str_field(inner, "owner"));
    let mut purpose = hide_lease_ids(str_field(inner, "purpose"));
    if purpose.chars().count() > 40 {
        purpose = format!("{}...", purpose.chars().take(40).collect::<String>());
    }
    if purpose.is_empty() {
        owner
    } else {
        format!("{owner} ({purpose})")
    }
}

/// One instance row. `row` is the instance's /status entry (`None` when /status
/// has none: the daemon is down, or the plan names a lane it lacks).
fn instance_view(
    name: &str,
    row: Option<&Value>,
    files: Option<&BatchFiles>,
    lanes: &[LaneState],
    now: DateTime<Utc>,
) -> InstanceView {
    let live = files.is_some_and(|f| f.state == BatchState::Live);
    let mut run = "-".to_string();
    let mut last = None;
    let mut run_dir: Option<String> = None;
    if let Some(lane) = lanes.iter().find(|l| l.name == name) {
        run = match lane.total {
            None => lane.count.to_string(),
            Some(total) if lane.braked => format!("{}/{total} braked", lane.count),
            Some(total) if live && lane.count < total => format!("{}/{total}", lane.count + 1),
            Some(total) => format!("{}/{total}", lane.count),
        };
        if let Some(record) = &lane.last {
            last = Some(last_line(record, lane.total.unwrap_or(0)));
            run_dir = record.run_dir.clone().filter(|d| !d.is_empty());
        }
    }
    let mut row_id = "-".to_string();
    let mut row_s = None;
    let mut client_pid = None;
    let mut lease = None;
    if let Some(r) = row {
        client_pid = r
            .get("client_pid")
            .and_then(Value::as_u64)
            .and_then(|p| u32::try_from(p).ok());
        lease = Some(lease_cell(r.get("lease")));
        match r.get("uat") {
            None => row_id = "?".to_string(),
            Some(uat) if uat.is_object() => {
                if let Some(current) = uat
                    .get("current_row")
                    .and_then(Value::as_str)
                    .filter(|c| !c.is_empty())
                {
                    row_id = hide_lease_ids(current);
                    row_s = uat
                        .get("row_started_at")
                        .and_then(Value::as_str)
                        .and_then(parse_time)
                        .map(|started| seconds_since(now, started));
                }
                if let Some(dir) = uat
                    .get("run_dir")
                    .and_then(Value::as_str)
                    .filter(|d| !d.is_empty())
                {
                    run_dir = Some(dir.to_string());
                }
            }
            Some(_) => {}
        }
    }
    InstanceView {
        instance: name.to_string(),
        client_pid,
        lease,
        run,
        row: row_id,
        row_s,
        shot_s: run_dir.and_then(|d| shot_age(&d, now)),
        last,
    }
}

/// The whole watch model. An error is an unreadable `batch.jsonl`.
pub fn build(inputs: &Inputs) -> io::Result<Snapshot> {
    let files = match inputs.batch_dir {
        Some(dir) => Some(read_batch(dir, inputs.is_alive)?),
        None => None,
    };
    let (batch, lanes) = match &files {
        Some(f) => {
            let (view, lanes) = panel(f);
            (Some(view), lanes)
        }
        None => (None, Vec::new()),
    };

    let rows: Vec<&Value> = inputs
        .status
        .and_then(|s| s.get("instances"))
        .and_then(Value::as_array)
        .map(|a| a.iter().filter(|v| !v.is_null()).collect())
        .unwrap_or_default();
    let mut instances = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for row in &rows {
        let name = str_field(Some(row), "instance").to_string();
        instances.push(instance_view(
            &name,
            Some(row),
            files.as_ref(),
            &lanes,
            inputs.now,
        ));
        seen.insert(name);
    }
    for lane in &lanes {
        if !seen.contains(&lane.name) {
            instances.push(instance_view(
                &lane.name,
                None,
                files.as_ref(),
                &lanes,
                inputs.now,
            ));
        }
    }

    let old_daemon = rows.iter().any(|row| row.get("uat").is_none());

    let mut orphans: Vec<u32> = Vec::new();
    if inputs.status.is_some() {
        let clients: HashSet<u32> = rows
            .iter()
            .filter_map(|r| r.get("client_pid").and_then(Value::as_u64))
            .filter_map(|p| u32::try_from(p).ok())
            .collect();
        for pid in inputs.sgw_pids {
            if !clients.contains(pid) && !orphans.contains(pid) {
                orphans.push(*pid);
            }
        }
    }

    let logs = files.as_ref().and_then(|f| f.plan.as_ref()).map(|plan| {
        let since = parse_time(&plan.started)
            .map(since_stamp)
            .unwrap_or_default();
        log_counts(inputs.log_lines, &since)
    });

    Ok(Snapshot {
        daemon_up: inputs.status.is_some(),
        old_daemon,
        batch,
        instances,
        orphans,
        logs,
    })
}
