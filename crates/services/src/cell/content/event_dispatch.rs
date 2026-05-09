//! Event firing — gameplay sites call into these functions to push events
//! through the content engine and execute any matched actions.
//!
//! Each `fire_*` function:
//!   1. Builds an [`ExecutionContext`] populated with event-specific params
//!      and the source entity's mission/archetype state.
//!   2. Constructs a [`TriggerEvent`] and resolves it against the engine.
//!   3. Dispatches the resolved actions via [`super::executor::execute_actions`].

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::executor;
use super::mission_context::{populate_mission_context, populate_stats_context};

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
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;

    // Diagnostic: confirm mission 622 state after chain execution
    if let Some(entity) = space_mgr.get_entity(entity_id) {
        let m622_active = entity
            .missions
            .get_mission(622)
            .is_some_and(|m| m.status == 1);
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
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
    }

    // Diagnostic: show mission 622 state and chain count before resolution
    let mission_status = ctx
        .params
        .get("mission_622_status")
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
        entity_id,
        dialog_id,
        matched_actions = resolved.actions.len(),
        "fire_dialog_open: resolved"
    );

    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
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
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    ctx.set_param(
        "target_entity_id".to_string(),
        serde_json::json!(target_entity_id),
    );

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
        executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
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
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param(
        "template_name".to_string(),
        serde_json::json!(template_name),
    );
    ctx.set_param(
        "target_entity_id".to_string(),
        serde_json::json!(target_entity_id),
    );

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
        executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
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
    let mut ctx =
        ExecutionContext::new().with_source(cimmeria_common::EntityId(killer_entity_id as i32));
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
    executor::execute_actions(resolved, killer_entity_id, player_id, tx, space_mgr, engine).await;
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
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("region_key".to_string(), serde_json::json!(region_tag));

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
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
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
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("region_key".to_string(), serde_json::json!(region_tag));

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
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
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

