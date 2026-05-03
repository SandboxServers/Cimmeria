//! Live-DB integration tests for `handle_grant_cash`.
//!
//! Skip cleanly when `DATABASE_URL` is unset; against the bundled local
//! Postgres they pin the WHERE-by-player_id contract that prevents
//! multi-character accounts from leaking grants between siblings.

use super::*;
use crate::test_support::require_db_or_skip;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;

/// Sentinel base for player_ids used by live-DB grant_cash tests. Stays
/// well below i32::MAX (sgw_player.player_id is `integer`). Per-test
/// offsets keep concurrent runs from colliding on the same rows.
const TEST_PLAYER_BASE: i32 = 0x7000_0100;

/// Cleanup by deleting the account row — sgw_player rows cascade off it
/// via the `ON DELETE CASCADE` on `sgw_player_account_id_fkey`.
async fn cleanup(pool: &sqlx::PgPool, account_id: i32) {
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

async fn insert_test_account(pool: &sqlx::PgPool, account_id: i32) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, $2, '')",
    )
    .bind(account_id)
    .bind(format!("grant-cash-test-{account_id}"))
    .execute(pool)
    .await
    .expect("INSERT test account row");
}

/// Insert a minimal sgw_player row that satisfies all NOT NULL constraints
/// and CHECK constraints (level/alignment/gender/etc. ranges) plus the
/// FKs (account_id, world_location). Only columns relevant to the
/// grant_cash assertions need test-specific values.
async fn insert_test_player(pool: &sqlx::PgPool, account_id: i32, player_id: i32, naquadah: i32) {
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, $4)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("test-{player_id}"))
    .bind(naquadah)
    .execute(pool)
    .await
    .expect("INSERT test sgw_player row");
}

async fn make_state() -> (
    Arc<UdpSocket>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    u32,
) {
    let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.expect("bind UDP"));
    let entity_id: u32 = 9_999_001;
    // entity_to_addr must contain `entity_id` so handle_grant_cash's first
    // lookup succeeds and reaches the DB UPDATE. `connected` stays empty —
    // send_to_witness will skip the wire emit gracefully when the addr
    // doesn't appear there.
    let fake_addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
    let entity_to_addr = Arc::new(Mutex::new({
        let mut m = HashMap::new();
        m.insert(entity_id, fake_addr);
        m
    }));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    (socket, entity_to_addr, connected, entity_id)
}

/// Regression guard: handle_grant_cash MUST scope its UPDATE by player_id,
/// not account_id. With two characters on the same account, granting cash
/// to character A must not credit character B. The pre-fix bug was that
/// the WHERE clause matched on account_id, so a multi-character account
/// would see grants leak to whichever character row sorted first.
#[tokio::test]
async fn credits_only_target_character_when_account_has_multiple() {
    let pool = require_db_or_skip!();
    let account_id = TEST_PLAYER_BASE;
    let player_a = TEST_PLAYER_BASE + 1;
    let player_b = TEST_PLAYER_BASE + 2;
    cleanup(&pool, account_id).await;
    insert_test_account(&pool, account_id).await;

    // Distinct starting naquadah so a regression that mistakenly credits
    // both characters can't end up looking right by coincidence.
    insert_test_player(&pool, account_id, player_a, 100).await;
    insert_test_player(&pool, account_id, player_b, 999).await;

    let (socket, entity_to_addr, connected, entity_id) = make_state().await;
    let db_pool = Some(Arc::new(pool.clone()));

    handle_grant_cash(
        entity_id,
        player_a,
        50,
        &db_pool,
        &socket,
        &connected,
        &entity_to_addr,
    )
    .await;

    let a_naq: i32 = sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(player_a)
        .fetch_one(&pool)
        .await
        .unwrap();
    let b_naq: i32 = sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(player_b)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(a_naq, 150, "target character A: 100 + 50 = 150");
    assert_eq!(
        b_naq, 999,
        "sibling character B (same account_id) must be untouched — \
         a non-999 here means the WHERE clause matched on account_id",
    );

    cleanup(&pool, account_id).await;
}

/// When the player_id doesn't exist, the UPDATE returns no row. The
/// function must not panic, must not INSERT a phantom row, and must
/// leave every other character row alone (asserted via a sentinel
/// sibling). The wire-side `tracing::warn!` is the only signal the
/// function emits on this path; we don't assert against it here.
#[tokio::test]
async fn does_not_credit_when_player_row_missing() {
    let pool = require_db_or_skip!();
    let account_id = TEST_PLAYER_BASE + 100;
    let bystander = TEST_PLAYER_BASE + 101;
    let nonexistent = TEST_PLAYER_BASE + 102;
    cleanup(&pool, account_id).await;
    insert_test_account(&pool, account_id).await;

    insert_test_player(&pool, account_id, bystander, 200).await;

    let (socket, entity_to_addr, connected, entity_id) = make_state().await;
    let db_pool = Some(Arc::new(pool.clone()));

    handle_grant_cash(
        entity_id,
        nonexistent,
        50,
        &db_pool,
        &socket,
        &connected,
        &entity_to_addr,
    )
    .await;

    let bystander_naq: i32 =
        sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
            .bind(bystander)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        bystander_naq, 200,
        "bystander row must be untouched when grant target doesn't exist",
    );

    // Confirm the missing row genuinely wasn't created as a side effect.
    let nonexistent_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_player WHERE player_id = $1")
            .bind(nonexistent)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(nonexistent_count, 0, "missing-row branch must not INSERT");

    cleanup(&pool, account_id).await;
}
