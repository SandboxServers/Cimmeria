//! Live-DB concurrency regression guards for `handle_move_inventory_item`.
//!
//! These tests use a multi-thread tokio runtime + a barrier to force the
//! spawned tasks to hit the advisory-lock path simultaneously, so a
//! regressed implementation that lost the lock loses the race against
//! the `sgw_inventory_unique_slot` UNIQUE INDEX or, in the swap pair,
//! against the FOR-UPDATE row locks (AB-BA deadlock).
//!
//! Skip cleanly when `DATABASE_URL` is unset; against the bundled local
//! Postgres they exercise the move-vs-grant lock contention path and
//! the opposite-direction-swap deadlock-free guarantee.
//!
//! Non-concurrency move-handler tests live in the sibling `tests` module;
//! the helpers from there are reused via `super::tests::…`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;

use super::super::grant::handle_grant_item;
use super::handle_move_inventory_item;
use super::tests::{
    cleanup as cleanup_inventory, insert_account_and_player, insert_item, pick_main_bag_type_ids,
    TEST_BASE,
};
use crate::base::ConnectedClientState;
use crate::test_support::require_db_or_skip;

/// Cleanup variant that also drains cell_event_outbox rows enqueued by
/// `handle_grant_item` pre-commit.
///
/// `entity_id` is bound as `i64` rather than `as i32` so a sentinel above
/// `i32::MAX` can't silently wrap to a negative `INTEGER` and clean up the
/// wrong rows. The `cell_event_outbox.entity_id` column is `INTEGER`;
/// Postgres handles the implicit narrowing at bind time and rejects the
/// query if the value really doesn't fit, which is the loud-failure mode
/// we want.
async fn cleanup_with_outbox(pool: &PgPool, account_id: i32, player_id: i32, entity_id: u32) {
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
        .bind(i64::from(entity_id))
        .execute(pool)
        .await;
    cleanup_inventory(pool, account_id, player_id).await;
}

