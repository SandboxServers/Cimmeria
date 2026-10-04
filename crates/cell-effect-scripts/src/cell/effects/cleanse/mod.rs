//! `RemoveEffects`: cleanses that take effects off by category
//! (ammo campaign AM-11c for the support darts, ability-mechanics AB-10 for
//! the ability cleanses).
//!
//! # Categories
//!
//! An effect declares its category with one `effect_nvps` row named
//! [`EFFECT_CATEGORY_NVP`]. Two sources write it:
//!
//! - the ammo seeds tag their on-hit DoTs `Poison`, `Disease`, `Wound`,
//!   `Burning` (the 2009 cleanses name "1 Effect of Moniker EFFECT_Poison",
//!   and those `EFFECT_*` monikers were never seeded);
//! - the `cleanse` family of `tools/ability_mechanics/effect_nvps_from_desc.py`
//!   tags `Mental`, `Kinetic` and `Health`. Those are the client data's own
//!   classes: `alias.xml` defines `kineticRes`, `mentalRes` and `healthRes`
//!   as "resistance to all harmful kinetic / mental / health effects", and
//!   every resist roll in the effect table names one of the three and shares
//!   an `effect_sequence` step with the effects it gates
//!   (`docs/reverse-engineering/findings/combat-formulas-status.md` §4). So a
//!   Suppression gated by a Mental Resist Roll is a Mental effect.
//!
//! An untagged effect is never cleansed.
//!
//! # What a cleanse removes
//!
//! [`REMOVE_CATEGORIES_NVP`] lists the categories, comma-separated, each
//! optionally with a count: `Mental:2,Health:2` is Absolution's "Purges 2
//! Mental and 2 Health effects". For each listed slot the script takes off
//! one effect of that category from the target, never the same one twice:
//! pulsing instances (`active_effects`, in registration order) first, then
//! timed effect ledger entries (in application order). A pulsing instance
//! gets its script's `on_remove`, as the pulse sweep would give it; a ledger
//! entry is reverted by the ledger (`StatBuffRemoval::Cleansed`).
//!
//! **Polarity.** [`REMOVE_POLARITY_NVP`] is `Harmful` (the default, every
//! seeded cleanse) or `Beneficial`. A harmful cleanse removes only effects
//! without `EF_Beneficial_Effect` and only from the caster or an ally; a
//! beneficial one (a buff strip) removes only beneficial effects and only
//! from a hostile. So a cleanse never strips the caster's own buffs, and a
//! purge that somehow lands on a hostile helps nobody: it is refused and
//! logged.
//!
//! The 50% and 33% chances in some 2009 effect names are not rolled: the
//! cleanse is deterministic (RECONSTRUCTION).
//!
//! The script is synchronous, so a removed effect's icon clear is queued on
//! the ledger (`pending_timer_clears`) and goes out with the caller's
//! `flush_stat_buff_timers`, or the stat-buff tick's.
//!
//! Log target `abilities`.

use cimmeria_entity::abilities::{EffectDef, EF_BENEFICIAL_EFFECT};

use super::stat_buff::StatBuffRemoval;
use super::{EffectContext, EffectScript};

mod relation;
#[cfg(test)]
mod seed_live_db_tests;

pub use relation::{relation, Relation};

/// The NVP that names the category an effect belongs to, e.g. `Poison`.
pub const EFFECT_CATEGORY_NVP: &str = "EffectCategory";

/// The NVP that lists the categories a [`RemoveEffects`] removes:
/// `Poison,Disease,Contagion,Wound,Burning`, or with counts `Mental:2`.
pub const REMOVE_CATEGORIES_NVP: &str = "RemoveCategories";

/// The NVP that says which side's effects a [`RemoveEffects`] removes:
/// `Harmful` (default) or `Beneficial`.
pub const REMOVE_POLARITY_NVP: &str = "RemovePolarity";

/// Which effects a cleanse takes off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// Debuffs, from the caster or an ally.
    Harmful,
    /// Buffs, from a hostile.
    Beneficial,
}

impl Polarity {
    fn label(self) -> &'static str {
        match self {
            Self::Harmful => "harmful",
            Self::Beneficial => "beneficial",
        }
    }
}

/// The cleanse's [`REMOVE_POLARITY_NVP`]; `None` for an unknown value.
pub fn polarity(effect: &EffectDef) -> Option<Polarity> {
    match effect.params.get(REMOVE_POLARITY_NVP).map(|v| v.trim()) {
        None | Some("") => Some(Polarity::Harmful),
        Some(v) if v.eq_ignore_ascii_case("harmful") => Some(Polarity::Harmful),
        Some(v) if v.eq_ignore_ascii_case("beneficial") => Some(Polarity::Beneficial),
        Some(_) => None,
    }
}

