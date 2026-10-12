//! What `lab_uat_calibrate` answers and how `lab calibrate` shows it: the
//! typed outcome, the plain-mode line, the `--json` objects (key order and
//! size budgets of the PowerShell), the exit code, the argument checks.
//!
//! Nothing here talks to the daemon or to git.

use std::time::Duration;

use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;
use serde_json::Value;

use crate::daemon::mcp::{hide_lease_ids, McpError};

/// The sweep confirms its winner this many times (`confirmed 3/3`).
pub const CONFIRM_RUNS: i64 = 3;
/// The largest `--budget-minutes`.
pub const MAX_BUDGET: u32 = 30;
/// The `--json` object of a failed `--apply` stays within this many characters.
pub const APPLY_ERROR_MAX: usize = 700;

/// The usage line (exit 2 when the section or the row is missing).
pub const USAGE: &str = "usage: lab calibrate <section> <row> [--pose <id>] [--instance <name>] [--prepare <row ids>] [--budget-minutes 1-30] [--apply] [--json] [--quiet]";

/// One pose's result from `lab_uat_calibrate`.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// The pose has a proposal (or is unchanged).
    pub ok: bool,
    /// The pose's id.
    pub pose: String,
    /// The winning (or best) candidate's hover score, when there was one.
    pub score: Option<f64>,
    /// How many of the [`CONFIRM_RUNS`] confirmation runs the winner passed.
    pub confirmed: i64,
    /// `(field, old, new)` in the order of the daemon's JSON object; a value
    /// is a number or the string `"none"`.
    pub changed: Vec<(String, Value, Value)>,
    /// Empty when this outcome proposed nothing or the tool named no file.
    pub diff_path: String,
    /// Why the pose has no proposal, or stopped early; empty when not said.
    pub reason: String,
}

impl Outcome {
    /// Reads one element of the reply's `outcomes`. Missing fields are empty,
    /// `false` or `0` (the PowerShell's `Get-UatField` returns `$null`).
    pub fn from_value(v: &Value) -> Outcome {
        let text = |key: &str| v.get(key).and_then(Value::as_str).unwrap_or("").to_string();
        Outcome {
            ok: v.get("ok").and_then(Value::as_bool).unwrap_or(false),
            pose: text("pose"),
            score: v.get("score").and_then(Value::as_f64),
            confirmed: v
                .get("confirmed")
                .and_then(Value::as_f64)
                .map_or(0, |c| c.round() as i64),
            changed: changes_from(v.get("changed")),
            diff_path: text("diff_path"),
            reason: text("reason"),
        }
    }
}

/// The `changed` object as `(field, old, new)`, in the order the parsed object
/// iterates, which is the order of the daemon's JSON: the daemon builds a
/// `serde_json::Map` without `preserve_order`, so it sends the keys sorted
/// (`bearing_deg`, `pitch_deg`, `radius_m`, `yaw_offset_deg`, `zoom`), and this
/// crate parses them the same way. That is the order the PowerShell printed.
/// An entry that is not a two-element array is skipped.
fn changes_from(changed: Option<&Value>) -> Vec<(String, Value, Value)> {
    let Some(map) = changed.and_then(Value::as_object) else {
        return Vec::new();
    };
    map.iter()
        .filter_map(|(name, pair)| {
            let pair = pair.as_array()?;
            if pair.len() != 2 {
                return None;
            }
            Some((name.clone(), pair[0].clone(), pair[1].clone()))
        })
        .collect()
}

/// The outcomes of a `lab_uat_calibrate` reply (none when the key is absent).
pub fn outcomes_from(reply: &Value) -> Vec<Outcome> {
    reply
        .get("outcomes")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|v| !v.is_null())
                .map(Outcome::from_value)
                .collect()
        })
        .unwrap_or_default()
}

/// The argument checks that need no daemon: an error message, or `None`.
/// An empty section or row counts as missing.
pub fn check_arguments(section: &str, row: &str, budget: Option<u32>) -> Option<String> {
    if section.is_empty() || row.is_empty() {
        return Some(USAGE.to_string());
    }
    if let Some(b) = budget {
        if !(1..=MAX_BUDGET).contains(&b) {
            return Some(format!("--budget-minutes must be 1 to {MAX_BUDGET}"));
        }
    }
    None
}

/// A changed field's short name: `bearing_deg` becomes `bearing`.
pub fn field_label(name: &str) -> &str {
    match name {
        "radius_m" => "radius",
        "bearing_deg" => "bearing",
        "pitch_deg" => "pitch",
        "yaw_offset_deg" => "yaw",
        other => other,
    }
}

