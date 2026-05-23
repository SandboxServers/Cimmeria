//! Live-DB integration tests for handle_buyback_vendor_items.
//!
//! Skip cleanly when DATABASE_URL is unset; against the bundled local
//! Postgres they exercise the full-buyback / partial-buyback / insufficient-
//! cash / item-not-in-buyback paths.

use super::*;
use crate::test_support::require_db_or_skip;
use crate::test_support::TestTransport;

/// Sentinel base for buyback tests. Distinct from prior live-DB
/// sentinels (sell at +0x800, paid_repair at +0x700, ...).
const TEST_BASE: i32 = 0x7000_0900;

/// vendor_template_id is read by the function but only used in
/// trace logs and the post-commit refresh — buyback rows are
/// keyed off `flags > 0` in INV_BUYBACK regardless of which
/// vendor opened the store. We pass the seeded id (25) for parity
/// with the rest of the test suite.
const SEEDED_VENDOR_TEMPLATE_ID: i32 = 25;
/// Type allowed in container 1 (verified via container_sets) so
/// the buyback move into INV_MAIN doesn't trip item_allows_container.
const BUYBACK_TYPE_ID: i32 = 21;
/// Per-unit buyback price stored in the `flags` column on the
/// INV_BUYBACK row by the sell flow.
const BUYBACK_UNIT_PRICE: i32 = 1_000;

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

async fn insert_account_and_player(pool: &PgPool, account_id: i32, player_id: i32, naquadah: i32) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, $2, '')",
    )
    .bind(account_id)
    .bind(format!("buyback-test-{account_id}"))
    .execute(pool)
    .await
    .expect("insert account");

    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, $4)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("test-{player_id}"))
    .bind(naquadah)
    .execute(pool)
    .await
    .expect("insert player");
}

/// Insert a row at `(container_id, slot_id)` with the given
/// stack_size and `flags` value. For INV_BUYBACK rows, `flags`
/// stores the per-unit buyback price.
async fn insert_buyback_row(
    pool: &PgPool,
    player_id: i32,
    type_id: i32,
    container_id: i32,
    slot_id: i32,
    stack_size: i32,
    flags: i32,
) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges, flags) \
         VALUES ($1, $2, $3, $4, $5, false, 100, 0, $6) \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(stack_size)
    .bind(slot_id)
    .bind(container_id)
    .bind(flags)
    .fetch_one(pool)
    .await
    .expect("insert buyback row")
}

async fn rows_for_item(pool: &PgPool, player_id: i32, item_id: i32) -> Vec<(i32, i32, i32, i32)> {
    sqlx::query_as::<_, (i32, i32, i32, i32)>(
        "SELECT container_id, slot_id, stack_size, flags FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2 \
         ORDER BY container_id",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_all(pool)
    .await
    .expect("rows_for_item query")
}

async fn naquadah_of(pool: &PgPool, player_id: i32) -> i32 {
    sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn make_state(
    entity_id: u32,
) -> (
    Arc<dyn Transport>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let fake_addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(entity_id, fake_addr);
        m
    }));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    (transport, entity_to_addr, connected)
}

/// Full-stack buyback clears the `flags` price on the moved row
/// (the item is back in main inventory and is no longer pending
/// a buyback refund) and debits the player by `unit_price *
/// stack_size`.
#[tokio::test]
async fn full_buyback_clears_flags_and_moves_to_main() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE;
    let player_id = TEST_BASE + 1;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id, 5_000).await;
    let item = insert_buyback_row(
        &pool,
        player_id,
        BUYBACK_TYPE_ID,
        INV_BUYBACK,
        0,
        1,
        BUYBACK_UNIT_PRICE,
    )
    .await;

    let (transport, e2a, conn) = make_state(0x7000_0901);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_buyback_vendor_items(
        0x7000_0901,
        player_id,
        99,
        SEEDED_VENDOR_TEMPLATE_ID,
        vec![(item, 1)],
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    let rows = rows_for_item(&pool, player_id, item).await;
    assert_eq!(rows.len(), 1, "exactly one row remains for the item");
    let (container, _slot, stack, flags) = rows[0];
    assert_eq!(container, INV_MAIN, "item must move from buyback to main");
    assert_eq!(stack, 1);
    assert_eq!(
        flags, 0,
        "flags must clear to 0 — item is no longer pending a buyback refund"
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        5_000 - BUYBACK_UNIT_PRICE,
        "balance must drop by exactly unit_price * 1",
    );

    cleanup(&pool, account_id, player_id).await;
}

