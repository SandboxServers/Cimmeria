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
/// Maps category_id -> { elementId -> raw XML bytes }.
#[derive(Clone)]
pub(crate) struct ResourceCache {
    categories: Arc<HashMap<u32, CategoryData>>,
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

impl ResourceCache {
    /// Load all PAK files from the given directory.
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

        Ok(Self {
            categories: Arc::new(categories),
        })
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
}
