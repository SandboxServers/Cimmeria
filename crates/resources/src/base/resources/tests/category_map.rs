//! Category-map tests for the cooked-data enum.
//!
//! These pin the client's resource-category numbering so a future drift
//! back toward the legacy `resource.cpp` table (which reserved 21 for
//! `pet_command` and pushed `behavior_event` to 22) fails loudly instead
//! of silently dropping behavior-event fragments on the wire.

use super::super::{CATEGORY_BEHAVIOR_EVENTS, CATEGORY_PAKS};

/// The client must be served a *contiguous* 1..=21 enum — no category 0,
/// no category 22, no `pet_command`. The legacy `resource.cpp` table had 22
/// entries and drifted at the high end.
#[test]
fn category_paks_cover_exactly_the_client_1_to_21_enum() {
    let mut ids: Vec<u32> = CATEGORY_PAKS.iter().map(|&(id, _)| id).collect();
    ids.sort_unstable();
    let expected: Vec<u32> = (1..=21).collect();
    assert_eq!(
        ids, expected,
        "Rust category map must match the client's contiguous 1..=21 registration",
    );
}

/// Category 21 must be `behavior_event` (`CookedBehaviorEvents.pak`), not
/// `pet_command` (legacy) and not 22 (legacy drift).
#[test]
fn behavior_event_sits_at_category_21_with_its_pak() {
    let (id, pak) = CATEGORY_PAKS
        .iter()
        .find(|&&(_, pak)| pak == "CookedBehaviorEvents.pak")
        .copied()
        .expect("CookedBehaviorEvents.pak must be registered");
    assert_eq!(
        id, CATEGORY_BEHAVIOR_EVENTS,
        "CookedBehaviorEvents.pak must be category {}, not 22",
        CATEGORY_BEHAVIOR_EVENTS,
    );
    assert_eq!(
        pak, "CookedBehaviorEvents.pak",
        "the pak backing the behavior-event category",
    );
    assert_eq!(
        CATEGORY_BEHAVIOR_EVENTS, 21,
        "the behavior-event category id must be 21",
    );
}
