//! Live-DB integration tests for handle_purchase_vendor_items.
//!
//! Skip cleanly when DATABASE_URL is unset; against the bundled local
//! Postgres they exercise the cash-debit + inventory insert happy path,
//! the item-prerequisite consumption path, the insufficient-naquadah
//! rollback path, and the not-in-buy-list rejection path.

use super::*;
use crate::test_support::require_db_or_skip;
use crate::test_support::TestTransport;

/// Sentinel base for purchase-vendor tests. Distinct from prior live-DB
/// sentinels (outbox 0x000 / grant_cash +0x100 / move +0x200 /
/// grant_item +0x300 / missions +0x400 / mail +0x500 / vendor/repair
/// +0x600 / paid_repair +0x700 / sell +0x800 / buyback +0x900).
const TEST_BASE: i32 = 0x7000_0A00;

/// Vendor template seeded in resources.entity_templates with a
/// populated buy_item_list. Verified via `SELECT template_id FROM
/// resources.entity_templates WHERE buy_item_list IS NOT NULL` — the
/// only seeded buy-vendor.
pub(super) const SEEDED_BUY_VENDOR_TEMPLATE_ID: i32 = 25;

/// store_index 0 in vendor 25's buy list (item_list_id=1):
/// design_id=5228, quantity=1, naquadah=100, no item costs.
const PURE_CASH_STORE_INDEX: i32 = 0;
const PURE_CASH_DESIGN_ID: i32 = 5228;
const PURE_CASH_PRICE: i32 = 100;

/// store_index 1: design_id=5192, quantity=1, naquadah=0, requires
/// 1× design_id 55 (the only item_list_prices row keyed off this list).
/// This line lets us cover the consume_design_quantity prereq path
/// without paying any cash.
pub(super) const ITEM_COST_STORE_INDEX: i32 = 1;
pub(super) const ITEM_COST_DESIGN_ID: i32 = 5192;
pub(super) const ITEM_COST_PREREQ_DESIGN_ID: i32 = 55;

/// Both designs vendor 25 sells here (5228, 5192) are `{17,15}` crafting
/// components, so a purchase puts them in the crafting bag, the first
/// carried bag they list. A purchase that still granted into the main bag
/// fails every count below.
pub(super) const INV_CRAFTING: i32 = 15;

pub(super) async fn cleanup(pool: &PgPool, entity_id: i32, account_id: i32, player_id: i32) {
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

pub(super) async fn insert_account_and_player(
    pool: &PgPool,
    account_id: i32,
    player_id: i32,
    naquadah: i32,
) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, $2, '')",
    )
    .bind(account_id)
    .bind(format!("purchase-test-{account_id}"))
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
pub(super) async fn insert_item(
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

/// Count rows in a player's container with a given design (`type_id`).
pub(super) async fn count_in_container(
    pool: &PgPool,
    player_id: i32,
    container_id: i32,
    type_id: i32,
) -> i64 {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2 AND type_id = $3",
    )
    .bind(player_id)
    .bind(container_id)
    .bind(type_id)
    .fetch_one(pool)
    .await
    .expect("count_in_container")
}

/// Sum stack_size across all rows of a given design in a container.
pub(super) async fn stack_sum(
    pool: &PgPool,
    player_id: i32,
    container_id: i32,
    type_id: i32,
) -> i64 {
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(stack_size), 0)::BIGINT FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2 AND type_id = $3",
    )
    .bind(player_id)
    .bind(container_id)
    .bind(type_id)
    .fetch_one(pool)
    .await
    .expect("stack_sum")
}

