//! Live-DB tests for `AmmoReserve` against real `sgw_inventory` rows (AM-F).
//!
//! Each test owns one sentinel account/player pair and cleans up by exact id.
//! Hollow Point (9001, via `ammo_item_types`) is the ammo under test; the
//! Health Slappack (2893) fills slots where a test needs full bags.

use cimmeria_entity::ammo_type::{BULLET_DEFAULT, BULLET_HOLLOW_POINT};
use cimmeria_entity::inventory::{INV_BANK, INV_CRAFTING, INV_MAIN};
use sqlx::PgPool;

use super::*;
use crate::test_support::require_db_or_skip;

/// Sentinel block for these tests: `0x7000_A500`..`0x7000_A50F`.
const TEST_BASE: i32 = 0x7000_A500;
const HOLLOW_POINT_ITEM: i32 = 9001;
const FILLER_ITEM: i32 = 2893;

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

/// A fresh account and player at `TEST_BASE + 2k` / `TEST_BASE + 2k + 1`.
async fn fixture(pool: &PgPool, k: i32) -> (i32, i32) {
    let account_id = TEST_BASE + 2 * k;
    let player_id = account_id + 1;
    cleanup(pool, account_id, player_id).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("ammo-reserve-{account_id}"))
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
    .bind(format!("ammo-reserve-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
    (account_id, player_id)
}

async fn put_stack(
    pool: &PgPool,
    player_id: i32,
    type_id: i32,
    bag: i32,
    slot: i32,
    size: i32,
) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
         VALUES ($1, $2, $3, $4, $5, false, 100, 0) RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(size)
    .bind(slot)
    .bind(bag)
    .fetch_one(pool)
    .await
    .expect("insert stack")
}

/// `(container_id, slot_id, stack_size)` of the player's Hollow Point rows.
async fn hp_rows(pool: &PgPool, player_id: i32) -> Vec<(i32, i32, i32)> {
    sqlx::query_as(
        "SELECT container_id, slot_id, stack_size FROM sgw_inventory \
          WHERE character_id = $1 AND type_id = $2 ORDER BY container_id, slot_id",
    )
    .bind(player_id)
    .bind(HOLLOW_POINT_ITEM)
    .fetch_all(pool)
    .await
    .expect("read HP rows")
}

/// Fill every free slot of `bag` with filler.
async fn fill_bag(pool: &PgPool, player_id: i32, bag: i32) {
    sqlx::query(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
         SELECT $1, $2, 1, s, $3, false, 100, 0 FROM generate_series(0, $4 - 1) AS s \
          WHERE NOT EXISTS (SELECT 1 FROM sgw_inventory \
                            WHERE character_id = $1 AND container_id = $3 AND slot_id = s)",
    )
    .bind(player_id)
    .bind(FILLER_ITEM)
    .bind(bag)
    .bind(bag_max_slots(bag))
    .execute(pool)
    .await
    .expect("fill bag");
}

#[tokio::test]
async fn live_db_draw_spans_two_stacks_and_deletes_the_emptied_one() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 0).await;
    let first = put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_MAIN, 2, 5).await;
    let second = put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_CRAFTING, 0, 20).await;

    let mut tx = pool.begin().await.expect("begin");
    let d = draw(&mut tx, player_id, BULLET_HOLLOW_POINT, 12)
        .await
        .expect("draw");
    tx.commit().await.expect("commit");

    assert_eq!(d.item_id, Some(HOLLOW_POINT_ITEM));
    assert_eq!((d.drawn, d.stack_before, d.stack_after), (12, 25, 13));
    assert_eq!(
        d.changes,
        vec![
            StackChange {
                instance_id: first,
                container_id: INV_MAIN,
                slot_id: 2,
                before: 5,
                after: 0
            },
            StackChange {
                instance_id: second,
                container_id: INV_CRAFTING,
                slot_id: 0,
                before: 20,
                after: 13
            },
        ]
    );
    assert_eq!(hp_rows(&pool, player_id).await, vec![(INV_CRAFTING, 0, 13)]);
    cleanup(&pool, account_id, player_id).await;
}

#[tokio::test]
async fn live_db_draw_never_overdraws_a_short_reserve() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 1).await;
    put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_MAIN, 0, 5).await;
    // The vault is storage, not a reserve: it neither counts nor gives.
    put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_BANK, 0, 50).await;

    let mut tx = pool.begin().await.expect("begin");
    assert_eq!(
        count(&mut tx, player_id, BULLET_HOLLOW_POINT)
            .await
            .expect("count"),
        5
    );
    let d = draw(&mut tx, player_id, BULLET_HOLLOW_POINT, 12)
        .await
        .expect("draw");
    assert_eq!(
        count(&mut tx, player_id, BULLET_HOLLOW_POINT)
            .await
            .expect("count"),
        0
    );
    tx.commit().await.expect("commit");

    assert_eq!((d.drawn, d.stack_before, d.stack_after), (5, 5, 0));
    assert_eq!(hp_rows(&pool, player_id).await, vec![(INV_BANK, 0, 50)]);

    // An empty reserve draws nothing and changes nothing.
    let mut tx = pool.begin().await.expect("begin");
    let d = draw(&mut tx, player_id, BULLET_HOLLOW_POINT, 12)
        .await
        .expect("draw");
    tx.commit().await.expect("commit");
    assert_eq!((d.drawn, d.changes.len()), (0, 0));
    cleanup(&pool, account_id, player_id).await;
}

