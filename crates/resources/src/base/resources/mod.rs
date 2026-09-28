use std::collections::HashMap;
use std::io::Read as IoRead;
use std::sync::Arc;

// ── Inventory constants (from python/Atrea/enums.py + Account.py) ────────────

/// Order in which starter items fill inventory bags (Account.py:12-31).
/// Equipment slots first so items get equipped and show on the char select screen.
pub const BAG_FILL_ORDER: &[i32] = &[
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

/// Max items per container (Constants.py:142-162). The table lives in `cimmeria-wire`,
/// which the cell's bandolier check also reads; this re-export keeps the old path.
pub use cimmeria_wire::containers::bag_max_slots;

/// Lowest assignable slot for a container. All current containers, including
/// the bandolier, start at slot 0 — there is no fist-weapon reservation in
/// this game's design (the bandolier is purely 4 weapon slots indexed 0..3).
///
/// Kept as a function (rather than inlining `0`) so the per-container
/// nonneg invariant test in this file still has a hook, and so a future
/// container with a different lower bound can be added without touching
/// every call site.
pub fn bag_min_slot(_container_id: i32) -> i32 {
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
pub fn pick_first_open_bag(
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
pub struct CategoryData {
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
/// client-artifact distribution. Each patched category's metadata is bumped,
/// so a client holding the shipped category resyncs it in full
/// (`cimmeria_base_session::base::cooked_sync`, #840). The overridden element
/// IDs per category are tracked for the element-push log level.
#[derive(Clone)]
pub struct ResourceCache {
    categories: Arc<HashMap<u32, CategoryData>>,
    /// `category_id -> sorted list of element IDs that were overridden`.
    /// Empty for categories with no overrides. Sorted so the on-wire
    /// `InvalidKeys` ARRAY<u32> ordering is deterministic across runs
    /// (a chain-replay-style guard for the client cache fingerprint).
    overridden_elements: Arc<HashMap<u32, Vec<u32>>>,
}

/// Category ID -> PAK filename mapping.
///
/// The IDs are the client's registration order, confirmed from
/// `CookedData_RegisterAllLibCategories` (SGW.exe `0x00420074`): the client
/// registers **exactly 21** ServerSource categories, numbered 1–21, with
/// category 21 = `BehaviorEventData` / `CookedBehaviorEvents.pak`. There is
/// **no** client-side category 0, category 22, or `pet_command` — the
/// legacy `resource.cpp`/`Def.py` table that reserved `21: pet_command` and
/// put `behavior_event` at 22 drifted from the client and is not the wire
/// contract. See `docs/reverse-engineering/findings/cooked-data-pipeline.md`
/// and `docs/protocol/client-verified-wire-formats.md` §8.
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
    (CATEGORY_BEHAVIOR_EVENTS, "CookedBehaviorEvents.pak"),
];

/// Category id for `CookedBehaviorEvents.pak` (see [`CATEGORY_PAKS`]).
///
/// The client registers this as `BehaviorEventData` — the 21st and final
/// `ServerSource` category at `CookedData_RegisterAllLibCategories`
/// (`SGW.exe` `0x00420074`). The legacy `resource.cpp`/`Def.py` map reserved
/// 21 for `pet_command` (never implemented client-side) and pushed
/// `behavior_event` to 22, drifting past the client's contiguous 1–21 enum:
/// a fragment tagged 22 is silently dropped. Match the client: 21.
pub const CATEGORY_BEHAVIOR_EVENTS: u32 = 21;

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

/// Category id for `CookedDataKismetSeqEvent.pak` (see [`CATEGORY_PAKS`]).
/// `onSequence` carries a sequence id; the client resolves it to a Kismet
/// script path from this catalogue, so a sequence the shipped PAK lacks must
/// be pushed via the cooked-data invalidation handshake.
const CATEGORY_KISMET_SEQUENCES: u32 = 1;

/// Category id for `CookedWorldInfo.pak` (see [`CATEGORY_PAKS`]).
/// `onClientMapLoad` names a `WorldID`; the client's world table comes from
/// this catalogue, so a world id the shipped PAK lacks must be pushed via the
/// cooked-data invalidation handshake.
const CATEGORY_WORLD_INFO: u32 = 12;

mod apply_overrides;
mod metadata_bump;

// For the `tests` child, which exercises each bump helper directly through
// `super::super::*`; `apply_overrides` imports them itself. Not re-exported
// past `resources`.
#[cfg(test)]
use metadata_bump::{
    compute_dialog_metadata_bump, compute_item_metadata_bump, compute_metadata_bump,
    compute_world_info_metadata_bump,
};

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
        // Kismet sequences category (1) — disjoint from the others.
        let sequence_overridden = Self::apply_sequence_overrides(&mut categories);
        overridden_elements.extend(sequence_overridden);
        // World info category (12) — disjoint from the others.
        let world_info_overridden = Self::apply_world_info_overrides(&mut categories);
        overridden_elements.extend(world_info_overridden);

        Ok(Self {
            categories: Arc::new(categories),
            overridden_elements: Arc::new(overridden_elements),
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
