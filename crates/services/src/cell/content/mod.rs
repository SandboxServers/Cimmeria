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
    fire_dialog_open, fire_enter_region, fire_entity_death, fire_entity_health_below,
    fire_exit_region, fire_health_below_for_hit, fire_interact_tag, fire_interact_template,
    fire_item_equipped, fire_item_use, fire_npc_flanked, fire_pending_health_below,
    fire_player_flanked_npc, fire_player_loaded, fire_stargate_crossed, fire_stargate_dialed,
    fire_teleport_in,
};
// Cell-tick drain for `content_actions.delay_ms > 0` (C08a) — called once
// per tick from `cell::service::message_loop`, same flat depth as the
// `fire_*` dispatchers above.
pub(crate) use executor::deferred_content_action_tick;
// Re-entrancy bound for the H52 step-activation region replay. Re-exported
// because the guard's state lives on `SpaceManager` (the `&mut` borrow the
// whole recursion already threads) while its logic belongs with the
// dispatcher that owns it.
pub(crate) use event_dispatch::{fire_step_activation_regions, StepRegionReplayGuard};

#[cfg(test)]
mod tests {
    use super::executor::item_container;
    use super::mission_context::{populate_mission_context, populate_world_context};
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

    // ── populate_world_context (Harset H07) ───────────────────────────────

