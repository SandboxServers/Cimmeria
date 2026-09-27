//! Wire-format guards for the world-entry `onBagInfo` (D-BV06).
//!
//! `onBagInfo(ARRAY<BagInfo>)` declares every container and its size. The
//! personal vault (17) is declared at the player's `sgw_player.bank_slots`,
//! not at the container ceiling of 100: the client lays out the vault grid
//! from this number, and the server refuses slots past it.

use super::super::*;
use super::sample_player_load_data;

/// `onBagInfo` args for containers 1-20, written out by hand so the pin
/// does not share code with the serializer: `count:u32`, then per bag
/// `bagId:i32, numberOfSlots:i32`, in id order. The sizes are typed in from
/// `BAG_SIZES` in `deprecated/python/common/Constants.py:142-163`, with the
/// vault (17) replaced by the player's `bank_slots`.
fn expected_bag_info(vault_slots: i32) -> Vec<u8> {
    let slots: [i32; 20] = [
        40,
        100,
        4,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        1,
        100,
        12,
        vault_slots,
        100,
        100,
        100,
    ];
    let mut args = Vec::new();
    args.extend_from_slice(&20u32.to_le_bytes());
    for (i, n) in slots.iter().enumerate() {
        args.extend_from_slice(&(i as i32 + 1).to_le_bytes());
        args.extend_from_slice(&n.to_le_bytes());
    }
    args
}

fn entry() -> WorldEntryInfo {
    WorldEntryInfo {
        player_entity_id: 100,
        space_id: 65552,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "CombatSim".into(),
        class_id: 0x02,
        world_stargates: vec![],
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// A default player (never expanded) is told their vault has 40 slots.
#[test]
fn map_loaded_bag_info_declares_default_vault_at_40() {
    let data = sample_player_load_data();
    assert_eq!(data.bank_slots, 40);
    let body = build_map_loaded_body(100, &data, &entry());
    assert!(
        contains(&body, &expected_bag_info(40)),
        "onBagInfo must declare containers 1-20 with the vault (17) at 40"
    );
    assert!(
        !contains(&body, &expected_bag_info(100)),
        "onBagInfo must not declare the vault at its ceiling (100)"
    );
}

/// An expanded vault is declared at the player's own size.
#[test]
fn map_loaded_bag_info_declares_vault_at_player_bank_slots() {
    let mut data = sample_player_load_data();
    data.bank_slots = 60;
    let body = build_map_loaded_body(100, &data, &entry());
    assert!(
        contains(&body, &expected_bag_info(60)),
        "onBagInfo must declare the vault (17) at PlayerLoadData::bank_slots"
    );
}
