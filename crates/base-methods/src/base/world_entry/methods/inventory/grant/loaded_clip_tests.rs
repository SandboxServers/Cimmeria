//! Live-DB guards for Class Start v6 L4/L5 (OD-CS03): a granted firearm
//! starts loaded.
//!
//! `GrantItem` (content chains, mission rewards, loot pickup and GM give all
//! send it) writes `sgw_inventory.ammo = clip_size` for a design with a clip,
//! and the equip epilogue tells the cell the same number in
//! `UpdateBandolierItem`. A zero-clip weapon (Serpent Staff 2797, Serpent
//! Ribbon Device 4565) keeps today's `ammo = charges` and an empty cell slot.
//!
//! Sentinels: `0x7000_C800..=0x7000_C802` (firearms) and
//! `0x7000_C810..=0x7000_C812` (zero-clip), as (account, player, entity).

use tokio::sync::mpsc;

use super::fall_through_tests::{cleanup, insert_account_and_player, state};
use super::*;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::require_db_or_skip;

/// `(account, player, entity)` per test, so the two never share rows.
const FIREARM_IDS: (i32, i32, u32) = (0x7000_C800, 0x7000_C801, 0x7000_C802);
const ZERO_CLIP_IDS: (i32, i32, u32) = (0x7000_C810, 0x7000_C811, 0x7000_C812);

/// The bandolier (equipment container 3), where the starter firearms go.
const BANDOLIER: i32 = 3;

/// `(design, seeded clip_size)`: Standard Pistol, High Capacity SMG and
/// Standard LMG, the class-start firearms.
const FIREARMS: [(i32, i32); 3] = [(55, 15), (21, 30), (3260, 250)];

/// Zero-clip weapons whose grant must not change (L5).
const ZERO_CLIP: [i32; 2] = [2797, 4565];

/// The seeded `(clip_size, charges)` of `item_id`.
async fn design(pool: &PgPool, item_id: i32) -> (i32, i32) {
    sqlx::query_as("SELECT clip_size, charges FROM resources.items WHERE item_id = $1")
        .bind(item_id)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("item {item_id} must be seeded: {e}"))
}

/// The `ammo` of the player's only row of design `item_id`.
async fn persisted_ammo(pool: &PgPool, player_id: i32, item_id: i32) -> i32 {
    sqlx::query_scalar("SELECT ammo FROM sgw_inventory WHERE character_id = $1 AND type_id = $2")
        .bind(player_id)
        .bind(item_id)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("item {item_id} must have been granted once: {e}"))
}

/// Grant `item_id` into the bandolier and return the `current_ammo` the base
/// sent the cell in `UpdateBandolierItem`.
async fn grant_into_bandolier(pool: &PgPool, ids: (i32, i32, u32), item_id: i32) -> i32 {
    let (_, player_id, entity_id) = ids;
    let (transport, conn, e2a) = state();
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(32);
    handle_grant_item(
        entity_id,
        player_id,
        item_id,
        BANDOLIER,
        1,
        false,
        &db_pool,
        &Some(tx),
        &transport,
        &conn,
        &e2a,
    )
    .await;
    let mut sent = None;
    while let Ok(msg) = rx.try_recv() {
        if let BaseToCellMsg::UpdateBandolierItem { item, .. } = msg {
            assert_eq!(item.item_id, item_id, "the update names the granted design");
            sent = Some(item.current_ammo);
        }
    }
    sent.unwrap_or_else(|| panic!("granting {item_id} into the bandolier must update the cell"))
}

/// Pistol 55, SMG 21 and LMG 3260 granted through `GrantItem` arrive with a
/// full clip, in the row and in the cell's bandolier slot. Reverting the
/// `CASE WHEN ri.clip_size > 0` in the grant INSERT writes 0 (their
/// `charges`); reverting the epilogue sends the cell 0.
#[tokio::test]
async fn live_db_granted_firearms_start_with_a_full_clip() {
    let pool = require_db_or_skip!();
    let ids @ (account_id, player_id, entity_id) = FIREARM_IDS;
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    for (item_id, clip) in FIREARMS {
        assert_eq!(
            design(&pool, item_id).await,
            (clip, 0),
            "seed drift: item {item_id} should be clip {clip}, charges 0",
        );
        let sent = grant_into_bandolier(&pool, ids, item_id).await;
        assert_eq!(
            persisted_ammo(&pool, player_id, item_id).await,
            clip,
            "item {item_id}: a granted firearm is written loaded (ammo = clip_size)",
        );
        assert_eq!(
            sent, clip,
            "item {item_id}: the cell is told the same loaded clip the row holds",
        );
    }

    cleanup(&pool, account_id, player_id, entity_id).await;
}

/// Staff 2797 and ribbon device 4565 have no clip: the grant keeps
/// `ammo = charges` and the cell slot stays at 0 (L5).
#[tokio::test]
async fn live_db_zero_clip_weapons_grant_unchanged() {
    let pool = require_db_or_skip!();
    let ids @ (account_id, player_id, entity_id) = ZERO_CLIP_IDS;
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    for item_id in ZERO_CLIP {
        let (clip, charges) = design(&pool, item_id).await;
        assert_eq!(clip, 0, "seed drift: item {item_id} should have no clip");
        let sent = grant_into_bandolier(&pool, ids, item_id).await;
        assert_eq!(
            persisted_ammo(&pool, player_id, item_id).await,
            charges,
            "item {item_id}: a zero-clip grant keeps ammo = charges",
        );
        assert_eq!(
            sent, 0,
            "item {item_id}: a zero-clip slot reports no rounds"
        );
    }

    cleanup(&pool, account_id, player_id, entity_id).await;
}