/// The category slots `effect` lists in [`REMOVE_CATEGORIES_NVP`], one per
/// effect to remove: `Mental:2,Health` is `["Mental", "Mental", "Health"]`.
/// Entries are trimmed; an empty entry, or one whose count is not a whole
/// number from 1 to 100, is dropped.
pub fn remove_categories(effect: &EffectDef) -> Vec<String> {
    let Some(raw) = effect.params.get(REMOVE_CATEGORIES_NVP) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in raw.split(',') {
        let (name, count) = match entry.split_once(':') {
            Some((name, n)) => (name.trim(), n.trim().parse::<usize>().ok()),
            None => (entry.trim(), Some(1)),
        };
        match count {
            Some(n @ 1..=100) if !name.is_empty() => {
                out.extend(std::iter::repeat_n(name.to_string(), n));
            }
            _ => {}
        }
    }
    out
}

/// The category `effect` carries in [`EFFECT_CATEGORY_NVP`], if any.
pub fn effect_category(effect: &EffectDef) -> Option<&str> {
    effect
        .params
        .get(EFFECT_CATEGORY_NVP)
        .map(|c| c.trim())
        .filter(|c| !c.is_empty())
}

/// Where a picked effect lives on the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Held {
    Pulsing,
    Ledger,
}

impl Held {
    fn label(self) -> &'static str {
        match self {
            Self::Pulsing => "pulsing",
            Self::Ledger => "ledger",
        }
    }
}

/// One effect the cleanse will take off.
#[derive(Debug, Clone)]
struct Pick {
    effect_id: i32,
    invoker_id: u32,
    held: Held,
    category: String,
}

/// Removes, per listed category slot, one matching effect from the target
/// (module docs).
pub struct RemoveEffects;

impl EffectScript for RemoveEffects {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let wanted = remove_categories(ctx.effect);
        let who = ctx.space_mgr.caster_identity(ctx.source_id);
        let target_who = ctx.space_mgr.player_identity(ctx.target_id);
        let polarity = polarity(ctx.effect);
        let rel = relation(ctx.space_mgr, ctx.source_id, ctx.target_id);
        let skip = if wanted.is_empty() {
            Some(("no_categories", "RemoveEffects has no RemoveCategories NVP; nothing removed (check the effect's effect_nvps seed)"))
        } else {
            match (polarity, rel) {
                (None, _) => Some(("unknown_polarity", "RemoveEffects has an unknown RemovePolarity; nothing removed (check the effect's effect_nvps seed)")),
                (Some(Polarity::Harmful), Relation::Caster | Relation::Ally) => None,
                (Some(Polarity::Beneficial), Relation::Hostile) => None,
                (Some(Polarity::Harmful), _) => Some(("target_not_ally", "a debuff cleanse landed on a target that is not the caster or an ally; nothing removed")),
                (Some(Polarity::Beneficial), _) => Some(("target_not_hostile", "a buff strip landed on the caster, an ally or a neutral; nothing removed")),
            }
        };
        if let Some((reason, msg)) = skip {
            tracing::warn!(
                target: "abilities",
                event = "remove_effects_skipped",
                cast_id = ctx.row_ids().cast_id, // nt:id-only per-cast sequence number, no name exists
                reason,
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                entity_id = ctx.source_id,
                entity_name = ctx.space_mgr.caster_label(ctx.source_id),
                target_id = ctx.target_id,
                target_name = ctx.space_mgr.entity_label(ctx.target_id),
                target_player_id = target_who.player_id,
                target_player_name = target_who.player_name,
                source_id = ctx.source_id,
                source_name = ctx.space_mgr.caster_label(ctx.source_id),
                effect_id = ctx.effect.effect_id,
                effect_name = cimmeria_names::book().effect(ctx.effect.effect_id),
                ability_id = ctx.effect.ability_id,
                ability_name = cimmeria_names::book().ability(ctx.effect.ability_id),
                relation = rel.label(),
                "{msg}"
            );
            return;
        }
        let polarity = polarity.unwrap_or(Polarity::Harmful);
        let picks = pick(ctx, &wanted, polarity);

