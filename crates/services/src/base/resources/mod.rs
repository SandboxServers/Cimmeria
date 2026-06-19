use std::collections::HashMap;
use std::io::Read as IoRead;
use std::sync::Arc;

// ── Inventory constants (from python/Atrea/enums.py + Account.py) ────────────

/// Order in which starter items fill inventory bags (Account.py:12-31).
/// Equipment slots first so items get equipped and show on the char select screen.
pub(crate) const BAG_FILL_ORDER: &[i32] = &[
    4,  // Head
    5,  // Face
    6,  // Neck
    7,  // Chest
    8,  // Hands
    9,  // Waist
    10, // Back
    11, // Legs
    12, // Feet
    13, // Artifact1
    14, // Artifact2
    3,  // Bandolier
    2,  // Mission
    1,  // Main
    15, // Crafting
];

/// Max items per container (Constants.py:142-162).
pub(crate) fn bag_max_slots(container_id: i32) -> i32 {
    match container_id {
        1 => 40,     // Main
        2 => 100,    // Mission
        3 => 4,      // Bandolier
        4..=14 => 1, // Equipment slots
        15 => 100,   // Crafting
        16 => 12,    // Vendor Buyback
        _ => 0,
    }
}

/// Lowest assignable slot for a container. All current containers, including
/// the bandolier, start at slot 0 — there is no fist-weapon reservation in
/// this game's design (the bandolier is purely 4 weapon slots indexed 0..3).
///
/// Kept as a function (rather than inlining `0`) so the per-container
/// nonneg invariant test in this file still has a hook, and so a future
/// container with a different lower bound can be added without touching
/// every call site.
pub(crate) fn bag_min_slot(_container_id: i32) -> i32 {
    0
}

/// Pick the first bag in [`BAG_FILL_ORDER`] that (a) is in the item's
/// `container_sets` list AND (b) still has room based on the per-bag
/// next-free-slot map.
///
/// Used by `character_create` to place starter items. Pre-fix this was
/// inline in `handle_create_character` and the selection ignored
/// fullness — picking the first valid bag unconditionally then
/// `continue`-ing if it was full. That dropped items that could
/// otherwise overflow to a later bag in the fill order (live observation
/// 2026-06-02: item 4343 dropped at character create because its
/// primary bag filled up first while a later valid bag still had room).
///
/// `slot_indices` carries the next free slot for each bag that's been
/// touched so far. A bag with no entry yet starts at
/// `bag_min_slot(bag)`; an entry that has reached `bag_max_slots(bag)`
/// is full.
///
/// Returns `Some(bag_id)` for the first bag that satisfies both gates,
/// `None` if no valid + non-full bag exists.
pub(crate) fn pick_first_open_bag(
    container_sets: &[i32],
    slot_indices: &HashMap<i32, i32>,
) -> Option<i32> {
    BAG_FILL_ORDER
        .iter()
        .find(|&&bag| {
            if !container_sets.contains(&bag) {
                return false;
            }
            let next_slot = slot_indices
                .get(&bag)
                .copied()
                .unwrap_or_else(|| bag_min_slot(bag));
            next_slot < bag_max_slots(bag)
        })
        .copied()
}

// ── Resource cache ───────────────────────────────────────────────────────────

/// Per-category cooked data loaded from a PAK file.
pub(crate) struct CategoryData {
    /// MetaData value (u32 from the PAK's MetaData entry).
    pub metadata: u32,
    /// elementId -> raw XML bytes.
    pub elements: HashMap<u32, Vec<u8>>,
}

