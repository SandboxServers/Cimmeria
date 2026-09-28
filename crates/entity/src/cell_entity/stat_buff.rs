//! Timed stat buffs held on an entity, and the ledger arithmetic behind them.
//!
//! The consumable stimpacks (Mark III / V / VII / X, items 6677-6733) raise a
//! primary attribute for an hour. Their effect rows carry `pulse_count = 1`,
//! which the pulsing layer never registers as a lasting instance, so the
//! duration lives here instead: one [`ActiveStatBuff`] per buffed stat in
//! [`CellEntity::stat_buffs`], expired by the cell's stat-buff tick. The
//! pet-side counterpart is `PetState::buffs` (pets PT-08); this ledger is the
//! same idea for any entity, players included.
//!
//! **The ledger is keyed by the stat, not by the effect.** Applying a buff
//! to a stat that already carries one takes the old buff off first
//! (restoring exactly what it moved) and then applies the new one. So a
//! Mark III Coordination stim (+5) followed by the Coordination half of a
//! Mark V stim (+7) leaves +7, not +12; re-using the same stim refreshes its
//! duration; and the two stats of a two-stat stim never collide, because
//! each lands on its own stat. The 2009 data does not say how the tiers
//! combined (none of these rows had a script), so this is a server-side
//! design decision, recorded in `docs/architecture/abilities-and-effects-system.md`.
//!
//! **Bounds widen instead of clamping.** A primary attribute sits at
//! `cur == max` (the archetype value), so `Stat::change` would clamp a
//! +5 to nothing. [`shift_stat_widening`] raises `max` (or lowers `min`)
//! only as far as the new value needs and records how far each bound
//! moved; [`unshift_stat`] takes back exactly that, so an expired buff
//! leaves the stat where it was, bounds included.

use std::time::Instant;

use crate::stats::StatList;

use super::CellEntity;

/// How far one buff moved each part of its stat. Removing the buff moves
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

/// One timed buff on one stat.
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveStatBuff {
    /// The stat it moves (`stats::stat_ids`).
    pub stat_id: i32,
    /// The delta the effect asked for (its NVP), before any bound.
    pub requested: i32,
    /// What it really moved, so removal restores exactly that.
    pub shift: StatShift,
    /// The effect that applied it (`resources.effects`). Also the client's
    /// duration-timer id.
    pub effect_id: i32,
    /// The ability the effect belongs to.
    pub ability_id: i32,
    /// Who applied it (the timer's source id).
    pub invoker_id: u32,
    /// The effect row's `EEffectFlag` bits (`EF_ClearOnDeath`, ...).
    pub effect_flags: u32,
    /// The buff's full length in seconds (the effect's `pulse_duration`).
    pub duration_secs: f32,
    /// When it lapses (server-local clock).
    pub expires_at: Instant,
    /// Whether the client has been sent its duration timer.
    pub timer_sent: bool,
}

/// What a caller asks [`CellEntity::apply_stat_buff`] to apply.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatBuffSpec {
    pub stat_id: i32,
    pub delta: i32,
    pub effect_id: i32,
    pub ability_id: i32,
    pub invoker_id: u32,
    pub effect_flags: u32,
    pub duration_secs: f32,
}

/// The stat buffs on an entity plus the client timer clears the
/// synchronous ledger owes. The cell's stat-buff tick sends both.
#[derive(Debug, Clone, Default)]
pub struct StatBuffLedger {
    /// At most one per stat.
    pub buffs: Vec<ActiveStatBuff>,
    /// `(effect_id, invoker_id)` pairs whose client duration timer must be
    /// cleared: the effect's last buff came off in synchronous code (an
    /// effect script, a replacement) that cannot send.
    pub pending_timer_clears: Vec<(i32, u32)>,
}

impl StatBuffLedger {
    /// Whether nothing is active or owed.
    pub fn is_idle(&self) -> bool {
        self.buffs.is_empty() && self.pending_timer_clears.is_empty()
    }
}

/// The result of [`CellEntity::apply_stat_buff`].
#[derive(Debug, Clone, PartialEq)]
pub struct StatBuffApplied {
    /// The buff now on the stat.
    pub applied: ActiveStatBuff,
    /// The buff it replaced on the same stat, already taken off.
    pub replaced: Option<ActiveStatBuff>,
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
    /// Apply a timed buff to `spec.stat_id`, first taking off any buff
    /// already on that stat (the stat-keyed rule in the module docs).
    /// Returns `None` when the entity has no such stat; nothing changes then.
    pub fn apply_stat_buff(&mut self, spec: StatBuffSpec, now: Instant) -> Option<StatBuffApplied> {
        self.stats.get(spec.stat_id)?;
        let replaced = self
            .stat_buffs
            .buffs
            .iter()
            .position(|b| b.stat_id == spec.stat_id)
            .map(|idx| self.remove_stat_buff_at(idx));
        let shift = shift_stat_widening(&mut self.stats, spec.stat_id, spec.delta)?;
        let applied = ActiveStatBuff {
            stat_id: spec.stat_id,
            requested: spec.delta,
            shift,
            effect_id: spec.effect_id,
            ability_id: spec.ability_id,
            invoker_id: spec.invoker_id,
            effect_flags: spec.effect_flags,
            duration_secs: spec.duration_secs,
            expires_at: now + std::time::Duration::from_secs_f32(spec.duration_secs.max(0.0)),
            timer_sent: false,
        };
        self.stat_buffs.buffs.push(applied.clone());
        // The new buff's start timer supersedes a clear owed for the same
        // effect (re-using the same stim).
        self.stat_buffs
            .pending_timer_clears
            .retain(|&(effect_id, _)| effect_id != spec.effect_id);
        Some(StatBuffApplied { applied, replaced })
    }

    /// Take the buff at `idx` off, restoring its stat. When it was the last
    /// buff of its effect, the effect's client timer clear is queued.
    ///
    /// # Panics
    ///
    /// When `idx` is out of range.
    pub fn remove_stat_buff_at(&mut self, idx: usize) -> ActiveStatBuff {
        let buff = self.stat_buffs.buffs.remove(idx);
        let _ = unshift_stat(&mut self.stats, buff.stat_id, buff.shift);
        if !self
            .stat_buffs
            .buffs
            .iter()
            .any(|b| b.effect_id == buff.effect_id)
        {
            self.stat_buffs
                .pending_timer_clears
                .push((buff.effect_id, buff.invoker_id));
        }
        buff
    }

    /// Take off every buff for which `pred` holds. Returns them in ledger
    /// order.
    pub fn remove_stat_buffs_where(
        &mut self,
        pred: impl Fn(&ActiveStatBuff) -> bool,
    ) -> Vec<ActiveStatBuff> {
        let mut removed = Vec::new();
        let mut idx = 0;
        while idx < self.stat_buffs.buffs.len() {
            if pred(&self.stat_buffs.buffs[idx]) {
                removed.push(self.remove_stat_buff_at(idx));
            } else {
                idx += 1;
            }
        }
        removed
    }
}
