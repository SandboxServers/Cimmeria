//! One-attribute patches to entries the client already ships, in any cooked
//! category: an ability icon (`CookedDataAbilities`, category 2) or an error
//! string (`ErrorStrings`, category 11). Error strings the client never
//! shipped are added whole ([`additions`]); they share this module's
//! category-11 metadata bump.
//!
//! Delivery is the same as every other override: the entry is patched in
//! memory at startup and the category's metadata is bumped by a value hashed
//! from the patches, so a client holding the shipped category resyncs it in
//! full (#840). **Never edit the PAK on disk** (see `world_info_overrides`).
//! The seed in `db/resources/` carries the same values, and a test holds the
//! two together.
//!
//! Every patch is sourced: the value is one the shipped client already uses
//! for the same thing, or text in the style of the shipped strings. The
//! reasons are on each row and in `docs/content/debug-area.md`.

use std::collections::HashMap;

mod additions;
pub use additions::{generate_error_text_xml, ErrorStringAddition, ERROR_STRING_ADDITIONS};

use crate::base::item_overrides::patch_attr;
use crate::base::resources::{category_name, CategoryData};

/// `CookedDataAbilities.pak`.
pub const CATEGORY_ABILITIES: u32 = 2;
/// `ErrorStrings.pak`.
pub const CATEGORY_ERROR_STRINGS: u32 = 11;

/// The Medkit art from `ItemIcons001.imageset` (imageset name `ItemIcon001`,
/// loaded by `TaharezLook.scheme`). It is the icon the shipped client gives
/// its health-restore items (Health Slappack 2893/4735, Plasma Burn
/// Treatment Kit 3122). The shipped imagesets have no health-heal ability
/// icon at all: `AbilityIcons001`/`002` hold one heal icon, `Heal_Focus_Heal`,
/// which the shipped Heal Focus (597) already uses on the same starter bar.
pub const MEDKIT_ICON: &str = "set:ItemIcon001 image:Medkit";

/// One attribute of one shipped cooked entry, replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttributePatch {
    pub category: u32,
    pub element_id: u32,
    /// The entry's own name, for logs (Rule 6).
    pub element_name: &'static str,
    pub attribute: &'static str,
    pub value: &'static str,
    /// Why, for the startup log and the reader.
    pub reason: &'static str,
}

/// Every patch, in category order. Each target must ship in the PAK; a
/// missing one is skipped with a warning.
pub const ATTRIBUTE_PATCHES: &[AttributePatch] = &[
    AttributePatch {
        category: CATEGORY_ABILITIES,
        element_id: 1218,
        element_name: "Medical Attention: Recuperation",
        attribute: "IconLocation",
        value: MEDKIT_ICON,
        reason: "Recuperation (a starter health heal) ships IconMissing (DA-F6)",
    },
    AttributePatch {
        category: CATEGORY_ABILITIES,
        element_id: 1646,
        element_name: "Health Heal",
        attribute: "IconLocation",
        value: MEDKIT_ICON,
        reason: "Health Heal (a starter health heal) ships IconMissing (DA-F6)",
    },
    AttributePatch {
        category: CATEGORY_ERROR_STRINGS,
        element_id: 42,
        element_name: "CONDITION_FEEDBACK_OutsideWeaponRange",
        attribute: "Text",
        value: "Your target is out of range",
        reason: "CONDITION_FEEDBACK_OutsideWeaponRange ships its moniker as its text, \
                 so the client printed the raw token (DA-F6); worded like the shipped _39 \
                 \"You do not have Line of Sight to your target\"",
    },
    // The character-creation refusals (Class Start v6 CS-08 F1). The client
    // shows the served `Text` of the code `onCharacterCreateFailed` carries
    // in its "Creation Error" prompt, and all four ship as the bare or quoted
    // moniker.
    AttributePatch {
        category: CATEGORY_ERROR_STRINGS,
        element_id: 10000,
        element_name: "ERROR_CharacterCreationNotEnoughInformation",
        attribute: "Text",
        value: "Character creation is missing some information. Please try again",
        reason: "ships its moniker as its text; sent for a short or malformed create request",
    },
    AttributePatch {
        category: CATEGORY_ERROR_STRINGS,
        element_id: 10001,
        element_name: "ERROR_CharacterCreationInvalidCharacterType",
        attribute: "Text",
        value: "That character type cannot be created",
        reason: "ships its quoted moniker as its text; sent for an unknown or unusable char_def",
    },
    AttributePatch {
        category: CATEGORY_ERROR_STRINGS,
        element_id: 10002,
        element_name: "ERROR_CharacterCreationInvalidSkinColor",
        attribute: "Text",
        value: "That skin color is not available",
        reason: "ships its quoted moniker as its text; sent for a skin tint outside 0-15",
    },
    AttributePatch {
        category: CATEGORY_ERROR_STRINGS,
        element_id: 10003,
        element_name: "ERROR_CharacterCreationUnspecifiedError",
        attribute: "Text",
        value: "The character could not be created. Please try again",
        reason: "ships its quoted moniker as its text; sent for a bad visual choice or a \
                 database error",
    },
];

