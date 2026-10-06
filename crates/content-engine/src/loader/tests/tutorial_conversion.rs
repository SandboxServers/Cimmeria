//! One-time tutorials (Class Start v6, CS-03): the `show_tutorial` row, the
//! `tutorial_shown` condition row, the `player_entered_combat` trigger row,
//! and the load-time refusal of a chain that names a non-tutorial dialog.
//!
//! Bug shapes: a malformed `show_tutorial` that drops only itself and runs
//! the rest of the milestone chain; a malformed `tutorial_shown` dropped
//! from its chain, which publishes the chain ungated and replays a
//! tutorial; and an id that is not a tutorial dialog loading anyway. No
//! database.

use std::collections::HashSet;

use super::super::*;
use crate::actions::Action;
use crate::conditions::{ComparisonOp, Condition};
use crate::triggers::Trigger;

const CHAIN: i32 = 7_311;

fn chain_row(chain_id: i32) -> DbChainRow {
    DbChainRow {
        chain_id,
        description: Some("CS-03 tutorial fixture".to_string()),
        scope_type: "global".to_string(),
        scope_id: None,
        enabled: true,
        priority: 0,
    }
}

fn action(action_type: &str, params: serde_json::Value, sort: i32) -> DbActionRow {
    DbActionRow {
        chain_id: CHAIN,
        action_type: action_type.to_string(),
        target_id: None,
        target_key: None,
        params,
        delay_ms: 0,
        sort_order: sort,
    }
}

fn condition(target_id: Option<i32>, operator: &str) -> DbConditionRow {
    DbConditionRow {
        chain_id: CHAIN,
        condition_type: "tutorial_shown".to_string(),
        target_id,
        target_key: None,
        operator: operator.to_string(),
        value: None,
        sort_order: 0,
    }
}

fn trigger(event_type: &str, event_key: Option<&str>) -> DbTriggerRow {
    DbTriggerRow {
        chain_id: CHAIN,
        event_type: event_type.to_string(),
        event_key: event_key.map(str::to_string),
        scope: "player".to_string(),
        once: false,
        sort_order: 0,
    }
}

/// A `show_tutorial` row plus a following `grant_xp`.
fn build(params: serde_json::Value) -> Vec<crate::chain::Chain> {
    build_chains_from_rows(
        vec![chain_row(CHAIN)],
        vec![],
        vec![],
        vec![
            action("show_tutorial", params, 0),
            action("grant_xp", serde_json::json!({"amount": 10}), 1),
        ],
    )
}

fn tutorials() -> HashSet<i32> {
    HashSet::from([5863, 5882, 5883])
}

#[test]
fn a_valid_show_tutorial_row_converts() {
    let chains = build(serde_json::json!({"tutorial_id": 5882}));
    assert_eq!(chains.len(), 1);
    assert!(matches!(
        chains[0].actions[0],
        Action::ShowTutorial { tutorial_id: 5882 }
    ));
    assert_eq!(chains[0].actions.len(), 2);
}

/// **Guard: a malformed `show_tutorial` refuses its whole chain.** Every
/// shape below must leave no chain; drop the `continue 'chains` and the
/// `grant_xp` beside it loads alone.
#[test]
fn a_malformed_show_tutorial_refuses_the_chain() {
    for params in [
        serde_json::json!({}),
        serde_json::json!({"tutorial_id": "5882"}),
        serde_json::json!({"tutorial_id": 0}),
        serde_json::json!({"tutorial_id": -5}),
        serde_json::json!({"tutorial_id": 5_000_000_000_i64}),
    ] {
        assert!(
            build(params.clone()).is_empty(),
            "{params} must refuse the chain"
        );
    }
    // An id in target_id is the wrong column, not a second spelling.
    let mut bad = action("show_tutorial", serde_json::json!({"tutorial_id": 5882}), 0);
    bad.target_id = Some(5882);
    assert!(build_chains_from_rows(vec![chain_row(CHAIN)], vec![], vec![], vec![bad]).is_empty());
}

#[test]
fn tutorial_shown_converts_eq_and_neq() {
    for (op, expected) in [("eq", ComparisonOp::Eq), ("neq", ComparisonOp::Neq)] {
        let chains = build_chains_from_rows(
            vec![chain_row(CHAIN)],
            vec![],
            vec![condition(Some(5882), op)],
            vec![],
        );
        match &chains[0].conditions[..] {
            [Condition::TutorialShown {
                tutorial_id: 5882,
                operator,
            }] => assert_eq!(*operator, expected),
            other => panic!("expected one TutorialShown, got {other:?}"),
        }
    }
}

