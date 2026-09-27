//! Where a granted item lands, from its `resources.items.container_sets`.
//!
//! `container_sets` lists the containers an item may sit in. For 752
//! crafting components the list is `{17,15}`: the bank first, then the
//! crafting bag. A grant that takes the first entry tries to put the item
//! into the bank, which is never a grant target, so the grant is refused.
//! [`first_player_container`] skips the storage containers and picks the
//! first bag the player carries. [`grant_container`] applies that to a
//! grant that asked for a container (loot, content, GM, vendors), and
//! [`default_grant_container`] is what the cell asks for when nothing
//! names one.

/// The player's main bag (`INV_Main`).
pub const INV_MAIN: i32 = 1;

/// The crafting bag (`INV_Crafting`).
pub const INV_CRAFTING: i32 = 15;

/// Storage containers: the bank (17) and the other stores up to 20. A grant
/// never places an item into one of them.
pub const STORAGE_CONTAINERS: std::ops::RangeInclusive<i32> = 17..=20;

/// The vendor buyback list (`INV_Buyback`). Only a sale puts an item here;
/// a grant never does.
pub const INV_BUYBACK: i32 = 16;

/// Containers a grant never writes: buyback and the storage containers.
pub fn never_granted(container_id: i32) -> bool {
    container_id == INV_BUYBACK || STORAGE_CONTAINERS.contains(&container_id)
}

/// The first carried bag an item may be granted into.
///
/// Walks `container_sets` in order and returns the first entry that is the
/// main bag (1) or the crafting bag (15), so storage containers
/// ([`STORAGE_CONTAINERS`]) listed ahead of it are passed over. An empty
/// list means the item is unrestricted, so it goes to the main bag. `None`
/// means the item may only sit in a storage, bandolier, equipment or
/// mission container, and a grant into a carried bag must be refused.
///
/// `{17,15}` gives 15, `{1,17}` gives 1, `{15,1}` gives 15, `{}` gives 1,
/// `{17}` and `{2}` give `None`.
pub fn first_player_container(container_sets: &[i32]) -> Option<i32> {
    if container_sets.is_empty() {
        return Some(INV_MAIN);
    }
    container_sets
        .iter()
        .copied()
        .find(|&c| c == INV_MAIN || c == INV_CRAFTING)
}

/// The container a grant lands in when its caller asked for `requested`.
///
/// - Buyback and the storage containers ([`never_granted`]) are never
///   written by a grant, so the grant falls through to
///   [`first_player_container`], which only ever gives 1 or 15. `None`
///   means the item lists no carried bag, and the grant is refused; it never
///   falls through to another vault or to buyback.
/// - A carried bag (1 or 15) is kept when the item may sit there (it is
///   listed, or the list is empty and the bag is the main bag). Otherwise
///   [`first_player_container`] picks the bag; an item that lists no
///   carried bag (a mission item, say) keeps the request.
/// - Any other container (mission bag, bandolier, equipment slot) is the
///   caller's explicit choice and is kept.
///
/// `({17,15}, 17)` and `({17,15}, 1)` give 15; `({3,1,17}, 1)` gives 1;
/// `({3,1,17}, 3)` gives 3; `({17}, 17)` gives `None`.
pub fn grant_container(container_sets: &[i32], requested: i32) -> Option<i32> {
    if never_granted(requested) {
        return first_player_container(container_sets);
    }
    if requested != INV_MAIN && requested != INV_CRAFTING {
        return Some(requested);
    }
    let allowed = if container_sets.is_empty() {
        requested == INV_MAIN
    } else {
        container_sets.contains(&requested)
    };
    if allowed {
        return Some(requested);
    }
    Some(first_player_container(container_sets).unwrap_or(requested))
}

/// The container a cell-side grant asks for when nothing names one: the
/// first `container_sets` entry that is not buyback or a storage container.
///
/// `{17,15}` gives 15, `{3,1,17}` gives 3 (weapons go to the bandolier),
/// `{2}` gives 2. An item that lists only storage containers gives its
/// first entry, so the grant still reaches the base and is refused there.
/// An empty list gives `None`: the caller's main-bag default applies.
pub fn default_grant_container(container_sets: &[i32]) -> Option<i32> {
    container_sets
        .iter()
        .copied()
        .find(|&c| !never_granted(c))
        .or_else(|| container_sets.first().copied())
}

