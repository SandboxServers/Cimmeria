//! Region-transition accept-mission chains for missions 682/684/686/687
//! (`castle_cellblock_chains.sql` chains 1081-1084) — full-seed
//! regression guard for audit.md defect B1's remaining shape.
//!
//! The purged auto-export (`space_castle_cellblock_chains.sql`, deleted
//! per decision D-CB02) carried a duplicate accept_mission chain for
//! each of these region edges, plus a second, further-duplicated chain
//! with the same condition repeated 2-3 times (issuing `accept_mission`
//! once per repeated condition row — see
//! `.github/instructions/content-chains.instructions.md`'s "Duplicate
//! conditions" note on chain 5005). Loaded through [`build_engine`] (the
//! same assembly path the live server uses) so a duplicate chain
//! anywhere in the DB shows up in the resolved action count, unlike the
//! single-chain loader used elsewhere in this test suite.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// Fire the given region trigger with the two mission-status params that
/// gate the curated accept chain, and assert exactly one
/// `AcceptMission(expected_mission_id)` resolves.
///
/// Before the purge, each of these edges matched its curated chain
/// (1081/1082/1083/1084) plus one or two auto-exported duplicates,
/// resolving 2-5 `accept_mission` actions instead of 1. This fails on
/// the pre-purge tree and passes once `space_castle_cellblock_chains.sql`
/// is deleted.
async fn assert_region_transition_resolves_exactly_one_accept(
    trigger_type: TriggerType,
    region_key: &str,
    gate_mission_id: i32,
    target_mission_id: i32,
) {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let mut ctx = ExecutionContext::new();
    ctx.set_param("region_key".to_string(), serde_json::json!(region_key));
    ctx.set_param(
        format!("mission_{gate_mission_id}_status"),
        serde_json::json!("completed"),
    );
    ctx.set_param(
        format!("mission_{target_mission_id}_status"),
        serde_json::json!("not_active"),
    );

    let event = TriggerEvent {
        trigger_type: trigger_type.clone(),
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };

    let resolved = engine.resolve_event(&event, &ctx);
    let accepts = resolved
        .actions
        .iter()
        .filter(|(_, action)| {
            matches!(
                action,
                Action::AcceptMission { mission_id } if *mission_id == target_mission_id
            )
        })
        .count();
    assert_eq!(
        accepts, 1,
        "region_key={region_key} trigger={trigger_type:?} must resolve \
         exactly one AcceptMission({target_mission_id}); got {accepts} — \
         duplicate chains from the purged auto-export are still in the \
         seed. Actions: {:?}",
        resolved.actions,
    );
}

/// Chain 1081: exiting Region3 with 681 completed accepts 682 exactly
/// once. The auto-export's chain 5023 duplicated the exit trigger; its
/// sibling chain 5022 (enter, not exit) additionally triple-fired on
/// Region3 *entry*, which this exit-scoped test does not exercise
/// (exercised implicitly by the exit-vs-enter event-type split — 5022
/// never matched an exit_region event).
#[tokio::test]
async fn region3_exit_resolves_exactly_one_accept_682() {
    assert_region_transition_resolves_exactly_one_accept(
        TriggerType::RegionExit,
        "Castle_Cellblock.Region3",
        681,
        682,
    )
    .await;
}

/// Chain 1082: entering Region4 with 683 completed accepts 684 exactly
/// once.
#[tokio::test]
async fn region4_enter_resolves_exactly_one_accept_684() {
    assert_region_transition_resolves_exactly_one_accept(
        TriggerType::RegionEnter,
        "Castle_Cellblock.Region4",
        683,
        684,
    )
    .await;
}

/// Chain 1083: entering Region5 with 685 completed accepts 686 exactly
/// once.
#[tokio::test]
async fn region5_enter_resolves_exactly_one_accept_686() {
    assert_region_transition_resolves_exactly_one_accept(
        TriggerType::RegionEnter,
        "Castle_Cellblock.Region5",
        685,
        686,
    )
    .await;
}

/// Chain 1084: entering Region6 with 686 completed accepts 687 exactly
/// once.
#[tokio::test]
async fn region6_enter_resolves_exactly_one_accept_687() {
    assert_region_transition_resolves_exactly_one_accept(
        TriggerType::RegionEnter,
        "Castle_Cellblock.Region6",
        686,
        687,
    )
    .await;
}
