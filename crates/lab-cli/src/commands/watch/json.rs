//! The one-line JSON of `lab watch --json`, and the usage-error object.
//!
//! The structs derive `Serialize`, so keys come out in field order; the
//! `skip_serializing_if` attributes give the omission rules.

use serde::{Serialize, Serializer};

use crate::ascii_json::to_ascii_json;
use crate::daemon::mcp::hide_lease_ids;

use super::snapshot::{InstanceView, Snapshot};

/// `{"ok":false,"exit":<code>,"error":"<message>"}`: a usage or read failure
/// under `--json` (lab uat's shape).
#[derive(Serialize)]
struct ErrorLine<'a> {
    ok: bool,
    exit: u8,
    error: &'a str,
}

/// The snapshot as one JSON line (see [`render_json`] for the keys).
#[derive(Serialize)]
struct WatchLine<'a> {
    ok: bool,
    daemon: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    batch: Option<&'a str>,
    state: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    section: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    runs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    passed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    known: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_failure: Option<LastFailureJson<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lanes: Option<Lanes<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    orphans: Vec<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    relaunches: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    login_retries: Option<u32>,
}

/// The `last_failure` object: `run`, `why`, then `issue` when there is one.
#[derive(Serialize)]
struct LastFailureJson<'a> {
    run: &'a str,
    why: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    issue: Option<&'a str>,
}

/// The `lanes` object: instance to its progress text, in snapshot order.
struct Lanes<'a>(Vec<(&'a str, String)>);

impl Serialize for Lanes<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(self.0.iter().map(|(name, text)| (*name, text.as_str())))
    }
}

fn to_line<T: Serialize>(value: &T) -> String {
    to_ascii_json(value)
}

/// `{"ok":false,"exit":<code>,"error":"<message>"}`: a usage or read failure
/// under `--json` (lab uat's shape).
pub fn error_line(code: u8, message: &str) -> String {
    to_line(&ErrorLine {
        ok: false,
        exit: code,
        error: message,
    })
}

/// The progress text of one lane: its run, then its row (when it has one) and
/// its screenshot age.
fn lane_text(instance: &InstanceView) -> String {
    let mut parts = vec![instance.run.clone()];
    let cell = instance.row_cell();
    if cell != "-" {
        parts.push(cell);
    }
    if let Some(s) = instance.shot_s {
        parts.push(format!("shot {s}s"));
    }
    parts.join(" ")
}

/// The snapshot as one JSON line. Keys in order: `ok`, `daemon`, then with a
/// batch `batch`, `state`, `section`, `runs`, `passed`, `failed`, `known`,
/// `last_failure` (`run`, `why`, `issue`), `lanes` (instance to
/// `<run> [<row>] [shot <n>s]`, only instances in the batch), without one
/// `state: "none"`; then `orphans` (at most 3), `relaunches` (the total),
/// `login_retries`.
pub fn render_json(snapshot: &Snapshot) -> String {
    let mut line = WatchLine {
        ok: true,
        daemon: if snapshot.daemon_up { "up" } else { "down" },
        batch: None,
        state: "none",
        section: None,
        runs: None,
        passed: None,
        failed: None,
        known: None,
        last_failure: None,
        lanes: None,
        orphans: snapshot.orphans.iter().take(3).copied().collect(),
        relaunches: None,
        login_retries: None,
    };
    if let Some(batch) = &snapshot.batch {
        line.batch = Some(&batch.name);
        line.state = batch.state.as_str();
        line.section = batch.section.as_deref();
        line.runs = Some(batch.runs_text());
        line.passed = Some(batch.passed);
        line.failed = Some(batch.failed);
        line.known = (batch.known > 0).then_some(batch.known);
        line.last_failure = batch.last_failure.as_ref().map(|last| LastFailureJson {
            run: &last.run,
            why: &last.why,
            issue: last.issue.as_deref(),
        });
        let lanes: Vec<(&str, String)> = snapshot
            .instances
            .iter()
            .filter(|i| i.run != "-")
            .map(|i| (i.instance.as_str(), lane_text(i)))
            .collect();
        line.lanes = (!lanes.is_empty()).then_some(Lanes(lanes));
    }
    if let Some(logs) = &snapshot.logs {
        let total: u32 = logs.relaunches.iter().map(|(_, c)| *c).sum();
        line.relaunches = (total > 0).then_some(total);
        line.login_retries = (logs.login_retries > 0).then_some(logs.login_retries);
    }
    hide_lease_ids(&to_line(&line))
}