/// Build a fresh per-test `(socket, entity_to_addr, connected)` context
/// tuple. The contained `Arc<...>` handles get cloned into each spawned
/// task by the concurrency tests below, which is the intended sharing
/// (the regression guards exercise contention on shared state, not
/// per-task isolation). Used by the multi-thread `tokio::test` runtimes.
fn make_state_for_entity(
    entity_id: u32,
) -> (
    Arc<UdpSocket>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
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

/// Regression guard: a move and a grant against the same container must
/// serialize through the per-(player, container) advisory lock so they pick
/// distinct slots. Without that lock, both the move's `reserve_free_inventory_slots`-
/// equivalent target read and the grant's slot reservation could see the
/// same slot free, both INSERT/UPDATE into it, and the unique-slot index
/// rejects one — turning a legitimate operation into a user-visible error.
///
/// Setup: A at (1, 0). Spawn move A → (1, 7) and a concurrent grant of a
/// new item into container 1. Both must commit cleanly with all rows on
/// distinct slots.
///
/// The cross-container axis isn't exercised here: under the current item
/// seed no `resources.items` row allows both container 1 and container 2,
/// so the move can't cross containers and "move-vs-grant on target" vs
/// "...on source" collapse to the same single-container scenario. Both
/// directions exercise the same `pg_advisory_xact_lock(player_id, 1)`
/// primitive.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn move_and_concurrent_grant_serialize_on_container_lock() {
    use tokio::sync::Barrier;

    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 500;
    let player_id = TEST_BASE + 501;
    let entity_id: u32 = 0x7000_0210;
    cleanup_with_outbox(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    let types = pick_main_bag_type_ids(&pool, 2).await;
    let item_a = insert_item(&pool, player_id, types[0], 1, 0, 1).await;

    let (socket, e2a, conn) = make_state_for_entity(entity_id);
    let db_pool = Some(Arc::new(pool.clone()));
    let barrier = Arc::new(Barrier::new(2));

    let move_handle = {
        let db_pool = db_pool.clone();
        let socket = socket.clone();
        let conn = conn.clone();
        let e2a = e2a.clone();
        let barrier = barrier.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            handle_move_inventory_item(
                entity_id, player_id, item_a, 1, 7, 1, &db_pool, &None, &socket, &conn, &e2a,
            )
            .await;
        })
    };
    let grant_handle = {
        let grant_type = types[1];
        let db_pool = db_pool.clone();
        let socket = socket.clone();
        let conn = conn.clone();
        let e2a = e2a.clone();
        let barrier = barrier.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            handle_grant_item(
                entity_id, player_id, grant_type, 1, 1, &db_pool, &None, &socket, &conn, &e2a,
            )
            .await;
        })
    };
    // Cap with a timeout so a regression that introduces a hang (e.g., a
    // missing lock release on either side of the move/grant pair) surfaces
    // as a clean test failure within 5s rather than wedging the suite.
    // 5s is generous — both ops together complete in <100ms when the
    // advisory lock works.
    let timeout = std::time::Duration::from_secs(5);
    tokio::time::timeout(timeout, async {
        move_handle.await.expect("move task panicked");
        grant_handle.await.expect("grant task panicked");
    })
    .await
    .expect("move + concurrent grant deadlocked or hung past 5s");

    // Final state: 2 rows in container 1, both on distinct slots, no row
    // at slot = -1 (proving the swap sentinel — if any was used — was
    // cleaned up; in this test no swap happens but the assertion costs
    // nothing and locks in the no-leak invariant).
    let rows: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT item_id, slot_id FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = 1 \
         ORDER BY slot_id",
    )
    .bind(player_id)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
        rows.len(),
        2,
        "both move and grant must commit (got {} rows)",
        rows.len()
    );
    let slot_ids: Vec<i32> = rows.iter().map(|(_, s)| *s).collect();
    let mut sorted = slot_ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 2, "slot_ids must be distinct: {slot_ids:?}");
    assert!(
        slot_ids.iter().all(|&s| s >= 0),
        "no row may be left at the swap sentinel slot -1: {slot_ids:?}"
    );

    // A must end up at (1, 7) regardless of scheduling — the move's target
    // is fixed. The grant lands at whichever slot was free at its turn
    // (slot 0 if the move ran first; slot 1 if the grant ran first while
    // A was still at slot 0).
    let a_slot = rows.iter().find(|(id, _)| *id == item_a).map(|(_, s)| *s);
    assert_eq!(
        a_slot,
        Some(7),
        "moved item A must land at its requested target slot 7"
    );

    cleanup_with_outbox(&pool, account_id, player_id, entity_id).await;
}

