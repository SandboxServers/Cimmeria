//! Live-DB integration tests for `handle_grant_cash`/`handle_grant_xp` plus
//! pure burst-shape regression guards for `handle_grant_xp`'s post-grant
//! bundle.
//!
//! Live-DB tests skip cleanly when `DATABASE_URL` is unset; against the
//! bundled local Postgres they pin the WHERE-by-player_id contract that
//! prevents multi-character accounts from leaking grants between siblings,
//! plus (P05, filter `legacy_p05_`) the `gm_feedback_to` caller/target
//! recipient split `.givecash`/`.givexp` depend on.

use super::*;
use crate::test_support::require_db_or_skip;
use crate::test_support::TestTransport;
use cimmeria_mercury::encryption::MercuryEncryption;
use cimmeria_mercury::packet::FRAGMENT_BODY_SIZE;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Sentinel base for player_ids used by live-DB grant_cash tests. Stays
/// well below i32::MAX (sgw_player.player_id is `integer`). Per-test
/// offsets keep concurrent runs from colliding on the same rows.
const TEST_PLAYER_BASE: i32 = 0x7000_0100;

/// Build a fully-connected `ConnectedClientState` (real session key + channel,
/// so `send_to_witness_reliable`/`send_gm_feedback_to_client` actually route
/// a packet through the transport instead of no-op'ing on a missing
/// `connected` entry). `active_player_id` seeds `handle_grant_xp`'s
/// `state.active_player_id` read when this session is the XP recipient; pass
/// `None` for a session that's only acting as the GM-feedback recipient.
fn make_connected_state(active_player_id: Option<i32>) -> ConnectedClientState {
    ConnectedClientState {
        enc: MercuryEncryption::from_session_key([0u8; 32]),
        key: [0u8; 32],
        enc_version: cimmeria_mercury::encryption::EncryptionVersion::V1,
        account_id: 0,
        account_name: None,
        access_level: 0,
        dnd_message: None,
        char_list_sent: false,
        world_entry_sent: false,
        pending_player_entity_id: None,
        player_entity_id: None,
        next_seq: Arc::new(AtomicU32::new(1)),
        next_seq_unreliable: Arc::new(AtomicU32::new(0)),
        pending_acks: Arc::new(Mutex::new(Vec::new())),
        last_recv: Arc::new(Mutex::new(Instant::now())),
        connected_at: Instant::now(),
        account_entity_id: 1,
        next_data_id: 0,
        pending_world_entry: None,
        pending_player_load_data: None,
        pending_map_loaded: None,
        pending_client_ready: None,
        deferred_aoi_msgs: Vec::new(),
        cached_appearance_args: None,
        cached_tint_args: None,
        weapon_holstered: true,
        cancelled: Arc::new(AtomicBool::new(false)),
        cinematic_spam_cancel: Arc::new(AtomicBool::new(false)),
        player_name: None,
        player_level: Some(1),
        player_archetype: None,
        world_name: None,
        player_xp: Some(0),
        player_training_points: Some(0),
        active_player_id,
        pending_destination_ring_id: None,
        channel: Mutex::new(cimmeria_mercury::channel::Channel::new(
            "127.0.0.1:9999".parse().unwrap(),
        )),
    }
}

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
        None, // gm_feedback_to: this is a persistence test, not a GM grant
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
        None, // gm_feedback_to: persistence test, not a GM grant
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