#[tokio::test]
async fn live_db_default_ammo_has_no_reserve() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 2).await;
    put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_MAIN, 0, 30).await;

    let mut tx = pool.begin().await.expect("begin");
    let d = draw(&mut tx, player_id, BULLET_DEFAULT, 12)
        .await
        .expect("draw");
    let r = return_rounds(&mut tx, player_id, BULLET_DEFAULT, 7)
        .await
        .expect("return");
    tx.commit().await.expect("commit");

    assert_eq!(d, AmmoDraw::default());
    assert_eq!((r.item_id, r.returned, r.remainder), (None, 0, 7));
    assert_eq!(hp_rows(&pool, player_id).await, vec![(INV_MAIN, 0, 30)]);
    cleanup(&pool, account_id, player_id).await;
}

#[tokio::test]
async fn live_db_return_merges_first_then_opens_a_free_slot() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 3).await;
    let cap: i32 =
        sqlx::query_scalar("SELECT max_stack_size FROM resources.items WHERE item_id = $1")
            .bind(HOLLOW_POINT_ITEM)
            .fetch_one(&pool)
            .await
            .expect("cap");
    // Slot 0 holds filler, slot 1 a nearly full stack, so the new stack
    // must land in slot 2, the first free one.
    put_stack(&pool, player_id, FILLER_ITEM, INV_MAIN, 0, 1).await;
    let near_full = put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_MAIN, 1, cap - 5).await;

    let mut tx = pool.begin().await.expect("begin");
    let r = return_rounds(&mut tx, player_id, BULLET_HOLLOW_POINT, 12)
        .await
        .expect("return");
    tx.commit().await.expect("commit");

    assert_eq!((r.returned, r.remainder), (12, 0));
    assert_eq!((r.stack_before, r.stack_after), (cap - 5, cap + 7));
    assert_eq!(r.changes.len(), 2);
    assert_eq!(
        r.changes[0],
        StackChange {
            instance_id: near_full,
            container_id: INV_MAIN,
            slot_id: 1,
            before: cap - 5,
            after: cap
        }
    );
    assert_eq!(
        (
            r.changes[1].container_id,
            r.changes[1].slot_id,
            r.changes[1].before,
            r.changes[1].after
        ),
        (INV_MAIN, 2, 0, 7)
    );
    assert_eq!(
        hp_rows(&pool, player_id).await,
        vec![(INV_MAIN, 1, cap), (INV_MAIN, 2, 7)]
    );
    cleanup(&pool, account_id, player_id).await;
}

#[tokio::test]
async fn live_db_return_keeps_the_remainder_when_the_bags_are_full() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 4).await;
    let cap: i32 =
        sqlx::query_scalar("SELECT max_stack_size FROM resources.items WHERE item_id = $1")
            .bind(HOLLOW_POINT_ITEM)
            .fetch_one(&pool)
            .await
            .expect("cap");
    put_stack(
        &pool,
        player_id,
        HOLLOW_POINT_ITEM,
        INV_CRAFTING,
        0,
        cap - 2,
    )
    .await;
    fill_bag(&pool, player_id, INV_MAIN).await;
    fill_bag(&pool, player_id, INV_CRAFTING).await;

    let mut tx = pool.begin().await.expect("begin");
    let r = return_rounds(&mut tx, player_id, BULLET_HOLLOW_POINT, 12)
        .await
        .expect("return");
    tx.commit().await.expect("commit");

    assert_eq!((r.returned, r.remainder), (2, 10));
    assert_eq!(r.stack_after, cap);
    assert_eq!(
        hp_rows(&pool, player_id).await,
        vec![(INV_CRAFTING, 0, cap)]
    );
    cleanup(&pool, account_id, player_id).await;
}

/// A draw that rolls back leaves the stack as it was.
#[tokio::test]
async fn live_db_draw_rolls_back_with_the_callers_transaction() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 5).await;
    put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_MAIN, 0, 8).await;

    let mut tx = pool.begin().await.expect("begin");
    let d = draw(&mut tx, player_id, BULLET_HOLLOW_POINT, 8)
        .await
        .expect("draw");
    assert_eq!(d.drawn, 8);
    tx.rollback().await.expect("rollback");

    assert_eq!(hp_rows(&pool, player_id).await, vec![(INV_MAIN, 0, 8)]);
    cleanup(&pool, account_id, player_id).await;
}
