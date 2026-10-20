//! Castle defense_q3 side story (mission 10312), CS-04b3c1: the seeded shape
//! of chains 10120-10129 in `castle_side_stories_chains.sql`.
//!
//! One test compares every chain whole against the sealed rows (trigger,
//! conditions, actions, in sort order). A condition row with a blank
//! `target_key` is dropped by the loader and leaves the chain ungated, so a
//! needle check on the debug text cannot see that drift; the whole-list
//! comparison can. The replay tests that execute the chains through the real
//! dispatcher live in `castle_side_defense_q3.rs`.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::conditions::{
    ComparisonOp, Condition, MissionStatusValue, StepStatusValue,
};

use super::super::engine_loader::load_single_chain_for_test;
use crate::test_support::require_db_or_skip;

const MISSION: i32 = 10312;
/// The `defense_q1` mission the offer gates on.
const PREREQ: i32 = 10310;
const STEP: i32 = 10366;
/// The giver offer: dialog-set map 1000010 bound at slot 425.
const OFFER_SET: i32 = 1000010;
const OFFER_SLOT: i32 = 425;
const OFFER_DIALOG: i32 = 10717;
const DONE_DIALOG: i32 = 10718;
const GIVER_TAG: &str = "Castle_DefenseQ3_Giver";
/// Kill tag and objective pairs, chains 10125-10128 in order.
const KILLS: [(&str, i32); 4] = [
    ("Castle_BravoOfficer1", 10486),
    ("Castle_BravoOfficer2", 10487),
    ("Castle_BravoOfficer3", 10488),
    ("Castle_Muelbach", 10489),
];

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

/// Debug text of a `mission_status = status` condition.
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

/// Debug text of a `step_status 10312/10366 = active` condition.
fn step_active() -> String {
    format!(
        "{:?}",
        Condition::StepStatus {
            mission_id: MISSION,
            step_id: STEP,
            operator: ComparisonOp::Eq,
            expected_status: StepStatusValue::Active
        }
    )
}

/// Debug text of an `objective_status 10312/objective_id neq completed`
/// condition, the re-kill gate.
fn objective_not_done(objective_id: i32) -> String {
    format!(
        "{:?}",
        Condition::ObjectiveStatus {
            mission_id: MISSION,
            objective_id,
            operator: ComparisonOp::Neq,
            expected_status: "completed".to_string()
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

/// **Chains 10120-10129 load with the sealed trigger, every condition in
/// order and the exact action list.** Each chain is compared whole: trigger,
/// condition list (type, ids, operator, status, `neq` on the kill gates),
/// action list in sort order, plus enabled, priority 0, not `once` and no
/// delays. Revert proof: blank 10125's `objective_status` `target_key` (the
/// loader drops the row) or swap 10124's two actions and the comparison
/// fails.
#[tokio::test]
async fn live_db_castle_side_defense_q3_chains_load() {
    let pool = require_db_or_skip!();
    let offer_gate = || {
        vec![
            world_8(),
            mission_is(PREREQ, MissionStatusValue::Completed),
            mission_is(MISSION, MissionStatusValue::NotActive),
        ]
    };
    let kill_gate = |objective_id| {
        vec![
            world_8(),
            mission_is(MISSION, MissionStatusValue::Active),
            step_active(),
            objective_not_done(objective_id),
        ]
    };
    // (chain, trigger Debug, conditions in order, actions in order)
    let mut sealed: Vec<(i32, String, Vec<String>, Vec<Action>)> = vec![
        (
            10120,
            "OnMissionCompleted { mission_id: 10310 }".to_string(),
            offer_gate(),
            vec![offer_bind()],
        ),
        (
            10121,
            "OnPlayerLoaded { world_name: Some(\"Castle\") }".to_string(),
            offer_gate(),
            vec![offer_bind()],
        ),
        (
            10122,
            "OnMissionAbandoned { mission_id: 10312 }".to_string(),
            offer_gate(),
            vec![offer_bind()],
        ),
        (
            10123,
            format!("OnInteractTag {{ entity_tag: {GIVER_TAG:?} }}"),
            offer_gate(),
            vec![Action::AcceptMission {
                mission_id: MISSION,
            }],
        ),
        (
            10124,
            "OnMissionAccepted { mission_id: 10312 }".to_string(),
            vec![world_8(), step_active()],
            vec![
                Action::RemoveDialogSet {
                    dialog_set_id: OFFER_SET,
                    slot: OFFER_SLOT,
                },
                Action::DisplayDialog {
                    dialog_id: OFFER_DIALOG,
                },
            ],
        ),
    ];
    for (offset, (tag, objective_id)) in KILLS.iter().enumerate() {
        sealed.push((
            10125 + offset as i32,
            format!("OnEntityDeath {{ entity_type: None, entity_tag: Some({tag:?}) }}"),
            kill_gate(*objective_id),
            vec![Action::CompleteObjective {
                mission_id: MISSION,
                objective_id: *objective_id,
            }],
        ));
    }
    sealed.push((
        10129,
        "OnMissionCompleted { mission_id: 10312 }".to_string(),
        vec![
            world_8(),
            mission_is(MISSION, MissionStatusValue::Completed),
        ],
        vec![Action::DisplayDialog {
            dialog_id: DONE_DIALOG,
        }],
    ));
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