/// One value of a changed pair: a number with at least one and at most three
/// decimals (`330.0`, `3.75`), or the tool's `"none"` for an unset field.
pub fn format_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => {
            let x = n.as_f64().unwrap_or(0.0);
            let fixed = format!("{x:.3}");
            let trimmed = fixed.trim_end_matches('0');
            if trimmed.ends_with('.') {
                format!("{trimmed}0")
            } else {
                trimmed.to_string()
            }
        }
        _ => "none".to_string(),
    }
}

/// A score as the PowerShell prints it: `8312`, not `8312.0`.
pub fn score_text(score: f64) -> String {
    if score.fract() == 0.0 && score.abs() < 1e15 {
        format!("{}", score as i64)
    } else {
        format!("{score}")
    }
}

/// The changed fields as `(label, "old -> new")`, in print order.
pub fn changes(o: &Outcome) -> Vec<(String, String)> {
    o.changed
        .iter()
        .map(|(name, old, new)| {
            (
                field_label(name).to_string(),
                format!("{} -> {}", format_value(old), format_value(new)),
            )
        })
        .collect()
}

/// The plain line printed for a pose:
///
/// ```text
/// frost: score 8312, confirmed 3/3, bearing 322.5 -> 330.0; diff <path>
/// frost: score 8312, confirmed 3/3, unchanged; diff <path>
/// frost: no proposal: <reason> (best score 5100, confirmed 1/3)
/// ```
pub fn format_line(o: &Outcome) -> String {
    let confirmed = format!("confirmed {}/{CONFIRM_RUNS}", o.confirmed);
    let reason = hide_lease_ids(&o.reason);
    if !o.ok {
        let mut line = format!("{}: no proposal", o.pose);
        if !reason.is_empty() {
            line.push_str(&format!(": {reason}"));
        }
        if let Some(score) = o.score {
            line.push_str(&format!(" (best score {}, {confirmed})", score_text(score)));
        }
        return line;
    }
    let mut parts: Vec<String> = Vec::new();
    if let Some(score) = o.score {
        parts.push(format!("score {}", score_text(score)));
    }
    parts.push(confirmed);
    let changes = changes(o);
    if changes.is_empty() {
        parts.push("unchanged".to_string());
    } else {
        for (label, text) in &changes {
            parts.push(format!("{label} {text}"));
        }
    }
    let mut line = format!("{}: {}", o.pose, parts.join(", "));
    if !reason.is_empty() {
        line.push_str(&format!(" ({reason})"));
    }
    if !o.diff_path.is_empty() {
        line.push_str(&format!("; diff {}", o.diff_path));
    }
    line
}

/// 0 when every pose has a proposal (or is unchanged); 1 when a pose found
/// none, or there were no outcomes.
pub fn exit_code(outcomes: &[Outcome]) -> u8 {
    if outcomes.is_empty() || outcomes.iter().any(|o| !o.ok) {
        1
    } else {
        0
    }
}

/// Text cut to `max` characters, ending `...` when cut.
pub fn limit_text(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let head: String = text.chars().take(max.saturating_sub(3)).collect();
    format!("{head}...")
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

/// The seconds a calibrate call may take: the budget (15 by default) plus
/// 5 minutes for the confirmation grace.
pub fn timeout_secs(budget: Option<u32>) -> u64 {
    (u64::from(budget.unwrap_or(15)) + 5) * 60
}

/// [`timeout_secs`] as a `Duration`.
pub fn timeout(budget: Option<u32>) -> Duration {
    Duration::from_secs(timeout_secs(budget))
}

/// A failed MCP call as the PowerShell's exception text: a tool's own message
/// starts with the tool's name (`<tool> failed: <text>`), other errors keep
/// their own wording.
pub fn call_error_text(tool: &str, error: &McpError) -> String {
    match error {
        McpError::Tool(message) => format!("{tool} failed: {message}"),
        other => other.to_string(),
    }
}

/// A failed `lab_uat_calibrate` call as one line, named once: the first line
/// of the message, lease ids hidden, `lab_uat_calibrate failed: ` in front
/// unless the message already starts with the tool's name.
pub fn format_call_error(message: &str) -> String {
    let first = message
        .split('\n')
        .next()
        .unwrap_or("")
        .trim_end_matches('\r');
    let m = hide_lease_ids(first);
    if m.starts_with("lab_uat_calibrate") {
        m
    } else {
        format!("lab_uat_calibrate failed: {m}")
    }
}

/// `{ "bearing": "322.5 -> 330.0" }`: a map in the order given.
#[derive(Debug, Clone)]
struct Changed(Vec<(String, String)>);

impl Serialize for Changed {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (label, text) in &self.0 {
            map.serialize_entry(label, text)?;
        }
        map.end()
    }
}