/// The metadata bump for one category: a 32-bit FNV-1a hash of every patch
/// in it (element id, attribute, value), low bit set so it is never 0 (a 0
/// bump would leave clients on the shipped version).
///
/// FNV-1a rather than `DefaultHasher` (which `metadata_bump` still uses):
/// std may change `DefaultHasher`'s algorithm between releases, and a
/// toolchain bump would then silently move the version and resync every
/// client again. 32 bits rather than 16 makes it unlikely that a later edit
/// lands on the same bump and leaves clients holding the stale entry.
fn bump_for(category: u32) -> u32 {
    const FNV_OFFSET: u32 = 0x811c_9dc5;
    const FNV_PRIME: u32 = 0x0100_0193;
    let mut hash = FNV_OFFSET;
    let mut feed = |bytes: &[u8]| {
        for &b in bytes {
            hash ^= u32::from(b);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    };
    for p in ATTRIBUTE_PATCHES.iter().filter(|p| p.category == category) {
        feed(&p.element_id.to_le_bytes());
        feed(p.attribute.as_bytes());
        feed(&[0]);
        feed(p.value.as_bytes());
        feed(&[0]);
    }
    if category == CATEGORY_ERROR_STRINGS {
        for a in ERROR_STRING_ADDITIONS {
            feed(&generate_error_text_xml(a));
            feed(&[0]);
        }
    }
    hash | 0x1
}

/// Apply [`ATTRIBUTE_PATCHES`] and [`ERROR_STRING_ADDITIONS`] to the loaded
/// categories, bump each changed category's metadata once, and return the
/// changed element ids per category, sorted.
pub(crate) fn apply_attribute_patches(
    categories: &mut HashMap<u32, CategoryData>,
) -> HashMap<u32, Vec<u32>> {
    let mut applied: HashMap<u32, Vec<u32>> = HashMap::new();
    for patch in ATTRIBUTE_PATCHES {
        let category_label = category_name(patch.category);
        let Some(category) = categories.get_mut(&patch.category) else {
            tracing::warn!(
                category = patch.category,
                category_name = category_label,
                element_id = patch.element_id,
                element_name = patch.element_name,
                reason = "category_not_loaded",
                "cooked attribute patch skipped: its category did not load"
            );
            continue;
        };
        let patched = category
            .elements
            .get(&patch.element_id)
            .and_then(|xml| std::str::from_utf8(xml).ok())
            .and_then(|xml| patch_attr(xml, patch.attribute, patch.value));
        let Some(patched) = patched else {
            tracing::warn!(
                category = patch.category,
                category_name = category_label,
                element_id = patch.element_id,
                element_name = patch.element_name,
                attribute = patch.attribute,
                reason = "entry_or_attribute_missing",
                "cooked attribute patch skipped: the shipped entry or its attribute is missing"
            );
            continue;
        };
        category
            .elements
            .insert(patch.element_id, patched.into_bytes());
        applied
            .entry(patch.category)
            .or_default()
            .push(patch.element_id);
        tracing::info!(
            category = patch.category,
            category_name = category_label,
            element_id = patch.element_id,
            element_name = patch.element_name,
            attribute = patch.attribute,
            value = patch.value,
            reason = patch.reason,
            "Applied Cimmeria cooked attribute patch"
        );
    }
    if let Some(added) = additions::apply(categories) {
        if !added.is_empty() {
            applied
                .entry(CATEGORY_ERROR_STRINGS)
                .or_default()
                .extend(added);
        }
    }
    for (&category_id, ids) in &mut applied {
        ids.sort_unstable();
        if let Some(category) = categories.get_mut(&category_id) {
            let bump = bump_for(category_id);
            category.metadata = category.metadata.wrapping_add(bump);
            tracing::info!(
                category = category_id,
                category_name = category_name(category_id),
                count = ids.len(),
                bump,
                bumped_metadata = category.metadata,
                "Cimmeria cooked attribute patches applied; metadata bumped"
            );
        }
    }
    applied
}

#[cfg(test)]
mod tests;
