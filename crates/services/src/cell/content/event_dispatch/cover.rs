//! Cover event dispatchers.
//!
//! `fire_cover_entered` / `fire_cover_left` / `fire_cover_duration` are
//! called from the periodic cover-detection tick (see
//! `crates/services/src/cell/service/ticks/cover.rs`). `fire_npc_flanked`
//! is called from the NPC AI tick when an NPC in cover detects a flank.
//!
//! All four mirror the region-event dispatcher shape: build an
//! `ExecutionContext`, populate event-specific params + mission context,
//! resolve, execute. The content engine matches against the new
//! `OnPlayerEnteredCover` / `OnPlayerLeftCover` / `OnPlayerInCoverDuration`
//! / `OnNpcFlanked` triggers landed in this PR.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::executor;
use super::super::mission_context::populate_mission_context;

/// Fire `OnPlayerEnteredCover` for a player who just entered a cover set.
///
/// Carries `cover_set_id` + representative `height` and `quality` in the
/// trigger params so chain authors can filter on cover quality (e.g.
/// only fire the Castle Cellblock med-bay gate if the player is at a
/// Mid+ cover). `player_id` is the canonical DB player id, used in
/// `populate_mission_context` for mission-state lookups.
pub async fn fire_cover_entered(
    entity_id: u32,
    player_id: i32,
    cover_set_id: i32,
    height: &str,
    quality: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("cover_set_id".to_string(), serde_json::json!(cover_set_id));
    ctx.set_param("height".to_string(), serde_json::json!(height));
    ctx.set_param("quality".to_string(), serde_json::json!(quality));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerEnteredCover,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(
            entity_id,
            player_id,
            cover_set_id,
            height,
            quality,
            actions = resolved.actions.len(),
            "fire_cover_entered: matched"
        );
    } else {
        tracing::debug!(
            entity_id,
            cover_set_id,
            "fire_cover_entered: no chains matched"
        );
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}

/// Paired with [`fire_cover_entered`] — fires when the player leaves the
/// cover set's proximity.
pub async fn fire_cover_left(
    entity_id: u32,
    player_id: i32,
    cover_set_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("cover_set_id".to_string(), serde_json::json!(cover_set_id));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLeftCover,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(
            entity_id,
            player_id,
            cover_set_id,
            actions = resolved.actions.len(),
            "fire_cover_left: matched"
        );
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}

/// Fire `OnPlayerInCoverDuration` when a debounced milestone crosses
/// (e.g. "player has been in cover for 5 seconds"). `seconds` is the
/// milestone value the chain trigger matches against.
pub async fn fire_cover_duration(
    entity_id: u32,
    player_id: i32,
    cover_set_id: i32,
    seconds: u32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("cover_set_id".to_string(), serde_json::json!(cover_set_id));
    ctx.set_param("seconds".to_string(), serde_json::json!(seconds));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerInCoverDuration,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(
            entity_id,
            player_id,
            cover_set_id,
            seconds,
            actions = resolved.actions.len(),
            "fire_cover_duration: matched"
        );
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}

/// Fire `OnNpcFlanked` when an NPC in cover detects a flank (top-threat
/// outside the cover-orient defensive arc). Carries `npc_template` so
/// chain authors can filter on which NPC types react narratively.
pub async fn fire_npc_flanked(
    npc_entity_id: u32,
    threat_entity_id: u32,
    npc_template: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(npc_entity_id as i32));
    ctx.set_param("npc_template".to_string(), serde_json::json!(npc_template));
    ctx.set_param("threat_id".to_string(), serde_json::json!(threat_entity_id));

    let event = TriggerEvent {
        trigger_type: TriggerType::NpcFlanked,
        source_entity: Some(cimmeria_common::EntityId(npc_entity_id as i32)),
        target_entity: Some(cimmeria_common::EntityId(threat_entity_id as i32)),
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(
            npc_entity_id,
            threat_entity_id,
            npc_template,
            actions = resolved.actions.len(),
            "fire_npc_flanked: matched"
        );
    }
    // Player id 0 — flank events don't have a single "player" frame of
    // reference; the executor's mission-context lookups would be a no-op
    // anyway since `npc_entity_id` isn't a player. Pass 0 explicitly so
    // the signature stays consistent with the other fire_* helpers.
    executor::execute_actions(resolved, npc_entity_id, 0, tx, space_mgr, engine).await;
}
