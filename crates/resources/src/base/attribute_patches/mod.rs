//! One-attribute patches to entries the client already ships, in any cooked
//! category: an ability icon (`CookedDataAbilities`, category 2) or an error
//! string (`ErrorStrings`, category 11).
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
use std::hash::{Hash, Hasher};

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
];

/// The metadata bump for one category: hashed from every field of its
/// patches, low bit set so it is never 0 (a 0 bump would leave clients on the
/// shipped version). Same discipline as `metadata_bump`.
fn bump_for(category: u32) -> u32 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for p in ATTRIBUTE_PATCHES.iter().filter(|p| p.category == category) {
        p.element_id.hash(&mut hasher);
        p.attribute.hash(&mut hasher);
        p.value.hash(&mut hasher);
    }
    ((hasher.finish() as u32) & 0xFFFF) | 0x1
}

/// Apply [`ATTRIBUTE_PATCHES`] to the loaded categories, bump each patched
/// category's metadata once, and return the patched element ids per
/// category, sorted.
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
