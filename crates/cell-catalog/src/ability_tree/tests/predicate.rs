//! One test per `TrainReject` variant, plus the passing plan.

use std::collections::HashSet;

use super::super::*;

const ARCH: i32 = 2;
const ABILITY: i32 = 597;

fn catalog() -> AbilityTreeCatalog {
    let mut node = TreeNode::with_defaults(ARCH, 1, ABILITY, 5, vec![100, 101]);
    node.skill_point_cost = 3;
    node.raw_training_cost = 7;
    AbilityTreeCatalog::from_nodes([node])
}

/// A context that passes every gate; each test breaks exactly one input.
fn ctx<'a>(catalog: &'a AbilityTreeCatalog, known: &'a HashSet<i32>) -> TrainContext<'a> {
    TrainContext {
        catalog,
        ability_id: ABILITY,
        ability_exists: true,
        player_id: Some(42),
        archetype_id: Some(ARCH),
        level: 5,
        known,
        tree_points_spent: 0,
        training_points: 3,
    }
}

fn prereqs_known() -> HashSet<i32> {
    HashSet::from([100, 101])
}

#[test]
fn passing_node_plans_tree_index_and_cost() {
    let cat = catalog();
    let known = prereqs_known();
    assert_eq!(
        evaluate_train(&ctx(&cat, &known)),
        Ok(TrainPlan {
            player_id: 42,
            archetype_id: ARCH,
            ability_id: ABILITY,
            tree_index: 1,
            cost: 3,
            raw_training_cost: 7,
        })
    );
}

#[test]
fn unknown_ability() {
    let cat = catalog();
    let known = prereqs_known();
    let mut c = ctx(&cat, &known);
    c.ability_exists = false;
    assert_eq!(evaluate_train(&c), Err(TrainReject::UnknownAbility));
}

#[test]
fn no_player_id() {
    let cat = catalog();
    let known = prereqs_known();
    let mut c = ctx(&cat, &known);
    c.player_id = None;
    assert_eq!(evaluate_train(&c), Err(TrainReject::NoPlayerId));
}

#[test]
fn already_known_is_its_own_variant() {
    let cat = catalog();
    let known = HashSet::from([100, 101, ABILITY]);
    assert_eq!(
        evaluate_train(&ctx(&cat, &known)),
        Err(TrainReject::AlreadyKnown)
    );
}

#[test]
fn no_archetype() {
    let cat = catalog();
    let known = prereqs_known();
    let mut c = ctx(&cat, &known);
    c.archetype_id = None;
    assert_eq!(evaluate_train(&c), Err(TrainReject::NoArchetype));
}

#[test]
fn not_in_archetype_tree() {
    let cat = catalog();
    let known = prereqs_known();
    let mut c = ctx(&cat, &known);
    c.archetype_id = Some(ARCH + 1);
    assert_eq!(evaluate_train(&c), Err(TrainReject::NotInArchetypeTree));
}

#[test]
fn level_too_low() {
    let cat = catalog();
    let known = prereqs_known();
    let mut c = ctx(&cat, &known);
    c.level = 4;
    assert_eq!(
        evaluate_train(&c),
        Err(TrainReject::LevelTooLow {
            required: 5,
            actual: 4
        })
    );
}

#[test]
fn missing_prerequisite_reports_first_missing() {
    let cat = catalog();
    let known = HashSet::from([100]);
    assert_eq!(
        evaluate_train(&ctx(&cat, &known)),
        Err(TrainReject::MissingPrerequisite { missing: 101 })
    );
}

#[test]
fn reason_names_keep_the_logged_values() {
    // `train.rs` has always logged these three `reason=` values.
    assert_eq!(
        TrainReject::NotInArchetypeTree.reason(),
        "not_in_archetype_tree"
    );
    assert_eq!(
        TrainReject::LevelTooLow {
            required: 0,
            actual: 0
        }
        .reason(),
        "level_too_low"
    );
    assert_eq!(
        TrainReject::MissingPrerequisite { missing: 0 }.reason(),
        "missing_prerequisite"
    );
}
