//! Live-DB integration tests for handle_paid_recharge_inventory_items.
//!
//! Skip cleanly when DATABASE_URL is unset.
//!
//! The seeded `resources.items` has zero rows with `charges > 0`, so a
//! meaningful happy-path test needs dedicated fixture data: a
//! synthetic `resources.items` row with `charges > 0`, plus a matching
//! `resources.item_list_items` row that puts it in vendor 25's
//! recharge list. Both fixture rows are keyed at sentinel `item_id`
//! values far outside the seeded sequences and are inserted
//! idempotently (`ON CONFLICT (item_id) DO NOTHING`). The synthetic
//! rows are NOT deleted by per-test cleanup because `cargo test` runs
//! tests in parallel within a binary, and deleting a shared FK target
//! between another test's insert and handler call would surface as
//! flaky PK conflicts or FK violations.

use super::*;
use crate::test_support::require_db_or_skip;
use crate::test_support::TestTransport;

/// Sentinel base. Steps past the highest live-DB sentinel reserved
/// elsewhere in the crate.
const TEST_BASE: i32 = 0x7000_1400;

/// Vendor template seeded with `recharge_item_list = 2` — same vendor
/// used by paid_repair / vendor data tests.
const SEEDED_RECHARGE_VENDOR_TEMPLATE_ID: i32 = 25;
const SEEDED_RECHARGE_LIST_ID: i32 = 2;

/// Synthetic resources.items.item_id used by these tests. Picked far
/// above the seed sequence's high water mark and outside any other
/// live test sentinel range so it can't collide.
const SYNTH_RECHARGEABLE_TYPE_ID: i32 = 0x7FFF_BBBB;
/// Base charges of the synthetic item — picked so the cost formula's
/// math is round and easy to read.
const SYNTH_BASE_CHARGES: i32 = 100;
/// Per-line naquadah for the synthetic recharge entry. The cost
/// formula is `GREATEST((naquadah * (ri.charges - inv.charges)) /
/// ri.charges, 1)` — picked so that with `inv.charges = 0` the cost
/// equals exactly `naquadah` and the assertions don't have to do
/// arithmetic in their head.
const SYNTH_RECHARGE_NAQUADAH: i32 = 1_000;

/// Sentinel item_id for the `resources.item_list_items` link row.
/// `item_list_items_pkey` is on `item_id` (auto-increment), so picking
/// a fixed sentinel lets the link insert use `ON CONFLICT (item_id)
/// DO NOTHING` and stay idempotent across concurrent tests.
const SYNTH_RECHARGE_LIST_LINK_ID: i32 = 0x7FFF_BBBC;

const INV_MAIN: i32 = 1;

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    // Per-test rows only. The synthetic `resources.items` design and
    // its `item_list_items` link row are intentionally NOT deleted:
    // `cargo test` runs tests in parallel within a binary, and
    // deleting a shared FK target between tests' insert/handler calls
    // causes spurious failures. The sentinel ids sit well outside any
    // production data range, so leaving them in place is safe.
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

/// Insert the synthetic resources.items row and the matching
/// resources.item_list_items entry that puts it in vendor 25's
/// recharge list. Both inserts are idempotent so concurrent tests
/// don't race on the shared sentinel rows.
async fn insert_synthetic_rechargeable_design(pool: &PgPool) {
    sqlx::query(
        "INSERT INTO resources.items (\
            item_id, description, name, quality_id, tech_comp, tier, \
            max_stack_size, charges \
         ) VALUES ($1, '', $2, 'ITEM_QUALITY_Normal', 0, 1, 1, $3) \
         ON CONFLICT (item_id) DO NOTHING",
    )
    .bind(SYNTH_RECHARGEABLE_TYPE_ID)
    .bind(format!("synth-rechargeable-{SYNTH_RECHARGEABLE_TYPE_ID}"))
    .bind(SYNTH_BASE_CHARGES)
    .execute(pool)
    .await
    .expect("insert synthetic rechargeable design");

    sqlx::query(
        "INSERT INTO resources.item_list_items \
            (item_id, item_list_id, design_id, quantity, naquadah) \
         VALUES ($1, $2, $3, 1, $4) \
         ON CONFLICT (item_id) DO NOTHING",
    )
    .bind(SYNTH_RECHARGE_LIST_LINK_ID)
    .bind(SEEDED_RECHARGE_LIST_ID)
    .bind(SYNTH_RECHARGEABLE_TYPE_ID)
    .bind(SYNTH_RECHARGE_NAQUADAH)
    .execute(pool)
    .await
    .expect("insert synthetic recharge list link");
}

async fn insert_account_and_player(pool: &PgPool, account_id: i32, player_id: i32, naquadah: i32) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, $2, '')",
    )
    .bind(account_id)
    .bind(format!("paid-recharge-{account_id}"))
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

async fn insert_inventory_row(
    pool: &PgPool,
    player_id: i32,
    type_id: i32,
    slot_id: i32,
    charges: i32,
) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges) \
         VALUES ($1, $2, 1, $3, $4, false, 100, $5) \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(slot_id)
    .bind(INV_MAIN)
    .bind(charges)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

