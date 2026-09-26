//! # cimmeria-cell-catalog
//!
//! The cell's database-backed startup catalogs, read once at boot:
//!
//! - [`cell::spawner`]: the loaders behind every `SpaceManager` cache —
//!   spawn records and entity templates, missions and objectives, dialog
//!   sets, stargates, regions, respawners, loot, item and weapon defs,
//!   ability and effect defs, eye heights, and the per-world rows
//!   (`world_id`, `NavmeshMode`).
//! - [`ability_tree`]: the archetype ability-tree catalog and the one
//!   trainability predicate the trainer window and the purchase gate share.
//! - [`cell::respawner_fallback`]: the nearest-valid-respawner search the
//!   arrival and movement-recovery paths share.
//!
//! Populating spaces from these catalogs is `SpaceManager` work and stays
//! above this crate. Split out of `cimmeria-services` (wave W2b of
//! `docs/architecture/services-crate-split.md`). The module tree keeps its
//! old nesting, so `crate::cell::spawner::…` and `super::…` paths inside it
//! are unchanged, and `cimmeria-services` re-exports each module at its old
//! path (`cimmeria_services::cell::spawner`, `cimmeria_services::ability_tree`).

#![warn(unreachable_pub)]

pub mod ability_tree;

/// The cell-side catalogs, under the `cell::` path they had in
/// `cimmeria-services`.
pub mod cell {
    pub mod respawner_fallback;
    pub mod spawner;
}

// Generic helpers come from `cimmeria-test-support` (a dev-dependency), so the
// moved tests keep importing them from `crate::test_support`.
#[cfg(test)]
mod test_support {
    pub(crate) use cimmeria_test_support::*;
}