        // Pulsing instances: off the list, then their scripts' `on_remove`.
        if let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) {
            target.active_effects.retain(|inst| {
                !picks.iter().any(|p| {
                    p.held == Held::Pulsing
                        && p.effect_id == inst.effect_id
                        && p.invoker_id == inst.invoker_id
                })
            });
        }
        for p in &picks {
            match p.held {
                Held::Pulsing => {
                    if let Some(def) = ctx.space_mgr.effect_defs.get(&p.effect_id).cloned() {
                        if let Some(script) = def.script_name.clone() {
                            let mut remove_ctx = EffectContext {
                                source_id: p.invoker_id,
                                target_id: ctx.target_id,
                                effect: &def,
                                space_mgr: ctx.space_mgr,
                            };
                            super::dispatch_on_remove(&script, &mut remove_ctx);
                        }
                    }
                    queue_icon_clear(ctx, p);
                }
                Held::Ledger => {
                    let key = (p.effect_id, p.invoker_id);
                    let _ = ctx.space_mgr.remove_timed_effects(
                        ctx.target_id,
                        StatBuffRemoval::Cleansed,
                        |b| b.key() == key,
                    );
                }
            }
            tracing::info!(
                target: "abilities",
                event = "effect_removed_by_cleanse",
                cast_id = ctx.row_ids().cast_id, // nt:id-only per-cast sequence number, no name exists
                account_id = who.account_id,
                account_name = who.account_name,
                player_id = who.player_id,
                player_name = who.player_name,
                entity_id = ctx.source_id,
                entity_name = ctx.space_mgr.caster_label(ctx.source_id),
                target_id = ctx.target_id,
                target_name = ctx.space_mgr.entity_label(ctx.target_id),
                target_player_id = target_who.player_id,
                target_player_name = target_who.player_name,
                source_id = ctx.source_id,
                source_name = ctx.space_mgr.caster_label(ctx.source_id),
                cleanse_effect_id = ctx.effect.effect_id,
                cleanse_effect_name = cimmeria_names::book().effect(ctx.effect.effect_id),
                ability_id = ctx.effect.ability_id,
                ability_name = cimmeria_names::book().ability(ctx.effect.ability_id),
                removed_effect_id = p.effect_id,
                removed_effect_name = cimmeria_names::book().effect(p.effect_id),
                removed_invoker_id = p.invoker_id,
                removed_invoker_name = ctx.space_mgr.entity_label(p.invoker_id),
                held = p.held.label(),
                category = p.category.as_str(),
                polarity = polarity.label(),
                "cleanse removed an effect"
            );
        }
        tracing::debug!(
            target: "abilities",
            event = "remove_effects_applied",
            cast_id = ctx.row_ids().cast_id, // nt:id-only per-cast sequence number, no name exists
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = who.player_id,
            player_name = who.player_name,
            entity_id = ctx.source_id,
            entity_name = ctx.space_mgr.caster_label(ctx.source_id),
            target_id = ctx.target_id,
            target_name = ctx.space_mgr.entity_label(ctx.target_id),
            target_player_id = target_who.player_id,
            target_player_name = target_who.player_name,
            source_id = ctx.source_id,
            source_name = ctx.space_mgr.caster_label(ctx.source_id),
            effect_id = ctx.effect.effect_id,
            effect_name = cimmeria_names::book().effect(ctx.effect.effect_id),
            ability_id = ctx.effect.ability_id,
            ability_name = cimmeria_names::book().ability(ctx.effect.ability_id),
            relation = rel.label(),
            polarity = polarity.label(),
            wanted = wanted.len(),
            removed = picks.len(),
            "RemoveEffects applied"
        );
    }
}

/// The effects to take off: per slot, the first matching one not already
/// picked, pulsing instances before ledger entries.
fn pick(ctx: &EffectContext, wanted: &[String], polarity: Polarity) -> Vec<Pick> {
    let Some(target) = ctx.space_mgr.get_entity(ctx.target_id) else {
        return Vec::new();
    };
    let defs = &ctx.space_mgr.effect_defs;
    let side_ok = |flags: u32| {
        let beneficial = flags & EF_BENEFICIAL_EFFECT != 0;
        beneficial == (polarity == Polarity::Beneficial)
    };
    let candidates: Vec<(i32, u32, Held, u32)> = target
        .active_effects
        .iter()
        .map(|i| {
            let flags = defs.get(&i.effect_id).map_or(0, |d| d.flags);
            (i.effect_id, i.invoker_id, Held::Pulsing, flags)
        })
        .chain(
            target
                .stat_buffs
                .entries
                .iter()
                .map(|b| (b.effect_id, b.invoker_id, Held::Ledger, b.effect_flags)),
        )
        .collect();
    let mut picks: Vec<Pick> = Vec::new();
    for category in wanted {
        let hit = candidates
            .iter()
            .find(|&&(effect_id, invoker_id, held, flags)| {
                side_ok(flags)
                    && !picks.iter().any(|p| {
                        p.effect_id == effect_id && p.invoker_id == invoker_id && p.held == held
                    })
                    && defs
                        .get(&effect_id)
                        .and_then(effect_category)
                        .is_some_and(|c| c.eq_ignore_ascii_case(category))
            });
        if let Some(&(effect_id, invoker_id, held, _)) = hit {
            picks.push(Pick {
                effect_id,
                invoker_id,
                held,
                category: category.clone(),
            });
        }
    }
    picks
}

/// A pulsing instance taken off by a cleanse owes the client the zero
/// `onTimerUpdate` the pulse sweep would send, unless another instance or
/// ledger entry of the same effect still holds the icon (one icon per
/// effect id). The ledger's queue carries it to the next flush.
fn queue_icon_clear(ctx: &mut EffectContext, p: &Pick) {
    let Some(target) = ctx.space_mgr.get_entity_mut(ctx.target_id) else {
        return;
    };
    let still_held = target
        .active_effects
        .iter()
        .any(|i| i.effect_id == p.effect_id)
        || target.stat_buffs.has_effect(p.effect_id);
    let queued = target
        .stat_buffs
        .pending_timer_clears
        .iter()
        .any(|&(effect_id, _)| effect_id == p.effect_id);
    if !still_held && !queued {
        target
            .stat_buffs
            .pending_timer_clears
            .push((p.effect_id, p.invoker_id));
    }
}

#[cfg(test)]
mod tests;
