//! World entry builders and data: map loading, world parameters, archetype stats,
//! ability trees, and the `mapLoaded()` multi-packet sequence.

// ── Submodules ───────────────────────────────────────────────────────────────

pub mod added_worlds;
pub mod historical_cellblocks;
mod map_loaded;
mod phases;
mod stats;

#[cfg(test)]
mod tests;

// ── Re-exports ───────────────────────────────────────────────────────────────
// Matches the public API that mercury/mod.rs imports from `world_data::*`.

pub use phases::{
    build_create_player, build_enter_world, build_enter_world_body, build_on_player_data_loaded,
    build_setup_world_parameters,
};

pub use map_loaded::{
    build_map_loaded, build_map_loaded_body, fragment_count, fragment_map_loaded,
    world_entry_stat_args, WORLD_ENTRY_STATE_FIELD,
};

pub use stats::archetype_stats;

// ── Shared imports from parent (mercury) ─────────────────────────────────────
// Used by submodules via `super::`.

pub(crate) use super::types::{ArchetypeStats, PlayerLoadData, WorldEntryInfo};
pub(crate) use super::{
    append_entity_method, encrypt_packet, method_idx, write_wstring, BASEMSG_CREATE_BASE_PLAYER,
    BASEMSG_CREATE_CELL_PLAYER, BASEMSG_FORCED_POSITION, BASEMSG_SPACE_VIEWPORT_INFO, REPLY_FLAGS,
    SKIN_TINTS,
};

// ── Data lookup functions ────────────────────────────────────────────────────
// Used by both phases.rs and map_loaded.rs, so they live here.

/// Look up the world_id for a world name (from db/resources/Worlds/Seed/worlds.sql).
///
/// The id the client keeps as its current world is `setupWorldParameters`'
/// `worldId` (`getCurrentWorldID`: minimap name, world map, mission and squad
/// map markers). `onClientMapLoad` also carries it, but the client reads that
/// copy and discards it (SGW.exe `FUN_00df27f0`; see the bigworld-engine-advisor
/// memory `client-world-id-and-same-map-load`). An unknown name is logged and
/// answered with 1 (CombatSim), so the client would label the world as
/// CombatSim; every `spaces.xml` world must resolve here
/// (`every_declared_space_resolves_to_its_seed_world_id_and_client_map`
/// guards that).
pub(crate) fn world_id_for_name(world_name: &str) -> i32 {
    known_world_id(world_name).unwrap_or_else(|| {
        tracing::warn!(world = %world_name, "Unknown world_id — using 1 (CombatSim)");
        1
    })
}

/// The `resources.worlds.world_id` of `world_name`, or `None` for a name this
/// table does not know. Exact, case-sensitive match on the `spaces.xml`
/// spelling. For logging a world id next to a world name on paths that run
/// before the cell has its `resources.worlds` rows.
pub fn known_world_id(world_name: &str) -> Option<i32> {
    if let Some(world) = added_worlds::added_world(world_name) {
        return Some(world.world_id);
    }
    Some(match world_name {
        "CombatSim" => 1,
        "SandBox" => 2,
        "Tol-Alpha-00" => 3,
        "Tol-Alpha-01" => 4,
        "Tol-Alpha-02" => 5,
        "Ca-Alpha-00" => 6,
        "Ca-Alpha-01" => 7,
        "Castle" => 8,
        "Tol-POI-06" => 9,
        "Agnos" => 10,
        "Anima_Vitrus" => 11,
        "Castle_CellBlock" => 12,
        "Hebridan" => 13,
        "Kheb" => 14,
        "Lucia" => 15,
        "Naitac" => 16,
        "PrimHatak" => 17,
        "Omega_Site" => 18,
        "Tollana" => 19,
        "Agnos_Library" => 20,
        "Playground" => 21,
        "TestSGC1" => 22,
        "Beta_Site_Evo_1" => 23,
        "Dakara" => 24,
        "Sewer_Falls" => 50,
        "Harset" => 57,
        "SGC_W1" => 58,
        "Dakara_E1" => 61,
        "Dakara_E1_StoryRm" => 62,
        "Harset_CmdCenter" => 68,
        "Harset_Market" => 69,
        "Harset_StorageRm" => 70,
        "Ihpet_Crater_Dark" => 72,
        "Ihpet_Crater_Light" => 73,
        "Menfa_Dark" => 77,
        "Menfa_Light" => 78,
        "Omega_Site_CmdCenter" => 80,
        "Pertho" => 83,
        "SGC" => 86,
        "SGC_W2" => 87,
        "Tollana_Curia" => 88,
        "Temple" => 89,
        "Yotunheim" => 90,
        "Holding_Area" => 91,
        "Vitrus" => 92,
        _ => return None,
    })
}

