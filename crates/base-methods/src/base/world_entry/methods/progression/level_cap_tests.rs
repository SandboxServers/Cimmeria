//! AT-07 guards: the level cap is 50 and each level grants one training
//! point (ability-trees campaign decision D-AT02, PROJECT FINAL v2 values).
//!
//! Two layers:
//! - Pure: `build_grant_xp_bundle` for the three boundary cases, decoded
//!   record by record, so the `onMaxExpUpdate` value the client is shown is
//!   pinned (460,000 after 20→21; the 26,525,000 display sentinel at 50).
//! - Live-DB: `handle_grant_xp` persists the new level and point total. It
//!   is live-DB because the handler drops a grant it cannot persist before
//!   any level math runs.
//!
//! Against the pre-AT-07 code (`MAX_LEVEL = 20`, two points per level) every
//! test here fails: 20→21 is refused, and the point totals are doubled.

use super::tests::{cleanup, insert_test_account, make_connected_state, TEST_PLAYER_BASE};
use super::*;
use crate::test_support::{require_db_or_skip, TestTransport};
use cimmeria_game::player::MAX_LEVEL;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// The level-50 display sentinel from workbook sheet `15_Emulator_Level_1_50`.
const SENTINEL: i32 = 26_525_000;

/// Decode a bundle into `(method_index, args)` records. `finalize` with an
/// identity "encryption" yields `[flags][body][u32 seq]`; the body is a run
/// of entity-method records, direct (`0x80 | idx`) or extended (`0xBD`,
/// `idx = IDBASE_SGW_PLAYER + sub_index`).
fn decode_records(bundle: ChannelBundle) -> Vec<(u16, Vec<u8>)> {
    let (packets, _) = bundle.finalize(0, 0, |plaintext| plaintext.to_vec());
    assert_eq!(packets.len(), 1, "grant bundle must stay one packet");
    let body = &packets[0][1..packets[0].len() - 4];
    let mut out = Vec::new();
    let mut i = 0;
    while i < body.len() {
        let marker = body[i];
        let len = u16::from_le_bytes([body[i + 1], body[i + 2]]) as usize;
        let payload = &body[i + 3..i + 3 + len];
        // payload = [u32 entity_id]([u8 sub_index] if extended)[args]
        let (idx, args) = if marker == 0xBD {
            (
                u16::from(IDBASE_SGW_PLAYER) + u16::from(payload[4]),
                &payload[5..],
            )
        } else {
            (u16::from(marker & 0x7F), &payload[4..])
        };
        out.push((idx, args.to_vec()));
        i += 3 + len;
    }
    out
}

fn i32_arg(args: &[u8]) -> i32 {
    i32::from_le_bytes(args[..4].try_into().unwrap())
}

/// The `onMaxExpUpdate` values in a decoded bundle, in order.
fn max_exp_values(records: &[(u16, Vec<u8>)]) -> Vec<i32> {
    records
        .iter()
        .filter(|(idx, _)| *idx == method_idx::ON_MAX_EXP_UPDATE)
        .map(|(_, a)| i32_arg(a))
        .collect()
}

/// The `onEntityProperty(TrainingPoints)` value, if the bundle carries one.
fn training_points_sent(records: &[(u16, Vec<u8>)]) -> Option<i32> {
    records
        .iter()
        .find(|(idx, a)| *idx == method_idx::ON_ENTITY_PROPERTY && i32_arg(a) == 1)
        .map(|(_, a)| i32_arg(&a[4..]))
}

/// Run the handler's own level-up rule, then build its bundle, exactly as
/// `handle_grant_xp` does between the state read and the send.
fn grant(level: u32, tp: u32, prev_xp: u64, amount: u64) -> (u32, u32, Vec<u32>, ChannelBundle) {
    let (mut level, mut tp) = (level, tp);
    let total = prev_xp.saturating_add(amount);
    let gained = apply_level_ups(&mut level, &mut tp, total);
    let bundle = build_grant_xp_bundle(42, total, level, tp, &gained);
    (level, tp, gained, bundle)
}

#[test]
fn grant_crossing_20_to_21_levels_and_shows_the_21_threshold() {
    let (level, tp, gained, bundle) = grant(20, 20, 400_000, 1);
    assert_eq!(gained, vec![21], "level 20 is no longer the cap");
    assert_eq!(level, 21);
    assert_eq!(tp, 21, "one point per level");
    let records = decode_records(bundle);
    assert_eq!(max_exp_values(&records), vec![460_000]);
    assert_eq!(training_points_sent(&records), Some(21));
}

#[test]
fn grant_crossing_49_to_50_sends_the_sentinel() {
    let (level, tp, gained, bundle) = grant(49, 49, 23_065_000, 1);
    assert_eq!(gained, vec![50]);
    assert_eq!(level, MAX_LEVEL);
    assert_eq!(tp, 50, "50 points in total at the cap");
    let records = decode_records(bundle);
    assert_eq!(
        max_exp_values(&records),
        vec![SENTINEL],
        "reaching 50 must show the display sentinel, not a level-51 threshold"
    );
    assert_eq!(training_points_sent(&records), Some(50));
}

/// XP at the cap is still credited, but there is no level 51 and no 51st
/// point, so the bundle carries only `onExpUpdate` (no level-up ceremony,
/// no points property). The client already holds the sentinel from the
/// 49→50 grant or from world entry (`mapLoaded`).
#[test]
fn grant_at_50_stays_50_with_50_points() {
    let (level, tp, gained, bundle) = grant(50, 50, 26_525_000, 5_000_000);
    assert!(gained.is_empty(), "no level 51: {gained:?}");
    assert_eq!(level, 50);
    assert_eq!(tp, 50);
    let records = decode_records(bundle);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].0, method_idx::ON_EXP_UPDATE);
    assert_eq!(i32_arg(&records[0].1), 31_525_000);
}

