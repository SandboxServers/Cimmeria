//! The kit and learn rules, without a database.

use std::collections::HashMap;

use cimmeria_cell_catalog::crafting::{Blueprint, Component, ComponentSet, CraftingCatalog};
use cimmeria_entity::crafting::CraftingState;

use super::super::craftkit::{kit_grants, KitRefusal, MAX_KIT_COUNT};
use super::super::learn_blueprint::{learn, LearnRefusal};

fn component(item_id: i32, quantity: i32) -> Component {
    Component { item_id, quantity }
}

fn blueprint(blueprint_id: i32, sets: Vec<(i32, Vec<Component>)>) -> Blueprint {
    Blueprint {
        blueprint_id,
        discipline_id: Some(78),
        is_alloy: false,
        product_id: Some(5398),
        quantity: 1,
        requires_elementary_components: false,
        component_sets: sets
            .into_iter()
            .map(|(set_id, components)| ComponentSet { set_id, components })
            .collect(),
    }
}

/// Blueprint 25's real shape (set 1: 13x 5254; set 2: 5x 5256), 412's
/// two-item set, one with no sets and one with only set 2.
fn catalog() -> CraftingCatalog {
    let blueprints = [
        blueprint(
            25,
            vec![
                (1, vec![component(5254, 13)]),
                (2, vec![component(5256, 5)]),
            ],
        ),
        blueprint(412, vec![(1, vec![component(5254, 1), component(5256, 5)])]),
        blueprint(21, vec![]),
        blueprint(30, vec![(2, vec![component(5254, 1)])]),
    ];
    CraftingCatalog {
        disciplines: HashMap::new(),
        blueprints: blueprints
            .into_iter()
            .map(|b| (b.blueprint_id, b))
            .collect(),
        items: HashMap::new(),
    }
}

#[test]
fn a_kit_is_set_one_times_the_count() {
    let c = catalog();
    assert_eq!(kit_grants(&c, 25, 1), Ok(vec![(5254, 13)]));
    assert_eq!(kit_grants(&c, 25, 3), Ok(vec![(5254, 39)]));
    assert_eq!(
        kit_grants(&c, 412, 2),
        Ok(vec![(5254, 2), (5256, 10)]),
        "every component of set 1, never another set's"
    );
}

#[test]
fn a_kit_outside_the_count_range_is_refused() {
    let c = catalog();
    assert_eq!(kit_grants(&c, 25, 0), Err(KitRefusal::BadCount));
    assert_eq!(kit_grants(&c, 25, -1), Err(KitRefusal::BadCount));
    assert_eq!(
        kit_grants(&c, 25, MAX_KIT_COUNT + 1),
        Err(KitRefusal::BadCount)
    );
    assert!(kit_grants(&c, 25, MAX_KIT_COUNT).is_ok());
}

#[test]
fn a_kit_needs_a_blueprint_with_a_first_set() {
    let c = catalog();
    assert_eq!(kit_grants(&c, 999, 1), Err(KitRefusal::UnknownBlueprint));
    assert_eq!(kit_grants(&c, 21, 1), Err(KitRefusal::NoComponents));
    assert_eq!(
        kit_grants(&c, 30, 1),
        Err(KitRefusal::NoComponents),
        "set 2 alone is not a kit"
    );
}

#[test]
fn kit_refusals_name_their_reason_and_read_as_sentences() {
    assert_eq!(KitRefusal::BadCount.reason(), "bad_count");
    assert_eq!(KitRefusal::UnknownBlueprint.reason(), "unknown_blueprint");
    assert_eq!(KitRefusal::NoComponents.reason(), "no_components");
    assert_eq!(
        KitRefusal::BadCount.text(25, 11),
        "craftkit: refused, count 11 is not between 1 and 10."
    );
    assert_eq!(
        KitRefusal::UnknownBlueprint.text(999, 1),
        "craftkit: refused, there is no blueprint 999."
    );
    assert_eq!(
        KitRefusal::NoComponents.text(21, 1),
        "craftkit: refused, blueprint 21 has no component set 1."
    );
}

#[test]
fn learning_adds_the_blueprint_in_order() {
    let mut state = CraftingState::new();
    state.blueprint_ids = vec![5, 30];
    assert_eq!(learn(&mut state, 25), Ok((2, 3)));
    assert_eq!(state.blueprint_ids, vec![5, 25, 30]);
}

#[test]
fn a_known_blueprint_is_refused_and_the_state_kept() {
    let mut state = CraftingState::new();
    state.blueprint_ids = vec![25];
    assert_eq!(learn(&mut state, 25), Err(LearnRefusal::AlreadyKnown));
    assert_eq!(state.blueprint_ids, vec![25]);
    assert_eq!(LearnRefusal::AlreadyKnown.reason(), "already_known");
    assert_eq!(
        LearnRefusal::AlreadyKnown.text(4300, 25),
        "learnblueprint [4300]: refused, blueprint 25 is already known."
    );
}
