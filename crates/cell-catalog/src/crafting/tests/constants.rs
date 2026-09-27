//! Pins every crafting constant to `entities/defs/enumerations.xml`.

use super::super::*;

fn enumerations_xml() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../entities/defs/enumerations.xml");
    std::fs::read_to_string(&path).expect("read enumerations.xml")
}

/// The value of token `name` inside the `<enum_name>` block. The defs pad
/// names and values with spaces, so both are trimmed.
fn token(xml: &str, enum_name: &str, name: &str) -> i64 {
    let open = format!("<{enum_name}>");
    let close = format!("</{enum_name}>");
    let start = xml
        .find(&open)
        .unwrap_or_else(|| panic!("{enum_name} block"));
    let end = start + xml[start..].find(&close).expect("closing tag");
    let block = &xml[start..end];
    let mut found = None;
    for chunk in block.split("<Token>").skip(1) {
        let n = &chunk[chunk.find("<Name>").expect("<Name>") + 6..chunk.find("</Name>").unwrap()];
        if n.trim() != name {
            continue;
        }
        let v =
            &chunk[chunk.find("<Value>").expect("<Value>") + 7..chunk.find("</Value>").unwrap()];
        assert!(found.is_none(), "{enum_name}::{name} is declared twice");
        found = Some(v.trim().parse::<i64>().expect("numeric value"));
    }
    found.unwrap_or_else(|| panic!("{enum_name}::{name} not in enumerations.xml"))
}

#[test]
fn every_constant_matches_enumerations_xml() {
    let xml = enumerations_xml();

    let craft_types = [
        (CraftType::Craft, "CRAFT_TYPE_Craft"),
        (CraftType::Research, "CRAFT_TYPE_Research"),
        (
            CraftType::ReverseEngineering,
            "CRAFT_TYPE_ReverseEngineering",
        ),
        (CraftType::Alloying, "CRAFT_TYPE_Alloying"),
    ];
    for (t, name) in craft_types {
        assert_eq!(
            i64::from(t.bit()),
            token(&xml, "ECraftTypeFlags", name),
            "{name}"
        );
    }

    let item_flags = [
        (
            ItemFlags::MINIGAME_INSTRUMENT,
            "ITEM_FLAG_MinigameInstrument",
        ),
        (
            ItemFlags::MINIGAME_CONSUMABLE,
            "ITEM_FLAG_MinigameConsumable",
        ),
        (ItemFlags::BIND_ON_ACQUIRE, "ITEM_FLAG_BindOnAcquire"),
        (ItemFlags::BIND_ON_EQUIP, "ITEM_FLAG_BindOnEquip"),
        (ItemFlags::NOT_RESEARCHABLE, "ITEM_FLAG_NotResearchable"),
        (ItemFlags::KICKER, "ITEM_FLAG_Kicker"),
        (ItemFlags::CRAFT_CRAFT, "ITEM_FLAG_Craft_Craft"),
        (ItemFlags::CRAFT_RESEARCH, "ITEM_FLAG_Craft_Research"),
        (ItemFlags::CRAFT_REV_ENG, "ITEM_FLAG_Craft_RevEng"),
        (ItemFlags::CRAFT_ALLOYING, "ITEM_FLAG_Craft_Alloying"),
        (ItemFlags::CAN_BE_SOLD, "ITEM_FLAG_CanBeSold"),
        (ItemFlags::CAN_BE_DELETED, "ITEM_FLAG_CanBeDeleted"),
        (ItemFlags::UNIQUE, "ITEM_FLAG_Unique"),
        (ItemFlags::MUST_EQUIP_TO_USE, "ITEM_FLAG_MustEquipToUse"),
        (ItemFlags::DESTROY_ON_CLEAR, "ITEM_FLAG_DestroyOnClear"),
        (
            ItemFlags::ELEMENTARY_COMPONENT,
            "ITEM_FLAG_ElementaryComponent",
        ),
    ];
    for (v, name) in item_flags {
        assert_eq!(i64::from(v), token(&xml, "EItemFlag", name), "{name}");
    }

    let entity_flags = [
        (ENTITYFLAG_CRAFT_CRAFT, "ENTITYFLAG_Craft_Craft"),
        (ENTITYFLAG_CRAFT_RESEARCH, "ENTITYFLAG_Craft_Research"),
        (ENTITYFLAG_CRAFT_REV_ENG, "ENTITYFLAG_Craft_RevEng"),
        (ENTITYFLAG_CRAFT_ALLOYING, "ENTITYFLAG_Craft_Alloying"),
    ];
    for (v, name) in entity_flags {
        assert_eq!(i64::from(v), token(&xml, "EEntityFlags", name), "{name}");
    }

    assert_eq!(
        i64::from(TIMER_CRAFT_INDUCTION),
        token(&xml, "ETimerUpdateType", "CraftInductionTimer")
    );

    let qualities = [
        (ItemQuality::Poor, "ITEM_QUALITY_Poor"),
        (ItemQuality::Normal, "ITEM_QUALITY_Normal"),
        (ItemQuality::Good, "ITEM_QUALITY_Good"),
        (ItemQuality::Great, "ITEM_QUALITY_Great"),
        (ItemQuality::Fantastic, "ITEM_QUALITY_Fantastic"),
    ];
    for (q, name) in qualities {
        assert_eq!(
            i64::from(q.value()),
            token(&xml, "EItemQuality", name),
            "{name}"
        );
        assert_eq!(ItemQuality::from_db_label(name), Some(q), "DB label {name}");
    }

    let feedback = [
        (
            CONDITION_FEEDBACK_ENOUGH_APPLIED_SCIENCE_POINTS,
            "CONDITION_FEEDBACK_EnoughAppliedSciencePoints",
        ),
        (
            CONDITION_FEEDBACK_NOT_ENOUGH_APPLIED_SCIENCE_POINTS,
            "CONDITION_FEEDBACK_NotEnoughAppliedSciencePoints",
        ),
        (
            CONDITION_FEEDBACK_EXPERTISE_VALUE_NOT_EQUAL,
            "CONDITION_FEEDBACK_ExpertiseValueNotEqual",
        ),
        (
            CONDITION_FEEDBACK_EXPERTISE_VALUE_EQUAL,
            "CONDITION_FEEDBACK_ExpertiseValueEqual",
        ),
        (
            CONDITION_FEEDBACK_EXPERTISE_VALUE_GREATER_THAN,
            "CONDITION_FEEDBACK_ExpertiseValueGreaterThan",
        ),
        (
            CONDITION_FEEDBACK_EXPERTISE_VALUE_GREATER_THAN_OR_EQUAL,
            "CONDITION_FEEDBACK_ExpertiseValueGreaterThanOrEqual",
        ),
        (
            CONDITION_FEEDBACK_EXPERTISE_VALUE_LESS_THAN_OR_EQUAL,
            "CONDITION_FEEDBACK_ExpertiseValueLessThanOrEqual",
        ),
        (
            CONDITION_FEEDBACK_EXPERTISE_VALUE_LESS_THAN,
            "CONDITION_FEEDBACK_ExpertiseValueLessThan",
        ),
        (
            CONDITION_FEEDBACK_EXPERTISE_NO_CRAFT,
            "CONDITION_FEEDBACK_ExpertiseNoCraft",
        ),
    ];
    for (v, name) in feedback {
        assert_eq!(
            i64::from(v),
            token(&xml, "EConditionHandlerFeedback", name),
            "{name}"
        );
    }
}

