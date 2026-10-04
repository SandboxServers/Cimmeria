//! Ability timing histograms (ability-mechanics AB-C6), shipped by the
//! uploader as `client.ability.timing` events.
//!
//! The ability hooks measure three intervals on the client's own clock
//! (milliseconds of the process's monotonic clock): press to send, send to
//! the first matching receive, and receive to applied. Each measurement is
//! also a field on its own event; the histogram is what survives the
//! per-name throttle, which drops events but never their observations
//! (they are recorded before it).
//!
//! **Labels are enumerations only.** `stage` is one of [`STAGES`]; `label`
//! is a method name from the hooks' own allowlists or an `applied` kind,
//! never an id. At most [`MAX_SERIES`] series are kept; any further label
//! shares the stage's `other` series, so the event count stays bounded.
//!
//! The governor drains the histograms on its health cadence and at
//! shutdown ([`super::Governor`]); a series with no observation in the
//! window writes no event.

use std::collections::BTreeMap;
#[cfg(not(test))]
use std::sync::Mutex;

use serde_json::{json, Value};

/// Target of the histogram events.
pub const TIMING_TARGET: &str = "client.ability.timing";

/// The measured intervals.
pub const STAGES: &[&str] = &["press_to_sent", "sent_to_recv", "recv_to_applied"];

/// Upper bounds of the buckets, in milliseconds. A value above the last
/// bound counts in the overflow bucket (`gt_10000_ms`).
pub const BUCKETS_MS: &[u64] = &[10, 25, 50, 100, 250, 500, 1000, 2500, 5000, 10000];

/// Series (stage, label) kept before new labels fold into `other`.
pub const MAX_SERIES: usize = 64;

/// The label a series beyond [`MAX_SERIES`] is counted under.
pub const OTHER_LABEL: &str = "other";

/// One series: bucket counts plus count, sum, min and max.
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    /// `BUCKETS_MS.len() + 1` counts; the last is the overflow.
    pub buckets: Vec<u64>,
    /// Observations.
    pub count: u64,
    /// Sum of the observations.
    pub sum_ms: u64,
    /// Smallest observation.
    pub min_ms: u64,
    /// Largest observation.
    pub max_ms: u64,
}

impl Series {
    fn new() -> Self {
        Self {
            buckets: vec![0; BUCKETS_MS.len() + 1],
            count: 0,
            sum_ms: 0,
            min_ms: u64::MAX,
            max_ms: 0,
        }
    }

    fn add(&mut self, ms: u64) {
        let i = BUCKETS_MS
            .iter()
            .position(|&b| ms <= b)
            .unwrap_or(BUCKETS_MS.len());
        self.buckets[i] += 1;
        self.count += 1;
        self.sum_ms = self.sum_ms.saturating_add(ms);
        self.min_ms = self.min_ms.min(ms);
        self.max_ms = self.max_ms.max(ms);
    }

    /// The event fields for this series.
    fn fields(&self, stage: &str, label: &str) -> BTreeMap<String, Value> {
        let mut f = BTreeMap::new();
        f.insert("stage".into(), json!(stage));
        f.insert("label".into(), json!(label));
        f.insert("count".into(), json!(self.count));
        f.insert("sum_ms".into(), json!(self.sum_ms));
        f.insert("min_ms".into(), json!(self.min_ms));
        f.insert("max_ms".into(), json!(self.max_ms));
        for (i, n) in self.buckets.iter().enumerate() {
            let name = match BUCKETS_MS.get(i) {
                Some(b) => format!("le_{b}_ms"),
                None => format!("gt_{}_ms", BUCKETS_MS[BUCKETS_MS.len() - 1]),
            };
            f.insert(name, json!(n));
        }
        f
    }
}

/// Every series of one window.
#[derive(Debug, Default)]
pub struct Histograms {
    series: BTreeMap<(&'static str, &'static str), Series>,
}

impl Histograms {
    /// Record `ms` under `stage` / `label`. An unknown stage is ignored: the
    /// callers pass constants from [`STAGES`].
    pub fn observe(&mut self, stage: &'static str, label: &'static str, ms: u64) {
        let Some(&stage) = STAGES.iter().find(|s| **s == stage) else {
            return;
        };
        let key = if self.series.contains_key(&(stage, label)) || self.series.len() < MAX_SERIES {
            (stage, label)
        } else {
            (stage, OTHER_LABEL)
        };
        self.series.entry(key).or_insert_with(Series::new).add(ms);
    }