    /// A space manager holding both Harset worlds with their real
    /// `resources.worlds.world_id` values stamped on. Harset (57) is
    /// non-instanced; Harset_CmdCenter (68) is instanced, matching
    /// `entities/spaces.xml`.
    fn make_harset_space_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="Harset" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
            <Space WorldName="Harset_CmdCenter" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" />
        </Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        mgr.stamp_world_rows(&std::collections::HashMap::from([
            (
                "Harset".to_string(),
                crate::cell::spawner::WorldRow::enforcing(57),
            ),
            (
                "Harset_CmdCenter".to_string(),
                crate::cell::spawner::WorldRow::enforcing(68),
            ),
        ]));
        mgr
    }

    /// The populator resolves the DB world id, and both forms land on the
    /// context.
    ///
    /// The id is the *world's*, never the space's: a space id is
    /// `(cell_id << 16) | local_index`, so conflating the two (as
    /// `console/net.rs` does when building `onMapInfo`) would make every
    /// `world` condition compare against a runtime handle. There is no
    /// `assert_ne!` against the space id here — with `SpaceManager::new(1)`
    /// the first space id is 65536, so such an assertion could never fail
    /// and would only decorate the test. [`Self::
    /// populate_world_context_distinguishes_the_two_harset_worlds`] is what
    /// actually rules out a populator returning something constant.
    #[test]
    fn populate_world_context_sets_the_db_world_id() {
        let mut mgr = make_harset_space_mgr();
        mgr.create_entity(1, "Harset", [0.0; 3], [0.0; 3]).unwrap();

        let mut ctx = cimmeria_content_engine::context::ExecutionContext::new();
        populate_world_context(1, &mgr, &mut ctx);

        assert_eq!(ctx.world_id, Some(57), "Harset is world 57");
        assert_eq!(
            ctx.params.get("world_name").and_then(|v| v.as_str()),
            Some("Harset"),
            "the string form the region dispatchers have always emitted must survive",
        );
    }

    /// Two players in two different worlds must get two different ids —
    /// this is the whole point of the condition, and a populator that
    /// resolved a constant would pass the single-world test above.
    #[test]
    fn populate_world_context_distinguishes_the_two_harset_worlds() {
        let mut mgr = make_harset_space_mgr();
        mgr.create_entity(1, "Harset", [0.0; 3], [0.0; 3]).unwrap();
        mgr.create_entity(2, "Harset_CmdCenter", [0.0; 3], [0.0; 3])
            .unwrap();

        let mut outer = cimmeria_content_engine::context::ExecutionContext::new();
        populate_world_context(1, &mgr, &mut outer);
        let mut inner = cimmeria_content_engine::context::ExecutionContext::new();
        populate_world_context(2, &mgr, &mut inner);

        assert_eq!(outer.world_id, Some(57));
        assert_eq!(inner.world_id, Some(68));
    }

    /// An unknown entity leaves `world_id` unset rather than defaulting.
    /// `Condition::World` reads that as fail-closed; a sentinel like `0`
    /// would be indistinguishable from a real world id.
    #[test]
    fn populate_world_context_leaves_world_id_unset_for_unknown_entity() {
        let mgr = make_harset_space_mgr();
        let mut ctx = cimmeria_content_engine::context::ExecutionContext::new();
        populate_world_context(9999, &mgr, &mut ctx);

        assert_eq!(ctx.world_id, None);
        assert_eq!(
            ctx.params.get("world_name").and_then(|v| v.as_str()),
            Some("Unknown"),
        );
    }

    /// Before `stamp_world_ids` runs (DB down at startup, or a world with
    /// no `resources.worlds` row), the world is known by name but has no
    /// id. `world_id` must stay `None` so world-gated chains fail closed
    /// instead of matching an arbitrary world.
    #[test]
    fn populate_world_context_unstamped_world_yields_no_world_id() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(cxml).unwrap();
        // Deliberately no `stamp_world_ids`.
        mgr.create_entity(1, "Harset", [0.0; 3], [0.0; 3]).unwrap();

        let mut ctx = cimmeria_content_engine::context::ExecutionContext::new();
        populate_world_context(1, &mgr, &mut ctx);

        assert_eq!(ctx.world_id, None);
        assert_eq!(
            ctx.params.get("world_name").and_then(|v| v.as_str()),
            Some("Harset"),
            "the name is still known — only the DB-sourced id is missing",
        );
    }

    /// `fire_player_loaded` takes its world from the *caller*, and the
    /// player may not be in a space yet when it fires — at which point the
    /// space-derived name `populate_world_context` computes degrades to
    /// `"Unknown"`. `OnPlayerLoaded`'s optional `world_name` filter matches
    /// on that param, so the caller's value has to win.
    ///
    /// The guard: an entity that exists in no space at all, with a chain
    /// scoped to `world_name = "Harset"` **and** gated on `world eq 57`.
    /// Move `populate_world_context` below the explicit `set_param` in
    /// `lifecycle.rs` and the param becomes `"Unknown"`, the trigger stops
    /// matching, and no `GrantXP` arrives. Drop the `world_id_for_world`
    /// fallback below it and the condition fails closed instead — same
    /// red, different cause, and the two are distinguished by which of the
    /// two chains below still fires.
    #[tokio::test]
    async fn fire_player_loaded_resolves_the_caller_supplied_world() {
        use cimmeria_content_engine::actions::Action;
        use cimmeria_content_engine::chain::Chain;
        use cimmeria_content_engine::conditions::{ComparisonOp, Condition};
        use cimmeria_content_engine::triggers::Trigger;

        /// Distinct, non-round amounts: the trigger-only chain and the
        /// world-gated one have to be told apart, and `1` is one edit away
        /// from the executor's `amount == 0` early-return path.
        const TRIGGER_ONLY_XP: u64 = 6_201;
        const WORLD_GATED_XP: u64 = 6_202;

        let world_scoped = |id: i64, xp: u64, conditions: Vec<Condition>| Chain {
            id,
            name: format!("world-scoped player_loaded {id}"),
            enabled: true,
            trigger: Trigger::OnPlayerLoaded {
                world_name: Some("Harset".to_string()),
            },
            conditions,
            actions: vec![Action::GrantXP { amount: xp }],
            action_delays: Vec::new(),
            priority: 0,
        };

        let mut engine = ChainEngine::new();
        engine.register_chain(world_scoped(1, TRIGGER_ONLY_XP, Vec::new()));
        engine.register_chain(world_scoped(
            2,
            WORLD_GATED_XP,
            vec![Condition::World {
                operator: ComparisonOp::Eq,
                world_id: 57,
            }],
        ));

        // No `create_entity` — this is the pre-space window, where the
        // entity → space → world chain cannot resolve anything.
        let mut mgr = make_harset_space_mgr();
        let (tx, mut rx) = mpsc::channel(16);
        fire_player_loaded(4242, 100, "Harset", &engine, &tx, &mut mgr).await;

        let mut granted: Vec<u64> = std::iter::from_fn(|| rx.try_recv().ok())
            .filter_map(|m| match m {
                crate::cell::messages::CellToBaseMsg::GrantXP { xp_amount, .. } => Some(xp_amount),
                _ => None,
            })
            .collect();
        granted.sort_unstable();

        assert_eq!(
            granted,
            vec![TRIGGER_ONLY_XP, WORLD_GATED_XP],
            "both chains must fire. Missing {TRIGGER_ONLY_XP} means the \
             space-derived \"Unknown\" overwrote the caller's world name and the \
             trigger stopped matching; missing {WORLD_GATED_XP} means world_id was \
             left unset in the pre-space window, so `world eq 57` failed closed",
        );
    }

    /// `fire_enter_region` must populate the world, in the gating no-DB
    /// test job.
    ///
    /// The live-DB sibling in
    /// `chain_replay_tests::world_condition` is the one that pins the
    /// *seed shape* (`condition_type='world'`, id in `target_id`), but it
    /// self-skips via `require_db_or_skip!` and still reports `ok` — so on
    /// its own it cannot be trusted to catch a dropped
    /// `populate_world_context` call during a revert-verify run without
    /// `DATABASE_URL`. This one has no such escape hatch.
    ///
    /// Region enter is the load-bearing site: `region_key` is a bare
    /// `point_sets.name`, so the Harset Command Center door chain and its
    /// mirror inside `Harset_CmdCenter` are only distinguishable by world.
    #[tokio::test]
    async fn fire_enter_region_gates_on_the_players_world() {
        use cimmeria_content_engine::actions::Action;
        use cimmeria_content_engine::chain::Chain;
        use cimmeria_content_engine::conditions::{ComparisonOp, Condition};
        use cimmeria_content_engine::triggers::Trigger;

        const DOOR_XP: u64 = 6_203;
        const REGION_TAG: &str = "CIMMERIA_TEST_H07.CommandCenterTransition";

        let mut engine = ChainEngine::new();
        engine.register_chain(Chain {
            id: 1,
            name: "Harset-only door chain".to_string(),
            enabled: true,
            trigger: Trigger::OnRegionEnter {
                region_key: REGION_TAG.to_string(),
            },
            conditions: vec![Condition::World {
                operator: ComparisonOp::Eq,
                world_id: 57,
            }],
            actions: vec![Action::GrantXP { amount: DOOR_XP }],
            action_delays: Vec::new(),
            priority: 0,
        });

        async fn enter_region_from(world_name: &str, engine: &ChainEngine, tag: &str) -> Vec<u64> {
            let mut mgr = make_harset_space_mgr();
            mgr.create_entity(1, world_name, [0.0; 3], [0.0; 3])
                .unwrap();
            mgr.get_entity_mut(1).unwrap().player_id = Some(100);
            let (tx, mut rx) = mpsc::channel(16);
            fire_enter_region(1, 100, tag, engine, &tx, &mut mgr).await;
            std::iter::from_fn(|| rx.try_recv().ok())
                .filter_map(|m| match m {
                    crate::cell::messages::CellToBaseMsg::GrantXP { xp_amount, .. } => {
                        Some(xp_amount)
                    }
                    _ => None,
                })
                .collect()
        }

        assert_eq!(
            enter_region_from("Harset", &engine, REGION_TAG).await,
            vec![DOOR_XP],
            "the door chain must fire in Harset (57). An empty list means \
             `fire_enter_region` no longer calls `populate_world_context`, so \
             `world eq 57` fails closed on a context with no world_id",
        );
        assert!(
            enter_region_from("Harset_CmdCenter", &engine, REGION_TAG)
                .await
                .is_empty(),
            "the same region tag one world over must NOT fire the chain — \
             `OnRegionEnter` carries no world, so the condition is the only \
             thing separating the door from its mirror",
        );
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
