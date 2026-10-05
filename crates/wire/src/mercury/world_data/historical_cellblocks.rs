//! Historical Castle CellBlock worlds (1201–1207) for map archaeology: the
//! [`super::added_worlds`] entries whose origin is a historical CellBlock
//! build.
//!
//! These have rules of their own that the other added worlds do not share:
//! each is `Instanced="true"` and never a startup space, and
//! `.gotolocation CellBlock43` alone lands on the stock CellBlock's
//! new-character start. See `docs/analysis/historical-cellblocks/README.md`.

use super::added_worlds::{AddedWorld, ADDED_WORLDS};

/// The seven recoverable historical CellBlock states, oldest build first.
pub fn historical_cellblocks() -> impl Iterator<Item = &'static AddedWorld> {
    ADDED_WORLDS.iter().filter(|w| w.is_historical_cellblock())
}

/// The historical CellBlock world named `world_name`, if it is one.
///
/// Exact match, like the rest of the base-side world tables: travel commands
/// canonicalise to the `spaces.xml` spelling before the name reaches here.
pub fn historical_cellblock(world_name: &str) -> Option<&'static AddedWorld> {
    historical_cellblocks().find(|w| w.world == world_name)
}
