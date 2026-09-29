//! The boot-time "what did this DLL manage to hook" record.
//!
//! Every sink and engine seam reports its install outcome here as well as
//! in its own `client.hooks.*` event. After the last install the sink module
//! emits one `client.hooks.capabilities` event listing them all, so a
//! session's SigNoz query can start from "which streams exist in this
//! session at all" instead of inferring it from which events happened to
//! arrive. A stream that is absent because nothing happened and a stream
//! that is absent because its hook never went in look identical without it.

use std::sync::Mutex;

use serde_json::Value;

use crate::capture::CaptureConfig;

/// Telemetry target of the summary event.
pub const TARGET: &str = "client.hooks.capabilities";

/// How one hook's install went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The hook is live.
    Installed,
    /// The install was attempted and failed.
    Failed(&'static str),
    /// The install was not attempted (a precondition was missing).
    Skipped(&'static str),
}

impl Outcome {
    /// The field value: `installed`, `failed: <why>`, `skipped: <why>`.
    pub fn text(&self) -> String {
        match self {
            Outcome::Installed => "installed".to_string(),
            Outcome::Failed(why) => format!("failed: {why}"),
            Outcome::Skipped(why) => format!("skipped: {why}"),
        }
    }
}

static RECORDED: Mutex<Vec<(&'static str, Outcome)>> = Mutex::new(Vec::new());

/// Record one hook's outcome. A hook that reports twice (a retry) keeps the
/// later outcome.
pub fn record(name: &'static str, outcome: Outcome) {
    let mut all = RECORDED.lock().unwrap_or_else(|e| e.into_inner());
    match all.iter_mut().find(|(n, _)| *n == name) {
        Some(slot) => slot.1 = outcome,
        None => all.push((name, outcome)),
    }
}

/// Everything recorded so far, in first-report order.
pub fn snapshot() -> Vec<(&'static str, Outcome)> {
    RECORDED.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

/// The fields of the summary event for `recorded` under `config`: one
/// `hook.<name>` field per hook, the counts, and the capture switches.
pub fn fields(recorded: &[(&'static str, Outcome)], config: CaptureConfig) -> Vec<(String, Value)> {
    let installed = recorded
        .iter()
        .filter(|(_, o)| *o == Outcome::Installed)
        .count();
    let mut out: Vec<(String, Value)> = recorded
        .iter()
        .map(|(name, outcome)| (format!("hook.{name}"), Value::String(outcome.text())))
        .collect();
    out.push(("installed".into(), Value::from(installed)));
    out.push(("attempted".into(), Value::from(recorded.len())));
    out.push(("capture.unfilter".into(), Value::Bool(config.unfilter)));
    out.push(("capture.firehose".into(), Value::Bool(config.firehose)));
    out
}

/// Telemetry level for the summary: `warn` when any hook that was tried did
/// not go in, so a degraded session stands out.
pub fn level(recorded: &[(&'static str, Outcome)]) -> &'static str {
    if recorded
        .iter()
        .any(|(_, o)| matches!(o, Outcome::Failed(_)))
    {
        "warn"
    } else {
        "info"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<(&'static str, Outcome)> {
        vec![
            ("bw_message", Outcome::Installed),
            ("log4cxx", Outcome::Failed("create_failed")),
            ("d3d9", Outcome::Skipped("module not loaded")),
        ]
    }

    #[test]
    fn outcomes_read_as_words() {
        assert_eq!(Outcome::Installed.text(), "installed");
        assert_eq!(Outcome::Failed("x").text(), "failed: x");
        assert_eq!(Outcome::Skipped("y").text(), "skipped: y");
    }

    #[test]
    fn the_summary_lists_each_hook_and_counts() {
        let f = fields(&sample(), CaptureConfig::default());
        let get = |k: &str| f.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
        assert_eq!(
            get("hook.bw_message"),
            Some(Value::String("installed".into()))
        );
        assert_eq!(
            get("hook.log4cxx"),
            Some(Value::String("failed: create_failed".into()))
        );
        assert_eq!(
            get("hook.d3d9"),
            Some(Value::String("skipped: module not loaded".into()))
        );
        assert_eq!(get("installed"), Some(Value::from(1)));
        assert_eq!(get("attempted"), Some(Value::from(3)));
        assert_eq!(get("capture.unfilter"), Some(Value::Bool(false)));
    }

    #[test]
    fn the_capture_switches_are_reported() {
        let f = fields(
            &[],
            CaptureConfig {
                unfilter: true,
                firehose: true,
            },
        );
        assert!(f.contains(&("capture.unfilter".to_string(), Value::Bool(true))));
        assert!(f.contains(&("capture.firehose".to_string(), Value::Bool(true))));
    }

    /// A failure makes the summary a warning; a skip does not (a hook whose
    /// module simply is not loaded is not a fault).
    #[test]
    fn only_a_failure_raises_the_level() {
        assert_eq!(level(&sample()), "warn");
        assert_eq!(
            level(&[("a", Outcome::Installed), ("b", Outcome::Skipped("s"))]),
            "info"
        );
        assert_eq!(level(&[]), "info");
    }

    #[test]
    fn a_repeat_report_replaces_the_earlier_outcome() {
        record("caps_test_hook", Outcome::Failed("first"));
        record("caps_test_hook", Outcome::Installed);
        let snap = snapshot();
        let ours: Vec<_> = snap
            .iter()
            .filter(|(n, _)| *n == "caps_test_hook")
            .collect();
        assert_eq!(ours.len(), 1);
        assert_eq!(ours[0].1, Outcome::Installed);
    }
}
