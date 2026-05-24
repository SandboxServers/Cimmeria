//! Live-DB integration tests for `handle_grant_cash` plus pure burst-shape
//! regression guards for `handle_grant_xp`'s post-grant bundle.
//!
//! Live-DB tests skip cleanly when `DATABASE_URL` is unset; against the
//! bundled local Postgres they pin the WHERE-by-player_id contract that
//! prevents multi-character accounts from leaking grants between siblings.

use super::*;
use crate::test_support::require_db_or_skip;
use crate::test_support::TestTransport;
use cimmeria_mercury::packet::FRAGMENT_BODY_SIZE;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

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
    Arc<dyn Transport>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    u32,
) {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
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
    (transport, entity_to_addr, connected, entity_id)
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

    let (transport, entity_to_addr, connected, entity_id) = make_state().await;
    let db_pool = Some(Arc::new(pool.clone()));

    handle_grant_cash(
        entity_id,
        player_a,
        50,
        &db_pool,
        &transport,
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

    let (transport, entity_to_addr, connected, entity_id) = make_state().await;
    let db_pool = Some(Arc::new(pool.clone()));

    handle_grant_cash(
        entity_id,
        nonexistent,
        50,
        &db_pool,
        &transport,
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

// ── Burst-shape regression guards for the handle_grant_xp bundle migration ──
//
// Pure assertions against `build_grant_xp_bundle` — no DB, no transport.
// The handler builds via this same helper, so a regression that drifts
// the bundle composition (drops a record, reorders the per-level pair,
// or skips the level-up tail) fails here before the wire desync reaches
// the client.

/// Zero-level (steady-state) grant: only `onExpUpdate` lands in the bundle.
/// Pre-bundle this was 1 packet → 1 packet (no wire saving), but the bundle
/// path still atomically drains pending ACKs onto the same fragment so the
/// reliable-stream behavior matches the multi-level path uniformly.
#[test]
fn grant_xp_no_level_bundle_is_single_message() {
    let bundle = build_grant_xp_bundle(42, 500, 1, 0, &[]);
    assert_eq!(
        bundle.num_messages(),
        1,
        "zero-level grant emits exactly onExpUpdate"
    );
    assert_eq!(
        bundle.estimated_packet_count(),
        1,
        "single message must fit one packet"
    );
}

/// Single-level grant burst: pre-bundle was 5 packets (onExpUpdate +
/// GIVE_XP_FOR_LEVEL + onMaxExpUpdate + onLevelUpdate + onEntityProperty).
/// Post-bundle: 1 packet.
#[test]
fn grant_xp_single_level_burst_bundles_to_single_packet() {
    let bundle = build_grant_xp_bundle(42, 2_500, 7, 8, &[7]);
    assert_eq!(
        bundle.num_messages(),
        5,
        "single-level grant must contain: onExpUpdate + GIVE_XP_FOR_LEVEL + \
         onMaxExpUpdate + onLevelUpdate + onEntityProperty(TRAINING_POINTS)"
    );
    assert_eq!(
        bundle.estimated_packet_count(),
        1,
        "single-level grant bundle must collapse to 1 reliable packet \
         (was 5 pre-bundle)"
    );
}

/// Max-level catch-up burst: simulates a grant that vaults a level-1
/// character to MAX_LEVEL (19 levels gained). Pre-bundle that was
/// `1 + 2*19 + 2 = 41` reliable packets — a single content-engine call
/// would chew up a quarter of the 32-slot reliable TX window before any
/// other state-change traffic. Post-bundle: 1 packet.
///
/// Pin the upper bound at `num_messages == 41` and assert the body still
/// fits one fragment so a regression that bloats per-message wire payloads
/// past the per-fragment cutoff is caught here.
#[test]
fn grant_xp_max_level_burst_bundles_to_single_packet() {
    use cimmeria_game::player::MAX_LEVEL;

    let levels_gained: Vec<u32> = (1..=MAX_LEVEL).collect();
    assert_eq!(
        levels_gained.len() as u32,
        MAX_LEVEL,
        "test invariant: covers every level transition up to MAX_LEVEL"
    );

    let bundle = build_grant_xp_bundle(
        42, // entity_id
        LEVEL_XP[MAX_LEVEL as usize],
        MAX_LEVEL,
        TRAINING_POINTS_PER_LEVEL * MAX_LEVEL,
        &levels_gained,
    );

    let expected = 1 + 2 * levels_gained.len() + 2;
    assert_eq!(
        bundle.num_messages(),
        expected,
        "max-level grant must contain onExpUpdate + 2 per level gained + \
         onLevelUpdate + onEntityProperty(TRAINING_POINTS) = {expected}"
    );
    assert!(
        bundle.body_len() < FRAGMENT_BODY_SIZE,
        "max-level grant body ({} B) must fit one fragment (limit {} B) — \
         a regression here means per-message wire payloads grew, and the \
         bundle's single-packet shape needs a re-audit",
        bundle.body_len(),
        FRAGMENT_BODY_SIZE
    );
    assert_eq!(
        bundle.estimated_packet_count(),
        1,
        "max-level grant bundle must collapse to 1 reliable packet \
         (was {} pre-bundle — would consume a quarter of the 32-slot TX window)",
        expected
    );
}
