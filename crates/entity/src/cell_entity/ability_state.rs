//! The ability state snapshot (ability-mechanics AB-T5): everything the
//! server believes about one entity's casts, cooldowns and effects, copied
//! out into one owned, serde-serialisable value.
//!
//! One builder, [`CellEntity::ability_state`], feeds three readers so they
//! can never disagree:
//!
//! - the `abilities.snapshot` INFO row (`.bug` bookmarks, deaths, logouts),
//!   written by `cimmeria-cell-world`'s `effects::ability_snapshot`;
//! - the lab's `server_ability_state` tool (`LabQuery::AbilityState`);
//! - the GM `.effects` readout.
//!
//! **Copied out, never borrowed.** Every field is a primitive or a small
//! owned vector, and every time is "seconds from `now`", not an `Instant`,
//! so the value survives a round trip through JSON and means the same thing
//! on the far side. Times are rounded to the millisecond to keep the
//! telemetry row compact.
//!
//! **Expired but not yet swept.** A cooldown the cleanup has not reached yet
//! still sits in the ability manager's map; the snapshot leaves it out
//! (`remaining <= 0`), since the gate already treats it as over. A ledger
//! entry past its expiry stays in, with `expires_in_secs = 0`, because it
//! still moves its stats until the stat-buff tick takes it off.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::stat_buff::TimedEffect;
use super::CellEntity;

/// One entity's ability state at one moment. See the module docs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbilityStateSnapshot {
    pub entity_id: u32,
    pub is_player: bool,
    /// The raw `state_field` (`BSF_*` bits).
    pub state_field: u32,
    /// The cast in its warmup, if any.
    pub pending_cast: Option<PendingCastState>,
    /// Running ability cooldowns, by ability id.
    pub cooldowns: Vec<CooldownState>,
    /// Running moniker-group cooldowns, by moniker id.
    pub moniker_cooldowns: Vec<MonikerCooldownState>,
    /// Pulsing effects (DoTs, HoTs, channels) on the entity.
    pub pulsing: Vec<PulsingEffectState>,
    /// Timed and held entries of the effect ledger, absorb pools included.
    pub ledger: Vec<LedgerEntryState>,
    /// Effect icons the ledger still owes the client a clear for:
    /// `(effect_id, invoker_id)`.
    pub pending_timer_clears: Vec<(i32, u32)>,
    /// Counted `state_field` references, one row per bit with a count.
    pub state_flag_refcounts: Vec<StateFlagRefcount>,
    /// Every stat the entity carries, by stat id.
    pub stats: Vec<StatState>,
}

/// A launched cast waiting for its warmup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingCastState {
    pub ability_id: i32,
    /// The cast's correlator (AB-T1).
    pub cast_id: i32,
    pub target_id: i32,
    pub wire_target_id: i32,
    /// Warmup length after the speed modifiers.
    pub warmup_secs: f32,
    /// Time left before it fires; 0 once due.
    pub warmup_remaining_secs: f32,
}

/// One running ability cooldown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CooldownState {
    pub ability_id: i32,
    pub remaining_secs: f32,
    pub total_secs: f32,
}

/// One running moniker-group cooldown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonikerCooldownState {
    pub moniker_id: i64,
    pub remaining_secs: f32,
    pub total_secs: f32,
}

/// One pulsing effect instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PulsingEffectState {
    pub effect_id: i32,
    pub ability_id: i32,
    pub invoker_id: u32,
    /// The cast that registered (or last refreshed) it; `None` outside a cast.
    pub cast_id: Option<i32>,
    pub pulses_left: i32,
    pub total_pulses: i32,
    pub pulse_interval_secs: f32,
    /// Time to its next pulse; 0 once due.
    pub next_pulse_in_secs: f32,
}

/// One stat a ledger entry moved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerStatDelta {
    pub stat_id: i32,
    /// The delta the effect asked for.
    pub requested: i32,
    /// What it really moved on `cur`, `min` and `max`.
    pub cur_shift: i32,
    pub min_shift: i32,
    pub max_shift: i32,
}

/// One absorb pool of a shield entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AbsorbPoolState {
    pub stat_id: i32,
    pub granted: i32,
    pub remaining: i32,
}

/// One timed (or held) ledger entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerEntryState {
    pub effect_id: i32,
    pub ability_id: i32,
    pub invoker_id: u32,
    pub cast_id: Option<i32>,
    pub stats: Vec<LedgerStatDelta>,
    /// Its full length; 0 for a held entry.
    pub duration_secs: f32,
    /// Time to its expiry, 0 once lapsed; `None` while held.
    pub expires_in_secs: Option<f32>,
    /// Held until something removes it (a toggle, a stance).
    pub held: bool,
    pub moniker_ids: Vec<i64>,
    /// `EEffectFlag` bits.
    pub effect_flags: u32,
    /// `state_field` bits it holds one counted reference on.
    pub state_flags: u32,
    pub absorb: Vec<AbsorbPoolState>,
    /// Whether the client was sent its icon.
    pub timer_sent: bool,
}

/// The counted references on one `state_field` bit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateFlagRefcount {
    /// Bit index (0 = `BSF_Unused`, 6 = `BSF_MovementLock`, ...).
    pub bit: u32,
    /// The single-bit mask, `1 << bit`.
    pub mask: u32,
    pub count: u32,
    /// Whether the bit is set on `state_field` right now. A count with the
    /// bit clear (or the reverse) is the leak signature AB-09 fixed.
    pub set: bool,
}

/// One stat, current and bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatState {
    pub stat_id: i32,
    pub cur: i32,
    pub min: i32,
    pub max: i32,
}