async fn naquadah_of(pool: &PgPool, player_id: i32) -> i32 {
    sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

pub(super) fn make_state(
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

/// Happy path: a pure-cash purchase debits the player's naquadah by
/// the per-line price and inserts one crafting-bag row containing the
/// granted design. Catches a future regression where the cash UPDATE
/// commits but the inventory INSERT silently no-ops (the prior bug
/// shape that motivated the rows_affected == 1 guard in mod.rs).
#[tokio::test]
async fn live_db_pure_cash_purchase_debits_balance_and_grants_inventory_row() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE;
    let player_id = TEST_BASE + 1;
    let entity_id: i32 = 0x7000_0A01;
    cleanup(&pool, entity_id, account_id, player_id).await;
    // Start with a known balance well above the price so we can assert
    // the exact delta rather than just "lower than before".
    insert_account_and_player(&pool, account_id, player_id, 5_000).await;

    let (transport, e2a, conn) = make_state(entity_id as u32);
    let db_pool = Some(Arc::new(pool.clone()));
    let capture = crate::test_support::LogCapture::install();

    handle_purchase_vendor_items(
        entity_id as u32,
        player_id,
        99,
        SEEDED_BUY_VENDOR_TEMPLATE_ID,
        vec![(PURE_CASH_STORE_INDEX, 1)],
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        count_in_container(&pool, player_id, INV_CRAFTING, PURE_CASH_DESIGN_ID).await,
        1,
        "purchase must produce exactly one crafting-bag row of the granted design",
    );
    let event = capture
        .find_message(tracing::Level::INFO, "grant_container_chosen")
        .expect("a purchase logs grant_container_chosen per line");
    assert_eq!(event.target, "inventory");
    for (key, value) in [
        ("account_id", account_id.to_string()),
        ("player_id", player_id.to_string()),
        ("entity_id", entity_id.to_string()),
        ("item_type_id", PURE_CASH_DESIGN_ID.to_string()),
        ("container_sets", "{17,15}".to_string()),
        ("requested_container_id", "1".to_string()),
        ("skipped_storage", "true".to_string()),
        ("container_id", INV_CRAFTING.to_string()),
        ("slot_id", "0".to_string()),
        ("qty_before", "0".to_string()),
        ("qty_after", "1".to_string()),
        ("source", "vendor_purchase".to_string()),
    ] {
        assert_eq!(event.fields.get(key), Some(&value), "field `{key}`");
    }
    assert_eq!(
        stack_sum(&pool, player_id, INV_CRAFTING, PURE_CASH_DESIGN_ID).await,
        1,
        "stack size must equal the line's grant_quantity",
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        5_000 - PURE_CASH_PRICE,
        "balance must drop by exactly per-line cash_cost",
    );
    // The `vendor` transaction row (2026-09-29: vendor rows had no identity).
    let row = capture
        .find_message(tracing::Level::INFO, "vendor: transaction committed")
        .expect("a purchase logs a vendor transaction row");
    assert_eq!(row.target, "vendor");
    for (key, value) in [
        ("action", "buy".to_string()),
        ("player_id", player_id.to_string()),
        ("vendor_entity_id", "99".to_string()),
        (
            "vendor_template_id",
            SEEDED_BUY_VENDOR_TEMPLATE_ID.to_string(),
        ),
        ("item_type_id", PURE_CASH_DESIGN_ID.to_string()),
        ("price", PURE_CASH_PRICE.to_string()),
        ("cash_before", "5000".to_string()),
        ("cash_after", (5_000 - PURE_CASH_PRICE).to_string()),
    ] {
        assert_eq!(row.fields.get(key), Some(&value), "vendor field `{key}`");
    }

    cleanup(&pool, entity_id, account_id, player_id).await;
}

/// Item-prerequisite path: the seeded line at store_index 1 costs no
/// cash but consumes 1× design_id 55. Verifies the prereq stack is
/// debited from INV_MAIN, the granted item lands in the crafting bag, and the
/// player's naquadah is unchanged (cash_cost == 0 short-circuits the
/// cash UPDATE in mod.rs — locking that branch in).
#[tokio::test]
async fn live_db_item_prereq_purchase_consumes_prereq_and_skips_cash_update() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 100;
    let player_id = TEST_BASE + 101;
    let entity_id: i32 = 0x7000_0A11;
    cleanup(&pool, entity_id, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id, 5_000).await;
    // Provide a 3-stack of the prereq so we can assert the post-purchase
    // stack drops by exactly 1, not "missing" or "zero".
    let prereq_item =
        insert_item(&pool, player_id, ITEM_COST_PREREQ_DESIGN_ID, INV_MAIN, 5, 3).await;

    let (transport, e2a, conn) = make_state(entity_id as u32);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_purchase_vendor_items(
        entity_id as u32,
        player_id,
        99,
        SEEDED_BUY_VENDOR_TEMPLATE_ID,
        vec![(ITEM_COST_STORE_INDEX, 1)],
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    let prereq_stack: Option<i32> =
        sqlx::query_scalar("SELECT stack_size FROM sgw_inventory WHERE item_id = $1")
            .bind(prereq_item)
            .fetch_optional(&pool)
            .await
            .expect("prereq stack lookup");
    assert_eq!(
        prereq_stack,
        Some(2),
        "consume_design_quantity must debit exactly 1 from the prereq stack",
    );

    assert_eq!(
        count_in_container(&pool, player_id, INV_CRAFTING, ITEM_COST_DESIGN_ID).await,
        1,
        "a granted crafting component must land in the crafting bag",
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        5_000,
        "naquadah must be unchanged when cash_cost is 0",
    );

    cleanup(&pool, entity_id, account_id, player_id).await;
}

/// Insufficient-cash rollback: a player below the line price gets the
/// purchase rejected atomically — no inventory grant, no cash debit.
/// Pre-fix bug shape: the inventory INSERT runs before the balance
/// check, leaving a free item if the cash UPDATE silently fails.
#[tokio::test]
async fn live_db_purchase_rejected_when_player_cannot_afford() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 200;
    let player_id = TEST_BASE + 201;
    let entity_id: i32 = 0x7000_0A21;
    cleanup(&pool, entity_id, account_id, player_id).await;
    // Less than PURE_CASH_PRICE — purchase must fail.
    insert_account_and_player(&pool, account_id, player_id, PURE_CASH_PRICE - 1).await;

    let (transport, e2a, conn) = make_state(entity_id as u32);
    let db_pool = Some(Arc::new(pool.clone()));
    let capture = crate::test_support::LogCapture::install();

    handle_purchase_vendor_items(
        entity_id as u32,
        player_id,
        99,
        SEEDED_BUY_VENDOR_TEMPLATE_ID,
        vec![(PURE_CASH_STORE_INDEX, 1)],
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        count_in_container(&pool, player_id, INV_CRAFTING, PURE_CASH_DESIGN_ID).await,
        0,
        "no inventory row may be granted when the purchase is rolled back",
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        PURE_CASH_PRICE - 1,
        "balance must not change when the purchase is rolled back",
    );
    let row = capture
        .find_event(
            tracing::Level::INFO,
            "vendor: request refused",
            "insufficient_cash",
        )
        .expect("an unaffordable purchase logs reason=insufficient_cash");
    assert!(
        row.has_field("player_id", &player_id.to_string()),
        "{row:#?}"
    );
    assert!(row.has_field("price", &PURE_CASH_PRICE.to_string()));
    assert!(row.has_field("cash", &(PURE_CASH_PRICE - 1).to_string()));
    assert!(row.has_field("item_type_id", &PURE_CASH_DESIGN_ID.to_string()));

    cleanup(&pool, entity_id, account_id, player_id).await;
}

