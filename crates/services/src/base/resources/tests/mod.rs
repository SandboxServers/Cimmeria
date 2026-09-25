//! Tests for the `resources` module, split by theme (issue #529):
//! - [`inventory_slots`]: bag-slot helpers + `pick_first_open_bag`.
//! - [`committed_paks`]: invariants on the PAK files under `data/cache/`.
//! - [`overrides`]: cooked-data override application + metadata bumps.

mod committed_paks;
mod inventory_slots;
mod overrides;
