//! [`check_alloy`] without a database: the blueprint rules, the
//! current-tier item, the tier rule and every count rule.

use cimmeria_cell_catalog::crafting::{
    Blueprint, ComponentRow, CraftItemAttrs, CraftingCatalog, Discipline, ItemFlags, ItemQuality,
};
use cimmeria_entity::crafting::CraftingState;

use super::super::rules::{check_alloy, AlloyCheck, AlloyPlan, AlloyRequest, HeldItem};
use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::transaction::NamedItem;

const DISCIPLINE: i32 = 21;
const ALLOY: i32 = 42;
const CRAFT: i32 = 25;
/// An alloy blueprint that needs no elementary components (none in the
/// seed; the flag is honoured anyway).
const PLAIN_ALLOY: i32 = 90;
/// An alloy blueprint whose component has no catalog attributes.
const BROKEN_ALLOY: i32 = 91;
/// An alloy blueprint that makes nothing (quantity 0).
const EMPTY_ALLOY: i32 = 92;
/// An alloy blueprint whose discipline the catalog lacks.
const ORPHAN_ALLOY: i32 = 93;
const MISSING_DISCIPLINE: i32 = 999;

const COMPONENT: i32 = 5192; // tier 2
const PRODUCT: i32 = 5191;
const NORMAL: i32 = 5188; // tier 1
const GOOD: i32 = 5189;
const GREAT: i32 = 5395;
const FANTASTIC: i32 = 2891;
const POOR: i32 = 2492;
const TIER_TWO: i32 = 5193;

const CURRENT: i32 = 1000;

fn attrs(tier: i32, quality: ItemQuality) -> CraftItemAttrs {
    CraftItemAttrs {
        flags: ItemFlags(0),
        tier,
        quality,
        tech_comp: 0,
        discipline_ids: vec![],
        applied_science_id: None,
    }
}

fn blueprint(blueprint_id: i32, is_alloy: bool, elementary: bool) -> Blueprint {
    Blueprint {
        blueprint_id,
        discipline_id: Some(DISCIPLINE),
        is_alloy,
        product_id: Some(PRODUCT),
        quantity: 2,
        requires_elementary_components: elementary,
        component_sets: vec![],
    }
}

fn catalog() -> CraftingCatalog {
    let component = |blueprint_id, item_id| ComponentRow {
        blueprint_id,
        set_id: 1,
        item_id,
        quantity: 1,
    };
    CraftingCatalog::from_rows(
        [Discipline {
            discipline_id: DISCIPLINE,
            applied_science_id: 1,
            racial_paradigm_id: 1,
            racial_paradigm_level: 0,
            tech_competency: 1,
            required_discipline_ids: vec![],
            name: "Biomedical".into(),
        }],
        [
            blueprint(ALLOY, true, true),
            blueprint(CRAFT, false, false),
            blueprint(PLAIN_ALLOY, true, false),
            blueprint(BROKEN_ALLOY, true, true),
            Blueprint {
                quantity: 0,
                ..blueprint(EMPTY_ALLOY, true, true)
            },
            Blueprint {
                discipline_id: Some(MISSING_DISCIPLINE),
                ..blueprint(ORPHAN_ALLOY, true, true)
            },
        ],
        [
            component(ALLOY, COMPONENT),
            component(CRAFT, COMPONENT),
            component(PLAIN_ALLOY, COMPONENT),
            component(BROKEN_ALLOY, 7777),
            component(EMPTY_ALLOY, COMPONENT),
            component(ORPHAN_ALLOY, COMPONENT),
        ],
        [
            (COMPONENT, attrs(2, ItemQuality::Good)),
            (PRODUCT, attrs(2, ItemQuality::Great)),
            (NORMAL, attrs(1, ItemQuality::Normal)),
            (GOOD, attrs(1, ItemQuality::Good)),
            (GREAT, attrs(1, ItemQuality::Great)),
            (FANTASTIC, attrs(1, ItemQuality::Fantastic)),
            (POOR, attrs(1, ItemQuality::Poor)),
            (TIER_TWO, attrs(2, ItemQuality::Good)),
        ],
    )
}

