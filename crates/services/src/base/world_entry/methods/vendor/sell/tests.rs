//! Live-DB integration tests for handle_sell_vendor_items.
//!
//! Skip cleanly when DATABASE_URL is unset; against the bundled local
//! Postgres they exercise the cash-credit + INV_BUYBACK move path, the
//! reject-on-unsellable-type path, and the i32-overflow rollback guard.

use super::*;
use crate::test_support::require_db_or_skip;
use crate::test_support::TestTransport;

/// Sentinel base for sell-vendor tests. Distinct from prior live-DB
/// sentinels (outbox 0x000 / grant_cash +0x100 / move +0x200 /
/// grant_item +0x300 / missions +0x400 / mail +0x500 / vendor/repair
/// +0x600 / paid_repair +0x700).
const TEST_BASE: i32 = 0x7000_0800;

/// Vendor template seeded in resources.entity_templates with a
/// populated sell_item_list (verified via `SELECT template_id FROM
/// resources.entity_templates WHERE sell_item_list IS NOT NULL`).
const SEEDED_SELL_VENDOR_TEMPLATE_ID: i32 = 25;
/// design_id 21 has an item_list_items row keyed off the seeded
/// vendor's sell list at naquadah=1000 — that's the per-unit sell
/// price the function multiplies by quantity.
const SELLABLE_TYPE_ID: i32 = 21;
const SELLABLE_TYPE_PRICE: i32 = 1_000;

async fn cleanup(pool: &PgPool, entity_id: i32, account_id: i32, player_id: i32) {
    // Delete the outbox rows the test enqueues so a shared live DB
    // doesn't accumulate stale entries from successful runs.
    let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
        .bind(entity_id)
        .execute(pool)
        .await;
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
    .bind(format!("sell-test-{account_id}"))
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
    .bind(format!("test-{player_id}"))
    .bind(naquadah)
    .execute(pool)
    .await
    .expect("insert player");
}

/// Insert a sgw_inventory row at a known (container, slot) and return
/// the auto-generated item_id.
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

