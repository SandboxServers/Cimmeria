//! Event firing — gameplay sites call into these functions to push events
//! through the content engine and execute any matched actions.
//!
//! Each `fire_*` function:
//!   1. Builds an [`ExecutionContext`] populated with event-specific params
//!      and the source entity's mission/archetype state.
//!   2. Constructs a [`TriggerEvent`] and resolves it against the engine.
//!   3. Dispatches the resolved actions via [`super::executor::execute_actions`].
//!
//! Functions are grouped by event family in sibling submodules:
//!
//! - [`lifecycle`]   — `fire_player_loaded`, `fire_entity_death`
//! - [`interaction`] — `fire_interact_tag`, `fire_interact_template`
//! - [`region`]      — `fire_enter_region`, `fire_exit_region`, `fire_teleport_in`
//! - [`inventory`]   — `fire_item_use`, `fire_item_equipped`
//! - [`dialog`]      — `fire_dialog_open`, `fire_dialog_choice`
//! - [`mission`]     — `fire_mission_accepted`, `fire_mission_completed`
//!
//! `fire_chain_by_id` (chain-id-driven dispatch, used by the minigame
//! victory callback path) doesn't fit the trigger-keyed pattern and stays
//! here as a sibling utility.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::executor;

mod dialog;
mod interaction;
mod inventory;
mod lifecycle;
mod mission;
mod region;

pub use dialog::{fire_dialog_choice, fire_dialog_open};
pub use interaction::{fire_interact_tag, fire_interact_template};
pub use inventory::{fire_item_equipped, fire_item_use};
pub use lifecycle::{fire_entity_death, fire_player_loaded};
pub use region::{fire_enter_region, fire_exit_region, fire_teleport_in};

pub(super) use mission::{fire_mission_accepted, fire_mission_completed};

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
