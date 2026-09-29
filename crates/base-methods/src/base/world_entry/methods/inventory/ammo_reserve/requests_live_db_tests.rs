//! Live-DB tests for the AM-02 reserve requests (`requests.rs`): the reload
//! draw and the switch return against real `sgw_inventory` rows, including
//! the double-spend guard.
//!
//! Each test owns one sentinel account/player pair and cleans up by exact
//! id. Hollow Point (9001) is the ammo; the weapon is the lowest-id Standard
//! Pistol, loaded to `clip - 12` so criterion 3a's "12 rounds short" holds
//! whatever the pistol's clip size is.

use std::time::{Duration, Instant};

use cimmeria_entity::ammo_type::{BULLET_ARMOR_PIERCING, BULLET_HOLLOW_POINT};
use cimmeria_entity::inventory::{INV_BANDOLIER, INV_CRAFTING, INV_MAIN};
use sqlx::PgPool;

use super::*;
use crate::base::resources::bag_max_slots;
use crate::cell::messages::ReserveRefusal;
use crate::test_support::require_db_or_skip;

/// Sentinel block for these tests: `0x7000_A520`..`0x7000_A53F`.
const TEST_BASE: i32 = 0x7000_A520;
pub(super) const HOLLOW_POINT_ITEM: i32 = 9001;
const FILLER_ITEM: i32 = 2893;
pub(super) const SLOT: i32 = 0;

pub(super) async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
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

pub(super) async fn fixture(pool: &PgPool, k: i32) -> (i32, i32) {
    let account_id = TEST_BASE + 2 * k;
    let player_id = account_id + 1;
    cleanup(pool, account_id, player_id).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("ammo-requests-{account_id}"))
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
    .bind(format!("ammo-requests-{player_id}"))
    .execute(pool)
    .await
    .expect("insert player");
    (account_id, player_id)
}

/// `(item_id, clip_size)` of the lowest-id Standard Pistol.
pub(super) async fn pistol(pool: &PgPool) -> (i32, i32) {
    sqlx::query_as(
        "SELECT item_id, clip_size FROM resources.items \
          WHERE description = 'Standard Pistol' AND clip_size >= 13 \
          ORDER BY item_id LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("a Standard Pistol with a clip of 13+")
}

/// Put the pistol in bandolier slot 0 holding `ammo` rounds of `ammo_type`.
pub(super) async fn put_weapon(pool: &PgPool, player_id: i32, ammo: i32, ammo_type: i32) -> i32 {
    let (type_id, _) = pistol(pool).await;
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, bound, durability, \
             charges, ammo, cur_ammo_type) \
         VALUES ($1, $2, 1, $3, $4, false, 100, 0, $5, $6) RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(SLOT)
    .bind(INV_BANDOLIER)
    .bind(ammo)
    .bind(ammo_type)
    .fetch_one(pool)
    .await
    .expect("insert weapon")
}

pub(super) async fn put_stack(
    pool: &PgPool,
    player_id: i32,
    type_id: i32,
    bag: i32,
    slot: i32,
    size: i32,
) {
    sqlx::query(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, bound, durability, charges) \
         VALUES ($1, $2, $3, $4, $5, false, 100, 0)",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(size)
    .bind(slot)
    .bind(bag)
    .execute(pool)
    .await
    .expect("insert stack");
}

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

/// Rounds of Hollow Point in the reserve bags.
pub(super) async fn hp_total(pool: &PgPool, player_id: i32) -> i64 {
    sqlx::query_scalar(
        "SELECT COALESCE(SUM(stack_size), 0)::bigint FROM sgw_inventory \
          WHERE character_id = $1 AND type_id = $2 AND container_id = ANY($3)",
    )
    .bind(player_id)
    .bind(HOLLOW_POINT_ITEM)
    .bind(&[INV_MAIN, INV_CRAFTING][..])
    .fetch_one(pool)
    .await
    .expect("sum HP")
}

/// `(ammo, cur_ammo_type)` of the weapon row.
async fn weapon(pool: &PgPool, instance_id: i32) -> (i32, i32) {
    sqlx::query_as("SELECT ammo, cur_ammo_type FROM sgw_inventory WHERE item_id = $1")
        .bind(instance_id)
        .fetch_one(pool)
        .await
        .expect("read weapon")
}

