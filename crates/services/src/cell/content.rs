//! Content engine bridge for the CellService.
//!
//! Wires the data-driven chain engine into the game loop. Loads chains from the
//! database at startup (falling back to hardcoded chains if DB is unavailable),
//! fires events from gameplay actions, and executes the resolved actions against
//! the game state.

use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine, ResolvedActions};
use cimmeria_content_engine::conditions::{ComparisonOp, Condition, MissionStatusValue};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::loader::{
    DbActionRow, DbChainRow, DbConditionRow, DbTriggerRow, build_chains_from_rows,
};
use cimmeria_content_engine::triggers::{Trigger, TriggerEvent, TriggerType};

use cimmeria_entity::missions::{MissionObjective, STATUS_ACTIVE};

use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// ── Engine construction ─────────────────────────────────────────────────────

/// Build the content engine by loading chains from the database.
///
/// Falls back to a minimal hardcoded engine if the DB pool is unavailable
/// or the content tables don't exist yet.
pub async fn build_engine(db_pool: Option<&PgPool>) -> ChainEngine {
    if let Some(pool) = db_pool {
        match load_chains_from_db(pool).await {
            Ok(chains) => {
                let mut engine = ChainEngine::new();
                for chain in chains {
                    engine.register_chain(chain);
                }
                tracing::info!(chains = engine.chain_count(), "Content engine loaded from database");
                return engine;
            }
            Err(e) => {
                tracing::warn!("Failed to load content chains from DB: {e} — using hardcoded fallback");
            }
        }
    } else {
        tracing::info!("No DB pool available — using hardcoded content engine");
    }

    build_fallback_engine()
}

/// Build a minimal fallback engine with hardcoded Mission 622 chains.
pub fn build_initial_engine() -> ChainEngine {
    build_fallback_engine()
}

fn build_fallback_engine() -> ChainEngine {
    let mut engine = ChainEngine::new();

    // Chain 1: Auto-accept mission 622 when player loads (any world)
    engine.register_chain(Chain {
        id: 1,
        name: "Mission 622: Auto-accept on load (fallback)".to_string(),
        enabled: true,
        trigger: Trigger::OnPlayerLoaded { world_name: None },
        conditions: vec![
            Condition::MissionStatus {
                mission_id: 622,
                operator: ComparisonOp::Eq,
                expected_status: MissionStatusValue::NotActive,
            },
        ],
        actions: vec![
            Action::AcceptMission { mission_id: 622 },
        ],
        priority: 0,
    });

    // Chain 2: Complete mission 622 when FrostBody dialog opens
    engine.register_chain(Chain {
        id: 2,
        name: "Mission 622: Complete on FrostBody interact (fallback)".to_string(),
        enabled: true,
        trigger: Trigger::OnDialogOpen { dialog_id: 3995 },
        conditions: vec![],
        actions: vec![
            Action::GrantItem { item_id: 55, count: 1, container_id: Some(3) },
            Action::GrantItem { item_id: 3730, count: 1, container_id: Some(0) },
            Action::CompleteMission { mission_id: 622 },
        ],
        priority: 0,
    });

    tracing::info!(chains = engine.chain_count(), "Content engine initialized (fallback)");
    engine
}

// ── DB loading ──────────────────────────────────────────────────────────────