/// All cooked game data, loaded from `data/cache/*.pak` at startup.
///
/// Maps `category_id -> { elementId -> raw XML bytes }`.
///
/// Cimmeria-side overrides (see [`super::mission_overrides`]) are applied
/// in-memory after the PAK load so the client picks up the modifications
/// via the existing cooked-data wire path — no on-disk PAK edit, no
/// client-artifact distribution. The set of overridden element IDs per
/// category is tracked so [`super::cooked_data::handle_version_info_request`]
/// can emit `invalidate_all = false` + per-key `InvalidKeys`, scoping the
/// client-side cache invalidation to just the patched entries.
#[derive(Clone)]
pub(crate) struct ResourceCache {
    categories: Arc<HashMap<u32, CategoryData>>,
    /// `category_id -> sorted list of element IDs that were overridden`.
    /// Empty for categories with no overrides. Sorted so the on-wire
    /// `InvalidKeys` ARRAY<u32> ordering is deterministic across runs
    /// (a chain-replay-style guard for the client cache fingerprint).
    overridden_elements: Arc<HashMap<u32, Vec<u32>>>,
}

/// Category ID -> PAK filename mapping (from `resource.cpp`).
pub(crate) const CATEGORY_PAKS: &[(u32, &str)] = &[
    (1, "CookedDataKismetSeqEvent.pak"),
    (2, "CookedDataAbilities.pak"),
    (3, "CookedDataMissions.pak"),
    (4, "CookedDataItems.pak"),
    (5, "CookedDataDialogs.pak"),
    (6, "CookedDataKismetSetEvent.pak"),
    (7, "CookedCharCreation.pak"),
    (8, "CookedInteractionSet.pak"),
    (9, "CookedDataEffects.pak"),
    (10, "TextStrings.pak"),
    (11, "ErrorStrings.pak"),
    (12, "CookedWorldInfo.pak"),
    (13, "CookedDataStargates.pak"),
    (14, "CookedDataContainers.pak"),
    (15, "CookedBlueprints.pak"),
    (16, "CookedSciences.pak"),
    (17, "CookedDisciplines.pak"),
    (18, "CookedParadigm.pak"),
    (19, "SpecialWords.pak"),
    (20, "CookedInteractions.pak"),
];

/// Category id for `CookedDataMissions.pak` (see [`CATEGORY_PAKS`]).
const CATEGORY_MISSIONS: u32 = 3;

/// Category id for `CookedDataItems.pak` (see [`CATEGORY_PAKS`]).
/// Source: the original C++ server's `resource.cpp` category map.
/// Per-item icon, name, max-stack lookups land here on the client.
const CATEGORY_ITEMS: u32 = 4;

/// Category id for `CookedDataDialogs.pak` (see [`CATEGORY_PAKS`]).
/// Per-dialog screen text is rendered from this catalogue client-side,
/// not from any wire message — so a corrected or new dialog body must be
/// pushed via the cooked-data invalidation handshake.
const CATEGORY_DIALOGS: u32 = 5;

/// Compute the deterministic metadata bump for a set of overrides.
///
/// The bump is hashed from every field that affects what the client sees:
/// `mission_id`, `insert_after_step_id`, and the injected XML. Two server
/// starts on the same override content produce the same bump (no
/// re-invalidation churn); changing any field changes the bump (client
/// mismatches and refetches).
///
/// The result is OR'd with 1 so the low bit is always set — guards
/// against the rare hash that lands on `0`, which would leave the
/// metadata unchanged and the client stuck on stale entries.
fn compute_metadata_bump(
    overrides: &[super::mission_overrides::MissionOverride],
    text_overrides: &[super::mission_overrides::StepTextOverride],
) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for ov in overrides {
        ov.mission_id.hash(&mut hasher);
        ov.insert_after_step_id.hash(&mut hasher);
        ov.injected_steps_xml.hash(&mut hasher);
    }
    for ov in text_overrides {
        ov.mission_id.hash(&mut hasher);
        ov.step_id.hash(&mut hasher);
        ov.new_step_display_log_text.hash(&mut hasher);
    }
    ((hasher.finish() as u32) & 0xFFFF) | 0x1
}