fn state() -> CraftingState {
    let mut state = CraftingState::new();
    state.discipline_ids = vec![DISCIPLINE];
    state.expertise.insert(DISCIPLINE, 1);
    state.blueprint_ids = vec![ALLOY, CRAFT, PLAIN_ALLOY, BROKEN_ALLOY, EMPTY_ALLOY];
    state
}

/// A held stack in the crafting bag.
fn held(item_id: i32, type_id: i32, stack_size: i32) -> HeldItem {
    HeldItem {
        item_id,
        type_id,
        stack_size,
        container_id: 15,
    }
}

/// `n` one-item stacks of `type_id`, ids from `first`.
fn singles(first: i32, type_id: i32, n: i32) -> Vec<HeldItem> {
    (0..n).map(|k| held(first + k, type_id, 1)).collect()
}

/// Check an alloy of `blueprint_id` with the current-tier item plus
/// `lower`, every one of them held.
fn check_with(
    state: &CraftingState,
    blueprint_id: i32,
    lower: &[HeldItem],
) -> Result<AlloyPlan, AlloyCheck> {
    let mut all = vec![held(CURRENT, COMPONENT, 1)];
    all.extend_from_slice(lower);
    let ids: Vec<i32> = lower.iter().map(|h| h.item_id).collect();
    let request = AlloyRequest {
        blueprint_id,
        current_tier_item_id: CURRENT,
        lower_tier_items: &ids,
    };
    check_alloy(state, &catalog(), &request, &all, 1)
}

fn check(lower: &[HeldItem]) -> Result<AlloyPlan, AlloyCheck> {
    check_with(&state(), ALLOY, lower)
}

