//! Live-DB integration tests for `handle_execute_trade`.
//!
//! Skip cleanly when `DATABASE_URL` is unset. Against the bundled
//! local Postgres they exercise the atomic happy-path, the rollback
//! semantics on insufficient cash / missing items, the bound-item
//! rejection guard, and the not-enough-slots rollback.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::inventory::{INV_BUYBACK, INV_MAIN};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::handle_execute_trade;
use crate::base::ConnectedClientState;
use crate::test_support::require_db_or_skip;
use crate::test_support::TestTransport;

/// Sentinel base for trade live-DB tests. Distinct from prior live-DB
/// sentinels (purchase +0x0A00 / ammo +0x0B00 / vendor_data +0x0C00 /
/// player_load +0x0D00 / sync_bandolier +0x0E00).
const TEST_BASE: i32 = 0x7000_0F00;

/// Tradeable item type id. Resources.items must have a row for this
/// id — picked from the live DB seed.
const WEAPON_TYPE_ID: i32 = 3241;
const ANOTHER_TYPE_ID: i32 = 3242;

async fn cleanup(pool: &PgPool, accounts: &[i32], players: &[i32]) {
    for &p in players {
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(p)
            .execute(pool)
            .await;
    }
    for &a in accounts {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(a)
            .execute(pool)
            .await;
    }
}

async fn insert_account_and_player(
    pool: &PgPool,
    account_id: i32,
    player_id: i32,
    naquadah: i32,
    label: &str,
) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, $2, '')",
    )
    .bind(account_id)
    .bind(format!("trade-test-{account_id}-{label}"))
    .execute(pool)
    .await
    .expect("insert account");

    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, $4, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("trader-{player_id}"))
    .bind(naquadah)
    .execute(pool)
    .await
    .expect("insert player");
}

async fn insert_item(
    pool: &PgPool,
    player_id: i32,
    type_id: i32,
    container_id: i32,
    slot_id: i32,
    bound: bool,
) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges) \
         VALUES ($1, $2, 1, $3, $4, $5, 100, 0) \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(slot_id)
    .bind(container_id)
    .bind(bound)
    .fetch_one(pool)
    .await
    .expect("insert item")
}

async fn naquadah_of(pool: &PgPool, player_id: i32) -> i32 {
    sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .expect("read naquadah")
}

async fn owner_of(pool: &PgPool, item_id: i32) -> Option<i32> {
    sqlx::query_scalar("SELECT character_id FROM sgw_inventory WHERE item_id = $1")
        .bind(item_id)
        .fetch_optional(pool)
        .await
        .expect("read owner")
}

#[derive(Debug, Clone)]
struct TradeFixtures {
    account_a: i32,
    account_b: i32,
    player_a: i32,
    player_b: i32,
    entity_a: u32,
    entity_b: u32,
}

fn fixtures(salt: i32) -> TradeFixtures {
    TradeFixtures {
        account_a: TEST_BASE + salt,
        account_b: TEST_BASE + salt + 1,
        player_a: TEST_BASE + salt + 10,
        player_b: TEST_BASE + salt + 11,
        entity_a: (TEST_BASE + salt + 100) as u32,
        entity_b: (TEST_BASE + salt + 101) as u32,
    }
}

fn make_state(
    entity_a: u32,
    entity_b: u32,
) -> (
    Arc<dyn Transport>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let addr_a: SocketAddr = "127.0.0.1:65534".parse().unwrap();
    let addr_b: SocketAddr = "127.0.0.1:65533".parse().unwrap();
    let mut m = HashMap::new();
    m.insert(entity_a, addr_a);
    m.insert(entity_b, addr_b);
    let entity_to_addr = Arc::new(Mutex::new(m));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    (transport, entity_to_addr, connected)
}

/// Happy path: each player gives one item + some cash. After the commit,
/// item ownership is swapped and naquadah is debited/credited per-side.
#[tokio::test]
async fn commit_swaps_items_atomically() {
    let pool = require_db_or_skip!();
    let f = fixtures(0);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 1_000, "a").await;
    insert_account_and_player(&pool, f.account_b, f.player_b, 500, "b").await;
    let item_a = insert_item(&pool, f.player_a, WEAPON_TYPE_ID, INV_MAIN, 0, false).await;
    let item_b = insert_item(&pool, f.player_b, ANOTHER_TYPE_ID, INV_MAIN, 0, false).await;

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
    let f = fixtures(200);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 10, "a").await; // poor
    insert_account_and_player(&pool, f.account_b, f.player_b, 500, "b").await;
    let item_a = insert_item(&pool, f.player_a, WEAPON_TYPE_ID, INV_MAIN, 0, false).await;
    let item_b = insert_item(&pool, f.player_b, ANOTHER_TYPE_ID, INV_MAIN, 0, false).await;

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
    let f = fixtures(400);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 100, "a").await;
    insert_account_and_player(&pool, f.account_b, f.player_b, 100, "b").await;
    let item_b = insert_item(&pool, f.player_b, ANOTHER_TYPE_ID, INV_MAIN, 0, false).await;

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
    let f = fixtures(600);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 0, "a").await;
    insert_account_and_player(&pool, f.account_b, f.player_b, 0, "b").await;
    let bound_item = insert_item(&pool, f.player_a, WEAPON_TYPE_ID, INV_MAIN, 0, true).await;
    let item_b = insert_item(&pool, f.player_b, ANOTHER_TYPE_ID, INV_MAIN, 0, false).await;

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
    let f = fixtures(800);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 0, "a").await;
    insert_account_and_player(&pool, f.account_b, f.player_b, 0, "b").await;
    let buyback_item = insert_item(&pool, f.player_a, WEAPON_TYPE_ID, INV_BUYBACK, 0, false).await;
    let item_b = insert_item(&pool, f.player_b, ANOTHER_TYPE_ID, INV_MAIN, 0, false).await;

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
    let f = fixtures(1000);
    cleanup(
        &pool,
        &[f.account_a, f.account_b],
        &[f.player_a, f.player_b],
    )
    .await;

    insert_account_and_player(&pool, f.account_a, f.player_a, 0, "a").await;
    insert_account_and_player(&pool, f.account_b, f.player_b, 0, "b").await;
    let item_a = insert_item(&pool, f.player_a, WEAPON_TYPE_ID, INV_MAIN, 0, false).await;
    let item_b = insert_item(&pool, f.player_b, ANOTHER_TYPE_ID, INV_MAIN, 0, false).await;

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
