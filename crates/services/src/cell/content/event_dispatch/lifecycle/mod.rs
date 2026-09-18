//! Player-lifecycle event dispatchers: world entry, entity death, and
//! health-threshold crossings.
//!
//! All three fire on whole-entity transitions rather than on a specific
//! interaction with a target — `PlayerLoaded` runs once per world enter
//! (after the BaseApp has restored the player's persisted state),
//! `EntityDeath` runs when an NPC's `health.cur` crosses to zero, and
//! `EntityHealthBelow` runs when a hit takes a tagged NPC downward past
//! a health percentage *without* killing it. Sibling to the more
//! granular interaction/region/dialog/inventory dispatchers.
//!
//! `EntityDeath` and `EntityHealthBelow` are mutually exclusive by
//! construction: the damage-path caller picks exactly one per hit, so a
//! chain author can rely on a killing blow never also firing a
//! threshold chain.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::executor;
use super::super::mission_context::{populate_mission_context, populate_world_context};

/// Fire the `PlayerLoaded` event for a player entering a world.
pub async fn fire_player_loaded(
    entity_id: u32,
    player_id: i32,
    world_name: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));

    // Order matters. `populate_world_context` sets `world_id` *and* a
    // space-derived `world_name`, but this site is the one that gets the
    // world from its caller — and at player-load time the entity may not
    // be in a space yet, where the space-derived name degrades to
    // "Unknown". `OnPlayerLoaded`'s optional `world_name` filter matches
    // on that param, so the caller's value has to win. Keep the explicit
    // `set_param` below this call.
    populate_world_context(entity_id, space_mgr, &mut ctx);
    ctx.set_param("world_name".to_string(), serde_json::json!(world_name));

    // Same window, same reason, for the numeric form: with no space there
    // is no entity → world resolution, but the caller just told us the
    // world by name, and that name resolves through the same stamped
    // table. Without this, a `world`-gated `player_loaded` chain — one of
    // the two shapes the condition was added for — would fail closed on
    // every arrival that fires before the entity is in its space.
    if ctx.world_id.is_none() {
        ctx.world_id = space_mgr.world_id_for_world(world_name);
    }

    // Populate mission/step/archetype context from entity state
    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(
            entity_id, player_id, %world_name,
            actions = resolved.actions.len(),
            "fire_player_loaded: matched"
        );
    } else {
        tracing::debug!(entity_id, %world_name, "fire_player_loaded: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}

/// Fire the `EntityDeath` event when an NPC is killed.
///
/// This triggers content chains that track kill counts for mission progression
/// (e.g., chains 1085-1086: kill Hallway01_Guard → increment counter → complete mission 681).
pub async fn fire_entity_death(
    killer_entity_id: u32,
    player_id: i32,
    entity_tag: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx =
        ExecutionContext::new().with_source(cimmeria_common::EntityId(killer_entity_id as i32));
    ctx.set_param("entity_tag".to_string(), serde_json::json!(entity_tag));

    populate_world_context(killer_entity_id, space_mgr, &mut ctx);
    if let Some(entity) = space_mgr.get_entity(killer_entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: Some(cimmeria_common::EntityId(killer_entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(
            killer = killer_entity_id, player_id, %entity_tag,
            actions = resolved.actions.len(), "fire_entity_death: matched"
        );
    }
    executor::execute_actions(resolved, killer_entity_id, player_id, tx, space_mgr, engine).await;
}

/// Damage-path entry point: fire `EntityHealthBelow` for a hit that
/// wounded `target_entity_id` without killing it.
///
/// `pct_before` is the target's health percentage sampled *before* the
/// ability resolved (see [`crate::cell::combat::health_pct`]); this
/// function samples the post-hit value itself. Everything the trigger
/// needs beyond that — the target's content tag, the attacker's
/// `player_id` — is resolved here so the combat caller stays a two-line
/// hook.
///
/// Silently does nothing (each case is a normal, high-frequency
/// occurrence, not an error) when:
///
/// - no chain anywhere is registered for this trigger — the common case
///   on every hit in the game, checked first so an unseeded server pays
///   one hash lookup per hit and nothing more;
/// - the target has no content tag (untagged mobs can't be addressed by
///   a chain);
/// - the target's health did not actually go down (a miss, a fully
///   absorbed hit, or a script that healed the target back past where it
///   started — no downward crossing happened);
/// - the target is dead after the hit. This is what *enforces* the "a
///   killing blow fires `entity_dead_tag` and not this" contract. The
///   trigger predicate itself is a pure band test, so `31% → 0%` would
///   satisfy a `:30` chain; the suppression lives here, at the one place
///   that knows the hit was lethal. Deadness is read from `BSF_DEAD`
///   rather than from `health.cur <= 0` **on purpose**: an effect script
///   runs after the NVP damage path and outside its `target_died` guard
///   (`abilities/damage_apply`), so a heal script on a killing blow can
///   leave a corpse at positive health. A health-based check would then
///   fire a threshold chain on that corpse. The zero-health check is
///   kept alongside it for an entity that is at zero but hasn't been
///   marked yet;
/// - the pre- or post-hit percentage is undefined (no HEALTH stat, or a
///   non-positive maximum — the percentage would be a NaN that compares
///   false against every threshold, silently disabling the trigger
///   instead of skipping it).
///
/// The one case that logs is a caller whose attacker has no `player_id`:
/// this function is only reachable from player-driven paths, so that
/// combination means a caller wired it up wrong. Per the negative-logging
/// convention it warns rather than failing silently.
pub async fn fire_health_below_for_hit(
    attacker_entity_id: u32,
    target_entity_id: u32,
    pct_before: Option<crate::cell::combat::HealthPct>,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if engine.chains_for_trigger(&TriggerType::EntityHealthBelow) == 0 {
        return;
    }
    let Some(pct_before) = pct_before else {
        return;
    };

    let Some(target) = space_mgr.get_entity(target_entity_id) else {
        return;
    };
    // Dead targets belong to `entity_dead_tag`, whatever their health
    // reads. See the doc comment for the post-death heal-script case
    // this guards.
    if crate::cell::combat::is_dead_state(target.state_field) {
        return;
    }
    let Some(entity_tag) = target.tag.clone() else {
        return;
    };
    let Some(pct_after) = crate::cell::combat::health_pct(target) else {
        return;
    };
    // Strictly-downward only: a miss, a fully absorbed hit, or a script
    // that healed the target past where it started is not a crossing.
    //
    // This cannot change an outcome on its own — the band predicate is
    // unsatisfiable for a non-downward move, since `after <= pct` and
    // `pct < before` together force `after < before`. It is a cheap
    // early-out that stops the event being built at all, and it keeps
    // the "downward" part of the contract stated in one place rather
    // than left implicit in the matcher's arithmetic.
    if pct_after >= pct_before {
        return;
    }
    // Lethal hits belong to `entity_dead_tag`. See the doc comment: the
    // band predicate alone would happily match `31% → 0%`.
    if pct_after.0 <= 0.0 {
        return;
    }

    let Some(player_id) = space_mgr
        .get_entity(attacker_entity_id)
        .and_then(|e| e.player_id)
    else {
        tracing::warn!(
            attacker = attacker_entity_id,
            target = target_entity_id,
            %entity_tag,
            "fire_health_below_for_hit: attacker has no player_id — \
             skipping EntityHealthBelow event"
        );
        return;
    };

    fire_entity_health_below(
        attacker_entity_id,
        player_id,
        &entity_tag,
        pct_before.0,
        pct_after.0,
        engine,
        tx,
        space_mgr,
    )
    .await;
}

/// Fire the `EntityHealthBelow` event for a hit that wounded a tagged
/// entity without killing it.
///
/// `pct_before` / `pct_after` are the damaged entity's health as a
/// percentage of its maximum, sampled immediately either side of the
/// hit. The trigger — not this function — decides which thresholds were
/// crossed (`pct_before > pct && pct_after <= pct`), so the dispatch
/// site stays independent of which thresholds content has seeded and
/// costs exactly one event per damaging hit no matter how large the hit
/// was. See `cell::combat::health_threshold` for the sampling.
///
/// The acting player is the **attacker**, matching [`fire_entity_death`]:
/// mission and step context comes from the attacker's entity, because
/// that is whose mission the chain is advancing.
///
/// Callers must have already established that the target did *not* die
/// on this hit; a killing blow goes to [`fire_entity_death`] instead.
pub async fn fire_entity_health_below(
    attacker_entity_id: u32,
    player_id: i32,
    entity_tag: &str,
    pct_before: f64,
    pct_after: f64,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx =
        ExecutionContext::new().with_source(cimmeria_common::EntityId(attacker_entity_id as i32));
    ctx.set_param("entity_tag".to_string(), serde_json::json!(entity_tag));
    ctx.set_param("pct_before".to_string(), serde_json::json!(pct_before));
    ctx.set_param("pct_after".to_string(), serde_json::json!(pct_after));

    // H07 contract: every `fire_*` populates the world context, or
    // `world`-gated chains fail closed on this path.
    populate_world_context(attacker_entity_id, space_mgr, &mut ctx);
    if let Some(entity) = space_mgr.get_entity(attacker_entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::EntityHealthBelow,
        source_entity: Some(cimmeria_common::EntityId(attacker_entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(
            attacker = attacker_entity_id, player_id, %entity_tag,
            pct_before, pct_after,
            actions = resolved.actions.len(),
            "fire_entity_health_below: matched"
        );
    }
    executor::execute_actions(
        resolved,
        attacker_entity_id,
        player_id,
        tx,
        space_mgr,
        engine,
    )
    .await;
}

#[cfg(test)]
mod tests;
