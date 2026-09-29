//! Special-ammo effect scripts, packet AM-11c (ammo campaign, issue #1026):
//! the buff and heal darts Stim, Coagulant, Nanites, Antidote and
//! Adrenaline.
//!
//! # How a beneficial dart fits the damage pipeline
//!
//! Each dart is an ordinary `ammo_modifiers` row
//! (`db/resources/Abilities/Seed/ammo_modifiers_dart_support.sql`). The
//! pipeline (`damage_apply`, AM-04) scales the shot by `damage_mult` and runs
//! `on_hit_effect_id` on the target after a hit. A support row sets
//! [`DART_SUPPORT_DAMAGE_MULT`], small enough that the shot deals no damage,
//! and an on-hit effect that helps the target:
//!
//! | Ammo | On-hit effect | Script |
//! |---|---|---|
//! | Stim | 9160, +10% Focus (effect 5008's text) | `HealFocus` |
//! | Antidote | 9161, removes Poison, Disease, Contagion, Wound, Burning | [`RemoveEffects`] |
//! | Coagulant | 9162, removes Wound | [`RemoveEffects`] |
//! | Adrenaline | 9163, +10% Health (RECONSTRUCTION) | `HealHealth` |
//! | Nanites | none: no evidence, so it has no row and fires as a plain dart | |
//!
//! Every on-hit effect here is server-only: it changes stats (flushed by
//! `damage_apply` as `onStatUpdate`) and sends no per-effect message. The
//! 91xx effect ids are not in the client's cooked data, and nobody has
//! checked what the client does with an unknown effect id in an
//! `onTimerUpdate` (an unknown cooked id crashed it before, #938). That is
//! why Adrenaline is a heal and not a `StatBuff`, which would send one.
//!
//! **Targeting (AM-11d).** Every row here is `beneficial = true`, so a
//! shot with one loaded lands on an ally player or the shooter and is
//! refused at a hostile target (`cimmeria-cell-combat`'s
//! `use_ability::support_shot`). It runs only the on-hit effect below, with
//! no damage, threat or combat state. `damage_apply` never runs a
//! beneficial row's on-hit effect, so these scripts only ever see an ally.
//!
//! # Effect categories
//!
//! The 2009 cleanse effects remove "1 Effect of Moniker EFFECT_Poison".
//! Those `EFFECT_*` monikers were never seeded, and an `EffectDef` carries
//! no name. So a category is an `effect_nvps` row named
//! [`EFFECT_CATEGORY_NVP`] on the effect to be removed (`Poison`, `Disease`,
//! `Contagion`, `Wound`, `Burning`), and the cleanse names the categories
//! it removes in [`REMOVE_CATEGORIES_NVP`]. An effect with no category row
//! is never removed.

use cimmeria_entity::abilities::EffectDef;

use super::{EffectContext, EffectScript};

/// `damage_mult` of every support dart row. The table's CHECK requires
/// `damage_mult > 0`, so the multiplier cannot be exactly 0. At this value
/// any shot under 5000 points of pre-armour damage rounds to 0 in
/// `combat::calculate_damage_penetrating`, and every seeded ability deals
/// far less than that.
pub const DART_SUPPORT_DAMAGE_MULT: f32 = 0.0001;

/// The NVP that names the category an effect belongs to, e.g. `Poison`.
pub const EFFECT_CATEGORY_NVP: &str = "EffectCategory";

/// The NVP that lists the categories a [`RemoveEffects`] removes, separated
/// by commas: `Poison,Disease,Contagion,Wound,Burning`.
pub const REMOVE_CATEGORIES_NVP: &str = "RemoveCategories";

