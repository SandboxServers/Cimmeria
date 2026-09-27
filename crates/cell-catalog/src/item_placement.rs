//! Where a granted item lands, from its `resources.items.container_sets`.
//!
//! `container_sets` lists the containers an item may sit in. For 752
//! crafting components the list is `{17,15}`: the bank first, then the
//! crafting bag. A grant that takes the first entry tries to put the item
//! into the bank, which is never a grant target, so the grant is refused.
//! [`first_player_container`] skips the storage containers and picks the
//! first bag the player carries.

/// The player's main bag (`INV_Main`).
pub const INV_MAIN: i32 = 1;

/// The crafting bag (`INV_Crafting`).
pub const INV_CRAFTING: i32 = 15;

/// Storage containers: the bank (17) and the other stores up to 20. A grant
/// never places an item into one of them.
pub const STORAGE_CONTAINERS: std::ops::RangeInclusive<i32> = 17..=20;

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
}
