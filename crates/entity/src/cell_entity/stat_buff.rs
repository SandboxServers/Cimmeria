//! The timed effect ledger: stat changes that last a while and then come
//! off, and the arithmetic behind them (ability-mechanics D-AB08, AB-04;
//! decision 28 of the abilities ADR).
//!
//! A `pulse_count = 1` effect with a duration ("+200 Accuracy: 15 Seconds",
//! a stimpack's hour of +5 Coordination) never gets an `active_effects`
//! instance: the pulsing layer computes `remaining = pulse_count - 1 = 0`
//! and registers nothing, and raising `pulse_count` would make the pulse
//! tick re-run `on_apply` at expiry. So the duration lives here instead: one
//! [`TimedEffect`] per `(effect_id, invoker_id)` in
//! [`CellEntity::stat_buffs`], holding the stat deltas it applied, its expiry
//! (or `None` for an effect held until something removes it, a toggle), its
//! effect flags and its ability's moniker ids. The cell's stat-buff tick
//! expires entries and sends the client's duration timers.
//!
//! **Stacking** ([`TimedStacking`]). The contract rule is `PerSource`: the
//! same effect from the same invoker refreshes (the old entry comes off,
//! restoring exactly what it moved, and the new one goes on with a fresh
//! expiry); the same effect from a different invoker, or a different effect,
//! stacks. Python keyed effect instances the same way (`AbilityManager.
//! addEffect`), and so does the pulsing layer. The consumable stimpacks keep
//! decision 28's stat-keyed rule (`ReplaceSameStat`): a Mark III
//! Coordination stim (+5) followed by the Coordination half of a Mark V (+7)
//! leaves +7, not +12. The 2009 data does not say how the stim tiers
//! combined; that rule is a server-side design decision.
//!
//! **Bounds widen instead of clamping.** A primary attribute sits at
//! `cur == max` (the archetype value) and Defense sits at 0/0/0, so
//! `Stat::change` would clamp a +5 or a -100 to nothing.
//! [`shift_stat_widening`] raises `max` (or lowers `min`) only as far as
//! the new value needs and records how far each bound moved;
//! [`unshift_stat`] takes back exactly that, so an expired entry leaves the
//! stat where it was, bounds included.

use std::time::{Duration, Instant};

use crate::stats::StatList;

use super::CellEntity;

/// `EF_Beneficial_Effect` (1): which side of the client's effect bar an
/// entry's icon sits on. Mirrors `abilities::EF_BENEFICIAL_EFFECT`.
const EF_BENEFICIAL_EFFECT: u32 = 1;

/// The client's effect bar shows at most this many icons per side
/// (`Effect.lua` `MAX_BENEFICIAL` = `MAX_HARMFUL` = 10 from 0.8309 on,
/// audit B-73). The ledger never refuses an effect for it; the cell logs
/// the overflow.
pub const EFFECT_BAR_SLOTS_PER_SIDE: usize = 10;

/// How far one entry moved each part of one stat. Removing the entry moves
/// each part back by the same amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatShift {
    /// Change to `cur`.
    pub cur: i32,
    /// Change to `min` (non-zero only for a debuff that went below it).
    pub min: i32,
    /// Change to `max` (non-zero only for a buff that went above it).
    pub max: i32,
}

/// One stat an entry moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppliedStat {
    /// The stat (`stats::stat_ids`).
    pub stat_id: i32,
    /// The delta the effect asked for (its NVP), before any bound.
    pub requested: i32,
    /// What it really moved, so removal restores exactly that.
    pub shift: StatShift,
}

/// How a new entry treats the entries already on the entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimedStacking {
    /// One entry per `(effect_id, invoker_id)`: the same source refreshes,
    /// any other source stacks. The ledger contract's rule (D-AB08).
    #[default]
    PerSource,
    /// Decision 28's stimpack rule: the new entry first takes off every
    /// entry, from any source, that moves one of its stats.
    ReplaceSameStat,
}