/// Fire `OnItemUse` event when a player uses an inventory item.
///
/// `item_id` is the item design id (type_id) — drives chain matching on
/// `item_use::<type_id>`. `instance_id` is the inventory row id the
/// player clicked — set into the context so `Action::RemoveItem` can
/// consume that exact stack instead of the player's first-by-type
/// instance (which is wrong when the player has multiple stacks of the
/// same item and clicks anything other than the first one).
pub async fn fire_item_use(
    entity_id: u32,
    player_id: i32,
    instance_id: i32,
    item_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("item_id".to_string(), serde_json::json!(item_id));
    ctx.set_param("instance_id".to_string(), serde_json::json!(instance_id));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        // Stats are needed by `Condition::StatBelowMax` so chains like
        // the Health Slappack (4001) can fizzle silently at full HP
        // instead of burning the stack.
        populate_stats_context(entity, &mut ctx);
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
        tracing::info!(
            entity_id,
            player_id,
            item_id,
            actions = resolved.actions.len(),
            "fire_item_use: matched"
        );
    } else {
        tracing::debug!(entity_id, item_id, "fire_item_use: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}

/// Fire `OnItemEquipped` when an item lands in the bandolier (`container_id = 3`)
/// from another container. Drives chains keyed on `item_equipped::<type_id>` —
/// e.g., the mission 622 / 641 "equip the weapon you just picked up" steps.
pub async fn fire_item_equipped(
    entity_id: u32,
    player_id: i32,
    type_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("item_id".to_string(), serde_json::json!(type_id));

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::ItemEquipped,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(
            entity_id,
            player_id,
            type_id,
            actions = resolved.actions.len(),
            "fire_item_equipped: matched"
        );
    } else {
        tracing::debug!(entity_id, type_id, "fire_item_equipped: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}

/// Fire the `teleport_in` event when a player arrives via a ring transporter.
///
/// Chain 1044 (`teleport_in` event_key=`2`) hooks this to complete mission 640
/// when the player teleports into Castle_CellBlock ring 2. The chain loader
/// converts the SQL `event_key` string into a typed `region_id` field on the
/// trigger, so matching only needs the typed `region_id` param.
pub async fn fire_teleport_in(
    entity_id: u32,
    player_id: i32,
    region_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mut ctx = ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
    ctx.set_param("region_id".to_string(), serde_json::json!(region_id));
    // The content engine's `teleport_in` trigger reads `region_id` as i64 (see
    // `Trigger::OnTeleportIn::matches` in crates/content-engine/src/triggers.rs).
    // No `event_key` params are needed — the loader already converts the SQL
    // event_key string into a typed `region_id` field on the trigger.

    if let Some(entity) = space_mgr.get_entity(entity_id) {
        populate_mission_context(entity, &mut ctx);
        if let Some(archetype_id) = entity.archetype_id {
            ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
        }
    }

    let event = TriggerEvent {
        trigger_type: TriggerType::TeleportIn,
        source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    if !resolved.actions.is_empty() {
        tracing::info!(
            entity_id,
            player_id,
            region_id,
            actions = resolved.actions.len(),
            "fire_teleport_in: matched"
        );
    } else {
        tracing::debug!(entity_id, region_id, "fire_teleport_in: no chains matched");
    }
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}

/// Fire `mission_accepted` immediately after the cell-side mission state
/// has been committed for the calling entity. Called from the executor's
/// combined `Action::AcceptMission | Action::AdvanceMission` branch so
/// chains tied to mission start (e.g., highlighting quest objects like
/// the Cellblock_WoodenCrate for mission 687) run without coupling to
/// the chain that triggered the accept.
///
/// Fires after the in-process `accept_mission` helper updates the
/// CellEntity's mission table — i.e., chain conditions reading
/// `mission_<id>_status` see the post-accept state. The persistence
/// path (the `MissionUpdate` send to base) is best-effort; this event
/// fires whether or not that send succeeded, since the cell's view of
/// mission state is the authoritative source for chain evaluation.
///
/// The return type is an explicit `Pin<Box<dyn Future ...>>` rather than an
/// `async fn` because this fires from inside `executor::execute_actions`
/// (which itself awaits this via the `AcceptMission` branch). An `async fn`
/// would compute its future size from the called future, and since
/// `execute_actions` calls back into here, the type would be infinitely
/// sized at compile time. Boxing breaks the cycle.
///
/// A `mission_accepted` chain that itself calls `accept_mission` for a
/// different mission will re-fire this dispatcher — intentional, so chained
/// accepts work without each one being wired by hand. Bounded in practice
/// because chain authors don't write self-accepting cycles; if that turns
/// out to be optimistic, guard with a depth counter here.
pub(super) fn fire_mission_accepted<'a>(
    entity_id: u32,
    player_id: i32,
    mission_id: i32,
    engine: &'a ChainEngine,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        let mut ctx =
            ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
        ctx.set_param("mission_id".to_string(), serde_json::json!(mission_id));

        if let Some(entity) = space_mgr.get_entity(entity_id) {
            populate_mission_context(entity, &mut ctx);
            if let Some(archetype_id) = entity.archetype_id {
                ctx.set_param("archetype".to_string(), serde_json::json!(archetype_id));
            }
        }

        let event = TriggerEvent {
            trigger_type: TriggerType::MissionAccepted,
            source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
            target_entity: None,
            params: ctx.params.clone(),
        };

        let resolved = engine.resolve_event(&event, &ctx);
        if !resolved.actions.is_empty() {
            tracing::info!(
                entity_id,
                player_id,
                mission_id,
                actions = resolved.actions.len(),
                "fire_mission_accepted: matched"
            );
            executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
        } else {
            tracing::debug!(
                entity_id,
                mission_id,
                "fire_mission_accepted: no chains matched"
            );
        }
    })
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
        tracing::warn!(
            chain_id,
            entity_id,
            "fire_chain_by_id: chain not found or has no actions"
        );
        return;
    }

    tracing::info!(
        chain_id,
        entity_id,
        action_count = actions.len(),
        "fire_chain_by_id: executing"
    );
    let resolved = cimmeria_content_engine::chain::ResolvedActions {
        actions: actions.into_iter().map(|a| (chain_id, a)).collect(),
        ..Default::default()
    };
    executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    // SpaceManager is re-exported through `super::*` because the parent
    // module imports it; no separate `use crate::cell::space_manager::SpaceManager`
    // needed here.

    /// Build the standard Castle-startup-space fixture used across the
    /// abilities/ event-dispatch tests. One player at id=1, one NPC at
    /// id=2, both at the origin so any AoI-related assertion has both
    /// in range.
    fn make_mgr_with_player_and_npc() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
        }
        mgr.connect_entity(1);
        mgr
    }

    /// Empty engine: every fire_* function must complete without
    /// panicking, emit no actions, and (for the bool-returning fns)
    /// return false. Pin so a refactor that adds a `default chain`
    /// fallback or panics on unmatched events gets caught.
    #[tokio::test]
    async fn fire_player_loaded_with_empty_engine_emits_nothing() {
        let mut mgr = make_mgr_with_player_and_npc();
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(8);

        fire_player_loaded(1, 100, "Castle", &engine, &tx, &mut mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "empty engine must produce no wire messages"
        );
    }

    /// `fire_interact_tag` returns false when the engine has no chain
    /// matching the InteractTag trigger. This is the load-bearing
    /// "should the caller fall through to default handling?" signal,
    /// per the doc comment.
    #[tokio::test]
    async fn fire_interact_tag_returns_false_when_no_chain_matches() {
        let mut mgr = make_mgr_with_player_and_npc();
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(8);

        let matched = fire_interact_tag(1, 100, "SomeTag", 2, &engine, &tx, &mut mgr).await;
        assert!(!matched);
    }

    /// `fire_interact_template` mirrors `fire_interact_tag` for
    /// template-keyed interactions. Same return-shape contract.
    #[tokio::test]
    async fn fire_interact_template_returns_false_when_no_chain_matches() {
        let mut mgr = make_mgr_with_player_and_npc();
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(8);

        let matched =
            fire_interact_template(1, 100, "TemplateName", 2, &engine, &tx, &mut mgr).await;
        assert!(!matched);
    }

    /// Missing source entity in the SpaceManager: fire_* must not
    /// panic, must complete cleanly, and must not emit messages. The
    /// pre-mutation entity-vanish branch — common after a destroy —
    /// is the regression shape this guards against.
    #[tokio::test]
    async fn fire_player_loaded_handles_missing_entity_gracefully() {
        let mut mgr = make_mgr_with_player_and_npc();
        // Destroy the player before firing.
        mgr.destroy_entity(1);
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(8);

        fire_player_loaded(1, 100, "Castle", &engine, &tx, &mut mgr).await;
        assert!(rx.try_recv().is_err());
    }

    /// `fire_item_equipped` with no chains registered must complete cleanly
    /// and emit nothing — the bandolier-arrival path runs through this on
    /// every successful manual move into container 3, even when no quest
    /// is gating on the equip event.
    #[tokio::test]
    async fn fire_item_equipped_with_empty_engine_emits_nothing() {
        let mut mgr = make_mgr_with_player_and_npc();
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(8);

        fire_item_equipped(1, 100, 55, &engine, &tx, &mut mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "no chain registered for item_equipped(55) → no wire output",
        );
    }

    /// `fire_item_equipped` resolves a registered `OnItemEquipped`
    /// chain that filters on the matching item id and routes the
    /// resolved actions through the executor. We use `IncrementCounter`
    /// as the observable side-effect (entity.counters is the same map
    /// the executor unit tests pin on; load-bearing path for kill-
    /// counter missions and equip-counter quests alike).
    #[tokio::test]
    async fn fire_item_equipped_runs_chain_with_matching_item_id() {
        use cimmeria_content_engine::actions::Action;
        use cimmeria_content_engine::chain::Chain;
        use cimmeria_content_engine::triggers::Trigger;

        let mut mgr = make_mgr_with_player_and_npc();
        let mut engine = ChainEngine::new();
        engine.register_chain(Chain {
            id: 9001,
            name: "test: equip pistol → bump counter".to_string(),
            enabled: true,
            trigger: Trigger::OnItemEquipped { item_id: Some(55) },
            conditions: vec![],
            actions: vec![Action::IncrementCounter {
                counter_name: "test_pistol_equipped".to_string(),
                amount: 1,
            }],
            priority: 0,
        });

        let (tx, _rx) = mpsc::channel(16);
        fire_item_equipped(1, 100, 55, &engine, &tx, &mut mgr).await;

        let entity = mgr.get_entity(1).expect("player must still exist");
        assert_eq!(
            entity.counters.get("test_pistol_equipped"),
            Some(&1),
            "matched chain must execute IncrementCounter on the entity",
        );
    }

    /// Wildcard `OnItemEquipped { item_id: None }` matches any equipped
    /// type. Pin so a future loader change that auto-collapses
    /// wildcards into typed filters surfaces here.
    #[tokio::test]
    async fn fire_item_equipped_wildcard_chain_matches_any_item() {
        use cimmeria_content_engine::actions::Action;
        use cimmeria_content_engine::chain::Chain;
        use cimmeria_content_engine::triggers::Trigger;

        let mut mgr = make_mgr_with_player_and_npc();
        let mut engine = ChainEngine::new();
        engine.register_chain(Chain {
            id: 9002,
            name: "test: any equip → bump counter".to_string(),
            enabled: true,
            trigger: Trigger::OnItemEquipped { item_id: None },
            conditions: vec![],
            actions: vec![Action::IncrementCounter {
                counter_name: "test_any_equip".to_string(),
                amount: 5,
            }],
            priority: 0,
        });

        let (tx, _rx) = mpsc::channel(16);
        fire_item_equipped(1, 100, 9999, &engine, &tx, &mut mgr).await;

        let entity = mgr.get_entity(1).expect("player must still exist");
        assert_eq!(
            entity.counters.get("test_any_equip"),
            Some(&5),
            "wildcard equip chain must match item_id=9999",
        );
    }

    /// A typed-filter chain on `OnItemEquipped { item_id: Some(55) }`
    /// must NOT fire on a different equipped type. Inverse of the
    /// matching test — guards against the "typo'd integer collapses to
    /// wildcard" bug shape CodeRabbit caught in the loader.
    #[tokio::test]
    async fn fire_item_equipped_typed_chain_does_not_fire_on_other_items() {
        use cimmeria_content_engine::actions::Action;
        use cimmeria_content_engine::chain::Chain;
        use cimmeria_content_engine::triggers::Trigger;

        let mut mgr = make_mgr_with_player_and_npc();
        let mut engine = ChainEngine::new();
        engine.register_chain(Chain {
            id: 9003,
            name: "test: pistol-only".to_string(),
            enabled: true,
            trigger: Trigger::OnItemEquipped { item_id: Some(55) },
            conditions: vec![],
            actions: vec![Action::IncrementCounter {
                counter_name: "test_pistol_only".to_string(),
                amount: 100,
            }],
            priority: 0,
        });

        let (tx, _rx) = mpsc::channel(16);
        // Fire with a different type id.
        fire_item_equipped(1, 100, 21, &engine, &tx, &mut mgr).await;

        let entity = mgr.get_entity(1).expect("player must still exist");
        assert!(
            !entity.counters.contains_key("test_pistol_only"),
            "pistol-only chain must reject equip(21); counter must remain unset, \
             got {:?}",
            entity.counters,
        );
    }

}
