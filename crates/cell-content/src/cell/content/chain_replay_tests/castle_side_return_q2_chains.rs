//! Castle return_q2 side story (mission 10321), CS-04c2c1: the seed shape of
//! chains 10140-10146 as `castle_side_stories_chains.sql` loads them.
//!
//! Every chain is compared whole against the sealed trigger, conditions and
//! action list. The behaviour of the same chains through the real dispatcher
//! is in `castle_side_return_q2.rs`.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::conditions::{
    ComparisonOp, Condition, MissionStatusValue, StepStatusValue,
};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

const MISSION: i32 = 10321;
const PREREQ: i32 = 10320;
const STEP_A: i32 = 10378;
const STEP_B: i32 = 10379;
const OBJ_B: i32 = 10499;
/// The Moh'katan offer: dialog-set map 1000014 bound at slot 54.
const OFFER_SET: i32 = 1000014;
const OFFER_SLOT: i32 = 54;
const DIALOG: i32 = 10725;

// ── Seed shape ───────────────────────────────────────────────────────────

/// Debug text of a `world = 8` condition.
fn world_8() -> String {
    format!(
        "{:?}",
        Condition::World {
            operator: ComparisonOp::Eq,
            world_id: 8
        }
    )
}

/// Debug text of a `mission_status` condition.
fn mission_is(mission_id: i32, expected_status: MissionStatusValue) -> String {
    format!(
        "{:?}",
        Condition::MissionStatus {
            mission_id,
            operator: ComparisonOp::Eq,
            expected_status
        }
    )
}

/// Debug text of a `step_status = active` condition on 10321.
fn step_active(step_id: i32) -> String {
    format!(
        "{:?}",
        Condition::StepStatus {
            mission_id: MISSION,
            step_id,
            operator: ComparisonOp::Eq,
            expected_status: StepStatusValue::Active
        }
    )
}

fn offer_bind() -> Action {
    Action::AddDialogSet {
        dialog_set_id: OFFER_SET,
        slot: OFFER_SLOT,
        mission_id: None,
    }
}

fn offer_unbind() -> Action {
    Action::RemoveDialogSet {
        dialog_set_id: OFFER_SET,
        slot: OFFER_SLOT,
    }
}

/// **Chains 10140-10146 load with the sealed trigger, every condition in
/// order and the exact action list.** Each chain is compared whole: trigger,
/// condition list (type, ids, operator, status), action list (type, ids,
/// slot, step) in sort order, plus enabled, priority 0, not `once` and no
/// delays. Revert proof: delete 10143 and the load fails; drop 10142's
/// `mission_status 10321 not_active` condition or swap 10143's action order
/// and the comparison fails.
#[tokio::test]
async fn live_db_castle_side_return_q2_chains_load() {
    let pool = require_db_or_skip!();
    let offer_gate = || {
        vec![
            world_8(),
            mission_is(PREREQ, MissionStatusValue::Completed),
            mission_is(MISSION, MissionStatusValue::NotActive),
        ]
    };
    let active_at = |step| {
        vec![
            world_8(),
            mission_is(MISSION, MissionStatusValue::Active),
            step_active(step),
        ]
    };
    // (chain, trigger Debug, conditions in order, actions in order)
    let sealed: [(i32, &str, Vec<String>, Vec<Action>); 7] = [
        (
            10140,
            "OnMissionCompleted { mission_id: 10320 }",
            offer_gate(),
            vec![offer_bind()],
        ),
        (
            10141,
            "OnPlayerLoaded { world_name: Some(\"Castle\") }",
            offer_gate(),
            vec![offer_bind()],
        ),
        (
            10142,
            "OnInteractTag { entity_tag: \"Castle_Mohkatan\" }",
            offer_gate(),
            vec![Action::AcceptMission {
                mission_id: MISSION,
            }],
        ),
        (
            10143,
            "OnMissionAccepted { mission_id: 10321 }",
            vec![world_8(), step_active(STEP_A)],
            vec![
                Action::AdvanceStep {
                    mission_id: MISSION,
                    step_id: STEP_B,
                },
                offer_unbind(),
                Action::DisplayDialog { dialog_id: DIALOG },
            ],
        ),
        (
            10144,
            "OnStargateDialed { destination_world: Some(\"Harset\") }",
            active_at(STEP_B),
            vec![Action::CompleteObjective {
                mission_id: MISSION,
                objective_id: OBJ_B,
            }],
        ),
        (
            10145,
            "OnPlayerLoaded { world_name: Some(\"Castle\") }",
            vec![
                world_8(),
                mission_is(PREREQ, MissionStatusValue::Completed),
                mission_is(MISSION, MissionStatusValue::Active),
                step_active(STEP_A),
            ],
            vec![Action::AdvanceStep {
                mission_id: MISSION,
                step_id: STEP_B,
            }],
        ),
        (
            10146,
            "OnMissionAbandoned { mission_id: 10321 }",
            offer_gate(),
            vec![offer_bind()],
        ),
    ];
    for (id, trigger, conditions, actions) in sealed {
        let chain = load_single_chain_for_test(&pool, id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {id} must exist in seeded content_chains"));
        assert!(chain.enabled, "chain {id} must be enabled");
        assert_eq!(chain.priority, 0, "chain {id} priority");
        assert!(!chain.once, "chain {id} must not be once");
        assert_eq!(
            format!("{:?}", chain.trigger),
            trigger,
            "chain {id} trigger"
        );
        let got: Vec<String> = chain.conditions.iter().map(|c| format!("{c:?}")).collect();
        assert_eq!(got, conditions, "chain {id} conditions in sort order");
        assert_eq!(chain.actions, actions, "chain {id} actions in sort order");
        assert!(
            chain.action_delays.iter().all(|&d| d == 0),
            "chain {id} must have no delays"
        );
    }
}
