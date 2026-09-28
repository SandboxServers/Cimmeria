//! Cimmeria's in-memory overrides, applied per category by
//! [`ResourceCache::load_all`] after the PAKs load.
//!
//! Each `apply_*` patches or adds entries in one category, bumps that
//! category's `MetaData` by a content-derived value, and returns the
//! overridden element ids. The bump is what makes a client holding the
//! shipped category resync it in full (#840), additions included; the ids
//! only pick the log level of an element push. Split out of
//! `resources/mod.rs` along that seam.

use std::collections::HashMap;

use super::metadata_bump::{
    compute_dialog_metadata_bump, compute_item_metadata_bump, compute_metadata_bump,
    compute_sequence_metadata_bump, compute_world_info_metadata_bump,
};
use super::{
    CategoryData, ResourceCache, CATEGORY_DIALOGS, CATEGORY_ITEMS, CATEGORY_KISMET_SEQUENCES,
    CATEGORY_MISSIONS, CATEGORY_WORLD_INFO,
};

impl ResourceCache {
    /// Patch the freshly-loaded `CookedDataItems` category with
    /// Cimmeria's icon + stack-size overrides, bumping the category
    /// metadata so the client's next `versionInfoRequest` sees a
    /// fresh value and triggers the per-key invalidation handshake.
    ///
    /// Same shape as [`Self::apply_mission_overrides`] — the cooked
    /// data wire path is category-agnostic; only the per-category
    /// override registry differs.
    pub(super) fn apply_item_overrides(
        categories: &mut HashMap<u32, CategoryData>,
    ) -> HashMap<u32, Vec<u32>> {
        use crate::base::item_overrides::{apply_override, ITEM_OVERRIDES};

        let mut overridden: HashMap<u32, Vec<u32>> = HashMap::new();
        let Some(items) = categories.get_mut(&CATEGORY_ITEMS) else {
            tracing::warn!(
                category = CATEGORY_ITEMS,
                "CookedDataItems not loaded; skipping item overrides"
            );
            return overridden;
        };

        let mut applied: Vec<u32> = Vec::with_capacity(ITEM_OVERRIDES.len());
        for ov in ITEM_OVERRIDES {
            let Some(original) = items.elements.get(&ov.item_id) else {
                tracing::warn!(
                    item_id = ov.item_id,
                    "item override skipped: entry not present in PAK",
                );
                continue;
            };
            match apply_override(original, ov) {
                Some(patched) => {
                    items.elements.insert(ov.item_id, patched);
                    applied.push(ov.item_id);
                    tracing::info!(
                        item_id = ov.item_id,
                        new_icon = ?ov.new_icon_location,
                        new_max_stack_size = ?ov.new_max_stack_size,
                        "Applied Cimmeria item override",
                    );
                }
                None => {
                    tracing::warn!(
                        item_id = ov.item_id,
                        "item override skipped: XML shape did not match — keeping unpatched entry",
                    );
                }
            }
        }

        if !applied.is_empty() {
            let bump = compute_item_metadata_bump(ITEM_OVERRIDES);
            items.metadata = items.metadata.wrapping_add(bump);
            applied.sort_unstable();
            tracing::info!(
                category = CATEGORY_ITEMS,
                count = applied.len(),
                bump,
                bumped_metadata = items.metadata,
                "Cimmeria item overrides applied; metadata bumped",
            );
            overridden.insert(CATEGORY_ITEMS, applied);
        }

        overridden
    }

