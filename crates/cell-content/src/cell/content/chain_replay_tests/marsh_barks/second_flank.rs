//! Chain 1178 — the second flank cue on entering Hallway05.
//!
//! Screen 96354 is screen 96352 with the table taken out of it: the same
//! play, phrased for the zone's only other flanking encounter. The region
//! is `Castle_Cellblock.Region5`, which contains both `Hallway05_Guard`
//! spawns.
//!
//! The gate is mission 686 active and 687 not yet active, the same shape
//! as chain 1177's. The first version copied chain 1083's gate (685
//! completed, 686 NOT active), on the belief that 1083 accepts 686 on this
//! crossing. In a real run chain 1091 has already accepted 686 when the
//! Hallway04 guard died, a room earlier, so that gate was never open and
//! the line never played for anyone (2026-09-26 colo UAT).
//! [`chain_1178_barks_in_the_state_the_hallway04_kill_leaves_behind`]
//! walks that real sequence and is the regression guard.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::{
    assert_refused, assert_single_bark, engine_with, load, resolve_region_enter, CHAIN_HALLWAY05,
    REGION_HALLWAY05, SCREEN_HALLWAY05,
};
use crate::test_support::require_db_or_skip;

/// Kill the Hallway04 guard: completes 685 and accepts 686.
const CHAIN_HALLWAY04_KILL: i32 = 1091;

/// The real sequence, not a hand-picked context. Kill the Hallway04 guard
/// with 685 active, apply what chain 1091 resolves to the mission state,
/// then cross into Hallway05. The bark must fire.
///
/// Reverting the seed to the old `686 not_active` gate fails this test:
/// 1091 has made 686 active before the player reaches Region5.
#[tokio::test]
async fn live_db_chain_1178_barks_in_the_state_the_hallway04_kill_leaves_behind() {
    let pool = require_db_or_skip!();
    let kill_engine = engine_with(load(&pool, CHAIN_HALLWAY04_KILL).await);
    let bark_engine = engine_with(load(&pool, CHAIN_HALLWAY05).await);

    // Step 1: the Hallway04 guard dies while 685 is active.
    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "entity_tag".to_string(),
        serde_json::json!("Hallway04_Guard"),
    );
    ctx.set_param(
        "mission_685_status".to_string(),
        serde_json::json!("active"),
    );
    let event = TriggerEvent {
        trigger_type: TriggerType::EntityDeath,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    let killed = kill_engine.resolve_event(&event, &ctx);

    // Step 2: carry chain 1091's mission mutations into the next context.
    let mut state = vec![
        ("mission_685_status", "active"),
        ("mission_686_status", "not_active"),
    ];
    for (_, action) in &killed.actions {
        match action {
            Action::CompleteMission { mission_id: 685 } => state[0].1 = "completed",
            Action::AcceptMission { mission_id: 686 } => state[1].1 = "active",
            _ => {}
        }
    }
    assert_eq!(
        state,
        [
            ("mission_685_status", "completed"),
            ("mission_686_status", "active")
        ],
        "chain 1091 must complete 685 and accept 686 on the Hallway04 kill; \
         that is the state every player reaches Region5 in. Resolved: {:?}",
        killed.actions,
    );

    // Step 3: cross into Hallway05 in that state (687 not yet accepted).
    let mut params = state.clone();
    params.push(("mission_687_status", "not_active"));
    let resolved = resolve_region_enter(&bark_engine, REGION_HALLWAY05, &params);
    assert_single_bark(&resolved, CHAIN_HALLWAY05 as i64, SCREEN_HALLWAY05);
}

/// Adjacent wrong state — phase not yet reached. Hallway04 is still being
/// fought, so 686 has not been accepted.
#[tokio::test]
async fn live_db_chain_1178_does_not_fire_before_686_is_accepted() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_HALLWAY05).await);

    let resolved = resolve_region_enter(
        &engine,
        REGION_HALLWAY05,
        &[
            ("mission_686_status", "not_active"),
            ("mission_687_status", "not_active"),
        ],
    );
    assert_refused(
        &resolved,
        CHAIN_HALLWAY05 as i64,
        "mission 686 (Hallway05) has not been accepted yet",
    );
}

/// Adjacent wrong state — phase already passed. Chain 1094 completes 686
/// and accepts 687 when the Hallway05 guards die, so walking back through
/// the cleared room must stay silent.
#[tokio::test]
async fn live_db_chain_1178_does_not_re_bark_once_hallway05_is_cleared() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_HALLWAY05).await);

    let resolved = resolve_region_enter(
        &engine,
        REGION_HALLWAY05,
        &[
            ("mission_686_status", "completed"),
            ("mission_687_status", "active"),
        ],
    );
    assert_refused(
        &resolved,
        CHAIN_HALLWAY05 as i64,
        "Hallway05 is cleared (686 completed, 687 accepted by chain 1094)",
    );
}

/// The gate must not depend on a chain that does not run in a normal
/// playthrough. Registering chain 1083 beside 1178 in the real state
/// (686 already active) must still bark, and 1083 must not accept 686
/// a second time.
#[tokio::test]
async fn live_db_chain_1178_does_not_depend_on_the_region5_fallback_accept() {
    let pool = require_db_or_skip!();
    let engine = {
        let mut e = ChainEngine::new();
        e.register_chain(load(&pool, 1083).await);
        e.register_chain(load(&pool, CHAIN_HALLWAY05).await);
        e
    };

    let resolved = resolve_region_enter(
        &engine,
        REGION_HALLWAY05,
        &[
            ("mission_685_status", "completed"),
            ("mission_686_status", "active"),
            ("mission_687_status", "not_active"),
        ],
    );
    assert_single_bark(&resolved, CHAIN_HALLWAY05 as i64, SCREEN_HALLWAY05);
    assert!(
        !resolved
            .actions
            .iter()
            .any(|(id, a)| *id == 1083 && matches!(a, Action::AcceptMission { .. })),
        "chain 1083 is the fallback accept and must stay shut when 686 is \
         already active. Resolved: {:?}",
        resolved.actions,
    );
}
