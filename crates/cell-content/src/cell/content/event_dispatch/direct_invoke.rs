//! Opt-in callback dispatch for triggerless minigame chains.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::executor;
use super::super::mission_context::{populate_mission_context, populate_world_context};

/// Resolve an explicitly selected minigame callback against the acting player's current state.
///
/// This is an opt-in dispatch seam; callers must establish result and player
/// identity separately before invoking it. Only a matching, enabled synthetic
/// callback chain can run, and all of its conditions are evaluated.
pub async fn fire_guarded_minigame_chain_by_id(
    chain_id: i64,
    entity_id: u32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let player_id = match space_mgr.get_entity(entity_id) {
        None => None,
        Some(entity) if !entity.is_player => None,
        Some(entity) => entity.player_id.filter(|player_id| *player_id != 0),
    };
    let Some(player_id) = player_id else {
        let reason = match space_mgr.get_entity(entity_id) {
            None => "missing_entity",
            Some(entity) if !entity.is_player => "non_player",
            Some(_) => "missing_or_zero_player_id",
        };
        tracing::warn!(
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            reason,
            "guarded minigame callback refused"
        );
        return;
    };

    let source = cimmeria_common::EntityId(entity_id as i32);
    let mut ctx = ExecutionContext::new().with_source(source);
    populate_world_context(entity_id, space_mgr, &mut ctx);
    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
    }
    let event_name = format!("__direct_invoke_{chain_id}");
    let event = TriggerEvent {
        trigger_type: TriggerType::CustomEvent,
        source_entity: Some(source),
        target_entity: None,
        params: std::collections::HashMap::from([(
            "event_name".to_string(),
            serde_json::json!(event_name),
        )]),
    };
    let resolved = engine.resolve_event_filtered(&event, &ctx, |chain| chain.id == chain_id);
    if resolved.actions.is_empty() {
        // Visible miss: a stale step, wrong world, or failed condition shows in telemetry.
        tracing::info!(
            chain_id,
            chain_name = cimmeria_names::book().chain(chain_id),
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            reason = "guarded_callback_not_matched",
            "guarded minigame callback not matched"
        );
        return;
    }
    tracing::info!(
        chain_id,
        chain_name = cimmeria_names::book().chain(chain_id),
        entity_id,
        entity_name = space_mgr.entity_label(entity_id),
        action_count = resolved.actions.len(),
        "guarded minigame callback matched"
    );
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}
