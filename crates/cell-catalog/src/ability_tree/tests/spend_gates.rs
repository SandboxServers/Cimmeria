//! The spend gates (`gates/spend.rs`) through `evaluate_train`.
//!
//! Fixture: a Soldier-shaped branch pair. Branch 0 has a root (no gate,
//! cost 1) and a level-50 capstone gated on 20 archetype-wide points whose
//! prerequisite is the root. Branch 1 has a tier-1 node gated on 2 points
//! and a signature root (cost 2) that content grants for free.
//!
//! Branch credit (Class Start v6 CS-01a, OD-CS06): the gate compares
//! `tree_points_spent` plus the `skill_point_cost` of every tree node held
//! through a non-`gm` grant provenance (`credited_grants`).

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
/// A branch-1 root a class gets free as its signature (cost 2).
const SIGNATURE: i32 = 712;
/// A credited grant that is not a node of the archetype's tree.
const OFF_TREE_GRANT: i32 = 1218;

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
    let mut signature = TreeNode::with_defaults(ARCH, 1, SIGNATURE, 1, vec![]);
    signature.is_branch_root = true;
    signature.skill_point_cost = 2;
    AbilityTreeCatalog::from_nodes([root, capstone, tier1, needs_starter, signature])
}

fn ctx<'a>(
    catalog: &'a AbilityTreeCatalog,
    known: &'a HashSet<i32>,
    ability_id: i32,
    level: i32,
    spent: i32,
    points: i32,
) -> TrainContext<'a> {
    credited_ctx(catalog, known, &[], ability_id, level, spent, points)
}

/// [`ctx`] with `credited_grants`, the non-`gm` grant provenance.
fn credited_ctx<'a>(
    catalog: &'a AbilityTreeCatalog,
    known: &'a HashSet<i32>,
    credited: &'a [i32],
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
        credited_grants: credited,
        training_points: points,
        // At a reachable trainer offering every node, so only the spend
        // gates can reject.
        trainer: TrainerPin::Trainer {
            offered: &[ROOT, CAPSTONE, OTHER_BRANCH_TIER1, NEEDS_STARTER, SIGNATURE],
            in_range: true,
        },
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
    // ...but knowing it does not open a spend gate: a starter has no
    // provenance row, so with nothing bought the spend is 0 and the same
    // node is locked.
    assert_eq!(
        evaluate_train(&ctx(&cat, &known, NEEDS_STARTER, 5, 0, 1)),
        Err(TrainReject::SpendGate {
            required: 2,
            spent: 0
        })
    );
}

/// **Guard (CS-01a): a granted signature root is branch credit.** Nothing
/// was bought, but the free signature (cost 2) opens the 2-point tier-1
/// node. Revert the credit term in `spend::branch_points` and this is
/// `SpendGate { required: 2, spent: 0 }`.
#[test]
fn a_granted_signature_root_counts_as_branch_credit() {
    let cat = catalog();
    let known = HashSet::from([SIGNATURE]);
    let plan = evaluate_train(&credited_ctx(
        &cat,
        &known,
        &[SIGNATURE],
        OTHER_BRANCH_TIER1,
        5,
        0,
        1,
    ))
    .expect("the signature's credit opens the tier-1 node");
    assert_eq!(plan.ability_id, OTHER_BRANCH_TIER1);
}

/// Trained points and credit add up: 1 bought + 2 credited opens nothing
/// gated on 20, and the rejection reports the effective spend, 3.
#[test]
fn trained_spend_and_grant_credit_add_up() {
    let cat = catalog();
    let known = HashSet::from([ROOT, SIGNATURE]);
    assert_eq!(
        evaluate_train(&credited_ctx(
            &cat,
            &known,
            &[SIGNATURE],
            CAPSTONE,
            50,
            1,
            10
        )),
        Err(TrainReject::SpendGate {
            required: 20,
            spent: 3
        })
    );
}

/// Trained-only spend still opens the node with no grants at all.
#[test]
fn trained_only_spend_still_opens_the_node() {
    let cat = catalog();
    let known = HashSet::new();
    assert!(evaluate_train(&credited_ctx(
        &cat,
        &known,
        &[],
        OTHER_BRANCH_TIER1,
        5,
        2,
        1
    ))
    .is_ok());
}

/// A GM grant never reaches `credited_grants` (the base and the world-entry
/// read filter `source_kind <> 'gm'`), so knowing the signature through a
/// GM grant gives no credit: the same node stays locked.
#[test]
fn a_gm_granted_signature_gives_no_credit() {
    let cat = catalog();
    let known = HashSet::from([SIGNATURE]);
    assert_eq!(
        evaluate_train(&credited_ctx(
            &cat,
            &known,
            &[],
            OTHER_BRANCH_TIER1,
            5,
            0,
            1
        )),
        Err(TrainReject::SpendGate {
            required: 2,
            spent: 0
        })
    );
}

/// A credited grant outside the archetype's tree, and a duplicated id, add
/// nothing beyond the one node's cost.
#[test]
fn credit_counts_only_tree_nodes_and_each_once() {
    let cat = catalog();
    assert_eq!(grant_credit(&cat, ARCH, &[OFF_TREE_GRANT]), 0);
    assert_eq!(grant_credit(&cat, ARCH, &[SIGNATURE, SIGNATURE]), 2);
    // Another archetype's tree has no such node.
    assert_eq!(grant_credit(&cat, ARCH + 1, &[SIGNATURE]), 0);
}

/// A granted node is known, so it can never be bought again: the trainer
/// shows it as known, and a purchase is refused before any gate.
#[test]
fn a_granted_node_cannot_be_repurchased() {
    let cat = catalog();
    let known = HashSet::from([SIGNATURE]);
    assert_eq!(
        evaluate_train(&credited_ctx(
            &cat,
            &known,
            &[SIGNATURE],
            SIGNATURE,
            5,
            0,
            10
        )),
        Err(TrainReject::AlreadyKnown)
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
