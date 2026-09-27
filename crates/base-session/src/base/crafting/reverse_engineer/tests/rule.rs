//! The reverse-engineering rule without a database: which items qualify,
//! the bias, the uniform picks and the recovery.

use cimmeria_cell_catalog::crafting::{
    Blueprint, Component, ComponentSet, CraftItemAttrs, CraftingCatalog, ItemFlags, ItemQuality,
};
use cimmeria_entity::crafting::CraftingState;

use super::super::rule::*;
use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::item_lookup::HeldInstance;
use crate::base::crafting::rng::ScriptedRng;

/// Reverse-engineerable, tech competency 20, made by blueprints 1 and 2.
const GEAR: i32 = 5481;
/// Reverse-engineerable, made by nothing.
const ORPHAN: i32 = 21;
/// Made by blueprint 3, which has no recipe.
const NO_RECIPE: i32 = 30;
/// Not reverse-engineerable.
const PLAIN: i32 = 5254;

fn attrs(flags: u32, tc: i32) -> CraftItemAttrs {
    CraftItemAttrs {
        flags: ItemFlags(flags),
        tier: 2,
        quality: ItemQuality::Normal,
        tech_comp: tc,
        discipline_ids: vec![21],
        applied_science_id: Some(1),
    }
}

fn set(set_id: i32, components: &[(i32, i32)]) -> ComponentSet {
    ComponentSet {
        set_id,
        components: components
            .iter()
            .map(|&(item_id, quantity)| Component { item_id, quantity })
            .collect(),
    }
}

fn blueprint(id: i32, discipline: i32, product: i32, sets: Vec<ComponentSet>) -> Blueprint {
    Blueprint {
        blueprint_id: id,
        discipline_id: Some(discipline),
        is_alloy: false,
        product_id: Some(product),
        quantity: 1,
        requires_elementary_components: false,
        component_sets: sets,
    }
}

fn catalog() -> CraftingCatalog {
    CraftingCatalog::from_rows(
        [],
        [
            blueprint(
                1,
                21,
                GEAR,
                vec![set(1, &[(5188, 2), (5189, 1)]), set(3, &[(5256, 4)])],
            ),
            blueprint(2, 22, GEAR, vec![set(1, &[(5224, 10)])]),
            blueprint(3, 21, NO_RECIPE, vec![]),
        ],
        [],
        [
            (GEAR, attrs(ItemFlags::CRAFT_REV_ENG, 20)),
            (ORPHAN, attrs(ItemFlags::CRAFT_REV_ENG, 1)),
            (NO_RECIPE, attrs(ItemFlags::CRAFT_REV_ENG, 1)),
            (PLAIN, attrs(ItemFlags::CAN_BE_SOLD, 3)),
        ],
    )
}

fn held(type_id: i32) -> HeldInstance {
    HeldInstance {
        item_id: 7,
        type_id,
        container_id: 15,
        stack_size: 1,
    }
}

fn knowing(discipline_id: i32, expertise: i32) -> CraftingState {
    let mut state = CraftingState::new();
    state.discipline_ids.push(discipline_id);
    state.set_expertise(discipline_id, expertise);
    state
}

#[test]
fn only_reverse_engineerable_items_made_by_a_recipe_qualify() {
    let c = catalog();
    assert_eq!(check_request(&c, &held(GEAR)), Ok(()));
    assert_eq!(
        check_request(&c, &held(PLAIN)),
        Err(CraftReject::NotReverseEngineerable {
            item_id: 7,
            type_id: PLAIN
        })
    );
    for type_id in [ORPHAN, NO_RECIPE] {
        assert_eq!(
            check_request(&c, &held(type_id)),
            Err(CraftReject::NoBlueprintForItem {
                item_id: 7,
                type_id
            }),
            "{type_id}"
        );
    }
}

#[test]
fn candidates_are_the_recipes_in_id_order() {
    let c = catalog();
    let ids: Vec<i32> = candidate_blueprints(&c, GEAR)
        .iter()
        .map(|b| b.blueprint_id)
        .collect();
    assert_eq!(ids, vec![1, 2]);
    assert!(candidate_blueprints(&c, NO_RECIPE).is_empty());
}

