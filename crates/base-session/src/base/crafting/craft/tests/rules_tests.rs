//! The pure craft rules: quantity bounds, the blueprint and discipline
//! checks, the named-instance checks and the component-set choice.

use cimmeria_cell_catalog::crafting::{
    Blueprint, Component, ComponentSet, CraftingCatalog, Discipline,
};
use cimmeria_entity::crafting::CraftingState;

use super::*;
use crate::base::crafting::craft::rules::{
    check_blueprint, check_named, check_quantity, plan_craft,
};

fn set(set_id: i32, components: &[(i32, i32)]) -> ComponentSet {
    ComponentSet {
        set_id,
        components: components
            .iter()
            .map(|&(item_id, quantity)| Component { item_id, quantity })
            .collect(),
    }
}

/// Blueprint 412 as seeded: set 1 is a subset of sets 2 and 4.
fn titanium_plating() -> Blueprint {
    Blueprint {
        blueprint_id: BLUEPRINT,
        discipline_id: Some(DISCIPLINE),
        is_alloy: false,
        product_id: Some(TITANIUM_PLATING),
        quantity: 1,
        requires_elementary_components: false,
        component_sets: vec![
            set(1, &[(STEEL_CORE, 14)]),
            set(2, &[(STEEL_CORE, 1), (TITANIUM_CORE, 5)]),
            set(3, &[(STEEL_CORE, 7), (5405, 1)]),
            set(4, &[(5188, 1), (STEEL_CORE, 1), (5366, 1)]),
        ],
    }
}

fn catalog() -> CraftingCatalog {
    let bare = |id, is_alloy, sets| Blueprint {
        blueprint_id: id,
        discipline_id: Some(DISCIPLINE),
        is_alloy,
        product_id: Some(19),
        quantity: if is_alloy { 2 } else { 1 },
        requires_elementary_components: is_alloy,
        component_sets: sets,
    };
    CraftingCatalog::from_rows(
        [Discipline {
            discipline_id: DISCIPLINE,
            applied_science_id: 1,
            racial_paradigm_id: 1,
            racial_paradigm_level: 5,
            tech_competency: 5,
            required_discipline_ids: vec![],
            name: "Biomedical".to_string(),
        }],
        [
            titanium_plating(),
            bare(NO_COMPONENTS, false, vec![]),
            bare(ALLOY, true, vec![set(1, &[(5191, 1)])]),
        ],
        [],
        [],
    )
}

fn state(disciplines: &[i32], blueprints: &[i32]) -> CraftingState {
    let mut s = CraftingState::new();
    for &d in disciplines {
        s.discipline_ids.push(d);
        s.set_expertise(d, 1);
    }
    s.blueprint_ids = blueprints.to_vec();
    s
}

fn instance(item_id: i32, type_id: i32, container_id: i32) -> NamedInstance {
    NamedInstance {
        item_id,
        type_id,
        container_id,
    }
}

fn totals(pairs: &[(i32, i64)]) -> HashMap<i32, i64> {
    pairs.iter().copied().collect()
}

#[test]
fn quantity_must_be_between_one_and_the_maximum() {
    assert_eq!(check_quantity(BLUEPRINT, 1), Ok(()));
    assert_eq!(check_quantity(BLUEPRINT, MAX_CRAFT_QUANTITY), Ok(()));
    for bad in [0, -1, MAX_CRAFT_QUANTITY + 1, i32::MIN, i32::MAX] {
        assert_eq!(
            check_quantity(BLUEPRINT, bad),
            Err(CraftReject::BadQuantity {
                blueprint_id: BLUEPRINT,
                quantity: bad,
                max: MAX_CRAFT_QUANTITY
            }),
            "{bad}"
        );
    }
}

