//! Castle Cellblock wake-up debuff — chains 5000 / 5001, `player_loaded`.
//!
//! Both chains carry `launch_ability 1372` ("Stasis Sickness - Stage 1"),
//! the debuff the player wakes up with in the stasis room. These guards pin
//! the seed → `Action::LaunchAbility` conversion, and pin two seed defects
//! that make the action unreachable in play today so that neither gets
//! mistaken for an executor bug:
//!
//! 1. **Both chains carry mutually-exclusive conditions.** Each has two
//!    `mission_status 622` rows — `eq not_active` *and* `eq completed` —
//!    and `ChainEngine` AND-s conditions. No context satisfies both, so
//!    neither chain ever resolves its actions. This is not specific to
//!    5001: chain 5000 has the identical pair.
//! 2. **Chain 5001 is a duplicate export.** It repeats 5000's action set
//!    with several actions fired twice, including `launch_ability 1372` at
//!    sort 9 and again at sort 10.
//!
//! The conditions are pinned as *currently unsatisfiable* rather than
//! asserted away. When the seed is corrected the pin trips, which is the
//! intended prompt to revisit these guards alongside the fix.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

/// Chain 5000 must convert its `launch_ability` row into
/// `Action::LaunchAbility { ability_id: 1372, entity_tag: None }` — exactly
/// once, and with no tag so the debuff self-targets the waking player.
///
/// A non-`None` `entity_tag` would route the executor through
/// `find_entity_by_tag` and (on a miss) skip the launch entirely, so the
/// tag is pinned, not just the id.
#[tokio::test]
async fn chain_5000_launches_stasis_sickness_on_the_player_itself() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 5000)
        .await
        .expect("DB query for chain 5000 must succeed")
        .expect("chain 5000 must exist in seeded content_chains and convert successfully");

    let launches: Vec<_> = chain
        .actions
        .iter()
        .filter_map(|a| match a {
            Action::LaunchAbility {
                ability_id,
                entity_tag,
            } => Some((*ability_id, entity_tag.clone())),
            _ => None,
        })
        .collect();

    assert_eq!(
        launches.len(),
        1,
        "chain 5000 must launch exactly one ability (the wake-up debuff); \
         actions: {:?}",
        chain.actions,
    );
    assert_eq!(
        launches[0].0, 1372,
        "chain 5000's launch must be ability 1372 (Stasis Sickness - Stage 1)",
    );
    assert_eq!(
        launches[0].1, None,
        "the wake-up debuff must be untagged so it self-targets the waking \
         player -- a tag would send it through NPC lookup instead",
    );
}

/// Chain 5001 is the auto-export duplicate: it fires `launch_ability 1372`
/// twice. Pinned so the duplication is a recorded fact rather than a
/// surprise, and so a de-duplication pass has to come past this test.
#[tokio::test]
async fn chain_5001_duplicates_the_stasis_sickness_launch() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 5001)
        .await
        .expect("DB query for chain 5001 must succeed")
        .expect("chain 5001 must exist in seeded content_chains and convert successfully");

    let launch_count = chain
        .actions
        .iter()
        .filter(|a| {
            matches!(
                a,
                Action::LaunchAbility {
                    ability_id: 1372,
                    entity_tag: None
                }
            )
        })
        .count();

    assert_eq!(
        launch_count, 2,
        "chain 5001 is a duplicate export of 5000 and repeats the \
         launch_ability 1372 action; if this is now 1 the seed was \
         de-duplicated -- revisit this module. Actions: {:?}",
        chain.actions,
    );
}

/// Both wake-up chains are gated on `mission_status 622 = not_active` AND
/// `= completed`. Conditions are AND-ed, so no player state satisfies them
/// and the `launch_ability` action can never resolve in play.
///
/// Fires `player_loaded` under each of the three reachable mission states
/// and asserts nothing resolves. When the seed's condition pair is fixed,
/// this test fails — that is the signal, not a regression.
///
/// First pins that both triggers are `OnPlayerLoaded { world_name: None }`,
/// which `Trigger::matches` accepts unconditionally. Without that, an empty
/// resolve would be ambiguous between "conditions rejected it" and "the
/// trigger never matched in the first place" — two very different fixes.
#[tokio::test]
async fn wakeup_chains_never_resolve_under_any_mission_622_state() {
    let pool = require_db_or_skip!();

    let mut engine = ChainEngine::new();
    for chain_id in [5000, 5001] {
        let chain = load_single_chain_for_test(&pool, chain_id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
            .unwrap_or_else(|| {
                panic!("chain {chain_id} must exist in seeded content_chains and convert")
            });
        assert!(
            matches!(
                chain.trigger,
                cimmeria_content_engine::triggers::Trigger::OnPlayerLoaded { world_name: None }
            ),
            "chain {chain_id}'s trigger must be an unqualified player_loaded \
             (event_key NULL) so the empty resolve below is attributable to \
             the conditions, not to a trigger mismatch. Got: {:?}",
            chain.trigger,
        );
        engine.register_chain(chain);
    }

    for status in ["not_active", "active", "completed"] {
        let mut ctx = ExecutionContext::new();
        ctx.set_param(
            "world_name".to_string(),
            serde_json::json!("Castle_CellBlock"),
        );
        ctx.set_param("mission_622_status".to_string(), serde_json::json!(status));

        let event = TriggerEvent {
            trigger_type: TriggerType::PlayerLoaded,
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };

        let resolved = engine.resolve_event(&event, &ctx);
        assert!(
            resolved.actions.is_empty(),
            "chains 5000/5001 carry mutually-exclusive mission_status 622 \
             conditions (not_active AND completed) and must resolve nothing \
             at mission status '{status}'. If this now resolves, the seed's \
             condition pair was corrected -- update these guards to assert \
             the launch actually fires. Got: {:?}",
            resolved.actions,
        );
    }
}
