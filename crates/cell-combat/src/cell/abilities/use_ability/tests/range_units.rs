//! #919: ability ranges are UE3 units in the data and metres on the server.
//!
//! 1652 Jaffa: Double Blast carries `max_range = 3000`. Compared raw against
//! metre positions that was a 3 km reach; converted it is 30 m. These fire a
//! 3000-unit ability at a hostile target at 29 m, 31 m and 3000 m and check
//! which casts hit the `OutsideWeaponRange` refusal.

use super::*;
use cimmeria_entity::abilities::ability_range_to_metres;

/// Fire ability 1652 (seeded `max_range` 3000 UE3 units, converted the way
/// the loader converts it) from the origin at a hostile NPC `distance`
/// metres away. Returns whether the cast drew the out-of-range refusal.
async fn refused_as_out_of_range(distance: f32) -> bool {
    let mut def = make_ability(1652, 0, 0);
    def.max_range = ability_range_to_metres(3000);
    fire_at_hostile(&def, distance).await
}

#[tokio::test]
async fn ue3_range_3000_reaches_a_target_at_29_metres() {
    assert!(
        !refused_as_out_of_range(29.0).await,
        "3000 UE3 units is 30 m: a target at 29 m is in range"
    );
}

/// Revert proof: with the raw 3000 compared against metres, both of these
/// casts pass the range check.
#[tokio::test]
async fn ue3_range_3000_refuses_targets_past_30_metres() {
    assert!(
        refused_as_out_of_range(31.0).await,
        "3000 UE3 units is 30 m: a target at 31 m is out of range"
    );
    assert!(
        refused_as_out_of_range(3000.0).await,
        "a target 3000 m away must be out of range, not at the edge of a 3 km reach"
    );
}
