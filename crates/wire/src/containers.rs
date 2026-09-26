//! Inventory container capacities.
//!
//! The cell's bandolier slot check reads the table from here so the combat
//! code does not depend on the base's resource loaders.
//!
//! TODO(split): `cimmeria_services::base::resources::bag_max_slots` is still a
//! second copy of this table, kept while `base/resources` moves into
//! `cimmeria-resources` (wave W1b). Dedupe the two when that lands.

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

// Copies of the `base/resources/tests/inventory_slots.rs` pins, which cover
// the base's copy of the table.
#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::inventory::{
        INV_ARTIFACT1, INV_ARTIFACT2, INV_BACK, INV_BANDOLIER, INV_BUYBACK, INV_CHEST,
        INV_CRAFTING, INV_FACE, INV_FEET, INV_HANDS, INV_HEAD, INV_LEGS, INV_MAIN, INV_MISSION,
        INV_NECK, INV_WAIST,
    };

    #[test]
    fn bag_max_slots_known_containers_match_constants() {
        assert_eq!(bag_max_slots(INV_MAIN), 40);
        assert_eq!(bag_max_slots(INV_MISSION), 100);
        assert_eq!(bag_max_slots(INV_BANDOLIER), 4);
        for container_id in [
            INV_HEAD,
            INV_FACE,
            INV_NECK,
            INV_CHEST,
            INV_HANDS,
            INV_WAIST,
            INV_BACK,
            INV_LEGS,
            INV_FEET,
            INV_ARTIFACT1,
            INV_ARTIFACT2,
        ] {
            assert_eq!(bag_max_slots(container_id), 1);
        }
        assert_eq!(bag_max_slots(INV_CRAFTING), 100);
        assert_eq!(bag_max_slots(INV_BUYBACK), 12);
    }

    #[test]
    fn bag_max_slots_out_of_range_returns_zero() {
        assert_eq!(bag_max_slots(0), 0);
        assert_eq!(bag_max_slots(17), 0);
        assert_eq!(bag_max_slots(100), 0);
        assert_eq!(bag_max_slots(-1), 0);
    }
}
