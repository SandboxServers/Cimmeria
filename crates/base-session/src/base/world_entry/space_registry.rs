//! Space registry: maps `world_name` -> `space_id`, populated by CellService
//! `SpaceData` messages at startup, plus the set of worlds the cell can
//! deliver a player to (`EnterableWorlds`, which also covers instanced
//! worlds with no startup space). Provides the fallback for the cases where
//! the CellService oneshot path is unavailable, failing closed for any world
//! the cell never announced.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::mercury::world_data::added_worlds::added_world;
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

/// The world a registered space belongs to, for a log line's `world` field
/// (Rule 6). `None` when the cell never announced the space.
///
/// A linear scan of a few dozen rows under the lock, so call it inside the
/// branch that logs. Space ids are allocated monotonically and never reused,
/// so a stale row can't name the wrong world.
pub fn world_for_space(space_id: u32) -> Option<&'static str> {
    let guard = SPACE_REGISTRY.lock().unwrap_or_else(|p| p.into_inner());
    let world = guard
        .iter()
        .find(|(_, &sid)| sid == space_id)
        .map(|(w, _)| w.as_str());
    cimmeria_entity::name_intern::intern_opt(world)
}

/// Worlds the cell can deliver a player to: a startup space or an instanced
/// world (`SpaceManager::world_is_enterable`). Sent once by the cell at
/// startup (`CellToBaseMsg::EnterableWorlds`). Character creation refuses a
/// start profile whose world is not here (Class Start v6, lock L3).
static ENTERABLE_WORLDS: std::sync::LazyLock<Mutex<std::collections::HashSet<String>>> =
    std::sync::LazyLock::new(|| Mutex::new(std::collections::HashSet::new()));

/// Record the worlds the cell can deliver a player to. Additive: a second
/// announcement (or a test) only adds names.
pub fn register_enterable_worlds<I, S>(worlds: I)
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut guard = ENTERABLE_WORLDS.lock().unwrap_or_else(|p| p.into_inner());
    let before = guard.len();
    guard.extend(worlds.into_iter().map(Into::into));
    tracing::debug!(
        added = guard.len() - before,
        total = guard.len(),
        "Registered enterable worlds in BaseApp registry"
    );
}

/// Whether the cell has a space (startup or instanced) for `world`: it
/// announced the world as enterable, or a space of it is registered.
/// Exact, case-sensitive match, like every space lookup.
pub fn is_world_enterable(world: &str) -> bool {
    if ENTERABLE_WORLDS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .contains(world)
    {
        return true;
    }
    SPACE_REGISTRY
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .contains_key(world)
}

/// Space ID fallback, used when the CellService oneshot fails or no cell is
/// attached.
///
/// `None` means there is no safe fallback and the caller must fail closed.
/// That is the answer for every Cimmeria-added world (the historical
/// CellBlocks 1201–1207 and the Debug Area 1300, see
/// [`crate::mercury::world_data::added_worlds`]) and, since Class Start v6
/// CS-02 (lock L3), for every world the cell never announced: an unknown
/// world used to default to the stock `Castle_CellBlock` space, which
/// silently handed the client the Cellblock's space id for an entity the
/// cell never placed there.
///
/// In order:
/// 1. an added world: `None`;
/// 2. a world whose startup space the cell announced (`SpaceData`): that
///    space's real id;
/// 3. the three historical fixed ids (`Castle_CellBlock`, `SGC_W1`,
///    `CombatSim`, the no-cell smoke paths);
/// 4. anything else: `None`, logged at ERROR.
pub fn resolve_space_id_fallback(world_name: &str) -> Option<u32> {
    // The callers log the refusal (`reason = "no_safe_space_fallback"`).
    if added_world(world_name).is_some() {
        return None;
    }
    if let Some(&space_id) = SPACE_REGISTRY
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(world_name)
    {
        return Some(space_id);
    }
    match world_name {
        "Castle_CellBlock" => Some(DEFAULT_SPACE_ID), // 65552
        "SGC_W1" => Some(DEFAULT_SPACE_ID + 1),       // 65553
        "CombatSim" => Some(DEFAULT_SPACE_ID + 2),    // 65554
        _ => {
            tracing::error!(
                world = world_name,
                reason = "unknown_world_no_space",
                "space fallback: the cell announced no space for this world; refusing \
                 instead of defaulting to Castle_CellBlock"
            );
            None
        }
    }
}
