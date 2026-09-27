//! Byte-exact tests for the crafting client-method payloads.

use super::*;

/// 112: one INT32, little-endian.
#[test]
fn crafting_respec_prompt_is_one_le_int32() {
    assert_eq!(crafting_respec_prompt_args(1000), [0xE8, 0x03, 0x00, 0x00]);
    assert_eq!(crafting_respec_prompt_args(-1), [0xFF; 4]);
}

/// 136 (re-exported): disciplineSeqId then expertise.
#[test]
fn update_discipline_is_id_then_expertise() {
    assert_eq!(
        update_discipline_args(21, 50),
        [0x15, 0x00, 0x00, 0x00, 0x32, 0x00, 0x00, 0x00]
    );
}

/// The ASP property: `onEntityProperty` propId 2 (the enumerations.xml
/// value), then the total, both INT32 LE.
#[test]
fn asp_property_is_prop_2_then_total() {
    assert_eq!(GENERICPROPERTY_APPLIED_SCIENCE_POINTS, 2);
    assert_eq!(
        applied_science_points_property_args(7),
        [0x02, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00]
    );
}

/// 137 has no arguments.
#[test]
fn discipline_respec_has_no_arguments() {
    assert!(discipline_respec_args().is_empty());
}

/// 138: INT32 paradigm id then one INT8 level, five bytes. A serializer that
/// widened the level to INT32 would send nine and shift every later method
/// in the bundle.
#[test]
fn racial_paradigm_level_is_int32_then_int8() {
    assert_eq!(
        racial_paradigm_level_args(3, 7),
        [0x03, 0x00, 0x00, 0x00, 0x07]
    );
    assert_eq!(
        racial_paradigm_level_args(0x0102_0304, -1),
        [0x04, 0x03, 0x02, 0x01, 0xFF]
    );
}

/// 139: u32 count then the ids.
#[test]
fn known_crafts_is_counted_int32_array() {
    assert_eq!(known_crafts_args(&[]), [0, 0, 0, 0]);
    assert_eq!(
        known_crafts_args(&[412, 42]),
        [
            0x02, 0x00, 0x00, 0x00, //
            0x9C, 0x01, 0x00, 0x00, // 412
            0x2A, 0x00, 0x00, 0x00, // 42
        ]
    );
}

/// 140 with every list empty: eight zero counts, 32 bytes.
#[test]
fn crafting_options_empty_is_eight_zero_counts() {
    assert_eq!(
        crafting_options_args(&CraftingOptions::default()),
        [0u8; 32]
    );
}

/// 140 section and field order: crafting, research, reverseEngineering,
/// alloying (`alias.xml`), each `items` then `entities`. Every list gets a
/// distinct id, so a swapped section or field moves an id and fails.
#[test]
fn crafting_options_sections_follow_alias_xml_order() {
    let options = CraftingOptions {
        crafting: CraftingInfo {
            items: vec![1],
            entities: vec![2],
        },
        research: CraftingInfo {
            items: vec![],
            entities: vec![3, 4],
        },
        reverse_engineering: CraftingInfo {
            items: vec![5],
            entities: vec![],
        },
        alloying: CraftingInfo {
            items: vec![],
            entities: vec![0x0A0B_0C0D],
        },
    };
    let expected: Vec<u8> = [
        1u32,
        1, // crafting.items = [1]
        1,
        2, // crafting.entities = [2]
        0, // research.items = []
        2,
        3,
        4, // research.entities = [3, 4]
        1,
        5, // reverseEngineering.items = [5]
        0, // reverseEngineering.entities = []
        0, // alloying.items = []
        1,
        0x0A0B_0C0D, // alloying.entities
    ]
    .iter()
    .flat_map(|v| v.to_le_bytes())
    .collect();
    assert_eq!(crafting_options_args(&options), expected);
}

#[test]
fn verb_method_names_match_the_def() {
    let names = [
        CraftVerb::Spend { discipline_id: 1 },
        CraftVerb::Craft {
            blueprint_id: 1,
            items: vec![],
            quantity: 1,
        },
        CraftVerb::Research {
            item_id: 1,
            kickers: vec![],
        },
        CraftVerb::ReverseEngineer { item_id: 1 },
        CraftVerb::Alloy {
            blueprint_id: 1,
            current_tier_item_id: 1,
            lower_tier_items: vec![],
        },
        CraftVerb::Respec,
    ]
    .map(|v| v.method_name());
    assert_eq!(
        names,
        [
            "spendAppliedSciencePoints",
            "craft",
            "research",
            "reverseEngineer",
            "alloying",
            "respecCrafting"
        ]
    );
}
