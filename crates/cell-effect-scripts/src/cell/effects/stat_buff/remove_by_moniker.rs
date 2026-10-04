//! `RemoveByMoniker`: "Remove Effect of moniker EFFECT_Stance" (ability
//! mechanics AB-08).
//!
//! A stance clears the previous stance first with an effect of its own
//! (859 Concentration's 4294, 857 Leading the Target's 920; the shipped
//! stacking model is remove-then-apply, combat-formulas-status §6). The
//! effect names the moniker in its [`REMOVE_MONIKER_NVP`] NVP, and only the
//! effect monikers the server knows resolve (`EFFECT_Stance`). It never
//! takes off its own ability's entries, so it can run before or after the
//! stance's own held effects, and pressing a stance to switch it off does
//! not fight the toggle.
//!
//! Ability monikers are not effect monikers: 1470900795 is on most combat
//! abilities, and removing by it would strip Aim with the stance. Only an
//! entry whose effect carries `EFFECT_Stance` (`EffectMoniker`) matches.

use cimmeria_entity::abilities::{effect_moniker_id, REMOVE_MONIKER_NVP};

use super::{EffectContext, EffectScript, StatBuffRemoval};

/// Take off the target's ledger entries that carry the effect's
/// [`REMOVE_MONIKER_NVP`] moniker, except its own ability's.
pub struct RemoveByMoniker;

impl EffectScript for RemoveByMoniker {
    fn on_apply(&self, ctx: &mut EffectContext) {
        let effect = ctx.effect;
        let who = ctx.space_mgr.player_identity(ctx.source_id);
        let target_who = ctx.space_mgr.player_identity(ctx.target_id);
        let name = effect
            .params
            .get(REMOVE_MONIKER_NVP)
            .map(String::as_str)
            .unwrap_or("");
        let Some(moniker) = effect_moniker_id(name) else {
            tracing::warn!(
                target: "abilities",
                event = "stat_buff_skipped",
                reason = "unknown_moniker",
                script = "RemoveByMoniker",
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id = ctx.source_id,
                target_id = ctx.target_id,
                target_player_id = target_who.player_id,
                effect_id = effect.effect_id,
                ability_id = effect.ability_id,
                moniker = %name,
                "RemoveByMoniker effect names no effect moniker the server knows; nothing removed \
                 (check the effect's RemoveMoniker NVP)"
            );
            return;
        };
        // Only the caster's own stance: a forged cast routed at someone else
        // must not strip their entries.
        if ctx.source_id != ctx.target_id {
            tracing::warn!(
                target: "abilities",
                event = "stat_buff_skipped",
                reason = "remove_not_self",
                script = "RemoveByMoniker",
                account_id = who.account_id,
                player_id = who.player_id,
                entity_id = ctx.source_id,
                target_id = ctx.target_id,
                target_player_id = target_who.player_id,
                effect_id = effect.effect_id,
                ability_id = effect.ability_id,
                "RemoveByMoniker removes only the caster's own entries; nothing removed"
            );
            return;
        }
        let own = effect.ability_id;
        let removed = ctx.space_mgr.remove_timed_effects(
            ctx.target_id,
            StatBuffRemoval::RemovedByMoniker,
            |b| b.ability_id != own && b.has_moniker(moniker),
        );
        tracing::debug!(
            target: "abilities",
            event = "removed_by_moniker",
            account_id = who.account_id,
            player_id = who.player_id,
            entity_id = ctx.source_id,
            target_id = ctx.target_id,
            target_player_id = target_who.player_id,
            effect_id = effect.effect_id,
            ability_id = own,
            moniker = %name,
            moniker_id = moniker,
            removed = removed.len(),
            removed_effect_ids = ?removed.iter().map(|b| b.effect_id).collect::<Vec<_>>(),
            "remove-by-moniker effect ran"
        );
    }
}
