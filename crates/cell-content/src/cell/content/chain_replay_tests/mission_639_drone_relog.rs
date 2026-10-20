//! Mission 639 relog re-arm of the prisoner-retrieval drone (chain 1037).
//!
//! Chain 1032 arms `ArmYourself_PrisonerRetrievalUnit` (seeded NEUTRAL)
//! when the player takes the Ambernol vial. A relog builds a fresh
//! Castle_CellBlock instance with the drone NEUTRAL again, so chain 1037
//! re-arms it on `player_loaded` while step 2144 is active and kill
//! objective 2482 is open. Like chain 1009 it adds no threat.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

#[tokio::test]
async fn live_db_chain_1037_rearms_drone_only_while_kill_open_at_step_2144() {
    let pool = require_db_or_skip!();
    let mut engine = ChainEngine::new();
    engine.register_chain(
        load_single_chain_for_test(&pool, 1037)
            .await
            .expect("DB query for chain 1037 must succeed")
            .expect("drone re-arm chain 1037 must exist"),
    );

    for (params, expect_rearm) in [
        (
            vec![
                ("mission_639_step_2144_status", "active"),
                ("mission_639_obj_2482_status", "active"),
            ],
            true,
        ),
        (
            vec![
                ("mission_639_step_2144_status", "active"),
                ("mission_639_obj_2482_status", "completed"),
            ],
            false,
        ),
        (vec![("mission_639_step_2145_status", "active")], false),
        (
            vec![
                ("mission_639_step_2144_status", "completed"),
                ("mission_639_step_2343_status", "active"),
            ],
            false,
        ),
    ] {
        let mut ctx = ExecutionContext::new();
        ctx.set_param(
            "world_name".to_string(),
            serde_json::json!("Castle_CellBlock"),
        );
        for (key, value) in &params {
            ctx.set_param(key.to_string(), serde_json::json!(value));
        }
        let event = TriggerEvent {
            trigger_type: TriggerType::PlayerLoaded,
            source_entity: None,
            target_entity: None,
            params: ctx.params.clone(),
        };
        let resolved = engine.resolve_event(&event, &ctx);
        let actions: Vec<&Action> = resolved
            .actions
            .iter()
            .filter_map(|(id, a)| if *id == 1037 { Some(a) } else { None })
            .collect();

        if expect_rearm {
            assert!(
                matches!(
                    actions.as_slice(),
                    [Action::SetAggression { entity_tag, level: 1 }]
                        if entity_tag == "ArmYourself_PrisonerRetrievalUnit"
                ),
                "{params:?}: expected exactly SetAggression(drone, 1); got {actions:?}"
            );
        } else {
            assert!(
                actions.is_empty(),
                "{params:?}: chain 1037 must not fire; got {actions:?}"
            );
        }
    }
}
