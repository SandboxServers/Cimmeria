//! Historical Castle CellBlock worlds (1201–1207) for map archaeology.
//!
//! Each world loads one recoverable historical state of the CellBlock map
//! from its own client mapset, `C<build>_CellBlock`. The historical CellBlock
//! client patch installs those mapsets beside the stock `Castle_CellBlock`, so
//! a GM can `.gotolocation CellBlock43 …` and compare builds in game before
//! anything is restored into stock world 12.
//!
//! The worlds are deliberately empty: no missions, spawns, cover, respawners
//! or ring routes are seeded for them.
//!
//! This table is the only Rust copy of the contract. Everything else that
//! names these worlds is data, checked against it by tests:
//!
//! - the `resources.worlds` seed rows (`db/resources/Worlds/Seed/worlds.sql`);
//! - `entities/spaces.xml`, with `Instanced="true"` and never an entry in
//!   `entities/cell_spaces.xml`;
//! - the category-12 `CookedWorldInfo` entries the server pushes
//!   (`cimmeria_resources::base::world_info_overrides`);
//! - the base-side space fallback, which must fail closed for these names
//!   rather than land them in the stock CellBlock space.
//!
//! The typed world name differs from the client package name on purpose, so
//! [`super::client_map_for_world`] must never fall through to its
//! `world_name` default for these worlds. See
//! `docs/analysis/historical-cellblocks/README.md`.

/// One historical CellBlock world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoricalCellBlock {
    /// CME build the mapset comes from.
    pub build: u32,
    /// `resources.worlds.world_id`, and the `WorldID` sent in `onClientMapLoad`.
    pub world_id: i32,
    /// World name a GM types and the name the server keys the world by.
    pub world: &'static str,
    /// Client package the map loads from (`mapPath` in `onClientMapLoad`).
    pub client_map: &'static str,
}

/// The seven recoverable historical CellBlock states, oldest build first.
///
/// 60130, 62429 and 63682 were rebuilt from 58674 with CME's own VPatch
/// deltas; the other four are complete builds. The older 45032→49486 delta
/// chain is not here because its source mapset is missing.
pub const HISTORICAL_CELLBLOCKS: [HistoricalCellBlock; 7] = [
    HistoricalCellBlock {
        build: 43485,
        world_id: 1201,
        world: "CellBlock43",
        client_map: "C43485_CellBlock",
    },
    HistoricalCellBlock {
        build: 55124,
        world_id: 1202,
        world: "CellBlock55",
        client_map: "C55124_CellBlock",
    },
    HistoricalCellBlock {
        build: 57050,
        world_id: 1203,
        world: "CellBlock57",
        client_map: "C57050_CellBlock",
    },
    HistoricalCellBlock {
        build: 58674,
        world_id: 1204,
        world: "CellBlock58",
        client_map: "C58674_CellBlock",
    },
    HistoricalCellBlock {
        build: 60130,
        world_id: 1205,
        world: "CellBlock60",
        client_map: "C60130_CellBlock",
    },
    HistoricalCellBlock {
        build: 62429,
        world_id: 1206,
        world: "CellBlock62",
        client_map: "C62429_CellBlock",
    },
    HistoricalCellBlock {
        build: 63682,
        world_id: 1207,
        world: "CellBlock63",
        client_map: "C63682_CellBlock",
    },
];

/// The historical CellBlock world named `world_name`, if it is one.
///
/// Exact match, like the rest of the base-side world tables: travel commands
/// canonicalise to the `spaces.xml` spelling before the name reaches here.
pub fn historical_cellblock(world_name: &str) -> Option<&'static HistoricalCellBlock> {
    HISTORICAL_CELLBLOCKS.iter().find(|w| w.world == world_name)
}
