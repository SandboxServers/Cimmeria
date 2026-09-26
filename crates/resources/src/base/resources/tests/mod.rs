//! Tests for the `resources` module, split by theme (issue #529):
//! - [`category_map`]: client-registration category numbering. The behavior-event
//!   wire-tag test drives the fragment builder, so it stayed in `cimmeria-services`
//!   (`base::resource_fragment_tests`).
//! - [`inventory_slots`]: bag-slot helpers + `pick_first_open_bag`.
//! - [`committed_paks`]: invariants on the PAK files under `data/cache/`.
//! - [`overrides`]: mission + item override application and metadata bumps.
//! - [`dialog_overrides`]: dialog override application and metadata bumps.

mod category_map;
mod committed_paks;
mod dialog_overrides;
mod inventory_slots;
mod overrides;
