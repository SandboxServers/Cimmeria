//! Combat-entry dispatcher (Class Start v6, CS-03).
//!
//! `fire_player_entered_combat` fires `OnPlayerEnteredCombat` for a player
//! whose `BSF_InCombat` flag just went on from an NPC's threat. The threat
//! seam (`cell::combat::enter_player_combat`) is synchronous and sits below
//! the content engine, so it queues `(player, mob)` on
//! `SpaceManager::pending_combat_entries`; [`fire_pending_combat_entries`]
//! drains that queue on the 100ms cell tick.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::executor;
use super::super::mission_context::{populate_mission_context, populate_world_context};

/// Drain `SpaceManager::pending_combat_entries`, firing
/// [`fire_player_entered_combat`] for each entry in queue order. The whole
/// queue is taken first, so an entry a chain queues while it runs waits
/// for the next tick instead of looping here.
pub async fn fire_pending_combat_entries(
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if space_mgr.pending_combat_entries.is_empty() {
        return;
    }
    let entries = std::mem::take(&mut space_mgr.pending_combat_entries);
    for (player_entity_id, mob_entity_id) in entries {
        fire_player_entered_combat(player_entity_id, mob_entity_id, engine, tx, space_mgr).await;
    }
}

/// Fire `OnPlayerEnteredCombat` for `player_entity_id`, who entered combat
/// with `mob_entity_id`. The player is the acting entity, with the standard
/// player params: mission context, `archetype`, the world, the shown
/// tutorials, and `mob_id`.
///
/// Returns at once when no `PlayerEnteredCombat` chain is registered. A
/// no-op when the entity is gone (left the space before the tick) or is not
/// a loaded player.
pub async fn fire_player_entered_combat(
    player_entity_id: u32,
    mob_entity_id: u32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    if engine.chains_for_trigger(&TriggerType::PlayerEnteredCombat) == 0 {
        return;
    }
    let mut ctx =
        ExecutionContext::new().with_source(cimmeria_common::EntityId(player_entity_id as i32));
    ctx.set_param("mob_id".to_string(), serde_json::json!(mob_entity_id));
    populate_world_context(player_entity_id, space_mgr, &mut ctx);

    let db_player_id = match space_mgr.get_entity(player_entity_id) {
        Some(entity) if entity.is_player => {
            let Some(db_player_id) = entity.player_id else {
                return;
            };
            populate_mission_context(entity, &mut ctx);
            if let Some(archetype_id) = entity.archetype_id {
                ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
            }
            db_player_id
        }
        _ => return,
    };

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerEnteredCombat,
        source_entity: Some(cimmeria_common::EntityId(player_entity_id as i32)),
        target_entity: Some(cimmeria_common::EntityId(mob_entity_id as i32)),
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        let who = space_mgr.player_identity(player_entity_id);
        let mob = space_mgr.entity_names(mob_entity_id);
        tracing::info!(
            target: "content",
            event = "player_entered_combat",
            entity_id = player_entity_id,
            entity_name = who.player_name,
            account_id = who.account_id,
            account_name = who.account_name,
            player_id = db_player_id,
            player_name = who.player_name,
            mob_id = mob_entity_id,
            mob_name = mob.entity_name,
            template_id = mob.template_id,
            template_name = mob.template_name,
            actions = resolved.actions.len(),
            "fire_player_entered_combat: matched"
        );
    }
    executor::execute_actions(
        resolved,
        player_entity_id,
        db_player_id,
        tx,
        space_mgr,
        engine,
    )
    .await;
}