#[test]
fn the_blueprint_must_be_known_not_an_alloy_and_its_discipline_known() {
    let c = catalog();
    let known = state(&[DISCIPLINE], &[BLUEPRINT, ALLOY]);
    assert_eq!(
        check_blueprint(&known, &c, BLUEPRINT).map(|b| b.blueprint_id),
        Ok(BLUEPRINT)
    );
    assert_eq!(
        check_blueprint(&state(&[DISCIPLINE], &[]), &c, BLUEPRINT).map(|b| b.blueprint_id),
        Err(CraftReject::UnknownBlueprint {
            blueprint_id: BLUEPRINT
        })
    );
    // Known to the player but missing from the catalog: still unknown.
    assert_eq!(
        check_blueprint(&state(&[DISCIPLINE], &[9999]), &c, 9999).map(|b| b.blueprint_id),
        Err(CraftReject::UnknownBlueprint { blueprint_id: 9999 })
    );
    assert_eq!(
        check_blueprint(&known, &c, ALLOY).map(|b| b.blueprint_id),
        Err(CraftReject::IsAlloy {
            blueprint_id: ALLOY
        })
    );
    assert_eq!(
        check_blueprint(&state(&[], &[BLUEPRINT]), &c, BLUEPRINT).map(|b| b.blueprint_id),
        Err(CraftReject::DisciplineUnknown {
            blueprint_id: BLUEPRINT,
            discipline_id: DISCIPLINE,
        })
    );
}

#[test]
fn named_instances_must_be_the_players_and_in_the_crafting_bags() {
    let found = [
        instance(10, STEEL_CORE, INV_MAIN),
        instance(11, TITANIUM_CORE, INV_CRAFTING),
        instance(12, STEEL_CORE, INV_BANK),
    ];
    assert_eq!(
        check_named(&[10, 11, 10], &found),
        Ok(vec![found[0], found[1]]),
        "each instance once, request order"
    );
    assert_eq!(
        check_named(&[10, 99], &found),
        Err(CraftReject::ComponentMissing { item_id: 99 })
    );
    assert_eq!(
        check_named(&[12], &found),
        Err(CraftReject::ComponentNotInCraftingBags {
            item_id: 12,
            container_id: INV_BANK
        })
    );
    assert_eq!(check_named(&[], &found), Ok(vec![]));
}

/// The subset trap: set 2's two designs also cover set 1 (Steel Core
/// only). Picking the first covered set would charge 14 Steel Cores for
/// what the player asked to make from set 2.
#[test]
fn the_set_is_the_one_whose_designs_match_exactly() {
    let bp = titanium_plating();
    let steel = instance(10, STEEL_CORE, INV_CRAFTING);
    let titanium = instance(11, TITANIUM_CORE, INV_CRAFTING);
    let plenty = totals(&[(STEEL_CORE, 50), (TITANIUM_CORE, 50)]);

    let set2 = plan_craft(&bp, TITANIUM_PLATING, &[steel, titanium], &plenty, 3).unwrap();
    assert_eq!(set2.component_set_id, 2);
    assert_eq!(
        set2.transaction.consume,
        vec![(STEEL_CORE, 3), (TITANIUM_CORE, 15)]
    );
    assert_eq!(set2.transaction.grant, vec![(TITANIUM_PLATING, 3)]);
    assert_eq!(set2.transaction.expertise, vec![(DISCIPLINE, 1)]);
    assert!(
        set2.transaction.named_items.is_empty() && set2.transaction.consume_named.is_empty(),
        "the named instances choose the set; the completion consumes by design"
    );

    let set1 = plan_craft(&bp, TITANIUM_PLATING, &[steel], &plenty, 2).unwrap();
    assert_eq!(set1.component_set_id, 1);
    assert_eq!(set1.transaction.consume, vec![(STEEL_CORE, 28)]);
}

#[test]
fn designs_that_match_no_set_are_refused() {
    let bp = titanium_plating();
    let plenty = totals(&[(STEEL_CORE, 50), (TITANIUM_CORE, 50), (5188, 5)]);
    let refused = |named: &[NamedInstance]| plan_craft(&bp, TITANIUM_PLATING, named, &plenty, 1);
    // Titanium Core alone is part of set 2 but not all of it.
    assert_eq!(
        refused(&[instance(11, TITANIUM_CORE, INV_MAIN)]),
        Err(CraftReject::NoComponentSet {
            blueprint_id: BLUEPRINT,
            type_ids: vec![TITANIUM_CORE],
        })
    );
    // An extra design on top of set 2.
    assert_eq!(
        refused(&[
            instance(10, STEEL_CORE, INV_MAIN),
            instance(11, TITANIUM_CORE, INV_MAIN),
            instance(12, 5188, INV_MAIN),
        ]),
        Err(CraftReject::NoComponentSet {
            blueprint_id: BLUEPRINT,
            type_ids: vec![5188, STEEL_CORE, TITANIUM_CORE],
        })
    );
    assert_eq!(
        refused(&[]),
        Err(CraftReject::NoComponentSet {
            blueprint_id: BLUEPRINT,
            type_ids: vec![],
        })
    );
}

