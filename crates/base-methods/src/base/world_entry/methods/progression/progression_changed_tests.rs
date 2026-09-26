//! Live-DB guards for `BaseToCellMsg::ProgressionChanged` (AT-03).
//!
//! The cell's trainer gates read the player's level and training points.
//! Without this message they keep the world-entry values after a level-up,
//! so a node the base would sell stays locked (and unbuyable) until relog.
//! Live-DB because the level is only computed on the persisted-grant path.

use super::tests::{cleanup, insert_test_account, insert_test_player, make_connected_state};
use super::*;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{require_db_or_skip, TestTransport};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Grant `xp` to a fresh level-1 player with 0 points; return the persisted
/// `(level, training_points)` and every `ProgressionChanged` sent to the cell.
async fn grant(pool: &sqlx::PgPool, id: i32, xp: u64) -> ((i32, i32), Vec<(u32, i32, i32)>) {
    let entity_id: u32 = 9_300_320;
    let addr: SocketAddr = "127.0.0.1:65320".parse().unwrap();
    cleanup(pool, id).await;
    insert_test_account(pool, id).await;
    insert_test_player(pool, id, id, 0).await;

    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        make_connected_state(Some(id)),
    )])));
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(8);

    handle_grant_xp(
        entity_id,
        xp,
        None,
        &Some(Arc::new(pool.clone())),
        &transport,
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
    )
    .await;

    let persisted: (i32, i32) =
        sqlx::query_as("SELECT level, training_points FROM sgw_player WHERE player_id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .unwrap();
    let mut changes = Vec::new();
    while let Ok(msg) = cell_rx.try_recv() {
        if let BaseToCellMsg::ProgressionChanged {
            entity_id,
            level,
            training_points,
        } = msg
        {
            changes.push((entity_id, level, training_points));
        }
    }
    cleanup(pool, id).await;
    (persisted, changes)
}

#[tokio::test]
async fn level_up_tells_the_cell_the_persisted_level_and_points() {
    let pool = require_db_or_skip!();
    let ((level, points), changes) = grant(&pool, 0x7030_0320, 1_000).await;
    assert!(
        level > 2,
        "test invariant: several levels crossed, got {level}"
    );
    assert_eq!(
        changes,
        vec![(9_300_320, level, points)],
        "exactly one ProgressionChanged, carrying what was persisted"
    );
}

#[tokio::test]
async fn xp_without_a_level_up_sends_no_progression_change() {
    let pool = require_db_or_skip!();
    let ((level, _), changes) = grant(&pool, 0x7030_0321, 50).await;
    assert_eq!(level, 1);
    assert!(changes.is_empty());
}
