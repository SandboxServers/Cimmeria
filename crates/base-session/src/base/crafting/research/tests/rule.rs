//! The research rule without a database: request checks, the roll, the
//! blueprints a success teaches, and the result line.

use cimmeria_cell_catalog::crafting::{
    Blueprint, CraftItemAttrs, CraftingCatalog, Discipline, ItemFlags, ItemQuality,
};
use cimmeria_entity::crafting::CraftingState;

use super::super::result_line;
use super::super::rule::*;
use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::item_lookup::HeldInstance;
use crate::base::crafting::rng::ScriptedRng;
use crate::base::crafting::transaction::{BlueprintsLearned, CraftApplied, ExpertiseChange};

/// The researched item: applied science 1, tech competency 20,
/// disciplines 21 and 22 (the seed's 5481).
const ITEM: i32 = 5481;
/// Kickers of sciences 1 (the item's own), 4 and 2.
const KICKER_OWN: i32 = 5668;
const KICKER_4: i32 = 5669;
const KICKER_2: i32 = 5671;
/// A plain component, neither researchable nor a kicker.
const PLAIN: i32 = 5254;
/// A kicker-flagged item with no applied science.
const KICKER_NO_SCIENCE: i32 = 9001;
/// An item of applied science 4 that is not flagged a kicker.
const SCIENCE_NOT_KICKER: i32 = 2925;

fn attrs(flags: u32, science: Option<i32>, tc: i32, disciplines: &[i32]) -> CraftItemAttrs {
    CraftItemAttrs {
        flags: ItemFlags(flags),
        tier: 2,
        quality: ItemQuality::Normal,
        tech_comp: tc,
        discipline_ids: disciplines.to_vec(),
        applied_science_id: science,
    }
}

fn discipline(id: i32, name: &str) -> Discipline {
    Discipline {
        discipline_id: id,
        applied_science_id: 1,
        racial_paradigm_id: 1,
        racial_paradigm_level: 5,
        tech_competency: 5,
        required_discipline_ids: vec![],
        name: name.to_string(),
    }
}

fn blueprint(id: i32, discipline: i32, product: i32) -> Blueprint {
    Blueprint {
        blueprint_id: id,
        discipline_id: Some(discipline),
        is_alloy: false,
        product_id: Some(product),
        quantity: 1,
        requires_elementary_components: false,
        component_sets: vec![],
    }
}

fn catalog() -> CraftingCatalog {
    let research = ItemFlags::CRAFT_RESEARCH | ItemFlags::CRAFT_REV_ENG;
    CraftingCatalog::from_rows(
        [discipline(21, "Biochemistry"), discipline(22, "Genetics")],
        [
            blueprint(1, 21, ITEM),
            blueprint(7, 22, ITEM),
            blueprint(9, 21, PLAIN),
        ],
        [],
        [
            (ITEM, attrs(research, Some(1), 20, &[21, 22])),
            (KICKER_OWN, attrs(ItemFlags::KICKER, Some(1), 33, &[21])),
            (KICKER_4, attrs(ItemFlags::KICKER, Some(4), 33, &[40])),
            (KICKER_2, attrs(ItemFlags::KICKER, Some(2), 33, &[78])),
            (PLAIN, attrs(ItemFlags::CAN_BE_SOLD, None, 3, &[])),
            (KICKER_NO_SCIENCE, attrs(ItemFlags::KICKER, None, 3, &[])),
            (SCIENCE_NOT_KICKER, attrs(research, Some(4), 1, &[])),
        ],
    )
}

fn held(item_id: i32, type_id: i32) -> HeldInstance {
    HeldInstance {
        item_id,
        type_id,
        container_id: 15,
        stack_size: 1,
    }
}

fn knowing(known: &[(i32, i32)]) -> CraftingState {
    let mut state = CraftingState::new();
    for &(d, e) in known {
        state.discipline_ids.push(d);
        state.set_expertise(d, e);
    }
    state
}

#[test]
fn a_researchable_item_with_kickers_of_other_sciences_is_accepted() {
    let c = catalog();
    let kickers = [held(2, KICKER_4), held(3, KICKER_2)];
    assert_eq!(check_request(&c, &held(1, ITEM), &kickers), Ok(()));
    assert_eq!(check_request(&c, &held(1, ITEM), &[]), Ok(()));
}

#[test]
fn an_item_not_flagged_researchable_is_refused() {
    let c = catalog();
    assert_eq!(
        check_request(&c, &held(1, PLAIN), &[]),
        Err(CraftReject::NotResearchable {
            item_id: 1,
            type_id: PLAIN
        })
    );
    // A design the catalog does not have is not researchable either.
    assert_eq!(
        check_request(&c, &held(1, 424_242), &[]),
        Err(CraftReject::NotResearchable {
            item_id: 1,
            type_id: 424_242
        })
    );
}

