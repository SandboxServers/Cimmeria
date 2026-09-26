//! Mission 742's relog-restore acceptance for step 2504 (Harset H50).
//!
//! Cut from `cimmeria-cell-content`'s `chain_replay_tests::mission_742` in
//! wave C3 of the services crate split
//! (docs/architecture/services-crate-split.md): it drives the relog
//! hydration, `player_init::mission_restore::build_restored_missions`,
//! which is the cell service's and still in this crate. `BASKETS`,
//! `engine_with`, `fire` and `actions_of` are copies of that file's.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::content::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

const HARSET: i32 = 57;
/// `(tag, own objective, other objective, other objective)` for the
/// three listening-device baskets. The pairing is the whole point of
/// chains 6107-6109: each "final" chain names its own objective plus
/// the two it must see already completed.
const BASKETS: [(&str, i32, i32, i32); 3] = [
    ("FirstBug", 2913, 2914, 2915),
    ("SecondBug", 2914, 2913, 2915),
    ("ThirdBug", 2915, 2913, 2914),
];

/// Load one chain by id into an otherwise-empty engine.
async fn engine_with(pool: &sqlx::PgPool, chain_id: i32) -> ChainEngine {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

fn fire(
    engine: &ChainEngine,
    trigger_type: TriggerType,
    ctx: &ExecutionContext,
) -> ResolvedActions {
    let event = TriggerEvent {
        trigger_type,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, ctx)
}

/// Actions contributed by one chain, in resolved order.
fn actions_of(resolved: &ResolvedActions, chain_id: i64) -> Vec<Action> {
    resolved
        .actions
        .iter()
        .filter(|(id, _)| *id == chain_id)
        .map(|(_, a)| a.clone())
        .collect()
}

/// **Inverted 2026-09-19 when H50 landed.** This test used to pin the
/// defect: it hand-built the instance `player_init` produced, asserted
/// the objective params were MISSING, and told the next reader to invert
/// it. The three halves of the fix (objective ids in
/// `active_objective_ids`, a `MissionUpdate` from the
/// `complete_objective` arm, and def-driven hydration that carries
/// `hidden`/`optional`) make all three assertions flip.
///
/// What it guards now: the *production* hydration path rebuilds step
/// 2504's roster, so the `objective_status` gates every other assertion
/// in this module seeds by hand are reachable for a real relogged
/// player, and chain 6104 is live rather than dead.
///
/// The end-to-end loop — executor arm through `MissionUpdate` through
/// hydration through `populate_mission_context` — lives in
/// [`super::mission_relog_persistence`]. This one is 742's own claim on
/// the basket relog-restore acceptance, which the header of
/// `chain_replay_tests::mission_742` used to disclaim.
#[tokio::test]
async fn hydrated_step_2504_carries_the_objective_params_and_lights_chain_6104() {
    use crate::cell::messages::SavedMission;
    use crate::cell::service::base_messages::player_init::mission_restore::build_restored_missions;
    use crate::cell::space_manager::SpaceManager;
    use crate::cell::spawner::{load_mission_defs, load_step_objectives};
    use cimmeria_entity::missions::MISSION_ACTIVE;

    let pool = require_db_or_skip!();

    // The real caches: hydration reconstructs the roster from
    // `resources.mission_objectives`, so hand-seeding them here would
    // make the test agree with itself — which is what the pinned
    // version did.
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset" Instanced="false" MinX="-1200" MaxX="1200" MinY="-1200" MaxY="1200" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Harset" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.mission_defs = load_mission_defs(&pool).await.unwrap();
    mgr.step_objectives = load_step_objectives(&pool).await.unwrap();

    // Deliberately the PRE-H50 row: `active_objective_ids` holding the
    // step id is what every row written before this fix contains, and
    // the repo does no DB migrations. If this test seeded the post-fix
    // array instead it would pass under a reverted hydration too — the
    // same self-agreement that let the old pin survive.
    let saved = SavedMission {
        mission_id: 742,
        status: MISSION_ACTIVE,
        current_step_id: Some(2504),
        completed_step_ids: vec![2502, 2503],
        completed_objective_ids: vec![],
        active_objective_ids: vec![2504],
        failed_objective_ids: vec![],
        repeats: 0,
    };
    let hydrated = build_restored_missions(&[saved], &mgr)
        .into_iter()
        .next()
        .expect("one saved row hydrates to one instance");

    let mut ctx = ExecutionContext::new();
    ctx.world_id = Some(HARSET);
    ctx.set_param("entity_tag".to_string(), serde_json::json!("FirstBug"));
    ctx.set_param(
        format!(
            "mission_742_step_{}_status",
            hydrated.current_step_id.unwrap()
        ),
        serde_json::json!("active"),
    );
    // Mirror `populate_mission_context`'s objective loop over exactly the
    // hydrated instance — no hand-seeded objective ids.
    for obj in &hydrated.active_objectives {
        ctx.set_param(
            format!("mission_742_obj_{}_status", obj.objective_id),
            serde_json::json!("active"),
        );
    }

    for (_, own, _, _) in BASKETS.iter().map(|b| (b.0, b.1, b.2, b.3)) {
        assert!(
            ctx.params
                .contains_key(&format!("mission_742_obj_{own}_status")),
            "objective {own} must survive hydration — the whole basket              mechanism is gated on `objective_status`",
        );
    }
    assert!(
        !ctx.params.contains_key("mission_742_obj_2504_status"),
        "the STEP id must not come back as a pseudo-objective",
    );

    let engine = engine_with(&pool, 6104).await;
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_eq!(
        actions_of(&resolved, 6104).len(),
        2,
        "chain 6104 must resolve against a hydrated context — pre-H50 it          was inert for every relogged player",
    );
}