/// A single grant that vaults a level-1 character (holding its 1 starting
/// point) past every threshold lands on 50 with exactly 50 points, and the
/// last `onMaxExpUpdate` is the sentinel.
#[test]
fn grant_from_1_past_the_whole_table_stops_at_50() {
    let (level, tp, gained, bundle) = grant(1, 1, 0, 100_000_000);
    assert_eq!(gained, (2..=50).collect::<Vec<u32>>());
    assert_eq!((level, tp), (50, 50));
    let records = decode_records(bundle);
    let max_exp = max_exp_values(&records);
    assert_eq!(max_exp.len(), 49);
    assert_eq!(max_exp.first(), Some(&200), "level 2 shows LEVEL_XP[2]");
    assert_eq!(max_exp.last(), Some(&SENTINEL));
}

// ── Live-DB: the handler persists the capped level and point total ──────

/// Seed a player at `(level, exp, training_points)`, grant `amount` through
/// `handle_grant_xp`, and return the persisted `(level, exp, training_points)`
/// plus the in-memory values the session now holds.
async fn grant_persisted(
    pool: &sqlx::PgPool,
    offset: i32,
    level: i32,
    exp: i32,
    tp: i32,
    amount: u64,
) -> ((i32, i32, i32), (Option<i32>, Option<u64>, Option<u32>)) {
    let account_id = TEST_PLAYER_BASE + offset;
    let player_id = TEST_PLAYER_BASE + offset + 1;
    let entity_id: u32 = 9_990_000 + offset as u32;
    let addr: SocketAddr = format!("127.0.0.1:{}", 56_000 + offset).parse().unwrap();
    cleanup(pool, account_id).await;
    insert_test_account(pool, account_id).await;
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, exp, training_points, alignment, archetype, \
            gender, player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id\
         ) VALUES ($1, $2, $3, $4, $5, 0, 1, 1, $6, '', 'CombatSim', \
                   'BS_HumanMale.BS_HumanMale', 0.0, 0.0, 0.0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(level)
    .bind(exp)
    .bind(tp)
    .bind(format!("cap-{player_id}"))
    .execute(pool)
    .await
    .expect("INSERT capped test sgw_player row");

    let mut state = make_connected_state(Some(player_id));
    state.player_level = Some(level);
    state.player_xp = Some(exp as u64);
    state.player_training_points = Some(tp as u32);
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));

    handle_grant_xp(
        entity_id,
        amount,
        None,
        &Some(Arc::new(pool.clone())),
        &transport,
        &connected,
        &entity_to_addr,
        &None,
    )
    .await;

    let persisted: (i32, i32, i32) =
        sqlx::query_as("SELECT level, exp, training_points FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .fetch_one(pool)
            .await
            .unwrap();
    let in_memory = {
        let map = connected.lock().unwrap();
        let s = map.get(&addr).unwrap();
        (s.player_level, s.player_xp, s.player_training_points)
    };
    cleanup(pool, account_id).await;
    (persisted, in_memory)
}

#[tokio::test]
async fn handle_grant_xp_persists_20_to_21() {
    let pool = require_db_or_skip!();
    let (persisted, in_memory) = grant_persisted(&pool, 600, 20, 400_000, 20, 1).await;
    assert_eq!(persisted, (21, 400_001, 21));
    assert_eq!(in_memory, (Some(21), Some(400_001), Some(21)));
}

#[tokio::test]
async fn handle_grant_xp_persists_49_to_50() {
    let pool = require_db_or_skip!();
    let (persisted, in_memory) = grant_persisted(&pool, 610, 49, 23_065_000, 49, 1).await;
    assert_eq!(persisted, (50, 23_065_001, 50));
    assert_eq!(in_memory, (Some(50), Some(23_065_001), Some(50)));
}

#[tokio::test]
async fn handle_grant_xp_at_50_credits_xp_but_no_level_or_point() {
    let pool = require_db_or_skip!();
    let (persisted, in_memory) = grant_persisted(&pool, 620, 50, 26_525_000, 50, 1_000_000).await;
    assert_eq!(persisted, (50, 27_525_000, 50));
    assert_eq!(in_memory, (Some(50), Some(27_525_000), Some(50)));
}

async fn insert_at_level(
    pool: &sqlx::PgPool,
    account_id: i32,
    player_id: i32,
    level: i32,
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id\
         ) VALUES ($1, $2, $3, 0, 1, 1, $4, '', 'CombatSim', \
                   'BS_HumanMale.BS_HumanMale', 0.0, 0.0, 0.0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(level)
    .bind(format!("sanity-{player_id}"))
    .execute(pool)
    .await
}

/// `sgw_player.level_sanity` accepts 50 and rejects 51. Before AT-07 the
/// CHECK was `level <= 20`, so the level-50 insert below fails against it.
#[tokio::test]
async fn level_sanity_accepts_50_and_rejects_51() {
    let pool = require_db_or_skip!();
    let account_id = TEST_PLAYER_BASE + 630;
    cleanup(&pool, account_id).await;
    insert_test_account(&pool, account_id).await;

    let at_50 = insert_at_level(&pool, account_id, TEST_PLAYER_BASE + 631, 50).await;
    let at_51 = insert_at_level(&pool, account_id, TEST_PLAYER_BASE + 632, 51).await;
    cleanup(&pool, account_id).await;

    assert!(at_50.is_ok(), "level 50 must be accepted: {at_50:?}");
    let err = at_51.expect_err("level 51 must be rejected");
    assert!(
        err.to_string().contains("level_sanity"),
        "level 51 must be rejected by level_sanity, got: {err}"
    );
}
