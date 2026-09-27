//! [`CraftReject`]'s labels, lines, compared values and codes.

use super::*;

#[test]
fn not_available_text_names_the_action() {
    let text = |verb| CraftReject::not_available(&verb).text();
    assert_eq!(
        text(CraftVerb::Spend { discipline_id: 21 }),
        "Learning disciplines is not available yet."
    );
    assert_eq!(
        text(CraftVerb::Craft {
            blueprint_id: 412,
            items: vec![],
            quantity: 1
        }),
        "Crafting is not available yet."
    );
    assert_eq!(
        text(CraftVerb::Research {
            item_id: 1,
            kickers: vec![]
        }),
        "Research is not available yet."
    );
    assert_eq!(
        text(CraftVerb::ReverseEngineer { item_id: 1 }),
        "Reverse engineering is not available yet."
    );
    assert_eq!(
        text(CraftVerb::Alloy {
            blueprint_id: 42,
            current_tier_item_id: 1,
            lower_tier_items: vec![]
        }),
        "Alloying is not available yet."
    );
    assert_eq!(
        text(CraftVerb::Respec),
        "Crafting respec is not available yet."
    );
}

#[test]
fn no_station_text_names_the_verb() {
    let text = |verb| {
        CraftReject::NoStationOrTool {
            verb,
            station_mask: 0,
            tools: vec![],
        }
        .text()
    };
    assert_eq!(
        text(CraftType::Craft),
        "No crafting station or tool for crafting nearby."
    );
    assert_eq!(
        text(CraftType::Research),
        "No crafting station or tool for research nearby."
    );
    assert_eq!(
        text(CraftType::ReverseEngineering),
        "No crafting station or tool for reverse engineering nearby."
    );
    assert_eq!(
        text(CraftType::Alloying),
        "No crafting station or tool for alloying nearby."
    );
}

fn spend_reasons() -> Vec<CraftReject> {
    vec![
        CraftReject::Unavailable {
            action: "Learning disciplines",
        },
        CraftReject::UnknownDiscipline { discipline_id: 9 },
        CraftReject::DisciplineAlreadyKnown {
            discipline_id: 78,
            name: "X".into(),
        },
        CraftReject::NoAppliedSciencePoints { asp: 0 },
        CraftReject::ParadigmTooLow {
            discipline_id: 82,
            discipline: "X".into(),
            paradigm_id: 2,
            paradigm: "Human",
            required: 3,
            have: 1,
        },
        CraftReject::PrerequisiteMissing {
            discipline_id: 79,
            discipline: "X".into(),
            prerequisite_id: 78,
            prerequisite: "Y".into(),
        },
        CraftReject::PrerequisiteExpertise {
            discipline_id: 79,
            discipline: "X".into(),
            prerequisite_id: 78,
            prerequisite: "Y".into(),
            expertise: 49,
            required: 50,
        },
    ]
}

/// Only the ASP reason carries a code; every other spend reason is text
/// only.
#[test]
fn only_no_asp_maps_a_condition_code() {
    for why in spend_reasons() {
        let expected = matches!(why, CraftReject::NoAppliedSciencePoints { .. }).then_some(214);
        assert_eq!(why.error_code(), expected, "{why:?}");
    }
}

/// The reason vocabulary is the fixed label set the metric documents.
#[test]
fn reasons_are_the_documented_labels() {
    let reasons: Vec<&str> = spend_reasons().iter().map(CraftReject::reason).collect();
    assert_eq!(
        reasons,
        [
            "unavailable",
            "unknown_discipline",
            "already_known",
            "no_asp",
            "paradigm_too_low",
            "prerequisite_missing",
            "prerequisite_expertise",
        ]
    );
}

/// Each rule refusal reports the two values it compared.
#[test]
fn compared_values_name_both_sides() {
    let reasons = spend_reasons();
    assert_eq!(reasons[3].compared().asp, Some(0));
    let paradigm = reasons[4].compared();
    assert_eq!(
        (
            paradigm.paradigm_id,
            paradigm.paradigm_level,
            paradigm.required_level
        ),
        (Some(2), Some(1), Some(3))
    );
    let expertise = reasons[6].compared();
    assert_eq!(
        (
            expertise.prerequisite_id,
            expertise.prerequisite_expertise,
            expertise.required_expertise
        ),
        (Some(78), Some(49), Some(50))
    );
}