#[test]
fn bias_rises_with_expertise_and_never_divides_by_zero() {
    assert_eq!(bias(0, 20), 1.0 / 20.0, "unknown discipline counts as 1");
    assert_eq!(bias(1, 20), 1.0 / 20.0);
    assert_eq!(bias(10, 20), 0.5);
    assert_eq!(bias(20, 20), 1.0);
    assert_eq!(bias(100, 20), 1.0, "capped at 1");
    assert_eq!(bias(0, 0), 1.0);
}

#[test]
fn picks_are_uniform_and_never_past_the_end() {
    let c = catalog();
    let candidates = candidate_blueprints(&c, GEAR);
    let state = knowing(21, 20);
    // Blueprint pick, set pick, then one sample per component.
    let r = recover(
        &candidates,
        &state,
        20,
        &mut ScriptedRng::new(vec![0.0, 0.6, 0.9]),
    )
    .unwrap();
    assert_eq!((r.blueprint_id, r.component_set_id), (1, 3));
    let r = recover(
        &candidates,
        &state,
        20,
        &mut ScriptedRng::new(vec![1.0, 1.0, 0.5]),
    )
    .unwrap();
    assert_eq!((r.blueprint_id, r.component_set_id), (2, 1));
    assert!(recover(&[], &state, 20, &mut ScriptedRng::new(vec![0.0])).is_none());
}

#[test]
fn high_expertise_recovers_floor_of_roll_times_quantity() {
    let c = catalog();
    let candidates = candidate_blueprints(&c, GEAR);
    let r = recover(
        &candidates,
        &knowing(21, 20),
        20,
        &mut ScriptedRng::new(vec![0.0, 0.0, 0.99, 0.5]),
    )
    .unwrap();
    assert_eq!(r.bias, 1.0);
    let got: Vec<(i32, i32)> = r
        .components
        .iter()
        .map(|c| (c.design_id, c.recovered))
        .collect();
    // floor(0.99 × 2) = 1, floor(0.5 × 1) = 0.
    assert_eq!(got, vec![(5188, 1), (5189, 0)]);
    assert_eq!(r.grants(), vec![(5188, 1)]);
    assert_eq!(r.rolls_field(), "5188:0.9900:1/2,5189:0.5000:0/1");
}

#[test]
fn low_expertise_recovers_less_but_always_something() {
    let c = catalog();
    let candidates = candidate_blueprints(&c, GEAR);
    // Unknown discipline: bias 1/20, so floor(0.99 × 0.05 × 2) = 0 and
    // floor(0.5 × 0.05 × 1) = 0. The highest roll gets one unit.
    let r = recover(
        &candidates,
        &CraftingState::new(),
        20,
        &mut ScriptedRng::new(vec![0.0, 0.0, 0.99, 0.5]),
    )
    .unwrap();
    assert_eq!(r.bias, 0.05);
    assert_eq!(r.grants(), vec![(5188, 1)]);
    // A tie keeps the first component.
    let r = recover(
        &candidates,
        &CraftingState::new(),
        20,
        &mut ScriptedRng::new(vec![0.0, 0.0, 0.3, 0.3]),
    )
    .unwrap();
    assert_eq!(r.grants(), vec![(5188, 1)]);
}

#[test]
fn the_same_rolls_recover_more_at_higher_expertise() {
    let c = catalog();
    let candidates = candidate_blueprints(&c, GEAR);
    let samples = || ScriptedRng::new(vec![0.0, 0.9, 0.99]);
    // Set 3 is 4 × 5256.
    let at = |expertise| {
        recover(&candidates, &knowing(21, expertise), 20, &mut samples())
            .unwrap()
            .grants()
    };
    assert_eq!(
        at(0),
        vec![(5256, 1)],
        "floor(0.99 × 0.05 × 4) = 0, then the floor of one"
    );
    assert_eq!(at(10), vec![(5256, 1)], "floor(0.99 × 0.5 × 4) = 1");
    assert_eq!(at(20), vec![(5256, 3)], "floor(0.99 × 1 × 4) = 3");
}
