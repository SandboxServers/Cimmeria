//! CB-N07a — the barracks escape response: chain 11020 in
//! `castle_cellblock_chains.sql` speaks one NID Guard alarm line (screen
//! 200170) on the first entry to `Castle_Cellblock.Region12` while mission
//! 687 step 2355 ("Eliminate the guards in the barracks.") is active.
//! Sealed in `docs/analysis/cellblock-continuation/worknotes/cb-n07-seal.md`.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;

use super::super::engine_loader::load_single_chain_for_test;
use super::assert_no_deferred_actions;
use crate::test_support::require_db_or_skip;

type Params = &'static [(&'static str, &'static str)];

const CHAIN_ID: i32 = 11020;
const REGION_KEY: &str = "Castle_Cellblock.Region12";
/// Another Cellblock room (chain 1084 accepts 687 there).
const OTHER_REGION_KEY: &str = "Castle_Cellblock.Region6";
const SCREEN_ID: i32 = 200170;
const SPEAKER: &str = "NID Guard";
/// `CHAN_say`; the loader accepts no other channel.
const CHAN_SAY: u8 = 0;
const STEP_KEY: &str = "mission_687_step_2355_status";
/// The two adjacent wrong states of step 2355 that must stay silent.
const REFUSALS: &[(&str, Params)] = &[
    (
        "step 2354 (the crate) is still active",
        &[(STEP_KEY, "not_active")],
    ),
    ("the barracks are cleared", &[(STEP_KEY, "completed")]),
];

async fn engine_for(pool: &PgPool, chain_id: i32) -> ChainEngine {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist and assemble"));
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

fn resolve_region_enter(engine: &ChainEngine, region_key: &str, params: Params) -> ResolvedActions {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_key".to_string(), serde_json::json!(region_key));
    for (k, v) in params {
        ctx.set_param((*k).to_string(), serde_json::json!(*v));
    }
    let event = TriggerEvent {
        trigger_type: TriggerType::RegionEnter,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

#[tokio::test]
async fn live_db_cellblock_barracks_response_barks_once_while_2355_is_active() {
    let pool = require_db_or_skip!();
    let id = i64::from(CHAIN_ID);
    let engine = engine_for(&pool, CHAIN_ID).await;
    assert!(
        engine.is_once(id),
        "chain {id} must be once = true (H52 replay)"
    );
    let resolved = resolve_region_enter(&engine, REGION_KEY, &[(STEP_KEY, "active")]);
    let barks: Vec<&Action> = resolved
        .actions
        .iter()
        .filter(|(c, a)| *c == id && matches!(a, Action::NpcBark { .. }))
        .map(|(_, a)| a)
        .collect();
    assert_eq!(
        barks.len(),
        1,
        "chain {id} must resolve one NpcBark: {:?}",
        resolved.actions
    );
    match barks[0] {
        Action::NpcBark {
            screen_id,
            speaker,
            channel,
        } => {
            assert_eq!(*screen_id, SCREEN_ID, "chain {id} speaks the wrong screen");
            assert_eq!(speaker, SPEAKER, "chain {id} has the wrong chat prefix");
            assert_eq!(*channel, CHAN_SAY, "chain {id} must ride CHAN_say");
        }
        other => panic!("filtered for NpcBark but got {other:?}"),
    }
    assert_no_deferred_actions(&resolved, id);
}

#[tokio::test]
async fn live_db_cellblock_barracks_response_refuses_the_crate_and_cleared_phases() {
    let pool = require_db_or_skip!();
    let id = i64::from(CHAIN_ID);
    let engine = engine_for(&pool, CHAIN_ID).await;
    for (why, params) in REFUSALS {
        let resolved = resolve_region_enter(&engine, REGION_KEY, params);
        assert!(
            !resolved.actions.iter().any(|(c, _)| *c == id),
            "chain {id} must not resolve when {why}: {:?}",
            resolved.actions
        );
    }
}

#[tokio::test]
async fn live_db_cellblock_barracks_response_refuses_another_room() {
    let pool = require_db_or_skip!();
    let id = i64::from(CHAIN_ID);
    let engine = engine_for(&pool, CHAIN_ID).await;
    let resolved = resolve_region_enter(&engine, OTHER_REGION_KEY, &[(STEP_KEY, "active")]);
    assert!(
        !resolved.actions.iter().any(|(c, _)| *c == id),
        "chain {id} must not resolve outside Region12: {:?}",
        resolved.actions
    );
}
