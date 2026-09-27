//! The pure rule: what the effect rows mean, and what a use changes.

use std::collections::HashMap;

use cimmeria_entity::crafting::CraftingState;

use super::super::rule::{decide, Applied, BlueprintChange, ItemEffects};
use super::super::MAX_RACIAL_PARADIGM_LEVEL;
use crate::base::crafting::feedback::CraftReject;

fn state(blueprints: &[i32], paradigms: &[(i32, i8)]) -> CraftingState {
    let mut state = CraftingState::new();
    state.blueprint_ids = blueprints.to_vec();
    state.racial_paradigm_levels = paradigms.iter().copied().collect::<HashMap<_, _>>();
    state
}

/// The parts of a state a use may change, comparable.
fn view(s: &CraftingState) -> (Vec<i32>, Vec<(i32, i8)>) {
    let mut levels: Vec<(i32, i8)> = s
        .racial_paradigm_levels
        .iter()
        .map(|(&k, &v)| (k, v))
        .collect();
    levels.sort_unstable();
    (s.blueprint_ids.clone(), levels)
}

#[test]
fn effect_rows_name_one_item_kind() {
    assert_eq!(
        ItemEffects::from_rows(&[(Some(369), None), (Some(367), None)]),
        Some(ItemEffects::TeachBlueprints(vec![367, 369]))
    );
    assert_eq!(
        ItemEffects::from_rows(&[(None, Some(3))]),
        Some(ItemEffects::RaiseParadigm(3))
    );
    assert_eq!(ItemEffects::from_rows(&[]), None, "no rows");
    assert_eq!(
        ItemEffects::from_rows(&[(None, Some(3)), (None, Some(4))]),
        None,
        "a guide for two paradigms"
    );
    assert_eq!(
        ItemEffects::from_rows(&[(Some(25), None), (None, Some(3))]),
        None,
        "a blueprint item and a guide at once"
    );
}

#[test]
fn a_blueprint_item_teaches_its_blueprint() {
    let mut s = state(&[40], &[]);
    let applied = decide(&mut s, &ItemEffects::TeachBlueprints(vec![25]), 6483).expect("taught");
    assert_eq!(
        applied,
        Applied::Learned {
            blueprints: vec![BlueprintChange {
                blueprint_id: 25,
                known_before: false,
                known_after: true,
            }],
            known_before: 1,
            known_after: 2,
        }
    );
    assert_eq!(s.blueprint_ids, vec![25, 40], "kept sorted");
}

/// An item that names a known and an unknown blueprint teaches the unknown
/// one; the event field shows both.
#[test]
fn a_two_blueprint_item_teaches_the_one_not_known() {
    let mut s = state(&[367], &[]);
    let applied =
        decide(&mut s, &ItemEffects::TeachBlueprints(vec![367, 369]), 8882).expect("taught");
    let Applied::Learned {
        blueprints,
        known_before,
        known_after,
    } = applied
    else {
        panic!("not a blueprint use: {applied:?}");
    };
    assert_eq!(
        Applied::blueprint_field(&blueprints),
        "367:true→true,369:false→true"
    );
    assert_eq!((known_before, known_after), (1, 2));
    assert_eq!(s.blueprint_ids, vec![367, 369]);
}

#[test]
fn a_blueprint_item_whose_blueprints_are_all_known_is_refused() {
    let mut s = state(&[367, 369], &[]);
    let before = view(&s);
    let why = decide(&mut s, &ItemEffects::TeachBlueprints(vec![367, 369]), 8882).unwrap_err();
    assert_eq!(
        why,
        CraftReject::BlueprintAlreadyKnown {
            type_id: 8882,
            blueprint_ids: vec![367, 369],
        }
    );
    assert_eq!(why.reason(), "already_known");
    assert_eq!(
        why.text(),
        "You already know these blueprints. The item was not used."
    );
    assert_eq!(why.blueprints_considered().as_deref(), Some("[367, 369]"));
    assert_eq!(view(&s), before, "a refusal changes nothing");
}

#[test]
fn a_guide_raises_its_paradigm_by_one() {
    let mut s = state(&[], &[(1, 5), (3, 9)]);
    let applied = decide(&mut s, &ItemEffects::RaiseParadigm(3), 7808).expect("raised");
    assert_eq!(
        applied,
        Applied::Raised {
            paradigm_id: 3,
            level_before: 9,
            level_after: 10,
        }
    );
    assert_eq!(s.racial_paradigm_levels[&3], MAX_RACIAL_PARADIGM_LEVEL);
    assert_eq!(s.racial_paradigm_levels[&1], 5, "other paradigms untouched");
}

#[test]
fn a_guide_at_the_maximum_is_refused() {
    let mut s = state(&[], &[(3, 10)]);
    let before = view(&s);
    let why = decide(&mut s, &ItemEffects::RaiseParadigm(3), 7808).unwrap_err();
    assert_eq!(why.reason(), "paradigm_max");
    assert_eq!(
        why.text(),
        "Your Goa'uld racial paradigm is already at 10, the maximum. The guide was not used."
    );
    let c = why.compared();
    assert_eq!(
        (
            c.design_id,
            c.paradigm_id,
            c.paradigm_level,
            c.required_level
        ),
        (Some(7808), Some(3), Some(10), Some(10))
    );
    assert_eq!(view(&s), before, "a refusal changes nothing");
}

#[test]
fn item_refusals_have_their_own_reasons_and_lines() {
    let missing = CraftReject::ItemMissing { item_id: 77 };
    let not_carried = CraftReject::ItemNotCarried {
        item_id: 77,
        type_id: 7808,
        container_id: 17,
    };
    assert_eq!(
        (missing.reason(), missing.text().as_str()),
        ("item_missing", "That item is no longer in your inventory.")
    );
    assert_eq!(
        (not_carried.reason(), not_carried.text().as_str()),
        (
            "not_carried",
            "Move that item to your crafting bag to use it."
        )
    );
    let c = not_carried.compared();
    assert_eq!(
        (c.item_id, c.design_id, c.container_id),
        (Some(77), Some(7808), Some(17))
    );
    for why in [missing, not_carried] {
        assert_eq!(why.error_code(), None, "text only");
    }
}