/// Opposite-direction concurrent moves (A → B's slot, B → A's slot) must
/// not deadlock on FOR-UPDATE row locks. The per-player advisory lock at
/// `(player_id, 0)` serializes them so the second move re-reads its
/// source after the first commits — and finds it already at the
/// requested target, hitting the same-source-as-target early return.
///
/// Without the per-player lock, each move locks its own source row first,
/// then tries to FOR-UPDATE the other's source as the swap path's
/// occupant query — classic AB-BA deadlock.
///
/// End state is deterministic regardless of which move acquires the lock
/// first: whichever runs first does a real swap (A and B exchange
/// positions); the second re-reads its source, finds it already at the
/// target, and returns via the same-slot early-out. So A must land at
/// (1, 5) and B at (1, 0). A weaker "A and B at distinct positions"
/// assertion would silently accept both moves rolling back (A still at
/// 0, B still at 5).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn opposite_direction_concurrent_swaps_do_not_deadlock() {
    use tokio::sync::Barrier;

    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 600;
    let player_id = TEST_BASE + 601;
    let entity_id: u32 = 0x7000_0220;
    cleanup_with_outbox(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    let types = pick_main_bag_type_ids(&pool, 2).await;
    let item_a = insert_item(&pool, player_id, types[0], 1, 0, 1).await;
    let item_b = insert_item(&pool, player_id, types[1], 1, 5, 1).await;

    let (socket, e2a, conn) = make_state_for_entity(entity_id);
    let db_pool = Some(Arc::new(pool.clone()));
    let barrier = Arc::new(Barrier::new(2));

    let move1 = {
        let db_pool = db_pool.clone();
        let socket = socket.clone();
        let conn = conn.clone();
        let e2a = e2a.clone();
        let barrier = barrier.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            handle_move_inventory_item(
                entity_id, player_id, item_a, 1, 5, 1, &db_pool, &None, &socket, &conn, &e2a,
            )
            .await;
        })
    };
    let move2 = {
        let db_pool = db_pool.clone();
        let socket = socket.clone();
        let conn = conn.clone();
        let e2a = e2a.clone();
        let barrier = barrier.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            handle_move_inventory_item(
                entity_id, player_id, item_b, 1, 0, 1, &db_pool, &None, &socket, &conn, &e2a,
            )
            .await;
        })
    };

    // Cap each move with a timeout so a deadlock surfaces as a test failure
    // rather than wedging the whole suite. 5s is generous — both moves
    // together complete in <100ms when the lock works.
    let timeout = std::time::Duration::from_secs(5);
    tokio::time::timeout(timeout, async {
        move1.await.expect("move1 task panicked");
        move2.await.expect("move2 task panicked");
    })
    .await
    .expect("opposite-direction moves deadlocked or hung past 5s");

    // Deterministic end state: whichever move took the lock first did a
    // real swap; the second saw its source already at the target and
    // early-returned. Either ordering produces A at (1, 5) and B at (1, 0).
    let a = sqlx::query_as::<_, (i32, i32, i32)>(
        "SELECT container_id, slot_id, stack_size FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2",
    )
    .bind(player_id)
    .bind(item_a)
    .fetch_optional(&pool)
    .await
    .unwrap();
    let b = sqlx::query_as::<_, (i32, i32, i32)>(
        "SELECT container_id, slot_id, stack_size FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2",
    )
    .bind(player_id)
    .bind(item_b)
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert_eq!(
        a,
        Some((1, 5, 1)),
        "item A must land at (1, 5) after the serialized swap pair",
    );
    assert_eq!(
        b,
        Some((1, 0, 1)),
        "item B must land at (1, 0) after the serialized swap pair",
    );

    cleanup_with_outbox(&pool, account_id, player_id, entity_id).await;
}

