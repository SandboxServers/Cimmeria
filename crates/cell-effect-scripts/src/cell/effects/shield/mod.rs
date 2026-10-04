//! `AbsorbShield`: an absorb shield as a timed effect ledger entry
//! (ability-mechanics AB-10, D-AB08).
//!
//! The effect's NVPs (the `shield` family of
//! `tools/ability_mechanics/effect_nvps_from_desc.py` writes them from the
//! designer text, RECONSTRUCTION):
//!
//! - `ShieldAmount` (required): capacity per damage type;
//! - `ShieldType` (optional): the damage types it blocks, comma-separated,
//!   by name (`Physical`, `Energy`, `Hazmat` or `Contamination`, `Psionic`,
//!   `Untyped`) or by the server's `DT_*` number. Absent means untyped.
//!   Personal Shield's "Absorption: 500 Physical / 500 Energy / 500
//!   Contamination" is `ShieldAmount 500`, `ShieldType Physical,Energy,Hazmat`:
//!   three pools of 500.
//!
//! Each type is one pool on the ledger entry, added to the matching
//! `absorb*` stat (`alias.xml`'s "how much physical damage to absorb"). The
//! damage pipeline drains the stat before Focus and Health; the damage seams
//! then settle the ledger (`SpaceManager::settle_absorb_shields`), which
//! takes the shield off when every pool is empty. Its expiry (the effect's
//! `pulse_duration`), death (`EF_ClearOnDeath`), a refresh or a cleanse
//! takes the unspent capacity back off the stat. A row with no duration is
//! held until drained or removed.
//!
//! A shield lands only on its caster or the caster's ally
//! (`cleanse::relation`); on anything else it is refused and logged.
//!
//! Before AB-10 the script added to the stat with no ledger entry, so a
//! timed shield's capacity outlived the shield (audit B-33).
//!
//! Log target `abilities`.

use std::time::Instant;

// The NVP parsing is `cimmeria-entity`'s, so the launch gate reads the same
// pools; re-exported for the script's callers and tests.
pub use cimmeria_entity::abilities::{
    shield_pool_id, shield_pools, shield_types, SHIELD_AMOUNT_NVP, SHIELD_TYPE_NVP,
};
use cimmeria_entity::cell_entity::TimedStacking;

use super::cleanse::{relation, Relation};
use super::stat_buff::{timed_spec, StatBuffRemoval};
use super::{EffectContext, EffectScript};

/// `shield_skipped`'s `reason` when every pool the shield would fill is full.
pub const REASON_ABSORB_FULL: &str = "absorb_full";

/// The absorb shield (module docs).
pub struct AbsorbShield;