/// GM-sourced grant: `handle_grant_cash` with `gm_feedback_to: Some(_)` must
/// run the definitive-feedback branch (the `send_gm_feedback_to_client` call
/// on the post-UPDATE success path) without panicking, and the naquadah write
/// must still commit. `connected` is empty so the actual feedback packet is
/// skipped gracefully — but the success branch (and its feedback call) is
/// exercised. Reverting the `if let Some(gm_id) = gm_feedback_to {
/// send_gm_feedback_to_client(...) }` block would leave this test green (it
/// asserts DB state, not the wire), but a panic in that branch — or moving
/// the feedback to a failure path — would surface here.
#[tokio::test]
async fn grant_cash_with_gm_feedback_commits_and_does_not_panic() {
    let pool = require_db_or_skip!();
    let account_id = TEST_PLAYER_BASE + 200;
    let player = TEST_PLAYER_BASE + 201;
    cleanup(&pool, account_id).await;
    insert_test_account(&pool, account_id).await;
    insert_test_player(&pool, account_id, player, 10).await;

    let (transport, entity_to_addr, connected, entity_id) = make_state().await;
    let db_pool = Some(Arc::new(pool.clone()));

    handle_grant_cash(
        entity_id,
        player,
        40,
        Some(entity_id), // gm_feedback_to: exercise the definitive-feedback success branch
        &db_pool,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    let naq: i32 = sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(player)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(naq, 50, "GM cash grant must commit (10 + 40 = 50)");

    cleanup(&pool, account_id).await;
}

// ── P05: gm_feedback_to caller/target recipient split ──────────────────────
//
// `.givecash`/`.givexp` grant to a SELECTED target while the calling GM
// receives the feedback line — the exact behavior gap this packet's
// `notify_gm: bool` -> `gm_feedback_to: Option<u32>` rename exists to close.
// Both tests below use two distinct fully-connected sessions (target and
// caller, different addresses) so a regression that sends the feedback to
// the target (or the wire push to the caller) shows up as a wrong
// `send_count_to`.

/// The core behavior this packet exists to fix: cash-grant GM feedback must
/// reach the CALLER, not the grant's DB/UI recipient (`entity_id`). The
/// target gets exactly the `onCashChanged` wire push; the caller gets
/// exactly the feedback line; neither crosses over.
#[tokio::test]
async fn legacy_p05_grant_cash_feedback_goes_to_caller_not_target() {
    let pool = require_db_or_skip!();
    let account_id = TEST_PLAYER_BASE + 300;
    let target_player = TEST_PLAYER_BASE + 301;
    cleanup(&pool, account_id).await;
    insert_test_account(&pool, account_id).await;
    insert_test_player(&pool, account_id, target_player, 10).await;

    let transport_typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = transport_typed.clone();
    let target_entity: u32 = 9_999_301;
    let caller_entity: u32 = 9_999_302;
    let target_addr: SocketAddr = "127.0.0.1:55301".parse().unwrap();
    let caller_addr: SocketAddr = "127.0.0.1:55302".parse().unwrap();

    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (target_entity, target_addr),
        (caller_entity, caller_addr),
    ])));
    let connected = Arc::new(Mutex::new(HashMap::from([
        (target_addr, make_connected_state(None)),
        (caller_addr, make_connected_state(None)),
    ])));
    let db_pool = Some(Arc::new(pool.clone()));

    handle_grant_cash(
        target_entity,
        target_player,
        40,
        Some(caller_entity), // gm_feedback_to: the GM caller, distinct from the target
        &db_pool,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    let naq: i32 = sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(target_player)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(naq, 50, "target must be credited (10 + 40 = 50)");

    assert_eq!(
        transport_typed.send_count_to(target_addr),
        1,
        "target must receive exactly the onCashChanged wire push"
    );
    assert_eq!(
        transport_typed.send_count_to(caller_addr),
        1,
        "caller must receive exactly the GM-feedback line, not the target"
    );
    assert_eq!(transport_typed.len(), 2, "no traffic to any other address");

    cleanup(&pool, account_id).await;
}

/// XP counterpart of `legacy_p05_grant_cash_feedback_goes_to_caller_not_target`.
/// `xp_amount` is well under `LEVEL_XP[1]` (100) so no level-up fires — keeps
/// the post-grant bundle a single packet and avoids the level-up Discord/
/// contact-fanout side paths, which aren't this test's concern.
#[tokio::test]
async fn legacy_p05_grant_xp_feedback_goes_to_caller_not_target() {
    let pool = require_db_or_skip!();
    let account_id = TEST_PLAYER_BASE + 400;
    let target_player = TEST_PLAYER_BASE + 401;
    cleanup(&pool, account_id).await;
    insert_test_account(&pool, account_id).await;
    insert_test_player(&pool, account_id, target_player, 0).await;

    let transport_typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = transport_typed.clone();
    let target_entity: u32 = 9_999_401;
    let caller_entity: u32 = 9_999_402;
    let target_addr: SocketAddr = "127.0.0.1:55401".parse().unwrap();
    let caller_addr: SocketAddr = "127.0.0.1:55402".parse().unwrap();

    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (target_entity, target_addr),
        (caller_entity, caller_addr),
    ])));
    let connected = Arc::new(Mutex::new(HashMap::from([
        (target_addr, make_connected_state(Some(target_player))),
        (caller_addr, make_connected_state(None)),
    ])));
    let db_pool = Some(Arc::new(pool.clone()));

    handle_grant_xp(
        target_entity,
        50,                  // well under LEVEL_XP[1] = 100 -- no level-up
        Some(caller_entity), // gm_feedback_to: the GM caller, distinct from the target
        &db_pool,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    let (exp, level, tp): (i32, i32, i32) =
        sqlx::query_as("SELECT exp, level, training_points FROM sgw_player WHERE player_id = $1")
            .bind(target_player)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(exp, 50, "target must be credited 50 xp");
    assert_eq!(level, 1, "no level-up expected below LEVEL_XP[1]");
    assert_eq!(tp, 0, "no training points expected without a level-up");

    assert_eq!(
        transport_typed.send_count_to(target_addr),
        1,
        "target must receive exactly the XP-update bundle (single packet, zero-level grant)"
    );
    assert_eq!(
        transport_typed.send_count_to(caller_addr),
        1,
        "caller must receive exactly the GM-feedback line, not the target"
    );
    assert_eq!(transport_typed.len(), 2, "no traffic to any other address");

    cleanup(&pool, account_id).await;
}

