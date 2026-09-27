//! Inventory container capacities.
//!
//! The cell's bandolier slot check reads the capacity table from here so
//! the combat code does not depend on the base's resource loaders;
//! `cimmeria-resources` re-exports it again at its old
//! `base::resources::bag_max_slots` path, and its `inventory_slots` tests pin
//! the values.
//!
//! The table itself is `cimmeria_entity::inventory::bag_max_slots`, the one
//! copy (D-BV06). It moved there so `BAG_SIZES`, which `onBagInfo` sends,
//! can be derived from it: this crate depends on `cimmeria-entity`, so the
//! table has to live in the lower crate.

pub use cimmeria_entity::inventory::bag_max_slots;
