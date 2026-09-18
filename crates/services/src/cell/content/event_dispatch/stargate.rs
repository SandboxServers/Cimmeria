//! Stargate event dispatchers (CA10).
//!
//! `fire_stargate_dialed` fires from `cell::gate_travel::handle_dial_gate`
//! once the destination address has validated and the four-second gate
//! timer is armed; `fire_stargate_crossed` fires from the gate-region
//! crossing, immediately before the `GateTravel` teardown. Both carry the
//! DESTINATION world's name as the match key so a chain can gate on
//! "dialled Harset" without also firing on every other gate in the zone.
//!
//! These exist so mission content can hook the gate without the dial
//! handler knowing anything about missions — mission 708's "dial the gate"
//! and "step through" objectives are the first consumers.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::executor;
use super::super::mission_context::populate_mission_context;

/// Fire `StargateDialed` for a successful dial to `destination_world`.
pub async fn fire_stargate_dialed(
    entity_id: u32,
    player_id: i32,
    destination_world: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    fire(
        TriggerType::StargateDialed,
        "fire_stargate_dialed",
        entity_id,
        player_id,
        destination_world,
        engine,
        tx,
        space_mgr,
    )
    .await;
}

/// Fire `StargateCrossed` as the player steps through the open gate.
pub async fn fire_stargate_crossed(
    entity_id: u32,
    player_id: i32,
    destination_world: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    fire(
        TriggerType::StargateCrossed,
        "fire_stargate_crossed",
        entity_id,
        player_id,
        destination_world,
        engine,
        tx,
        space_mgr,
    )
    .await;
}

/// Shared body — the two dispatchers differ only in trigger type and log
/// label, so the context population (mission state + archetype, which
/// `step_status` / `archetype` chain conditions read) lives once.
async fn fire(
    trigger_type: TriggerType,
    label: &'static str,
    entity_id: u32,
    player_id: i32,
    destination_world: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param(
        "destination_world".to_string(),
        serde_json::json!(destination_world),
    );

    // The ORIGIN world, for chains that want to distinguish "left Castle"
    // from "left Harset" while keying on the same destination.
    let world_name = space_mgr
        .get_entity_world_name(entity_id)
        .unwrap_or_else(|| "Unknown".to_string());
    ctx.set_param("world_name".to_string(), serde_json::json!(&world_name));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(
            entity_id,
            player_id,
            destination_world,
            actions = resolved.actions.len(),
            "{label}: matched"
        );
    } else {
        tracing::debug!(entity_id, destination_world, "{label}: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}
