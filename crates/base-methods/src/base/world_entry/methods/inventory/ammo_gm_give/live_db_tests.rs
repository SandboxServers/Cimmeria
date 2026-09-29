//! Live-DB tests for `.giveammo`'s grant (AM-06): the rounds land as
//! capped stacks, and what does not fit comes back as `remainder`.
//!
//! Sentinel block `0x7000_A600`..`0x7000_A60F`; cleanup by exact id.
//! Bug shape guarded: a grant written as one row of the whole count (what
//! the generic `GrantItem` does), i.e. an over-cap stack, or rounds silently
//! dropped when the bags are full.

use cimmeria_entity::ammo_type::BULLET_HOLLOW_POINT;
use cimmeria_entity::inventory::{INV_CRAFTING, INV_MAIN};
use sqlx::PgPool;

use super::grant_rounds;
use crate::base::resources::{bag_max_slots, bag_min_slot};
use crate::test_support::require_db_or_skip;

const TEST_BASE: i32 = 0x7000_A600;
const HOLLOW_POINT_ITEM: i32 = 9001;
const FILLER_ITEM: i32 = 2893;

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    for (sql, id) in [
        (
            "DELETE FROM sgw_inventory WHERE character_id = $1",
            player_id,
        ),
        ("DELETE FROM sgw_player WHERE player_id = $1", player_id),
        ("DELETE FROM account WHERE account_id = $1", account_id),
    ] {
        let _ = sqlx::query(sql).bind(id).execute(pool).await;
    }
}

async fn fixture(pool: &PgPool, k: i32) -> (i32, i32) {
    let account_id = TEST_BASE + 2 * k;
    let player_id = account_id + 1;
    cleanup(pool, account_id, player_id).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("am06-giveammo-{account_id}"))
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
    .bind(format!("am06-giveammo-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
    (account_id, player_id)
}

async fn put_stack(pool: &PgPool, player_id: i32, type_id: i32, bag: i32, slot: i32, n: i32) {
    sqlx::query(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
         VALUES ($1, $2, $3, $4, $5, false, 100, 0)",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(n)
    .bind(slot)
    .bind(bag)
    .execute(pool)
    .await
    .expect("insert stack");
}

/// Hollow Point stack sizes, bag then slot order.
async fn hp_sizes(pool: &PgPool, player_id: i32) -> Vec<i32> {
    sqlx::query_scalar(
        "SELECT stack_size FROM sgw_inventory WHERE character_id = $1 AND type_id = $2 \
          ORDER BY container_id, slot_id",
    )
    .bind(player_id)
    .bind(HOLLOW_POINT_ITEM)
    .fetch_all(pool)
    .await
    .expect("read HP rows")
}

async fn max_stack(pool: &PgPool) -> i32 {
    sqlx::query_scalar("SELECT max_stack_size FROM resources.items WHERE item_id = $1")
        .bind(HOLLOW_POINT_ITEM)
        .fetch_one(pool)
        .await
        .expect("HP item")
}

/// 700 rounds onto an existing 400-stack with a 500 cap: top up to 500,
/// then 500 + 100 in new stacks. No row is ever over the cap.
#[tokio::test]
async fn live_db_am06_giveammo_tops_up_then_opens_capped_stacks() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 0).await;
    let cap = max_stack(&pool).await;
    assert_eq!(cap, 500, "AM-F seeds a 500-round cap");
    let first = bag_min_slot(INV_MAIN);
    put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_MAIN, first, 400).await;

    let out = grant_rounds(&pool, player_id, BULLET_HOLLOW_POINT, 700)
        .await
        .expect("grant");

    let sizes = hp_sizes(&pool, player_id).await;
    cleanup(&pool, account_id, player_id).await;
    assert_eq!((out.returned, out.remainder), (700, 0));
    assert_eq!((out.stack_before, out.stack_after), (400, 1100));
    assert_eq!(sizes, vec![500, 500, 100], "topped up, then capped stacks");
}

/// Bags with one free slot: 500 rounds land, 200 come back as `remainder`
/// and nothing is written over the cap.
#[tokio::test]
async fn live_db_am06_giveammo_reports_what_does_not_fit() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 1).await;
    for bag in [INV_MAIN, INV_CRAFTING] {
        sqlx::query(
            "INSERT INTO sgw_inventory \
                (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
             SELECT $1, $2, 1, s, $3, false, 100, 0 FROM generate_series($4, $4 + $5 - 1) AS s",
        )
        .bind(player_id)
        .bind(FILLER_ITEM)
        .bind(bag)
        .bind(bag_min_slot(bag))
        .bind(bag_max_slots(bag))
        .execute(&pool)
        .await
        .expect("fill bag");
    }
    sqlx::query(
        "DELETE FROM sgw_inventory WHERE character_id = $1 AND container_id = $2 AND slot_id = $3",
    )
    .bind(player_id)
    .bind(INV_MAIN)
    .bind(bag_min_slot(INV_MAIN))
    .execute(&pool)
    .await
    .expect("free one slot");

    let out = grant_rounds(&pool, player_id, BULLET_HOLLOW_POINT, 700)
        .await
        .expect("grant");

    let sizes = hp_sizes(&pool, player_id).await;
    cleanup(&pool, account_id, player_id).await;
    assert_eq!((out.returned, out.remainder), (500, 200));
    assert_eq!(sizes, vec![500]);
}
