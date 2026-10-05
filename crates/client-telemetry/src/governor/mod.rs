//! The telemetry governor: throttles and summarizes the upload stream
//! without losing what matters.
//!
//! Every event a hook hands to [`crate::queue::Producer::try_emit`] passes
//! through [`Governor::admit`] before it takes a ring slot. The governor
//! classifies it with the table in [`classify`] and then:
//!
//! | Class | What happens |
//! |---|---|
//! | [`Class::MustKeep`] | Forwarded untouched. Warn/error, failures, entity lifecycle, Mercury anomalies, hook install, boot. |
//! | [`Class::Hot`] | Never forwarded one by one; counted into the target's [`rollup::Rollup`]. |
//! | [`Class::PerEntity`] | Identical repeats collapse; the first K per (target, entity, key) are forwarded; the rest go into the rollup. |
//! | [`Class::Budgeted`] | Identical repeats collapse; forwarded while under the target's rate budget; the rest go into the rollup. |
//!
//! Every window ([`GovernorConfig::window_ms`], at every scene change (see
//! [`SCENE_TARGETS`]), and at shutdown) each non-empty rollup is emitted as one
//! `client.telemetry.rollup` event, and each collapsed run as one repeat
//! event (see [`collapse`]). Every [`GovernorConfig::health_every_ms`] a
//! `client.telemetry.health` event reports the totals, including the ring
//! and upload-retry drops that used to be silent.
//!
//! **Conservation.** For every target, the rows forwarded plus the sum of
//! `repeat_count` on its repeat events plus the sum of `count` on its
//! rollups equals the number of events raised. The volume test pins this.
//!
//! **What stays local.** The lab bridge's ring is fed by the hooks before
//! the producer (`hooks::emit`), so a lab investigation sees the
//! pre-governor stream. The `raw` capture switch turns the governor off for
//! the upload path too.

pub mod ability_timing;
pub mod budget;
pub mod classify;
pub mod collapse;
pub mod rollup;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};

use crate::capture::CaptureConfig;
use crate::events::ClientNativeEvent;

pub use classify::{classify, Class, KeepReason};
pub use rollup::{RollupReason, ROLLUP_TARGET};

use budget::Budget;
use collapse::{Collapser, Observed};
use rollup::Rollup;

/// Target of the periodic health event.
pub const HEALTH_TARGET: &str = "client.telemetry.health";

/// Rollups (one per target and reason) tracked before further targets
/// share one overflow rollup per reason.
pub const MAX_ROLLUP_TARGETS: usize = 256;

/// The shared overflow rollup's target.
pub const OVERFLOW_TARGET: &str = "<overflow>";

/// Targets whose `level_name` names the persistent map, so a new value is a
/// scene change. Only these count: other hooks reuse the field name for
/// something else (`client.ui.cegui_log` puts the CEGUI log severity in
/// it), and treating that as a map would close the window and reset the
/// per-entity first K on every change of log level.
pub const SCENE_TARGETS: &[&str] = &["client.streaming.update"];

/// (target, entity, key) pairs tracked for the per-entity first K. When
/// full the table restarts: a few extra events, never a lost first
/// sighting.
pub const MAX_ENTITY_PAIRS: usize = 8192;

/// Tuning. [`GovernorConfig::default`] is what a normal session runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GovernorConfig {
    /// Forward everything (the `raw` switch).
    pub raw: bool,
    /// Rollup window.
    pub window_ms: i64,
    /// Health event cadence.
    pub health_every_ms: i64,
    /// Budgeted events back to back per target.
    pub budget_burst: u32,
    /// Budgeted events per second per target, sustained.
    pub budget_per_sec: u32,
    /// Multiplier on each per-entity rule's first K.
    pub first_k_scale: u32,
}

impl Default for GovernorConfig {
    fn default() -> Self {
        Self {
            raw: false,
            window_ms: 10_000,
            health_every_ms: 60_000,
            budget_burst: 20,
            budget_per_sec: 2,
            first_k_scale: 1,
        }
    }
}