impl EffectScript for AbsorbShield {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let amount = ctx.effect.param_i32(SHIELD_AMOUNT_NVP);
        let types = shield_types(ctx.effect);
        let reason = match (&types, amount > 0) {
            (_, false) => Some("no_amount"),
            (None, _) => Some("unknown_shield_type"),
            _ => None,
        };
        let who = ctx.space_mgr.player_identity(ctx.source_id);
        let target_who = ctx.space_mgr.player_identity(ctx.target_id);
        if let Some(reason) = reason {
            tracing::warn!(
                target: "abilities",
                event = "shield_skipped",
                cast_id = ctx.row_ids().cast_id,
                reason,
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id = ctx.source_id,
                target_id = ctx.target_id,
                target_player_id = target_who.player_id,
                effect_id = ctx.effect.effect_id,
                ability_id = ctx.effect.ability_id,
                shield_type = ctx.effect.params.get(SHIELD_TYPE_NVP).map(String::as_str),
                amount,
                "AbsorbShield has no positive ShieldAmount or an unknown ShieldType; \
                 nothing applied (check the effect's effect_nvps seed)"
            );
            return;
        }
        // A shield only ever helps: never put one on a hostile, whoever
        // cast it (an NPC's cast, or a shield beside a damage effect on the
        // hostile path). The cleanse's rule (`cleanse::relation`).
        let rel = relation(ctx.space_mgr, ctx.source_id, ctx.target_id);
        if !matches!(rel, Relation::Caster | Relation::Ally) {
            tracing::warn!(
                target: "abilities",
                event = "shield_skipped",
                cast_id = ctx.row_ids().cast_id,
                reason = "target_not_ally",
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id = ctx.source_id,
                target_id = ctx.target_id,
                target_player_id = target_who.player_id,
                effect_id = ctx.effect.effect_id,
                ability_id = ctx.effect.ability_id,
                relation = rel.label(),
                "AbsorbShield landed on a target that is not the caster or an ally; nothing applied"
            );
            return;
        }
        let types = types.unwrap_or_default();
        let mut spec = timed_spec(ctx, TimedStacking::PerSource);
        spec.absorb = types
            .iter()
            .map(|&dt| (shield_pool_id(dt), amount))
            .collect();
        spec.duration_secs = (ctx.effect.pulse_duration > 0.0).then_some(ctx.effect.pulse_duration);
        // Every pool already full: the entry would be an icon that absorbs
        // nothing, so the ledger refuses it. Say so here, with the reason;
        // a player's launch is refused with feedback before this
        // (`use_ability/shield_full.rs`).
        let room = ctx.space_mgr.get_entity(ctx.target_id).map_or(0, |e| {
            e.absorb_room(&spec.absorb, (ctx.effect.effect_id, ctx.source_id))
        });
        if room == 0 {
            tracing::debug!(
                target: "abilities",
                event = "shield_skipped",
                cast_id = ctx.row_ids().cast_id,
                reason = REASON_ABSORB_FULL,
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id = ctx.source_id,
                target_id = ctx.target_id,
                target_player_id = target_who.player_id,
                effect_id = ctx.effect.effect_id,
                ability_id = ctx.effect.ability_id,
                "AbsorbShield: every pool it would fill is full; nothing applied"
            );
            return;
        }
        let Some(out) = ctx
            .space_mgr
            .apply_timed_effect(ctx.target_id, spec, Instant::now())
        else {
            // The ledger refused the entry (no target, or nothing to hold).
            tracing::debug!(
                target: "abilities",
                event = "shield_skipped",
                stage = "ledger",
                reason = "ledger_refused",
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id = ctx.source_id,
                cast_id = ctx.space_mgr.current_cast_id(),
                target_id = ctx.target_id,
                target_player_id = target_who.player_id,
                effect_id = ctx.effect.effect_id,
                ability_id = ctx.effect.ability_id,
                "AbsorbShield: expected the ledger to take the shield entry, it refused; no shield, no icon, the target takes full damage"
            );
            return;
        };
        tracing::info!(
            target: "abilities",
            event = "shield_granted",
            cast_id = ctx.row_ids().cast_id,
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id = ctx.source_id,
            target_id = ctx.target_id,
            target_player_id = target_who.player_id,
            effect_id = ctx.effect.effect_id,
            ability_id = ctx.effect.ability_id,
            damage_types = ?types,
            amount,
            pools = ?out.applied.absorb.iter().map(|p| (p.stat_id, p.granted)).collect::<Vec<_>>(),
            duration_secs = out.applied.duration_secs,
            held = out.applied.expires_at.is_none(),
            "AbsorbShield applied"
        );
    }

    /// A pulsing instance that carried the script ended, or a cleanse took
    /// it: the ledger entry comes off and its unspent capacity with it.
    fn on_remove(&self, ctx: &mut EffectContext) {
        let key = (ctx.effect.effect_id, ctx.source_id);
        let removed =
            ctx.space_mgr
                .remove_timed_effects(ctx.target_id, StatBuffRemoval::Removed, |b| b.key() == key);
        if removed.is_empty() {
            crate::cell::effects::script_rows::on_remove_found_nothing(ctx, "AbsorbShield");
        }
    }
}

#[cfg(test)]
mod tests;
