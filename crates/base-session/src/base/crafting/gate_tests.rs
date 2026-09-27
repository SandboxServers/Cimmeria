//! Tests for the gate's pure rules. The dispatcher-level guard (a forged
//! request refused with feedback) is `cell_dispatch::tests_dispatch_arms::
//! crafting_gate` in `cimmeria-base-world-entry`.

use super::*;
use crate::base::crafting::tools::ToolSpec;

fn discipline(id: i32, applied_science_id: i32, tech_competency: i32) -> Discipline {
    Discipline {
        discipline_id: id,
        applied_science_id,
        racial_paradigm_id: 1,
        racial_paradigm_level: 1,
        tech_competency,
        required_discipline_ids: vec![],
        name: format!("d{id}"),
    }
}

fn tool(applied_science_id: i32, tech_comp: i32) -> HeldTool {
    HeldTool {
        instance_id: 20_001,
        spec: ToolSpec {
            applied_science_id,
            tech_comp,
        },
    }
}

#[test]
fn only_the_four_craft_family_verbs_are_gated() {
    assert_eq!(
        gated_verb(&CraftVerb::Craft {
            blueprint_id: 1,
            items: vec![],
            quantity: 1
        }),
        Some(CraftType::Craft)
    );
    assert_eq!(
        gated_verb(&CraftVerb::Research {
            item_id: 1,
            kickers: vec![]
        }),
        Some(CraftType::Research)
    );
    assert_eq!(
        gated_verb(&CraftVerb::ReverseEngineer { item_id: 1 }),
        Some(CraftType::ReverseEngineering)
    );
    assert_eq!(
        gated_verb(&CraftVerb::Alloy {
            blueprint_id: 1,
            current_tier_item_id: 1,
            lower_tier_items: vec![]
        }),
        Some(CraftType::Alloying)
    );
    assert_eq!(gated_verb(&CraftVerb::Spend { discipline_id: 1 }), None);
    assert_eq!(gated_verb(&CraftVerb::Respec), None);
}

/// Research and reverse engineering pass when a tool covers **any** of the
/// item's disciplines.
#[test]
fn a_tool_covering_one_of_several_disciplines_is_enough() {
    let bio = discipline(21, 1, 10);
    let power = discipline(61, 3, 10);
    let tools = [tool(3, 10)];
    assert!(tools_cover(CraftType::Research, &tools, &[&bio, &power]));
    assert!(tools_cover(
        CraftType::ReverseEngineering,
        &tools,
        &[&bio, &power]
    ));
    assert!(!tools_cover(CraftType::Research, &tools, &[&bio]));
}

#[test]
fn tech_comp_and_science_bound_the_tool() {
    let d = discipline(21, 1, 25);
    assert!(tools_cover(CraftType::Craft, &[tool(1, 25)], &[&d]));
    assert!(!tools_cover(CraftType::Craft, &[tool(1, 20)], &[&d]));
    assert!(!tools_cover(CraftType::Craft, &[tool(2, 55)], &[&d]));
    assert!(tools_cover(
        CraftType::Craft,
        &[tool(1, 20), tool(1, 55)],
        &[&d]
    ));
}

/// Alloying needs a station: no tool covers it.
#[test]
fn no_tool_covers_alloying() {
    let d = discipline(21, 1, 1);
    assert!(!tools_cover(CraftType::Alloying, &[tool(1, 55)], &[&d]));
}

#[test]
fn nothing_to_cover_means_not_covered() {
    assert!(!tools_cover(CraftType::Craft, &[tool(1, 55)], &[]));
    assert!(!tools_cover(
        CraftType::Craft,
        &[],
        &[&discipline(21, 1, 1)]
    ));
}
