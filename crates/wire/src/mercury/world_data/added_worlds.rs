//! Worlds Cimmeria adds to the game: world ids the shipped client catalogue
//! (`CookedWorldInfo.pak`) has never had.
//!
//! - **Historical CellBlocks (1201–1207)**, map archaeology. Each loads one
//!   recoverable historical state of the CellBlock map from its own client
//!   mapset, `C<build>_CellBlock`, which the historical CellBlock client patch
//!   installs beside the stock `Castle_CellBlock`. Deliberately empty. See
//!   `docs/analysis/historical-cellblocks/README.md`.
//! - **DebugArea (1300)**, the GM test map: a new world on the shipped client
//!   map `Ihpet_Crater_Light`, shared and always loaded, GM-only travel. See
//!   `docs/analysis/debug-area/README.md` (decisions D-DA1..D-DA6).
//!
//! This table is the only Rust copy of the contract. Everything else that
//! names these worlds is data, checked against it by tests:
//!
//! - [`super::world_id_for_name`] and [`super::client_map_for_world`], which
//!   build `setupWorldParameters` (`worldId`, the client's current world) and
//!   `onClientMapLoad` (`mapPath`, the package it loads);
//! - the category-12 `CookedWorldInfo` entries the server pushes
//!   (`cimmeria_resources::base::world_info_overrides`), which is how the
//!   client learns the id exists at all;
//! - the `resources.worlds` seed rows (`db/resources/Worlds/Seed/worlds.sql`);
//! - `entities/spaces.xml` (and `entities/cell_spaces.xml` for a shared world);
//! - the base-side space fallback, which fails closed for every added world
//!   rather than landing it in the stock CellBlock space.
//!
//! The typed world name differs from the client package name for every
//! entry, so [`super::client_map_for_world`] must never fall through to its
//! `world_name` default for these worlds.

/// Where an added world comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddedWorldOrigin {
    /// A historical CellBlock state from CME build `build`. Its client folder
    /// is `C<build>_CellBlock`.
    HistoricalCellBlock { build: u32 },
    /// The Debug Area (D-DA1): a new world id on a map the client ships.
    DebugArea,
}

/// One Cimmeria-added world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddedWorld {
    /// `resources.worlds.world_id`, the `worldId` sent in
    /// `setupWorldParameters` (what the client takes as its current world;
    /// `onClientMapLoad` repeats it and the client discards that copy) and the
    /// cooked catalogue key (`_<world_id>`).
    pub world_id: i32,
    /// World name a GM types and the name the server keys the world by.
    pub world: &'static str,
    /// Client package the map loads from (`mapPath` in `onClientMapLoad`).
    /// The client skips the UE3 load when `mapPath` equals the map it already
    /// has, so travel between this world and the shipped world on the same map
    /// keeps the loaded level (DA-06 checks the result).
    pub client_map: &'static str,
    /// `Flags` of the pushed `COOKED_WORLD_INFO` entry. Copied from the
    /// shipped entry of the map the world plays on: 1 for the CellBlock
    /// (`_12`), 0 for Ihpet_Crater_Light (`_73`).
    pub world_info_flags: u32,
    pub origin: AddedWorldOrigin,
}

impl AddedWorld {
    pub const fn is_historical_cellblock(&self) -> bool {
        matches!(self.origin, AddedWorldOrigin::HistoricalCellBlock { .. })
    }
}

/// World id of the Debug Area (D-DA1).
pub const DEBUG_AREA_WORLD_ID: i32 = 1300;
/// World name of the Debug Area, as typed in `.gotolocation DebugArea`.
pub const DEBUG_AREA_WORLD: &str = "DebugArea";

const fn historical(
    build: u32,
    world_id: i32,
    world: &'static str,
    client_map: &'static str,
) -> AddedWorld {
    AddedWorld {
        world_id,
        world,
        client_map,
        world_info_flags: 1,
        origin: AddedWorldOrigin::HistoricalCellBlock { build },
    }
}

/// Every world Cimmeria adds, by ascending world id.
///
/// The historical CellBlocks: 60130, 62429 and 63682 were rebuilt from 58674
/// with CME's own VPatch deltas; the other four are complete builds. The
/// older 45032→49486 delta chain is not here because its source mapset is
/// missing.
pub const ADDED_WORLDS: [AddedWorld; 8] = [
    historical(43485, 1201, "CellBlock43", "C43485_CellBlock"),
    historical(55124, 1202, "CellBlock55", "C55124_CellBlock"),
    historical(57050, 1203, "CellBlock57", "C57050_CellBlock"),
    historical(58674, 1204, "CellBlock58", "C58674_CellBlock"),
    historical(60130, 1205, "CellBlock60", "C60130_CellBlock"),
    historical(62429, 1206, "CellBlock62", "C62429_CellBlock"),
    historical(63682, 1207, "CellBlock63", "C63682_CellBlock"),
    AddedWorld {
        world_id: DEBUG_AREA_WORLD_ID,
        world: DEBUG_AREA_WORLD,
        client_map: "Ihpet_Crater_Light",
        world_info_flags: 0,
        origin: AddedWorldOrigin::DebugArea,
    },
];

/// The added world named `world_name`, if it is one.
///
/// Exact match, like the rest of the base-side world tables: travel commands
/// canonicalise to the `spaces.xml` spelling before the name reaches here.
pub fn added_world(world_name: &str) -> Option<&'static AddedWorld> {
    ADDED_WORLDS.iter().find(|w| w.world == world_name)
}
