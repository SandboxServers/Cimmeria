//! Mission lifecycle event dispatchers.
//!
//! Fired by the executor after `Action::AcceptMission`, `Action::AdvanceMission`,
//! and `Action::CompleteMission` so chains gated on `mission_accepted` or
//! `mission_completed` triggers see the post-mutation state. Sibling to the
//! generic event-firing functions in `super::mod` that aren't tied to mission
//! state transitions specifically.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::executor;
use super::super::mission_context::populate_mission_context;

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
pub(in crate::cell::content) fn fire_mission_accepted<'a>(
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
            tracing::info!(
                entity_id,
                mission_id,
                event_type = "mission_accepted",
                matched = false,
                "fire_mission_accepted: no chains matched"
            );
        }
    })
}

/// Fire `OnMissionCompleted` event after `Action::CompleteMission` flips
/// the in-process `MissionInstance` to MISSION_COMPLETED. Lets chains
/// gate on a mission completing without having to re-derive completion
/// from `mission_<id>_status == 'completed'` on every event the player
/// generates.
///
/// Mirrors `fire_mission_accepted` exactly (same boxing rationale —
/// chain authors can chain a complete → accept-next which re-enters this
/// dispatcher). Gated by the executor on a real active→completed
/// transition so re-running `Action::CompleteMission` against an
/// already-completed mission doesn't re-fire downstream chains.
pub(in crate::cell::content) fn fire_mission_completed<'a>(
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
            trigger_type: TriggerType::MissionCompleted,
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
                "fire_mission_completed: matched"
            );
            executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;
        } else {
            tracing::info!(
                entity_id,
                mission_id,
                event_type = "mission_completed",
                matched = false,
                "fire_mission_completed: no chains matched"
            );
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    fn make_test_space_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        let cxml =
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr
    }

    fn make_player(mgr: &mut SpaceManager) {
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(42);
        }
        mgr.connect_entity(1);
    }

    fn make_chain(id: i64, trigger: Trigger, actions: Vec<Action>) -> Chain {
        Chain {
            id,
            name: format!("test_chain_{id}"),
            enabled: true,
            trigger,
            conditions: Vec::new(),
            actions,
            priority: 0,
        }
    }

    /// Pin the runtime path that was dead before the executor started
    /// firing this dispatcher. A `mission_completed N` chain must
    /// execute its actions when `fire_mission_completed(.., N, ..)`
    /// runs against an engine that has the chain registered.
    ///
    /// Detection shape: the chain's action is `IncrementCounter`, which
    /// mutates `entity.counters` directly — observable without needing
    /// `mission_defs` populated (which the lighter test SpaceManager
    /// fixture deliberately omits). Replacing the dispatcher body with
    /// `Box::pin(async {})` would leave the counter unset and fail
    /// the assertion below — so the test is a real regression guard,
    /// not a "didn't panic" smoke check.
    #[tokio::test]
    async fn fire_mission_completed_runs_matched_chain_actions() {
        let mut mgr = make_test_space_mgr();
        make_player(&mut mgr);

        let mut engine = ChainEngine::new();
        engine.register_chain(make_chain(
            1105,
            Trigger::OnMissionCompleted { mission_id: 687 },
            vec![Action::IncrementCounter {
                counter_name: "completion_chain_fired".to_string(),
                amount: 1,
            }],
        ));

        let (tx, _rx) = mpsc::channel(64);
        fire_mission_completed(1, 42, 687, &engine, &tx, &mut mgr).await;

        let entity = mgr.get_entity(1).expect("player must still exist");
        assert_eq!(
            entity.counters.get("completion_chain_fired"),
            Some(&1),
            "matched OnMissionCompleted chain must execute its actions",
        );
    }

    /// Negative: when no chain matches the mission_id, the dispatcher
    /// must be a no-op (no executor calls, no panic). Pins the
    /// `resolved.actions.is_empty()` branch.
    #[tokio::test]
    async fn fire_mission_completed_with_no_matching_chain_is_no_op() {
        let mut mgr = make_test_space_mgr();
        make_player(&mut mgr);

        let mut engine = ChainEngine::new();
        engine.register_chain(make_chain(
            9999,
            Trigger::OnMissionCompleted { mission_id: 687 },
            vec![],
        ));

        let (tx, mut rx) = mpsc::channel(64);
        // mission_id 999 does not match the registered trigger key 687.
        fire_mission_completed(1, 42, 999, &engine, &tx, &mut mgr).await;

        // No actions ran → no executor messages drained.
        assert!(
            rx.try_recv().is_err(),
            "fire_mission_completed must be a no-op when no chain matches"
        );
    }

    /// Symmetric coverage for `fire_mission_accepted` — the dispatcher
    /// existed before this PR but had no unit test; the same boxed-future
    /// shape applies and the same null-guard / context-population invariants
    /// hold. Co-locate the test with the dispatcher that owns the behaviour.
    ///
    /// Same detection shape as the completed-counterpart above:
    /// `IncrementCounter` against the player's counters map gives an
    /// observable side-effect without depending on tagged entities
    /// being loaded in the fixture (the production `mission_accepted`
    /// chain uses `SetInteractionType`, but that no-ops silently when
    /// the tag isn't in the space — useless for regression-guarding).
    #[tokio::test]
    async fn fire_mission_accepted_runs_matched_chain_actions() {
        let mut mgr = make_test_space_mgr();
        make_player(&mut mgr);

        let mut engine = ChainEngine::new();
        engine.register_chain(make_chain(
            1097,
            Trigger::OnMissionAccepted { mission_id: 687 },
            vec![Action::IncrementCounter {
                counter_name: "accept_chain_fired".to_string(),
                amount: 1,
            }],
        ));

        let (tx, _rx) = mpsc::channel(64);
        fire_mission_accepted(1, 42, 687, &engine, &tx, &mut mgr).await;

        let entity = mgr.get_entity(1).expect("player must still exist");
        assert_eq!(
            entity.counters.get("accept_chain_fired"),
            Some(&1),
            "matched OnMissionAccepted chain must execute its actions",
        );
    }
}