/// Regression guard for the cleanup helper itself. The pre-fix WHERE
/// clause was `entity_id < 0`, which matched zero rows because the
/// test sentinels are positive — so successful runs leaked outbox
/// rows into the shared live DB. Insert one outbox row at the test
/// sentinel, run cleanup, assert the row is gone. Reverting cleanup
/// to `entity_id < 0` leaves count == 1 and fails this guard.
#[tokio::test]
async fn live_db_cleanup_deletes_outbox_rows_for_test_entity() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 400;
    let player_id = TEST_BASE + 401;
    let entity_id: i32 = 0x7000_0A41;
    cleanup(&pool, entity_id, account_id, player_id).await;

    sqlx::query(
        "INSERT INTO cell_event_outbox (entity_id, event_type, payload) \
         VALUES ($1, 'TestEvent', '{}'::jsonb)",
    )
    .bind(entity_id)
    .execute(&pool)
    .await
    .expect("seed outbox row");

    let pre: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cell_event_outbox WHERE entity_id = $1")
            .bind(entity_id)
            .fetch_one(&pool)
            .await
            .expect("pre-cleanup count");
    assert_eq!(pre, 1, "fixture must have inserted one outbox row");

    cleanup(&pool, entity_id, account_id, player_id).await;

    let post: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM cell_event_outbox WHERE entity_id = $1")
            .bind(entity_id)
            .fetch_one(&pool)
            .await
            .expect("post-cleanup count");
    assert_eq!(
        post, 0,
        "cleanup must delete outbox rows for the test sentinel; \
         reverting the WHERE clause to `entity_id < 0` would leave the \
         row in place and fail this guard",
    );
}

/// A store_index outside the buy list (here: 99, well past the 3 seeded
/// rows) is rejected before any state mutates. Asserts via the
/// no-DB-changes invariant: balance unchanged, no crafting-bag rows of any
/// of the seeded designs.
#[tokio::test]
async fn live_db_purchase_rejected_for_index_not_in_buy_list() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 300;
    let player_id = TEST_BASE + 301;
    let entity_id: i32 = 0x7000_0A31;
    cleanup(&pool, entity_id, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id, 5_000).await;

    let (transport, e2a, conn) = make_state(entity_id as u32);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_purchase_vendor_items(
        entity_id as u32,
        player_id,
        99,
        SEEDED_BUY_VENDOR_TEMPLATE_ID,
        // Buy list only has store_indices 0, 1, 2.
        vec![(99, 1)],
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        count_in_container(&pool, player_id, INV_CRAFTING, PURE_CASH_DESIGN_ID).await,
        0,
        "no item may be granted when the buy-list index is invalid",
    );
    assert_eq!(
        count_in_container(&pool, player_id, INV_CRAFTING, ITEM_COST_DESIGN_ID).await,
        0,
        "no item may be granted when the buy-list index is invalid",
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        5_000,
        "balance must not change when the index is invalid",
    );

    cleanup(&pool, entity_id, account_id, player_id).await;
}
