//! The two scripts that write the timed effect ledger
//! ([`cimmeria_entity::cell_entity::StatBuffLedger`], ability-mechanics
//! AB-04 and decision 28 of `docs/architecture/abilities-and-effects-system.md`):
//!
//! - [`StatBuff`]: the consumable stimpacks (items 6677-6682, 6697,
//!   6717-6733), stacked by stat (decision 28's rule);
//! - [`TimedStat`]: ability buffs and debuffs ("+200 Accuracy: 15 Seconds",
//!   "-100 Defense: 15 Seconds"), one entry per `(effect, invoker)`: the same
//!   caster refreshes, another caster stacks (D-AB08).
//!
//! | Items | Effects | Magnitude, each stat | Duration |
//! |---|---|---|---|
//! | Mark III (6677-6682) | 3949-3954, one stat each | +5 | 3600 s |
//! | Mark V (6697, 6719, 6722, 6725, 6728, 6731) | 3955-3966, two stats | +7 / +3 | 3600 s |
//! | Mark VII (6717, 6720, 6723, 6726, 6729, 6732) | 3967-3978, two stats | +7 / +7 | 3600 s |
//! | Mark X (6718, 6721, 6724, 6727, 6730, 6733) | 3979-3990, two stats | +10 / +10 | 3600 s |
//!
//! Both read the effect's NVPs named after a stat ([`STAT_BUFF_NVPS`]); the
//! 2009 rows shipped none, so every value is the number in the effect's own
//! `effect_desc`: hand rows for the stimpacks, the `stat` family of
//! `tools/ability_mechanics/effect_nvps_from_desc.py` for abilities, with
//! D-AB09's unit conversions. The duration is the effect's `pulse_duration`.
//!
//! **Why not the pulsing layer.** These rows are `pulse_count = 1`, and
//! `register_active_effect` computes `remaining = pulse_count - 1 = 0` and
//! registers nothing, so such an effect never gets an `active_effects`
//! instance and never an `on_remove`. Raising `pulse_count` would make the
//! pulse tick call `on_apply` again at expiry. The cell's stat-buff tick in
//! `cimmeria-cell-combat` expires ledger entries and sends the client's
//! duration timers; the callers that run a script (`fire_beneficial`,
//! `damage_apply`, content's `apply_effect`) flush those timers right after.
//!
//! **Held effects** (`pulse_duration = 0`, AB-08) are [`held`]'s: a toggle
//! (`AF_TOGGLED`, a stance) switches on and off with each press, a passive
//! (`EF_AlwaysPersist`) holds while the ability is known, and only ever on
//! its own invoker. Any other held effect would never come off, so it is
//! refused with `no_duration`. [`RemoveByMoniker`] is the "Remove Effect of
//! moniker EFFECT_Stance" half a stance authors.
//!
//! Log target `abilities`.

mod held;
mod remove_by_moniker;
#[cfg(test)]
mod seed_live_db_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod toggle_tests;

pub use remove_by_moniker::RemoveByMoniker;

use std::time::Instant;

use cimmeria_entity::abilities::{effect_moniker_id, EffectDef, EFFECT_MONIKER_NVP};
use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};
use cimmeria_entity::stats::{
    ACCURACY, COORDINATION, COVER_ACCURACY, COVER_DEFENSE, CROUCHING_ACCURACY, CROUCHING_DEFENSE,
    DEFENSE, ENGAGEMENT, FOCUS_REGEN, FORTITUDE, HEALTH_REGEN, HEALTH_RES, INTELLIGENCE,
    INTERRUPT_RES, KINETIC_RES, MENTAL_RES, MITIGATION, MORALE, MOVEMENT_SPEED_MOD, PERCEPTION,
    RESPONSE, SUBTLETY, TRACKING,
};

use super::{EffectContext, EffectScript};