impl GovernorConfig {
    /// The configuration the capture switches ask for (`raw`, `firehose`;
    /// `unfilter` is the sinks' switch and does not affect the governor).
    pub fn from_capture(s: CaptureConfig) -> Self {
        let base = Self::default();
        if s.firehose {
            Self {
                raw: s.raw,
                budget_burst: base.budget_burst * 16,
                budget_per_sec: base.budget_per_sec * 16,
                first_k_scale: 4,
                ..base
            }
        } else {
            Self { raw: s.raw, ..base }
        }
    }

    /// `raw`, `firehose` or `governed`, for logs and the health event.
    pub fn mode(&self) -> &'static str {
        if self.raw {
            "raw"
        } else if self.first_k_scale > 1 {
            "firehose"
        } else {
            "governed"
        }
    }
}

/// Running totals, reported by the health event.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stats {
    /// Events admitted.
    pub seen: u64,
    /// Events forwarded one by one (must-keep included).
    pub forwarded: u64,
    /// Must-keep events (all forwarded).
    pub must_keep: u64,
    /// Events counted into rollups.
    pub rolled_up: u64,
    /// Events held as repeats.
    pub collapsed: u64,
    /// Rollup events emitted.
    pub rollup_events: u64,
    /// Repeat events emitted.
    pub repeat_events: u64,
    /// Per-entity table restarts.
    pub entity_table_resets: u64,
    /// Scene changes seen.
    pub scene_changes: u64,
}

/// Drop counters kept outside the governor, passed in by the uploader so
/// the health event can report them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExternalDrops {
    /// Events the producer ring refused because it was full.
    pub ring_dropped: u64,
    /// Events the uploader discarded after failed POSTs.
    pub upload_dropped: u64,
}

/// The governor state. One per process, behind a mutex (see
/// [`crate::queue::governed_channel`]).
#[derive(Debug)]
pub struct Governor {
    cfg: GovernorConfig,
    seq: Arc<AtomicU64>,
    collapser: Collapser,
    budget: Budget,
    entity_seen: HashMap<(String, i64, String), u32>,
    /// Keyed by (target, reason): a target can be absorbed for more than
    /// one reason (`client.cme.event` with and without an `entity_id`),
    /// and each rollup reports exactly one.
    rollups: BTreeMap<(String, &'static str), Rollup>,
    window_start_ms: Option<i64>,
    last_health_ms: Option<i64>,
    last_level_name: Option<String>,
    last_drops: ExternalDrops,
    stats: Stats,
}

impl Governor {
    /// A governor that stamps the events it generates from `seq`.
    pub fn new(cfg: GovernorConfig, seq: Arc<AtomicU64>) -> Self {
        Self {
            cfg,
            seq,
            collapser: Collapser::new(),
            budget: Budget::new(cfg.budget_burst, cfg.budget_per_sec),
            entity_seen: HashMap::new(),
            rollups: BTreeMap::new(),
            window_start_ms: None,
            last_health_ms: None,
            last_level_name: None,
            last_drops: ExternalDrops::default(),
            stats: Stats::default(),
        }
    }

    /// The configuration in force.
    pub fn config(&self) -> &GovernorConfig {
        &self.cfg
    }

