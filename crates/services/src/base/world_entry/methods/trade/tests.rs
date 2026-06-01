//! Live-DB integration tests for `handle_execute_trade`.
//!
//! Skip cleanly when `DATABASE_URL` is unset. Against the bundled
//! local Postgres they exercise the atomic happy-path, the rollback
//! semantics on insufficient cash / missing items, the bound-item
//! rejection guard, and the not-enough-slots rollback.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::inventory::{
    INV_BANDOLIER, INV_BANK, INV_BUYBACK, INV_CHEST, INV_MAIN, INV_MISSION,
};
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

/// Container-whitelist regression guard for the security review on
/// PR #438.
///
/// `lock_items` must reject items from any container that isn't
/// `INV_MAIN`. The pre-fix code only rejected `INV_BUYBACK`; equipped
/// gear (INV_CHEST), mission items (INV_MISSION), bank (INV_BANK),
/// bandolier (INV_BANDOLIER) were all trade-eligible.
///
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
            WEAPON_TYPE_ID,
            *forbidden_container,
            0,
            false,
        )
        .await;
        let good_item_b = insert_item(&pool, f.player_b, ANOTHER_TYPE_ID, INV_MAIN, 0, false).await;

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
    let f = fixtures(1500);
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

/// `handle_execute_trade` early-returns and emits Cancelled to both
/// players when `db_pool` is `None`. This path is reachable in
/// production only if the server starts in DB-less mode, but the
/// branch must remain a clean early-return — without it, the
/// `pool.clone()` on `None` would panic instead of cancelling.
///
/// Unit-level guard (no DB needed). The fan-out byte test would be
/// brittle against the witness-helper fan-out shape; the assertion
/// here is the warn log + the no-panic behaviour, which is what the
/// branch promises.
///
/// Revert-verifier: replacing the `None => { ... return; }` arm with
/// `None => panic!(...)` (or removing the early return) crashes the
/// test by panicking inside the trade task.
#[tokio::test]
async fn no_db_pool_cancels_both_without_panic() {
    use crate::test_support::LogCapture;
    let capture = LogCapture::install();

    let entity_a: u32 = 1234;
    let entity_b: u32 = 5678;
    let (transport, e2a, conn) = make_state(entity_a, entity_b);
    let db: Option<Arc<PgPool>> = None;

    // The handler must not panic with db_pool: None — it must early-
    // return after emitting Cancelled to both clients.
    handle_execute_trade(
        entity_a,
        11,
        entity_b,
        22,
        vec![100],
        0,
        vec![200],
        0,
        &db,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    let event = capture
        .find_message(
            tracing::Level::WARN,
            "ExecuteTrade: no DB pool — sending Cancelled to both",
        )
        .expect(
            "handle_execute_trade with db_pool=None MUST log the WARN \
             before sending Cancelled. A regression that drops the warn \
             (or replaces it with the wrong-level log) makes operator \
             diagnosis of DB-less startups invisible.",
        );
    assert!(event.has_field("entity_id", "1234"));
    assert!(event.has_field("partner_entity_id", "5678"));
}

/// Negative-cash rejection is documented as a no-DB-work early return.
/// We exercise it WITHOUT a DB pool here to prove the order of
/// operations: db_pool-None check FIRST (the warn says "no DB pool"),
/// negative-cash check SECOND. If a refactor swaps the order, the
/// log substring on no-pool no-cash trades would change.
///
/// The no-DB no-pool warn must fire FIRST when both conditions hold,
/// because the function takes `db_pool` by reference and reads it
/// before inspecting the cash fields.
#[tokio::test]
async fn no_db_pool_warn_fires_before_negative_cash_warn() {
    use crate::test_support::LogCapture;
    let capture = LogCapture::install();

    let entity_a: u32 = 0xAAAA;
    let entity_b: u32 = 0xBBBB;
    let (transport, e2a, conn) = make_state(entity_a, entity_b);
    let db: Option<Arc<PgPool>> = None;

    handle_execute_trade(
        entity_a,
        11,
        entity_b,
        22,
        vec![],
        -100,
        vec![],
        0,
        &db,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    // Only the no-DB-pool warn must fire — not the negative-cash one.
    assert!(
        capture
            .find_message(tracing::Level::WARN, "ExecuteTrade: no DB pool")
            .is_some(),
        "no-DB-pool warn must fire first when both no-pool and negative-cash hold"
    );
    assert!(
        capture
            .find_message(tracing::Level::WARN, "negative cash in proposal")
            .is_none(),
        "negative-cash warn must NOT fire when the no-DB-pool branch already \
         early-returned. A regression that re-orders the checks would let \
         both fire — fixable but the ordering invariant is load-bearing for \
         the SigNoz dashboard's 'reason' breakdown."
    );
}

async fn slot_id_of(pool: &PgPool, item_id: i32) -> Option<i32> {
    sqlx::query_scalar("SELECT slot_id FROM sgw_inventory WHERE item_id = $1")
        .bind(item_id)
        .fetch_optional(pool)
        .await
        .expect("read slot_id")
}

/// Regression guard for the recipient-slot-reservation accounting bug.
///
/// Pre-fix, `reserve_main_slots_for` queried the recipient's INV_MAIN
/// directly and counted every existing row as occupied — including the
/// row(s) about to vacate via the same trade transaction. A full-bag
/// swap where one of the recipient's "full" slots holds the item they
/// themselves are trading away would then fail spuriously with
/// `NotEnoughSlots`, even though the slot WILL be free by commit time.
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
    let item_a = insert_item(&pool, f.player_a, WEAPON_TYPE_ID, INV_MAIN, 0, false).await;

    // player_b: 40 items in INV_MAIN slots 0..=39 (the bag is full).
    // Slot 39 holds the item being traded out.
    let mut b_filler_items: Vec<i32> = Vec::with_capacity(40);
    for slot in 0..40 {
        // Mix the type ids so a future stack-aware accounting change
        // can't accidentally pass by coalescing stacks.
        let t = if slot % 2 == 0 {
            WEAPON_TYPE_ID
        } else {
            ANOTHER_TYPE_ID
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