/// Sentinel-slot rollback invariant: the swap path's three-step procedure
/// parks the source row at `slot_id = -1` between statements 1 and 3.
/// If the surrounding transaction rolls back partway, Postgres MUST
/// restore the source row's `slot_id` to its pre-park value — otherwise
/// a failed swap would leave a permanent ghost row at the sentinel slot,
/// and the next move/grant against that container would either spuriously
/// see the row or fail the unique-slot index when it tried to insert at
/// slot 0.
///
/// We can't easily inject a failure between steps 1 and 3 of the real
/// move handler without modifying production code, so this test stages
/// the same SQL sequence directly inside a Rust-managed transaction:
/// park source at -1 (step 1), move occupant into source's old slot
/// (step 2), then roll back. After the rollback both rows must be at
/// their original (container_id, slot_id), and no row may have
/// `slot_id = -1`.
///
/// The complementary success-path invariant — that no row ever observably
/// holds `slot_id = -1` outside an active swap transaction — is exercised
/// implicitly by every other concurrency test in this file: each one
/// finishes with a follow-up sgw_inventory query, and the helpers panic
/// loudly if any row pops up at the sentinel.
#[tokio::test]
async fn sentinel_slot_does_not_leak_when_swap_tx_rolls_back() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 700;
    let player_id = TEST_BASE + 701;
    let entity_id: u32 = 0x7000_0230;
    cleanup_with_outbox(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    let types = pick_main_bag_type_ids(&pool, 2).await;
    // Initial layout matches the swap-path setup: source at (1, 0),
    // occupant at (1, 5).
    let source_item = insert_item(&pool, player_id, types[0], 1, 0, 1).await;
    let occupant_item = insert_item(&pool, player_id, types[1], 1, 5, 1).await;

    // Stage the swap's first two steps inside a tx, then deliberately
    // roll back. This mimics the failure window between the move
    // handler's park-source and move-source-to-target — the rollback
    // must restore both rows to their pre-swap state.
    let mut tx = pool.begin().await.expect("begin tx");

    // Step 1: park source at slot_id = -1. Assert rows_affected == 1 so a
    // future fixture mismatch (source row missing under this character)
    // surfaces here as a clear failure rather than as a confusing SELECT
    // error later in the test.
    let park = sqlx::query(
        "UPDATE sgw_inventory SET slot_id = -1 \
         WHERE character_id = $1 AND item_id = $2",
    )
    .bind(player_id)
    .bind(source_item)
    .execute(&mut *tx)
    .await
    .expect("park source");
    assert_eq!(
        park.rows_affected(),
        1,
        "park-source UPDATE should affect exactly the source row; got {}",
        park.rows_affected(),
    );

    // Step 2: move occupant into source's old slot. Same rows_affected
    // guard for the same reason.
    let move_occ = sqlx::query(
        "UPDATE sgw_inventory SET container_id = 1, slot_id = 0 \
         WHERE character_id = $1 AND item_id = $2",
    )
    .bind(player_id)
    .bind(occupant_item)
    .execute(&mut *tx)
    .await
    .expect("move occupant");
    assert_eq!(
        move_occ.rows_affected(),
        1,
        "move-occupant UPDATE should affect exactly the occupant row; got {}",
        move_occ.rows_affected(),
    );

    // Sanity-check inside the tx: source should be at -1 right now.
    let source_in_tx: i32 = sqlx::query_scalar(
        "SELECT slot_id FROM sgw_inventory WHERE character_id = $1 AND item_id = $2",
    )
    .bind(player_id)
    .bind(source_item)
    .fetch_one(&mut *tx)
    .await
    .expect("read source slot inside tx");
    assert_eq!(
        source_in_tx, -1,
        "inside the staged tx, source should be parked at sentinel slot -1",
    );

    // Roll back instead of committing — this is the "failed swap"
    // simulation.
    tx.rollback().await.expect("rollback");

    // After rollback: both rows must be back at their original
    // positions, and NO row anywhere should have slot_id = -1 for this
    // player (or any player — the sentinel is an in-flight value that
    // must never persist).
    let source_after: (i32, i32) = sqlx::query_as(
        "SELECT container_id, slot_id FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2",
    )
    .bind(player_id)
    .bind(source_item)
    .fetch_one(&pool)
    .await
    .expect("read source after rollback");
    assert_eq!(
        source_after,
        (1, 0),
        "source row's (container, slot) must restore to (1, 0) after rollback; got {source_after:?}",
    );

    let occupant_after: (i32, i32) = sqlx::query_as(
        "SELECT container_id, slot_id FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2",
    )
    .bind(player_id)
    .bind(occupant_item)
    .fetch_one(&pool)
    .await
    .expect("read occupant after rollback");
    assert_eq!(
        occupant_after,
        (1, 5),
        "occupant row's (container, slot) must restore to (1, 5) after rollback; got {occupant_after:?}",
    );

    let sentinel_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_inventory WHERE slot_id = -1")
            .fetch_one(&pool)
            .await
            .expect("count sentinel rows");
    assert_eq!(
        sentinel_rows, 0,
        "no row may persist at slot_id=-1 after rollback; found {sentinel_rows} sentinel rows",
    );

    cleanup_with_outbox(&pool, account_id, player_id, entity_id).await;
}

