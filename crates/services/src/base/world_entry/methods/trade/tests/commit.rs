//! Core commit/rollback live-DB tests for `handle_execute_trade`.
//!
//! Happy path + per-validation rollback scenarios:
//! - happy path: items + cash both swap
//! - insufficient cash on one side → rollback
//! - offering an item the actor doesn't own → rollback
//! - bound (soul-bound) item offered → rollback
//! - item in buyback bag offered → rollback
//! - same instance listed twice in a proposal → rollback
//! - negative cash field → rollback (pre-DB-work)

use std::sync::Arc;

use cimmeria_entity::inventory::{INV_BUYBACK, INV_MAIN};

use super::{
    cleanup, fixtures, insert_account_and_player, insert_item, make_state, naquadah_of, owner_of,
    tradeable_type_ids,
};
use crate::base::world_entry::methods::trade::handle_execute_trade;
use crate::test_support::require_db_or_skip;

/// Happy path: each player gives one item + some cash. After the commit,
/// item ownership is swapped and naquadah is debited/credited per-side.
#[tokio::test]
async fn commit_swaps_items_atomically() {
    let pool = require_db_or_skip!();
    let (weapon_type_id, another_type_id) = tradeable_type_ids(&pool).await;
    let f = fixtures(0);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 1_000, "a").await;
    insert_account_and_player(&pool, f.account_b, f.player_b, 500, "b").await;
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
        100,
        vec![item_b],
        50,
        &db,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    // Item ownership flipped.
    assert_eq!(
        owner_of(&pool, item_a).await,
        Some(f.player_b),
        "item_a should now belong to player_b"
    );
    assert_eq!(
        owner_of(&pool, item_b).await,
        Some(f.player_a),
        "item_b should now belong to player_a"
    );

    // Cash net delta: a gave 100, received 50 → -50. b gave 50, received 100 → +50.
    assert_eq!(naquadah_of(&pool, f.player_a).await, 950);
    assert_eq!(naquadah_of(&pool, f.player_b).await, 550);

    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;
}

/// Insufficient cash on one side rolls back the entire swap. No items
/// move; no cash changes.
#[tokio::test]
async fn commit_rolls_back_on_insufficient_cash() {
    let pool = require_db_or_skip!();
    let (weapon_type_id, another_type_id) = tradeable_type_ids(&pool).await;
    let f = fixtures(200);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 10, "a").await; // poor
    insert_account_and_player(&pool, f.account_b, f.player_b, 500, "b").await;
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
        1_000_000, // far more than player_a has
        vec![item_b],
        50,
        &db,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    // Nothing moved.
    assert_eq!(owner_of(&pool, item_a).await, Some(f.player_a));
    assert_eq!(owner_of(&pool, item_b).await, Some(f.player_b));
    assert_eq!(naquadah_of(&pool, f.player_a).await, 10);
    assert_eq!(naquadah_of(&pool, f.player_b).await, 500);

    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;
}

/// Offering an item the player doesn't own (was forged client-side, or
/// the item has been removed between proposal-update and commit) rolls
/// back the whole swap.
#[tokio::test]
async fn commit_rolls_back_on_missing_item() {
    let pool = require_db_or_skip!();
    let (_weapon_type_id, another_type_id) = tradeable_type_ids(&pool).await;
    let f = fixtures(400);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 100, "a").await;
    insert_account_and_player(&pool, f.account_b, f.player_b, 100, "b").await;
    let item_b = insert_item(&pool, f.player_b, another_type_id, INV_MAIN, 0, false).await;

    let (transport, e2a, conn) = make_state(f.entity_a, f.entity_b);
    let db = Some(Arc::new(pool.clone()));

    handle_execute_trade(
        f.entity_a,
        f.player_a,
        f.entity_b,
        f.player_b,
        vec![999_999_999], // player_a doesn't own this
        0,
        vec![item_b],
        0,
        &db,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    // item_b is still with player_b; no cash change.
    assert_eq!(owner_of(&pool, item_b).await, Some(f.player_b));
    assert_eq!(naquadah_of(&pool, f.player_a).await, 100);
    assert_eq!(naquadah_of(&pool, f.player_b).await, 100);

    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;
}