// The ledger (`SpaceManager::apply_timed_effect` / `remove_timed_effects`
// and `StatBuffRemoval`) stays in `cimmeria-cell-world`, where the cell's
// stat-buff tick in `cimmeria-cell-combat` calls it too.
pub use cimmeria_cell_world::cell::effects::stat_buff::*;

/// `effect_nvps` names the ledger scripts read, and the stat each moves.
/// The first six are the stimpacks' own words (Intellect moves the stat the
/// server calls `INTELLIGENCE`); `Mitigation` is the `shield` family's
/// (`families/shield.py`); the rest are the `stat` family's
/// (`tools/ability_mechanics/families/stat.py` writes the same names, and
/// `stat_nvp_names_match_the_generator` pins the two lists together). The
/// regen names are AB-05's: the generator does not write them until the
/// regen model reads them as percentages (D-AB04).
pub const STAT_BUFF_NVPS: &[(&str, i32)] = &[
    ("Coordination", COORDINATION),
    ("Engagement", ENGAGEMENT),
    ("Fortitude", FORTITUDE),
    ("Intellect", INTELLIGENCE),
    ("Morale", MORALE),
    ("Perception", PERCEPTION),
    ("Accuracy", ACCURACY),
    ("Defense", DEFENSE),
    ("CoverAccuracy", COVER_ACCURACY),
    ("CoverDefense", COVER_DEFENSE),
    ("CrouchingAccuracy", CROUCHING_ACCURACY),
    ("CrouchingDefense", CROUCHING_DEFENSE),
    ("Response", RESPONSE),
    ("InterruptResistance", INTERRUPT_RES),
    ("KineticResistance", KINETIC_RES),
    ("MentalResistance", MENTAL_RES),
    ("HealthResistance", HEALTH_RES),
    ("MovementSpeedMod", MOVEMENT_SPEED_MOD),
    ("Tracking", TRACKING),
    ("Subtlety", SUBTLETY),
    ("FocusRegen", FOCUS_REGEN),
    ("HealthRegen", HEALTH_REGEN),
    // The `shield` family's mitigation shields (AB-10): `mitigation` is
    // "armor mitigation percent (0-100%)" in `alias.xml`, so "+15%
    // Mitigation" is 15 points with no conversion.
    ("Mitigation", MITIGATION),
];

/// The `(stat, delta)` pairs an effect's NVPs ask for; zero NVPs are
/// skipped.
pub fn stat_mods(effect: &EffectDef) -> Vec<(i32, i32)> {
    STAT_BUFF_NVPS
        .iter()
        .filter_map(|&(name, stat)| match effect.param_i32(name) {
            0 => None,
            delta => Some((stat, delta)),
        })
        .collect()
}

/// The ledger entry `effect` asks for when `invoker_id` lands it: its stat
/// NVPs, its `pulse_duration`, its flags and its ability's monikers. A
/// caller with its own stats (AB-05's regen buffs, AB-09's CC, AB-10's
/// shields) builds the spec from this and replaces `stats`.
pub fn timed_spec(ctx: &EffectContext, stacking: TimedStacking) -> TimedEffectSpec {
    let effect = ctx.effect;
    TimedEffectSpec {
        cast_id: None, // stamped from the cast scope by SpaceManager::apply_timed_effect
        effect_id: effect.effect_id,
        ability_id: effect.ability_id,
        invoker_id: ctx.source_id,
        effect_flags: effect.flags,
        moniker_ids: entry_monikers(ctx),
        stats: stat_mods(effect),
        absorb: Vec::new(),
        state_flags: 0,
        duration_secs: Some(effect.pulse_duration),
        stacking,
        // `SpaceManager::apply_timed_effect` fills it from the invoker.
        invoker_identity: Default::default(),
    }
}

