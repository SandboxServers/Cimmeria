//! Inventory container capacities.
//!
//! The one copy of the container-capacity table. The cell's bandolier slot
//! check reads it from here so the combat code does not depend on the base's
//! resource loaders; `cimmeria-resources` re-exports it at its old
//! `base::resources::bag_max_slots` path, and its `inventory_slots` tests pin
//! the values.

/// Max items per container (Constants.py:142-162).
pub fn bag_max_slots(container_id: i32) -> i32 {
    match container_id {
        1 => 40,     // Main
        2 => 100,    // Mission
        3 => 4,      // Bandolier
        4..=14 => 1, // Equipment slots
        15 => 100,   // Crafting
        16 => 12,    // Vendor Buyback
        _ => 0,
    }
}
