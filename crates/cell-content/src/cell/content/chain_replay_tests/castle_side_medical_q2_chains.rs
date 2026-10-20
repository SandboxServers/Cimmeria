//! Castle medical side story Q2 (mission 10301), CS-04a2c1: the seeded shape
//! of chains 10010-10017 in `castle_side_stories_chains.sql`.
//!
//! One test compares every chain whole against the sealed rows (trigger,
//! conditions, actions, in sort order). The replay tests that execute the
//! chains through the real dispatcher live in `castle_side_medical_q2.rs`.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::conditions::{
    ComparisonOp, Condition, MissionStatusValue, StepStatusValue,
};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

const MISSION: i32 = 10301;
/// The Q1 medical story, the prerequisite the offer gates on.
const Q1: i32 = 10300;
const STEP_A: i32 = 10342;
const STEP_B: i32 = 10343;
const OBJ_B: i32 = 10463;
/// The prisoner offer: dialog-set map 1000002 bound at slot 417.
const OFFER_SET: i32 = 1000002;
const OFFER_SLOT: i32 = 417;
/// The medic turn-in: dialog-set map 1000003 bound at slot 177.
const TURN_IN_SET: i32 = 1000003;
const TURN_IN_SLOT: i32 = 177;
const ACCEPT_DIALOG: i32 = 10701;
const TURN_IN_DIALOG: i32 = 10702;

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

/// Debug text of a `mission_status = expected` condition.
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

/// Debug text of a `step_status = active` condition on 10301.
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

fn turn_in_bind() -> Action {
    Action::AddDialogSet {
        dialog_set_id: TURN_IN_SET,
        slot: TURN_IN_SLOT,
        mission_id: None,
    }
}

/// **Chains 10010-10017 load with the sealed trigger, every condition in
/// order and the exact action list.** Each chain is compared whole: trigger,
/// condition list (type, ids, operator, status), action list (type, ids,
/// slot, step) in sort order, plus enabled, priority 0, not `once` and no
/// delays. Revert proof: delete 10014 and the load fails; drop 10015's
/// `mission_status 10300 completed` or swap 10014's action order and the
/// comparison fails.
#[tokio::test]
async fn live_db_castle_side_medical_q2_chains_load() {
    let pool = require_db_or_skip!();
    let offer_gate = || {
        vec![
            world_8(),
            mission_is(Q1, MissionStatusValue::Completed),
            mission_is(MISSION, MissionStatusValue::NotActive),
        ]
    };
    let turn_in_gate = |step| {
        vec![
            world_8(),
            mission_is(Q1, MissionStatusValue::Completed),
            mission_is(MISSION, MissionStatusValue::Active),
            step_active(step),
        ]
    };
    // (chain, trigger Debug, conditions in order, actions in order)
    let sealed: [(i32, &str, Vec<String>, Vec<Action>); 8] = [
        (
            10010,
            "OnMissionCompleted { mission_id: 10300 }",
            offer_gate(),
            vec![offer_bind()],
        ),
        (
            10011,
            "OnPlayerLoaded { world_name: Some(\"Castle\") }",
            offer_gate(),
            vec![offer_bind()],
        ),
        (
            10012,
            "OnMissionAbandoned { mission_id: 10301 }",
            offer_gate(),
            vec![
                offer_bind(),
                Action::RemoveDialogSet {
                    dialog_set_id: TURN_IN_SET,
                    slot: TURN_IN_SLOT,
                },
            ],
        ),
        (
            10013,
            "OnInteractTag { entity_tag: \"Castle_Pop_Prisoner5\" }",
            offer_gate(),
            vec![Action::AcceptMission {
                mission_id: MISSION,
            }],
        ),
        (
            10014,
            "OnMissionAccepted { mission_id: 10301 }",
            vec![world_8(), step_active(STEP_A)],
            vec![
                Action::AdvanceStep {
                    mission_id: MISSION,
                    step_id: STEP_B,
                },
                Action::RemoveDialogSet {
                    dialog_set_id: OFFER_SET,
                    slot: OFFER_SLOT,
                },
                turn_in_bind(),
                Action::DisplayDialog {
                    dialog_id: ACCEPT_DIALOG,
                },
            ],
        ),
        (
            10015,
            "OnInteractTag { entity_tag: \"Castle_Pop_Infirmary_Medic1\" }",
            turn_in_gate(STEP_B),
            vec![
                Action::RemoveDialogSet {
                    dialog_set_id: TURN_IN_SET,
                    slot: TURN_IN_SLOT,
                },
                Action::DisplayDialog {
                    dialog_id: TURN_IN_DIALOG,
                },
                Action::CompleteObjective {
                    mission_id: MISSION,
                    objective_id: OBJ_B,
                },
            ],
        ),
        (
            10016,
            "OnPlayerLoaded { world_name: Some(\"Castle\") }",
            turn_in_gate(STEP_A),
            vec![
                Action::AdvanceStep {
                    mission_id: MISSION,
                    step_id: STEP_B,
                },
                turn_in_bind(),
            ],
        ),
        (
            10017,
            "OnPlayerLoaded { world_name: Some(\"Castle\") }",
            turn_in_gate(STEP_B),
            vec![turn_in_bind()],
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