#[test]
fn a_blueprint_with_no_component_set_is_never_craftable() {
    let c = catalog();
    let bp = c.blueprint(NO_COMPONENTS).unwrap();
    for named in [vec![], vec![instance(10, STEEL_CORE, INV_MAIN)]] {
        let types: Vec<i32> = named.iter().map(|n| n.type_id).collect();
        assert_eq!(
            plan_craft(bp, 19, &named, &totals(&[(STEEL_CORE, 99)]), 1),
            Err(CraftReject::NoComponentSet {
                blueprint_id: NO_COMPONENTS,
                type_ids: types,
            })
        );
    }
}

/// The bags must hold `quantity × component.quantity`, counted across
/// stacks, not what the named instance alone holds.
#[test]
fn the_bags_must_hold_quantity_times_each_component() {
    let bp = titanium_plating();
    let steel = [instance(10, STEEL_CORE, INV_CRAFTING)];
    assert!(plan_craft(
        &bp,
        TITANIUM_PLATING,
        &steel,
        &totals(&[(STEEL_CORE, 28)]),
        2
    )
    .is_ok());
    assert_eq!(
        plan_craft(
            &bp,
            TITANIUM_PLATING,
            &steel,
            &totals(&[(STEEL_CORE, 27)]),
            2
        ),
        Err(CraftReject::InsufficientComponents {
            blueprint_id: BLUEPRINT,
            design_id: STEEL_CORE,
            needed: 28,
            available: 27,
        })
    );
    assert_eq!(
        plan_craft(&bp, TITANIUM_PLATING, &steel, &totals(&[]), 1),
        Err(CraftReject::InsufficientComponents {
            blueprint_id: BLUEPRINT,
            design_id: STEEL_CORE,
            needed: 14,
            available: 0,
        })
    );
}

/// Blueprint 159: sets 1 and 2 have the same designs (15 + 5 or 1 + 5).
/// The first set the bags can pay for wins; when neither can, the refusal
/// names the first set's shortfall.
#[test]
fn among_sets_with_the_same_designs_the_first_affordable_wins() {
    let bp = Blueprint {
        blueprint_id: 159,
        discipline_id: Some(DISCIPLINE),
        is_alloy: false,
        product_id: Some(5339),
        quantity: 1,
        requires_elementary_components: false,
        component_sets: vec![
            set(1, &[(5188, 15), (5189, 5)]),
            set(2, &[(5188, 1), (5189, 5)]),
        ],
    };
    let named = [instance(10, 5188, INV_MAIN), instance(11, 5189, INV_MAIN)];
    let plan = |have: &[(i32, i64)]| plan_craft(&bp, 5339, &named, &totals(have), 1);
    assert_eq!(plan(&[(5188, 15), (5189, 5)]).unwrap().component_set_id, 1);
    assert_eq!(plan(&[(5188, 2), (5189, 5)]).unwrap().component_set_id, 2);
    assert_eq!(
        plan(&[(5188, 2), (5189, 4)]),
        Err(CraftReject::InsufficientComponents {
            blueprint_id: 159,
            design_id: 5188,
            needed: 15,
            available: 2,
        })
    );
}

#[test]
fn a_product_quantity_that_overflows_is_a_bad_quantity() {
    let mut bp = titanium_plating();
    bp.quantity = i32::MAX;
    let steel = [instance(10, STEEL_CORE, INV_CRAFTING)];
    assert_eq!(
        plan_craft(
            &bp,
            TITANIUM_PLATING,
            &steel,
            &totals(&[(STEEL_CORE, 1_000)]),
            2
        ),
        Err(CraftReject::BadQuantity {
            blueprint_id: BLUEPRINT,
            quantity: 2,
            max: MAX_CRAFT_QUANTITY
        })
    );
}
