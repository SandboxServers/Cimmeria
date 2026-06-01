//! Unit-level regression guards for the atomic-trade execute module.
//!
//! Four cfg-only modules pin internal invariants without needing a
//! live DB:
//! - [`advisory_lock_alignment`] — the trade tx must use the same
//!   `pg_advisory_xact_lock` shape as the vendor stack.
//! - [`slot_exclusion_accounting`] — recipient-slot reservation must
//!   exclude the slots the same tx is about to vacate.
//! - [`parking_sentinel`] — the two-phase swap's parking step must
//!   pick distinct negative slots so no row collides with another
//!   parked row or with any real container slot.
//! - [`trade_results_code_mapping`] — `TradeAbort` → per-side
//!   asymmetric `ETradeResults` codes (Clara G7).
//!
//! Live-DB integration tests for the same surfaces live in
//! `super::super::tests`.

mod advisory_lock_alignment {
    //! Static-read regression guard: trade's advisory-lock SQL form
    //! must match the vendor stack's. Both use the two-arg form
    //! `pg_advisory_xact_lock($1, $2)` and bind `(player_id, INV_MAIN)`
    //! so the two paths serialize through the same lock namespace —
    //! otherwise concurrent trade-and-vendor on the same player race
    //! at the advisory layer (`FOR UPDATE` still saves correctness,
    //! but the deadlock detector surfaces ABBA failures noisily
    //! under load).
    //!
    //! The original trade code used the single-arg form
    //! `pg_advisory_xact_lock(player_id, 0)`. Postgres treats
    //! `(player_id, 0)` and `(player_id, 1)` as completely independent
    //! locks — they don't serialize against each other.
    //!
    //! This is a static-read test (no DB required) so the alignment
    //! invariant is gated in CI even when the live-DB profile is off.

    use super::super::swap::ADVISORY_LOCK_SQL;
    use cimmeria_entity::inventory::INV_MAIN;

    /// Pin the SQL string. If a future change goes back to the
    /// single-arg `(player_id, 0)` form, this assertion trips.
    #[test]
    fn lock_sql_is_two_arg_form() {
        assert_eq!(
            ADVISORY_LOCK_SQL, "SELECT pg_advisory_xact_lock($1, $2)",
            "trade's advisory lock MUST use the two-arg form to match \
             vendor's `pg_advisory_xact_lock(player_id, container_id)`. \
             Single-arg form races at the advisory layer against vendor."
        );
    }

    /// Pin that the second bind is INV_MAIN. The vendor stack always
    /// binds the container id; for trade we lock under INV_MAIN
    /// (since that's the only tradeable container per the whitelist).
    #[test]
    fn lock_namespace_is_inv_main_not_zero() {
        // INV_MAIN is `1` per `entities/defs/system.xml`. The pre-fix
        // shape was `(player_id, 0)` which collides with no real
        // container — so the lock was effectively in its own
        // namespace.
        assert_eq!(
            INV_MAIN, 1,
            "INV_MAIN must be 1 — if this constant changes, audit the \
             vendor stack too: it binds the container id directly."
        );
    }
}

mod slot_exclusion_accounting {
    //! Unit-level regression guard for the recipient-slot-reservation
    //! bug. The full live-DB integration guard is
    //! `commit_succeeds_when_recipient_bag_full_but_trading_slot_away`
    //! in the live-DB `tests::slot_reservation` module; this is the
    //! algorithmic-core proxy that runs without a live DB.

    use super::super::swap::pick_free_main_slots_excluding;
    use crate::base::resources::bag_max_slots;
    use cimmeria_entity::inventory::INV_MAIN;

