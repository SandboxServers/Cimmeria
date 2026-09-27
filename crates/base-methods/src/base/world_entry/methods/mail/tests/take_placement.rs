//! Live-DB guards for where a taken item lands (SS-M4, owner decision
//! 2026-09-27): by its `container_sets` and the grant rule
//! (`item_placement::first_player_container`), so a crafting component goes
//! to the crafting bag (15). A full destination refuses without spilling
//! into another bag, and an item with no carried bag stays in escrow.
//! Sentinels: accounts, players and entities `0x7300_2500` up, items
//! `0x7300_2580` up (and `0x7300_2600`-`0x7300_2663` for the filled bag).

use std::time::Instant;

use cimmeria_entity::inventory::{INV_CRAFTING, INV_MAIN};

use super::packets::{Client, Received};
use super::*;
use crate::cell::messages::MailOp;
use crate::test_support::LogCapture;

const BASE: i32 = 0x7300_2500;
const ITEMS: i32 = 0x7300_2580;

/// Owner (`base + 1`) and sender (`base + 2`) on account `base`.
async fn two_players(pool: &PgPool, base: i32, tag: &str) -> (i32, i32) {
    cleanup(pool, base).await;
    let (owner, sender) = (base + 1, base + 2);
    insert_players(
        pool,
        base,
        &[
            (owner, &format!("SsmFourPlaceO{tag}")),
            (sender, &format!("SsmFourPlaceS{tag}")),
        ],
    )
    .await;
    (owner, sender)
}

/// A crafting-component type: its first carried bag is 15 (`{17,15}`).
async fn component_type(pool: &PgPool) -> i32 {
    sqlx::query_scalar(
        "SELECT item_id FROM resources.items \
         WHERE container_sets = ARRAY[17, 15] ORDER BY item_id LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .expect("the seed has {17,15} crafting components")
}

async fn take(c: &Client, pool: &PgPool, mail_id: i32) {
    c.op(
        MailOp::TakeItem {
            mail_id,
            container_id: -1,
            slot_id: -1,
        },
        Some(pool),
        Instant::now(),
    )
    .await;
}

/// A crafting component taken out of mail lands in the crafting bag (15),
/// where the grant path puts it, not in the backpack. Fails when the take
/// goes back to the old backpack insert (the row lands in container 1).
#[tokio::test]
async fn take_places_a_crafting_component_in_the_crafting_bag() {
    let pool = require_db_or_skip!();
    let (owner, sender) = two_players(&pool, BASE, "Comp").await;
    let type_id = component_type(&pool).await;
    let mail_id = AttachedMail::from(owner, sender, "SsmFourPlaceSComp")
        .item(ITEMS, type_id, 4)
        .insert(&pool)
        .await;

    let c = Client::new(BASE as u32 + 0x10, owner, 55_280, "SsmFourPlaceOComp");
    take(&c, &pool, mail_id).await;

    assert_eq!(
        inventory_rows(&pool, ITEMS).await,
        vec![(owner, INV_CRAFTING, 0, 4)],
        "in the crafting bag's first free slot"
    );
    assert!(!has_escrow(&pool, mail_id).await);
    c.take();

    cleanup(&pool, BASE).await;
}

/// A full crafting bag refuses the take with its own line and reason
/// (`crafting_bag_full`), keeps the item in escrow, and does not spill it
/// into the empty backpack. Fails when the take goes back to the backpack
/// (the item would land in bag 1).
#[tokio::test]
async fn take_refuses_a_full_crafting_bag_and_keeps_escrow() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let base = BASE + 0x08;
    let (owner, sender) = two_players(&pool, base, "Full").await;
    let type_id = component_type(&pool).await;
    // All 100 crafting-bag slots taken; the backpack is empty.
    sqlx::query(
        "INSERT INTO sgw_inventory \
            (item_id, character_id, type_id, stack_size, container_id, slot_id) \
         SELECT $1 + s, $2, $3, 1, $4, s FROM generate_series(0, 99) AS s",
    )
    .bind(0x7300_2600)
    .bind(owner)
    .bind(type_id)
    .bind(INV_CRAFTING)
    .execute(&pool)
    .await
    .unwrap();
    let item_id = ITEMS + 1;
    let mail_id = AttachedMail::from(owner, sender, "SsmFourPlaceSFull")
        .item(item_id, type_id, 1)
        .insert(&pool)
        .await;

    let c = Client::new(base as u32 + 0x10, owner, 55_281, "SsmFourPlaceOFull");
    take(&c, &pool, mail_id).await;

    assert_eq!(
        c.take(),
        vec![Received::Feedback(
            "Your crafting bag is full. Make room and take the item again; it stays in the \
             message until then."
                .to_string()
        )]
    );
    assert_eq!(inventory_rows(&pool, item_id).await, vec![], "no spill");
    let in_backpack: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_inventory WHERE character_id = $1 AND container_id = $2",
    )
    .bind(owner)
    .bind(INV_MAIN)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(in_backpack, 0);
    assert!(has_escrow(&pool, mail_id).await);
    assert_refused(&capture, "take_item", "crafting_bag_full", mail_id);

    cleanup(&pool, base).await;
}

/// An item whose `container_sets` names no carried bag (a mission-only
/// type, `{2}`) is refused `no_carried_bag` and stays in escrow, never
/// forced into the backpack or a vault.
#[tokio::test]
async fn take_refuses_an_item_with_no_carried_bag() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let base = BASE + 0x10;
    let (owner, sender) = two_players(&pool, base, "None").await;
    let type_id: i32 = sqlx::query_scalar(
        "SELECT item_id FROM resources.items WHERE container_sets = ARRAY[2] \
         ORDER BY item_id LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("the seed has mission-only items");
    let item_id = ITEMS + 2;
    let mail_id = AttachedMail::from(owner, sender, "SsmFourPlaceSNone")
        .item(item_id, type_id, 1)
        .insert(&pool)
        .await;

    let c = Client::new(base as u32 + 0x10, owner, 55_282, "SsmFourPlaceONone");
    take(&c, &pool, mail_id).await;

    assert_eq!(inventory_rows(&pool, item_id).await, vec![]);
    assert!(has_escrow(&pool, mail_id).await);
    assert_refused(&capture, "take_item", "no_carried_bag", mail_id);
    c.take();

    cleanup(&pool, base).await;
}
