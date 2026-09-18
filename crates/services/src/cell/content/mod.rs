//! Content engine bridge for the CellService.
//!
//! Wires the data-driven chain engine into the game loop. Loads chains from the
//! database at startup ([`engine_loader`]), fires events from gameplay actions
//! ([`event_dispatch`]), and executes resolved actions against the game state
//! ([`executor`]).

// NOT `pub` — `effect_apply` bypasses the combat pipeline's caster/target
// gates, and module privacy is what keeps it unreachable from any
// client-dispatched path. See the module doc before widening this.
mod effect_apply;
mod engine_loader;
mod event_dispatch;
mod executor;
mod mission_context;

#[cfg(test)]
mod chain_replay_tests;

// Public surface — preserve the flat `crate::cell::content::<fn>` paths that
// callers across the cell service already use.
pub use engine_loader::build_engine;
pub use event_dispatch::{
    fire_chain_by_id, fire_cover_duration, fire_cover_entered, fire_cover_left, fire_dialog_choice,
    fire_dialog_open, fire_enter_region, fire_entity_death, fire_exit_region, fire_interact_tag,
    fire_interact_template, fire_item_equipped, fire_item_use, fire_npc_flanked,
    fire_player_loaded, fire_teleport_in,
};

#[cfg(test)]
mod tests {
    use super::executor::item_container;
    use super::mission_context::populate_mission_context;
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use cimmeria_content_engine::chain::ChainEngine;
    use cimmeria_entity::missions::{
        MissionInstance, MissionObjective, MISSION_COMPLETED, STATUS_ACTIVE,
    };
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
        map.insert(55, 3); // SI 3 9mm Pistol → bandolier
        map.insert(21, 3); // weapon → bandolier
        map.insert(3730, 2); // Frost's Letter → mission bag
        map.insert(19, 2); // Ambernol Vial → mission bag

        assert_eq!(item_container(55, &map), 3);
        assert_eq!(item_container(21, &map), 3);
        assert_eq!(item_container(3730, &map), 2); // was wrongly 1 before
        assert_eq!(item_container(19, &map), 2); // was wrongly 1 before
        assert_eq!(item_container(999, &map), 1); // unknown item defaults to INV_Main
    }

    // ── populate_mission_context ──────────────────────────────────────────

    #[test]
    fn populate_mission_context_sets_active_status() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();

        // Add an active mission
        let mission = MissionInstance::new(
            622,
            700,
            vec![MissionObjective {
                objective_id: 800,
                status: STATUS_ACTIVE,
                hidden: false,
                optional: false,
            }],
        );
        mgr.get_entity_mut(1).unwrap().missions.add_mission(mission);

        let entity = mgr.get_entity(1).unwrap();
        let mut ctx = cimmeria_content_engine::context::ExecutionContext::new();
        populate_mission_context(entity, &mut ctx);

        assert_eq!(
            ctx.params
                .get("mission_622_status")
                .and_then(|v| v.as_str()),
            Some("active"),
        );
        assert_eq!(
            ctx.params
                .get("mission_622_step_700_status")
                .and_then(|v| v.as_str()),
            Some("active"),
        );
    }

    #[test]
    fn populate_mission_context_sets_completed_status() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();

        let mut mission = MissionInstance::new(622, 700, vec![]);
        mission.complete();
        mgr.get_entity_mut(1).unwrap().missions.add_mission(mission);

        let entity = mgr.get_entity(1).unwrap();
        let mut ctx = cimmeria_content_engine::context::ExecutionContext::new();
        populate_mission_context(entity, &mut ctx);

        assert_eq!(
            ctx.params
                .get("mission_622_status")
                .and_then(|v| v.as_str()),
            Some("completed"),
        );
    }

    #[test]
    fn populate_mission_context_empty_when_no_missions() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();

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
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
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
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();

        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(16);

        fire_exit_region(1, 100, "Castle_Cellblock.Region3", &engine, &tx, &mut mgr).await;
        // No panic = success
    }

    // ── fire_entity_death ────────────────────────────────────────────────

    #[tokio::test]
    async fn fire_entity_death_no_chains_no_crash() {
        let mut mgr = make_test_space_mgr();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();

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
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();

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