    /// Patch the freshly-loaded `CookedDataDialogs` category with
    /// Cimmeria's dialog overrides, bumping the category metadata so the
    /// client's next `versionInfoRequest` triggers the per-key
    /// invalidation handshake.
    ///
    /// Two kinds run here, in this order:
    ///
    /// 1. **Full regenerations** (`DIALOG_OVERRIDES`). Each emits a whole
    ///    `<COOKED_DIALOG>` from Rust-authored text, so it works whether
    ///    or not the dialog id was present in the PAK: a corrected
    ///    existing dialog (Frost's 3995) and a brand-new one (the Guard
    ///    corpse's 3996) are both just an `elements.insert`. There's no
    ///    "entry not present" skip and no shape failure — generation is
    ///    infallible.
    /// 2. **Patches** (`DIALOG_PATCH_TABLES`). Each transforms the entry
    ///    the client already shipped, so it CAN fail: a missing dialog
    ///    id, a missing `OnlyOn` screen, or a cooked shape that no longer
    ///    parses all warn and skip, leaving the canonical bytes intact.
    ///    See [`crate::base::dialog_overrides::apply_dialog_patches`].
    ///
    /// Regenerations run first so a patch could in principle transform a
    /// regenerated entry; nothing does that today, and a unit test keeps
    /// the two tables disjoint.
    pub(super) fn apply_dialog_overrides(
        categories: &mut HashMap<u32, CategoryData>,
    ) -> HashMap<u32, Vec<u32>> {
        use crate::base::dialog_overrides::{
            apply_dialog_patches, generate_dialog_xml, no_patches_registered, DIALOG_OVERRIDES,
            DIALOG_PATCH_TABLES,
        };

        let mut overridden: HashMap<u32, Vec<u32>> = HashMap::new();
        if DIALOG_OVERRIDES.is_empty() && no_patches_registered() {
            return overridden;
        }

        let Some(dialogs) = categories.get_mut(&CATEGORY_DIALOGS) else {
            tracing::warn!(
                category = CATEGORY_DIALOGS,
                "CookedDataDialogs not loaded; skipping dialog overrides"
            );
            return overridden;
        };

        let mut applied: Vec<u32> = Vec::with_capacity(DIALOG_OVERRIDES.len());
        for ov in DIALOG_OVERRIDES {
            let was_present = dialogs.elements.contains_key(&ov.dialog_id);
            let patched = generate_dialog_xml(ov);
            dialogs.elements.insert(ov.dialog_id, patched);
            applied.push(ov.dialog_id);
            tracing::info!(
                dialog_id = ov.dialog_id,
                replaced_existing = was_present,
                "Applied Cimmeria dialog override",
            );
        }

        applied.extend(apply_dialog_patches(
            &mut dialogs.elements,
            DIALOG_PATCH_TABLES,
        ));

        // Every patch may have been skipped (all targets absent), in which
        // case nothing changed and bumping would make every client refetch
        // entries that are byte-identical to what they hold.
        if applied.is_empty() {
            return overridden;
        }

        let bump = compute_dialog_metadata_bump(DIALOG_OVERRIDES, DIALOG_PATCH_TABLES);
        dialogs.metadata = dialogs.metadata.wrapping_add(bump);
        applied.sort_unstable();
        // A dialog that is both regenerated and patched would otherwise be
        // named twice in `InvalidKeys`.
        applied.dedup();
        tracing::info!(
            category = CATEGORY_DIALOGS,
            count = applied.len(),
            bump,
            bumped_metadata = dialogs.metadata,
            "Cimmeria dialog overrides applied; metadata bumped",
        );
        overridden.insert(CATEGORY_DIALOGS, applied);

        overridden
    }

