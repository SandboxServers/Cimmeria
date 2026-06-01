//! Slot-reservation accounting live-DB regression guard.
//!
//! Pre-fix, `reserve_main_slots_for` queried the recipient's INV_MAIN
//! directly and counted every existing row as occupied — including the
//! row(s) about to vacate via the same trade transaction. The fix
//! (`reserve_main_slots_excluding`) subtracts the soon-to-vacate
//! slot ids from the occupancy set before picking new slots.

use std::sync::Arc;

use cimmeria_entity::inventory::INV_MAIN;

use super::{
    cleanup, fixtures, insert_account_and_player, insert_item, make_state, owner_of, slot_id_of,
    tradeable_type_ids,
};
use crate::base::world_entry::methods::trade::handle_execute_trade;
use crate::test_support::require_db_or_skip;

/// Regression guard for the recipient-slot-reservation accounting bug.
///
/// A full-bag swap where one of the recipient's "full" slots holds the
/// item they themselves are trading away would fail spuriously with
/// `NotEnoughSlots` if the slot accounting double-counts the vacating
/// row.
///
/// Scenario set up here:
/// - INV_MAIN holds 40 slots (`bag_max_slots(INV_MAIN) = 40`).
/// - player_a: 1 item in INV_MAIN slot 0 (item_a). Plenty of space.
/// - player_b: 40 items filling INV_MAIN slots 0..=39 (the bag is
///   completely full). The item at slot 39 is the one player_b is
///   trading to player_a.
///
/// Trade:
/// - player_a → player_b: item_a (needs 1 slot in player_b's INV_MAIN)
/// - player_b → player_a: outgoing_b (needs 1 slot in player_a's INV_MAIN)
///
/// Expected:
/// - The commit succeeds.
/// - item_a is now owned by player_b (somewhere in INV_MAIN).
/// - outgoing_b is now owned by player_a (slot 0 of player_a's
///   INV_MAIN was the lowest free slot, since player_a's slot 0 was
///   freed up by item_a leaving).
///
/// Revert-verifier: undoing the fix in `reserve_main_slots_excluding`
/// (or routing the call back through the original
/// `reserve_main_slots_for` that didn't exclude vacating slots) causes
/// this test to fail with the swap aborted — item_a stays on
/// player_a, outgoing_b stays on player_b.
#[tokio::test]
async fn commit_succeeds_when_recipient_bag_full_but_trading_slot_away() {
    let pool = require_db_or_skip!();
    let (weapon_type_id, another_type_id) = tradeable_type_ids(&pool).await;
    let f = fixtures(1600);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 0, "a").await;
    insert_account_and_player(&pool, f.account_b, f.player_b, 0, "b").await;

    // player_a: one item in INV_MAIN slot 0.
    let item_a = insert_item(&pool, f.player_a, weapon_type_id, INV_MAIN, 0, false).await;

    // player_b: 40 items in INV_MAIN slots 0..=39 (the bag is full).
    // Slot 39 holds the item being traded out.
    let mut b_filler_items: Vec<i32> = Vec::with_capacity(40);
    for slot in 0..40 {
        // Mix the type ids so a future stack-aware accounting change
        // can't accidentally pass by coalescing stacks.
        let t = if slot % 2 == 0 {
            weapon_type_id
        } else {
            another_type_id
        };
        let id = insert_item(&pool, f.player_b, t, INV_MAIN, slot, false).await;
        b_filler_items.push(id);
    }
    // The outgoing-from-player_b item sits at slot 39.
    let outgoing_b = b_filler_items[39];

    let (transport, e2a, conn) = make_state(f.entity_a, f.entity_b);
    let db = Some(Arc::new(pool.clone()));

    handle_execute_trade(
        f.entity_a,
        f.player_a,
        f.entity_b,
        f.player_b,
        vec![item_a],
        0,
        vec![outgoing_b],
        0,
        &db,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    // The commit MUST succeed: net-slot delta is 0 on both sides.
    assert_eq!(
        owner_of(&pool, item_a).await,
        Some(f.player_b),
        "item_a must move to player_b: the recipient's outgoing item \
         frees up slot 39, so the swap has no net bag growth and must commit. \
         If this assertion fails, the slot-reservation function is still \
         counting the recipient's outgoing items as occupied — see the \
         trade execute.rs `reserve_main_slots_excluding` fix."
    );
    assert_eq!(
        owner_of(&pool, outgoing_b).await,
        Some(f.player_a),
        "outgoing_b must move to player_a — symmetric arm of the swap"
    );

    // player_a's INV_MAIN slot 0 was the lowest free slot after item_a
    // vacated it; outgoing_b should land there.
    assert_eq!(
        slot_id_of(&pool, outgoing_b).await,
        Some(0),
        "outgoing_b should occupy player_a's slot 0 (lowest free slot \
         after item_a vacates). If a future change picks slots top-down \
         this assertion will need updating, but the bottom-up pick is \
         the documented invariant."
    );

    // The 39 OTHER filler items (slots 0..=38 of player_b) must remain
    // on player_b. Sanity-check that the rollback didn't accidentally
    // bulk-move anything else.
    for &id in &b_filler_items[0..39] {
        assert_eq!(
            owner_of(&pool, id).await,
            Some(f.player_b),
            "filler item {id} must remain on player_b — the trade is \
             only meant to move outgoing_b ({outgoing_b}), not the rest \
             of the bag"
        );
    }

    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;
}
