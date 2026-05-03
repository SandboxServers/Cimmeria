//! Live-DB integration tests for `handle_move_inventory_item`.
//!
//! Skip cleanly when `DATABASE_URL` is unset; against the bundled local
//! Postgres they exercise all three transaction paths (simple/split/swap)
//! plus the source-not-found and split-rejected boundary cases.

use super::*;
use crate::test_support::require_db_or_skip;

/// Sentinel base for live-DB move-inventory tests. Stays well below
/// i32::MAX (sgw_player.player_id is `integer`). Per-test offsets
/// keep concurrent runs from colliding on the same rows.
const TEST_BASE: i32 = 0x7000_0200;

/// Cleanup deletes the account; sgw_player rows cascade off via
/// sgw_player_account_id_fkey. sgw_inventory does NOT cascade off
/// account, so delete those rows by character_id explicitly.
async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

async fn insert_account_and_player(pool: &PgPool, account_id: i32, player_id: i32) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, $2, '')",
    )
    .bind(account_id)
    .bind(format!("move-inv-{account_id}"))
    .execute(pool)
    .await
    .expect("insert account");

    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("test-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
}

/// Picks `count` distinct item_ids from resources.items whose
/// container_sets either is NULL or includes container 1 — i.e. items
/// that pass `item_allows_container(.., container_id=1)`. Necessary
/// because sgw_inventory.type_id has an FK to resources.items, so the
/// fixture rows can't use synthetic type_ids.
async fn pick_main_bag_type_ids(pool: &PgPool, count: usize) -> Vec<i32> {
    let ids: Vec<i32> = sqlx::query_scalar::<_, i32>(
        "SELECT item_id FROM resources.items \
         WHERE container_sets IS NULL OR 1 = ANY(container_sets) \
         ORDER BY item_id LIMIT $1",
    )
    .bind(count as i64)
    .fetch_all(pool)
    .await
    .expect("pick item_ids");
    assert_eq!(
        ids.len(),
        count,
        "resources.items has fewer than {count} container-1-allowed types — \
         the bundled seed (db/resources/Items/Seed/items.sql) must be loaded \
         before running live-DB tests",
    );
    ids
}

/// Compute an item_id that's guaranteed not to exist in `sgw_inventory`.
/// Hard-coded sentinels collide with the auto-increment sequence as it
/// advances over time; deriving from MAX(item_id) is stable across runs.
async fn unused_item_id(pool: &PgPool) -> i32 {
    let max_id: Option<i32> = sqlx::query_scalar("SELECT MAX(item_id) FROM sgw_inventory")
        .fetch_one(pool)
        .await
        .expect("MAX(item_id) query");
    max_id.unwrap_or(10_000) + 1_000_000
}

/// Insert a sgw_inventory row at a known (container, slot) and return
/// the auto-generated item_id. The caller must supply a `type_id`
/// that exists in resources.items (use [`pick_main_bag_type_ids`]).
async fn insert_item(
    pool: &PgPool,
    player_id: i32,
    type_id: i32,
    container_id: i32,
    slot_id: i32,
    stack_size: i32,
) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges) \
         VALUES ($1, $2, $3, $4, $5, false, 100, 0) \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(stack_size)
    .bind(slot_id)
    .bind(container_id)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

async fn slot_of(pool: &PgPool, player_id: i32, item_id: i32) -> Option<(i32, i32, i32)> {
    sqlx::query_as::<_, (i32, i32, i32)>(
        "SELECT container_id, slot_id, stack_size FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(pool)
    .await
    .expect("slot_of query")
}

fn make_state(
    entity_id: u32,
) -> (
    Arc<UdpSocket>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    // bind synchronously via std then wrap — avoids needing tokio at the
    // helper boundary (caller is already in #[tokio::test]).
    let std_sock = std::net::UdpSocket::bind("127.0.0.1:0").expect("bind UDP");
    std_sock.set_nonblocking(true).unwrap();
    let socket = Arc::new(UdpSocket::from_std(std_sock).expect("from_std"));
    let fake_addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(entity_id, fake_addr);
        m
    }));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    (socket, entity_to_addr, connected)
}

#[tokio::test]
async fn simple_move_relocates_full_stack_to_empty_slot() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE;
    let player_id = TEST_BASE + 1;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    let types = pick_main_bag_type_ids(&pool, 2).await;
    let item_a = insert_item(&pool, player_id, types[0], 1, 0, 1).await;
    let item_b = insert_item(&pool, player_id, types[1], 1, 1, 1).await;

    let (socket, e2a, conn) = make_state(9_999_010);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_move_inventory_item(
        9_999_010, player_id, item_a, 1, 5, 1, &db_pool, &None, &socket, &conn, &e2a,
    )
    .await;

    assert_eq!(
        slot_of(&pool, player_id, item_a).await,
        Some((1, 5, 1)),
        "item_a moved from slot 0 to slot 5"
    );
    assert_eq!(
        slot_of(&pool, player_id, item_b).await,
        Some((1, 1, 1)),
        "item_b at slot 1 must be untouched"
    );

    cleanup(&pool, account_id, player_id).await;
}

