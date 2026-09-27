//! Wire-format and pure-logic guards for the vault expansion (BV-05): the
//! re-sent `onBagInfo`, the refusal classification and the stable reasons.
//! No database.

use super::persist::{classify_refusal, ExpandOutcome, ExpansionState, VAULT_CEILING};
use super::*;

/// `onBagInfo` args for containers 1-20, written out by hand so the pin
/// does not share code with the serializer: `count:u32`, then per bag
/// `bagId:i32, numberOfSlots:i32`, in id order. The sizes are
/// `BAG_SIZES` from `deprecated/python/common/Constants.py:142-163`, with
/// the vault (17) at the new `bank_slots`.
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

/// Byte-exact: after a purchase to 50 slots the re-sent `onBagInfo`
/// declares all twenty containers, and only the vault's entry changed. A
/// one-entry array, a vault left at 40 or a reordered bag list all fail.
#[test]
fn resent_bag_info_redeclares_every_container_with_the_new_vault_size() {
    let args = vault_resize_bag_info_args(50);
    assert_eq!(args, expected_bag_info(50));
    assert_eq!(args.len(), 4 + 20 * 8);
    // Container 17's entry: bagId at offset 4 + 16*8, its size right after.
    let at = 4 + 16 * 8;
    assert_eq!(&args[at..at + 4], &17i32.to_le_bytes());
    assert_eq!(&args[at + 4..at + 8], &50i32.to_le_bytes());
    assert_ne!(args, vault_resize_bag_info_args(40), "the size must move");
}

/// The ceiling is container 17's capacity, and it is 100 (D-BV02).
#[test]
fn the_vault_ceiling_is_100() {
    assert_eq!(VAULT_CEILING, 100);
    assert_eq!(vault_resize_bag_info_args(100), expected_bag_info(100));
}

fn state(bank_slots: i16, naquadah: i32, next_price: Option<i32>) -> ExpansionState {
    ExpansionState {
        bank_slots,
        naquadah,
        next_price,
    }
}

/// A zero-row purchase is classified from the row as it is now, replay
/// key first: a size the vault has left behind is a replay whatever the
/// cash; then the ceiling, the missing price and the cash.
#[test]
fn a_refused_purchase_is_classified_replay_first() {
    let s = state(50, 0, Some(100));
    assert_eq!(classify_refusal(s, 40), ExpandOutcome::Replay { state: s });
    let s = state(100, 1000, None);
    assert_eq!(
        classify_refusal(s, 100),
        ExpandOutcome::AtCeiling { state: s }
    );
    let s = state(40, 1000, None);
    assert_eq!(
        classify_refusal(s, 40),
        ExpandOutcome::PriceMissing { state: s }
    );
    let s = state(40, 99, Some(100));
    assert_eq!(
        classify_refusal(s, 40),
        ExpandOutcome::InsufficientCash {
            state: s,
            price: 100
        }
    );
    // Everything holds now: the row moved between the write and the read.
    let s = state(40, 100, Some(100));
    assert_eq!(classify_refusal(s, 40), ExpandOutcome::Replay { state: s });
}

#[test]
fn refusal_reasons_are_stable() {
    let cases = [
        (ExpandRefusal::Vault("no_vault_session"), "no_vault_session"),
        (
            ExpandRefusal::Vault("banker_out_of_range"),
            "banker_out_of_range",
        ),
        (ExpandRefusal::NoOffer, "no_offer"),
        (ExpandRefusal::Replay, "replay"),
        (ExpandRefusal::AtCeiling, "at_ceiling"),
        (ExpandRefusal::InsufficientCash, "insufficient_cash"),
        (ExpandRefusal::PriceMissing, "price_missing"),
        (ExpandRefusal::PlayerRowMissing, "player_row_missing"),
        (ExpandRefusal::DbUnavailable, "db_unavailable"),
        (ExpandRefusal::QueryFailed, "query_failed"),
    ];
    for (refusal, reason) in cases {
        assert_eq!(refusal.reason(), reason);
        assert!(
            !refusal.feedback(Some(100)).is_empty(),
            "{reason} has a line"
        );
    }
    assert!(ExpandRefusal::InsufficientCash
        .feedback(Some(100))
        .contains("100 naquadah"));
}