/// Cross-handler concurrency: a vendor purchase and a move on the same
/// container must serialize through the per-(player, container) advisory
/// lock. Without it, both `reserve_free_inventory_slots` (purchase's
/// slot picker) and the move's swap path could see the same target slot
/// state, race, and either:
///   - fail with a unique-slot index violation on the loser, or
///   - silently land the purchase on a slot the move was about to vacate,
///     producing an inconsistent end state.
///
/// Setup: player has item A at (1, 0) and 5000 naquadah. The cheapest
/// seeded cash-priced vendor entry without item prerequisites is
/// resolved at runtime — this avoids pinning the test to a specific
/// vendor template, store_index, or design id, so seed-data renumbers
/// follow automatically. Spawn a move A→(1, 7) and a concurrent
/// purchase of that resolved entry against the same player and
/// container.
///
/// End-state invariants (lock-order independent):
/// - A ends at (1, 7) with stack 1.
/// - Exactly two rows for this player in container 1 (the moved A + the
///   purchased item).
/// - All `slot_id` values in container 1 for this player are distinct
///   (the unique-slot index would have rejected the test mid-flight if
///   they weren't, but pin it explicitly so a future "drop the index"
///   refactor doesn't silently undo this guarantee).
/// - No row anywhere holds `slot_id = -1` (the swap sentinel must not
///   leak past commit).
/// - Naquadah dropped by exactly the resolved entry's price.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn vendor_purchase_and_concurrent_move_serialize_on_container_lock() {
    use tokio::sync::Barrier;

    use super::super::super::vendor::handle_purchase_vendor_items;

    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 800;
    let player_id = TEST_BASE + 801;
    let entity_id: u32 = 0x7000_0240;
    cleanup_with_outbox(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    // Resolve the cheapest seeded cash-priced buy entry that has no
    // item prerequisites BEFORE setting the player's naquadah, so the
    // funded amount tracks whatever the seed actually charges. ORDER BY
    // naquadah ASC picks the actual cheapest; the NOT EXISTS guard
    // excludes entries with item-prereq rows in
    // `resources.item_list_prices` since the test's player has only
    // its starter inventory and can't satisfy them.
    //
    // The store_index calculation MUST match handle_purchase_vendor_items'
    // load_vendor_purchase_lines (purchase_helpers.rs:108-114), which
    // computes ROW_NUMBER over the *unfiltered* buy list. Filtering
    // before the window function (e.g. excluding item-prereq rows
    // inside the WHERE) would re-rank surviving rows and the test
    // would address the wrong entry. Mirror the handler's structure
    // exactly: window first, filter outside.
    let (vendor_template_id, store_index, expected_price): (i32, i32, i32) = sqlx::query_as(
        "WITH ordered AS ( \
            SELECT et.template_id, \
                   (ROW_NUMBER() OVER (PARTITION BY et.template_id ORDER BY ili.item_id) - 1)::INT \
                       AS store_index, \
                   ili.item_id AS list_item_id, \
                   ili.naquadah \
            FROM resources.entity_templates et \
            JOIN resources.item_list_items ili ON ili.item_list_id = et.buy_item_list \
            WHERE et.buy_item_list IS NOT NULL \
         ) \
         SELECT template_id, store_index, naquadah \
         FROM ordered \
         WHERE naquadah > 0 \
           AND NOT EXISTS ( \
               SELECT 1 FROM resources.item_list_prices ilp \
               WHERE ilp.item_id = ordered.list_item_id \
           ) \
         ORDER BY naquadah ASC, template_id, store_index LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("resolve a positive-price vendor purchase entry without item prerequisites");

    // Fund the player based on the resolved price + a buffer so the
    // test stays seed-agnostic — a future seed bump on the cheapest
    // entry is just a higher price, not a brittle <= 5000 assumption.
    // The buffer keeps the post-purchase balance positive enough that
    // the assertion's intent ("balance dropped by exactly the price")
    // can't be satisfied by an accidental zero-out.
    let starting_naquadah: i32 = expected_price + 1_000;
    sqlx::query("UPDATE sgw_player SET naquadah = $1 WHERE player_id = $2")
        .bind(starting_naquadah)
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("top up naquadah");

    let types = pick_main_bag_type_ids(&pool, 1).await;
    let item_a = insert_item(&pool, player_id, types[0], 1, 0, 1).await;

    let (socket, e2a, conn) = make_state_for_entity(entity_id);
    let db_pool = Some(Arc::new(pool.clone()));
    let barrier = Arc::new(Barrier::new(2));

    let move_handle = {
        let db_pool = db_pool.clone();
        let socket = socket.clone();
        let conn = conn.clone();
        let e2a = e2a.clone();
        let barrier = barrier.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            handle_move_inventory_item(
                entity_id, player_id, item_a, 1, 7, 1, &db_pool, &None, &socket, &conn, &e2a,
            )
            .await;
        })
    };

    let purchase_handle = {
        let db_pool = db_pool.clone();
        let socket = socket.clone();
        let conn = conn.clone();
        let e2a = e2a.clone();
        let barrier = barrier.clone();
        tokio::spawn(async move {
            barrier.wait().await;
            handle_purchase_vendor_items(
                entity_id,
                player_id,
                /* vendor_entity_id */ 99,
                vendor_template_id,
                vec![(store_index, 1)],
                &db_pool,
                &None,
                &socket,
                &conn,
                &e2a,
            )
            .await;
        })
    };

    let timeout = std::time::Duration::from_secs(5);
    tokio::time::timeout(timeout, async {
        move_handle.await.expect("move task panicked");
        purchase_handle.await.expect("purchase task panicked");
    })
    .await
    .expect("move + purchase tasks deadlocked or hung past 5s");

    // Final state checks — order-independent.
    let item_a_pos: (i32, i32, i32) = sqlx::query_as(
        "SELECT container_id, slot_id, stack_size FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2",
    )
    .bind(player_id)
    .bind(item_a)
    .fetch_one(&pool)
    .await
    .expect("read item A after race");
    assert_eq!(
        item_a_pos,
        (1, 7, 1),
        "item A must end at (1, 7) regardless of which task acquired the lock first",
    );

    let row_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_inventory WHERE character_id = $1 AND container_id = 1",
    )
    .bind(player_id)
    .fetch_one(&pool)
    .await
    .expect("count rows in container 1");
    assert_eq!(
        row_count, 2,
        "expected exactly 2 rows in container 1 (moved A + purchased item); got {row_count}",
    );

    let distinct_slots: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT slot_id) FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = 1",
    )
    .bind(player_id)
    .fetch_one(&pool)
    .await
    .expect("count distinct slots");
    assert_eq!(
        distinct_slots, 2,
        "all slot_id values in container 1 must be distinct; got {distinct_slots} distinct \
         out of {row_count} rows (a duplicate would mean the lock failed)",
    );

    let sentinel_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_inventory WHERE slot_id = -1")
            .fetch_one(&pool)
            .await
            .expect("count sentinel rows");
    assert_eq!(
        sentinel_rows, 0,
        "no row may persist at slot_id=-1 after the race; found {sentinel_rows}",
    );

    let final_naquadah: i32 =
        sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .fetch_one(&pool)
            .await
            .expect("read final naquadah");
    assert_eq!(
        final_naquadah,
        starting_naquadah - expected_price,
        "naquadah must drop by exactly the purchase price; \
         expected {} got {final_naquadah}",
        starting_naquadah - expected_price,
    );

    cleanup_with_outbox(&pool, account_id, player_id, entity_id).await;
}
