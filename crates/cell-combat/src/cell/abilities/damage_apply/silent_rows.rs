//! The hit pipeline's quiet exits (ability-mechanics AB-T2).
//!
//! Each place a hit drops an effect, a script or the whole hit without an
//! answer of its own logs one row here, under `abilities.effect` (the
//! per-effect plan and dispatch) or `abilities` (the hit itself), with a
//! stable `event`, the core ids and the cast's `cast_id`. They sit in their
//! own file so `mod.rs` stays under the file cap.

use super::HitIds;
use crate::cell::space_manager::SpaceManager;

/// An effect id the ability (or a special round) names has no row in
/// `effect_defs`. A seed or loader bug, so WARN: the effect silently does
/// nothing on every hit until the data is fixed.
pub(super) fn effect_def_missing(
    ids: HitIds,
    cast_id: Option<i32>,
    effect_id: i32,
    site: &'static str,
) {
    tracing::warn!(
        target: "abilities.effect",
        event = "effect_def_missing",
        stage = "route",
        reason = "effect_def_missing",
        site,
        account_id = ids.actor.account_id,
        account_name = ids.actor.account_name,
        player_id = ids.actor.player_id,
        player_name = ids.actor.player_name,
        entity_id = ids.entity_id,
        entity_name = ids.entity_name,
        caster_id = ids.entity_id,
        caster_name = ids.entity_name,
        cast_id, // nt:id-only per-cast sequence number, no name exists
        ability_id = ids.ability_id,
        ability_name = cimmeria_names::book().ability(ids.ability_id),
        effect_id,
        effect_name = cimmeria_names::book().effect(effect_id),
        target_id = ids.target_eid,
        target_name = ids.target_name,
        target_player_id = ids.target.player_id,
        target_player_name = ids.target.player_name,
        "hit pipeline ({site}): expected an effect definition for this id, the effect table has none; the effect is skipped on this hit and the player sees nothing from it"
    );
}

/// A hit with no `AbilityDef` deals the generic 15-HP swing. The ability id
/// came from a caster the server has no data for (an NPC's chooseAbility
/// pick, a content chain), so WARN: the numbers the player sees are made up.
pub(super) fn unknown_ability_fallback(ids: HitIds, cast_id: Option<i32>, health: i32) {
    tracing::warn!(
        target: "abilities.effect",
        event = "unknown_ability_fallback_damage",
        stage = "route",
        reason = "no_ability_def",
        account_id = ids.actor.account_id,
        account_name = ids.actor.account_name,
        player_id = ids.actor.player_id,
        player_name = ids.actor.player_name,
        entity_id = ids.entity_id,
        entity_name = ids.entity_name,
        caster_id = ids.entity_id,
        caster_name = ids.entity_name,
        cast_id, // nt:id-only per-cast sequence number, no name exists
        ability_id = ids.ability_id,
        ability_name = cimmeria_names::book().ability(ids.ability_id),
        target_id = ids.target_eid,
        target_name = ids.target_name,
        target_player_id = ids.target.player_id,
        target_player_name = ids.target.player_name,
        health_damage = health,
        "hit pipeline: expected an ability definition, there is none; the hit deals a generic fallback HEALTH swing instead of the ability's effects"
    );
}

/// A script dispatch reached an effect with no `script_name`. The plan only
/// queues scripted effects, so this is a plan/dispatch mismatch.
pub(super) fn script_name_missing(ids: HitIds, cast_id: Option<i32>, effect_id: i32) {
    tracing::debug!(
        target: "abilities.effect",
        event = "effect_script_name_missing",
        stage = "apply",
        reason = "no_script_name",
        account_id = ids.actor.account_id,
        account_name = ids.actor.account_name,
        player_id = ids.actor.player_id,
        player_name = ids.actor.player_name,
        entity_id = ids.entity_id,
        entity_name = ids.entity_name,
        cast_id, // nt:id-only per-cast sequence number, no name exists
        ability_id = ids.ability_id,
        ability_name = cimmeria_names::book().ability(ids.ability_id),
        effect_id,
        effect_name = cimmeria_names::book().effect(effect_id),
        target_id = ids.target_eid,
        target_name = ids.target_name,
        "hit pipeline: expected a script name on an effect queued for script dispatch, found none; no script runs for it"
    );
}

/// The hit's attacker or target entity is gone by the time the hit
/// resolves. The ammo still flushes; nothing else happens.
pub(super) fn hit_gone(
    space_mgr: &SpaceManager,
    entity_id: u32,
    target_eid: u32,
    ability_id: i32,
    missing: &'static str,
) {
    let who = space_mgr.player_identity(entity_id);
    let cast_id = space_mgr.current_cast_id();
    tracing::debug!(
        target: "abilities",
        event = "hit_entity_missing",
        stage = "apply",
        reason = missing,
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        cast_id, // nt:id-only per-cast sequence number, no name exists
        ability_id,
        ability_name = cimmeria_names::book().ability(ability_id),
        target_id = target_eid,
        target_name = space_mgr.entity_label(target_eid),
        "hit pipeline: expected the attacker and the target alive in the space, one is gone ({missing}); no roll, no damage, no onEffectResults (the shot's ammo is still spent)"
    );
}

#[cfg(test)]
#[path = "silent_rows_tests.rs"]
mod tests;