/// Criterion 3a: 12 rounds short with a 100-round stack → 12 drawn, the
/// weapon full, 88 left, in one commit.
#[tokio::test]
async fn live_db_reload_draw_fills_the_clip_from_a_big_stack() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 0).await;
    let (_, clip) = pistol(&pool).await;
    let w = put_weapon(&pool, player_id, clip - 12, BULLET_HOLLOW_POINT).await;
    put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_MAIN, 3, 100).await;

    let c = commit_reload_draw(&pool, player_id, SLOT, w, BULLET_HOLLOW_POINT, clip - 12)
        .await
        .expect("db")
        .expect("drawn");

    assert_eq!((c.requested, c.draw.drawn, c.clip_after), (12, 12, clip));
    assert_eq!((c.draw.stack_before, c.draw.stack_after), (100, 88));
    assert_eq!(hp_total(&pool, player_id).await, 88);
    assert_eq!(weapon(&pool, w).await, (clip, BULLET_HOLLOW_POINT));
    cleanup(&pool, account_id, player_id).await;
}

/// Criterion 3b: 12 short with a 5-round stack → 5 drawn, the stack row
/// deleted, the weapon at `clip - 7`.
#[tokio::test]
async fn live_db_reload_draw_short_stack_loads_what_is_there() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 1).await;
    let (_, clip) = pistol(&pool).await;
    let w = put_weapon(&pool, player_id, clip - 12, BULLET_HOLLOW_POINT).await;
    put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_MAIN, 3, 5).await;

    let c = commit_reload_draw(&pool, player_id, SLOT, w, BULLET_HOLLOW_POINT, clip - 12)
        .await
        .expect("db")
        .expect("drawn");

    assert_eq!(
        (c.draw.drawn, c.clip_after, c.draw.stack_after),
        (5, clip - 7, 0)
    );
    assert_eq!(c.draw.changes.len(), 1);
    assert_eq!(c.draw.changes[0].after, 0, "the emptied stack is deleted");
    assert_eq!(hp_total(&pool, player_id).await, 0);
    assert_eq!(weapon(&pool, w).await, (clip - 7, BULLET_HOLLOW_POINT));
    cleanup(&pool, account_id, player_id).await;
}

/// Empty stack: refused, nothing committed, the weapon row untouched.
#[tokio::test]
async fn live_db_reload_draw_empty_stack_is_refused_and_changes_nothing() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 2).await;
    let (_, clip) = pistol(&pool).await;
    let w = put_weapon(&pool, player_id, clip - 12, BULLET_HOLLOW_POINT).await;

    let r = commit_reload_draw(&pool, player_id, SLOT, w, BULLET_HOLLOW_POINT, clip - 12)
        .await
        .expect("db");

    assert_eq!(r, Err(ReserveRefusal::StackEmpty));
    assert_eq!(weapon(&pool, w).await, (clip - 12, BULLET_HOLLOW_POINT));
    cleanup(&pool, account_id, player_id).await;
}

/// A weapon no longer in the slot: refused, the stack untouched.
#[tokio::test]
async fn live_db_reload_draw_for_a_moved_weapon_draws_nothing() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 3).await;
    let (_, clip) = pistol(&pool).await;
    let w = put_weapon(&pool, player_id, clip - 12, BULLET_HOLLOW_POINT).await;
    put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_MAIN, 3, 100).await;

    let r = commit_reload_draw(
        &pool,
        player_id,
        SLOT + 1,
        w,
        BULLET_HOLLOW_POINT,
        clip - 12,
    )
    .await
    .expect("db");

    assert_eq!(r, Err(ReserveRefusal::WeaponChanged));
    assert_eq!(hp_total(&pool, player_id).await, 100);
    cleanup(&pool, account_id, player_id).await;
}

/// The same request delivered twice (the cell's `clip_before` is stale the
/// second time) draws once: the base counts from the weapon row it locks.
#[tokio::test]
async fn live_db_duplicate_reload_draw_does_not_spend_twice() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 4).await;
    let (_, clip) = pistol(&pool).await;
    let w = put_weapon(&pool, player_id, clip - 12, BULLET_HOLLOW_POINT).await;
    put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_MAIN, 3, 100).await;

    for _ in 0..2 {
        commit_reload_draw(&pool, player_id, SLOT, w, BULLET_HOLLOW_POINT, clip - 12)
            .await
            .expect("db")
            .expect("ok");
    }

    assert_eq!(
        hp_total(&pool, player_id).await,
        88,
        "12 drawn once, not 24"
    );
    assert_eq!(weapon(&pool, w).await.0, clip);
    cleanup(&pool, account_id, player_id).await;
}

