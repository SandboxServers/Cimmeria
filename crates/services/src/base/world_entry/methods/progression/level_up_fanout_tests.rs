//! Live-DB guards for the witness fan-out of a level-up.
//!
//! `handle_grant_xp` pushes the XP/level bundle to the levelling player's own
//! client; everyone *watching* that player has to be told separately, through
//! the cell (`BaseToCellMsg::BroadcastToWitnesses`). Live-DB because the
//! level is only computed on the persisted-grant path — with no pool the
//! grant is dropped before any level math runs.

use super::tests::{
    cleanup, insert_test_account, insert_test_player, make_connected_state, TEST_PLAYER_BASE,
};
use super::*;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{require_db_or_skip, TestTransport};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Grant `xp` to a fresh level-1 player and return `(persisted level, every
/// BroadcastToWitnesses the grant put on the base->cell channel)`.
async fn grant_and_collect(
    pool: &sqlx::PgPool,
    offset: i32,
    xp: u64,
) -> (i32, Vec<(u32, u16, Vec<u8>)>) {
    let account_id = TEST_PLAYER_BASE + offset;
    let player_id = TEST_PLAYER_BASE + offset + 1;
    let entity_id: u32 = 9_990_000 + offset as u32;
    let addr: SocketAddr = format!("127.0.0.1:{}", 56_000 + offset).parse().unwrap();
    cleanup(pool, account_id).await;
    insert_test_account(pool, account_id).await;
    insert_test_player(pool, account_id, player_id, 0).await;

    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        make_connected_state(Some(player_id)),
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

    let level: i32 = sqlx::query_scalar("SELECT level FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .unwrap();
    let mut broadcasts = Vec::new();
    while let Ok(msg) = cell_rx.try_recv() {
        if let BaseToCellMsg::BroadcastToWitnesses {
            entity_id,
            method_index,
            args,
        } = msg
        {
            broadcasts.push((entity_id, method_index, args));
        }
    }
    cleanup(pool, account_id).await;
    (level, broadcasts)
}

/// Regression guard: a level-up must be handed to the cell for witness
/// fan-out. Before this, `onLevelUpdate` reached only the levelling player's
/// own client and everyone nearby kept the introduction-time level.
/// Reverting the `broadcast_to_witnesses` call leaves `broadcasts` empty.
#[tokio::test]
async fn level_up_is_fanned_out_to_witnesses_with_the_final_level() {
    let pool = require_db_or_skip!();
    // Far past several boundaries: a multi-level catch-up grant must still
    // produce ONE broadcast, carrying the level that was persisted.
    let (level, broadcasts) = grant_and_collect(&pool, 500, 1_000).await;

    assert!(
        level > 2,
        "test invariant: grant must cross several levels, got {level}"
    );
    assert_eq!(
        broadcasts,
        vec![(
            9_990_500,
            crate::mercury::method_idx::ON_LEVEL_UPDATE,
            level.to_le_bytes().to_vec(),
        )],
        "exactly one onLevelUpdate for the final persisted level"
    );
}

/// No boundary crossed, nothing to tell anyone: an XP grant that does not
/// change the level must not spam every witness with a no-op update.
#[tokio::test]
async fn xp_grant_without_a_level_up_sends_no_witness_broadcast() {
    let pool = require_db_or_skip!();
    let (level, broadcasts) = grant_and_collect(&pool, 510, 50).await;

    assert_eq!(level, 1);
    assert!(
        broadcasts.is_empty(),
        "no level change, no fan-out: {broadcasts:?}"
    );
}
