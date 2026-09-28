//! Pins the SGWPet wire constants against the entity definitions they come
//! from, not against copies of the same literals.
//!
//! The client-method indices come from the shared BigWorld flattener,
//! `mercury::def_conformance::flatten` (#801), which also checks every other
//! method-index constant in the workspace. It is checked against SGWMob's
//! 27/28 and SGWBeing's 26 before it is trusted for SGWPet. The
//! generic-property id and the flag bits are read from
//! `entities/defs/enumerations.xml`.

use super::pet::*;
use crate::mercury::def_conformance::flatten::{flatten, Section};

const ENTITIES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../entities");

/// `text` with every `<!-- ... -->` span removed.
fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        rest = rest[start..]
            .find("-->")
            .map_or("", |end| &rest[start + end + 3..]);
    }
    out.push_str(rest);
    out
}

/// Text of every `<tag>...</tag>` inside `body`, trimmed.
fn tag_texts(body: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    body.split(&open)
        .skip(1)
        .filter_map(|s| s.split(&close).next())
        .map(|s| s.trim().to_string())
        .collect()
}

fn read_def(path: &str) -> String {
    strip_comments(&std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {path}: {e}")))
}

/// The flattened client-method table of entity `name`: index = position.
pub(super) fn flattened_client_methods(name: &str) -> Vec<String> {
    flatten(name, Section::Client)
}

pub(super) fn index_of(table: &[String], method: &str) -> u16 {
    let i = table
        .iter()
        .position(|m| m == method)
        .unwrap_or_else(|| panic!("{method} not in the flattened table {table:?}"));
    u16::try_from(i).unwrap()
}

/// `<Value>` of the enumeration token named `token` in `enumerations.xml`.
pub(super) fn enum_value(token: &str) -> u64 {
    let xml = read_def(&format!("{ENTITIES}/defs/enumerations.xml"));
    for entry in xml.split("<Token>").skip(1) {
        let name = tag_texts(entry, "Name");
        if name.first().map(String::as_str) == Some(token) {
            return tag_texts(entry, "Value")[0].parse().unwrap();
        }
    }
    panic!("{token} not in enumerations.xml");
}

/// The flattener agrees with the indices already verified elsewhere
/// (the SGWMob and SGWPlayer dispatch tables), so a bug in it cannot pass
/// the SGWPet check below by accident.
#[test]
fn flattener_reproduces_the_known_mob_and_being_indices() {
    let mob = flattened_client_methods("SGWMob");
    assert_eq!(mob.len(), 29, "SGWMob flattens to 0-28: {mob:?}");
    assert_eq!(
        index_of(&mob, "onAggressionOverrideUpdate"),
        crate::mercury::method_idx::ON_AGGRESSION_OVERRIDE_UPDATE
    );
    assert_eq!(
        index_of(&mob, "onAggressionOverrideCleared"),
        crate::mercury::method_idx::ON_AGGRESSION_OVERRIDE_CLEARED
    );
    assert_eq!(
        index_of(&mob, "BeingAppearance"),
        super::being::BEING_APPEARANCE
    );
}

/// The three SGWPet client-method constants are the positions of those
/// methods in SGWPet's flattened table (SGWMob 0-28 plus its own three,
/// total 32 = entity-property-sync App. B). PT-E1 confirmed the same values
/// against the client.
#[test]
fn pet_method_indices_match_the_flattened_sgwpet_def() {
    let pet = flattened_client_methods("SGWPet");
    assert_eq!(pet.len(), 32, "SGWPet flattens to 32 methods: {pet:?}");
    assert_eq!(index_of(&pet, "onPetAbilityList"), ON_PET_ABILITY_LIST);
    assert_eq!(index_of(&pet, "onPetStanceList"), ON_PET_STANCE_LIST);
    assert_eq!(index_of(&pet, "onPetStanceUpdate"), ON_PET_STANCE_UPDATE);
}

#[test]
fn pet_owner_property_id_matches_enumerations_xml() {
    assert_eq!(
        enum_value("GENERICPROPERTY_PetOwnerId"),
        GENERICPROPERTY_PET_OWNER_ID as u64
    );
}

#[test]
fn pet_flag_values_match_enumerations_xml() {
    for (token, value) in [
        ("ENTITYFLAG_NoPetLeveling", ENTITYFLAG_NO_PET_LEVELING),
        ("ENTITYFLAG_NoPetTargeting", ENTITYFLAG_NO_PET_TARGETING),
        (
            "ENTITYFLAG_DespawnOnOwnerLeash",
            ENTITYFLAG_DESPAWN_ON_OWNER_LEASH,
        ),
        ("ENTITYFLAG_NoPassive", ENTITYFLAG_NO_PASSIVE),
        ("ENTITYFLAG_NoDefensive", ENTITYFLAG_NO_DEFENSIVE),
        ("ENTITYFLAG_NoAggressive", ENTITYFLAG_NO_AGGRESSIVE),
        ("ENTITYFLAG_DetectionPet", ENTITYFLAG_DETECTION_PET),
        ("ENTITYFLAG_Pet", ENTITYFLAG_PET),
        (
            "ENTITYFLAG_DespawnOnLeashFromOwner",
            ENTITYFLAG_DESPAWN_ON_LEASH_FROM_OWNER,
        ),
        (
            "ENTITYFLAG_PetUseOwnFaction",
            ENTITYFLAG_PET_USE_OWN_FACTION,
        ),
        (
            "ENTITYFLAG_PetWaitToDespawn",
            ENTITYFLAG_PET_WAIT_TO_DESPAWN,
        ),
    ] {
        assert_eq!(enum_value(token), value, "{token}");
    }
}