    /// Running totals.
    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    /// Admit one stamped event. Whatever should go to the upload ring now
    /// is pushed to `out`, in order: rollups closed by a scene change, a
    /// repeat event the event ended, then the event itself.
    pub fn admit(&mut self, ev: ClientNativeEvent, out: &mut Vec<ClientNativeEvent>) {
        self.stats.seen += 1;
        let now = ev.ts_ms;
        self.window_start_ms.get_or_insert(now);
        self.last_health_ms.get_or_insert(now);

        // A new map closes the window, so rollups never mix two scenes, and
        // every entity gets a fresh first K for the new world entry.
        let scene = if SCENE_TARGETS.contains(&ev.target.as_str()) {
            ev.fields.get("level_name").and_then(Value::as_str)
        } else {
            None
        };
        if let Some(level) = scene {
            if self.last_level_name.as_deref() != Some(level) {
                if self.last_level_name.is_some() {
                    self.stats.scene_changes += 1;
                    self.close_window(now, "scene_change", out);
                    self.entity_seen.clear();
                }
                self.last_level_name = Some(level.to_string());
            }
        }

        let verdict = classify(&ev.target, &ev.level, &ev.fields);
        if self.cfg.raw {
            if matches!(verdict.class, Class::MustKeep(_)) {
                self.stats.must_keep += 1;
            }
            self.forward(ev, out);
            return;
        }

        match verdict.class {
            Class::MustKeep(_) => {
                self.stats.must_keep += 1;
                self.forward(ev, out);
            }
            Class::Hot => self.roll_up(&ev, RollupReason::HotStream, verdict.key),
            Class::PerEntity { first_k } => {
                if !self.collapse(&ev, out) {
                    return;
                }
                let entity = ev.fields.get("entity_id").and_then(Value::as_i64);
                match entity {
                    Some(id) => {
                        let k = first_k.saturating_mul(self.cfg.first_k_scale);
                        if self.first_k(&ev, id, verdict.key, k) {
                            self.forward(ev, out);
                        } else {
                            self.roll_up(&ev, RollupReason::PerEntityOverflow, verdict.key);
                        }
                    }
                    None => self.budgeted(ev, verdict.key, out),
                }
            }
            Class::Budgeted => {
                if self.collapse(&ev, out) {
                    self.budgeted(ev, verdict.key, out);
                }
            }
        }
    }

    /// Called on the uploader's cadence with wall time `now_ms`: closes the
    /// window when it is due and emits the health event when it is due.
    pub fn tick(&mut self, now_ms: i64, drops: ExternalDrops, out: &mut Vec<ClientNativeEvent>) {
        let start = *self.window_start_ms.get_or_insert(now_ms);
        if now_ms - start >= self.cfg.window_ms {
            self.close_window(now_ms, "window", out);
        }
        let last = *self.last_health_ms.get_or_insert(now_ms);
        if now_ms - last >= self.cfg.health_every_ms {
            self.emit_health(now_ms, drops, out);
        }
    }

    /// Close the window and emit health unconditionally (shutdown).
    pub fn finish(&mut self, now_ms: i64, drops: ExternalDrops, out: &mut Vec<ClientNativeEvent>) {
        self.close_window(now_ms, "shutdown", out);
        self.emit_health(now_ms, drops, out);
    }

    /// Returns `false` when the event was held as a repeat.
    fn collapse(&mut self, ev: &ClientNativeEvent, out: &mut Vec<ClientNativeEvent>) -> bool {
        match self.collapser.observe(ev) {
            Observed::Duplicate => {
                self.stats.collapsed += 1;
                false
            }
            Observed::New { ended } => {
                if let Some(r) = ended {
                    self.stats.repeat_events += 1;
                    self.push_generated(r, out);
                }
                true
            }
        }
    }

    fn budgeted(
        &mut self,
        ev: ClientNativeEvent,
        key: Option<&'static str>,
        out: &mut Vec<ClientNativeEvent>,
    ) {
        if self.budget.take(&ev.target, ev.ts_ms) {
            self.forward(ev, out);
        } else {
            self.roll_up(&ev, RollupReason::OverBudget, key);
        }
    }

    fn first_k(
        &mut self,
        ev: &ClientNativeEvent,
        entity: i64,
        key: Option<&'static str>,
        k: u32,
    ) -> bool {
        let pair = (
            ev.target.clone(),
            entity,
            rollup::key_of(&ev.fields, key).unwrap_or_default(),
        );
        if !self.entity_seen.contains_key(&pair) && self.entity_seen.len() >= MAX_ENTITY_PAIRS {
            self.entity_seen.clear();
            self.stats.entity_table_resets += 1;
        }
        let n = self.entity_seen.entry(pair).or_insert(0);
        *n += 1;
        *n <= k
    }

