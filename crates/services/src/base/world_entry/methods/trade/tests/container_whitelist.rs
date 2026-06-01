//! Container-whitelist regression guards for the security review on
//! PR #438.
//!
//! `lock_items` must reject items from any container that isn't
//! `INV_MAIN`. The pre-fix code only rejected `INV_BUYBACK`; equipped
//! gear (INV_CHEST), mission items (INV_MISSION), bank (INV_BANK),
//! bandolier (INV_BANDOLIER) were all trade-eligible — a dupe-strip
//! exploit and a bypass of the banker-NPC gate.

use std::sync::Arc;

use cimmeria_entity::inventory::{
    INV_BANDOLIER, INV_BANK, INV_BUYBACK, INV_CHEST, INV_MAIN, INV_MISSION,
};

use super::{
    cleanup, fixtures, insert_account_and_player, insert_item, make_state, owner_of,
    tradeable_type_ids,
};
use crate::base::world_entry::methods::trade::handle_execute_trade;
use crate::test_support::require_db_or_skip;

/// Each iteration:
/// 1. Inserts an item in the offending container for player_a.
/// 2. Attempts the trade.
/// 3. Asserts the item is **still** in its original container with
///    player_a as the owner (the swap rolled back).
///
/// Revert-verifier: replacing the whitelist check with the prior
/// "reject only INV_BUYBACK" branch causes this test to fail on the
/// non-INV_BUYBACK rows because the item moves to player_b's INV_MAIN
/// instead of staying put.
#[tokio::test]
async fn lock_items_rejects_non_inv_main_containers() {
    let pool = require_db_or_skip!();
    let (weapon_type_id, another_type_id) = tradeable_type_ids(&pool).await;

    // One forbidden container per iteration. Each gets a distinct salt
    // so the sentinels don't collide.
    let cases: &[(&str, i32)] = &[
        ("INV_CHEST equipped armor", INV_CHEST),
        ("INV_MISSION quest item", INV_MISSION),
        ("INV_BANK stored item", INV_BANK),
        ("INV_BANDOLIER ammo slot", INV_BANDOLIER),
        // INV_BUYBACK was already blocked by the blacklist; including
        // it here makes the test the single source of truth for which
        // containers are rejected, so a future "buyback is tradeable"
        // regression also trips this guard.
        ("INV_BUYBACK reclaim slot", INV_BUYBACK),
    ];

    for (label, forbidden_container) in cases {
        let salt = 1400 + forbidden_container * 10;
        let f = fixtures(salt);
        cleanup(
            &pool,
            &[f.account_a, f.account_b],
            &[f.player_a, f.player_b],
        )
        .await;

        insert_account_and_player(&pool, f.account_a, f.player_a, 0, "a").await;
        insert_account_and_player(&pool, f.account_b, f.player_b, 0, "b").await;
        let bad_item = insert_item(
            &pool,
            f.player_a,
            weapon_type_id,
            *forbidden_container,
            0,
            false,
        )
        .await;
        let good_item_b = insert_item(&pool, f.player_b, another_type_id, INV_MAIN, 0, false).await;

        let (transport, e2a, conn) = make_state(f.entity_a, f.entity_b);
        let db = Some(Arc::new(pool.clone()));

        handle_execute_trade(
            f.entity_a,
            f.player_a,
            f.entity_b,
            f.player_b,
            vec![bad_item],
            0,
            vec![good_item_b],
            0,
            &db,
            &transport,
            &conn,
            &e2a,
        )
        .await;

        // Owner unchanged (still player_a).
        assert_eq!(
            owner_of(&pool, bad_item).await,
            Some(f.player_a),
            "{label}: bad_item must still belong to player_a (whitelist rolled back)"
        );
        // And the container is unchanged — verifies the row wasn't
        // moved to INV_MAIN under player_a as a side-effect of some
        // partial transaction.
        let container_now: i32 =
            sqlx::query_scalar("SELECT container_id FROM sgw_inventory WHERE item_id = $1")
                .bind(bad_item)
                .fetch_one(&pool)
                .await
                .expect("read container");
        assert_eq!(
            container_now, *forbidden_container,
            "{label}: bad_item must still be in container {forbidden_container}"
        );
        // Partner's good item also unchanged — symmetric rollback.
        assert_eq!(
            owner_of(&pool, good_item_b).await,
            Some(f.player_b),
            "{label}: partner's good item must roll back too"
        );

        cleanup(
            &pool,
            &[f.account_a, f.account_b],
            &[f.player_a, f.player_b],
        )
        .await;
    }
}

/// Positive control for the whitelist: an INV_MAIN item still trades
/// cleanly. Without this, a fix that accidentally rejects *all*
/// containers would still pass the negative test above.
#[tokio::test]
async fn lock_items_accepts_inv_main() {
    let pool = require_db_or_skip!();
    let (weapon_type_id, another_type_id) = tradeable_type_ids(&pool).await;
    let f = fixtures(1500);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 0, "a").await;
    insert_account_and_player(&pool, f.account_b, f.player_b, 0, "b").await;
    let item_a = insert_item(&pool, f.player_a, weapon_type_id, INV_MAIN, 0, false).await;
    let item_b = insert_item(&pool, f.player_b, another_type_id, INV_MAIN, 0, false).await;

    let (transport, e2a, conn) = make_state(f.entity_a, f.entity_b);
    let db = Some(Arc::new(pool.clone()));

    handle_execute_trade(
        f.entity_a,
        f.player_a,
        f.entity_b,
        f.player_b,
        vec![item_a],
        0,
        vec![item_b],
        0,
        &db,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        owner_of(&pool, item_a).await,
        Some(f.player_b),
        "INV_MAIN item must move on success"
    );
    assert_eq!(owner_of(&pool, item_b).await, Some(f.player_a));

    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;
}
