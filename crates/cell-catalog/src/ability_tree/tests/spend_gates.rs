//! The spend gates (`gates/spend.rs`) through `evaluate_train`.
//!
//! Fixture: a Soldier-shaped branch pair. Branch 0 has a root (no gate,
//! cost 1) and a level-50 capstone gated on 20 archetype-wide points whose
//! prerequisite is the root. Branch 1 has a tier-1 node gated on 2 points.

use std::collections::HashSet;

use super::super::*;

const ARCH: i32 = 1;
const ROOT: i32 = 700;
const CAPSTONE: i32 = 701;
const OTHER_BRANCH_TIER1: i32 = 710;
/// A starter ability (from `char_creation_abilities`), known but never
/// bought: it is in the known set and adds nothing to `tree_points_spent`.
const STARTER: i32 = 1646;
const NEEDS_STARTER: i32 = 711;

fn catalog() -> AbilityTreeCatalog {
    let mut root = TreeNode::with_defaults(ARCH, 0, ROOT, 1, vec![]);
    root.is_branch_root = true;
    let mut capstone = TreeNode::with_defaults(ARCH, 0, CAPSTONE, 50, vec![ROOT]);
    capstone.is_capstone = true;
    capstone.required_branch_points = 20;
    capstone.skill_point_cost = 3;
    let mut tier1 = TreeNode::with_defaults(ARCH, 1, OTHER_BRANCH_TIER1, 5, vec![]);
    tier1.required_branch_points = 2;
    let mut needs_starter = TreeNode::with_defaults(ARCH, 1, NEEDS_STARTER, 5, vec![STARTER]);
    needs_starter.required_branch_points = 2;
    AbilityTreeCatalog::from_nodes([root, capstone, tier1, needs_starter])
}

fn ctx<'a>(
    catalog: &'a AbilityTreeCatalog,
    known: &'a HashSet<i32>,
    ability_id: i32,
    level: i32,
    spent: i32,
    points: i32,
) -> TrainContext<'a> {
    TrainContext {
        catalog,
        ability_id,
        ability_exists: true,
        player_id: Some(42),
        archetype_id: Some(ARCH),
        level,
        known,
        tree_points_spent: spent,
        training_points: points,
    }
}

#[test]
fn capstone_at_level_50_is_rejected_with_too_little_spend() {
    let cat = catalog();
    let known = HashSet::from([ROOT]);
    assert_eq!(
        evaluate_train(&ctx(&cat, &known, CAPSTONE, 50, 19, 10)),
        Err(TrainReject::SpendGate {
            required: 20,
            spent: 19
        })
    );
}

#[test]
fn capstone_at_level_50_is_accepted_with_enough_spend() {
    let cat = catalog();
    let known = HashSet::from([ROOT]);
    let plan = evaluate_train(&ctx(&cat, &known, CAPSTONE, 50, 20, 3)).expect("capstone opens");
    assert_eq!((plan.tree_index, plan.cost), (0, 3));
}

#[test]
fn spend_counts_across_the_archetype_not_per_branch() {
    // Two points were spent in branch 0 (the root plus one more); the
    // tier-1 node in branch 1 opens on them (D-AT01).
    let cat = catalog();
    let known = HashSet::from([ROOT]);
    assert!(evaluate_train(&ctx(&cat, &known, OTHER_BRANCH_TIER1, 5, 2, 1)).is_ok());
}

#[test]
fn starter_ability_satisfies_a_prerequisite_but_is_not_spend() {
    let cat = catalog();
    let known = HashSet::from([STARTER]);
    // The prerequisite is met by the starter alone...
    assert!(evaluate_train(&ctx(&cat, &known, NEEDS_STARTER, 5, 2, 1)).is_ok());
    // ...but knowing it does not open a spend gate: with nothing bought,
    // spend is 0 and the same node is locked.
    assert_eq!(
        evaluate_train(&ctx(&cat, &known, NEEDS_STARTER, 5, 0, 1)),
        Err(TrainReject::SpendGate {
            required: 2,
            spent: 0
        })
    );
}

#[test]
fn not_enough_points_for_the_node_cost() {
    let cat = catalog();
    let known = HashSet::from([ROOT]);
    assert_eq!(
        evaluate_train(&ctx(&cat, &known, CAPSTONE, 50, 20, 2)),
        Err(TrainReject::NotEnoughPoints {
            cost: 3,
            available: 2
        })
    );
}

#[test]
fn spend_gate_ranks_before_points() {
    // Both fail: the structural lock is reported, not the transient one.
    let cat = catalog();
    let known = HashSet::from([ROOT]);
    assert!(matches!(
        evaluate_train(&ctx(&cat, &known, CAPSTONE, 50, 0, 0)),
        Err(TrainReject::SpendGate { .. })
    ));
}

#[test]
fn prerequisite_ranks_before_spend() {
    // The node gates keep their pre-AT-03 priority.
    let cat = catalog();
    let known = HashSet::new();
    assert_eq!(
        evaluate_train(&ctx(&cat, &known, CAPSTONE, 50, 0, 0)),
        Err(TrainReject::MissingPrerequisite { missing: ROOT })
    );
}

#[test]
fn spend_reason_names() {
    assert_eq!(
        TrainReject::SpendGate {
            required: 0,
            spent: 0
        }
        .reason(),
        "spend_gate"
    );
    assert_eq!(
        TrainReject::NotEnoughPoints {
            cost: 0,
            available: 0
        }
        .reason(),
        "not_enough_points"
    );
}