/// The monikers an entry carries: its ability's `moniker_ids` plus the
/// effect's own [`EFFECT_MONIKER_NVP`] (a stance's `EFFECT_Stance`). An
/// unknown effect-moniker name is dropped with a WARN: it must never match
/// anything by accident.
fn entry_monikers(ctx: &EffectContext) -> Vec<i64> {
    let effect = ctx.effect;
    let mut ids = ctx.space_mgr.ability_moniker_ids(effect.ability_id);
    if let Some(name) = effect.params.get(EFFECT_MONIKER_NVP) {
        match effect_moniker_id(name) {
            Some(id) if !ids.contains(&id) => ids.push(id),
            Some(_) => {}
            None => tracing::warn!(
                target: "abilities",
                event = "effect_moniker_unknown",
                account_id = ctx.space_mgr.caster_identity(ctx.source_id).account_id,
                player_id = ctx.space_mgr.caster_identity(ctx.source_id).player_id,
                entity_id = ctx.source_id,
                target_id = ctx.target_id,
                target_player_id = ctx.space_mgr.player_identity(ctx.target_id).player_id,
                effect_id = effect.effect_id,
                ability_id = effect.ability_id,
                moniker = %name,
                "effect names an effect moniker the server does not know; entry carries only its ability's monikers"
            ),
        }
    }
    ids
}

/// Apply `ctx`'s effect as a ledger entry, or log why not.
fn apply_entry(ctx: &mut EffectContext, stacking: TimedStacking, script: &'static str) {
    let spec = timed_spec(ctx, stacking);
    if !spec.stats.is_empty() && ctx.effect.pulse_duration <= 0.0 && held::is_held_kind(ctx) {
        held::apply_held(ctx, spec, script);
        return;
    }
    let reason = if spec.stats.is_empty() {
        Some("no_stat_nvps")
    } else if ctx.effect.pulse_duration <= 0.0 {
        Some("no_duration")
    } else {
        None
    };
    if let Some(reason) = reason {
        let who = ctx.space_mgr.caster_identity(ctx.source_id);
        let target_who = ctx.space_mgr.player_identity(ctx.target_id);
        tracing::warn!(
            target: "abilities",
            event = "stat_buff_skipped",
            reason,
            script,
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id = ctx.source_id,
            target_id = ctx.target_id,
            target_player_id = target_who.player_id,
            source_id = ctx.source_id,
            effect_id = ctx.effect.effect_id,
            ability_id = ctx.effect.ability_id,
            "{script} effect has no stat NVP or no duration; nothing applied \
             (check the effect's effect_nvps and pulse_duration seed)"
        );
        return;
    }
    let _ = ctx
        .space_mgr
        .apply_timed_effect(ctx.target_id, spec, Instant::now());
}

/// A stimpack: a timed buff to the target's stats from the effect's
/// [`STAT_BUFF_NVPS`], lasting the effect's `pulse_duration`. A second stim
/// on the same stat replaces the first, whatever its tier or source.
pub struct StatBuff;

impl EffectScript for StatBuff {
    fn on_apply(&self, ctx: &mut EffectContext) {
        apply_entry(ctx, TimedStacking::ReplaceSameStat, "StatBuff");
    }

    fn on_remove(&self, ctx: &mut EffectContext) {
        let effect_id = ctx.effect.effect_id;
        let _ = ctx
            .space_mgr
            .remove_timed_effects(ctx.target_id, StatBuffRemoval::Removed, |b| {
                b.effect_id == effect_id
            });
    }
}

/// An ability's timed buff or debuff: the effect's [`STAT_BUFF_NVPS`] for
/// its `pulse_duration`, one entry per `(effect, invoker)`.
pub struct TimedStat;

impl EffectScript for TimedStat {
    fn on_apply(&self, ctx: &mut EffectContext) {
        apply_entry(ctx, TimedStacking::PerSource, "TimedStat");
    }

    fn on_remove(&self, ctx: &mut EffectContext) {
        let key = (ctx.effect.effect_id, ctx.source_id);
        let _ = ctx
            .space_mgr
            .remove_timed_effects(ctx.target_id, StatBuffRemoval::Removed, |b| b.key() == key);
    }
}