/// `handle_grant_xp` with no DB pool must drop the grant entirely — no wire
/// push to the target, no "definitive" GM feedback, no in-memory mutation —
/// rather than falsely telling the GM the grant succeeded when it will
/// vanish on the target's next reconnect. Mirrors `handle_grant_cash`'s
/// existing no-pool behavior (CodeRabbit caught the two functions diverging
/// on this exact point during P05's review).
#[tokio::test]
async fn legacy_p05_grant_xp_with_no_db_pool_drops_grant_silently() {
    let transport_typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = transport_typed.clone();
    let target_entity: u32 = 9_999_403;
    let caller_entity: u32 = 9_999_404;
    let target_addr: SocketAddr = "127.0.0.1:55403".parse().unwrap();
    let caller_addr: SocketAddr = "127.0.0.1:55404".parse().unwrap();

    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (target_entity, target_addr),
        (caller_entity, caller_addr),
    ])));
    let connected = Arc::new(Mutex::new(HashMap::from([
        (target_addr, make_connected_state(Some(1234))),
        (caller_addr, make_connected_state(None)),
    ])));

    handle_grant_xp(
        target_entity,
        50,
        Some(caller_entity),
        &None, // no DB pool
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    assert_eq!(
        transport_typed.len(),
        0,
        "no DB pool must drop the grant with zero wire traffic — no target push, no GM feedback"
    );
    let state = connected.lock().unwrap();
    assert_eq!(
        state.get(&target_addr).unwrap().player_xp,
        Some(0),
        "in-memory xp must stay at its pre-grant value (0) — the grant must not apply"
    );
}

/// Same shape as above, for the `(Some(pool), None)` branch: a DB pool
/// exists but the target has no `active_player_id` (pre-character-select).
/// Must drop the grant, not apply an unpersisted mutation with a false
/// success message.
#[tokio::test]
async fn legacy_p05_grant_xp_with_no_active_player_id_drops_grant_silently() {
    let pool = require_db_or_skip!();
    let db_pool = Some(Arc::new(pool));

    let transport_typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = transport_typed.clone();
    let target_entity: u32 = 9_999_405;
    let caller_entity: u32 = 9_999_406;
    let target_addr: SocketAddr = "127.0.0.1:55405".parse().unwrap();
    let caller_addr: SocketAddr = "127.0.0.1:55406".parse().unwrap();

    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (target_entity, target_addr),
        (caller_entity, caller_addr),
    ])));
    let connected = Arc::new(Mutex::new(HashMap::from([
        (target_addr, make_connected_state(None)), // no active_player_id
        (caller_addr, make_connected_state(None)),
    ])));

    handle_grant_xp(
        target_entity,
        50,
        Some(caller_entity),
        &db_pool,
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    assert_eq!(
        transport_typed.len(),
        0,
        "missing active_player_id must drop the grant with zero wire traffic"
    );
    let state = connected.lock().unwrap();
    assert_eq!(
        state.get(&target_addr).unwrap().player_xp,
        Some(0),
        "in-memory xp must stay at its pre-grant value (0) when there's no active character to persist to"
    );
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
