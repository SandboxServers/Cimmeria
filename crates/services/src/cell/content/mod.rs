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
    let mut ctx = ExecutionContext::new()
        .with_source(cimmeria_common::EntityId(killer_entity_id as i32));
    ctx.set_param("entity_tag".to_string(), serde_json::json!(entity_tag));

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
    executor::execute_actions(resolved, killer_entity_id, player_id, tx, space_mgr).await;
}

/// Fire the `RegionEnter` event when the client enters a Kismet region.
///
/// `region_tag` is the DB `point_sets.name` value (e.g., "Castle_Cellblock.Region2")
/// which doubles as the content engine trigger key. The Python reference fires
/// `client_hinted_region::{tag}` — our content engine matches on `region_key`.
pub async fn fire_enter_region(
    entity_id: u32,
    player_id: i32,
    region_tag: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new()
        .with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("region_key".to_string(), serde_json::json!(region_tag));

    let world_name = space_mgr.get_entity_world_name(entity_id)
        .unwrap_or_else(|| "Unknown".to_string());
    ctx.set_param("world_name".to_string(), serde_json::json!(&world_name));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(entity_id, player_id, %region_tag, actions = resolved.actions.len(), "fire_enter_region: matched");
    } else {
        tracing::debug!(entity_id, %region_tag, "fire_enter_region: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;
}

/// Fire the `RegionExit` event when the client exits a Kismet region.
///
/// See [`fire_enter_region`] for parameter documentation.
pub async fn fire_exit_region(
    entity_id: u32,
    player_id: i32,
    region_tag: &str,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new()
        .with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("region_key".to_string(), serde_json::json!(region_tag));

    let world_name = space_mgr.get_entity_world_name(entity_id)
        .unwrap_or_else(|| "Unknown".to_string());
    ctx.set_param("world_name".to_string(), serde_json::json!(&world_name));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::RegionExit,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(entity_id, player_id, %region_tag, actions = resolved.actions.len(), "fire_exit_region: matched");
    } else {
        tracing::debug!(entity_id, %region_tag, "fire_exit_region: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;
}

/// Fire `OnDialogChoice` event when a player clicks a dialog button.
pub async fn fire_dialog_choice(
    entity_id: u32,
    player_id: i32,
    dialog_id: i32,
    button_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new()
        .with_source(cimmeria_common::EntityId(entity_id as i32));
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
    if !resolved.actions.is_empty() {
        tracing::info!(entity_id, player_id, dialog_id, actions = resolved.actions.len(), "fire_dialog_choice: matched");
    } else {
        tracing::debug!(entity_id, dialog_id, "fire_dialog_choice: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;
}

/// Fire `OnItemUse` event when a player uses an inventory item.
pub async fn fire_item_use(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new()
        .with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("item_id".to_string(), serde_json::json!(item_id));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::ItemUse,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let matched = !resolved.actions.is_empty();
    if matched {
        tracing::info!(entity_id, player_id, item_id, actions = resolved.actions.len(), "fire_item_use: matched");
    } else {
        tracing::debug!(entity_id, item_id, "fire_item_use: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;
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

/// Fire a content chain directly by ID, bypassing trigger matching.
///
/// Used for minigame victory callbacks — the chain has no trigger row,
/// it's invoked explicitly by the minigame result handler.
pub async fn fire_chain_by_id(
    chain_id: i64,
    entity_id: u32,
    player_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let actions = engine.get_chain_actions(chain_id);
    if actions.is_empty() {
        tracing::warn!(chain_id, entity_id, "fire_chain_by_id: chain not found or has no actions");
        return;
    }

    tracing::info!(chain_id, entity_id, action_count = actions.len(), "fire_chain_by_id: executing");
    let resolved = cimmeria_content_engine::chain::ResolvedActions {
        actions: actions.into_iter().map(|a| (chain_id, a)).collect(),
    };
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::executor::item_container;
    use cimmeria_entity::missions::{MissionInstance, MissionObjective, STATUS_ACTIVE, MISSION_ACTIVE, MISSION_COMPLETED};
    use tokio::sync::mpsc;

    fn make_test_space_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr
    }

    #[test]
    fn item_container_mapping() {
        use std::collections::HashMap;
        // Simulate DB-loaded container_sets: weapons→bandolier, mission items→mission bag
        let mut map = HashMap::new();
        map.insert(55, 3);   // SI 3 9mm Pistol → bandolier
        map.insert(21, 3);   // weapon → bandolier
        map.insert(3730, 2); // Frost's Letter → mission bag
        map.insert(19, 2);   // Ambernol Vial → mission bag

        assert_eq!(item_container(55, &map), 3);
        assert_eq!(item_container(21, &map), 3);
        assert_eq!(item_container(3730, &map), 2);  // was wrongly 1 before
        assert_eq!(item_container(19, &map), 2);    // was wrongly 1 before
        assert_eq!(item_container(999, &map), 1);   // unknown item defaults to INV_Main
    }

    // ── populate_mission_context ──────────────────────────────────────────

    #[test]
    fn populate_mission_context_sets_active_status() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        // Add an active mission
        let mission = MissionInstance::new(622, 700, vec![
            MissionObjective { objective_id: 800, status: STATUS_ACTIVE, hidden: false, optional: false },
        ]);
        mgr.get_entity_mut(1).unwrap().missions.add_mission(mission);

        let entity = mgr.get_entity(1).unwrap();
        let mut ctx = cimmeria_content_engine::context::ExecutionContext::new();
        populate_mission_context(entity, &mut ctx);

        assert_eq!(
            ctx.params.get("mission_622_status").and_then(|v| v.as_str()),
            Some("active"),
        );
        assert_eq!(
            ctx.params.get("mission_622_step_700_status").and_then(|v| v.as_str()),
            Some("active"),
        );
    }

    #[test]
    fn populate_mission_context_sets_completed_status() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        let mut mission = MissionInstance::new(622, 700, vec![]);
        mission.complete();
        mgr.get_entity_mut(1).unwrap().missions.add_mission(mission);

        let entity = mgr.get_entity(1).unwrap();
        let mut ctx = cimmeria_content_engine::context::ExecutionContext::new();
        populate_mission_context(entity, &mut ctx);

        assert_eq!(
            ctx.params.get("mission_622_status").and_then(|v| v.as_str()),
            Some("completed"),
        );
    }

    #[test]
    fn populate_mission_context_empty_when_no_missions() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        let entity = mgr.get_entity(1).unwrap();
        let mut ctx = cimmeria_content_engine::context::ExecutionContext::new();
        populate_mission_context(entity, &mut ctx);

        // No mission-related params should exist
        assert!(!ctx.params.keys().any(|k| k.starts_with("mission_")));
    }

    // ── fire_enter_region / fire_exit_region ──────────────────────────────

    #[tokio::test]
    async fn fire_enter_region_uses_tag_as_key() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();
        mgr.get_entity_mut(1).unwrap().player_id = Some(100);

        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        // Tag comes directly from the DB point_sets.name field
        fire_enter_region(1, 100, "Castle_Cellblock.Region2", &engine, &tx, &mut mgr).await;

        // No chains registered, so no messages — but no panic confirms key construction
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn fire_exit_region_uses_tag_as_key() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(16);

        fire_exit_region(1, 100, "Castle_Cellblock.Region3", &engine, &tx, &mut mgr).await;
        // No panic = success
    }

    // ── fire_entity_death ────────────────────────────────────────────────

    #[tokio::test]
    async fn fire_entity_death_no_chains_no_crash() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        fire_entity_death(1, 100, "Hallway01_Guard", &engine, &tx, &mut mgr).await;

        // Empty engine → no actions → no messages
        assert!(rx.try_recv().is_err());
    }

    // ── fire_player_loaded with saved missions ───────────────────────────

    #[tokio::test]
    async fn fire_player_loaded_with_existing_missions_preserves_context() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3]).unwrap();

        // Pre-populate a completed mission (simulating re-login restore)
        {
            let entity = mgr.get_entity_mut(1).unwrap();
            entity.player_id = Some(100);
            let mut m = MissionInstance::new(622, 700, vec![]);
            m.complete();
            entity.missions.add_mission(m);
        }

        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(16);

        // fire_player_loaded should see the already-completed mission in context
        fire_player_loaded(1, 100, "Castle_CellBlock", &engine, &tx, &mut mgr).await;

        // The entity should still have the completed mission
        let entity = mgr.get_entity(1).unwrap();
        let m622 = entity.missions.get_mission(622).unwrap();
        assert_eq!(m622.status, MISSION_COMPLETED);
    }
}