/// `HasCraft` / `DoesNotHaveCraft` share their values with the ability
/// pulse-check tokens. The module leaves them out on purpose; this pins the
/// reason, so a corrected def shows up here and the constants can be added.
#[test]
fn has_craft_feedback_values_collide_in_the_defs() {
    let xml = enumerations_xml();
    let e = "EConditionHandlerFeedback";
    assert_eq!(
        token(&xml, e, "CONDITION_FEEDBACK_HasCraft"),
        token(&xml, e, "CONDITION_FEEDBACK_AbilityPulseCheckFailed")
    );
    assert_eq!(
        token(&xml, e, "CONDITION_FEEDBACK_DoesNotHaveCraft"),
        token(&xml, e, "CONDITION_FEEDBACK_AbilityPulseCheckPassed")
    );
}

#[test]
fn craft_type_bits_masks_and_station_flags() {
    assert_eq!(CraftType::ALL.map(CraftType::bit), [1, 2, 4, 8]);
    for t in CraftType::ALL {
        assert_eq!(CraftType::try_from(t.bit()), Ok(t));
        assert!(t.allowed_by(0x0F));
        assert!(!t.allowed_by(0));
    }
    assert!(CraftType::Research.allowed_by(0b0010));
    assert!(!CraftType::Craft.allowed_by(0b0010));
    assert_eq!(CraftType::try_from(3), Err(3), "a mask is not a verb");
    assert_eq!(CraftType::try_from(0), Err(0));
    assert_eq!(
        CraftType::ALL.map(CraftType::entity_flag),
        [
            ENTITYFLAG_CRAFT_CRAFT,
            ENTITYFLAG_CRAFT_RESEARCH,
            ENTITYFLAG_CRAFT_REV_ENG,
            ENTITYFLAG_CRAFT_ALLOYING
        ]
    );
}

#[test]
fn item_flag_predicates() {
    let f = ItemFlags(ItemFlags::CRAFT_RESEARCH | ItemFlags::ELEMENTARY_COMPONENT);
    assert!(f.is_researchable());
    assert!(!f.is_reverse_engineerable());
    assert!(!f.is_kicker());
    assert!(ItemFlags(ItemFlags::KICKER).is_kicker());
    assert!(ItemFlags(ItemFlags::CRAFT_REV_ENG).is_reverse_engineerable());
    assert_eq!(ItemQuality::from_db_label("ITEM_QUALITY_Bogus"), None);
}
