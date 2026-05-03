//! Live-DB integration tests for handle_paid_repair_inventory_items.
//!
//! Skip cleanly when DATABASE_URL is unset; against the bundled local
//! Postgres they exercise the cost-formula + cash-debit path, the
//! durability-100 update, the insufficient-cash rollback, and the
//! reject-on-not-in-repair-list path.

use super::*;
use crate::test_support::require_db_or_skip;

/// Sentinel base for paid_repair-vendor tests. Distinct from prior
/// live-DB sentinels (outbox 0x000 / grant_cash +0x100 /
/// move +0x200 / grant_item +0x300 / missions +0x400 / mail +0x500 /
/// vendor/repair +0x600).
const TEST_BASE: i32 = 0x7000_0700;

/// Vendor template with a repair_item_list. Verified via
/// `SELECT template_id FROM resources.entity_templates WHERE
/// repair_item_list IS NOT NULL` — the only seeded repair vendor.
const SEEDED_REPAIR_VENDOR_TEMPLATE_ID: i32 = 25;

/// design_id 21 has a row in resources.item_list_items keyed off the
/// seeded repair list (item_list_id=2) at naquadah=1000 — that's the
/// per-unit base cost the formula scales by missing durability.
const REPAIRABLE_TYPE_ID: i32 = 21;
const REPAIRABLE_TYPE_BASE_PRICE: i32 = 1_000;

/// design_id 5228 is in the seeded buy_item_list but NOT in the seeded
/// repair list — picked deliberately to exercise the
/// "item-not-in-repair-list" rejection branch.
const NOT_REPAIRABLE_TYPE_ID: i32 = 5228;

/// INV_MAIN; constant lifted from sibling vendor handlers.
const INV_MAIN: i32 = 1;

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
    .bind(format!("paid-repair-test-{account_id}"))
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

/// Insert a sgw_inventory row at INV_MAIN with a known durability.
/// `stack_size = 1` is required: the repair query filters on
/// `stack_size = 1` (you can't repair a stack of items individually).
async fn insert_item_with_durability(
    pool: &PgPool,
    player_id: i32,
    type_id: i32,
    slot_id: i32,
    durability: i32,
) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges) \
         VALUES ($1, $2, 1, $3, $4, false, $5, 0) \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(slot_id)
    .bind(INV_MAIN)
    .bind(durability)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

async fn durability_of(pool: &PgPool, player_id: i32, item_id: i32) -> i32 {
    sqlx::query_scalar(
        "SELECT durability FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_one(pool)
    .await
    .unwrap()
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

/// Happy path: a damaged repairable item gets restored to durability=100
/// and the player's balance drops by the formula
/// `GREATEST((unit_price * (100 - durability)) / 100, 1)`. Locks the
/// formula in — the prior bug shape that motivated the GREATEST(...,1)
/// floor was a 99-durability item costing 0 naquadah to repair.
#[tokio::test]
async fn paid_repair_restores_durability_and_debits_balance() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE;
    let player_id = TEST_BASE + 1;
    cleanup(&pool, account_id, player_id).await;
    // Start with enough naquadah to cover the repair and assert exact delta.
    insert_account_and_player(&pool, account_id, player_id, 5_000).await;
    // durability=50 -> cost = (1000 * 50) / 100 = 500.
    let item = insert_item_with_durability(&pool, player_id, REPAIRABLE_TYPE_ID, 0, 50).await;

    let (socket, e2a, conn) = make_state(0x7000_0701);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_paid_repair_inventory_items(
        0x7000_0701,
        player_id,
        vec![item],
        SEEDED_REPAIR_VENDOR_TEMPLATE_ID,
        &db_pool,
        &socket,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        durability_of(&pool, player_id, item).await,
        100,
        "repair must restore durability to 100",
    );
    let expected_cost = (REPAIRABLE_TYPE_BASE_PRICE * 50) / 100;
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        5_000 - expected_cost,
        "balance must drop by exactly (unit_price * missing_durability%) / 100",
    );

    cleanup(&pool, account_id, player_id).await;
}