/// (container_id, slot_id, stack_size, flags) for an item, or None.
async fn position_of(pool: &PgPool, player_id: i32, item_id: i32) -> Option<(i32, i32, i32, i32)> {
    sqlx::query_as::<_, (i32, i32, i32, i32)>(
        "SELECT container_id, slot_id, stack_size, flags FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(pool)
    .await
    .expect("position_of query")
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

/// Happy path: a full-stack sell credits the player by `unit_price`
/// and moves the item row into INV_BUYBACK (container 16) with its
/// `flags` column overwritten to the unit price. The latter is how
/// the buyback path reconstructs the sell price when the player
/// changes their mind — it's load-bearing, not a side-channel.
#[tokio::test]
async fn full_stack_sell_credits_balance_and_moves_item_to_buyback() {
    let pool = require_db_or_skip!();
    let entity_id: i32 = 0x7000_0801;
    let account_id = TEST_BASE;
    let player_id = TEST_BASE + 1;
    cleanup(&pool, entity_id, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id, 100).await;
    let item = insert_item(&pool, player_id, SELLABLE_TYPE_ID, 1, 0, 1).await;

    let (transport, e2a, conn) = make_state(entity_id as u32);
    let db_pool = Some(Arc::new(pool.clone()));

    // vendor_entity_id=99 here is just a wire-side identifier; the
    // function uses vendor_template_id (25) to look up the sell list.
    handle_sell_vendor_items(
        entity_id as u32,
        player_id,
        99,
        SEEDED_SELL_VENDOR_TEMPLATE_ID,
        vec![(item, 1)],
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    let pos = position_of(&pool, player_id, item).await;
    // Buyback container is 16 (INV_BUYBACK). The flags column on the
    // moved row carries the per-unit sell price for the buyback
    // refund-math.
    assert_eq!(
        pos,
        Some((INV_BUYBACK, 0, 1, SELLABLE_TYPE_PRICE)),
        "sold item must move to INV_BUYBACK with flags = unit_price",
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        100 + SELLABLE_TYPE_PRICE,
        "balance must rise by exactly unit_price * quantity",
    );

    cleanup(&pool, entity_id, account_id, player_id).await;
}

/// An item whose type isn't in the vendor's sell_item_list is
/// silently rejected (the JOIN returns no row → the rows_by_id
/// lookup misses → tx rolls back). Asserts via the no-DB-changes
/// invariant: item still in original slot, balance unchanged.
#[tokio::test]
async fn sell_rejected_for_item_not_in_vendor_sell_list() {
    let pool = require_db_or_skip!();
    let entity_id: i32 = 0x7000_0802;
    let account_id = TEST_BASE + 100;
    let player_id = TEST_BASE + 101;
    cleanup(&pool, entity_id, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id, 100).await;

    // Find a resources.items row that's main-bag allowed AND has the
    // sellable flag bit set, but is NOT in the seeded sell list
    // (item_list_id=2). Constraining on the flag matters: if we picked
    // a type that fails the handler's `(flags & ITEM_FLAG_CAN_BE_SOLD)
    // <> 0` check, the rejection would land for the *wrong* reason and
    // the regression guard would no longer be testing the not-in-list
    // path. The sell list contains design_ids {21, 55, 3437}.
    let unsellable_type: i32 = sqlx::query_scalar(
        "SELECT item_id FROM resources.items \
         WHERE (container_sets IS NULL OR 1 = ANY(container_sets)) \
           AND (flags & $1) <> 0 \
           AND item_id NOT IN (21, 55, 3437) \
         ORDER BY item_id LIMIT 1",
    )
    .bind(ITEM_FLAG_CAN_BE_SOLD)
    .fetch_one(&pool)
    .await
    .expect("pick unsellable_type");
    let item = insert_item(&pool, player_id, unsellable_type, 1, 0, 1).await;

    let (transport, e2a, conn) = make_state(entity_id as u32);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_sell_vendor_items(
        entity_id as u32,
        player_id,
        99,
        SEEDED_SELL_VENDOR_TEMPLATE_ID,
        vec![(item, 1)],
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    let pos = position_of(&pool, player_id, item).await;
    assert_eq!(
        pos.map(|(c, _, _, _)| c),
        Some(1),
        "item must stay in its original container — vendor refused the sale",
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        100,
        "balance must not change when the sale is refused",
    );

    cleanup(&pool, entity_id, account_id, player_id).await;
}

/// Naquadah-overflow guard: if the resulting balance would exceed
/// `i32::MAX`, the function rejects the sale before mutating any
/// state. Pre-fix bugs in the vendor stack would have committed the
/// inventory move and let Postgres error on the cash UPDATE, leaving
/// the item in INV_BUYBACK with no cash credit.
#[tokio::test]
async fn sell_rejected_when_balance_would_overflow_i32() {
    let pool = require_db_or_skip!();
    let entity_id: i32 = 0x7000_0803;
    let account_id = TEST_BASE + 200;
    let player_id = TEST_BASE + 201;
    cleanup(&pool, entity_id, account_id, player_id).await;
    // Set naquadah to i32::MAX; selling for 1000 more would overflow.
    insert_account_and_player(&pool, account_id, player_id, i32::MAX).await;
    let item = insert_item(&pool, player_id, SELLABLE_TYPE_ID, 1, 0, 1).await;

    let (transport, e2a, conn) = make_state(entity_id as u32);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_sell_vendor_items(
        entity_id as u32,
        player_id,
        99,
        SEEDED_SELL_VENDOR_TEMPLATE_ID,
        vec![(item, 1)],
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    // Item must still be in its original container (rollback worked).
    let pos = position_of(&pool, player_id, item).await;
    assert_eq!(
        pos.map(|(c, s, q, _)| (c, s, q)),
        Some((1, 0, 1)),
        "item must stay in original slot when sale rolls back on overflow",
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        i32::MAX,
        "balance must stay at i32::MAX — no partial credit on overflow",
    );

    cleanup(&pool, entity_id, account_id, player_id).await;
}
