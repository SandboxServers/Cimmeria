//! Content engine bridge for the CellService.
//!
//! Wires the data-driven chain engine into the game loop. Loads chains from the
//! database at startup, fires events from gameplay actions, and executes the
//! resolved actions against the game state.

use sqlx::PgPool;
use tokio::sync::mpsc;

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{Chain, ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::loader::{
    DbActionRow, DbChainRow, DbConditionRow, DbTriggerRow, build_chains_from_rows,
};
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use cimmeria_entity::missions::{MissionObjective, STATUS_ACTIVE};

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
    execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;

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

    execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;
}

/// Fire `OnInteractTag` event when a player interacts with a tagged entity.
///
/// Returns `true` if a content chain handled the interaction (caller should
/// NOT fall through to the default `handle_interact()` logic).
///
/// Reference: `python/cell/SGWPlayer.py:1191-1194`
/// ```python
/// if target.tag is not None and self.first('entity.interact.tag::' + target.tag, ...):
///     return
/// ```
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
        execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;
    } else {
        tracing::debug!(entity_id, %tag, "fire_interact_tag: no chains matched");
    }
    matched
}

/// Fire `OnInteractTemplate` event when a player interacts with a templated entity.
///
/// Returns `true` if a content chain handled the interaction.
///
/// Reference: `python/cell/SGWPlayer.py:1196-1200`
/// ```python
/// if target.template is not None:
///     if self.first('entity.interact.template::' + target.template.templateName, ...):
///         return
/// ```
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
        execute_actions(resolved, entity_id, player_id, tx, space_mgr).await;
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
                if let Some(def) = space_mgr.mission_defs.get(&mission_id) {
                    let step_id = def.step_id;
                    let objectives: Vec<MissionObjective> = def.objectives.iter().map(|o| {
                        MissionObjective {
                            objective_id: o.objective_id,
                            status: STATUS_ACTIVE,
                            hidden: o.is_hidden,
                            optional: o.is_optional,
                        }
                    }).collect();
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
                        failed_objective_ids: vec![],
                    }).await;
                } else {
                    tracing::warn!(mission_id, chain_id, "No mission_defs entry — cannot accept mission");
                }
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
                    failed_objective_ids: vec![],
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
                // The original Python scripts pass the player's own entity_id as the
                // NPC entity for content-triggered dialogs (see Castle_CellBlock.py:16
                // where n83_var_NPC = player). Passing 0 breaks world entry because
                // the client can't resolve entity 0 during the loading transition.
                super::interactions::send_dialog_display(entity_id, entity_id as i32, dialog_id, tx).await;
            }
            Action::PlaySequence { sequence_id } => {
                tracing::info!(entity_id, sequence_id, chain_id, "Content: playing sequence");
                // onSequence (index 1, SGWSpawnableEntity) — 8 args:
                //   KismetEventSetSeqID, SourceID, TargetID, PrimaryTarget,
                //   ImpactTime, NameValuePairs[], ViewType, InstanceId
                // NOTE: method 22 is onMeleeRangeUpdate, NOT a sequence method!
                let mut args = Vec::with_capacity(26);
                args.extend_from_slice(&sequence_id.to_le_bytes()); // KismetEventSetSeqID
                args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // SourceID
                args.extend_from_slice(&(entity_id as i32).to_le_bytes()); // TargetID
                args.push(1);                                       // PrimaryTarget = true
                args.extend_from_slice(&0.0f32.to_le_bytes());     // ImpactTime
                args.extend_from_slice(&0u32.to_le_bytes());        // NameValuePairs count = 0
                args.push(0);                                       // ViewType = 0
                args.extend_from_slice(&0i32.to_le_bytes());        // InstanceId
                let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: 1, // onSequence (SGWSpawnableEntity)
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
                    failed_objective_ids: vec![],
                }).await;
            }
            Action::AddDialogSet { dialog_set_id, slot, mission_id: _ } => {
                // slot = template_id of the NPC entity to make interactable
                // dialog_set_id = dialog_set_map_id (e.g. 5229 for Frost's body)
                tracing::info!(entity_id, dialog_set_id, slot, chain_id, "Content: adding dialog set");

                if let Some(entry) = space_mgr.dialog_set_maps.get(&dialog_set_id).cloned() {
                    tracing::info!(
                        entity_id, dialog_set_id, slot,
                        dialog_id = entry.dialog_id,
                        interaction_flags = entry.interaction_flags,
                        "add_dialog_set: resolved dialog_set_map entry"
                    );

                    // Store in player entity's available_interactions
                    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
                        player.available_interactions
                            .entry(slot)
                            .or_default()
                            .push((dialog_set_id, entry.dialog_id, entry.interaction_flags));

                        tracing::info!(
                            entity_id, slot,
                            interactions_count = player.available_interactions.get(&slot).map_or(0, |v| v.len()),
                            "add_dialog_set: stored in available_interactions"
                        );
                    }

                    // Find the target NPC entity with this template_id in the player's world.
                    if let Some(world_name) = space_mgr.get_entity_world_name(entity_id) {
                        if let Some(&target_id) = space_mgr.find_entities_by_template(&world_name, slot).first() {
                            let target_eid = cimmeria_common::EntityId(target_id as i32);
                            let in_witness_set = space_mgr.get_entity(entity_id)
                                .map_or(false, |p| p.witnesses.contains(&target_eid));

                            if in_witness_set {
                                // Compute merged InteractionType for THIS player
                                let base_flags = space_mgr.get_entity(target_id)
                                    .map(|e| e.interaction_type_flags).unwrap_or(0);
                                let merged = base_flags | entry.interaction_flags;

                                tracing::debug!(
                                    entity_id, target_id, dialog_set_id,
                                    dialog_id = entry.dialog_id, base_flags, merged,
                                    "Sending per-player InteractionType for add_dialog_set"
                                );

                                let _ = tx.send(CellToBaseMsg::WitnessEntityMethod {
                                    witness_id: entity_id,
                                    entity_id: target_id,
                                    method_index: 3, // InteractionType
                                    args: (merged as u64).to_le_bytes().to_vec(),
                                }).await;
                            } else {
                                tracing::debug!(
                                    entity_id, target_id, dialog_set_id,
                                    "NPC not yet in player AoI — deferring InteractionType to AoI create"
                                );
                            }
                        }
                    }
                } else {
                    tracing::warn!(dialog_set_id, "dialog_set_maps cache miss for add_dialog_set");
                }
            }
            Action::RemoveDialogSet { dialog_set_id, slot } => {
                tracing::info!(entity_id, dialog_set_id, slot, chain_id, "Content: removing dialog set");

                // Remove entry from player's available_interactions
                let removed_flags = if let Some(player) = space_mgr.get_entity_mut(entity_id) {
                    if let Some(entries) = player.available_interactions.get_mut(&slot) {
                        entries.retain(|&(dsm_id, _, _)| dsm_id != dialog_set_id);
                        if entries.is_empty() {
                            player.available_interactions.remove(&slot);
                        }
                    }
                    // Recompute merged flags from remaining entries for this slot
                    player.available_interactions.get(&slot)
                        .map(|entries| entries.iter().fold(0i64, |acc, &(_, _, flags)| acc | flags))
                } else {
                    None
                };

                // Send updated InteractionType only if NPC already in player's AoI
                if let Some(world_name) = space_mgr.get_entity_world_name(entity_id) {
                    if let Some(&target_id) = space_mgr.find_entities_by_template(&world_name, slot).first() {
                        let target_eid = cimmeria_common::EntityId(target_id as i32);
                        let in_witness_set = space_mgr.get_entity(entity_id)
                            .map_or(false, |p| p.witnesses.contains(&target_eid));

                        if in_witness_set {
                            let base_flags = space_mgr.get_entity(target_id)
                                .map(|e| e.interaction_type_flags).unwrap_or(0);
                            let merged = base_flags | removed_flags.unwrap_or(0);

                            let _ = tx.send(CellToBaseMsg::WitnessEntityMethod {
                                witness_id: entity_id,
                                entity_id: target_id,
                                method_index: 3, // InteractionType
                                args: (merged as u64).to_le_bytes().to_vec(),
                            }).await;
                        }
                    }
                }
            }
            Action::RemoveItem { item_id, count } => {
                tracing::info!(entity_id, item_id, count, chain_id, "Content: removing item");
                // TODO: Send onRemoveItem + DB persist
            }
            Action::SetInteractionType { entity_tag, operation, mask } => {
                if let Some(world_name) = space_mgr.get_entity_world_name(entity_id) {
                    if let Some(target_id) = space_mgr.find_entity_by_tag(&world_name, &entity_tag) {
                        let new_flags = if let Some(target) = space_mgr.get_entity_mut(target_id) {
                            let old = target.interaction_type_flags;
                            match operation.as_str() {
                                "add" => target.interaction_type_flags |= mask,
                                "remove" => target.interaction_type_flags &= !mask,
                                "set" => target.interaction_type_flags = mask,
                                _ => tracing::warn!(%operation, "Unknown interaction type operation"),
                            }
                            tracing::debug!(
                                entity_id, %entity_tag, target_id, %operation, mask,
                                old, new = target.interaction_type_flags, chain_id,
                                "Content: set interaction type"
                            );
                            Some(target.interaction_type_flags)
                        } else {
                            None
                        };

                        // Broadcast InteractionType(u64) to all players witnessing this entity
                        // Mirrors Python: self.witnesses.InteractionType(self.interactionType)
                        if let Some(flags) = new_flags {
                            let witnesses = space_mgr.get_witnesses_of(target_id);
                            for witness_id in witnesses {
                                let _ = tx.send(CellToBaseMsg::WitnessEntityMethod {
                                    witness_id,
                                    entity_id: target_id,
                                    method_index: 3, // InteractionType
                                    args: (flags as u64).to_le_bytes().to_vec(),
                                }).await;
                            }
                        }
                    } else {
                        tracing::debug!(entity_id, %entity_tag, chain_id, "Content: entity tag not found for SetInteractionType");
                    }
                }
            }
            Action::StartMinigame { minigame_type, on_victory_chains } => {
                tracing::info!(entity_id, %minigame_type, ?on_victory_chains, chain_id, "Content: starting minigame");
                // TODO: Send onStartMinigame to client, track victory chains
            }
            Action::SetAggression { entity_tag, level: agg_level } => {
                if let Some(world_name) = space_mgr.get_entity_world_name(entity_id) {
                    if let Some(target_id) = space_mgr.find_entity_by_tag(&world_name, &entity_tag) {
                        tracing::debug!(entity_id, %entity_tag, target_id, agg_level, chain_id, "Content: set aggression");
                        // Store aggression level as a property for future combat use
                        if let Some(target) = space_mgr.get_entity_mut(target_id) {
                            target.properties.insert(
                                "aggression".to_string(),
                                cimmeria_entity::base_entity::PropertyValue::Int32(agg_level),
                            );
                        }
                    }
                }
            }
            Action::DestroyTaggedEntity { entity_tag } => {
                if let Some(world_name) = space_mgr.get_entity_world_name(entity_id) {
                    if let Some(target_id) = space_mgr.find_entity_by_tag(&world_name, &entity_tag) {
                        tracing::info!(entity_id, %entity_tag, target_id, chain_id, "Content: destroying tagged entity");
                        space_mgr.destroy_entity(target_id);
                    } else {
                        tracing::debug!(entity_id, %entity_tag, chain_id, "Content: entity tag not found for DestroyTaggedEntity");
                    }
                }
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
            Action::AddDialog { dialog_set_id, entity_template, mission_id: _ } => {
                // entity_template = template_id of the NPC (same role as AddDialogSet.slot)
                let slot = match entity_template {
                    Some(tmpl) => tmpl,
                    None => {
                        tracing::warn!(entity_id, dialog_set_id, chain_id, "AddDialog: missing entity_template — skipping");
                        continue;
                    }
                };

                tracing::info!(entity_id, dialog_set_id, slot, chain_id, "Content: add dialog (via entity_template)");

                if let Some(entry) = space_mgr.dialog_set_maps.get(&dialog_set_id).cloned() {
                    tracing::info!(
                        entity_id, dialog_set_id, slot,
                        dialog_id = entry.dialog_id,
                        interaction_flags = entry.interaction_flags,
                        "add_dialog: resolved dialog_set_map entry"
                    );

                    // Store in player entity's available_interactions
                    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
                        player.available_interactions
                            .entry(slot)
                            .or_default()
                            .push((dialog_set_id, entry.dialog_id, entry.interaction_flags));
                    }

                    // Find the target NPC entity with this template_id in the player's world.
                    if let Some(world_name) = space_mgr.get_entity_world_name(entity_id) {
                        if let Some(&target_id) = space_mgr.find_entities_by_template(&world_name, slot).first() {
                            let target_eid = cimmeria_common::EntityId(target_id as i32);
                            let in_witness_set = space_mgr.get_entity(entity_id)
                                .map_or(false, |p| p.witnesses.contains(&target_eid));

                            if in_witness_set {
                                let base_flags = space_mgr.get_entity(target_id)
                                    .map(|e| e.interaction_type_flags).unwrap_or(0);
                                let merged = base_flags | entry.interaction_flags;

                                tracing::debug!(
                                    entity_id, target_id, dialog_set_id,
                                    dialog_id = entry.dialog_id, base_flags, merged,
                                    "Sending per-player InteractionType for add_dialog"
                                );

                                let _ = tx.send(CellToBaseMsg::WitnessEntityMethod {
                                    witness_id: entity_id,
                                    entity_id: target_id,
                                    method_index: 3, // InteractionType
                                    args: (merged as u64).to_le_bytes().to_vec(),
                                }).await;
                            } else {
                                tracing::debug!(
                                    entity_id, target_id, dialog_set_id,
                                    "NPC not yet in player AoI — deferring InteractionType to AoI create"
                                );
                            }
                        }
                    }
                } else {
                    tracing::warn!(dialog_set_id, "dialog_set_maps cache miss for add_dialog");
                }
            }
            Action::GenerateThreat { entity_tag, threat_level } => {
                tracing::debug!(entity_id, ?entity_tag, threat_level, chain_id, "Content: generate threat");
                // Threat/aggro is a combat-system concept — log for now
            }
            Action::SetVisible { entity_tag, visible } => {
                if let Some(world_name) = space_mgr.get_entity_world_name(entity_id) {
                    if let Some(target_id) = space_mgr.find_entity_by_tag(&world_name, &entity_tag) {
                        tracing::debug!(entity_id, %entity_tag, target_id, visible, chain_id, "Content: set visible");
                        let vis_byte: u8 = if visible { 1 } else { 0 };
                        let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                            entity_id: target_id,
                            method_index: 11, // onVisible
                            args: vec![vis_byte],
                        }).await;
                    }
                }
            }
            Action::MoveWaypoint { entity_tag, destination, speed: _ } => {
                if let Some(world_name) = space_mgr.get_entity_world_name(entity_id) {
                    if let Some(target_id) = space_mgr.find_entity_by_tag(&world_name, &entity_tag) {
                        tracing::debug!(entity_id, %entity_tag, target_id, ?destination, chain_id, "Content: move waypoint");
                        // Update the entity's position (instant for now — animated pathing is a future enhancement)
                        space_mgr.update_entity_position(
                            target_id,
                            destination,
                            [0, 0, 0], // direction unchanged
                            [0.0; 3],  // velocity
                        );
                    }
                }
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
///
/// Wire format: `ARRAY<InvItem>` where InvItem is a FIXED_DICT with:
///   id, dbid, stackSize, slotID, containerID, isBound, durability,
///   ammoTypes (ARRAY<INT32>), curAmmoType, charges.
/// The array count prefix is REQUIRED — without it the client reads the
/// first field (instance_id) as the count and crashes.
async fn grant_item_runtime(
    entity_id: u32,
    item_id: i32,
    container_id: i32,
    count: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let instance_id = item_id * 1000 + 1;
    let slot_id: i32 = 1;

    let mut args = Vec::with_capacity(44);
    // ARRAY count prefix (1 item)
    args.extend_from_slice(&1u32.to_le_bytes());
    // InvItem FIXED_DICT fields
    args.extend_from_slice(&instance_id.to_le_bytes());  // id
    args.extend_from_slice(&item_id.to_le_bytes());      // dbid
    args.extend_from_slice(&count.to_le_bytes());         // stackSize
    args.extend_from_slice(&slot_id.to_le_bytes());       // slotID
    args.extend_from_slice(&container_id.to_le_bytes());  // containerID
    args.push(0);                                          // isBound = false
    args.extend_from_slice(&100i32.to_le_bytes());        // durability
    args.extend_from_slice(&0u32.to_le_bytes());          // ammoTypes count = 0
    args.extend_from_slice(&0i32.to_le_bytes());          // curAmmoType
    args.extend_from_slice(&0i32.to_le_bytes());          // charges

    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 72, // onUpdateItem
        args,
    }).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_container_mapping() {
        assert_eq!(item_container(55), 3);
        assert_eq!(item_container(21), 3);
        assert_eq!(item_container(3730), 1);
        assert_eq!(item_container(999), 1);
    }
}