/// Swap path: full-quantity move into an occupied slot. Both rows exchange
/// positions via the three-step park/move/move sequence guarded by the
/// (player_id, 0) advisory lock and the unique-slot index.
#[tokio::test]
async fn swap_exchanges_two_items_full_stacks() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 100;
    let player_id = TEST_BASE + 101;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    let types = pick_main_bag_type_ids(&pool, 2).await;
    let item_a = insert_item(&pool, player_id, types[0], 1, 0, 1).await;
    let item_b = insert_item(&pool, player_id, types[1], 1, 7, 1).await;

    let (socket, e2a, conn) = make_state(9_999_011);
    let db_pool = Some(Arc::new(pool.clone()));

    // Move A from slot 0 → slot 7 (occupied by B). Both should exchange.
    handle_move_inventory_item(
        9_999_011, player_id, item_a, 1, 7, 1, &db_pool, &None, &socket, &conn, &e2a,
    )
    .await;

    assert_eq!(slot_of(&pool, player_id, item_a).await, Some((1, 7, 1)));
    assert_eq!(slot_of(&pool, player_id, item_b).await, Some((1, 0, 1)));

    cleanup(&pool, account_id, player_id).await;
}

/// Split rejected: a partial-quantity move (quantity < source.stack_size)
/// targeting an occupied slot must abort with no DB changes. Without the
/// guard, the split path's INSERT would collide with the unique-slot
/// index and surface as a user-visible error on what should be a
/// silent "won't merge stacks" rejection.
#[tokio::test]
async fn split_rejected_when_target_occupied() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 200;
    let player_id = TEST_BASE + 201;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    let types = pick_main_bag_type_ids(&pool, 2).await;
    let item_a = insert_item(&pool, player_id, types[0], 1, 0, 5).await; // stack of 5
    let item_b = insert_item(&pool, player_id, types[1], 1, 3, 1).await;

    let (socket, e2a, conn) = make_state(9_999_012);
    let db_pool = Some(Arc::new(pool.clone()));

    // Try to split 2 of A into slot 3 (occupied by B). Should reject.
    handle_move_inventory_item(
        9_999_012, player_id, item_a, 1, 3, 2, &db_pool, &None, &socket, &conn, &e2a,
    )
    .await;

    // A's stack must still be 5 — split decrement was rolled back.
    assert_eq!(
        slot_of(&pool, player_id, item_a).await,
        Some((1, 0, 5)),
        "split rejection must not decrement source stack"
    );
    // B unchanged.
    assert_eq!(slot_of(&pool, player_id, item_b).await, Some((1, 3, 1)));
    // No new row inserted at slot 3.
    let row_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = 1 AND slot_id = 3",
    )
    .bind(player_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row_count, 1, "exactly one row at slot 3 (the original B)");

    cleanup(&pool, account_id, player_id).await;
}

/// Split into an EMPTY target slot is the happy path: the source stack
/// drops by `quantity` and a new row appears at the target with that
/// quantity. The source-comment guard at the split decrement (matched 0
/// rows -> warn + abort) protects this path against concurrent
/// modification eating the decrement silently.
#[tokio::test]
async fn split_into_empty_slot_creates_new_row_and_decrements_source() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 300;
    let player_id = TEST_BASE + 301;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    let types = pick_main_bag_type_ids(&pool, 1).await;
    let item_a = insert_item(&pool, player_id, types[0], 1, 0, 5).await;

    let (socket, e2a, conn) = make_state(9_999_013);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_move_inventory_item(
        9_999_013, player_id, item_a, 1, 8, 2, &db_pool, &None, &socket, &conn, &e2a,
    )
    .await;

    // Source stack: 5 - 2 = 3, still at slot 0.
    assert_eq!(slot_of(&pool, player_id, item_a).await, Some((1, 0, 3)));

    // New row at slot 8 with stack 2 and the same type_id.
    let new_row: Option<(i32, i32, i32)> = sqlx::query_as(
        "SELECT type_id, slot_id, stack_size FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = 1 AND slot_id = 8",
    )
    .bind(player_id)
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert_eq!(
        new_row,
        Some((types[0], 8, 2)),
        "split must create a new row at the target with the same type_id"
    );

    cleanup(&pool, account_id, player_id).await;
}

/// Source-not-found case: the function silently returns (with a warn)
/// without modifying any rows. Asserts via a sentinel sibling whose
/// position must not change.
#[tokio::test]
async fn source_item_not_found_makes_no_changes() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 400;
    let player_id = TEST_BASE + 401;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    let types = pick_main_bag_type_ids(&pool, 1).await;
    let bystander = insert_item(&pool, player_id, types[0], 1, 4, 1).await;
    let nonexistent_item_id = unused_item_id(&pool).await;

    let (socket, e2a, conn) = make_state(9_999_014);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_move_inventory_item(
        9_999_014,
        player_id,
        nonexistent_item_id,
        1,
        7,
        1,
        &db_pool,
        &None,
        &socket,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        slot_of(&pool, player_id, bystander).await,
        Some((1, 4, 1)),
        "bystander must be untouched when source item doesn't exist"
    );
    assert!(
        slot_of(&pool, player_id, nonexistent_item_id)
            .await
            .is_none(),
        "function must not INSERT a phantom row for the missing source"
    );

    cleanup(&pool, account_id, player_id).await;
}