async fn charges_of(pool: &PgPool, item_id: i32) -> i32 {
    sqlx::query_scalar("SELECT charges FROM sgw_inventory WHERE item_id = $1")
        .bind(item_id)
        .fetch_one(pool)
        .await
        .expect("charges_of")
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

/// Happy path: a fully-depleted item (charges=0) of the synthetic
/// design gets restored to ri.charges, and the player's balance
/// drops by the formula
/// `GREATEST((naquadah * (ri.charges - inv.charges)) / ri.charges, 1)`.
/// With inv.charges=0 and ri.charges=SYNTH_BASE_CHARGES, the cost is
/// exactly SYNTH_RECHARGE_NAQUADAH. Locks the BIGINT-cast cost formula
/// in — the prior bug shape was an i32 overflow on
/// `naquadah * (ri.charges - inv.charges)`.
#[tokio::test]
async fn paid_recharge_restores_charges_and_debits_balance() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE;
    let player_id = TEST_BASE + 1;
    cleanup(&pool, account_id, player_id).await;
    insert_synthetic_rechargeable_design(&pool).await;
    insert_account_and_player(&pool, account_id, player_id, 5_000).await;
    let item = insert_inventory_row(&pool, player_id, SYNTH_RECHARGEABLE_TYPE_ID, 0, 0).await;

    let (transport, e2a, conn) = make_state(0x7000_1401);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_paid_recharge_inventory_items(
        0x7000_1401,
        player_id,
        vec![item],
        SEEDED_RECHARGE_VENDOR_TEMPLATE_ID,
        &db_pool,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        charges_of(&pool, item).await,
        SYNTH_BASE_CHARGES,
        "paid recharge must restore charges to ri.charges",
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        5_000 - SYNTH_RECHARGE_NAQUADAH,
        "balance must drop by exactly the formula's cost",
    );

    cleanup(&pool, account_id, player_id).await;
}

/// Insufficient-cash rollback: the player can't afford the recharge,
/// so charges stay at 0 and the balance is unchanged. Pre-fix bug
/// shape: cash UPDATE runs before the balance check, leaving a
/// partial recharge when the rollback fails.
#[tokio::test]
async fn paid_recharge_rolls_back_when_player_cannot_afford() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 100;
    let player_id = TEST_BASE + 101;
    cleanup(&pool, account_id, player_id).await;
    insert_synthetic_rechargeable_design(&pool).await;
    let starting_balance = SYNTH_RECHARGE_NAQUADAH - 1;
    insert_account_and_player(&pool, account_id, player_id, starting_balance).await;
    let item = insert_inventory_row(&pool, player_id, SYNTH_RECHARGEABLE_TYPE_ID, 0, 0).await;

    let (transport, e2a, conn) = make_state(0x7000_1411);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_paid_recharge_inventory_items(
        0x7000_1411,
        player_id,
        vec![item],
        SEEDED_RECHARGE_VENDOR_TEMPLATE_ID,
        &db_pool,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        charges_of(&pool, item).await,
        0,
        "charges must NOT change when the recharge rolls back",
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        starting_balance,
        "balance must NOT change when the recharge rolls back",
    );

    cleanup(&pool, account_id, player_id).await;
}

/// Vendor without a recharge_item_list returns early with a warn —
/// must NOT silently succeed and must NOT mutate any state. Picking
/// a vendor template_id with no recharge list (we use a sentinel id
/// known to be missing from the seeded `entity_templates`) drives
/// the early-bail branch in `load_vendor_template_lists` /
/// `recharge_item_list = None`.
#[tokio::test]
async fn paid_recharge_no_op_when_vendor_has_no_recharge_list() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 200;
    let player_id = TEST_BASE + 201;
    cleanup(&pool, account_id, player_id).await;
    insert_synthetic_rechargeable_design(&pool).await;
    let starting_balance = 5_000;
    insert_account_and_player(&pool, account_id, player_id, starting_balance).await;
    let item = insert_inventory_row(&pool, player_id, SYNTH_RECHARGEABLE_TYPE_ID, 0, 0).await;

    let (transport, e2a, conn) = make_state(0x7000_1421);
    let db_pool = Some(Arc::new(pool.clone()));

    // Sentinel vendor template id that doesn't exist in
    // entity_templates — `load_vendor_template_lists` returns None
    // and the handler bails before any tx.
    const MISSING_VENDOR_TEMPLATE_ID: i32 = -42;
    handle_paid_recharge_inventory_items(
        0x7000_1421,
        player_id,
        vec![item],
        MISSING_VENDOR_TEMPLATE_ID,
        &db_pool,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    assert_eq!(
        charges_of(&pool, item).await,
        0,
        "charges must NOT change when the vendor has no recharge list",
    );
    assert_eq!(
        naquadah_of(&pool, player_id).await,
        starting_balance,
        "balance must NOT change when the vendor has no recharge list",
    );

    cleanup(&pool, account_id, player_id).await;
}