/// One pose in the `--json` objects. Field order is the key order.
#[derive(Debug, Clone, Serialize)]
struct JsonOutcome {
    pose: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<i64>,
    confirmed: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    changed: Option<Changed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

/// The `--json` entries of the outcomes: `{ pose, ok, score, confirmed,
/// changed, diff, reason }`, `diff` only when `with_diff`, a reason cut to 40
/// characters (the full text is on the human line).
fn compact_outcomes(outcomes: &[Outcome], with_diff: bool) -> Vec<JsonOutcome> {
    outcomes
        .iter()
        .map(|o| {
            let changes = changes(o);
            let reason = hide_lease_ids(&o.reason);
            JsonOutcome {
                pose: o.pose.clone(),
                ok: o.ok,
                score: o.score.map(|s| s.round() as i64),
                confirmed: o.confirmed,
                changed: if changes.is_empty() {
                    None
                } else {
                    Some(Changed(changes))
                },
                diff: if with_diff && !o.diff_path.is_empty() {
                    Some(o.diff_path.clone())
                } else {
                    None
                },
                reason: if reason.is_empty() {
                    None
                } else {
                    Some(limit_text(&collapse_newlines(&reason), 40))
                },
            }
        })
        .collect()
}

#[derive(Serialize)]
struct CompactReply<'a> {
    ok: bool,
    section: &'a str,
    row: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    outcomes: Option<Vec<JsonOutcome>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    applied: Option<bool>,
}

fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("a plain struct serialises")
}

/// The `--json` object: `{ok, section, row, outcomes, applied}` on one line,
/// no nulls and no empty arrays (`outcomes` only when there are some,
/// `applied` only when true). `ok` is "every pose proposed or unchanged".
pub fn compact_json(section: &str, row: &str, outcomes: &[Outcome], applied: bool) -> String {
    let entries = compact_outcomes(outcomes, true);
    to_json(&CompactReply {
        ok: exit_code(outcomes) == 0,
        section,
        row,
        outcomes: if entries.is_empty() {
            None
        } else {
            Some(entries)
        },
        applied: if applied { Some(true) } else { None },
    })
}

#[derive(Serialize)]
struct StopJson {
    ok: bool,
    exit: u8,
    error: String,
}

/// A usage or pre-flight failure under `--json`:
/// `{"ok":false,"exit":N,"error":"..."}`, the message cut to 300 characters.
pub fn stop_json(code: u8, message: &str) -> String {
    to_json(&StopJson {
        ok: false,
        exit: code,
        error: limit_text(message, 300),
    })
}

#[derive(Serialize)]
struct ApplyError {
    ok: bool,
    exit: u8,
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outcomes: Option<Vec<JsonOutcome>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outcomes_cut: Option<usize>,
}

/// The `--json` object of a failed `--apply` (exit 2): `{ok, exit, error,
/// run_dir, diff, outcomes, outcomes_cut}`, so an agent keeps the proposal
/// and the poses' results. The error (which starts with the diff path) is cut
/// to 200 characters and the outcomes carry no `diff` of their own. Outcomes
/// are dropped from the end until the object fits `max` characters, counted
/// as `outcomes_cut`. Pass [`APPLY_ERROR_MAX`] for `max`.
pub fn apply_error_json(
    message: &str,
    run_dir: &str,
    diff_path: &str,
    outcomes: &[Outcome],
    max: usize,
) -> String {
    let entries = compact_outcomes(outcomes, false);
    let total = entries.len();
    let non_empty = |s: &str| {
        if s.is_empty() {
            None
        } else {
            Some(s.to_string())
        }
    };
    let mut out = ApplyError {
        ok: false,
        exit: 2,
        error: limit_text(&hide_lease_ids(message), 200),
        run_dir: non_empty(run_dir),
        diff: non_empty(diff_path),
        outcomes: None,
        outcomes_cut: None,
    };
    let mut keep = total;
    loop {
        out.outcomes = if keep > 0 {
            Some(entries[..keep].to_vec())
        } else {
            None
        };
        out.outcomes_cut = if keep < total {
            Some(total - keep)
        } else {
            None
        };
        let json = to_json(&out);
        if json.chars().count() <= max || keep == 0 {
            return json;
        }
        keep -= 1;
    }
}