/// Bound items must be rejected by the validation gauntlet — bound items
/// are "soul-bound" and never tradeable. The whole swap rolls back.
#[tokio::test]
async fn commit_rolls_back_on_bound_item() {
    let pool = require_db_or_skip!();
    let (weapon_type_id, another_type_id) = tradeable_type_ids(&pool).await;
    let f = fixtures(600);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 0, "a").await;
    insert_account_and_player(&pool, f.account_b, f.player_b, 0, "b").await;
    let bound_item = insert_item(&pool, f.player_a, weapon_type_id, INV_MAIN, 0, true).await;
    let item_b = insert_item(&pool, f.player_b, another_type_id, INV_MAIN, 0, false).await;

    let (transport, e2a, conn) = make_state(f.entity_a, f.entity_b);
    let db = Some(Arc::new(pool.clone()));

    handle_execute_trade(
        f.entity_a,
        f.player_a,
        f.entity_b,
        f.player_b,
        vec![bound_item],
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
        owner_of(&pool, bound_item).await,
        Some(f.player_a),
        "bound items must not move"
    );
    assert_eq!(owner_of(&pool, item_b).await, Some(f.player_b));

    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;
}

/// Items in the buyback bag (INV_BUYBACK = 16) can be reclaimed by the
/// player who sold them, but they MUST NOT be trade-eligible — otherwise
/// players could shuttle items through the buyback bag to skirt other
/// item-state restrictions. The commit rolls back.
#[tokio::test]
async fn commit_rolls_back_on_buyback_bag_item() {
    let pool = require_db_or_skip!();
    let (weapon_type_id, another_type_id) = tradeable_type_ids(&pool).await;
    let f = fixtures(800);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 0, "a").await;
    insert_account_and_player(&pool, f.account_b, f.player_b, 0, "b").await;
    let buyback_item = insert_item(&pool, f.player_a, weapon_type_id, INV_BUYBACK, 0, false).await;
    let item_b = insert_item(&pool, f.player_b, another_type_id, INV_MAIN, 0, false).await;

    let (transport, e2a, conn) = make_state(f.entity_a, f.entity_b);
    let db = Some(Arc::new(pool.clone()));

    handle_execute_trade(
        f.entity_a,
        f.player_a,
        f.entity_b,
        f.player_b,
        vec![buyback_item],
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
        owner_of(&pool, buyback_item).await,
        Some(f.player_a),
        "buyback-bag items must not change hands"
    );
    assert_eq!(owner_of(&pool, item_b).await, Some(f.player_b));

    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;
}

/// Same item instance listed twice in a single proposal would otherwise
/// cause the FOR UPDATE to lock the same row twice (harmless) AND the
/// UPDATE to fire twice against a stale (now-recipient) row — the second
/// move would silently move the item to a third character_id. Reject up
/// front.
#[tokio::test]
async fn commit_rolls_back_on_duplicate_instance_in_proposal() {
    let pool = require_db_or_skip!();
    let (weapon_type_id, another_type_id) = tradeable_type_ids(&pool).await;
    let f = fixtures(1000);
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
        vec![item_a, item_a], // same id twice
        0,
        vec![item_b],
        0,
        &db,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(owner_of(&pool, item_a).await, Some(f.player_a));
    assert_eq!(owner_of(&pool, item_b).await, Some(f.player_b));

    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;
}

/// Negative cash in either proposal is structurally invalid — reject
/// before any DB work.
#[tokio::test]
async fn commit_rolls_back_on_negative_cash() {
    let pool = require_db_or_skip!();
    let f = fixtures(1200);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 100, "a").await;
    insert_account_and_player(&pool, f.account_b, f.player_b, 100, "b").await;

    let (transport, e2a, conn) = make_state(f.entity_a, f.entity_b);
    let db = Some(Arc::new(pool.clone()));

    handle_execute_trade(
        f.entity_a,
        f.player_a,
        f.entity_b,
        f.player_b,
        vec![],
        -50, // negative — would credit player_a if processed
        vec![],
        0,
        &db,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    // No cash change on either side.
    assert_eq!(naquadah_of(&pool, f.player_a).await, 100);
    assert_eq!(naquadah_of(&pool, f.player_b).await, 100);

    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;
}