/// One timed (or held) effect on one entity.
#[derive(Debug, Clone, PartialEq)]
pub struct TimedEffect {
    /// The effect that applied it (`resources.effects`). Also the client's
    /// duration-timer id and SecondaryId.
    pub effect_id: i32,
    /// The ability the effect belongs to.
    pub ability_id: i32,
    /// Who applied it (the timer's source id).
    pub invoker_id: u32,
    /// The effect row's `EEffectFlag` bits (`EF_ClearOnDeath`, ...).
    pub effect_flags: u32,
    /// The ability's moniker ids, for removal by moniker (AB-08, AB-10).
    pub moniker_ids: Vec<i64>,
    /// The stats it moved. Empty for an entry that only carries an icon
    /// and a duration.
    pub stats: Vec<AppliedStat>,
    /// Its full length in seconds (the effect's `pulse_duration`); 0 for a
    /// held entry.
    pub duration_secs: f32,
    /// When it lapses (server-local clock), or `None` while held.
    pub expires_at: Option<Instant>,
    /// Whether the client has been sent its duration timer.
    pub timer_sent: bool,
}

impl TimedEffect {
    /// The ledger key.
    pub fn key(&self) -> (i32, u32) {
        (self.effect_id, self.invoker_id)
    }

    /// Whether its icon sits on the beneficial side of the effect bar.
    pub fn is_beneficial(&self) -> bool {
        self.effect_flags & EF_BENEFICIAL_EFFECT != 0
    }

    /// Whether it moves `stat_id`.
    pub fn moves(&self, stat_id: i32) -> bool {
        self.stats.iter().any(|s| s.stat_id == stat_id)
    }

    /// Whether it carries `moniker_id`.
    pub fn has_moniker(&self, moniker_id: i64) -> bool {
        self.moniker_ids.contains(&moniker_id)
    }

    /// Whether it has lapsed by `now`. A held entry never does.
    pub fn is_expired(&self, now: Instant) -> bool {
        self.expires_at.is_some_and(|t| t <= now)
    }
}

/// What a caller asks [`CellEntity::apply_timed_effect`] to apply.
#[derive(Debug, Clone, PartialEq)]
pub struct TimedEffectSpec {
    pub effect_id: i32,
    pub ability_id: i32,
    pub invoker_id: u32,
    pub effect_flags: u32,
    pub moniker_ids: Vec<i64>,
    /// `(stat_id, delta)` pairs. Zero deltas are dropped.
    pub stats: Vec<(i32, i32)>,
    /// Seconds until it lapses, or `None` to hold it until removed.
    pub duration_secs: Option<f32>,
    pub stacking: TimedStacking,
}

/// The stat-changing effects on an entity plus the client timer clears the
/// synchronous ledger owes. The cell's stat-buff tick sends both.
#[derive(Debug, Clone, Default)]
pub struct StatBuffLedger {
    /// At most one per `(effect_id, invoker_id)`.
    pub entries: Vec<TimedEffect>,
    /// `(effect_id, invoker_id)` pairs whose client duration timer must be
    /// cleared: the entry came off in synchronous code (an effect script, a
    /// replacement) that cannot send.
    pub pending_timer_clears: Vec<(i32, u32)>,
}

impl StatBuffLedger {
    /// Whether nothing is active or owed.
    pub fn is_idle(&self) -> bool {
        self.entries.is_empty() && self.pending_timer_clears.is_empty()
    }

    /// Icons on one side of the client's effect bar: the entries whose
    /// beneficial bit is `beneficial` (each entry is one timer, so one icon).
    pub fn bar_icons(&self, beneficial: bool) -> usize {
        self.entries
            .iter()
            .filter(|e| e.is_beneficial() == beneficial)
            .count()
    }
}

/// The result of [`CellEntity::apply_timed_effect`].
#[derive(Debug, Clone, PartialEq)]
pub struct TimedEffectApplied {
    /// The entry now on the entity.
    pub applied: TimedEffect,
    /// The entries it took off first, already reverted: the same source's
    /// earlier application (a refresh) or, under `ReplaceSameStat`, every
    /// entry on one of its stats.
    pub replaced: Vec<TimedEffect>,
    /// Stats in the spec the entity does not have; skipped.
    pub missing_stats: Vec<i32>,
}

/// Move `stat` by `delta`, widening a bound instead of clamping to it.
/// Returns what moved, or `None` when the entity has no such stat.
pub fn shift_stat_widening(stats: &mut StatList, stat: i32, delta: i32) -> Option<StatShift> {
    let s = stats.get_mut(stat)?;
    let (min, cur, max) = (s.min, s.cur, s.max);
    let new_cur = cur.saturating_add(delta);
    let new_min = min.min(new_cur);
    let new_max = max.max(new_cur);
    s.update(new_min, new_cur, new_max);
    Some(StatShift {
        cur: new_cur - cur,
        min: new_min - min,
        max: new_max - max,
    })
}