#[test]
fn a_kicker_must_be_flagged_and_carry_a_science() {
    let c = catalog();
    for type_id in [PLAIN, KICKER_NO_SCIENCE, SCIENCE_NOT_KICKER] {
        assert_eq!(
            check_request(&c, &held(1, ITEM), &[held(2, type_id)]),
            Err(CraftReject::NotKicker {
                item_id: 2,
                type_id
            }),
            "{type_id}"
        );
    }
}

#[test]
fn a_kicker_of_the_items_own_science_is_refused() {
    assert_eq!(
        check_request(&catalog(), &held(1, ITEM), &[held(2, KICKER_OWN)]),
        Err(CraftReject::KickerSameScience {
            item_id: 2,
            type_id: KICKER_OWN,
            applied_science_id: 1
        })
    );
}

#[test]
fn a_second_kicker_of_one_science_is_refused() {
    let kickers = [held(2, KICKER_4), held(3, KICKER_2), held(4, KICKER_4)];
    assert_eq!(
        check_request(&catalog(), &held(1, ITEM), &kickers),
        Err(CraftReject::KickerDuplicateScience {
            item_id: 4,
            type_id: KICKER_4,
            applied_science_id: 4
        })
    );
    // The same instance named twice is the same science twice.
    let twice = [held(2, KICKER_4), held(2, KICKER_4)];
    assert!(matches!(
        check_request(&catalog(), &held(1, ITEM), &twice),
        Err(CraftReject::KickerDuplicateScience { item_id: 2, .. })
    ));
}

#[test]
fn only_known_disciplines_strictly_between_zero_and_the_tech_competency_are_eligible() {
    let item = catalog().items[&ITEM].clone();
    // 21 at 0 (the zero-expertise case) and 22 at 20 (= tc) are both out.
    let state = knowing(&[(21, 0), (22, 20)]);
    let r = roll(&item, &state, 0, &mut ScriptedRng::new(vec![0.0]));
    assert!(r.eligible.is_empty());
    assert_eq!((r.discipline_id, r.chance, r.roll), (None, None, None));
    assert!(!r.success);

    // An expertise row for a discipline not in the known list is ignored.
    let mut stray = CraftingState::new();
    stray.set_expertise(21, 10);
    let r = roll(&item, &stray, 0, &mut ScriptedRng::new(vec![0.0]));
    assert!(r.eligible.is_empty());

    let state = knowing(&[(21, 1), (22, 19), (40, 5)]);
    let r = roll(&item, &state, 0, &mut ScriptedRng::new(vec![0.0, 0.0]));
    assert_eq!(r.eligible, vec![21, 22]);
}

/// The request and the completion transaction refuse a research with no
/// eligible discipline, by the same rule the roll uses, and report the
/// values compared: the item's science, tech competency and disciplines,
/// and every discipline the player knows with its expertise.
#[test]
fn a_research_with_no_eligible_discipline_is_refused_with_the_values_compared() {
    let c = catalog();
    let item = researched_item(&held(7, ITEM), &c.items[&ITEM]);
    assert_eq!(
        (
            item.applied_science_id,
            item.tech_comp,
            &item.discipline_ids[..]
        ),
        (Some(1), 20, &[21, 22][..])
    );
    let refused = |known: &[(i32, i32)]| CraftReject::NoEligibleDiscipline {
        item_id: 7,
        type_id: ITEM,
        applied_science_id: Some(1),
        tech_comp: 20,
        item_disciplines: vec![21, 22],
        known: known.to_vec(),
    };

    // Nothing known; 21 at 0; 22 at the tech competency; only a discipline
    // the item does not list.
    assert_eq!(check_eligible(&item, &knowing(&[])), Err(refused(&[])));
    let state = knowing(&[(78, 40), (21, 0), (22, 20)]);
    assert_eq!(
        check_eligible(&item, &state),
        Err(refused(&[(21, 0), (22, 20), (78, 40)])),
        "known disciplines are reported in id order"
    );
    // A known discipline with no expertise row is ineligible and reads 0.
    let mut no_row = CraftingState::new();
    no_row.discipline_ids.push(21);
    assert_eq!(check_eligible(&item, &no_row), Err(refused(&[(21, 0)])));

    // One eligible discipline is enough, and the rule agrees with the roll.
    let state = knowing(&[(21, 0), (22, 19)]);
    assert_eq!(check_eligible(&item, &state), Ok(vec![22]));
    let r = roll(
        &c.items[&ITEM],
        &state,
        0,
        &mut ScriptedRng::new(vec![0.0, 0.0]),
    );
    assert_eq!(r.eligible, vec![22]);
}

#[test]
fn the_no_eligible_discipline_refusal_reads_and_logs_its_rule() {
    let why = CraftReject::NoEligibleDiscipline {
        item_id: 7,
        type_id: ITEM,
        applied_science_id: Some(1),
        tech_comp: 20,
        item_disciplines: vec![21, 22],
        known: vec![(21, 0), (78, 40)],
    };
    assert_eq!(why.reason(), "no_eligible_discipline");
    assert_eq!(
        why.text(),
        "None of your disciplines can learn from that item: research needs one of its \
         disciplines at an expertise above 0 and below 20. Nothing was used."
    );
    assert_eq!(why.error_code(), None);
    let c = why.compared();
    assert_eq!(
        (c.item_id, c.type_id, c.applied_science_id, c.tech_comp),
        (Some(7), Some(ITEM), Some(1), Some(20))
    );
    assert_eq!(why.item_disciplines().as_deref(), Some("21,22"));
    assert_eq!(why.known_disciplines().as_deref(), Some("21:0,78:40"));
    let other = CraftReject::NotKicker {
        item_id: 1,
        type_id: 2,
    };
    assert_eq!(
        (other.item_disciplines(), other.known_disciplines()),
        (None, None)
    );
}

