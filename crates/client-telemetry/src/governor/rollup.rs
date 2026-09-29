//! The per-target summary of events the governor did not forward.
//!
//! One [`Rollup`] per target per window. It keeps enough to recover the
//! total and the shape in SigNoz: the exact count, first and last
//! timestamps, the distinct values of one key field with an exact top-N,
//! min / max / sum / n of every numeric field, the last key seen and the
//! last event's fields as an exemplar (for a load freeze, "what was loading
//! last" is the question).
//!
//! Everything is bounded: [`MAX_KEYS`] distinct keys (the rest are counted
//! in `untracked_key_events`), [`MAX_NUMERIC_FIELDS`] numeric fields.

use std::collections::{BTreeMap, HashMap};

use serde_json::{json, Map, Value};

use crate::events::ClientNativeEvent;

use super::classify::FALLBACK_KEY_FIELDS;

/// Keys listed in a rollup's `top_keys`.
pub const TOP_N: usize = 10;

/// Distinct keys tracked per rollup before new ones are counted only in
/// `untracked_key_events`.
pub const MAX_KEYS: usize = 256;

/// Numeric fields tracked per rollup.
pub const MAX_NUMERIC_FIELDS: usize = 16;

/// Target every rollup event is emitted under.
pub const ROLLUP_TARGET: &str = "client.telemetry.rollup";

/// min / max / sum / n of one numeric field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumStat {
    /// Values seen.
    pub n: u64,
    /// Smallest value.
    pub min: f64,
    /// Largest value.
    pub max: f64,
    /// Sum of values.
    pub sum: f64,
}

impl NumStat {
    fn new(v: f64) -> Self {
        Self {
            n: 1,
            min: v,
            max: v,
            sum: v,
        }
    }

    fn add(&mut self, v: f64) {
        self.n += 1;
        self.min = self.min.min(v);
        self.max = self.max.max(v);
        self.sum += v;
    }
}

/// Why events ended up in a rollup instead of being forwarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollupReason {
    /// A [`super::Class::Hot`] stream.
    HotStream,
    /// A [`super::Class::Budgeted`] target over its rate budget.
    OverBudget,
    /// A [`super::Class::PerEntity`] pair past its first K.
    PerEntityOverflow,
}

impl RollupReason {
    /// The `reason` field value.
    pub fn as_str(self) -> &'static str {
        match self {
            RollupReason::HotStream => "hot_stream",
            RollupReason::OverBudget => "over_budget",
            RollupReason::PerEntityOverflow => "per_entity_overflow",
        }
    }
}

/// The summary of one target's absorbed events in one window.
#[derive(Debug, Clone)]
pub struct Rollup {
    target: String,
    reason: RollupReason,
    key_field: Option<&'static str>,
    count: u64,
    first_ts_ms: i64,
    last_ts_ms: i64,
    keys: HashMap<String, u64>,
    untracked_key_events: u64,
    last_key: Option<String>,
    numeric: BTreeMap<String, NumStat>,
    last_fields: BTreeMap<String, Value>,
}

impl Rollup {
    /// An empty rollup for `target`.
    pub fn new(target: &str, reason: RollupReason, key_field: Option<&'static str>) -> Self {
        Self {
            target: target.to_string(),
            reason,
            key_field,
            count: 0,
            first_ts_ms: 0,
            last_ts_ms: 0,
            keys: HashMap::new(),
            untracked_key_events: 0,
            last_key: None,
            numeric: BTreeMap::new(),
            last_fields: BTreeMap::new(),
        }
    }