    /// Mutate the freshly-loaded `CookedDataMissions` category to include
    /// Cimmeria's added mission steps, bumping the category metadata so
    /// the client's version check sees a fresh value and triggers the
    /// full-category resync (#840).
    ///
    /// Returns the overridden-elements map keyed by category id. An entry
    /// with an empty vec is omitted.
    pub(super) fn apply_mission_overrides(
        categories: &mut HashMap<u32, CategoryData>,
    ) -> HashMap<u32, Vec<u32>> {
        use crate::base::mission_overrides::{
            apply_override, apply_step_text_override, MISSION_OVERRIDES, STEP_TEXT_OVERRIDES,
        };

        let mut overridden: HashMap<u32, Vec<u32>> = HashMap::new();

        let Some(missions) = categories.get_mut(&CATEGORY_MISSIONS) else {
            tracing::warn!(
                category = CATEGORY_MISSIONS,
                "CookedDataMissions not loaded; skipping mission overrides"
            );
            return overridden;
        };

        let mut applied: Vec<u32> =
            Vec::with_capacity(MISSION_OVERRIDES.len() + STEP_TEXT_OVERRIDES.len());
        for ov in MISSION_OVERRIDES {
            let Some(original) = missions.elements.get(&ov.mission_id) else {
                tracing::warn!(
                    mission_id = ov.mission_id,
                    "mission override skipped: entry not present in PAK",
                );
                continue;
            };
            match apply_override(original, ov) {
                Some(patched) => {
                    missions.elements.insert(ov.mission_id, patched);
                    // A mission can have multiple overrides (e.g. 622 injects
                    // both the Guard-search and equip steps); list its id once
                    // so InvalidKeys names each patched entry a single time.
                    if !applied.contains(&ov.mission_id) {
                        applied.push(ov.mission_id);
                    }
                    tracing::info!(
                        mission_id = ov.mission_id,
                        "Applied Cimmeria mission override",
                    );
                }
                None => {
                    tracing::warn!(
                        mission_id = ov.mission_id,
                        "mission override skipped: XML shape did not match — keeping unpatched entry",
                    );
                }
            }
        }

        // Step-text overrides patch existing `<StepDisplayLogText>` content
        // in-place. Run after MISSION_OVERRIDES so a patched mission XML
        // (with new injected steps) can have one of its existing step
        // captions corrected in the same pass.
        for ov in STEP_TEXT_OVERRIDES {
            let Some(original) = missions.elements.get(&ov.mission_id) else {
                tracing::warn!(
                    mission_id = ov.mission_id,
                    step_id = ov.step_id,
                    "step text override skipped: mission entry not present in PAK",
                );
                continue;
            };
            match apply_step_text_override(original, ov) {
                Some(patched) => {
                    missions.elements.insert(ov.mission_id, patched);
                    if !applied.contains(&ov.mission_id) {
                        applied.push(ov.mission_id);
                    }
                    tracing::info!(
                        mission_id = ov.mission_id,
                        step_id = ov.step_id,
                        "Applied Cimmeria step text override",
                    );
                }
                None => {
                    tracing::warn!(
                        mission_id = ov.mission_id,
                        step_id = ov.step_id,
                        "step text override skipped: XML shape did not match — keeping unpatched entry",
                    );
                }
            }
        }

        if !applied.is_empty() {
            let bump = compute_metadata_bump(MISSION_OVERRIDES, STEP_TEXT_OVERRIDES);
            missions.metadata = missions.metadata.wrapping_add(bump);
            applied.sort_unstable();
            tracing::info!(
                category = CATEGORY_MISSIONS,
                count = applied.len(),
                bump,
                bumped_metadata = missions.metadata,
                "Cimmeria mission overrides applied; metadata bumped",
            );
            overridden.insert(CATEGORY_MISSIONS, applied);
        }

        overridden
    }

