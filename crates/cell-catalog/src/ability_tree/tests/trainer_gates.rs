//! Trainer authority gates (AT-04): one test per rejection on
//! `evaluate_train`, plus the priority rules that keep a duplicate silent
//! and a node gate ahead of the trainer gates.

use std::collections::HashSet;

use super::super::*;

const ARCH: i32 = 2;
const ABILITY: i32 = 597;
const OFFERED: &[i32] = &[ABILITY, 646];

fn catalog() -> AbilityTreeCatalog {
    AbilityTreeCatalog::from_nodes([TreeNode::with_defaults(ARCH, 0, ABILITY, 1, vec![])])
}

/// A context that passes every gate at a reachable trainer.
fn ctx<'a>(
    catalog: &'a AbilityTreeCatalog,
    known: &'a HashSet<i32>,
    trainer: TrainerPin<'a>,
) -> TrainContext<'a> {
    TrainContext {
        catalog,
        ability_id: ABILITY,
        ability_exists: true,
        player_id: Some(42),
        archetype_id: Some(ARCH),
        level: 1,
        known,
        tree_points_spent: 0,
        training_points: 1,
        trainer,
    }
}

const AT_TRAINER: TrainerPin<'static> = TrainerPin::Trainer {
    offered: OFFERED,
    in_range: true,
};

#[test]
fn reachable_trainer_offering_the_node_passes() {
    let cat = catalog();
    let known = HashSet::new();
    assert!(evaluate_train(&ctx(&cat, &known, AT_TRAINER)).is_ok());
}

#[test]
fn no_pin_is_rejected() {
    let cat = catalog();
    let known = HashSet::new();
    assert_eq!(
        evaluate_train(&ctx(&cat, &known, TrainerPin::Unpinned)),
        Err(TrainReject::NoTrainerPinned)
    );
}

#[test]
fn despawned_target_is_rejected() {
    let cat = catalog();
    let known = HashSet::new();
    assert_eq!(
        evaluate_train(&ctx(&cat, &known, TrainerPin::Despawned)),
        Err(TrainReject::TrainerDespawned)
    );
}

#[test]
fn pin_that_is_not_a_trainer_is_rejected() {
    let cat = catalog();
    let known = HashSet::new();
    assert_eq!(
        evaluate_train(&ctx(&cat, &known, TrainerPin::NotATrainer)),
        Err(TrainReject::PinNotATrainer)
    );
}

#[test]
fn node_the_trainer_does_not_offer_is_rejected() {
    let cat = catalog();
    let known = HashSet::new();
    let pin = TrainerPin::Trainer {
        offered: &[646],
        in_range: true,
    };
    assert_eq!(
        evaluate_train(&ctx(&cat, &known, pin)),
        Err(TrainReject::NotOfferedByTrainer)
    );
}

#[test]
fn out_of_range_trainer_is_rejected() {
    let cat = catalog();
    let known = HashSet::new();
    let pin = TrainerPin::Trainer {
        offered: OFFERED,
        in_range: false,
    };
    assert_eq!(
        evaluate_train(&ctx(&cat, &known, pin)),
        Err(TrainReject::TrainerOutOfRange)
    );
}

/// A replayed purchase from far away must still read as a duplicate, which
/// the caller keeps silent, not as a trainer rejection that sends feedback.
#[test]
fn already_known_outranks_every_trainer_gate() {
    let cat = catalog();
    let known = HashSet::from([ABILITY]);
    assert_eq!(
        evaluate_train(&ctx(&cat, &known, TrainerPin::Unpinned)),
        Err(TrainReject::AlreadyKnown)
    );
}

/// Node gates run before trainer gates, so a player at no trainer who is also
/// under-levelled is told about the level.
#[test]
fn node_gates_outrank_trainer_gates() {
    let cat = catalog();
    let known = HashSet::new();
    let mut c = ctx(&cat, &known, TrainerPin::Unpinned);
    c.level = 0;
    assert_eq!(
        evaluate_train(&c),
        Err(TrainReject::LevelTooLow {
            required: 1,
            actual: 0
        })
    );
}

#[test]
fn trainer_reason_names() {
    assert_eq!(TrainReject::NoTrainerPinned.reason(), "no_trainer_pinned");
    assert_eq!(TrainReject::TrainerDespawned.reason(), "trainer_despawned");
    assert_eq!(TrainReject::PinNotATrainer.reason(), "pin_not_a_trainer");
    assert_eq!(
        TrainReject::NotOfferedByTrainer.reason(),
        "not_offered_by_trainer"
    );
    assert_eq!(
        TrainReject::TrainerOutOfRange.reason(),
        "trainer_out_of_range"
    );
}
