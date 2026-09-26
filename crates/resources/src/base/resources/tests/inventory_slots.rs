//! Inventory bag-slot helper tests: `bag_min_slot`, `bag_max_slots`, and
//! `pick_first_open_bag`.
//!
//! Split out of the monolithic `resources/tests.rs` (issue #529) — every
//! test body and assertion is byte-identical to the original.

use super::super::*;
use cimmeria_entity::inventory::{
    INV_ARTIFACT1, INV_ARTIFACT2, INV_BACK, INV_BANDOLIER, INV_BUYBACK, INV_CHEST, INV_CRAFTING,
    INV_FACE, INV_FEET, INV_HANDS, INV_HEAD, INV_LEGS, INV_MAIN, INV_MISSION, INV_NECK, INV_WAIST,
};

/// Sentinel-slot invariant: the move handler's three-step swap parks the
/// source row at `slot_id = -1` mid-transaction. For that to be safe,
/// `bag_min_slot` MUST never return a value `<= -1` for any container —
/// otherwise a concurrent grant could legitimately reserve `slot_id = -1`
/// and collide with the parked sentinel.
///
/// This test pins that invariant across `container_id` 0..=16. The
/// game-defined range is 1..=16 (main, mission, bandolier, equipment
/// slots 4..=14, crafting, vendor buyback); 0 is included as a sentinel
/// for "no container" so the symmetry with `bag_max_slots` (which
/// returns 0 for 0) is also exercised. Any out-of-range container_id
/// returning 0 from `bag_min_slot` is fine — `bag_max_slots` returns 0
/// there too, so no slot is ever reservable.
///
/// Documented as the regression guard in `move_/mod.rs`'s swap-path
/// comment (`bag_max_slots() never reserves negative slots, so
/// grant/purchase paths cannot land there`).
#[test]
fn bag_min_slot_is_non_negative_for_every_container() {
    for container_id in 0..=16 {
        let min = bag_min_slot(container_id);
        assert!(
            min >= 0,
            "bag_min_slot({container_id}) returned {min}; must be >= 0"
        );
    }
}

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

#[test]
fn bag_min_slot_bandolier_is_zero() {
    assert_eq!(
        bag_min_slot(INV_BANDOLIER),
        0,
        "bandolier slots are zero-based weapon slots"
    );
}

#[test]
fn bag_min_slot_other_containers_is_zero() {
    for container_id in [1, 2, 4, 15, 16] {
        assert_eq!(
            bag_min_slot(container_id),
            0,
            "container {container_id} must start at slot 0"
        );
    }
}

// ── pick_first_open_bag ─────────────────────────────────────────────────────

/// Happy path: a bag with room in the item's container set is picked.
#[test]
fn pick_first_open_bag_picks_first_valid_when_empty() {
    let slot_indices = HashMap::new();
    // Item can live in INV_MAIN (1) or INV_BANDOLIER (3). BAG_FILL_ORDER
    // visits BANDOLIER (3) before MAIN (1), so BANDOLIER wins.
    let container_sets = vec![INV_MAIN, INV_BANDOLIER];
    assert_eq!(
        pick_first_open_bag(&container_sets, &slot_indices),
        Some(INV_BANDOLIER),
        "BANDOLIER comes before MAIN in BAG_FILL_ORDER and both are empty"
    );
}