impl AbilityStateSnapshot {
    /// The stat `stat_id`, if the entity carries it.
    pub fn stat(&self, stat_id: i32) -> Option<&StatState> {
        self.stats.iter().find(|s| s.stat_id == stat_id)
    }

    /// Whether nothing ability-related is in flight: no warmup, cooldown,
    /// pulse, ledger entry or owed clear.
    pub fn is_quiet(&self) -> bool {
        self.pending_cast.is_none()
            && self.cooldowns.is_empty()
            && self.moniker_cooldowns.is_empty()
            && self.pulsing.is_empty()
            && self.ledger.is_empty()
            && self.pending_timer_clears.is_empty()
    }
}

/// Seconds from `now` to `at`, 0 once passed, rounded to the millisecond.
fn secs_until(at: Instant, now: Instant) -> f32 {
    ms(at.saturating_duration_since(now).as_secs_f32())
}

/// `secs` rounded to the millisecond.
fn ms(secs: f32) -> f32 {
    (secs * 1000.0).round() / 1000.0
}

fn ledger_entry(e: &TimedEffect, now: Instant) -> LedgerEntryState {
    LedgerEntryState {
        effect_id: e.effect_id,
        ability_id: e.ability_id,
        invoker_id: e.invoker_id,
        cast_id: e.cast_id,
        stats: e
            .stats
            .iter()
            .map(|s| LedgerStatDelta {
                stat_id: s.stat_id,
                requested: s.requested,
                cur_shift: s.shift.cur,
                min_shift: s.shift.min,
                max_shift: s.shift.max,
            })
            .collect(),
        duration_secs: ms(e.duration_secs),
        expires_in_secs: e.expires_at.map(|t| secs_until(t, now)),
        held: e.expires_at.is_none(),
        moniker_ids: e.moniker_ids.clone(),
        effect_flags: e.effect_flags,
        state_flags: e.state_flags,
        absorb: e
            .absorb
            .iter()
            .map(|p| AbsorbPoolState {
                stat_id: p.stat_id,
                granted: p.granted,
                remaining: p.remaining,
            })
            .collect(),
        timer_sent: e.timer_sent,
    }
}

impl CellEntity {
    /// Snapshot this entity's ability state as of `now`. Read-only; every
    /// list is sorted so two snapshots of the same state compare equal.
    pub fn ability_state(&self, now: Instant) -> AbilityStateSnapshot {
        let pending_cast = self.pending_cast.as_ref().map(|pc| PendingCastState {
            ability_id: pc.ability_id,
            cast_id: pc.cast_id(),
            target_id: pc.target_id,
            wire_target_id: pc.wire_target_id,
            warmup_secs: ms(pc.warmup_secs),
            warmup_remaining_secs: secs_until(pc.fire_at, now),
        });

        let mut cooldowns: Vec<CooldownState> = self
            .abilities
            .ability_cooldowns()
            .filter(|(_, c)| c.expires_at > now)
            .map(|(ability_id, c)| CooldownState {
                ability_id,
                remaining_secs: secs_until(c.expires_at, now),
                total_secs: ms(c.total_duration.as_secs_f32()),
            })
            .collect();
        cooldowns.sort_by_key(|c| c.ability_id);

        let mut moniker_cooldowns: Vec<MonikerCooldownState> = self
            .abilities
            .moniker_cooldowns()
            .filter(|(_, c)| c.expires_at > now)
            .map(|(moniker_id, c)| MonikerCooldownState {
                moniker_id,
                remaining_secs: secs_until(c.expires_at, now),
                total_secs: ms(c.total_duration.as_secs_f32()),
            })
            .collect();
        moniker_cooldowns.sort_by_key(|c| c.moniker_id);

        let mut pulsing: Vec<PulsingEffectState> = self
            .active_effects
            .iter()
            .map(|i| PulsingEffectState {
                effect_id: i.effect_id,
                ability_id: i.ability_id,
                invoker_id: i.invoker_id,
                cast_id: i.cast_id,
                pulses_left: i.remaining_pulses,
                total_pulses: i.total_pulses,
                pulse_interval_secs: ms(i.pulse_interval_secs),
                next_pulse_in_secs: secs_until(i.next_pulse_at, now),
            })
            .collect();
        pulsing.sort_by_key(|p| (p.effect_id, p.invoker_id));

        let mut ledger: Vec<LedgerEntryState> = self
            .stat_buffs
            .entries
            .iter()
            .map(|e| ledger_entry(e, now))
            .collect();
        ledger.sort_by_key(|e| (e.effect_id, e.invoker_id));

        let mut state_flag_refcounts: Vec<StateFlagRefcount> = self
            .state_flag_counts
            .iter()
            .filter(|&(&mask, &count)| count > 0 && mask.is_power_of_two())
            .map(|(&mask, &count)| StateFlagRefcount {
                bit: mask.trailing_zeros(),
                mask,
                count,
                set: self.state_field & mask != 0,
            })
            .collect();
        state_flag_refcounts.sort_by_key(|r| r.bit);

        let mut stats: Vec<StatState> = self
            .stats
            .iter()
            .map(|(&stat_id, s)| StatState {
                stat_id,
                cur: s.cur,
                min: s.min,
                max: s.max,
            })
            .collect();
        stats.sort_by_key(|s| s.stat_id);

        AbilityStateSnapshot {
            entity_id: self.entity_id.0 as u32,
            is_player: self.is_player,
            state_field: self.state_field,
            pending_cast,
            cooldowns,
            moniker_cooldowns,
            pulsing,
            ledger,
            pending_timer_clears: self.stat_buffs.pending_timer_clears.clone(),
            state_flag_refcounts,
            stats,
        }
    }
}
