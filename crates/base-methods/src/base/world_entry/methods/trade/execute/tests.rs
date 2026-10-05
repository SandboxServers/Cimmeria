//! Unit-level regression guards for the atomic-trade execute module.
//!
//! cfg-only modules pin internal invariants without needing a live DB:
//! - [`tradeable_containers`] — the source-bag whitelist is exactly the
//!   backpack and the crafting bag.
//! - [`destination`] — where each traded item lands, by its
//!   `container_sets`.
//! - [`slot_exclusion_accounting`] — recipient-slot reservation must
//!   exclude the slots the same tx is about to vacate, per bag.
//! - [`parking_sentinel`] — the two-phase swap's parking step must
//!   pick distinct negative slots so no row collides with another
//!   parked row or with any real container slot.
//! - [`refusal_result_code`] — every refusal is `Cancelled` on the wire.
//! - [`refusal_feedback`] — the feedback line each side gets.
//!
//! Live-DB integration tests for the same surfaces live in
//! `super::super::tests`.

mod tradeable_containers {
    //! The whitelist, pinned both ways: the two carried bags are in it,
    //! and nothing else a player owns rows in is. The live-DB guards in
    //! `super::super::super::tests::container_whitelist` run the same
    //! containers through a real swap.

    use super::super::placement::TRADEABLE_CONTAINERS;
    use cimmeria_entity::inventory::{INV_CRAFTING, INV_MAIN};

    #[test]
    fn backpack_and_crafting_bag_are_tradeable() {
        assert_eq!(TRADEABLE_CONTAINERS, &[INV_MAIN, INV_CRAFTING]);
    }

    /// Mission (2), bandolier (3), equipment (4-14), buyback (16) and
    /// the vaults (17-20) stay out.
    #[test]
    fn every_other_container_is_refused() {
        for container in (2..=14).chain(16..=20) {
            assert!(
                !TRADEABLE_CONTAINERS.contains(&container),
                "container {container} must not be a trade source"
            );
        }
    }
}

mod destination {
    use super::super::placement::trade_destination;
    use cimmeria_entity::inventory::{INV_CRAFTING, INV_MAIN};

    /// The crafting-component shape: `{17,15}` lands in the crafting bag
    /// whichever carried bag it was traded from; never in the bank.
    #[test]
    fn crafting_component_lands_in_the_crafting_bag() {
        assert_eq!(
            trade_destination(&[17, 15], INV_CRAFTING),
            Some(INV_CRAFTING)
        );
        assert_eq!(trade_destination(&[17, 15], INV_MAIN), Some(INV_CRAFTING));
    }

    #[test]
    fn backpack_items_stay_in_the_backpack() {
        assert_eq!(trade_destination(&[3, 1, 17], INV_MAIN), Some(INV_MAIN));
        assert_eq!(trade_destination(&[1, 17], INV_MAIN), Some(INV_MAIN));
        assert_eq!(trade_destination(&[], INV_MAIN), Some(INV_MAIN));
    }

    /// An item that lists no carried bag keeps the bag it was traded
    /// from, as every backpack trade did before the crafting bag joined.
    #[test]
    fn item_listing_no_carried_bag_keeps_its_source_bag() {
        assert_eq!(trade_destination(&[2], INV_MAIN), Some(INV_MAIN));
    }

    /// An unrestricted item found in the crafting bag goes to the
    /// backpack: an empty list allows only the backpack.
    #[test]
    fn unrestricted_item_from_the_crafting_bag_goes_to_the_backpack() {
        assert_eq!(trade_destination(&[], INV_CRAFTING), Some(INV_MAIN));
    }

    /// Whatever the item lists, a trade lands in 1 or 15.
    #[test]
    fn destination_is_always_a_tradeable_bag() {
        let shapes: &[&[i32]] = &[&[17, 15], &[17], &[16], &[2], &[3, 1, 17], &[15, 1], &[]];
        for sets in shapes {
            for source in [INV_MAIN, INV_CRAFTING] {
                let got = trade_destination(sets, source);
                assert!(
                    matches!(got, Some(INV_MAIN) | Some(INV_CRAFTING)),
                    "{sets:?} from {source} gave {got:?}"
                );
            }
        }
    }
}

