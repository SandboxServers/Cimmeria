//! Live-DB guard that what the debug hub's crafting supplies vendor
//! (template 314, buy list 310) sells is usable once bought: a purchase
//! lands in the crafting bag, where a Racial Paradigm Guide and a Blueprint
//! item are used through the ordinary `useItem` path and the crafting
//! transaction consumes components.
//!
//! Crafting sentinels (`0x7000_Cxxx`): `0x7000_CBA0` (account, doubling as
//! the entity id) and `0x7000_CBA1` (player).

use cimmeria_entity::crafting::CraftingState;
use cimmeria_wire::cell::vault::VaultAccess;

use super::tests::{cleanup, insert_account_and_player, make_state, INV_CRAFTING};
use super::*;
use crate::base::world_entry::methods::inventory::core::handle_use_inventory_item;
use crate::test_support::require_db_or_skip;
use cimmeria_base_crafting::base::crafting::persistence::load_crafting_state;
use cimmeria_base_crafting::base::crafting::telemetry::JobIds;
use cimmeria_base_crafting::base::crafting::transaction::{
    run_craft_transaction, CraftTransaction,
};

const ACCOUNT_ID: i32 = 0x7000_CBA0;
const PLAYER_ID: i32 = 0x7000_CBA1;

/// The crafting supplies vendor.
const SUPPLIES_VENDOR: i32 = 314;

/// Store indices in buy list 310 (rows in `item_list_items.item_id`
/// order, 3101 first).
const STEEL_CORE_INDEX: i32 = 0;
const COMMON_GUIDE_INDEX: i32 = 19;
const BLUEPRINT_ITEM_INDEX: i32 = 23;

const STEEL_CORE: i32 = 5254;
const COMMON_GUIDE: i32 = 7806;
const STEEL_PLATING_BLUEPRINT_ITEM: i32 = 6483;
/// Common is racial paradigm 1.
const COMMON: i32 = 1;

/// `(item_id, type_id, container_id, stack_size)` of every item held.
async fn held(pool: &PgPool) -> Vec<(i32, i32, i32, i32)> {
    sqlx::query_as(
        "SELECT item_id, type_id, container_id, stack_size FROM sgw_inventory \
         WHERE character_id = $1 ORDER BY type_id",
    )
    .bind(PLAYER_ID)
    .fetch_all(pool)
    .await
    .expect("inventory")
}

async fn use_item(pool: &PgPool, item_id: i32) {
    let (transport, e2a, conn) = make_state(ACCOUNT_ID as u32);
    // The crafting use is the crafting plugin's item-use hook (#962 step 5),
    // which core reaches through the player's session: give the fixture's
    // address one, with the plugin installed.
    let addr = e2a.lock().unwrap()[&(ACCOUNT_ID as u32)];
    let mut state = crate::test_support::test_default_connected_client_state();
    state.plugins = cimmeria_base_session::base::plugin::BasePlugins::build(&[
        &cimmeria_base_crafting::CraftingPlugin,
    ])
    .unwrap();
    conn.lock().unwrap().insert(addr, state);
    handle_use_inventory_item(
        ACCOUNT_ID as u32,
        PLAYER_ID,
        item_id,
        0,
        VaultAccess::NO_SESSION,
        &Some(Arc::new(pool.clone())),
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;
}

fn find(rows: &[(i32, i32, i32, i32)], type_id: i32) -> (i32, i32, i32, i32) {
    *rows
        .iter()
        .find(|r| r.1 == type_id)
        .unwrap_or_else(|| panic!("{type_id} was not bought: {rows:?}"))
}

/// Buy a Common guide, the Blueprint item for blueprint 25 and its 13 Steel
/// Cores for 1 naquadah each; all three land in the crafting bag (the first
/// carried bag their `{17,15}` lists), the guide and
/// the Blueprint item are used from there, and the cores are consumed by a
/// crafting transaction for blueprint 25's product.
#[tokio::test]
async fn live_db_crafting_supplies_land_in_the_crafting_bag_and_are_usable() {
    let pool = require_db_or_skip!();
    cleanup(&pool, ACCOUNT_ID, ACCOUNT_ID, PLAYER_ID).await;
    insert_account_and_player(&pool, ACCOUNT_ID, PLAYER_ID, 100).await;
    let (transport, e2a, conn) = make_state(ACCOUNT_ID as u32);
    let db_pool = Some(Arc::new(pool.clone()));

    handle_purchase_vendor_items(
        ACCOUNT_ID as u32,
        PLAYER_ID,
        99,
        SUPPLIES_VENDOR,
        vec![
            (COMMON_GUIDE_INDEX, 1),
            (BLUEPRINT_ITEM_INDEX, 1),
            (STEEL_CORE_INDEX, 13),
        ],
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;
    let bought = held(&pool).await;
    let naquadah: i32 = sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(PLAYER_ID)
        .fetch_one(&pool)
        .await
        .expect("naquadah");
    let before = load_crafting_state(&pool, PLAYER_ID)
        .await
        .expect("state before");

    let guide = find(&bought, COMMON_GUIDE);
    let blueprint_item = find(&bought, STEEL_PLATING_BLUEPRINT_ITEM);
    use_item(&pool, guide.0).await;
    use_item(&pool, blueprint_item.0).await;
    let after = load_crafting_state(&pool, PLAYER_ID)
        .await
        .expect("state after");
    let ids = JobIds {
        job_id: 1,
        verb: "craft",
        account_id: ACCOUNT_ID as u32,
        player_id: PLAYER_ID,
        entity_id: ACCOUNT_ID as u32,
        gm_entity_id: None,
    };
    let crafted = run_craft_transaction(
        &pool,
        &ids,
        &CraftTransaction {
            consume: vec![(STEEL_CORE, 13)],
            grant: vec![(5398, 1)],
            ..CraftTransaction::default()
        },
    )
    .await;
    let left = held(&pool).await;
    cleanup(&pool, ACCOUNT_ID, ACCOUNT_ID, PLAYER_ID).await;

    assert_eq!(naquadah, 100 - 15, "15 items at 1 naquadah each");
    assert_eq!(bought.len(), 3, "{bought:?}");
    assert!(
        bought.iter().all(|r| r.2 == INV_CRAFTING),
        "a purchase lands in the crafting bag: {bought:?}"
    );
    assert_eq!(find(&bought, STEEL_CORE).3, 13);

    let level = |s: &CraftingState| s.racial_paradigm_levels.get(&COMMON).copied();
    assert_eq!(
        level(&after),
        level(&before).map(|l| l + 1),
        "the bought guide raised Common by one"
    );
    assert!(!before.blueprint_ids.contains(&25));
    assert!(
        after.blueprint_ids.contains(&25),
        "the bought Blueprint item taught 25"
    );
    let (applied, _) = crafted.expect("the bought cores are consumable by a craft");
    assert_eq!(applied.consumed_field().split(',').count(), 1);
    assert_eq!(
        left.iter().map(|r| r.1).collect::<Vec<_>>(),
        vec![5398],
        "guide, Blueprint item and cores all used up; the product remains"
    );
}
