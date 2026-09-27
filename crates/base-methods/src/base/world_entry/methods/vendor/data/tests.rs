//! Live-DB integration tests for vendor data-loader queries.
//!
//! Skip cleanly when DATABASE_URL is unset; against the bundled local
//! Postgres they exercise:
//!
//! - `load_store_buy_items` ordering + cost-list shape against the
//!   seeded buy list (item_list_id=1).
//! - `load_vendor_buyback_prices` `flags > 0` sentinel filter (the
//!   "free buyback" guard documented in mod.rs).
//! - `load_vendor_repair_prices` durability filter (full-durability
//!   items must NOT appear).
//! - `load_vendor_sell_prices` ITEM_FLAG_CAN_BE_SOLD filter +
//!   bound-item exclusion.

use super::*;
use crate::test_support::require_db_or_skip;

/// Sentinel base for vendor-data-loader tests. Distinct from prior
/// live-DB sentinels (outbox 0x000 / grant_cash +0x100 / move +0x200 /
/// grant_item +0x300 / missions +0x400 / mail +0x500 /
/// vendor/repair +0x600 / paid_repair +0x700 / sell +0x800 /
/// buyback +0x900 / purchase +0x0A00 / ammo +0x0B00).
const TEST_BASE: i32 = 0x7000_0C00;

/// Vendor 25's seeded lists. All four are populated:
/// buy_item_list=1, sell_item_list=2, repair_item_list=2,
/// recharge_item_list=2.
const SEEDED_BUY_LIST_ID: i32 = 1;
const SEEDED_SELL_LIST_ID: i32 = 2;
const SEEDED_REPAIR_LIST_ID: i32 = 2;

/// design_id 21 is in the seeded sell list and has
/// ITEM_FLAG_CAN_BE_SOLD set in resources.items.flags. Used as the
/// "this should appear" type for sell_prices and repair_prices.
const SELLABLE_REPAIRABLE_TYPE: i32 = 21;
const SELLABLE_TYPE_LIST_PRICE: i32 = 1_000;

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

async fn insert_account_and_player(pool: &PgPool, account_id: i32, player_id: i32) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, $2, '')",
    )
    .bind(account_id)
    .bind(format!("vendor-data-test-{account_id}"))
    .execute(pool)
    .await
    .expect("insert account");

    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, 0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("test-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
}

/// Insert a sgw_inventory row with explicit slot/durability/flags so
/// each test can pin the exact state the loader queries are reading.
async fn insert_item(
    pool: &PgPool,
    player_id: i32,
    type_id: i32,
    container_id: i32,
    slot_id: i32,
    durability: i32,
    flags: i32,
    bound: bool,
) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges, flags) \
         VALUES ($1, $2, 1, $3, $4, $5, $6, 0, $7) \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(slot_id)
    .bind(container_id)
    .bind(bound)
    .bind(durability)
    .bind(flags)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

/// `load_store_buy_items` returns an indexed-and-ordered StoreItem
/// per row in the buy list. Pinning the seeded list (item_list_id=1)
/// at 3 rows: bug shape this catches is a future schema split that
/// drops the ORDER BY or moves the price join, which would either
/// reorder the items or strip costs.
#[tokio::test]
async fn load_store_buy_items_returns_indexed_rows_with_cost_lists() {
    let pool = require_db_or_skip!();
    let arc_pool = Arc::new(pool.clone());

    let items = load_store_buy_items(&arc_pool, Some(SEEDED_BUY_LIST_ID)).await;

    assert_eq!(
        items.len(),
        3,
        "seeded buy list (id=1) has exactly 3 rows; \
         add/remove a seed entry and update this assertion deliberately",
    );

    // Indexes are 0-based and sequential — clients consume them as a
    // store_index identifier in PurchaseVendorItems requests, so a
    // re-indexing regression silently misroutes purchases.
    assert_eq!(
        items.iter().map(|i| i.index).collect::<Vec<_>>(),
        vec![0, 1, 2],
    );

    // Row order matches `ORDER BY item_id` from the buy-list query.
    // The seeded rows are item_ids 1, 3, 7 with design_ids 5228, 5192,
    // 3241 — pinning the design_ids locks the load order in.
    assert_eq!(
        items.iter().map(|i| i.item_id).collect::<Vec<_>>(),
        vec![5228, 5192, 3241],
    );

    // The middle entry (item_id=3, design 5192) is the only one with
    // an item-cost prereq (design 55 from item_list_prices). The other
    // two have only naquadah cost lists.
    let item_cost_count = items
        .iter()
        .find(|i| i.item_id == 5192)
        .expect("design 5192 must be present")
        .cost_list
        .iter()
        .filter(|c| c.cost_type == COST_ITEM)
        .count();
    assert_eq!(
        item_cost_count, 1,
        "design 5192 must surface its single item prereq in cost_list",
    );
}

/// Option::None on the buy-list id short-circuits — the loader returns
/// an empty Vec without hitting the DB. Important because vendor
/// templates with no buy list are common and the caller relies on
/// the empty-Vec path to render an "empty store" panel.
#[tokio::test]
async fn load_store_buy_items_with_none_returns_empty() {
    let pool = require_db_or_skip!();
    let arc_pool = Arc::new(pool.clone());

    let items = load_store_buy_items(&arc_pool, None).await;
    assert!(
        items.is_empty(),
        "None list_id must short-circuit to an empty Vec",
    );
}

