//! Live-DB guard for OD-CS13 on the vendor path: a bought gun arrives with
//! 0 rounds, like every other acquisition, and the player reloads once
//! (default reloads are free, D-AM02).
//!
//! Buys the SI 3 9mm Pistol (55) from the Debug Area munitions vendor (buy
//! list 1300). Sentinels: `0x7000_C820` (account, doubling as the entity
//! id) and `0x7000_C821` (player).

use super::tests::{cleanup, insert_account_and_player, make_state};
use super::*;
use crate::test_support::require_db_or_skip;

const ACCOUNT_ID: i32 = 0x7000_C820;
const PLAYER_ID: i32 = 0x7000_C821;
const MUNITIONS_LIST: i32 = 1300;
const PISTOL: i32 = 55;

#[tokio::test]
async fn live_db_a_bought_gun_arrives_empty() {
    let pool = require_db_or_skip!();
    let entity_id = ACCOUNT_ID;
    cleanup(&pool, entity_id, ACCOUNT_ID, PLAYER_ID).await;
    insert_account_and_player(&pool, ACCOUNT_ID, PLAYER_ID, 5_000).await;

    let vendor: i32 = sqlx::query_scalar(
        "SELECT template_id FROM resources.entity_templates WHERE buy_item_list = $1 \
         ORDER BY template_id LIMIT 1",
    )
    .bind(MUNITIONS_LIST)
    .fetch_one(&pool)
    .await
    .expect("seed: a vendor sells buy list 1300");
    // The store lists rows in `item_list_items.item_id` order.
    let store_index: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resources.item_list_items \
         WHERE item_list_id = $1 AND item_id < \
               (SELECT item_id FROM resources.item_list_items \
                WHERE item_list_id = $1 AND design_id = $2)",
    )
    .bind(MUNITIONS_LIST)
    .bind(PISTOL)
    .fetch_one(&pool)
    .await
    .expect("seed: buy list 1300 sells the pistol");
    let clip: i32 = sqlx::query_scalar("SELECT clip_size FROM resources.items WHERE item_id = $1")
        .bind(PISTOL)
        .fetch_one(&pool)
        .await
        .expect("seed: item 55");
    assert!(
        clip > 0,
        "the pistol must be a gun for this guard to mean anything"
    );

    let (transport, e2a, conn) = make_state(entity_id as u32);
    let db_pool = Some(Arc::new(pool.clone()));
    handle_purchase_vendor_items(
        entity_id as u32,
        PLAYER_ID,
        99,
        vendor,
        vec![(store_index as i32, 1)],
        &db_pool,
        &None,
        &transport,
        &conn,
        &e2a,
    )
    .await;

    let ammo: Vec<i32> = sqlx::query_scalar(
        "SELECT ammo FROM sgw_inventory WHERE character_id = $1 AND type_id = $2",
    )
    .bind(PLAYER_ID)
    .bind(PISTOL)
    .fetch_all(&pool)
    .await
    .expect("ammo query");
    assert_eq!(
        ammo,
        vec![0],
        "one pistol bought, with an empty clip (OD-CS13)"
    );

    cleanup(&pool, entity_id, ACCOUNT_ID, PLAYER_ID).await;
}