/// **Regression guard for the live 2026-06-02 bug:** when the
/// fill-order-preferred bag is full but a later valid bag still has
/// room, the picker must overflow to the later bag rather than dropping
/// the item.
///
/// Pre-fix the inline `.find()` in `character_create` only checked
/// "does this bag accept this item type" — it ignored fullness — so
/// once the preferred bag filled up, every subsequent same-type item
/// got `continue`'d. Real impact: starter item 4343 disappeared at
/// character create.
///
/// Reverting `pick_first_open_bag` to ignore fullness must fail this
/// assertion.
#[test]
fn pick_first_open_bag_overflows_when_preferred_bag_is_full() {
    // Item can live in BANDOLIER (3, max 4) or MAIN (1, max 40).
    let container_sets = vec![INV_MAIN, INV_BANDOLIER];
    let mut slot_indices = HashMap::new();
    // Bandolier already at max: next_slot would be 4, == bag_max_slots(3).
    slot_indices.insert(INV_BANDOLIER, bag_max_slots(INV_BANDOLIER));
    assert_eq!(
        pick_first_open_bag(&container_sets, &slot_indices),
        Some(INV_MAIN),
        "BANDOLIER full → overflow to MAIN. Reverting to the pre-fix \
         'ignore fullness' selection would return BANDOLIER and then the \
         caller would drop the item."
    );
}

/// All valid bags full → `None`. Caller is expected to log "all valid
/// containers full" and drop the item, but the picker just reports the
/// fact — keeps the policy where it belongs.
#[test]
fn pick_first_open_bag_returns_none_when_all_valid_bags_full() {
    let container_sets = vec![INV_MAIN, INV_BANDOLIER];
    let mut slot_indices = HashMap::new();
    slot_indices.insert(INV_BANDOLIER, bag_max_slots(INV_BANDOLIER));
    slot_indices.insert(INV_MAIN, bag_max_slots(INV_MAIN));
    assert_eq!(pick_first_open_bag(&container_sets, &slot_indices), None);
}

/// Item with no valid container → `None`. Distinct from "all full" at
/// the caller level (different warn message), but at the picker level
/// the answer is the same.
#[test]
fn pick_first_open_bag_returns_none_when_no_valid_container() {
    // Container 17 is out of BAG_FILL_ORDER entirely.
    let container_sets = vec![17];
    let slot_indices = HashMap::new();
    assert_eq!(pick_first_open_bag(&container_sets, &slot_indices), None);
}

/// Edge case: a bag with one slot free (`next_slot == max - 1`) is
/// picked, and a bag at exactly max is rejected. Guards the boundary
/// arithmetic between `<` and `<=`.
#[test]
fn pick_first_open_bag_boundary_one_slot_free_vs_at_max() {
    let container_sets = vec![INV_BANDOLIER];
    let mut indices = HashMap::new();
    indices.insert(INV_BANDOLIER, bag_max_slots(INV_BANDOLIER) - 1);
    assert_eq!(
        pick_first_open_bag(&container_sets, &indices),
        Some(INV_BANDOLIER),
        "one slot free must be pickable"
    );

    indices.insert(INV_BANDOLIER, bag_max_slots(INV_BANDOLIER));
    assert_eq!(
        pick_first_open_bag(&container_sets, &indices),
        None,
        "exactly at max must NOT be pickable"
    );
}

/// Multi-item placement walk: simulate placing N items, ensuring the
/// picker advances across bags as they fill. End-to-end shape of the
/// production loop in `character_create`.
#[test]
fn pick_first_open_bag_multi_item_walk_fills_then_overflows() {
    let container_sets = vec![INV_MAIN, INV_BANDOLIER];
    let mut slot_indices: HashMap<i32, i32> = HashMap::new();
    let mut placements: Vec<i32> = Vec::new();

    // Place 6 items. First 4 land in BANDOLIER (slots 0..3), next 2
    // land in MAIN (slots 0, 1).
    for _ in 0..6 {
        let bag = pick_first_open_bag(&container_sets, &slot_indices)
            .expect("six placements with this fixture must all succeed");
        placements.push(bag);
        // Mirror the loop's bookkeeping.
        *slot_indices.entry(bag).or_insert_with(|| bag_min_slot(bag)) += 1;
    }
    assert_eq!(
        placements,
        vec![
            INV_BANDOLIER,
            INV_BANDOLIER,
            INV_BANDOLIER,
            INV_BANDOLIER,
            INV_MAIN,
            INV_MAIN
        ],
        "first 4 fill BANDOLIER, next 2 overflow to MAIN"
    );
}