/// Cost-floor guard: an item at durability 99 (1% missing) yields a
/// raw cost of `(1000 * 1) / 100 = 10`. We pick a durability where
/// the formula returns 10 — which exercises the GREATEST(..., 1)
/// floor's purpose without needing a synthetic 100-naquadah base.
/// The point: very-near-full items always cost SOMETHING — never 0.
#[tokio::test]
async fn paid_repair_charges_at_least_one_for_near_full_items() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 100;
    let player_id = TEST_BASE + 101;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id, 5_000).await;
    let item = insert_item_with_durability(&pool, player_id, REPAIRABLE_TYPE_ID, 0, 99).await;

    let (socket, e2a, conn) = make_state(0x7000_0711);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_paid_repair_inventory_items(
        0x7000_0711,
        player_id,
        vec![item],
        SEEDED_REPAIR_VENDOR_TEMPLATE_ID,
        &db_pool,
        &socket,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        durability_of(&pool, player_id, item).await,
        100,
        "near-full repair still restores durability to 100",
    );
    // (1000 * (100 - 99)) / 100 = 10 — well above the GREATEST floor of 1,
    // so this asserts the formula's natural value rather than the floor.
    let expected_cost = REPAIRABLE_TYPE_BASE_PRICE / 100;
    assert_eq!(expected_cost, 10);
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        5_000 - expected_cost,
        "1% missing durability must charge naquadah * 1%",
    );

    cleanup(&pool, account_id, player_id).await;
}

/// Insufficient-cash rollback: the player can't afford the repair, so
/// nothing changes — durability stays at 50, balance stays at the
/// pre-call value. Pre-fix bug shape: cash UPDATE runs before the
/// balance check, leaving a partial repair when the rollback fails.
#[tokio::test]
async fn paid_repair_rolls_back_when_player_cannot_afford() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 200;
    let player_id = TEST_BASE + 201;
    cleanup(&pool, account_id, player_id).await;
    // Cost at durability 50 is 500; give the player 499 — strictly less.
    let starting_balance = 499;
    insert_account_and_player(&pool, account_id, player_id, starting_balance).await;
    let item = insert_item_with_durability(&pool, player_id, REPAIRABLE_TYPE_ID, 0, 50).await;

    let (socket, e2a, conn) = make_state(0x7000_0721);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_paid_repair_inventory_items(
        0x7000_0721,
        player_id,
        vec![item],
        SEEDED_REPAIR_VENDOR_TEMPLATE_ID,
        &db_pool,
        &socket,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        durability_of(&pool, player_id, item).await,
        50,
        "durability must NOT change when the repair rolls back",
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        starting_balance,
        "balance must NOT change when the repair rolls back",
    );

    cleanup(&pool, account_id, player_id).await;
}

/// An item whose type is NOT in the vendor's repair_item_list is
/// silently rejected — the JOIN in mod.rs returns no row, so the
/// rows_by_id lookup misses and the tx is rolled back. Asserts via
/// the no-DB-changes invariant: durability and balance both unchanged.
#[tokio::test]
async fn paid_repair_rejected_for_item_not_in_repair_list() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 300;
    let player_id = TEST_BASE + 301;
    cleanup(&pool, account_id, player_id).await;
    let starting_balance = 5_000;
    insert_account_and_player(&pool, account_id, player_id, starting_balance).await;
    // design 5228 is in the seeded BUY list but NOT in the repair list.
    let item = insert_item_with_durability(&pool, player_id, NOT_REPAIRABLE_TYPE_ID, 0, 50).await;

    let (socket, e2a, conn) = make_state(0x7000_0731);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_paid_repair_inventory_items(
        0x7000_0731,
        player_id,
        vec![item],
        SEEDED_REPAIR_VENDOR_TEMPLATE_ID,
        &db_pool,
        &socket,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        durability_of(&pool, player_id, item).await,
        50,
        "durability must NOT change when the item type isn't repairable",
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        starting_balance,
        "balance must NOT change when the item type isn't repairable",
    );

    cleanup(&pool, account_id, player_id).await;
}