/// Partial buyback inserts the moved portion as a new INV_MAIN row
/// with flags = 0, and decrements the original INV_BUYBACK row's
/// stack_size. Critically, the remainder's `flags` (the buyback
/// price) must NOT be touched — the player should still be able to
/// buy back the rest at the same price.
#[tokio::test]
async fn partial_buyback_preserves_price_on_remainder() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 100;
    let player_id = TEST_BASE + 101;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id, 5_000).await;
    // Buyback row holds 5 units at price 1000 each.
    let item = insert_buyback_row(
        &pool,
        player_id,
        BUYBACK_TYPE_ID,
        INV_BUYBACK,
        0,
        5,
        BUYBACK_UNIT_PRICE,
    )
    .await;

    let (transport, e2a, conn) = make_state(0x7000_0902);
    let db_pool = Some(Arc::new(pool.clone()));

    // Buy back 2 of 5.
    handle_buyback_vendor_items(
        0x7000_0902,
        player_id,
        99,
        SEEDED_VENDOR_TEMPLATE_ID,
        vec![(item, 2)],
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    // The original item_id row stays in INV_BUYBACK at stack=3,
    // flags=1000 (price preserved). The partial buyback creates a
    // NEW row in INV_MAIN — different item_id — with stack=2 and
    // flags=0. So query by character + container, not item_id.
    let buyback_rows: Vec<(i32, i32, i32)> = sqlx::query_as(
        "SELECT item_id, stack_size, flags FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2",
    )
    .bind(player_id)
    .bind(INV_BUYBACK)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        buyback_rows.len(),
        1,
        "exactly one buyback row remains (the decremented original)"
    );
    let (b_item, b_stack, b_flags) = buyback_rows[0];
    assert_eq!(
        b_item, item,
        "the buyback row's item_id must be the original"
    );
    assert_eq!(
        b_stack, 3,
        "stack must drop by exactly the bought-back quantity"
    );
    assert_eq!(
        b_flags, BUYBACK_UNIT_PRICE,
        "remainder must keep its unit_price flag — partial buyback shouldn't reset the refund math",
    );

    let main_rows: Vec<(i32, i32, i32)> = sqlx::query_as(
        "SELECT item_id, stack_size, flags FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2",
    )
    .bind(player_id)
    .bind(INV_MAIN)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(main_rows.len(), 1, "exactly one new main row");
    let (m_item, m_stack, m_flags) = main_rows[0];
    assert_ne!(
        m_item, item,
        "partial buyback creates a NEW item_id row in main"
    );
    assert_eq!(
        m_stack, 2,
        "main row's stack must equal the bought-back quantity"
    );
    assert_eq!(m_flags, 0, "main row's flags must be cleared");

    assert_eq!(
        naquadah_of(&pool, player_id).await,
        5_000 - 2 * BUYBACK_UNIT_PRICE,
        "balance must drop by unit_price * quantity",
    );

    cleanup(&pool, account_id, player_id).await;
}

/// Insufficient naquadah rolls back the entire transaction. Item
/// stays in INV_BUYBACK with original stack + flags; balance
/// unchanged. Pre-fix bugs in this stack would have committed the
/// inventory move and let the cash UPDATE fail separately.
#[tokio::test]
async fn buyback_rejected_when_player_cannot_afford() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 200;
    let player_id = TEST_BASE + 201;
    cleanup(&pool, account_id, player_id).await;
    // Player has 100 naquadah but the item costs 1000 to buy back.
    insert_account_and_player(&pool, account_id, player_id, 100).await;
    let item = insert_buyback_row(
        &pool,
        player_id,
        BUYBACK_TYPE_ID,
        INV_BUYBACK,
        0,
        1,
        BUYBACK_UNIT_PRICE,
    )
    .await;

    let (transport, e2a, conn) = make_state(0x7000_0903);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_buyback_vendor_items(
        0x7000_0903,
        player_id,
        99,
        SEEDED_VENDOR_TEMPLATE_ID,
        vec![(item, 1)],
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    let rows = rows_for_item(&pool, player_id, item).await;
    assert_eq!(rows.len(), 1);
    let (container, _, stack, flags) = rows[0];
    assert_eq!(
        container, INV_BUYBACK,
        "item must stay in buyback — purchase was refused",
    );
    assert_eq!(stack, 1);
    assert_eq!(
        flags, BUYBACK_UNIT_PRICE,
        "flags must stay at the original price — no partial mutation",
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        100,
        "balance must not change when buyback is refused",
    );

    cleanup(&pool, account_id, player_id).await;
}

/// An item_id that's not in INV_BUYBACK (e.g., already in main, or
/// in a different container, or doesn't exist) is silently rejected
/// via the rows_by_id miss — no DB changes.
#[tokio::test]
async fn buyback_rejected_for_item_not_in_buyback_container() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 300;
    let player_id = TEST_BASE + 301;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id, 5_000).await;
    // Item is in INV_MAIN, not INV_BUYBACK. Setting flags > 0 even
    // though it shouldn't matter — the WHERE filter on container_id
    // is what we're really testing.
    let item = insert_buyback_row(
        &pool,
        player_id,
        BUYBACK_TYPE_ID,
        INV_MAIN,
        0,
        1,
        BUYBACK_UNIT_PRICE,
    )
    .await;

    let (transport, e2a, conn) = make_state(0x7000_0904);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_buyback_vendor_items(
        0x7000_0904,
        player_id,
        99,
        SEEDED_VENDOR_TEMPLATE_ID,
        vec![(item, 1)],
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    let rows = rows_for_item(&pool, player_id, item).await;
    assert_eq!(rows.len(), 1);
    let (container, _, stack, flags) = rows[0];
    assert_eq!(
        container, INV_MAIN,
        "item must stay in its original container"
    );
    assert_eq!(stack, 1);
    assert_eq!(flags, BUYBACK_UNIT_PRICE);
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        5_000,
        "balance must not change — buyback was refused",
    );

    cleanup(&pool, account_id, player_id).await;
}