/// Load all enabled content chains from the database.
async fn load_chains_from_db(pool: &PgPool) -> Result<Vec<Chain>, sqlx::Error> {
    use sqlx::Row;

    let chain_rows: Vec<DbChainRow> = sqlx::query(
        "SELECT chain_id, description, scope_type, scope_id, enabled, priority \
         FROM resources.content_chains ORDER BY chain_id"
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbChainRow {
        chain_id: r.get("chain_id"),
        description: r.get("description"),
        scope_type: r.get("scope_type"),
        scope_id: r.get("scope_id"),
        enabled: r.get("enabled"),
        priority: r.get("priority"),
    })
    .collect();

    let trigger_rows: Vec<DbTriggerRow> = sqlx::query(
        "SELECT chain_id, event_type, event_key, scope, once, sort_order \
         FROM resources.content_triggers ORDER BY chain_id, sort_order"
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbTriggerRow {
        chain_id: r.get("chain_id"),
        event_type: r.get("event_type"),
        event_key: r.get("event_key"),
        scope: r.get("scope"),
        once: r.get("once"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    let condition_rows: Vec<DbConditionRow> = sqlx::query(
        "SELECT chain_id, condition_type, target_id, target_key, operator, value, sort_order \
         FROM resources.content_conditions ORDER BY chain_id, sort_order"
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbConditionRow {
        chain_id: r.get("chain_id"),
        condition_type: r.get("condition_type"),
        target_id: r.get("target_id"),
        target_key: r.get("target_key"),
        operator: r.get("operator"),
        value: r.get("value"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    let action_rows: Vec<DbActionRow> = sqlx::query(
        "SELECT chain_id, action_type, target_id, target_key, params, delay_ms, sort_order \
         FROM resources.content_actions ORDER BY chain_id, sort_order"
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| DbActionRow {
        chain_id: r.get("chain_id"),
        action_type: r.get("action_type"),
        target_id: r.get("target_id"),
        target_key: r.get("target_key"),
        params: r.get("params"),
        delay_ms: r.get("delay_ms"),
        sort_order: r.get("sort_order"),
    })
    .collect();

    tracing::info!(
        chains = chain_rows.len(),
        triggers = trigger_rows.len(),
        conditions = condition_rows.len(),
        actions = action_rows.len(),
        "Loaded content engine rows from database"
    );

    Ok(build_chains_from_rows(chain_rows, trigger_rows, condition_rows, action_rows))
}

// ── Event firing ────────────────────────────────────────────────────────────

/// Fire the `PlayerLoaded` event for a player entering a world.
pub async fn fire_player_loaded(
    entity_id: u32,
    player_id: i32,
    world_name: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new()
        .with_source(cimmeria_common::EntityId(entity_id as i32));

    ctx.set_param("world_name".to_string(), serde_json::json!(world_name));

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
    execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;
}

/// Fire the `DialogOpen` event when a dialog is displayed to a player.
pub async fn fire_dialog_open(
    entity_id: u32,
    player_id: i32,
    dialog_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new()
        .with_source(cimmeria_common::EntityId(entity_id as i32));
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
    execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;
}

/// Populate mission status and step status context params from entity state.
fn populate_mission_context(entity: &cimmeria_entity::cell_entity::CellEntity, ctx: &mut ExecutionContext) {
    for mission in entity.missions.all_missions() {
        let status_str = match mission.status {
            0 => "not_active",
            1 => "active",
            2 => "completed",
            _ => "not_active",
        };
        ctx.set_param(
            format!("mission_{}_status", mission.mission_id),
            serde_json::json!(status_str),
        );

        // Also set step statuses for the current step
        if let Some(step_id) = mission.current_step_id {
            ctx.set_param(
                format!("mission_{}_step_{}_status", mission.mission_id, step_id),
                serde_json::json!("active"),
            );
        }
    }
}

// ── Action execution ────────────────────────────────────────────────────────

/// Execute resolved actions from the content engine against the game state.
async fn execute_actions(
    resolved: ResolvedActions,
    entity_id: u32,
    player_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    for (chain_id, action) in resolved.actions {
        match action {
            Action::AcceptMission { mission_id } | Action::AdvanceMission { mission_id } => {
                tracing::info!(entity_id, mission_id, chain_id, "Content: accepting mission");
                let (step_id, objectives) = mission_data(mission_id);
                super::missions::accept_mission(
                    entity_id, mission_id, step_id, objectives, tx, space_mgr,
                ).await;
                let _ = tx.send(CellToBaseMsg::MissionUpdate {
                    player_id,
                    mission_id,
                    status: 1,
                    current_step_id: Some(step_id),
                    completed_step_ids: vec![],
                    completed_objective_ids: vec![],
                    active_objective_ids: vec![step_id],
                }).await;
            }
            Action::CompleteMission { mission_id } => {
                tracing::info!(entity_id, mission_id, chain_id, "Content: completing mission");
                super::missions::complete_mission_direct(
                    entity_id, mission_id, tx, space_mgr,
                ).await;
                let _ = tx.send(CellToBaseMsg::MissionUpdate {
                    player_id,
                    mission_id,
                    status: 2,
                    current_step_id: None,
                    completed_step_ids: vec![],
                    completed_objective_ids: vec![],
                    active_objective_ids: vec![],
                }).await;
            }
            Action::GrantItem { item_id, count, container_id } => {
                tracing::info!(entity_id, item_id, count, chain_id, "Content: granting item");
                let cid = container_id.unwrap_or_else(|| item_container(item_id));
                grant_item_runtime(entity_id, item_id, cid, count, tx).await;
                let _ = tx.send(CellToBaseMsg::GrantItem {
                    entity_id,
                    player_id,
                    item_id,
                    container_id: cid,
                    count,
                }).await;
            }
            Action::DisplayDialog { dialog_id } | Action::StartDialog { dialog_set_id: dialog_id } => {
                tracing::info!(entity_id, dialog_id, chain_id, "Content: displaying dialog");
                super::interactions::send_dialog_display(entity_id, 0, dialog_id, tx).await;
            }
            Action::PlaySequence { sequence_id } => {
                tracing::info!(entity_id, sequence_id, chain_id, "Content: playing sequence");
                // Send onPlaySequence (method_index 22) to client
                let mut args = Vec::with_capacity(4);
                args.extend_from_slice(&sequence_id.to_le_bytes());
                let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: 22, // onPlaySequence
                    args,
                }).await;
            }
            Action::AdvanceStep { mission_id, step_id } => {
                tracing::info!(entity_id, mission_id, step_id, chain_id, "Content: advancing step");
                // TODO: Update mission manager step tracking + send wire update
                let _ = tx.send(CellToBaseMsg::MissionUpdate {
                    player_id,
                    mission_id,
                    status: 1,
                    current_step_id: Some(step_id),
                    completed_step_ids: vec![],
                    completed_objective_ids: vec![],
                    active_objective_ids: vec![step_id],
                }).await;
            }
            Action::AddDialogSet { dialog_set_id, slot, mission_id: _ } => {
                tracing::info!(entity_id, dialog_set_id, slot, chain_id, "Content: adding dialog set");
                // TODO: Send onAddDialogSet to client
            }
            Action::RemoveDialogSet { dialog_set_id, slot } => {
                tracing::info!(entity_id, dialog_set_id, slot, chain_id, "Content: removing dialog set");
                // TODO: Send onRemoveDialogSet to client
            }
            Action::RemoveItem { item_id, count } => {
                tracing::info!(entity_id, item_id, count, chain_id, "Content: removing item");
                // TODO: Send onRemoveItem + DB persist
            }
            Action::SetInteractionType { entity_tag, operation, mask } => {
                tracing::debug!(entity_id, %entity_tag, %operation, mask, chain_id, "Content: set interaction type");
                // TODO: Find tagged entity and modify interaction flags
            }
            Action::StartMinigame { minigame_type, on_victory_chains } => {
                tracing::info!(entity_id, %minigame_type, ?on_victory_chains, chain_id, "Content: starting minigame");
                // TODO: Send onStartMinigame to client, track victory chains
            }
            Action::SetAggression { entity_tag, level } => {
                tracing::debug!(entity_id, %entity_tag, level, chain_id, "Content: set aggression");
                // TODO: Find tagged NPC and set aggression level
            }
            Action::DestroyTaggedEntity { entity_tag } => {
                tracing::info!(entity_id, %entity_tag, chain_id, "Content: destroying tagged entity");
                // TODO: Find and destroy tagged entity in space
            }
            Action::TriggerTransporter { region_id } => {
                tracing::info!(entity_id, region_id, chain_id, "Content: triggering transporter");
                // TODO: Trigger ring transport sequence
            }
            Action::SystemMessage { message_id } => {
                tracing::info!(entity_id, message_id, chain_id, "Content: system message");
                // TODO: Send onPlayerCommunication with message_id
            }
            Action::AbandonMission { mission_id } => {
                tracing::info!(entity_id, mission_id, chain_id, "Content: abandoning mission");
                super::missions::abandon_mission(entity_id, mission_id, tx, space_mgr).await;
            }
            Action::IncrementCounter { counter_name, amount } => {
                tracing::debug!(entity_id, %counter_name, amount, chain_id, "Content: increment counter");
                // TODO: Increment per-entity counter tracking
            }
            Action::ResetCounter { counter_name } => {
                tracing::debug!(entity_id, %counter_name, chain_id, "Content: reset counter");
                // TODO: Reset per-entity counter
            }
            Action::CompleteObjective { mission_id, objective_id } => {
                tracing::info!(entity_id, mission_id, objective_id, chain_id, "Content: complete objective");
                // TODO: Complete specific objective in mission manager
            }
            Action::SendMessage { channel, message } => {
                tracing::info!(entity_id, %channel, %message, chain_id, "Content: sending message");
            }
            Action::TriggerChain { chain_id: target_chain_id } => {
                tracing::debug!(entity_id, target_chain_id, chain_id, "Content: trigger chain (caller must re-dispatch)");
            }
            other => {
                tracing::debug!(entity_id, chain_id, action = ?other, "Content: unhandled action");
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Determine the inventory container for an item.
fn item_container(item_id: i32) -> i32 {
    match item_id {
        55 | 21 => 3, // weapons → bandolier
        _ => 1,       // general inventory
    }
}

/// Send an `onUpdateItem` (method_index 72) to grant an item at runtime.
async fn grant_item_runtime(
    entity_id: u32,
    item_id: i32,
    container_id: i32,
    count: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let instance_id = item_id * 1000 + 1;
    let slot_id: i32 = 1;

    let mut args = Vec::with_capacity(40);
    args.extend_from_slice(&instance_id.to_le_bytes());
    args.extend_from_slice(&item_id.to_le_bytes());
    args.extend_from_slice(&count.to_le_bytes());
    args.extend_from_slice(&slot_id.to_le_bytes());
    args.extend_from_slice(&container_id.to_le_bytes());
    args.push(0); // isBound = false
    args.extend_from_slice(&100i32.to_le_bytes()); // durability
    args.extend_from_slice(&0u32.to_le_bytes());   // ammoTypes count = 0
    args.extend_from_slice(&0i32.to_le_bytes());   // curAmmoType
    args.extend_from_slice(&0i32.to_le_bytes());   // charges

    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 72,
        args,
    }).await;
}

/// Return the step ID and objectives for a known mission.
fn mission_data(mission_id: i32) -> (i32, Vec<MissionObjective>) {
    match mission_id {
        622 => (
            2113,
            vec![MissionObjective {
                objective_id: 2113,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        ),
        _ => {
            tracing::warn!(mission_id, "No mission data for ID — using empty objectives");
            (0, vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn fallback_engine_has_two_chains() {
        let engine = build_initial_engine();
        assert_eq!(engine.chain_count(), 2);
    }

    #[test]
    fn chain_1_matches_player_loaded_with_mission_not_active() {
        let engine = build_initial_engine();

        let mut ctx = ExecutionContext::new();
        ctx.set_param("mission_622_status".to_string(), serde_json::json!("not_active"));

        let event = TriggerEvent {
            trigger_type: TriggerType::PlayerLoaded,
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };

        let resolved = engine.resolve_event(&event, &ctx);
        assert_eq!(resolved.actions.len(), 1);
        match &resolved.actions[0].1 {
            Action::AcceptMission { mission_id } => assert_eq!(*mission_id, 622),
            other => panic!("Expected AcceptMission, got {:?}", other),
        }
    }

    #[test]
    fn chain_1_skips_when_mission_already_active() {
        let engine = build_initial_engine();

        let mut ctx = ExecutionContext::new();
        ctx.set_param("mission_622_status".to_string(), serde_json::json!("active"));

        let event = TriggerEvent {
            trigger_type: TriggerType::PlayerLoaded,
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };

        let resolved = engine.resolve_event(&event, &ctx);
        assert_eq!(resolved.actions.len(), 0);
    }

    #[test]
    fn chain_2_matches_frostbody_dialog() {
        let engine = build_initial_engine();

        let mut params = HashMap::new();
        params.insert("dialog_id".to_string(), serde_json::json!(3995));

        let event = TriggerEvent {
            trigger_type: TriggerType::DialogOpen,
            source_entity: None,
            target_entity: None,
            params,
        };

        let ctx = ExecutionContext::new();
        let resolved = engine.resolve_event(&event, &ctx);
        assert_eq!(resolved.actions.len(), 3);
    }

    #[test]
    fn chain_2_does_not_match_other_dialog() {
        let engine = build_initial_engine();

        let mut params = HashMap::new();
        params.insert("dialog_id".to_string(), serde_json::json!(1));

        let event = TriggerEvent {
            trigger_type: TriggerType::DialogOpen,
            source_entity: None,
            target_entity: None,
            params,
        };

        let ctx = ExecutionContext::new();
        let resolved = engine.resolve_event(&event, &ctx);
        assert_eq!(resolved.actions.len(), 0);
    }

    #[test]
    fn item_container_mapping() {
        assert_eq!(item_container(55), 3);
        assert_eq!(item_container(21), 3);
        assert_eq!(item_container(3730), 1);
        assert_eq!(item_container(999), 1);
    }

    #[test]
    fn mission_622_data() {
        let (step, objectives) = mission_data(622);
        assert_eq!(step, 2113);
        assert_eq!(objectives.len(), 1);
        assert_eq!(objectives[0].objective_id, 2113);
    }
}
