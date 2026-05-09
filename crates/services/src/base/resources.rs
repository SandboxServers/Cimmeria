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
fn compute_metadata_bump(overrides: &[super::mission_overrides::MissionOverride]) -> u32 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for ov in overrides {
        ov.mission_id.hash(&mut hasher);
        ov.insert_after_step_id.hash(&mut hasher);
        ov.injected_steps_xml.hash(&mut hasher);
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

        let overridden_elements = Self::apply_mission_overrides(&mut categories);

        Ok(Self {
            categories: Arc::new(categories),
            overridden_elements: Arc::new(overridden_elements),
        })
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
        use super::mission_overrides::{apply_override, MissionOverride, MISSION_OVERRIDES};

        let mut overridden: HashMap<u32, Vec<u32>> = HashMap::new();

        let Some(missions) = categories.get_mut(&CATEGORY_MISSIONS) else {
            tracing::warn!(
                category = CATEGORY_MISSIONS,
                "CookedDataMissions not loaded; skipping mission overrides"
            );
            return overridden;
        };

        let mut applied: Vec<u32> = Vec::with_capacity(MISSION_OVERRIDES.len());
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
                    applied.push(ov.mission_id);
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

        if !applied.is_empty() {
            // Hash only the overrides that *actually applied*, not the
            // full `MISSION_OVERRIDES` registry. If a target mission's
            // entry is missing from the PAK or its XML shape doesn't
            // match, that override skips silently — the cache state
            // for that mission is unchanged. Mixing the skipped
            // override's content into the bump would advance the
            // metadata as if it had applied, and the client would
            // re-fetch entries that didn't actually change.
            let applied_overrides: Vec<MissionOverride> = MISSION_OVERRIDES
                .iter()
                .filter(|ov| applied.contains(&ov.mission_id))
                .map(|ov| MissionOverride {
                    mission_id: ov.mission_id,
                    insert_after_step_id: ov.insert_after_step_id,
                    injected_steps_xml: ov.injected_steps_xml,
                })
                .collect();
            let bump = compute_metadata_bump(&applied_overrides);
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
mod tests {
    use super::*;
    use cimmeria_entity::inventory::{
        INV_ARTIFACT1, INV_ARTIFACT2, INV_BACK, INV_BANDOLIER, INV_BUYBACK, INV_CHEST,
        INV_CRAFTING, INV_FACE, INV_FEET, INV_HANDS, INV_HEAD, INV_LEGS, INV_MAIN, INV_MISSION,
        INV_NECK, INV_WAIST,
    };

    /// Sentinel-slot invariant: the move handler's three-step swap parks the
    /// source row at `slot_id = -1` mid-transaction. For that to be safe,
    /// `bag_min_slot` MUST never return a value `<= -1` for any container —
    /// otherwise a concurrent grant could legitimately reserve `slot_id = -1`
    /// and collide with the parked sentinel.
    ///
    /// This test pins that invariant across `container_id` 0..=16. The
    /// game-defined range is 1..=16 (main, mission, bandolier, equipment
    /// slots 4..=14, crafting, vendor buyback); 0 is included as a sentinel
    /// for "no container" so the symmetry with `bag_max_slots` (which
    /// returns 0 for 0) is also exercised. Any out-of-range container_id
    /// returning 0 from `bag_min_slot` is fine — `bag_max_slots` returns 0
    /// there too, so no slot is ever reservable.
    ///
    /// Documented as the regression guard in `move_/mod.rs`'s swap-path
    /// comment (`bag_max_slots() never reserves negative slots, so
    /// grant/purchase paths cannot land there`).
    #[test]
    fn bag_min_slot_is_non_negative_for_every_container() {
        for container_id in 0..=16 {
            let min = bag_min_slot(container_id);
            assert!(
                min >= 0,
                "bag_min_slot({container_id}) returned {min}; must be >= 0"
            );
        }
    }

    #[test]
    fn bag_max_slots_known_containers_match_constants() {
        assert_eq!(bag_max_slots(INV_MAIN), 40);
        assert_eq!(bag_max_slots(INV_MISSION), 100);
        assert_eq!(bag_max_slots(INV_BANDOLIER), 4);
        for container_id in [
            INV_HEAD,
            INV_FACE,
            INV_NECK,
            INV_CHEST,
            INV_HANDS,
            INV_WAIST,
            INV_BACK,
            INV_LEGS,
            INV_FEET,
            INV_ARTIFACT1,
            INV_ARTIFACT2,
        ] {
            assert_eq!(bag_max_slots(container_id), 1);
        }
        assert_eq!(bag_max_slots(INV_CRAFTING), 100);
        assert_eq!(bag_max_slots(INV_BUYBACK), 12);
    }

    #[test]
    fn bag_max_slots_out_of_range_returns_zero() {
        assert_eq!(bag_max_slots(0), 0);
        assert_eq!(bag_max_slots(17), 0);
        assert_eq!(bag_max_slots(100), 0);
        assert_eq!(bag_max_slots(-1), 0);
    }

    #[test]
    fn bag_min_slot_bandolier_is_zero() {
        assert_eq!(
            bag_min_slot(INV_BANDOLIER),
            0,
            "bandolier slots are zero-based weapon slots"
        );
    }

    #[test]
    fn bag_min_slot_other_containers_is_zero() {
        for container_id in [1, 2, 4, 15, 16] {
            assert_eq!(
                bag_min_slot(container_id),
                0,
                "container {container_id} must start at slot 0"
            );
        }
    }

    // ── compute_metadata_bump invariants ─────────────────────────────
    // The bump is what makes the cooked-data version-info handshake see a
    // mismatch and reship overridden entries. Same content → same bump
    // (no churn). Any field change (mission_id, insert_after_step_id, or
    // injected XML) → different bump (client refetches).

    use super::super::mission_overrides::MissionOverride;

    fn ov(mission_id: u32, after: u32, xml: &'static str) -> MissionOverride {
        MissionOverride {
            mission_id,
            insert_after_step_id: after,
            injected_steps_xml: xml,
        }
    }

    #[test]
    fn compute_metadata_bump_is_deterministic_across_calls() {
        let overrides = [
            ov(622, 2113, "<Steps>x</Steps>"),
            ov(641, 2121, "<Steps>y</Steps>"),
        ];
        assert_eq!(
            compute_metadata_bump(&overrides),
            compute_metadata_bump(&overrides),
            "same overrides must hash to the same bump on every call",
        );
    }

    #[test]
    fn compute_metadata_bump_low_bit_is_always_set() {
        // Even an empty-overrides bump must be non-zero so a future caller
        // that hits this path doesn't leave metadata unchanged.
        let bump_empty = compute_metadata_bump(&[]);
        assert_eq!(bump_empty & 0x1, 0x1, "low bit must be set");

        let bump_one = compute_metadata_bump(&[ov(1, 2, "x")]);
        assert_eq!(bump_one & 0x1, 0x1, "low bit must be set");
    }

    #[test]
    fn compute_metadata_bump_changes_when_xml_changes() {
        let a = [ov(622, 2113, "<Steps>aaa</Steps>")];
        let b = [ov(622, 2113, "<Steps>bbb</Steps>")];
        assert_ne!(
            compute_metadata_bump(&a),
            compute_metadata_bump(&b),
            "edits to injected XML must produce a different bump so the \
             client refetches the patched entry",
        );
    }

    #[test]
    fn compute_metadata_bump_changes_when_insert_after_changes() {
        // The load-bearing case: a maintainer pivots only the insertion
        // anchor (e.g., 2113 → some other anchor in the same mission)
        // while keeping mission_id and XML identical. Without
        // insert_after_step_id in the hash this would silently re-use
        // the previous bump and the client would never refetch even
        // though the patched XML's structure changed.
        let a = [ov(622, 2113, "<Steps>x</Steps>")];
        let b = [ov(622, 9999, "<Steps>x</Steps>")];
        assert_ne!(
            compute_metadata_bump(&a),
            compute_metadata_bump(&b),
            "insert_after_step_id must participate in the hash",
        );
    }

    #[test]
    fn compute_metadata_bump_changes_when_mission_id_changes() {
        let a = [ov(622, 2113, "<Steps>x</Steps>")];
        let b = [ov(641, 2113, "<Steps>x</Steps>")];
        assert_ne!(compute_metadata_bump(&a), compute_metadata_bump(&b));
    }

    // ── apply_mission_overrides on hand-built CategoryData ───────────
    // The PAK-load path is integration-tested by running the real server,
    // but the in-memory mutation logic is covered here without going
    // through ZIP IO. Build the same `HashMap<u32, CategoryData>` shape
    // that `load_pak` would have produced, call the private helper, and
    // assert the post-state.

    /// Minimal but realistic mission XML: the QA-build root + one `<Steps>`
    /// child per known step id. `apply_override` looks for `StepID="<id>"`
    /// and the next `</Steps>`, both present here.
    fn fake_mission_xml(mission_id: u32, step_ids: &[u32]) -> Vec<u8> {
        let mut s = String::new();
        s.push_str(&format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
             <COOKED_MISSION MissionID=\"{mission_id}\">"
        ));
        for sid in step_ids {
            s.push_str(&format!(
                "<Steps StepEnabled=\"false\" StepID=\"{sid}\" \
                 AwardXP=\"false\" Difficulty=\"1\">\
                 <StepDisplayLogText>step {sid}</StepDisplayLogText>\
                 </Steps>"
            ));
        }
        s.push_str("</COOKED_MISSION>");
        s.into_bytes()
    }

    fn missions_category_with(entries: &[(u32, Vec<u32>)], metadata: u32) -> CategoryData {
        let mut elements = HashMap::new();
        for (mid, steps) in entries {
            elements.insert(*mid, fake_mission_xml(*mid, steps));
        }
        CategoryData { metadata, elements }
    }

    /// Pin the post-condition of `apply_mission_overrides`: every
    /// registered override mutates the corresponding entry, the metadata
    /// is bumped to a new value, and the returned overridden-elements
    /// map names the patched ids.
    #[test]
    fn apply_mission_overrides_patches_entries_and_bumps_metadata() {
        let starting_metadata = 7538;
        let mut categories: HashMap<u32, CategoryData> = HashMap::new();
        // Match the live PAK shape: mission 622 has step 2113; mission
        // 641 has steps 2121, 3563, 3564.
        categories.insert(
            CATEGORY_MISSIONS,
            missions_category_with(
                &[(622, vec![2113]), (641, vec![2121, 3563, 3564])],
                starting_metadata,
            ),
        );

        let overridden = ResourceCache::apply_mission_overrides(&mut categories);

        let missions = categories
            .get(&CATEGORY_MISSIONS)
            .expect("missions category must remain after apply");

        // Metadata bumped to a content-derived non-zero offset.
        assert_ne!(
            missions.metadata, starting_metadata,
            "apply must bump the category metadata so the client refetches",
        );
        let bump = missions.metadata.wrapping_sub(starting_metadata);
        assert_eq!(bump & 0x1, 0x1, "low bit of bump must be set");

        // Each override's mission entry now contains the new step.
        let m622 = std::str::from_utf8(missions.elements.get(&622).unwrap()).unwrap();
        assert!(
            m622.contains("StepID=\"80622\""),
            "_622 must carry the equip-pistol step after override; got: {m622}",
        );
        let m641 = std::str::from_utf8(missions.elements.get(&641).unwrap()).unwrap();
        assert!(
            m641.contains("StepID=\"80641\""),
            "_641 must carry the equip-P90 step after override; got: {m641}",
        );

        // Returned map lists exactly the overridden ids, sorted ascending,
        // for the missions category.
        let ids = overridden
            .get(&CATEGORY_MISSIONS)
            .expect("missions must appear in returned map");
        assert_eq!(
            ids.as_slice(),
            &[622u32, 641u32],
            "overridden_elements must name both patched ids in ascending order",
        );
    }

    /// Idempotency at the bump level: running apply twice on the same
    /// fresh input yields the same total metadata advancement (because
    /// the bump is content-derived, not random). Two server starts in a
    /// row see the same value and the client doesn't churn.
    #[test]
    fn apply_mission_overrides_bump_is_deterministic_per_content() {
        let first_meta = {
            let mut categories = HashMap::new();
            categories.insert(
                CATEGORY_MISSIONS,
                missions_category_with(&[(622, vec![2113]), (641, vec![2121, 3563, 3564])], 7538),
            );
            ResourceCache::apply_mission_overrides(&mut categories);
            categories.get(&CATEGORY_MISSIONS).unwrap().metadata
        };
        let second_meta = {
            let mut categories = HashMap::new();
            categories.insert(
                CATEGORY_MISSIONS,
                missions_category_with(&[(622, vec![2113]), (641, vec![2121, 3563, 3564])], 7538),
            );
            ResourceCache::apply_mission_overrides(&mut categories);
            categories.get(&CATEGORY_MISSIONS).unwrap().metadata
        };
        assert_eq!(first_meta, second_meta);
    }

    /// Defensive path: when the missions category itself is absent (PAK
    /// missing on disk, e.g. for a partial bootstrap), apply_mission_
    /// overrides must return an empty map and not panic. The warn log
    /// is enough — server startup keeps going.
    #[test]
    fn apply_mission_overrides_no_op_when_category_missing() {
        let mut categories: HashMap<u32, CategoryData> = HashMap::new();
        let overridden = ResourceCache::apply_mission_overrides(&mut categories);
        assert!(
            overridden.is_empty(),
            "missing category must not produce override entries",
        );
    }

    /// Defensive path: when an override's target mission isn't present
    /// in the loaded PAK, that single override is skipped (logged as a
    /// warn) without aborting the rest. This protects a partial cooker
    /// run or a Cimmeria override pointing at a mission that hasn't
    /// shipped yet.
    #[test]
    fn apply_mission_overrides_skips_missing_target_mission() {
        let mut categories = HashMap::new();
        // Only mission 622 is present; mission 641's override should
        // skip with a warn but mission 622's still applies.
        categories.insert(
            CATEGORY_MISSIONS,
            missions_category_with(&[(622, vec![2113])], 7538),
        );

        let overridden = ResourceCache::apply_mission_overrides(&mut categories);

        let ids = overridden.get(&CATEGORY_MISSIONS).unwrap();
        assert!(ids.contains(&622), "mission 622 override must still apply",);
        assert!(
            !ids.contains(&641),
            "mission 641 must be skipped when entry is absent",
        );
    }

    /// The metadata bump must reflect what *actually applied*, not the
    /// full `MISSION_OVERRIDES` registry. A deployment where a target
    /// mission entry is missing (and that override therefore skips)
    /// must produce a different bump than one where every override
    /// applies — otherwise the version stamp the client sees is out
    /// of sync with the InvalidKeys list it actually receives.
    #[test]
    fn apply_mission_overrides_bump_reflects_only_applied_subset() {
        // Full apply: both 622 and 641 present.
        let full_meta = {
            let mut c = HashMap::new();
            c.insert(
                CATEGORY_MISSIONS,
                missions_category_with(&[(622, vec![2113]), (641, vec![2121, 3563, 3564])], 7538),
            );
            ResourceCache::apply_mission_overrides(&mut c);
            c.get(&CATEGORY_MISSIONS).unwrap().metadata
        };
        // Partial apply: only 622 present; 641's override skips.
        let partial_meta = {
            let mut c = HashMap::new();
            c.insert(
                CATEGORY_MISSIONS,
                missions_category_with(&[(622, vec![2113])], 7538),
            );
            ResourceCache::apply_mission_overrides(&mut c);
            c.get(&CATEGORY_MISSIONS).unwrap().metadata
        };

        assert_ne!(
            full_meta, partial_meta,
            "metadata bump must differ when override application differs — \
             a partial-apply deployment must not produce the same bump as \
             a full-apply",
        );
    }

    /// `overridden_elements` accessor returns sorted slice for known
    /// categories and an empty slice for unknown ones (the latter is
    /// what handle_version_info_request needs to fall back to the
    /// legacy invalidate-all path).
    #[test]
    fn overridden_elements_returns_empty_slice_for_unknown_category() {
        let cache = ResourceCache {
            categories: Arc::new(HashMap::new()),
            overridden_elements: Arc::new(HashMap::new()),
        };
        assert!(
            cache.overridden_elements(999).is_empty(),
            "unknown category must yield empty slice",
        );
    }
}