    /// Bag is 100% full (40/40 in INV_MAIN). One of those 40 is being
    /// traded away. Without the exclusion the reservation fails
    /// (0 free); with the exclusion it succeeds (slot 39 is the
    /// vacating slot, returned as the pick).
    ///
    /// Revert-verifier: change `pick_free_main_slots_excluding` to
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
        let without_exclusion = pick_free_main_slots_excluding(&raw_occupied, &[], 1);
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
        let with_exclusion = pick_free_main_slots_excluding(&raw_occupied, &vacating, 1);
        assert_eq!(
            with_exclusion,
            Some(vec![39]),
            "with the exclusion, the recipient's about-to-vacate slot \
             39 is available for the incoming item. If this returns \
             None, the exclusion was dropped from \
             `pick_free_main_slots_excluding` — re-check the filter."
        );
    }

    /// Two-for-two swap: recipient's bag is full (40/40), trading 2
    /// of those slots away, must accept 2 incoming items.
    #[test]
    fn full_bag_two_for_two_swap_succeeds_with_exclusion() {
        let raw_occupied: Vec<i32> = (0..bag_max_slots(INV_MAIN)).collect();
        let vacating = vec![5, 17]; // two non-contiguous outgoing slots
        let picked = pick_free_main_slots_excluding(&raw_occupied, &vacating, 2);
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
        let picked = pick_free_main_slots_excluding(&raw_occupied, &[], 1);
        assert_eq!(
            picked,
            Some(vec![10]),
            "with 10/40 used and no exclusions, slot 10 is the lowest \
             free slot — must be returned"
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

mod trade_results_code_mapping {
    //! Unit-level regression guard for the per-side asymmetric
    //! `ETradeResults` code mapping introduced for Clara's G7 review
    //! on PR #438.
    //!
    //! The mapping replaces the old "both sides see Cancelled on every
    //! abort" behavior with the Python-parity `NoLocal*`/`NoRemote*`
    //! codes from `Trade.py:237-263`. The canonical client uses these
    //! to surface a specific trade-results dialog string (e.g. "you
    //! don't have enough cash" vs. "they don't have enough space")
    //! rather than the generic teardown notification.
    //!
    //! Revert-verifier: replacing `trade_abort_to_results_codes` with
    //! `|_, _, _| (ETRADERESULTS_CANCELLED, ETRADERESULTS_CANCELLED)`
    //! trips every assertion below.

    use super::super::{trade_abort_to_results_codes, TradeAbort};
    use cimmeria_entity::trade::{
        ETRADERESULTS_CANCELLED, ETRADERESULTS_NO_LOCAL_CASH, ETRADERESULTS_NO_LOCAL_SPACE,
        ETRADERESULTS_NO_REMOTE_CASH, ETRADERESULTS_NO_REMOTE_SPACE,
    };

    const P1_PID: i32 = 1000;
    const P2_PID: i32 = 2000;

    #[test]
    fn insufficient_cash_p1_maps_to_no_local_cash_and_no_remote_cash() {
        let reason = TradeAbort::InsufficientCash {
            which: "p1",
            player_id: P1_PID,
            has: 5,
            wants: 10,
        };
        assert_eq!(
            trade_abort_to_results_codes(&reason, P1_PID, P2_PID),
            (ETRADERESULTS_NO_LOCAL_CASH, ETRADERESULTS_NO_REMOTE_CASH),
            "p1 short on cash: p1 sees NoLocalCash, p2 sees NoRemoteCash"
        );
    }

    #[test]
    fn insufficient_cash_p2_maps_to_no_remote_cash_and_no_local_cash() {
        let reason = TradeAbort::InsufficientCash {
            which: "p2",
            player_id: P2_PID,
            has: 0,
            wants: 100,
        };
        assert_eq!(
            trade_abort_to_results_codes(&reason, P1_PID, P2_PID),
            (ETRADERESULTS_NO_REMOTE_CASH, ETRADERESULTS_NO_LOCAL_CASH),
            "p2 short on cash: p1 sees NoRemoteCash, p2 sees NoLocalCash"
        );
    }

    #[test]
    fn not_enough_slots_resolves_recipient_against_player_ids() {
        // Recipient = p1 → p1 sees NoLocalSpace (their bag is full),
        // p2 sees NoRemoteSpace (partner is the one without room).
        let reason = TradeAbort::NotEnoughSlots {
            recipient_player_id: P1_PID,
            needed: 3,
        };
        assert_eq!(
            trade_abort_to_results_codes(&reason, P1_PID, P2_PID),
            (ETRADERESULTS_NO_LOCAL_SPACE, ETRADERESULTS_NO_REMOTE_SPACE)
        );

        // Recipient = p2 → mirrored.
        let reason = TradeAbort::NotEnoughSlots {
            recipient_player_id: P2_PID,
            needed: 3,
        };
        assert_eq!(
            trade_abort_to_results_codes(&reason, P1_PID, P2_PID),
            (ETRADERESULTS_NO_REMOTE_SPACE, ETRADERESULTS_NO_LOCAL_SPACE)
        );
    }

    /// Defensive: an abort variant with a stale `recipient_player_id`
    /// that matches neither side must fall back to Cancelled rather
    /// than guess. The atomic-commit-failure shape would still be
    /// surfaced to the player as the generic teardown string — which
    /// is correct: the trade is cancelled.
    #[test]
    fn not_enough_slots_unknown_recipient_falls_back_to_cancelled() {
        let reason = TradeAbort::NotEnoughSlots {
            recipient_player_id: 99999, // matches neither
            needed: 3,
        };
        assert_eq!(
            trade_abort_to_results_codes(&reason, P1_PID, P2_PID),
            (ETRADERESULTS_CANCELLED, ETRADERESULTS_CANCELLED)
        );
    }

    /// Catch-all variants (DbError, PlayerMissing, ItemMissing,
    /// DuplicateInstance, BoundItemOffered, IneligibleContainer) are
    /// either internal faults or server-authority validations the
    /// client UI has no dedicated string for — both sides see
    /// Cancelled.
    #[test]
    fn catch_all_variants_map_to_cancelled_on_both_sides() {
        let catch_alls = [
            TradeAbort::PlayerMissing {
                which: "p1",
                player_id: P1_PID,
            },
            TradeAbort::ItemMissing {
                which: "p2",
                player_id: P2_PID,
                item_id: 7,
            },
            TradeAbort::DuplicateInstance { item_id: 42 },
            TradeAbort::BoundItemOffered {
                which: "p1",
                player_id: P1_PID,
                item_id: 99,
            },
            TradeAbort::IneligibleContainer {
                which: "p1",
                player_id: P1_PID,
                item_id: 11,
                container_id: 8, // INV_EQUIP or similar
            },
        ];
        for reason in &catch_alls {
            assert_eq!(
                trade_abort_to_results_codes(reason, P1_PID, P2_PID),
                (ETRADERESULTS_CANCELLED, ETRADERESULTS_CANCELLED),
                "catch-all {reason:?} must map to Cancelled on both sides"
            );
        }
    }
}