/// Take back a [`StatShift`]. `cur` is clamped into the restored bounds in
/// case something else moved the stat meanwhile. Returns `None` when the
/// entity has no such stat.
pub fn unshift_stat(stats: &mut StatList, stat: i32, shift: StatShift) -> Option<()> {
    let s = stats.get_mut(stat)?;
    let new_min = s.min.saturating_sub(shift.min);
    let new_max = s.max.saturating_sub(shift.max).max(new_min);
    let new_cur = s.cur.saturating_sub(shift.cur).clamp(new_min, new_max);
    s.update(new_min, new_cur, new_max);
    Some(())
}

impl CellEntity {
    /// Apply a timed effect, first taking off what [`TimedStacking`] says it
    /// replaces. Returns `None`, changing nothing, when the spec names stats
    /// and the entity has none of them. A spec with no stats applies an
    /// icon-only entry.
    pub fn apply_timed_effect(
        &mut self,
        spec: TimedEffectSpec,
        now: Instant,
    ) -> Option<TimedEffectApplied> {
        let wanted: Vec<(i32, i32)> = spec
            .stats
            .iter()
            .copied()
            .filter(|&(_, d)| d != 0)
            .collect();
        let (present, missing_stats): (Vec<(i32, i32)>, Vec<(i32, i32)>) = wanted
            .iter()
            .partition(|&&(stat, _)| self.stats.get(stat).is_some());
        if !wanted.is_empty() && present.is_empty() {
            return None;
        }
        let key = (spec.effect_id, spec.invoker_id);
        let replaced = self.remove_timed_effects_where(|e| {
            e.key() == key
                || (spec.stacking == TimedStacking::ReplaceSameStat
                    && present.iter().any(|&(stat, _)| e.moves(stat)))
        });
        let stats = present
            .iter()
            .filter_map(|&(stat_id, delta)| {
                shift_stat_widening(&mut self.stats, stat_id, delta).map(|shift| AppliedStat {
                    stat_id,
                    requested: delta,
                    shift,
                })
            })
            .collect();
        let applied = TimedEffect {
            effect_id: spec.effect_id,
            ability_id: spec.ability_id,
            invoker_id: spec.invoker_id,
            effect_flags: spec.effect_flags,
            moniker_ids: spec.moniker_ids,
            stats,
            duration_secs: spec.duration_secs.unwrap_or(0.0).max(0.0),
            expires_at: spec
                .duration_secs
                .map(|d| now + Duration::from_secs_f32(d.max(0.0))),
            timer_sent: false,
        };
        self.stat_buffs.entries.push(applied.clone());
        // The new start timer supersedes a clear owed for the same key (a
        // refresh, or re-using the same stim).
        self.stat_buffs.pending_timer_clears.retain(|&k| k != key);
        Some(TimedEffectApplied {
            applied,
            replaced,
            missing_stats: missing_stats.into_iter().map(|(s, _)| s).collect(),
        })
    }

    /// Take the entry at `idx` off, restoring every stat it moved, and queue
    /// its client timer clear.
    ///
    /// # Panics
    ///
    /// When `idx` is out of range.
    pub fn remove_timed_effect_at(&mut self, idx: usize) -> TimedEffect {
        let entry = self.stat_buffs.entries.remove(idx);
        for s in &entry.stats {
            let _ = unshift_stat(&mut self.stats, s.stat_id, s.shift);
        }
        if !self.stat_buffs.pending_timer_clears.contains(&entry.key()) {
            self.stat_buffs.pending_timer_clears.push(entry.key());
        }
        entry
    }

    /// Take off every entry for which `pred` holds. Returns them in ledger
    /// order.
    pub fn remove_timed_effects_where(
        &mut self,
        pred: impl Fn(&TimedEffect) -> bool,
    ) -> Vec<TimedEffect> {
        let mut removed = Vec::new();
        let mut idx = 0;
        while idx < self.stat_buffs.entries.len() {
            if pred(&self.stat_buffs.entries[idx]) {
                removed.push(self.remove_timed_effect_at(idx));
            } else {
                idx += 1;
            }
        }
        removed
    }
}
