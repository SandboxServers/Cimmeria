//! [`StatBuff`]: a timed buff to one or more primary attributes (the
//! consumable stimpacks, items 6677-6682 and 6697 / 6717-6733).
//!
//! | Items | Effects | Magnitude, each stat | Duration |
//! |---|---|---|---|
//! | Mark III (6677-6682) | 3949-3954, one stat each | +5 | 3600 s |
//! | Mark V (6697, 6719, 6722, 6725, 6728, 6731) | 3955-3966, two stats | +7 / +3 | 3600 s |
//! | Mark VII (6717, 6720, 6723, 6726, 6729, 6732) | 3967-3978, two stats | +7 / +7 | 3600 s |
//! | Mark X (6718, 6721, 6724, 6727, 6730, 6733) | 3979-3990, two stats | +10 / +10 | 3600 s |
//!
//! Each effect row moves one stat; a two-stat stim's ability owns two
//! effects, so one use applies both. The magnitudes are `effect_nvps` rows
//! named after the stat ([`STAT_BUFF_NVPS`]); the 2009 rows shipped none, so
//! each value is the number in the effect's own `effect_desc` ("+7
//! Coordination"). The duration is the effect's `pulse_duration`.
//!
//! **Why not the pulsing layer.** These rows are `pulse_count = 1`, and
//! `register_active_effect` computes `remaining = pulse_count - 1 = 0` and
//! registers nothing, so such an effect never gets an `active_effects`
//! instance and never an `on_remove`. Raising `pulse_count` would make the
//! pulse tick call `on_apply` again at expiry. The buff lives in
//! `CellEntity::stat_buffs` instead ([`cimmeria_entity::cell_entity::StatBuffLedger`]),
//! the player-side counterpart of the pet buff ledger (decision 25 of
//! `docs/architecture/abilities-and-effects-system.md`), and the cell's
//! stat-buff tick in `cimmeria-cell-combat` expires it and sends the
//! client's duration timers.
//!
//! **Stacking is keyed by stat** (see the ledger's module docs): a second
//! buff on the same stat replaces the first, whatever its tier; buffs on
//! different stats never interact.
//!
//! Log target `abilities`.

#[cfg(test)]
mod tests;

use std::time::Instant;

use cimmeria_entity::cell_entity::StatBuffSpec;
use cimmeria_entity::stats::{
    COORDINATION, ENGAGEMENT, FORTITUDE, INTELLIGENCE, MORALE, PERCEPTION,
};

use super::{EffectContext, EffectScript};

// The ledger (`SpaceManager::apply_stat_buff` / `remove_stat_buffs` and
// `StatBuffRemoval`) stays in `cimmeria-cell-world`, where the cell's
// stat-buff tick in `cimmeria-cell-combat` calls it too.
pub use cimmeria_cell_world::cell::effects::stat_buff::*;

/// `effect_nvps` names [`StatBuff`] reads, and the stat each moves. The
/// names are the stimpack's own words, so Intellect maps to the stat the
/// server calls `INTELLIGENCE`.
pub const STAT_BUFF_NVPS: [(&str, i32); 6] = [
    ("Coordination", COORDINATION),
    ("Engagement", ENGAGEMENT),
    ("Fortitude", FORTITUDE),
    ("Intellect", INTELLIGENCE),
    ("Morale", MORALE),
    ("Perception", PERCEPTION),
];

/// The `(stat, delta)` pairs an effect's NVPs ask for; zero NVPs are
/// skipped.
pub fn stat_buff_mods(ctx: &EffectContext) -> Vec<(i32, i32)> {
    STAT_BUFF_NVPS
        .iter()
        .filter_map(|&(name, stat)| match ctx.effect.param_i32(name) {
            0 => None,
            delta => Some((stat, delta)),
        })
        .collect()
}

/// A timed buff to the target's primary attributes, from the effect's
/// [`STAT_BUFF_NVPS`], lasting the effect's `pulse_duration`.
pub struct StatBuff;

impl EffectScript for StatBuff {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let effect_id = ctx.effect.effect_id;
        let ability_id = ctx.effect.ability_id;
        let mods = stat_buff_mods(ctx);
        let reason = if mods.is_empty() {
            Some("no_stat_nvps")
        } else if ctx.effect.pulse_duration <= 0.0 {
            Some("no_duration")
        } else {
            None
        };
        if let Some(reason) = reason {
            let id = ctx.space_mgr.player_identity(ctx.target_id);
            tracing::warn!(
                target: "abilities",
                event = "stat_buff_skipped",
                reason,
                entity_id = ctx.target_id,
                account_id = id.account_id,
                player_id = id.player_id,
                source_id = ctx.source_id,
                effect_id,
                ability_id,
                "StatBuff effect has no stat NVP or no duration; nothing applied \
                 (check the effect's effect_nvps and pulse_duration seed)"
            );
            return;
        }
        let now = Instant::now();
        for (stat_id, delta) in mods {
            let _ = ctx.space_mgr.apply_stat_buff(
                ctx.target_id,
                StatBuffSpec {
                    stat_id,
                    delta,
                    effect_id,
                    ability_id,
                    invoker_id: ctx.source_id,
                    effect_flags: ctx.effect.flags,
                    duration_secs: ctx.effect.pulse_duration,
                },
                now,
            );
        }
    }

    fn on_remove(&self, ctx: &mut EffectContext) {
        let effect_id = ctx.effect.effect_id;
        let _ = ctx
            .space_mgr
            .remove_stat_buffs(ctx.target_id, StatBuffRemoval::Removed, |b| {
                b.effect_id == effect_id
            });
    }
}
