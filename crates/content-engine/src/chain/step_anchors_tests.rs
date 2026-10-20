//! TG-MIS-03: [`ChainEngine::step_anchors`], the scan behind the
//! `mission.step_anchor` row.

use super::tests::make_chain;
use super::*;
use crate::conditions::{ComparisonOp, StepStatusValue};

const MISSION: i32 = 1360;
/// `resources.worlds.world_id` of Castle and Harset.
const CASTLE: i32 = 8;
const HARSET: i32 = 57;

fn gate(step_id: i32) -> Condition {
    Condition::StepStatus {
        mission_id: MISSION,
        step_id,
        operator: ComparisonOp::Eq,
        expected_status: StepStatusValue::Active,
    }
}

fn gated(id: i64, trigger: Trigger, step: i32, actions: Vec<Action>) -> Chain {
    let mut c = make_chain(id, trigger, actions, 0);
    c.conditions = vec![gate(step)];
    c
}

fn tag(name: &str) -> Trigger {
    Trigger::OnInteractTag {
        entity_tag: name.into(),
    }
}

fn advance() -> Action {
    Action::AdvanceStep {
        mission_id: MISSION,
        step_id: 4039,
    }
}

/// A step's advancers are the enabled chains gated on that step that move
/// its mission forward; their triggers name the anchors. A chain gated on
/// another step, one that only shows a dialog, a disabled one and an
/// `OnMissionStep` chain (never loaded, never dispatched) are not advancers,
/// and a step nothing gates (step 4037's shape) has none.
#[test]
fn step_anchors() {
    let complete = Action::CompleteObjective {
        mission_id: MISSION,
        objective_id: 7001,
    };
    let mut engine = ChainEngine::new();
    engine.register_chain(gated(
        10,
        tag("Frost_Terminal"),
        4038,
        vec![complete.clone()],
    ));
    engine.register_chain(gated(
        11,
        Trigger::OnInteractTemplate {
            template_name: "Colonel Frost".into(),
        },
        4038,
        vec![advance()],
    ));
    engine.register_chain(gated(
        12,
        Trigger::OnDialogChoice { dialog_id: 6110 },
        4038,
        vec![advance()],
    ));
    // The same anchor twice is one anchor.
    engine.register_chain(gated(13, tag("Frost_Terminal"), 4038, vec![advance()]));
    // Gated on the step, but moves nothing.
    engine.register_chain(gated(
        14,
        tag("Chatter"),
        4038,
        vec![Action::DisplayDialog { dialog_id: 1 }],
    ));
    // Moves the mission, but gated on another step.
    engine.register_chain(gated(15, tag("Later"), 4039, vec![advance()]));
    let mut disabled = gated(16, tag("Off"), 4038, vec![advance()]);
    disabled.enabled = false;
    engine.register_chain(disabled);
    // An `OnMissionStep` trigger is not a gate: `loader/trigger.rs` never
    // builds one and nothing dispatches it, so this chain can never fire.
    engine.register_chain(make_chain(
        17,
        Trigger::OnMissionStep {
            mission_id: MISSION,
            step: 4038,
        },
        vec![complete],
        0,
    ));

    let anchors = engine.step_anchors(MISSION, 4038, Some(CASTLE));
    assert_eq!(anchors.advancers, vec![10, 11, 12, 13]);
    assert_eq!(anchors.tags, vec!["Frost_Terminal".to_string()]);
    assert_eq!(anchors.templates, vec!["Colonel Frost".to_string()]);
    assert_eq!(anchors.dialog_ids, vec![6110]);

    assert_eq!(
        engine.step_anchors(MISSION, 4037, Some(CASTLE)),
        StepAnchors::default(),
        "a step no chain gates has no advancers and no anchors"
    );
}

/// An advancer held to another world by a `world` condition cannot fire for
/// this player, so it and its anchors are left out. One held to the player's
/// world, or excluded only from another, still counts; an unknown world
/// fails closed as `Condition::World` does.
#[test]
fn step_anchors_skip_advancers_held_to_another_world() {
    let world = |operator, world_id| Condition::World { operator, world_id };
    let mut engine = ChainEngine::new();
    let mut here = gated(20, tag("Castle_Door"), 4038, vec![advance()]);
    here.conditions.push(world(ComparisonOp::Eq, CASTLE));
    engine.register_chain(here);
    let mut elsewhere = gated(21, tag("Harset_Door"), 4038, vec![advance()]);
    elsewhere.conditions.push(world(ComparisonOp::Eq, HARSET));
    engine.register_chain(elsewhere);
    let mut not_harset = gated(22, tag("Anywhere_But"), 4038, vec![advance()]);
    not_harset.conditions.push(world(ComparisonOp::Neq, HARSET));
    engine.register_chain(not_harset);
    engine.register_chain(gated(23, tag("Unscoped"), 4038, vec![advance()]));

    let in_castle = engine.step_anchors(MISSION, 4038, Some(CASTLE));
    assert_eq!(in_castle.advancers, vec![20, 22, 23]);
    assert_eq!(
        in_castle.tags,
        vec![
            "Anywhere_But".to_string(),
            "Castle_Door".to_string(),
            "Unscoped".to_string()
        ]
    );

    let in_harset = engine.step_anchors(MISSION, 4038, Some(HARSET));
    assert_eq!(in_harset.advancers, vec![21, 23]);

    let unknown = engine.step_anchors(MISSION, 4038, None);
    assert_eq!(
        unknown.advancers,
        vec![23],
        "an unknown world fails every world-gated chain closed"
    );
}

/// A registered chain with one condition reports `true`. It fails if the
/// accessor ignores `conditions`.
#[test]
fn chain_has_conditions_true_for_gated_chain() {
    let mut engine = ChainEngine::new();
    engine.register_chain(gated(30, tag("Frost_Terminal"), 4038, vec![advance()]));
    assert!(engine.chain_has_conditions(30));
}

/// A registered chain with no conditions reports `false`.
#[test]
fn chain_has_conditions_false_for_unconditioned_chain() {
    let mut engine = ChainEngine::new();
    engine.register_chain(make_chain(31, tag("Frost_Terminal"), vec![advance()], 0));
    assert!(!engine.chain_has_conditions(31));
}

/// An id no chain was registered under reports `false`.
#[test]
fn chain_has_conditions_false_for_unknown_id() {
    let mut engine = ChainEngine::new();
    engine.register_chain(gated(32, tag("Frost_Terminal"), 4038, vec![advance()]));
    assert!(!engine.chain_has_conditions(999));
}
