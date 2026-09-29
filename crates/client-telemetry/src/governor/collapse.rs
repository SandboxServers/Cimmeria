//! Duplicate collapse: identical consecutive events on one target become
//! one event plus a repeat count.
//!
//! "Consecutive" is per target: the previous event *on the same target*.
//! Hooks fire on several game threads, so a global "previous event" would
//! almost never match and the collapse would do nothing.
//!
//! The first event of a run is forwarded as usual. Its identical followers
//! are held as a count. When a different event arrives on the target, or
//! the window closes, one **repeat event** is emitted: the same target,
//! level and fields, plus `repeat_count` (followers held, not counting the
//! first), `repeat_first_ts_ms` and `repeat_last_ts_ms`. So for any target,
//! rows + the sum of `repeat_count` is the number of events raised.
//!
//! At most [`MAX_RUNS`] targets are tracked; events on further targets are
//! never collapsed (forwarded, not lost).

use std::collections::{BTreeMap, HashMap};

use serde_json::{json, Value};

use crate::events::ClientNativeEvent;

/// Targets tracked for collapse.
pub const MAX_RUNS: usize = 256;

#[derive(Debug, Clone)]
struct Run {
    level: String,
    fields: BTreeMap<String, Value>,
    repeats: u64,
    first_repeat_ts_ms: i64,
    last_repeat_ts_ms: i64,
}

impl Run {
    fn start(ev: &ClientNativeEvent) -> Self {
        Self {
            level: ev.level.clone(),
            fields: ev.fields.clone(),
            repeats: 0,
            first_repeat_ts_ms: 0,
            last_repeat_ts_ms: 0,
        }
    }

    /// The repeat event for the held followers, if any, and reset the count.
    fn take_summary(&mut self, target: &str) -> Option<ClientNativeEvent> {
        if self.repeats == 0 {
            return None;
        }
        let mut fields = self.fields.clone();
        fields.insert("repeat_count".into(), json!(self.repeats));
        fields.insert("repeat_first_ts_ms".into(), json!(self.first_repeat_ts_ms));
        fields.insert("repeat_last_ts_ms".into(), json!(self.last_repeat_ts_ms));
        let ev = ClientNativeEvent {
            ts_ms: self.last_repeat_ts_ms,
            seq: 0,
            target: target.to_string(),
            level: self.level.clone(),
            fields,
        };
        self.repeats = 0;
        Some(ev)
    }
}

/// What [`Collapser::observe`] decided.
#[derive(Debug)]
pub enum Observed {
    /// Identical to the previous event on the target: held as a repeat.
    Duplicate,
    /// A new event. `ended` is the repeat event of the run it ended, to be
    /// emitted before it.
    New {
        /// The repeat event of the previous run, if it had followers.
        ended: Option<ClientNativeEvent>,
    },
}

/// The per-target run table.
#[derive(Debug, Default)]
pub struct Collapser {
    runs: HashMap<String, Run>,
    untracked: u64,
}

impl Collapser {
    /// An empty table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Events that could not be tracked because the table was full.
    pub fn untracked(&self) -> u64 {
        self.untracked
    }

    /// Look at one event.
    pub fn observe(&mut self, ev: &ClientNativeEvent) -> Observed {
        if let Some(run) = self.runs.get_mut(&ev.target) {
            if run.level == ev.level && run.fields == ev.fields {
                if run.repeats == 0 {
                    run.first_repeat_ts_ms = ev.ts_ms;
                }
                run.repeats += 1;
                run.last_repeat_ts_ms = ev.ts_ms;
                return Observed::Duplicate;
            }
            let ended = run.take_summary(&ev.target);
            *run = Run::start(ev);
            return Observed::New { ended };
        }
        if self.runs.len() < MAX_RUNS {
            self.runs.insert(ev.target.clone(), Run::start(ev));
        } else {
            self.untracked += 1;
        }
        Observed::New { ended: None }
    }

    /// The repeat events of every run with held followers, in target order.
    /// Runs stay open, so an identical event after the flush is still a
    /// duplicate (one repeat event per window for a steady repeat).
    pub fn flush(&mut self) -> Vec<ClientNativeEvent> {
        let mut out: Vec<ClientNativeEvent> = self
            .runs
            .iter_mut()
            .filter_map(|(t, r)| r.take_summary(t))
            .collect();
        out.sort_by(|a, b| a.target.cmp(&b.target));
        out
    }
}