    /// The window's events' fields, one per non-empty series, and reset.
    pub fn drain(&mut self) -> Vec<BTreeMap<String, Value>> {
        std::mem::take(&mut self.series)
            .into_iter()
            .map(|((stage, label), s)| s.fields(stage, label))
            .collect()
    }
}

#[cfg(not(test))]
static HISTOGRAMS: Mutex<Option<Histograms>> = Mutex::new(None);

#[cfg(not(test))]
fn with<R>(f: impl FnOnce(&mut Histograms) -> R) -> R {
    let mut g = HISTOGRAMS.lock().unwrap_or_else(|e| e.into_inner());
    f(g.get_or_insert_with(Histograms::default))
}

// Per test thread, so a hook test's observations never show up in a
// governor test running beside it.
#[cfg(test)]
thread_local! {
    static HISTOGRAMS: std::cell::RefCell<Histograms> = std::cell::RefCell::default();
}

#[cfg(test)]
fn with<R>(f: impl FnOnce(&mut Histograms) -> R) -> R {
    HISTOGRAMS.with(|h| f(&mut h.borrow_mut()))
}

/// Record one interval (the hooks call this). Poisoning is ignored.
pub fn observe(stage: &'static str, label: &'static str, ms: u64) {
    with(|h| h.observe(stage, label, ms));
}

/// Take the window's series (the governor calls this).
pub fn drain() -> Vec<BTreeMap<String, Value>> {
    with(Histograms::drain)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_land_in_the_first_bucket_that_holds_them() {
        let mut h = Histograms::default();
        for ms in [0, 10, 11, 250, 251, 10_000, 10_001, 60_000] {
            h.observe("sent_to_recv", "useAbility", ms);
        }
        let ev = h.drain();
        assert_eq!(ev.len(), 1);
        let f = &ev[0];
        assert_eq!(f["stage"], "sent_to_recv");
        assert_eq!(f["label"], "useAbility");
        assert_eq!(f["count"], 8);
        assert_eq!(f["le_10_ms"], 2);
        assert_eq!(f["le_25_ms"], 1);
        assert_eq!(f["le_250_ms"], 1);
        assert_eq!(f["le_500_ms"], 1);
        assert_eq!(f["le_10000_ms"], 1);
        assert_eq!(f["gt_10000_ms"], 2);
        assert_eq!(f["min_ms"], 0);
        assert_eq!(f["max_ms"], 60_000);
        assert_eq!(f["sum_ms"], 10 + 11 + 250 + 251 + 10_000 + 10_001 + 60_000);
        assert!(h.drain().is_empty(), "drain resets the window");
    }

    /// The governor ships the window's series with its health event (and
    /// at shutdown), then starts a new window.
    #[test]
    fn the_governor_ships_the_histograms_on_its_health_cadence() {
        use super::super::{ExternalDrops, Governor, GovernorConfig};
        let mut g = Governor::new(GovernorConfig::default(), Default::default());
        observe("recv_to_applied", "effect_bar", 40);
        observe("recv_to_applied", "effect_bar", 60);
        observe("press_to_sent", "useAbility", 3);
        let mut out = Vec::new();
        g.finish(1_000, ExternalDrops::default(), &mut out);
        let timing: Vec<_> = out.iter().filter(|e| e.target == TIMING_TARGET).collect();
        assert_eq!(timing.len(), 2, "{out:#?}");
        let applied = timing
            .iter()
            .find(|e| e.fields["stage"] == "recv_to_applied")
            .unwrap();
        assert_eq!(applied.fields["count"], 2);
        assert_eq!(applied.fields["le_50_ms"], 1);
        assert_eq!(applied.fields["le_100_ms"], 1);
        let mut again = Vec::new();
        g.finish(2_000, ExternalDrops::default(), &mut again);
        assert!(again.iter().all(|e| e.target != TIMING_TARGET));
    }

    #[test]
    fn series_are_bounded_and_unknown_stages_ignored() {
        // Leak a few distinct labels; production labels are constants.
        let labels: Vec<&'static str> = (0..MAX_SERIES + 5)
            .map(|i| &*Box::leak(format!("m{i}").into_boxed_str()))
            .collect();
        let mut h = Histograms::default();
        for l in &labels {
            h.observe("press_to_sent", l, 5);
        }
        h.observe("not_a_stage", "useAbility", 5);
        let ev = h.drain();
        assert_eq!(ev.len(), MAX_SERIES + 1, "MAX_SERIES plus the other series");
        let other = ev.iter().find(|f| f["label"] == OTHER_LABEL).unwrap();
        assert_eq!(other["count"], 5);
        assert!(ev.iter().all(|f| f["stage"] == "press_to_sent"));
    }
}