/// Companion of [`compute_metadata_bump`] for the items category.
/// Same deterministic-hash + low-bit-set discipline so two server
/// starts on identical override content produce identical bumps.
fn compute_item_metadata_bump(overrides: &[super::item_overrides::ItemOverride]) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for ov in overrides {
        ov.item_id.hash(&mut hasher);
        ov.new_icon_location.hash(&mut hasher);
        ov.new_max_stack_size.hash(&mut hasher);
    }
    ((hasher.finish() as u32) & 0xFFFF) | 0x1
}

/// Companion of [`compute_metadata_bump`] for the dialogs category.
/// Hashes every field that affects the rendered XML (dialog id, flags,
/// kismet/screen-type, and each screen's id/speaker/text) so two server
/// starts on identical override content produce identical bumps and no
/// re-invalidation churn; any edit changes the bump and refetches.
fn compute_dialog_metadata_bump(overrides: &[super::dialog_overrides::DialogOverride]) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for ov in overrides {
        ov.dialog_id.hash(&mut hasher);
        ov.dialog_flags.hash(&mut hasher);
        ov.kismet_event_set_id.hash(&mut hasher);
        ov.ui_screen_type.hash(&mut hasher);
        for screen in ov.screens {
            screen.screen_id.hash(&mut hasher);
            screen.speaker_id.hash(&mut hasher);
            screen.text.hash(&mut hasher);
        }
    }
    ((hasher.finish() as u32) & 0xFFFF) | 0x1
}

impl ResourceCache {
    /// Load all PAK files from the given directory and apply Cimmeria
    /// overrides (mission XML for new "Equip the …" steps).
    pub fn load_all(data_dir: &str) -> Result<Self, String> {
        let mut categories = HashMap::new();

        for &(cat_id, filename) in CATEGORY_PAKS {
            let pak_path = format!("{}/{}", data_dir, filename);
            match Self::load_pak(&pak_path) {
                Ok(cat_data) => {
                    tracing::info!(
                        category = cat_id,
                        file = filename,
                        elements = cat_data.elements.len(),
                        metadata = cat_data.metadata,
                        "Loaded PAK"
                    );
                    categories.insert(cat_id, cat_data);
                }
                Err(e) => {
                    tracing::warn!(
                        category = cat_id,
                        file = filename,
                        "Failed to load PAK: {e}"
                    );
                }
            }
        }

        tracing::info!(
            categories = categories.len(),
            total_elements = categories.values().map(|c| c.elements.len()).sum::<usize>(),
            "Resource cache loaded"
        );

        let mut overridden_elements = Self::apply_mission_overrides(&mut categories);
        // Apply item overrides into the same overridden-elements map.
        // `extend` over disjoint category keys means each call owns its
        // own category id (missions=3, items=4) and there's no risk of
        // one wiping the other's invalid-keys list. Cross-pollination
        // between categories would surface as the client invalidating
        // the wrong PAK category on handshake; the categories live in
        // separate XML files on disk so it can't physically conflict,
        // but keep both writes disjoint anyway in case a future
        // override module starts producing entries for an existing
        // category.
        let item_overridden = Self::apply_item_overrides(&mut categories);
        overridden_elements.extend(item_overridden);
        // Dialogs category (5) — disjoint from missions (3) / items (4),
        // so the same `extend` discipline applies: each call owns its own
        // category id and can't clobber another's invalid-keys list.
        let dialog_overridden = Self::apply_dialog_overrides(&mut categories);
        overridden_elements.extend(dialog_overridden);

        Ok(Self {
            categories: Arc::new(categories),
            overridden_elements: Arc::new(overridden_elements),
        })
    }