/// **Guard: a malformed `tutorial_shown` is kept as a never-matching
/// condition, never dropped.** Dropping it publishes the chain ungated, and
/// an ungated `show_tutorial` chain asks for its tutorial on every trigger.
/// A missing id becomes 0, which the tutorial check refuses; an ordered or
/// unknown operator becomes `Gt`, which never matches.
#[test]
fn a_malformed_tutorial_shown_is_kept_and_never_matches() {
    for (target_id, op) in [(None, "eq"), (Some(5882), "gt"), (Some(5882), "bogus")] {
        let chains = build_chains_from_rows(
            vec![chain_row(CHAIN)],
            vec![],
            vec![condition(target_id, op)],
            vec![],
        );
        assert_eq!(chains[0].conditions.len(), 1, "{target_id:?} {op}: kept");
        let ctx = crate::context::ExecutionContext::new().with_shown_tutorials([5882]);
        assert!(
            !chains[0].conditions[0].evaluate(&ctx),
            "{target_id:?} {op}: never matches"
        );
    }
}

#[test]
fn player_entered_combat_takes_no_key() {
    let ok = build_chains_from_rows(
        vec![chain_row(CHAIN)],
        vec![trigger("player_entered_combat", None)],
        vec![],
        vec![],
    );
    assert!(matches!(ok[0].trigger, Trigger::OnPlayerEnteredCombat));
    // A key is an authoring mistake: the trigger row is dropped, and a
    // chain left with no trigger is skipped.
    let keyed = build_chains_from_rows(
        vec![chain_row(CHAIN)],
        vec![trigger("player_entered_combat", Some("Castle"))],
        vec![],
        vec![],
    );
    assert!(keyed.is_empty());
}

/// **Guard: a chain naming a dialog that is not a tutorial is refused**,
/// whether the id is in a `show_tutorial` action or a `tutorial_shown`
/// condition. 2298 is a seeded Blurb, 9_999_999 no dialog at all. Every
/// expansion of a multi-trigger chain goes, and other chains stay.
#[test]
fn unknown_or_non_tutorial_ids_refuse_the_chain() {
    const OTHER: i32 = 7_312;
    let known = tutorials();
    for (actions, conditions) in [
        (
            vec![action(
                "show_tutorial",
                serde_json::json!({"tutorial_id": 2298}),
                0,
            )],
            vec![],
        ),
        (
            vec![action(
                "show_tutorial",
                serde_json::json!({"tutorial_id": 9_999_999}),
                0,
            )],
            vec![],
        ),
        (vec![], vec![condition(Some(2298), "eq")]),
        (vec![], vec![condition(None, "eq")]),
    ] {
        let chains = build_chains_from_rows(
            vec![chain_row(CHAIN), chain_row(OTHER)],
            vec![
                trigger("player_entered_combat", None),
                trigger("player_loaded", None),
                DbTriggerRow {
                    chain_id: OTHER,
                    ..trigger("player_loaded", None)
                },
            ],
            conditions,
            actions,
        );
        assert_eq!(
            chains.len(),
            3,
            "fixture: two expansions plus the other chain"
        );
        let kept = refuse_chains_with_unknown_tutorials(chains, Some(&known));
        assert_eq!(
            kept.iter().map(|c| c.id).collect::<Vec<_>>(),
            vec![i64::from(OTHER)],
            "only the chain that names no bad id is kept"
        );
    }
}

#[test]
fn tutorial_ids_that_are_tutorials_load() {
    let chains = build_chains_from_rows(
        vec![chain_row(CHAIN)],
        vec![trigger("player_entered_combat", None)],
        vec![condition(Some(5882), "eq"), condition(Some(5883), "neq")],
        vec![action(
            "show_tutorial",
            serde_json::json!({"tutorial_id": 5883}),
            0,
        )],
    );
    assert_eq!(
        refuse_chains_with_unknown_tutorials(chains, Some(&tutorials())).len(),
        1
    );
}

/// An unreadable dialog table refuses only the chains that name a tutorial.
#[test]
fn an_unreadable_dialog_table_refuses_only_tutorial_chains() {
    const PLAIN: i32 = 7_313;
    let chains = build_chains_from_rows(
        vec![chain_row(CHAIN), chain_row(PLAIN)],
        vec![],
        vec![],
        vec![
            action("show_tutorial", serde_json::json!({"tutorial_id": 5882}), 0),
            DbActionRow {
                chain_id: PLAIN,
                ..action("grant_xp", serde_json::json!({"amount": 10}), 0)
            },
        ],
    );
    let kept = refuse_chains_with_unknown_tutorials(chains, None);
    assert_eq!(
        kept.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![i64::from(PLAIN)]
    );
}