/// The categories `effect` lists in [`REMOVE_CATEGORIES_NVP`], trimmed,
/// with empty entries dropped.
pub fn remove_categories(effect: &EffectDef) -> Vec<String> {
    effect
        .params
        .get(REMOVE_CATEGORIES_NVP)
        .map(|v| {
            v.split(',')
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The category `effect` carries in [`EFFECT_CATEGORY_NVP`], if any.
pub fn effect_category(effect: &EffectDef) -> Option<&str> {
    effect
        .params
        .get(EFFECT_CATEGORY_NVP)
        .map(|c| c.trim())
        .filter(|c| !c.is_empty())
}

/// Removes from the target, for each category in the effect's
/// [`REMOVE_CATEGORIES_NVP`], the oldest active effect of that category
/// (the 2009 text: "Remove 1 Effect of ..."). A removed effect gets its
/// script's `on_remove`, exactly as the pulse sweep would give it, so a
/// Stun or a shield cleans up after itself.
///
/// The 50% and 33% chances in some 2009 effect names are not rolled: the
/// cleanse is deterministic (RECONSTRUCTION).
///
/// Known limit: this script runs synchronously and cannot send, so the
/// client is not sent the zero `onTimerUpdate` the pulse sweep sends. The
/// removed effect's icon stays until its own countdown runs out. Stat
/// changes from `on_remove` are flushed by `damage_apply`, which flushes
/// after every on-hit script.
pub struct RemoveEffects;

impl EffectScript for RemoveEffects {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let wanted = remove_categories(ctx.effect);
        let id = ctx.space_mgr.player_identity(ctx.target_id);
        if wanted.is_empty() {
            tracing::warn!(
                target: "abilities",
                event = "remove_effects_skipped",
                reason = "no_categories",
                account_id = id.account_id,
                player_id = id.player_id,
                entity_id = ctx.target_id,
                source_id = ctx.source_id,
                effect_id = ctx.effect.effect_id,
                ability_id = ctx.effect.ability_id,
                "RemoveEffects has no RemoveCategories NVP; nothing removed \
                 (check the effect's effect_nvps seed)"
            );
            return;
        }

        // Pick first, remove second: the pick needs `effect_defs` and the
        // target together, the removal a mutable target.
        let picks: Vec<(i32, u32, String)> = {
            let Some(target) = ctx.space_mgr.get_entity(ctx.target_id) else {
                return;
            };
            let mut picks: Vec<(i32, u32, String)> = Vec::new();
            for category in &wanted {
                let hit = target.active_effects.iter().find(|inst| {
                    !picks
                        .iter()
                        .any(|(e, i, _)| *e == inst.effect_id && *i == inst.invoker_id)
                        && ctx
                            .space_mgr
                            .effect_defs
                            .get(&inst.effect_id)
                            .and_then(effect_category)
                            .is_some_and(|c| c.eq_ignore_ascii_case(category))
                });
                if let Some(inst) = hit {
                    picks.push((inst.effect_id, inst.invoker_id, category.clone()));
                }
            }
            picks
        };

        if let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) {
            target.active_effects.retain(|inst| {
                !picks
                    .iter()
                    .any(|(e, i, _)| *e == inst.effect_id && *i == inst.invoker_id)
            });
        }

        for (effect_id, invoker_id, category) in &picks {
            if let Some(def) = ctx.space_mgr.effect_defs.get(effect_id).cloned() {
                if let Some(script) = def.script_name.clone() {
                    let mut remove_ctx = EffectContext {
                        source_id: *invoker_id,
                        target_id: ctx.target_id,
                        effect: &def,
                        space_mgr: ctx.space_mgr,
                    };
                    super::dispatch_on_remove(&script, &mut remove_ctx);
                }
            }
            tracing::info!(
                target: "abilities",
                event = "effect_removed_by_cleanse",
                account_id = id.account_id,
                player_id = id.player_id,
                entity_id = ctx.target_id,
                source_id = ctx.source_id,
                cleanse_effect_id = ctx.effect.effect_id,
                ability_id = ctx.effect.ability_id,
                removed_effect_id = *effect_id,
                removed_invoker_id = *invoker_id,
                category = category.as_str(),
                "cleanse removed an active effect"
            );
        }
        tracing::debug!(
            target: "abilities",
            event = "remove_effects_applied",
            account_id = id.account_id,
            player_id = id.player_id,
            entity_id = ctx.target_id,
            source_id = ctx.source_id,
            effect_id = ctx.effect.effect_id,
            wanted = wanted.len(),
            removed = picks.len(),
            "RemoveEffects applied"
        );
    }
}

#[cfg(test)]
mod tests;