mod slot_exclusion_accounting {
    //! Unit-level regression guard for the recipient-slot-reservation
    //! bug. The full live-DB integration guard is
    //! `commit_succeeds_when_recipient_bag_full_but_trading_slot_away`
    //! in the live-DB `tests::slot_reservation` module; this is the
    //! algorithmic-core proxy that runs without a live DB.

    use super::super::placement::pick_free_slots_excluding;
    use crate::base::resources::bag_max_slots;
    use cimmeria_entity::inventory::{INV_CRAFTING, INV_MAIN};

    /// Bag is 100% full (40/40 in INV_MAIN). One of those 40 is being
    /// traded away. Without the exclusion the reservation fails
    /// (0 free); with the exclusion it succeeds (slot 39 is the
    /// vacating slot, returned as the pick).
    ///
    /// Revert-verifier: change `pick_free_slots_excluding` to
    /// pass `raw_occupied` straight through to `free_inventory_slots`
    /// without subtracting `vacating`; the second assertion below
    /// fails with `None != Some([39])`.
    #[test]
    fn full_bag_swap_succeeds_with_exclusion_fails_without() {
        // 40/40 occupied — slots 0..=39.
        let raw_occupied: Vec<i32> = (0..bag_max_slots(INV_MAIN)).collect();
        let vacating = vec![39]; // the slot the recipient is trading away

        // Sanity: with NO exclusion (vacating is empty), the
        // fully-occupied bag rejects the reservation. This is the
        // pre-fix shape — what the bug looked like in production.
        let without_exclusion = pick_free_slots_excluding(INV_MAIN, &raw_occupied, &[], 1);
        assert!(
            without_exclusion.is_none(),
            "sanity: without the exclusion (i.e., vacating list empty), \
             a 40/40 INV_MAIN bag can't reserve a slot. This is the \
             pre-fix shape — the bug Copilot flagged was that the \
             recipient's outgoing trade item counted as occupied even \
             though it's about to leave."
        );

        // Post-fix behaviour: with the vacating slot excluded, the
        // 40-slot bag has 1 free slot (39 itself, since the picker
        // returns the lowest free slot in [min, max)).
        let with_exclusion = pick_free_slots_excluding(INV_MAIN, &raw_occupied, &vacating, 1);
        assert_eq!(
            with_exclusion,
            Some(vec![39]),
            "with the exclusion, the recipient's about-to-vacate slot \
             39 is available for the incoming item. If this returns \
             None, the exclusion was dropped from \
             `pick_free_slots_excluding` — re-check the filter."
        );
    }

    /// Two-for-two swap: recipient's bag is full (40/40), trading 2
    /// of those slots away, must accept 2 incoming items.
    #[test]
    fn full_bag_two_for_two_swap_succeeds_with_exclusion() {
        let raw_occupied: Vec<i32> = (0..bag_max_slots(INV_MAIN)).collect();
        let vacating = vec![5, 17]; // two non-contiguous outgoing slots
        let picked = pick_free_slots_excluding(INV_MAIN, &raw_occupied, &vacating, 2);
        assert_eq!(
            picked,
            Some(vec![5, 17]),
            "the two vacating slots are the only free slots and must \
             be returned in ascending order"
        );
    }

    /// Recipient still has free slots even WITHOUT counting the
    /// vacating ones — the exclusion is a no-op in that case. This
    /// pins the "happy path" doesn't regress when the fix is in:
    /// the exclusion must not poison the pick when it isn't needed.
    #[test]
    fn partially_full_bag_doesnt_need_exclusion() {
        // 10/40 used — slots 0..=9. Plenty of room.
        let raw_occupied: Vec<i32> = (0..10).collect();
        let picked = pick_free_slots_excluding(INV_MAIN, &raw_occupied, &[], 1);
        assert_eq!(
            picked,
            Some(vec![10]),
            "with 10/40 used and no exclusions, slot 10 is the lowest \
             free slot — must be returned"
        );
    }

    /// The crafting bag holds 100. A full crafting bag with one slot
    /// being traded away takes exactly one incoming item; a second
    /// does not fit.
    #[test]
    fn full_crafting_bag_counts_its_vacating_slot() {
        let raw_occupied: Vec<i32> = (0..bag_max_slots(INV_CRAFTING)).collect();
        assert_eq!(raw_occupied.len(), 100);
        assert_eq!(
            pick_free_slots_excluding(INV_CRAFTING, &raw_occupied, &[], 1),
            None
        );
        assert_eq!(
            pick_free_slots_excluding(INV_CRAFTING, &raw_occupied, &[63], 1),
            Some(vec![63])
        );
        assert_eq!(
            pick_free_slots_excluding(INV_CRAFTING, &raw_occupied, &[63], 2),
            None
        );
    }

