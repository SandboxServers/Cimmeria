//! Tests for the `resources` module, split by theme (issue #529):
//! - [`category_map`]: client-registration category numbering + behavior-event wire tag.
//! - [`inventory_slots`]: bag-slot helpers + `pick_first_open_bag`.
//! - [`overrides`]: cooked-data override application + metadata bumps.

mod category_map;
mod inventory_slots;
mod overrides;
