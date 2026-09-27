//! Space registry: maps `world_name` -> `space_id`, populated by CellService
//! `SpaceData` messages at startup. Provides a hardcoded fallback table for
//! the cases where the CellService oneshot path is unavailable.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::mercury::world_data::historical_cellblocks::historical_cellblock;
use crate::mercury::DEFAULT_SPACE_ID;

/// Thread-safe space registry mapping world_name -> space_id.
/// Populated at startup when CellService sends SpaceData for each space.
static SPACE_REGISTRY: std::sync::LazyLock<Mutex<HashMap<String, u32>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Register a space in the global registry (called from CellToBase message handler).
pub fn register_space(world_name: String, space_id: u32) {
    tracing::debug!(world = %world_name, space_id, "Registered space in BaseApp registry");
    // Recover from a poisoned mutex: a panic mid-mutation would otherwise
    // wedge every subsequent space registration. The HashMap is in a known
    // state (insert is atomic from the caller's perspective), so reusing
    // the inner guard is safe here.
    let mut guard = SPACE_REGISTRY.lock().unwrap_or_else(|p| p.into_inner());
    guard.insert(world_name, space_id);
}

/// Hardcoded space ID fallback (used when CellService oneshot fails or is unavailable).
///
/// `None` means there is no safe fallback and the caller must fail closed.
/// That is the answer for the historical CellBlock worlds (1201–1207): the
/// unknown-world default below is the stock `Castle_CellBlock` space, and a
/// world-entry packet naming it for a player bound for `CellBlock43` would
/// hand the client the stock map's space id for an entity the cell never
/// placed there.
pub fn resolve_space_id_fallback(world_name: &str) -> Option<u32> {
    if historical_cellblock(world_name).is_some() {
        return None;
    }
    Some(match world_name {
        "Castle_CellBlock" => DEFAULT_SPACE_ID, // 65552
        "SGC_W1" => DEFAULT_SPACE_ID + 1,       // 65553
        "CombatSim" => DEFAULT_SPACE_ID + 2,    // 65554
        _ => {
            tracing::warn!("Unknown world_location: {world_name}, defaulting to Castle_CellBlock");
            DEFAULT_SPACE_ID
        }
    })
}