    /// Events absorbed so far.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Add one event. `key_override` replaces the key-field lookup (the
    /// overflow rollup keys by the original target).
    pub fn add(&mut self, ev: &ClientNativeEvent, key_override: Option<&str>) {
        if self.count == 0 {
            self.first_ts_ms = ev.ts_ms;
        }
        self.count += 1;
        self.first_ts_ms = self.first_ts_ms.min(ev.ts_ms);
        self.last_ts_ms = self.last_ts_ms.max(ev.ts_ms);

        let key = match key_override {
            Some(k) => Some(k.to_string()),
            None => key_of(&ev.fields, self.key_field),
        };
        if let Some(k) = key {
            if let Some(n) = self.keys.get_mut(&k) {
                *n += 1;
            } else if self.keys.len() < MAX_KEYS {
                self.keys.insert(k.clone(), 1);
            } else {
                self.untracked_key_events += 1;
            }
            self.last_key = Some(k);
        }

        for (name, v) in &ev.fields {
            let Some(x) = v.as_f64() else { continue };
            if let Some(s) = self.numeric.get_mut(name) {
                s.add(x);
            } else if self.numeric.len() < MAX_NUMERIC_FIELDS {
                self.numeric.insert(name.clone(), NumStat::new(x));
            }
        }
        self.last_fields.clone_from(&ev.fields);
    }

    /// The exact top-N keys: by count descending, then key ascending.
    pub fn top_keys(&self) -> Vec<(String, u64)> {
        let mut v: Vec<(String, u64)> = self.keys.iter().map(|(k, n)| (k.clone(), *n)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v.truncate(TOP_N);
        v
    }

    /// The fields of the rollup event for the window `[window_start_ms,
    /// window_end_ms)`, closed by `trigger` (`window`, `scene_change`,
    /// `shutdown`).
    pub fn to_fields(
        &self,
        window_start_ms: i64,
        window_end_ms: i64,
        trigger: &str,
    ) -> BTreeMap<String, Value> {
        let window_ms = (window_end_ms - window_start_ms).max(1);
        // Rounded to thousandths so the value is stable across platforms.
        let rate = ((self.count as f64) * 1000.0 / window_ms as f64 * 1000.0).round() / 1000.0;
        let top = self.top_keys();
        let top_sum: u64 = top.iter().map(|(_, n)| n).sum();
        let keyed: u64 = self.keys.values().sum::<u64>() + self.untracked_key_events;

        let mut f = BTreeMap::new();
        f.insert("rollup_target".into(), json!(self.target));
        f.insert("reason".into(), json!(self.reason.as_str()));
        f.insert("trigger".into(), json!(trigger));
        f.insert("count".into(), json!(self.count));
        f.insert("window_start_ms".into(), json!(window_start_ms));
        f.insert("window_end_ms".into(), json!(window_end_ms));
        f.insert("window_ms".into(), json!(window_ms));
        f.insert("rate_per_sec".into(), json!(rate));
        f.insert("first_ts_ms".into(), json!(self.first_ts_ms));
        f.insert("last_ts_ms".into(), json!(self.last_ts_ms));
        if keyed > 0 {
            if let Some(k) = self.key_field {
                f.insert("key_field".into(), json!(k));
            }
            f.insert("distinct_keys".into(), json!(self.keys.len()));
            f.insert(
                "distinct_keys_capped".into(),
                json!(self.untracked_key_events > 0),
            );
            f.insert(
                "top_keys".into(),
                Value::Array(
                    top.iter()
                        .map(|(k, n)| json!({"key": k, "count": n}))
                        .collect(),
                ),
            );
            // Keyed events outside the top N, so the top list plus this
            // always adds back up to the keyed total.
            f.insert("other_key_events".into(), json!(keyed - top_sum));
            if let Some(k) = &self.last_key {
                f.insert("last_key".into(), json!(k));
            }
        }
        if !self.numeric.is_empty() {
            let mut m = Map::new();
            for (name, s) in &self.numeric {
                m.insert(
                    name.clone(),
                    json!({"n": s.n, "min": s.min, "max": s.max, "sum": s.sum}),
                );
            }
            f.insert("numeric".into(), Value::Object(m));
        }
        if !self.last_fields.is_empty() {
            f.insert(
                "last_fields".into(),
                Value::Object(self.last_fields.clone().into_iter().collect()),
            );
        }
        f
    }
}

/// The key value of an event: the named field, or the first fallback field
/// present. Strings are used as is; other values in their JSON form.
pub fn key_of(fields: &BTreeMap<String, Value>, key_field: Option<&str>) -> Option<String> {
    let v = match key_field {
        Some(k) => fields.get(k),
        None => FALLBACK_KEY_FIELDS.iter().find_map(|k| fields.get(*k)),
    }?;
    Some(match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}
