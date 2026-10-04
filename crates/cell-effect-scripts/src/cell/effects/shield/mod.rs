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
//! Before AB-10 the script added to the stat with no ledger entry, so a
//! timed shield's capacity outlived the shield (audit B-33).
//!
//! Log target `abilities`.

use std::time::Instant;

use cimmeria_entity::abilities::{
    EffectDef, DT_ENERGY, DT_HAZMAT, DT_PHYSICAL, DT_PSIONIC, DT_UNTYPED,
};
use cimmeria_entity::cell_entity::TimedStacking;
use cimmeria_entity::stats::{
    ABSORB_ENERGY, ABSORB_HAZMAT, ABSORB_PHYSICAL, ABSORB_PSIONIC, ABSORB_UNTYPED,
};

use super::stat_buff::{timed_spec, StatBuffRemoval};
use super::{EffectContext, EffectScript};

/// The capacity NVP.
pub const SHIELD_AMOUNT_NVP: &str = "ShieldAmount";
/// The damage-type list NVP.
pub const SHIELD_TYPE_NVP: &str = "ShieldType";

/// The `absorb*` stat a damage type's pool fills. An unknown type falls
/// back to the untyped pool, as the pipeline's drain does.
pub fn shield_pool_id(damage_type: i8) -> i32 {
    match damage_type {
        DT_PHYSICAL => ABSORB_PHYSICAL,
        DT_ENERGY => ABSORB_ENERGY,
        DT_HAZMAT => ABSORB_HAZMAT,
        DT_PSIONIC => ABSORB_PSIONIC,
        _ => ABSORB_UNTYPED,
    }
}

/// One `ShieldType` token as a server damage type.
fn damage_type_of(token: &str) -> Option<i8> {
    let t = token.trim();
    if let Ok(n) = t.parse::<i8>() {
        return [DT_UNTYPED, DT_ENERGY, DT_HAZMAT, DT_PHYSICAL, DT_PSIONIC]
            .contains(&n)
            .then_some(n);
    }
    match t.to_ascii_lowercase().as_str() {
        "physical" => Some(DT_PHYSICAL),
        "energy" => Some(DT_ENERGY),
        "hazmat" | "contamination" => Some(DT_HAZMAT),
        "psionic" => Some(DT_PSIONIC),
        "untyped" => Some(DT_UNTYPED),
        _ => None,
    }
}

/// The damage types `effect` blocks: its `ShieldType` list, untyped when
/// absent or blank, `None` when a token is not a damage type.
pub fn shield_types(effect: &EffectDef) -> Option<Vec<i8>> {
    let Some(raw) = effect.params.get(SHIELD_TYPE_NVP) else {
        return Some(vec![DT_UNTYPED]);
    };
    let tokens: Vec<&str> = raw.split(',').filter(|t| !t.trim().is_empty()).collect();
    if tokens.is_empty() {
        return Some(vec![DT_UNTYPED]);
    }
    let mut out: Vec<i8> = Vec::new();
    for token in tokens {
        let dt = damage_type_of(token)?;
        if !out.contains(&dt) {
            out.push(dt);
        }
    }
    Some(out)
}

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
        let types = types.unwrap_or_default();
        let mut spec = timed_spec(ctx, TimedStacking::PerSource);
        spec.absorb = types
            .iter()
            .map(|&dt| (shield_pool_id(dt), amount))
            .collect();
        spec.duration_secs = (ctx.effect.pulse_duration > 0.0).then_some(ctx.effect.pulse_duration);
        let Some(out) = ctx
            .space_mgr
            .apply_timed_effect(ctx.target_id, spec, Instant::now())
        else {
            return;
        };
        tracing::info!(
            target: "abilities",
            event = "shield_granted",
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
        let _ = ctx
            .space_mgr
            .remove_timed_effects(ctx.target_id, StatBuffRemoval::Removed, |b| b.key() == key);
    }
}

#[cfg(test)]
mod tests;
