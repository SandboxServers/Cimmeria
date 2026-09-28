//! Which categories hold world entry, and the order the background stream
//! pushes the rest in.
//!
//! The client asks for a missing entry (`elementDataRequest`) through a
//! per-category request function, one template instance per category
//! (the `Event_NetOut_elementDataRequest` constructor `0x00cfdeb0` has one
//! caller per instance). A headless decompile on 2026-09-28 found instances
//! for categories 1-11, 13, 14, 15 and 19, and none for 12, 16, 17, 18, 20
//! or 21. A category without one can never recover an entry it looks up
//! before the entry is pushed, so those are held.

/// Categories world entry waits for: the ones with no client miss path.
///
/// - 12 `CookedWorldInfo`: `onClientMapLoad` names a world the client
///   resolves from this table at map load.
/// - 16 Sciences, 17 Disciplines, 18 Paradigm, 20 Interactions, 21 Behavior
///   events: no miss path either.
///
/// Together they are about 230 entries (under a second at the resync's
/// pace), so holding them costs nothing noticeable.
pub const HELD_CATEGORIES: [u32; 6] = [12, 16, 17, 18, 20, 21];

pub fn is_held(category_id: u32) -> bool {
    HELD_CATEGORIES.contains(&category_id)
}

/// Stream order, lowest first: the held categories, then the ones a player
/// hits first in the world (missions, dialogs, items), then the rest, with
/// the 29,000-entry TextStrings last. Misses jump the whole stream anyway.
pub fn rank(category_id: u32) -> u8 {
    if is_held(category_id) {
        return 0;
    }
    match category_id {
        3 => 1,  // missions
        5 => 2,  // dialogs
        4 => 3,  // items
        10 => 9, // TextStrings
        _ => 5,
    }
}