#[test]
fn the_discipline_is_picked_uniformly_and_the_chance_counts_kickers() {
    let item = catalog().items[&ITEM].clone();
    let state = knowing(&[(21, 10), (22, 15)]);
    // First sample picks the discipline, the second is the roll.
    let r = roll(&item, &state, 2, &mut ScriptedRng::new(vec![0.1, 0.90]));
    assert_eq!(r.discipline_id, Some(21));
    assert_eq!(r.chance, Some(100.0 - 10.0 + 10.0));
    assert_eq!(r.roll, Some(90.0));
    assert!(r.success);

    let r = roll(&item, &state, 0, &mut ScriptedRng::new(vec![0.6, 0.90]));
    assert_eq!(r.discipline_id, Some(22));
    assert_eq!(r.chance, Some(85.0));
    assert!(!r.success, "90 is not below 85");

    // The top of the unit interval still picks the last discipline, never
    // past the end.
    let r = roll(&item, &state, 0, &mut ScriptedRng::new(vec![1.0, 0.0]));
    assert_eq!(r.discipline_id, Some(22));
}

#[test]
fn a_success_teaches_the_blueprints_of_known_disciplines_only() {
    let c = catalog();
    let mut state = knowing(&[(21, 10)]);
    assert_eq!(blueprints_taught(&c, &state, ITEM), vec![(1, 21)]);
    state.blueprint_ids = vec![1];
    assert!(blueprints_taught(&c, &state, ITEM).is_empty());
    let state = knowing(&[(21, 10), (22, 3)]);
    assert_eq!(blueprints_taught(&c, &state, ITEM), vec![(1, 21), (7, 22)]);
    assert!(blueprints_taught(&c, &knowing(&[]), ITEM).is_empty());
}

#[test]
fn the_result_line_says_what_happened() {
    let c = catalog();
    let failed = ResearchRoll {
        eligible: vec![21],
        discipline_id: Some(21),
        chance: Some(90.0),
        roll: Some(95.0),
        success: false,
    };
    assert_eq!(
        result_line(&c, &failed, &CraftApplied::default()),
        "Research complete, but no expertise was gained."
    );
    let applied = CraftApplied {
        expertise: vec![ExpertiseChange {
            discipline_id: 21,
            before: 10,
            after: 15,
        }],
        blueprints: Some(BlueprintsLearned {
            taught: vec![1],
            known_before: 0,
            blueprint_ids: vec![1],
        }),
        ..CraftApplied::default()
    };
    let won = ResearchRoll {
        success: true,
        roll: Some(5.0),
        ..failed
    };
    assert_eq!(
        result_line(&c, &won, &applied),
        "Research succeeded: Biochemistry expertise increased to 15. You learned 1 new blueprint."
    );
}

#[test]
fn research_reasons_are_the_documented_labels() {
    let reasons = [
        (
            CraftReject::NotResearchable {
                item_id: 1,
                type_id: 2,
            },
            "not_researchable",
        ),
        (
            CraftReject::NotKicker {
                item_id: 1,
                type_id: 2,
            },
            "not_kicker",
        ),
        (
            CraftReject::KickerSameScience {
                item_id: 1,
                type_id: 2,
                applied_science_id: 3,
            },
            "kicker_same_science",
        ),
        (
            CraftReject::KickerDuplicateScience {
                item_id: 1,
                type_id: 2,
                applied_science_id: 3,
            },
            "kicker_duplicate_science",
        ),
        (
            CraftReject::NotReverseEngineerable {
                item_id: 1,
                type_id: 2,
            },
            "not_reverse_engineerable",
        ),
        (
            CraftReject::NoBlueprintForItem {
                item_id: 1,
                type_id: 2,
            },
            "no_blueprint_for_item",
        ),
    ];
    for (why, label) in reasons {
        assert_eq!(why.reason(), label);
        assert!(why.text().ends_with("Nothing was used."), "{}", why.text());
        // A string continuation that lost its backslash leaves a run of
        // spaces in the middle of the line.
        assert!(!why.text().contains("  "), "{:?}", why.text());
        assert_eq!(why.error_code(), None);
        let c = why.compared();
        assert_eq!((c.item_id, c.type_id), (Some(1), Some(2)));
    }
    let c = CraftReject::KickerSameScience {
        item_id: 1,
        type_id: 2,
        applied_science_id: 3,
    }
    .compared();
    assert_eq!(c.applied_science_id, Some(3));
}