    /// Slots 40-99 exist only in the crafting bag: a backpack pick never
    /// returns them, and a crafting-bag pick uses them.
    #[test]
    fn each_bag_uses_its_own_capacity() {
        let raw_occupied: Vec<i32> = (0..40).collect();
        assert_eq!(
            pick_free_slots_excluding(INV_MAIN, &raw_occupied, &[], 1),
            None
        );
        assert_eq!(
            pick_free_slots_excluding(INV_CRAFTING, &raw_occupied, &[], 1),
            Some(vec![40])
        );
    }
}

mod parking_sentinel {
    //! Unit-level regression guard for the two-phase swap parking step.
    //!
    //! Background: `sgw_inventory` has a UNIQUE INDEX on
    //! `(character_id, container_id, slot_id)` (see
    //! `db/sgw/Inventory/Tables/sgw_inventory.sql`). The pre-fix
    //! single-statement re-key (item_a → recipient's destination slot,
    //! immediately followed by item_b → sender's destination slot)
    //! violated the constraint whenever the destination slot still
    //! contained the partner's outgoing-this-trade item.
    //!
    //! The fix: phase 1 parks each outgoing item at a distinct
    //! NEGATIVE slot in INV_MAIN, vacating its original slot before
    //! phase 2 re-keys the row to the recipient at its reserved
    //! positive slot. Distinctness within the parked set is critical —
    //! two items parked at the same `(player, INV_MAIN, sentinel)`
    //! would themselves collide on the UNIQUE INDEX.

    use super::super::swap::park_sentinel_slot;
    use std::collections::HashSet;

    /// Every parked item must land on a distinct slot. The smallest
    /// trade that exposed the original bug had 2 items (one per side);
    /// the largest realistic case has 40 per side (full INV_MAIN
    /// each). Sweep the whole range.
    #[test]
    fn parked_slots_are_pairwise_distinct() {
        for total in [2usize, 4, 10, 40, 80] {
            let slots: Vec<i32> = (0..total as i32)
                .map(|n| park_sentinel_slot(n, total))
                .collect();
            let unique: HashSet<i32> = slots.iter().copied().collect();
            assert_eq!(
                unique.len(),
                total,
                "parking slots must be distinct for total={total}; \
                 got {slots:?}. A duplicate sentinel collides on the \
                 sgw_inventory_unique_slot index during phase 1."
            );
        }
    }

    /// Parked slots must be negative, so they never collide with a
    /// real container slot (every container's `bag_min_slot` is 0)
    /// and the grant / purchase / move paths can never accidentally
    /// land there.
    #[test]
    fn parked_slots_are_strictly_negative() {
        for total in [2usize, 40, 80] {
            for n in 0..total as i32 {
                let s = park_sentinel_slot(n, total);
                assert!(
                    s < 0,
                    "park_sentinel_slot({n}, {total}) = {s}; sentinel \
                     slots must be negative so they can't be reached \
                     from any legitimate slot-allocation path"
                );
            }
        }
    }
}

mod refusal_result_code {
    //! Every refusal is `Cancelled` (2) on the wire: the shipped client's
    //! trade window closes on 1 and 2 only. The byte-exact live-DB guard
    //! is `super::super::super::tests::crafting_bag::
    //! full_destination_crafting_bag_refuses_the_whole_trade`.

    use super::super::abort::REFUSAL_RESULT;
    use cimmeria_entity::trade::ETRADERESULTS_CANCELLED;

    #[test]
    fn refusals_are_cancelled() {
        assert_eq!(REFUSAL_RESULT, ETRADERESULTS_CANCELLED);
        assert_eq!(REFUSAL_RESULT, 2);
    }
}

mod refusal_feedback {
    //! Every refusal is a bare `Cancelled` on the wire, so the cause must
    //! reach each side as a feedback line, naming whose bag is full or
    //! whose naquadah is short.
    //!
    //! Revert-verifier: returning `(None, None)` from `refusal_lines`
    //! trips every assertion below.