fn refused(result: Result<AlloyPlan, AlloyCheck>) -> CraftReject {
    match result {
        Err(AlloyCheck::Refused(why)) => why,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn used(plan: &AlloyPlan) -> Vec<(i32, i32)> {
    plan.elementary
        .iter()
        .map(|e| (e.item_id, e.quantity))
        .collect()
}

#[test]
fn ten_normal_elementaries_make_the_alloy() {
    let plan = check(&singles(1, NORMAL, 10)).expect("valid alloy");
    assert_eq!(plan.bucket, Some(ItemQuality::Normal));
    assert_eq!(used(&plan).len(), 10);
    let tx = plan.transaction();
    assert_eq!(tx.consume, vec![(COMPONENT, 1)]);
    assert_eq!(tx.grant, vec![(PRODUCT, 2)]);
    assert_eq!(tx.expertise, vec![(DISCIPLINE, 1)]);
    assert_eq!(tx.named_items[0], NamedItem::new(1, NORMAL));
    assert_eq!(tx.named_items.len(), 10, "the component is not named");
    assert_eq!(
        tx.consume_named,
        (1..=10).map(|id| (id, 1)).collect::<Vec<_>>()
    );
    assert_eq!(
        plan.elementary_field().split(',').next(),
        Some("1:5188:normal:1:1")
    );
}

/// Each quality's count, counted by stack quantity: one short is refused,
/// exactly the count is accepted.
#[test]
fn each_quality_needs_its_own_count() {
    for (type_id, count) in [(NORMAL, 10), (GOOD, 5), (GREAT, 2), (FANTASTIC, 1)] {
        let short = refused(check(&[held(1, type_id, count - 1)]));
        assert_eq!(short.reason(), "count_not_met", "{type_id} x {}", count - 1);
        let plan = check(&[held(1, type_id, count)]).expect("count met");
        assert_eq!(used(&plan), vec![(1, count)], "{type_id} x {count}");
    }
}

/// The count is summed stack quantity, not the number of items: two Good
/// stacks of 3 meet Good's 5, and only 5 are used.
#[test]
fn counts_are_stack_quantities_and_only_the_count_is_used() {
    let plan = check(&[held(1, GOOD, 3), held(2, GOOD, 3)]).expect("6 Good");
    assert_eq!(used(&plan), vec![(1, 3), (2, 2)]);
    let plan = check(&[held(1, NORMAL, 12)]).expect("12 Normal");
    assert_eq!(used(&plan), vec![(1, 10)]);
}

#[test]
fn poor_counts_toward_nothing() {
    let why = refused(check(&singles(1, POOR, 10)));
    assert_eq!(why, CraftReject::CountNotMet { counts: [0; 4] });
}

#[test]
fn two_qualities_met_at_once_are_refused() {
    let mut lower = singles(1, NORMAL, 10);
    lower.extend(singles(20, GOOD, 5));
    let why = refused(check(&lower));
    assert_eq!(
        why,
        CraftReject::MultipleBuckets {
            counts: [10, 5, 0, 0]
        }
    );
}

/// A second quality below its count does not block the one that is met,
/// and is not used.
#[test]
fn an_unmet_second_quality_is_not_used() {
    let lower = [held(1, GREAT, 1), held(2, NORMAL, 3), held(3, GREAT, 1)];
    let plan = check(&lower).expect("2 Great");
    assert_eq!(plan.bucket, Some(ItemQuality::Great));
    assert_eq!(used(&plan), vec![(1, 1), (3, 1)]);
    assert!(!plan
        .transaction()
        .named_items
        .contains(&NamedItem::new(2, NORMAL)));
}

/// The tier guard: an elementary item that is not exactly one tier below
/// the component is refused, even when the counts would be met.
#[test]
fn an_elementary_of_the_wrong_tier_is_refused() {
    let mut lower = singles(1, NORMAL, 10);
    lower.push(held(30, TIER_TWO, 1));
    let why = refused(check(&lower));
    assert_eq!(
        why,
        CraftReject::WrongTier {
            item_id: 30,
            type_id: TIER_TWO,
            tier: 2,
            required_tier: 1,
        }
    );
}

#[test]
fn a_repeated_instance_counts_once() {
    let lower = [held(1, NORMAL, 1)];
    let mut all = vec![held(CURRENT, COMPONENT, 1)];
    all.extend_from_slice(&lower);
    let ids = [1; 10];
    let request = AlloyRequest {
        blueprint_id: ALLOY,
        current_tier_item_id: CURRENT,
        lower_tier_items: &ids,
    };
    let why = refused(check_alloy(&state(), &catalog(), &request, &all, 1));
    assert_eq!(
        why,
        CraftReject::CountNotMet {
            counts: [1, 0, 0, 0]
        }
    );
}

#[test]
fn blueprint_rules() {
    let lower = singles(1, NORMAL, 10);
    assert_eq!(
        refused(check_with(&state(), CRAFT, &lower)),
        CraftReject::NotAlloy {
            blueprint_id: CRAFT
        }
    );
    assert_eq!(
        refused(check_with(&state(), 4242, &lower)),
        CraftReject::UnknownBlueprint { blueprint_id: 4242 }
    );
    let mut unlearned = state();
    unlearned.blueprint_ids.retain(|&b| b != ALLOY);
    assert_eq!(
        refused(check_with(&unlearned, ALLOY, &lower)),
        CraftReject::UnknownBlueprint {
            blueprint_id: ALLOY
        }
    );
    let mut no_discipline = state();
    no_discipline.discipline_ids.clear();
    no_discipline.expertise.clear();
    assert_eq!(
        refused(check_with(&no_discipline, ALLOY, &lower)),
        CraftReject::DisciplineUnknown {
            blueprint_id: ALLOY,
            discipline_id: DISCIPLINE
        }
    );
}

#[test]
fn the_current_tier_item_must_be_the_component() {
    let lower: Vec<i32> = (1..=10).collect();
    let mut all = singles(1, NORMAL, 10);
    let request = |current| AlloyRequest {
        blueprint_id: ALLOY,
        current_tier_item_id: current,
        lower_tier_items: &lower,
    };
    // Not held at all (an empty slot sends 0).
    let why = refused(check_alloy(&state(), &catalog(), &request(0), &all, 1));
    assert_eq!(why, CraftReject::ComponentMissing { item_id: 0 });
    // Another design.
    all.push(held(CURRENT, GOOD, 1));
    let why = refused(check_alloy(
        &state(),
        &catalog(),
        &request(CURRENT),
        &all,
        1,
    ));
    assert_eq!(why.reason(), "component_mismatch");
    // In the bank.
    all.pop();
    all.push(HeldItem {
        container_id: 17,
        ..held(CURRENT, COMPONENT, 1)
    });
    let why = refused(check_alloy(
        &state(),
        &catalog(),
        &request(CURRENT),
        &all,
        1,
    ));
    assert_eq!(
        why,
        CraftReject::ComponentNotInCraftingBags {
            item_id: CURRENT,
            container_id: 17
        }
    );
    // None in the carried bags.
    all.pop();
    all.push(held(CURRENT, COMPONENT, 1));
    let why = refused(check_alloy(
        &state(),
        &catalog(),
        &request(CURRENT),
        &all,
        0,
    ));
    assert_eq!(why.reason(), "not_enough_components");
}

#[test]
fn an_elementary_outside_the_carried_bags_is_refused() {
    let mut lower = singles(1, NORMAL, 10);
    lower[4].container_id = 17;
    let why = refused(check(&lower));
    assert_eq!(
        why,
        CraftReject::ComponentNotInCraftingBags {
            item_id: 5,
            container_id: 17
        }
    );
}

#[test]
fn a_blueprint_without_elementary_components_uses_none() {
    let plan = check_with(&state(), PLAIN_ALLOY, &singles(1, NORMAL, 10)).expect("plain alloy");
    assert_eq!(plan.bucket, None);
    assert!(plan.elementary.is_empty());
    assert!(plan.transaction().named_items.is_empty());
}

#[test]
fn a_catalog_gap_is_a_lookup_fault_not_a_refusal() {
    let lower = singles(1, NORMAL, 10);
    let mut all = vec![held(CURRENT, 7777, 1)];
    all.extend_from_slice(&lower);
    let ids: Vec<i32> = (1..=10).collect();
    let request = AlloyRequest {
        blueprint_id: BROKEN_ALLOY,
        current_tier_item_id: CURRENT,
        lower_tier_items: &ids,
    };
    assert_eq!(
        check_alloy(&state(), &catalog(), &request, &all, 1),
        Err(AlloyCheck::Catalog {
            phase: "alloy_item",
            id: 7777
        })
    );
}

/// A blueprint that grants nothing would consume the inputs for nothing
/// (the transaction skips a non-positive grant): a data fault, refused.
#[test]
fn an_alloy_that_makes_nothing_is_a_catalog_fault() {
    assert_eq!(
        check_with(&state(), EMPTY_ALLOY, &singles(1, NORMAL, 10)),
        Err(AlloyCheck::Catalog {
            phase: "alloy_product_quantity",
            id: EMPTY_ALLOY
        })
    );
}

/// An unlearned blueprint is `unknown_blueprint` whatever kind it is, so
/// the answer does not reveal that a non-alloy blueprint exists.
#[test]
fn an_unlearned_craft_blueprint_is_unknown_not_not_alloy() {
    let mut unlearned = state();
    unlearned.blueprint_ids.retain(|&b| b != CRAFT);
    assert_eq!(
        refused(check_with(&unlearned, CRAFT, &singles(1, NORMAL, 10))),
        CraftReject::UnknownBlueprint {
            blueprint_id: CRAFT
        }
    );
}

/// A discipline the player lists but the catalog lacks is a data fault:
/// otherwise the inputs would be taken and the expertise silently skipped.
#[test]
fn a_discipline_missing_from_the_catalog_is_a_catalog_fault() {
    let mut stale = state();
    stale.discipline_ids.push(MISSING_DISCIPLINE);
    stale.expertise.insert(MISSING_DISCIPLINE, 1);
    stale.blueprint_ids.push(ORPHAN_ALLOY);
    assert_eq!(
        check_with(&stale, ORPHAN_ALLOY, &singles(1, NORMAL, 10)),
        Err(AlloyCheck::Catalog {
            phase: "alloy_discipline",
            id: ORPHAN_ALLOY
        })
    );
}