    fn roll_up(&mut self, ev: &ClientNativeEvent, reason: RollupReason, key: Option<&'static str>) {
        self.stats.rolled_up += 1;
        let id = (ev.target.clone(), reason.as_str());
        if let Some(r) = self.rollups.get_mut(&id) {
            r.add(ev, None);
        } else if self.rollups.len() < MAX_ROLLUP_TARGETS {
            let mut r = Rollup::new(&ev.target, reason, key);
            r.add(ev, None);
            self.rollups.insert(id, r);
        } else {
            // Key the shared overflow rollup by the real target, so its top
            // keys say which targets it absorbed (one per reason).
            self.rollups
                .entry((OVERFLOW_TARGET.to_string(), reason.as_str()))
                .or_insert_with(|| Rollup::new(OVERFLOW_TARGET, reason, None))
                .add(ev, Some(&ev.target));
        }
    }

    fn close_window(&mut self, now_ms: i64, trigger: &str, out: &mut Vec<ClientNativeEvent>) {
        let start = self.window_start_ms.unwrap_or(now_ms);
        for (_, r) in std::mem::take(&mut self.rollups) {
            if r.count() == 0 {
                continue;
            }
            self.stats.rollup_events += 1;
            let fields = r.to_fields(start, now_ms, trigger);
            self.push_generated(event(ROLLUP_TARGET, "info", now_ms, fields), out);
        }
        for r in self.collapser.flush() {
            self.stats.repeat_events += 1;
            self.push_generated(r, out);
        }
        self.window_start_ms = Some(now_ms);
    }

    fn emit_health(&mut self, now_ms: i64, drops: ExternalDrops, out: &mut Vec<ClientNativeEvent>) {
        // A drop since the last report is a non-happy outcome: warn, so the
        // report itself can never be summarized away and stands out.
        let new_drops = drops.ring_dropped > self.last_drops.ring_dropped
            || drops.upload_dropped > self.last_drops.upload_dropped;
        self.last_drops = drops;
        self.last_health_ms = Some(now_ms);
        let s = &self.stats;
        let mut f = BTreeMap::new();
        f.insert("mode".into(), json!(self.cfg.mode()));
        f.insert("seen_total".into(), json!(s.seen));
        f.insert("forwarded_total".into(), json!(s.forwarded));
        f.insert("must_keep_total".into(), json!(s.must_keep));
        f.insert("rolled_up_total".into(), json!(s.rolled_up));
        f.insert("collapsed_total".into(), json!(s.collapsed));
        f.insert("rollup_events_total".into(), json!(s.rollup_events));
        f.insert("repeat_events_total".into(), json!(s.repeat_events));
        f.insert("entity_table_resets".into(), json!(s.entity_table_resets));
        f.insert("scene_changes".into(), json!(s.scene_changes));
        f.insert(
            "uncollapsed_events_total".into(),
            json!(self.collapser.untracked()),
        );
        f.insert("ring_dropped_total".into(), json!(drops.ring_dropped));
        f.insert("upload_dropped_total".into(), json!(drops.upload_dropped));
        let level = if new_drops { "warn" } else { "info" };
        self.push_generated(event(HEALTH_TARGET, level, now_ms, f), out);
        // The AB-C6 timing histograms ride the health cadence (and the
        // shutdown flush): one event per non-empty series.
        for f in ability_timing::drain() {
            self.push_generated(event(ability_timing::TIMING_TARGET, "info", now_ms, f), out);
        }
    }

    fn forward(&mut self, ev: ClientNativeEvent, out: &mut Vec<ClientNativeEvent>) {
        self.stats.forwarded += 1;
        out.push(ev);
    }

    fn push_generated(&mut self, mut ev: ClientNativeEvent, out: &mut Vec<ClientNativeEvent>) {
        ev.seq = self.seq.fetch_add(1, Ordering::Relaxed);
        out.push(ev);
    }
}

fn event(
    target: &str,
    level: &str,
    ts_ms: i64,
    fields: BTreeMap<String, Value>,
) -> ClientNativeEvent {
    ClientNativeEvent {
        ts_ms,
        seq: 0,
        target: target.to_string(),
        level: level.to_string(),
        fields,
    }
}