    use super::super::abort::{
        refusal_lines, TradeAbort, LOCAL_BACKPACK_FULL, LOCAL_CRAFTING_BAG_FULL, LOCAL_NO_CASH,
        LOCAL_UNTRADEABLE_ITEM, REMOTE_BACKPACK_FULL, REMOTE_CRAFTING_BAG_FULL, REMOTE_NO_CASH,
        REMOTE_UNTRADEABLE_ITEM,
    };

    const P1_PID: i32 = 1000;
    const P2_PID: i32 = 2000;

    fn slots(recipient_player_id: i32, container_id: i32) -> TradeAbort {
        TradeAbort::NotEnoughSlots {
            recipient_player_id,
            container_id,
            needed: 1,
            free: 0,
        }
    }

    #[test]
    fn full_crafting_bag_names_the_crafting_bag_to_both_sides() {
        assert_eq!(
            refusal_lines(&slots(P2_PID, 15), P1_PID),
            (
                Some(REMOTE_CRAFTING_BAG_FULL),
                Some(LOCAL_CRAFTING_BAG_FULL)
            )
        );
        assert_eq!(
            refusal_lines(&slots(P1_PID, 15), P1_PID),
            (
                Some(LOCAL_CRAFTING_BAG_FULL),
                Some(REMOTE_CRAFTING_BAG_FULL)
            )
        );
    }

    #[test]
    fn full_backpack_names_the_backpack() {
        assert_eq!(
            refusal_lines(&slots(P1_PID, 1), P1_PID),
            (Some(LOCAL_BACKPACK_FULL), Some(REMOTE_BACKPACK_FULL))
        );
    }

    #[test]
    fn short_cash_tells_both_sides() {
        let reason = TradeAbort::InsufficientCash {
            which: "p2",
            player_id: P2_PID,
            has: 0,
            wants: 5,
        };
        assert_eq!(
            refusal_lines(&reason, P1_PID),
            (Some(REMOTE_NO_CASH), Some(LOCAL_NO_CASH))
        );
    }

    #[test]
    fn untradeable_item_tells_both_sides() {
        let reason = TradeAbort::IneligibleContainer {
            which: "p1",
            player_id: P1_PID,
            item_id: 1,
            container_id: 17,
        };
        assert_eq!(
            refusal_lines(&reason, P1_PID),
            (Some(LOCAL_UNTRADEABLE_ITEM), Some(REMOTE_UNTRADEABLE_ITEM))
        );
    }
}

mod item_moved_names {
    //! Rule 6: `trade.item_moved`, the most-read trade event, names both
    //! parties and the item next to their IDs.

    use cimmeria_entity::cell_entity::PlayerIdentity;

    use super::super::log_item_moved;
    use super::super::placement::ItemMove;
    use crate::test_support::LogCapture;

    const ITEM_TYPE: i32 = 912_346;

    fn mv() -> ItemMove {
        ItemMove {
            item_id: 5,
            type_id: ITEM_TYPE,
            from_entity: 70,
            from_player: 12,
            from_container: 1,
            from_slot: 2,
            to_entity: 71,
            to_player: 13,
            to_container: 1,
            to_slot: 3,
        }
    }

    fn identity(account: u32, player: i32, pn: &'static str, an: &'static str) -> PlayerIdentity {
        PlayerIdentity {
            account_id: Some(account),
            player_id: Some(player),
            player_name: Some(pn),
            account_name: Some(an),
        }
    }

    #[test]
    fn names_both_parties_and_the_item() {
        let mut book = cimmeria_names::NameBook::empty();
        book.insert(
            cimmeria_names::Table::Items,
            i64::from(ITEM_TYPE),
            "Zat'nik'tel",
        );
        cimmeria_names::global().store(book);
        let capture = LogCapture::install();
        log_item_moved(
            &mv(),
            &identity(6, 12, "Teal'c", "sgc_login"),
            &identity(7, 13, "Daniel", "abydos_login"),
        );
        let ev = capture
            .all()
            .into_iter()
            .find(|e| e.message_contains("trade: item changed hands"))
            .expect("trade.item_moved row");
        for (key, want) in [
            ("player_name", "Teal'c"),
            ("account_name", "sgc_login"),
            ("target_player_name", "Daniel"),
            ("target_account_name", "abydos_login"),
            ("item_name", "Zat'nik'tel"),
        ] {
            assert!(ev.has_field(key, want), "{key}: {ev:#?}");
        }
    }
}
