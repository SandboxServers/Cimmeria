//! Mission 568 chains 9011-9018: the three food-unit props (Core, Nest,
//! Fungus). Each prop carries all of the mission's progression: the Core
//! advances to 1966 and binds the turn-in, the Nest credits optional 2312,
//! the Fungus advances to 1965. Binds are per player and the props carry
//! none of their own (template 421 is `INT_MissionWorldObject`).
//!
//! Each prop is disjoint by state: exactly one of its chains fires in every
//! mission state, so the walk below is exhaustive.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::triggers::{Trigger, TriggerType};

use super::super::super::engine_loader::build_engine;
use super::{ctx, fired, load, with_tag, ALL, TOLLANA};
use crate::test_support::require_db_or_skip;

/// Whether `trigger` is the F5 trigger for `chain_id`.
fn trigger_is(trigger: &Trigger, chain_id: i32) -> bool {
    let want = match chain_id {
        9011..=9013 => "Tollana_FoodUnit_Core",
        9014..=9016 => "Tollana_FoodUnit_Nest",
        9017..=9018 => "Tollana_FoodUnit_Fungus",
        _ => return false,
    };
    matches!(trigger, Trigger::OnInteractTag { entity_tag } if entity_tag == want)
}

#[tokio::test]
async fn live_db_568_chains_9011_to_9018_load_with_exact_shapes() {
    let pool = require_db_or_skip!();
    let slot = 420;
    let expected: [(i32, usize, Vec<Action>); 8] = [
        (
            9011,
            1,
            vec![
                Action::AdvanceStep {
                    mission_id: 568,
                    step_id: 1965,
                },
                Action::AdvanceStep {
                    mission_id: 568,
                    step_id: 1966,
                },
                Action::RemoveDialogSet {
                    dialog_set_id: 4959,
                    slot,
                },
                Action::AddDialogSet {
                    dialog_set_id: 6393,
                    slot,
                    mission_id: Some(568),
                },
                Action::DisplayDialog { dialog_id: 4211 },
            ],
        ),
        (
            9012,
            1,
            vec![
                Action::AdvanceStep {
                    mission_id: 568,
                    step_id: 1966,
                },
                Action::RemoveDialogSet {
                    dialog_set_id: 4959,
                    slot,
                },
                Action::AddDialogSet {
                    dialog_set_id: 6393,
                    slot,
                    mission_id: Some(568),
                },
                Action::DisplayDialog { dialog_id: 4211 },
            ],
        ),
        (9013, 2, vec![Action::DisplayDialog { dialog_id: 4211 }]),
        (
            9014,
            1,
            vec![
                Action::AdvanceStep {
                    mission_id: 568,
                    step_id: 1965,
                },
                Action::CompleteObjective {
                    mission_id: 568,
                    objective_id: 2312,
                },
                Action::DisplayDialog { dialog_id: 4213 },
            ],
        ),
        (
            9015,
            1,
            vec![
                Action::CompleteObjective {
                    mission_id: 568,
                    objective_id: 2312,
                },
                Action::DisplayDialog { dialog_id: 4213 },
            ],
        ),
        (9016, 2, vec![Action::DisplayDialog { dialog_id: 4213 }]),
        (
            9017,
            1,
            vec![
                Action::AdvanceStep {
                    mission_id: 568,
                    step_id: 1965,
                },
                Action::DisplayDialog { dialog_id: 4214 },
            ],
        ),
        (9018, 1, vec![Action::DisplayDialog { dialog_id: 4214 }]),
    ];
    for (id, cond, actions) in expected {
        let chain = load(&pool, id).await;
        assert!(chain.enabled, "chain {id} must be enabled");
        assert_eq!(chain.conditions.len(), cond, "chain {id} condition count");
        assert_eq!(chain.actions, actions, "chain {id} action list");
        assert!(
            trigger_is(&chain.trigger, id),
            "chain {id} trigger {:?}",
            chain.trigger
        );
    }
}

#[tokio::test]
async fn live_db_568_props_are_disjoint_per_state() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // Fired chain per state, in `ALL` order: S0, S1964, S1965, S1966, Done.
    let props: [(&str, [i64; 5]); 3] = [
        ("Tollana_FoodUnit_Core", [9013, 9011, 9012, 9013, 9013]),
        ("Tollana_FoodUnit_Nest", [9016, 9014, 9015, 9016, 9016]),
        ("Tollana_FoodUnit_Fungus", [9018, 9017, 9018, 9018, 9018]),
    ];
    for (tag, want) in props {
        for (s, want) in ALL.into_iter().zip(want) {
            let got = fired(
                &engine,
                TriggerType::InteractTag,
                &with_tag(ctx(TOLLANA, s), tag),
            )
            .await;
            assert_eq!(got, vec![want], "{tag} in {s:?}");
        }
    }
}