/// Type 5 (concurrency): two draws for the same clip truly overlap and the
/// stack is spent once. The test holds `SHARE` on `sgw_inventory` (reads
/// and `FOR UPDATE` pass, every stack write blocks) until the first draw is
/// parked at its stack write and the second is parked behind it; then it
/// lets both go. With the weapon row counted under the lock, the second
/// finds a full clip; trusting the cell's `clip_before` instead would draw
/// 12 more and the stack would end at 76.
#[tokio::test]
async fn live_db_concurrent_reload_draws_spend_the_stack_once() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 5).await;
    let (_, clip) = pistol(&pool).await;
    let w = put_weapon(&pool, player_id, clip - 12, BULLET_HOLLOW_POINT).await;
    put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_MAIN, 3, 100).await;

    let mut gate = pool.begin().await.unwrap();
    let gate_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    sqlx::query("LOCK TABLE sgw_inventory IN SHARE MODE")
        .execute(&mut *gate)
        .await
        .unwrap();
    let release = async {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let waiting: i64 = sqlx::query_scalar(
                "WITH held AS (\
                     SELECT pid FROM pg_stat_activity WHERE $1 = ANY(pg_blocking_pids(pid))\
                 ) \
                 SELECT COUNT(*) FROM pg_stat_activity a \
                 WHERE a.pid IN (SELECT pid FROM held) \
                    OR EXISTS (SELECT 1 FROM held h WHERE h.pid = ANY(pg_blocking_pids(a.pid)))",
            )
            .bind(gate_pid)
            .fetch_one(&pool)
            .await
            .unwrap();
            if waiting >= 2 {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "both draws should be parked behind the gate (saw {waiting})"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        gate.commit().await.unwrap();
    };
    let draw_once =
        || commit_reload_draw(&pool, player_id, SLOT, w, BULLET_HOLLOW_POINT, clip - 12);
    let (a, b, ()) = tokio::join!(draw_once(), draw_once(), release);
    let drawn: i32 = [a, b]
        .into_iter()
        .map(|r| r.expect("db").map_or(0, |c| c.draw.drawn))
        .sum();

    assert_eq!(drawn, 12);
    assert_eq!(hp_total(&pool, player_id).await, 88);
    assert_eq!(weapon(&pool, w).await.0, clip);
    cleanup(&pool, account_id, player_id).await;
}

/// Everything fits: the rounds merge into the stack and the weapon row
/// takes the new type with an empty clip.
#[tokio::test]
async fn live_db_switch_return_all_fit_switches_the_weapon() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 6).await;
    let w = put_weapon(&pool, player_id, 20, BULLET_HOLLOW_POINT).await;
    put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_MAIN, 3, 50).await;

    let c = commit_switch_return(
        &pool,
        player_id,
        SLOT,
        w,
        BULLET_HOLLOW_POINT,
        BULLET_ARMOR_PIERCING,
        20,
    )
    .await
    .expect("db")
    .expect("ok");

    assert!(c.switched);
    assert_eq!((c.rounds, c.ret.returned, c.ret.remainder), (20, 20, 0));
    assert_eq!(hp_total(&pool, player_id).await, 70);
    assert_eq!(weapon(&pool, w).await, (0, BULLET_ARMOR_PIERCING));

    // Delivered again, the row is on the new type: nothing moves.
    let again = commit_switch_return(
        &pool,
        player_id,
        SLOT,
        w,
        BULLET_HOLLOW_POINT,
        BULLET_ARMOR_PIERCING,
        20,
    )
    .await
    .expect("db");
    assert_eq!(again, Err(ReserveRefusal::WeaponChanged));
    assert_eq!(hp_total(&pool, player_id).await, 70, "no rounds minted");
    cleanup(&pool, account_id, player_id).await;
}

/// Bags full, one partial stack: 13 of 20 fit (the stack tops up to 500),
/// the 7 left stay in the weapon as Hollow Point and the switch does not
/// happen. Rounds are conserved: 487 + 20 = 500 + 7.
#[tokio::test]
async fn live_db_switch_return_bags_full_keeps_the_remainder_loaded() {
    let pool = require_db_or_skip!();
    let (account_id, player_id) = fixture(&pool, 7).await;
    let w = put_weapon(&pool, player_id, 20, BULLET_HOLLOW_POINT).await;
    put_stack(&pool, player_id, HOLLOW_POINT_ITEM, INV_MAIN, 3, 487).await;
    fill_bag(&pool, player_id, INV_MAIN).await;
    fill_bag(&pool, player_id, INV_CRAFTING).await;

    let c = commit_switch_return(
        &pool,
        player_id,
        SLOT,
        w,
        BULLET_HOLLOW_POINT,
        BULLET_ARMOR_PIERCING,
        20,
    )
    .await
    .expect("db")
    .expect("ok");

    assert!(!c.switched);
    assert_eq!((c.ret.returned, c.ret.remainder), (13, 7));
    assert_eq!(hp_total(&pool, player_id).await, 500);
    assert_eq!(weapon(&pool, w).await, (7, BULLET_HOLLOW_POINT));
    cleanup(&pool, account_id, player_id).await;
}
