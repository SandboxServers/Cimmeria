//! Live-DB guards for OD-CS13 on the grant path: every gun is acquired
//! empty. The player reloads once (default reloads are free and unlimited,
//! D-AM02).
//!
//! `CellToBaseMsg::GrantItem` is how content chains, mission rewards and GM
//! `gmGiveItem` grant (`handle_grant_item`) and how a loot pickup grants
//! (`handle_loot_grant`). Both write `sgw_inventory.ammo = charges`, 0 for
//! every gun, and the equip epilogue tells the cell the same 0 in
//! `UpdateBandolierItem`. A change that loads the clip on grant fails here.
//!
//! Sentinels, as (account, player, entity): `0x7000_C800..=0x7000_C802`
//! (plain grant) and `0x7000_C810..=0x7000_C812` (loot grant).

use tokio::sync::mpsc;

use super::fall_through_tests::{cleanup, insert_account_and_player, state};
use super::*;
use crate::cell::messages::{BaseToCellMsg, LootGrantSource};
use crate::test_support::require_db_or_skip;

const GRANT_IDS: (i32, i32, u32) = (0x7000_C800, 0x7000_C801, 0x7000_C802);
const LOOT_IDS: (i32, i32, u32) = (0x7000_C810, 0x7000_C811, 0x7000_C812);

/// The bandolier (equipment container 3), where the starter guns go.
const BANDOLIER: i32 = 3;

/// `(design, seeded clip_size)`: Standard Pistol, High Capacity SMG and
/// Standard LMG, the class-start guns.
const GUNS: [(i32, i32); 3] = [(55, 15), (21, 30), (3260, 250)];

#[derive(Clone, Copy, Debug)]
enum Path {
    /// `handle_grant_item`: content chains, mission rewards, GM give.
    Grant,
    /// `handle_loot_grant`: a loot-window take.
    Loot,
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

/// Grant `item_id` into the bandolier along `path` and return the
/// `(clip_size, current_ammo)` the base sent the cell in
/// `UpdateBandolierItem`.
async fn grant_into_bandolier(
    pool: &PgPool,
    path: Path,
    ids: (i32, i32, u32),
    item_id: i32,
) -> (i32, i32) {
    let (_, player_id, entity_id) = ids;
    let (transport, conn, e2a) = state();
    let db_pool = Some(Arc::new(pool.clone()));
    let (tx, mut rx) = mpsc::channel(32);
    let cell_tx = Some(tx);
    match path {
        Path::Grant => {
            // `notify_gm` on: the GM give is the same call with feedback.
            handle_grant_item(
                entity_id, player_id, item_id, BANDOLIER, 1, true, &db_pool, &cell_tx, &transport,
                &conn, &e2a,
            )
            .await
        }
        Path::Loot => {
            let source = LootGrantSource {
                corpse_id: 0x7000_C81F,
                index: 0,
                corpse_respawn_at: None,
                corpse_template_id: None,
            };
            handle_loot_grant(
                entity_id, player_id, item_id, BANDOLIER, 1, source, &db_pool, &cell_tx,
                &transport, &conn, &e2a,
            )
            .await
        }
    }
    let mut sent = None;
    while let Ok(msg) = rx.try_recv() {
        if let BaseToCellMsg::UpdateBandolierItem { item, .. } = msg {
            assert_eq!(item.item_id, item_id, "the update names the granted design");
            sent = Some((item.clip_size, item.current_ammo));
        }
    }
    sent.unwrap_or_else(|| {
        panic!("{path:?}: granting {item_id} into the bandolier must update the cell")
    })
}

async fn assert_guns_arrive_empty(path: Path, ids: (i32, i32, u32)) {
    let pool = require_db_or_skip!();
    let (account_id, player_id, entity_id) = ids;
    cleanup(&pool, account_id, player_id, entity_id).await;
    insert_account_and_player(&pool, account_id, player_id).await;

    for (item_id, clip) in GUNS {
        let sent = grant_into_bandolier(&pool, path, ids, item_id).await;
        assert_eq!(
            persisted_ammo(&pool, player_id, item_id).await,
            0,
            "{path:?}: gun {item_id} is written with an empty clip (OD-CS13)",
        );
        assert_eq!(
            sent,
            (clip, 0),
            "{path:?}: the cell is told gun {item_id} holds 0 of {clip} rounds",
        );
    }

    cleanup(&pool, account_id, player_id, entity_id).await;
}

/// Pistol 55, SMG 21 and LMG 3260 granted by a content chain, mission reward
/// or GM give arrive with 0 rounds, in the row and on the cell.
#[tokio::test]
async fn live_db_granted_guns_arrive_empty() {
    assert_guns_arrive_empty(Path::Grant, GRANT_IDS).await;
}

/// The same three guns taken from a loot window arrive with 0 rounds.
#[tokio::test]
async fn live_db_looted_guns_arrive_empty() {
    assert_guns_arrive_empty(Path::Loot, LOOT_IDS).await;
}
