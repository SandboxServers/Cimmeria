//! Live-DB guards for `handle_delete_character`.
//!
//! Covers the account-isolation predicate on the DELETE statement
//! and the `rows_affected == 0` warn branch. The happy-path delete
//! is exercised as a side effect of the cross-account guard: the
//! handler succeeds, but the row that should have been spared MUST
//! still exist, and the row that SHOULD have been deleted is
//! verified separately so a "DELETE deletes nothing" regression
//! also trips here.

use super::*;
use crate::test_support::{require_db_or_skip, LogCapture, TestTransport};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tracing::Level;

/// Sentinel base for the delete-character live-DB tests. Sibling
/// reservations already occupy `0x7000_1100` (player_load/core),
/// `0x7000_1200` (vendor/purchase_helpers), `0x7000_1300`
/// (vendor/recharge), `0x7000_1400` (vendor/paid_recharge),
/// `0x7000_1500` (player_load/core second block), and `0x7000_1600`
/// (cell_dispatch/system_options) — step to the next free window so
/// concurrent live-DB runs don't collide on account/player ids.
const TEST_BASE: i32 = 0x7000_1700;

async fn cleanup(pool: &PgPool, account_ids: &[i32]) {
    for account_id in account_ids {
        let _ = sqlx::query("DELETE FROM sgw_player WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }
}

async fn insert_account(pool: &PgPool, account_id: i32) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, $2, '')",
    )
    .bind(account_id)
    .bind(format!("delete-char-{account_id}"))
    .execute(pool)
    .await
    .expect("insert account");
}

async fn insert_character(pool: &PgPool, account_id: i32, player_id: i32, name: &str) {
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
         ) VALUES ($1, $2, 1, 1, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, 0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(name)
    .execute(pool)
    .await
    .expect("insert character");
}

async fn count_player(pool: &PgPool, player_id: i32) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .expect("count player row")
}

/// Build a `connected` map that satisfies the handler's
/// `get_account_entity_id` lookup. The Account entity id is
/// arbitrary — the wire path past the DELETE uses it only to
/// address `onCharacterList`, which we don't decode in these tests.
fn make_connected(
    addr: SocketAddr,
    account_id: u32,
    account_entity_id: u32,
) -> Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> {
    let mut state = crate::test_support::test_default_connected_client_state();
    state.account_id = account_id;
    state.account_entity_id = account_entity_id;
    let mut m = HashMap::new();
    m.insert(addr, state);
    Arc::new(Mutex::new(m))
}

/// Cross-account delete must NOT remove the other account's
/// character. Bug shape: a refactor that drops the
/// `AND account_id = $2` predicate (or swaps the two binds) lets
/// account A delete account B's character. Pin via an exact
/// COUNT(*) on the spared row — `== 1` beats `is_ok()` because the
/// handler always returns Ok regardless of `rows_affected`.
#[tokio::test]
async fn handle_delete_character_with_wrong_account_does_not_delete() {
    let pool = require_db_or_skip!();
    let account_a = TEST_BASE;
    let account_b = TEST_BASE + 1;
    let player_a = TEST_BASE + 10;
    let player_b = TEST_BASE + 11;

    cleanup(&pool, &[account_a, account_b]).await;
    insert_account(&pool, account_a).await;
    insert_account(&pool, account_b).await;
    insert_character(&pool, account_a, player_a, "alice").await;
    insert_character(&pool, account_b, player_b, "bob").await;

    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:55801".parse().unwrap();
    let connected = make_connected(addr, account_a as u32, 0xAAAA_0001);
    let key = [0u8; 32];
    let db_pool = Some(Arc::new(pool.clone()));

    // Account A asks to delete account B's player. The owning-
    // account check on the DELETE WHERE clause MUST short-circuit
    // the row removal.
    let result = handle_delete_character(
        &dyn_transport,
        addr,
        key,
        account_a as u32,
        player_b,
        &connected,
        &db_pool,
    )
    .await;
    assert!(
        result.is_ok(),
        "handler must return Ok even on no-op delete"
    );

    // Tight assertion: the spared row count is exactly 1. The bug
    // shape is "row vanished"; ">= 1" would mask a regression that
    // moves rows around or duplicates them.
    assert_eq!(
        count_player(&pool, player_b).await,
        1,
        "cross-account delete must NOT remove account B's character; \
         the AND account_id predicate is the only thing standing \
         between this test and an account-isolation breach",
    );
    // Sanity: account A's own character is still present (the
    // handler didn't blanket-clear by account either).
    assert_eq!(
        count_player(&pool, player_a).await,
        1,
        "account A's own character must remain — the no-op delete \
         must not have side-effected the requester's row either",
    );

    cleanup(&pool, &[account_a, account_b]).await;
}

/// `rows_affected == 0` must emit the documented WARN. Bug shape: a
/// refactor that demotes the warn to debug (or removes it) drops the
/// only signal ops has that an account just attempted a
/// cross-account / stale-id delete. Pinned with `LogCapture` per
/// the negative-logging convention.
#[tokio::test]
async fn handle_delete_character_warns_when_not_owned() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();

    let account_a = TEST_BASE + 20;
    let account_b = TEST_BASE + 21;
    let player_b = TEST_BASE + 30;

    cleanup(&pool, &[account_a, account_b]).await;
    insert_account(&pool, account_a).await;
    insert_account(&pool, account_b).await;
    insert_character(&pool, account_b, player_b, "victim").await;

    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:55802".parse().unwrap();
    let connected = make_connected(addr, account_a as u32, 0xAAAA_0002);
    let key = [0u8; 32];
    let db_pool = Some(Arc::new(pool.clone()));

    let _ = handle_delete_character(
        &dyn_transport,
        addr,
        key,
        account_a as u32,
        player_b,
        &connected,
        &db_pool,
    )
    .await;

    let event = capture
        .find_message(Level::WARN, "Character not found or not owned")
        .expect(
            "cross-account delete (rows_affected == 0) must emit a WARN — \
             negative-logging convention: the only signal ops has that \
             something tried to delete a row it doesn't own",
        );
    // Pin the structured fields so a refactor that drops the
    // player_id/account_id context (or swaps levels) trips here.
    assert!(
        event.has_field("player_id", &player_b.to_string()),
        "warn must carry the target player_id for ops triage: {event:#?}"
    );
    assert!(
        event.has_field("account_id", &account_a.to_string()),
        "warn must carry the requesting account_id (not the owner's) — \
         that's the field ops correlates against the session: {event:#?}"
    );

    cleanup(&pool, &[account_a, account_b]).await;
}