    /// Patch the freshly-loaded `CookedDataItems` category with
    /// Cimmeria's icon + stack-size overrides, bumping the category
    /// metadata so the client's next `versionInfoRequest` sees a
    /// fresh value and triggers the per-key invalidation handshake.
    ///
    /// Same shape as [`Self::apply_mission_overrides`] — the cooked
    /// data wire path is category-agnostic; only the per-category
    /// override registry differs.
    fn apply_item_overrides(categories: &mut HashMap<u32, CategoryData>) -> HashMap<u32, Vec<u32>> {
        use super::item_overrides::{apply_override, ITEM_OVERRIDES};

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
    /// Cimmeria's regenerated dialog entries, bumping the category
    /// metadata so the client's next `versionInfoRequest` triggers the
    /// per-key invalidation handshake.
    ///
    /// Unlike [`Self::apply_item_overrides`] / [`Self::apply_mission_overrides`],
    /// each override *regenerates* the whole `<COOKED_DIALOG>` entry rather
    /// than patching a substring, so it works whether or not the dialog id
    /// was present in the PAK: a corrected existing dialog (Frost's 3995)
    /// and a brand-new one (the Guard corpse's 3996) are both just an
    /// `elements.insert(dialog_id, generated)`. There's no "entry not
    /// present" skip and no "XML shape didn't match" failure — generation
    /// is infallible.
    fn apply_dialog_overrides(
        categories: &mut HashMap<u32, CategoryData>,
    ) -> HashMap<u32, Vec<u32>> {
        use super::dialog_overrides::{generate_dialog_xml, DIALOG_OVERRIDES};

        let mut overridden: HashMap<u32, Vec<u32>> = HashMap::new();
        if DIALOG_OVERRIDES.is_empty() {
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

        let bump = compute_dialog_metadata_bump(DIALOG_OVERRIDES);
        dialogs.metadata = dialogs.metadata.wrapping_add(bump);
        applied.sort_unstable();
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
    /// per-key invalidation handshake.
    ///
    /// Returns the overridden-elements map keyed by category id. An entry
    /// with an empty vec is omitted; absence of a category means
    /// `handle_version_info_request` falls through to the legacy
    /// "echo or invalidate-all" path for that category.
    fn apply_mission_overrides(
        categories: &mut HashMap<u32, CategoryData>,
    ) -> HashMap<u32, Vec<u32>> {
        use super::mission_overrides::{
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

    /// Load a single PAK file (ZIP archive) into a CategoryData.
    fn load_pak(pak_path: &str) -> Result<CategoryData, String> {
        let file =
            std::fs::File::open(pak_path).map_err(|e| format!("Failed to open {pak_path}: {e}"))?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("Failed to read ZIP {pak_path}: {e}"))?;

        let mut elements = HashMap::new();
        let mut metadata: u32 = 0;

        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| format!("ZIP entry {i}: {e}"))?;
            let name = entry.name().to_string();

            if name == "MetaData" {
                let mut buf = [0u8; 4];
                if entry.read_exact(&mut buf).is_ok() {
                    metadata = u32::from_le_bytes(buf);
                }
            } else if let Some(id_str) = name.strip_prefix('_') {
                if let Ok(id) = id_str.parse::<u32>() {
                    let mut data = Vec::with_capacity(entry.size() as usize);
                    entry
                        .read_to_end(&mut data)
                        .map_err(|e| format!("Failed to read entry {name}: {e}"))?;
                    elements.insert(id, data);
                }
            }
        }

        Ok(CategoryData { metadata, elements })
    }

    /// Get a category's data.
    pub fn category(&self, category_id: u32) -> Option<&CategoryData> {
        self.categories.get(&category_id)
    }

    /// Get XML data for a given category + element.
    pub fn get(&self, category_id: u32, element_id: u32) -> Option<&Vec<u8>> {
        self.categories.get(&category_id)?.elements.get(&element_id)
    }

    /// Element IDs that Cimmeria overrides for the given category, sorted
    /// ascending. Returns an empty slice for categories that are unmodified
    /// (so the version-info handler can fall back to "echo current
    /// version, no invalidation needed" when the client's already in
    /// sync). Wrapped in a function so call sites don't need to reach
    /// into the field directly.
    pub fn overridden_elements(&self, category_id: u32) -> &[u32] {
        self.overridden_elements
            .get(&category_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests;
