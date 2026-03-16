//! Content engine bridge for the CellService.
//!
//! Wires the data-driven chain engine into the game loop. Loads chains from the
//! database at startup, fires events from gameplay actions, and executes the
//! resolved actions against the game state.

mod executor;

use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_content_engine::chain::{Chain, ChainEngine};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::loader::{
    DbActionRow, DbChainRow, DbConditionRow, DbTriggerRow, build_chains_from_rows,
};
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

// ── Engine construction ─────────────────────────────────────────────────────

/// Build the content engine by loading chains from the database.
///
/// Returns an empty engine if the DB pool is unavailable or the content
/// tables don't exist yet — all chain data lives in the database.
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
                tracing::error!("Failed to load content chains from DB: {e} — content engine will be empty");
            }
        }
    } else {
        tracing::warn!("No DB pool available — content engine will be empty");
    }

    ChainEngine::new()
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
    if resolved.actions.is_empty() {
        tracing::info!(entity_id, player_id, %world_name, "fire_player_loaded: no chains matched");
    } else {
        for (chain_id, action) in &resolved.actions {
            tracing::info!(entity_id, player_id, %world_name, chain_id, action = ?action, "fire_player_loaded: matched action");
        }
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;

    // Diagnostic: confirm mission 622 state after chain execution
    if let Some(entity) = space_mgr.get_entity(entity_id) {
        let m622_active = entity.missions.get_mission(622)
            .map_or(false, |m| m.status == 1);
        tracing::info!(
            entity_id, player_id, %world_name,
            mission_622_active = m622_active,
            total_missions = entity.missions.count(),
            "fire_player_loaded: post-execute state"
        );
    }
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

    // Diagnostic: show mission 622 state and chain count before resolution
    let mission_status = ctx.params.get("mission_622_status")
        .and_then(|v| v.as_str())
        .unwrap_or("<not set>");
    tracing::info!(
        entity_id, player_id, dialog_id,
        mission_622_status = %mission_status,
        dialog_open_chains = engine.chains_for_trigger(&TriggerType::DialogOpen),
        "fire_dialog_open: resolving"
    );

    let event = TriggerEvent {
        trigger_type: TriggerType::DialogOpen,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);

    tracing::info!(
        entity_id, dialog_id,
        matched_actions = resolved.actions.len(),
        "fire_dialog_open: resolved"
    );

    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;
}

/// Fire `OnInteractTag` event when a player interacts with a tagged entity.
///
/// Returns `true` if a content chain handled the interaction (caller should
/// NOT fall through to the default `handle_interact()` logic).
pub async fn fire_interact_tag(
    entity_id: u32,
    player_id: i32,
    tag: &str,
    target_entity_id: u32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let mut ctx = ExecutionContext::new()
        .with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    ctx.set_param("target_entity_id".to_string(), serde_json::json!(target_entity_id));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTag,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: Some(cimmeria_common::EntityId(target_entity_id as i32)),
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let matched = !resolved.actions.is_empty();
    if matched {
        tracing::info!(entity_id, player_id, %tag, actions = resolved.actions.len(), "fire_interact_tag: matched");
        executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;
    } else {
        tracing::debug!(entity_id, %tag, "fire_interact_tag: no chains matched");
    }
    matched
}

/// Fire `OnInteractTemplate` event when a player interacts with a templated entity.
///
/// Returns `true` if a content chain handled the interaction.
pub async fn fire_interact_template(
    entity_id: u32,
    player_id: i32,
    template_name: &str,
    target_entity_id: u32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> bool {
    let mut ctx = ExecutionContext::new()
        .with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("template_name".to_string(), serde_json::json!(template_name));
    ctx.set_param("target_entity_id".to_string(), serde_json::json!(target_entity_id));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::InteractTemplate,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: Some(cimmeria_common::EntityId(target_entity_id as i32)),
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let matched = !resolved.actions.is_empty();
    if matched {
        tracing::info!(entity_id, player_id, %template_name, actions = resolved.actions.len(), "fire_interact_template: matched");
        executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;
    } else {
        tracing::debug!(entity_id, %template_name, "fire_interact_template: no chains matched");
    }
    matched
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

#[cfg(test)]
mod tests {
    use super::executor::item_container;

    #[test]
    fn item_container_mapping() {
        assert_eq!(item_container(55), 3);
        assert_eq!(item_container(21), 3);
        assert_eq!(item_container(3730), 1);
        assert_eq!(item_container(999), 1);
    }
}