/// Whether a grant placed at `chosen` passed over a container it never
/// writes (buyback or storage): the request named one, or the item lists
/// one ahead of `chosen`.
pub fn skipped_storage(container_sets: &[i32], requested: i32, chosen: i32) -> bool {
    (never_granted(requested) && requested != chosen)
        || container_sets
            .iter()
            .take_while(|&&c| c != chosen)
            .any(|&c| never_granted(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The crafting-component shape: the bank is listed first, and the
    /// grant must fall through to the crafting bag, not refuse.
    #[test]
    fn bank_first_component_lands_in_the_crafting_bag() {
        assert_eq!(first_player_container(&[17, 15]), Some(INV_CRAFTING));
    }

    #[test]
    fn main_bag_items_keep_the_main_bag() {
        assert_eq!(first_player_container(&[1, 17]), Some(INV_MAIN));
        assert_eq!(first_player_container(&[1, 7, 17]), Some(INV_MAIN));
    }

    /// List order decides, not a fixed preference between 1 and 15.
    #[test]
    fn first_listed_carried_bag_wins() {
        assert_eq!(first_player_container(&[15, 1]), Some(INV_CRAFTING));
        assert_eq!(first_player_container(&[18, 1, 15]), Some(INV_MAIN));
    }

    /// A weapon lists the bandolier first; a grant into a carried bag
    /// takes the main bag behind it.
    #[test]
    fn bandolier_and_equipment_entries_are_skipped() {
        assert_eq!(first_player_container(&[3, 1, 17]), Some(INV_MAIN));
    }

    #[test]
    fn unrestricted_item_goes_to_the_main_bag() {
        assert_eq!(first_player_container(&[]), Some(INV_MAIN));
    }

    #[test]
    fn storage_or_mission_only_items_have_no_carried_bag() {
        for storage in STORAGE_CONTAINERS {
            assert_eq!(first_player_container(&[storage]), None, "{storage}");
        }
        assert_eq!(first_player_container(&[2]), None);
        assert_eq!(first_player_container(&[17, 3]), None);
    }

    /// Every caller's request shape for the `{17,15}` component: the old
    /// cell cache asked for 17, the GM path asks for 1, the new cache asks
    /// for 15. All three land in the crafting bag.
    #[test]
    fn bank_first_component_grant_falls_through_to_the_crafting_bag() {
        for requested in [17, INV_MAIN, INV_CRAFTING] {
            assert_eq!(
                grant_container(&[17, 15], requested),
                Some(INV_CRAFTING),
                "request {requested}"
            );
        }
    }

    #[test]
    fn storage_only_grant_is_refused() {
        for storage in STORAGE_CONTAINERS {
            assert_eq!(grant_container(&[storage], storage), None);
        }
        assert_eq!(grant_container(&[INV_BUYBACK], INV_BUYBACK), None);
    }

    /// Whatever the item lists and whatever the caller asks for, a grant
    /// that asked for buyback or a vault lands in 1 or 15 or is refused:
    /// it never falls through to another vault or to buyback.
    #[test]
    fn fall_through_never_lands_in_buyback_or_a_vault() {
        let shapes: &[&[i32]] = &[
            &[17, 15],
            &[17, 18, 19, 20, 16, 15],
            &[18, 17],
            &[16, 17, 1],
            &[20, 2],
            &[3, 1, 17],
            &[1, 17],
            &[2],
            &[],
        ];
        for sets in shapes {
            for requested in 16..=20 {
                let got = grant_container(sets, requested);
                assert!(
                    matches!(got, None | Some(INV_MAIN) | Some(INV_CRAFTING)),
                    "{sets:?} asked for {requested} gave {got:?}"
                );
            }
            for requested in 1..=15 {
                if let Some(c) = grant_container(sets, requested) {
                    assert!(!never_granted(c), "{sets:?} asked for {requested} gave {c}");
                }
            }
            if let Some(c) = default_grant_container(sets) {
                let only_ungrantable = sets.iter().all(|&s| never_granted(s));
                assert!(
                    only_ungrantable || !never_granted(c),
                    "cache for {sets:?} gave {c}"
                );
            }
        }
    }

    /// Weapons: loot asks for the bandolier, the GM path for the main bag;
    /// both are allowed and kept.
    #[test]
    fn allowed_requests_are_kept() {
        assert_eq!(grant_container(&[3, 1, 17], 3), Some(3));
        assert_eq!(grant_container(&[3, 1, 17], INV_MAIN), Some(INV_MAIN));
        assert_eq!(grant_container(&[1, 17], INV_MAIN), Some(INV_MAIN));
        assert_eq!(grant_container(&[], INV_MAIN), Some(INV_MAIN));
        assert_eq!(grant_container(&[2], 2), Some(2));
    }

    /// A mission item given into the main bag keeps the main bag: it lists
    /// no carried bag to fall through to.
    #[test]
    fn carried_request_with_no_carried_bag_listed_is_kept() {
        assert_eq!(grant_container(&[2], INV_MAIN), Some(INV_MAIN));
    }

    #[test]
    fn default_grant_container_skips_storage_only() {
        assert_eq!(default_grant_container(&[17, 15]), Some(INV_CRAFTING));
        assert_eq!(default_grant_container(&[3, 1, 17]), Some(3));
        assert_eq!(default_grant_container(&[2]), Some(2));
        assert_eq!(default_grant_container(&[1, 7, 17]), Some(INV_MAIN));
        assert_eq!(default_grant_container(&[17]), Some(17));
        assert_eq!(default_grant_container(&[]), None);
    }

    #[test]
    fn skipped_storage_names_the_fall_through() {
        assert!(skipped_storage(&[17, 15], 17, 15));
        assert!(skipped_storage(&[17, 15], INV_MAIN, 15));
        assert!(skipped_storage(&[17, 15], 15, 15));
        assert!(!skipped_storage(&[3, 1, 17], 3, 3));
        assert!(!skipped_storage(&[1, 17], INV_MAIN, INV_MAIN));
    }
}
