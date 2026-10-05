//! The rows a script logs when it lands nothing (ability-mechanics AB-T2).
//!
//! An effect script's `on_apply` used to `return` without a word when its
//! NVPs were zero, its target was gone or its math came to nothing. Each of
//! those exits now logs one DEBUG row here, under `abilities.effect`, with
//! the script, a stable `reason`, the caster's ids and the cast's `cast_id`
//! (the cast scope is still open while a script runs, AB-T1).

use super::EffectContext;

/// A script that ran and changed nothing. `reason` is one of
/// `no_damage_nvp`, `target_gone`, `no_health_stat`, `no_health_bleed`.
pub(crate) fn script_skipped(ctx: &EffectContext, script: &'static str, reason: &'static str) {
    let who = ctx.space_mgr.caster_identity(ctx.source_id);
    tracing::debug!(
        target: "abilities.effect",
        event = "effect_script_skipped",
        stage = "apply",
        script,
        reason,
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        entity_id = ctx.source_id,
        entity_name = ctx.space_mgr.caster_label(ctx.source_id),
        caster_id = ctx.source_id,
        caster_name = ctx.space_mgr.caster_label(ctx.source_id),
        cast_id = ctx.space_mgr.current_cast_id(), // nt:id-only per-cast sequence number, no name exists
        effect_id = ctx.effect.effect_id,
        effect_name = cimmeria_names::book().effect(ctx.effect.effect_id),
        ability_id = ctx.effect.ability_id,
        ability_name = cimmeria_names::book().ability(ctx.effect.ability_id),
        target_id = ctx.target_id,
        target_name = ctx.space_mgr.entity_label(ctx.target_id),
        target_player_id = ctx.space_mgr.player_identity(ctx.target_id).player_id,
        target_player_name = ctx.space_mgr.player_identity(ctx.target_id).player_name,
        "{script}: expected to land on the target, landed nothing ({reason}); the target's pools are unchanged and the hit shows no damage from this effect"
    );
}

/// A script's `on_remove` took no ledger entry off the target: the entry
/// expired or was cleansed first, or never landed. `script` names whose
/// cleanup it was.
pub(crate) fn on_remove_found_nothing(ctx: &EffectContext, script: &'static str) {
    let who = ctx.space_mgr.caster_identity(ctx.source_id);
    tracing::debug!(
        target: "abilities.ledger",
        event = "effect_on_remove_no_entry",
        stage = "end",
        script,
        reason = "no_ledger_entry",
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        entity_id = ctx.source_id,
        entity_name = ctx.space_mgr.caster_label(ctx.source_id),
        cast_id = ctx.space_mgr.current_cast_id(), // nt:id-only per-cast sequence number, no name exists
        effect_id = ctx.effect.effect_id,
        effect_name = cimmeria_names::book().effect(ctx.effect.effect_id),
        ability_id = ctx.effect.ability_id,
        ability_name = cimmeria_names::book().ability(ctx.effect.ability_id),
        target_id = ctx.target_id,
        target_name = ctx.space_mgr.entity_label(ctx.target_id),
        target_player_id = ctx.space_mgr.player_identity(ctx.target_id).player_id,
        target_player_name = ctx.space_mgr.player_identity(ctx.target_id).player_name,
        "{script} on_remove: expected the caster's ledger entry on the target, found none; nothing comes off (it expired or was cleansed already)"
    );
}