    /// Add Cimmeria's Kismet sequences to the freshly-loaded
    /// `CookedDataKismetSeqEvent` category and bump its metadata so a client's
    /// next `versionInfoRequest` resyncs the category, additions included.
    ///
    /// Before #840 this was also what kept category 1 off the destructive
    /// path: with no override list, a version mismatch answered
    /// `invalidate_all = true` and pushed nothing, and the client emptied its
    /// whole sequence table. Builds without #840 still do.
    pub(super) fn apply_sequence_overrides(
        categories: &mut HashMap<u32, CategoryData>,
    ) -> HashMap<u32, Vec<u32>> {
        use crate::base::sequence_overrides::{generate_sequence_xml, SEQUENCE_OVERRIDES};

        let mut overridden: HashMap<u32, Vec<u32>> = HashMap::new();
        if SEQUENCE_OVERRIDES.is_empty() {
            return overridden;
        }

        let Some(sequences) = categories.get_mut(&CATEGORY_KISMET_SEQUENCES) else {
            tracing::warn!(
                category = CATEGORY_KISMET_SEQUENCES,
                "CookedDataKismetSeqEvent not loaded; skipping sequence overrides"
            );
            return overridden;
        };

        let mut applied: Vec<u32> = Vec::with_capacity(SEQUENCE_OVERRIDES.len());
        for ov in SEQUENCE_OVERRIDES {
            let was_present = sequences.elements.contains_key(&ov.sequence_id);
            sequences
                .elements
                .insert(ov.sequence_id, generate_sequence_xml(ov));
            applied.push(ov.sequence_id);
            tracing::info!(
                sequence_id = ov.sequence_id,
                event_id = ov.event_id,
                replaced_existing = was_present,
                "Applied Cimmeria Kismet sequence override",
            );
        }

        let bump = compute_sequence_metadata_bump(SEQUENCE_OVERRIDES);
        sequences.metadata = sequences.metadata.wrapping_add(bump);
        applied.sort_unstable();
        tracing::info!(
            category = CATEGORY_KISMET_SEQUENCES,
            count = applied.len(),
            bump,
            bumped_metadata = sequences.metadata,
            "Cimmeria Kismet sequence overrides applied; metadata bumped",
        );
        overridden.insert(CATEGORY_KISMET_SEQUENCES, applied);

        overridden
    }

    /// Add Cimmeria's worlds (the historical CellBlock worlds) to the
    /// freshly-loaded `CookedWorldInfo` category and bump its metadata so a
    /// client's next `versionInfoRequest` takes the per-key handshake.
    ///
    /// Same shape as [`Self::apply_sequence_overrides`]: every entry is a
    /// full regeneration, so there is nothing to fail on. The override list
    /// is also what keeps category 12 off the destructive invalidate-all
    /// path once its version has moved.
    pub(super) fn apply_world_info_overrides(
        categories: &mut HashMap<u32, CategoryData>,
    ) -> HashMap<u32, Vec<u32>> {
        use crate::base::world_info_overrides::{generate_world_info_xml, WORLD_INFO_OVERRIDES};

        let mut overridden: HashMap<u32, Vec<u32>> = HashMap::new();
        if WORLD_INFO_OVERRIDES.is_empty() {
            return overridden;
        }

        let Some(worlds) = categories.get_mut(&CATEGORY_WORLD_INFO) else {
            tracing::warn!(
                category = CATEGORY_WORLD_INFO,
                "CookedWorldInfo not loaded; skipping world info overrides"
            );
            return overridden;
        };

        let mut applied: Vec<u32> = Vec::with_capacity(WORLD_INFO_OVERRIDES.len());
        for ov in WORLD_INFO_OVERRIDES {
            let was_present = worlds.elements.contains_key(&ov.world_id);
            worlds
                .elements
                .insert(ov.world_id, generate_world_info_xml(ov));
            applied.push(ov.world_id);
            tracing::info!(
                world_id = ov.world_id,
                world = ov.world,
                client_map = ov.client_map,
                replaced_existing = was_present,
                "Applied Cimmeria world info override",
            );
        }

        let bump = compute_world_info_metadata_bump(WORLD_INFO_OVERRIDES);
        worlds.metadata = worlds.metadata.wrapping_add(bump);
        applied.sort_unstable();
        tracing::info!(
            category = CATEGORY_WORLD_INFO,
            count = applied.len(),
            bump,
            bumped_metadata = worlds.metadata,
            "Cimmeria world info overrides applied; metadata bumped",
        );
        overridden.insert(CATEGORY_WORLD_INFO, applied);

        overridden
    }
}