/// `load_vendor_buyback_prices` excludes rows with `flags = 0` —
/// documented as the "free-buyback exploit" guard in mod.rs. Bug
/// shape: a future refactor that uses `flags >= 0` would surface
/// freshly-created rows with default flags as free buyback offerings.
#[tokio::test]
async fn load_vendor_buyback_prices_excludes_zero_flag_rows() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 100;
    let player_id = TEST_BASE + 101;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    // Two INV_BUYBACK rows: one priced at 500, one at 0. Only the
    // 500-priced row may be returned. flags<0 (-1) is also excluded
    // by the same predicate; covered implicitly here since both
    // forbidden states (0 and negative) are treated identically.
    let priced_item = insert_item(
        &pool,
        player_id,
        SELLABLE_REPAIRABLE_TYPE,
        INV_BUYBACK,
        0,
        100,
        500,
        false,
    )
    .await;
    let _zero_flag_item = insert_item(
        &pool,
        player_id,
        SELLABLE_REPAIRABLE_TYPE,
        INV_BUYBACK,
        1,
        100,
        0,
        false,
    )
    .await;

    let arc_pool = Arc::new(pool.clone());
    let prices = load_vendor_buyback_prices(&arc_pool, player_id).await;

    assert_eq!(
        prices.len(),
        1,
        "only the flags>0 row may surface; flags=0 must be excluded",
    );
    assert_eq!(
        prices[0],
        StoreItemCost {
            cost: 500,
            item_id: priced_item,
        },
        "the surviving row must be the 500-priced one — its flags column \
         is the buyback unit price the client uses to refund",
    );

    cleanup(&pool, account_id, player_id).await;
}

/// `load_vendor_repair_prices` excludes items at full durability
/// (`durability < 100` predicate). Bug shape: a regression that flips
/// to `<= 100` would charge players naquadah=0 to "repair" already-
/// pristine items, which the GREATEST(...,1) floor would then bill
/// as 1 — confusing UX and a money sink.
#[tokio::test]
async fn load_vendor_repair_prices_excludes_full_durability() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 200;
    let player_id = TEST_BASE + 201;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    // Two main-bag items of the same repairable type — one at full
    // durability (must be excluded), one at half (must appear with
    // cost = (1000 * 50) / 100 = 500).
    let _full = insert_item(
        &pool,
        player_id,
        SELLABLE_REPAIRABLE_TYPE,
        INV_MAIN,
        0,
        100, // full durability
        0,
        false,
    )
    .await;
    let damaged = insert_item(
        &pool,
        player_id,
        SELLABLE_REPAIRABLE_TYPE,
        INV_MAIN,
        1,
        50, // 50% durability
        0,
        false,
    )
    .await;

    let arc_pool = Arc::new(pool.clone());
    let prices = load_vendor_repair_prices(&arc_pool, player_id, Some(SEEDED_REPAIR_LIST_ID)).await;

    assert_eq!(
        prices.len(),
        1,
        "full-durability item must NOT appear in the repair list",
    );
    assert_eq!(
        prices[0],
        StoreItemCost {
            cost: (SELLABLE_TYPE_LIST_PRICE * 50) / 100,
            item_id: damaged,
        },
        "repair cost must follow GREATEST((unit_price * missing%) / 100, 1)",
    );

    cleanup(&pool, account_id, player_id).await;
}

/// `load_vendor_sell_prices` excludes `bound = true` rows. Bound items
/// (typically quest rewards or soulbound gear) cannot be sold, and a
/// regression that drops the predicate would let players dump bound
/// gear at a vendor — both a UX bug (the sell action would later
/// fail) and an exploit risk if the inverse path stops checking too.
#[tokio::test]
async fn load_vendor_sell_prices_excludes_bound_rows() {
    let pool = require_db_or_skip!();
    let account_id = TEST_BASE + 300;
    let player_id = TEST_BASE + 301;
    cleanup(&pool, account_id, player_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    // Both rows are the same sellable+flagged type; only the one with
    // bound=false may surface.
    let unbound = insert_item(
        &pool,
        player_id,
        SELLABLE_REPAIRABLE_TYPE,
        INV_MAIN,
        0,
        100,
        0,
        false, // bound=false → sellable
    )
    .await;
    let _bound = insert_item(
        &pool,
        player_id,
        SELLABLE_REPAIRABLE_TYPE,
        INV_MAIN,
        1,
        100,
        0,
        true, // bound=true → must be excluded
    )
    .await;

    let arc_pool = Arc::new(pool.clone());
    let prices = load_vendor_sell_prices(&arc_pool, player_id, Some(SEEDED_SELL_LIST_ID)).await;

    assert_eq!(
        prices.len(),
        1,
        "bound=true row must NOT appear in sell prices",
    );
    assert_eq!(
        prices[0],
        StoreItemCost {
            cost: SELLABLE_TYPE_LIST_PRICE,
            item_id: unbound,
        },
        "the unbound row must surface at its list naquadah price",
    );

    cleanup(&pool, account_id, player_id).await;
}
