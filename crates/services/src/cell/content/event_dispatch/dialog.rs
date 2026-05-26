//! Dialog event dispatchers: open and choice. Both are fired by the
//! NPC-interaction path when the client is showing or interacting with
//! a dialog window.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::executor;
use super::super::mission_context::populate_mission_context;

/// Fire the `DialogOpen` event when a dialog is displayed to a player.
#[tracing::instrument(
    name = "dialog.event_open",
    level = "info",
    skip_all,
    fields(entity_id, player_id, dialog_id, matched_actions = tracing::field::Empty),
)]
pub async fn fire_dialog_open(
    entity_id: u32,
    player_id: i32,
    dialog_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::DialogOpen,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    tracing::Span::current().record("matched_actions", resolved.actions.len());
    if !resolved.actions.is_empty() {
        tracing::info!(
            entity_id,
            player_id,
            dialog_id,
            actions = resolved.actions.len(),
            "fire_dialog_open: matched"
        );
    } else {
        tracing::debug!(entity_id, dialog_id, "fire_dialog_open: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}

/// Fire `OnDialogChoice` event when a player clicks a dialog button.
#[tracing::instrument(
    name = "dialog.event_choice",
    level = "info",
    skip_all,
    fields(entity_id, player_id, dialog_id, button_id, matched_actions = tracing::field::Empty),
)]
pub async fn fire_dialog_choice(
    entity_id: u32,
    player_id: i32,
    dialog_id: i32,
    button_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));
    ctx.set_param("button_id".to_string(), serde_json::json!(button_id));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::DialogChoice,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    tracing::Span::current().record("matched_actions", resolved.actions.len());
    if !resolved.actions.is_empty() {
        tracing::info!(
            entity_id,
            player_id,
            dialog_id,
            actions = resolved.actions.len(),
            "fire_dialog_choice: matched"
        );
    } else {
        tracing::debug!(
            entity_id,
            dialog_id,
            "fire_dialog_choice: no chains matched"
        );
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}