/// Look up the client terrain path for a world name (client_map from worlds.sql).
/// Most worlds use the same name; a few differ.
///
/// Every Cimmeria-added world differs (`CellBlock43` loads
/// `C43485_CellBlock`, `DebugArea` loads `Ihpet_Crater_Light`), so they
/// resolve explicitly and never reach the `world_name` default, which would
/// ask the client for a package that does not exist.
///
/// Public because the cell reads a world's navmesh and occluder from its
/// client map's files when the world has none of its own (D-DA5).
pub fn client_map_for_world(world_name: &str) -> &str {
    if let Some(world) = added_worlds::added_world(world_name) {
        return world.client_map;
    }
    match world_name {
        "CombatSim" => "Combat_Terrain_Test",
        "SandBox" => "Harset_CmdCenter",
        "Tol-Alpha-00" => "Tol-Alpha_Pocket_00",
        "Tol-Alpha-01" => "Tol-Alpha_Pocket_01",
        "Tol-Alpha-02" => "Tol-Alpha_Pocket_02",
        "Ca-Alpha-00" => "Ca-Alpha_Pocket_00",
        "Ca-Alpha-01" => "Ca-Alpha_Pocket_01",
        "Tol-POI-06" => "Tol_POI_Test06",
        _ => world_name, // Most worlds: client_map == world_name
    }
}

/// Serialize setupWorldParameters argument payload (22 args from World.py defaults).
pub(crate) fn build_world_params_args(world_name: &str) -> Vec<u8> {
    let mut args = Vec::with_capacity(88);
    args.extend_from_slice(&world_id_for_name(world_name).to_le_bytes()); // worldId
    args.extend_from_slice(&0i32.to_le_bytes()); // weatherSetId
    args.extend_from_slice(&1i32.to_le_bytes()); // minToRealMinutes
    args.extend_from_slice(&1440i32.to_le_bytes()); // minutesPerDay
    args.extend_from_slice(&100000i32.to_le_bytes()); // currentTimeInSeconds
    args.extend_from_slice(&(-9.8f32).to_le_bytes()); // gravity
    args.extend_from_slice(&6.0f32.to_le_bytes()); // runSpeed
    args.extend_from_slice(&4.0f32.to_le_bytes()); // sidewaysRunSpeed
    args.extend_from_slice(&3.0f32.to_le_bytes()); // backwardsRunSpeed
    args.extend_from_slice(&2.0f32.to_le_bytes()); // walkSpeed
    args.extend_from_slice(&1.5f32.to_le_bytes()); // sidewaysWalkSpeed
    args.extend_from_slice(&1.0f32.to_le_bytes()); // backwardsWalkSpeed
    args.extend_from_slice(&3.0f32.to_le_bytes()); // crouchRunSpeed
    args.extend_from_slice(&2.0f32.to_le_bytes()); // sidewaysCrouchRunSpeed
    args.extend_from_slice(&1.5f32.to_le_bytes()); // backwardsCrouchRunSpeed
    args.extend_from_slice(&1.5f32.to_le_bytes()); // crouchWalkSpeed
    args.extend_from_slice(&1.0f32.to_le_bytes()); // sidewaysCrouchWalkSpeed
    args.extend_from_slice(&0.75f32.to_le_bytes()); // backwardsCrouchWalkSpeed
    args.extend_from_slice(&4.0f32.to_le_bytes()); // swimSpeed
    args.extend_from_slice(&2.5f32.to_le_bytes()); // sidewaysSwimSpeed
    args.extend_from_slice(&2.0f32.to_le_bytes()); // backwardsSwimSpeed
    args.extend_from_slice(&8.0f32.to_le_bytes()); // jumpSpeed
    args
}
