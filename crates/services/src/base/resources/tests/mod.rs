//! Tests for the `resources` module, split by theme (issue #529):
//! - [`category_map`]: client-registration category numbering + behavior-event wire tag.
//! - [`inventory_slots`]: bag-slot helpers + `pick_first_open_bag`.
//! - [`committed_paks`]: invariants on the PAK files under `data/cache/`.
//! - [`overrides`]: mission + item override application and metadata bumps.
//! - [`dialog_overrides`]: dialog override application and metadata bumps.

mod category_map;
mod committed_paks;
mod dialog_overrides;
mod inventory_slots;
mod overrides;
