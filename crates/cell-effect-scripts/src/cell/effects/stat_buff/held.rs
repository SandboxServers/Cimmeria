//! Held ledger entries: a `pulse_duration = 0` effect with no expiry
//! (ability mechanics AB-08, D-AB08).
//!
//! Two kinds, and nothing else, may hold an entry, because something must
//! take it off again:
//!
//! - **Toggles** (`AF_TOGGLED` abilities: the stances). Each press flips the
//!   ability: the first applies its held effects, the next takes them off
//!   with `ToggledOff`. The cooldown is charged at launch on every press, as
//!   for the owner-pet toggles (2824 Holy Warrior).
//! - **Passives** (`EF_AlwaysPersist`). `apply_passives` runs them while the
//!   ability is known (login, purchase) and takes them off on a respec.
//!
//! **Only on the invoker.** A held entry lands only when the target is its
//! own invoker. Every toggle and passive the generator binds is a Self
//! ability, so this only refuses a forged cast at someone else, which would
//! otherwise leave a permanent stat on another entity that no press of the
//! caster's could find again (`held_not_self`).
//!
//! **One switch per ability, never a second "on".** Effects run one at a
//! time in `effect_ids` order (`fire_beneficial`), and each decides alone.
//! They all read the same switch: the entry of the ability's *last* held
//! effect (the anchor), which no earlier effect of the press has touched.
//! On means every held effect removes its own `(effect, invoker)` entry;
//! off means each applies it, and the ledger's `PerSource` rule refreshes
//! rather than stacks. So a repeated or forged press only ever flips the
//! ability, and an entry another hook removed (a cleanse) comes back in step
//! with the rest on the next "on".
//!
//! **Stances replace each other.** A held entry that carries `EFFECT_Stance`
//! takes off every other ability's `EFFECT_Stance` entries on its entity
//! before it goes on (`RemovedByMoniker`). The 2009 data authors that
//! removal as its own effect on 2 of the bound stances (859, 857; see
//! [`super::RemoveByMoniker`]); the others (1642 Stance: Soldier among them)
//! author none, and would otherwise stack with the previous stance.

use std::time::Instant;

use cimmeria_entity::abilities::{AF_TOGGLED, EFFECT_STANCE_MONIKER, EF_ALWAYS_PERSIST};
use cimmeria_entity::cell_entity::TimedEffectSpec;

use super::{stat_mods, EffectContext, StatBuffRemoval};

/// Whether `ctx`'s effect is a kind that may hold an entry: its ability
/// toggles, or the effect persists (a passive).
pub(super) fn is_held_kind(ctx: &EffectContext) -> bool {
    is_toggle(ctx) || ctx.effect.flags & EF_ALWAYS_PERSIST != 0
}

fn is_toggle(ctx: &EffectContext) -> bool {
    ctx.space_mgr
        .ability_defs
        .get(&ctx.effect.ability_id)
        .is_some_and(|d| d.flags & AF_TOGGLED != 0)
}

/// The ability's last held ledger effect: the toggle's switch (module docs).
fn toggle_anchor(ctx: &EffectContext) -> Option<i32> {
    let def = ctx.space_mgr.ability_defs.get(&ctx.effect.ability_id)?;
    def.effect_ids.iter().rev().copied().find(|id| {
        ctx.space_mgr.effect_defs.get(id).is_some_and(|e| {
            e.script_name.as_deref() == Some("TimedStat")
                && e.pulse_duration <= 0.0
                && !stat_mods(e).is_empty()
        })
    })
}

/// Apply, or (a toggle that is on) remove, `ctx`'s held entry.
pub(super) fn apply_held(ctx: &mut EffectContext, spec: TimedEffectSpec, script: &'static str) {
    let effect_id = ctx.effect.effect_id;
    let ability_id = ctx.effect.ability_id;
    let (source, target) = (ctx.source_id, ctx.target_id);
    let who = ctx.space_mgr.player_identity(source);
    let toggle = is_toggle(ctx);
    let kind = if toggle { "toggle" } else { "passive" };
    if source != target {
        let target_who = ctx.space_mgr.player_identity(target);
        tracing::warn!(
            target: "abilities",
            event = "stat_buff_skipped",
            reason = "held_not_self",
            script,
            kind,
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id = source,
            target_id = target,
            target_player_id = target_who.player_id,
            effect_id,
            ability_id,
            "a held {kind} effect lands only on its own invoker; nothing applied"
        );
        return;
    }
    let mut spec = spec;
    spec.duration_secs = None;
    if toggle {
        let anchor = toggle_anchor(ctx);
        let on = anchor.is_some_and(|a| {
            ctx.space_mgr
                .get_entity(target)
                .is_some_and(|e| e.stat_buffs.entries.iter().any(|b| b.key() == (a, source)))
        });
        if anchor == Some(effect_id) {
            // One row per press: the anchor is the last held effect to run.
            tracing::info!(
                target: "abilities",
                event = "toggle_pressed",
                decision_outcome = if on { "off" } else { "on" },
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id = source,
                target_id = target,
                ability_id,
                effect_id,
                "toggled ability switched {}",
                if on { "off" } else { "on" }
            );
        }
        if on {
            let _ = ctx
                .space_mgr
                .remove_timed_effects(target, StatBuffRemoval::ToggledOff, |b| {
                    b.key() == (effect_id, source)
                });
            return;
        }
    }
    if spec.moniker_ids.contains(&EFFECT_STANCE_MONIKER) {
        let _ =
            ctx.space_mgr
                .remove_timed_effects(target, StatBuffRemoval::RemovedByMoniker, |b| {
                    b.ability_id != ability_id && b.has_moniker(EFFECT_STANCE_MONIKER)
                });
    }
    let _ = ctx
        .space_mgr
        .apply_timed_effect(target, spec, Instant::now());
}
