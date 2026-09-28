//! Live-DB guards for the Applied Science Points a level-up earns.
//!
//! `handle_grant_xp` adds one point per level gained in the same statement
//! that raises the level, logs `asp_earned` under `crafting`, and pushes the
//! new total to the owning client as the ASP property after the XP bundle.

use super::tests::{cleanup, insert_test_account, make_connected_state};
use super::*;
use crate::mercury::build_player_entity_method_packet;
use crate::test_support::{
    require_db_or_skip, Captured, LogCapture, LogCaptureGuard, TestTransport,
};
use cimmeria_mercury::encryption::EncryptionVersion;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/// Sentinel base in the crafting `0x7000_Cxxx` block. Fits in i32.
const TEST_BASE: i32 = 0x7000_C700;
const ACCOUNT_ID: u32 = 4711;

/// Seed a player row at `(db_level, exp, asp)` and a session whose cache
/// says `cached_level`.
struct Fixture {
    account_id: i32,
    player_id: i32,
    entity_id: u32,
    addr: SocketAddr,
    transport_typed: Arc<TestTransport>,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

async fn fixture(
    pool: &sqlx::PgPool,
    offset: i32,
    db_level: i32,
    cached_level: i32,
    exp: i32,
    asp: i32,
) -> Fixture {
    let account_id = TEST_BASE + offset;
    let player_id = TEST_BASE + offset + 1;
    cleanup(pool, account_id).await;
    insert_test_account(pool, account_id).await;
    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, exp, training_points, applied_science_points, \
            alignment, archetype, gender, player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id\
         ) VALUES ($1, $2, $3, $4, $3, $5, 0, 1, 1, $6, '', 'CombatSim', \
                   'BS_HumanMale.BS_HumanMale', 0.0, 0.0, 0.0, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(db_level)
    .bind(exp)
    .bind(asp)
    .bind(format!("asp-{player_id}"))
    .execute(pool)
    .await
    .expect("INSERT asp test sgw_player row");

    let entity_id: u32 = 9_970_000 + offset as u32;
    let addr: SocketAddr = format!("127.0.0.1:{}", 56_700 + offset).parse().unwrap();
    let mut state = make_connected_state(Some(player_id));
    state.account_id = ACCOUNT_ID;
    state.player_level = Some(cached_level);
    state.player_xp = Some(exp as u64);
    state.player_training_points = Some(cached_level as u32);
    let transport_typed = Arc::new(TestTransport::new());
    Fixture {
        account_id,
        player_id,
        entity_id,
        addr,
        transport: transport_typed.clone(),
        transport_typed,
        connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
        entity_to_addr: Arc::new(Mutex::new(HashMap::from([(entity_id, addr)]))),
    }
}

impl Fixture {
    async fn grant(&self, pool: &sqlx::PgPool, xp: u64) {
        handle_grant_xp(
            self.entity_id,
            xp,
            None,
            &Some(Arc::new(pool.clone())),
            &self.transport,
            &self.connected,
            &self.entity_to_addr,
            &None,
        )
        .await;
    }

    async fn persisted(&self, pool: &sqlx::PgPool) -> (i32, i32) {
        sqlx::query_as("SELECT level, applied_science_points FROM sgw_player WHERE player_id = $1")
            .bind(self.player_id)
            .fetch_one(pool)
            .await
            .expect("read back level and ASP")
    }

    /// The ASP property packet the owning client should receive last.
    fn asp_packet(&self, seq: u32, total: i32) -> Vec<u8> {
        build_player_entity_method_packet(
            &[0u8; 32],
            seq,
            &[],
            self.entity_id,
            method_idx::ON_ENTITY_PROPERTY,
            &cimmeria_wire::crafting::applied_science_points_property_args(total),
            EncryptionVersion::V1,
        )
    }
}

fn asp_earned_events(capture: &LogCaptureGuard) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "crafting" && c.has_field("event", "asp_earned"))
        .collect()
}

/// Regression guard: a one-level grant persists the level and one more
/// point in the same write, logs `asp_earned` with the full identity and
/// both totals, and pushes the new total after the XP bundle. Removing the
/// grant from the statement leaves the points at 3; removing the push
/// leaves the XP bundle as the only packet.
#[tokio::test]
async fn live_db_level_up_earns_one_asp_and_pushes_the_total() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = fixture(&pool, 0, 1, 1, 0, 3).await;

    // Past LEVEL_XP[1] = 100 (a level-up needs more than the threshold): exactly one boundary.
    f.grant(&pool, 101).await;
    let persisted = f.persisted(&pool).await;
    let sent = f.transport_typed.filter_to(f.addr);
    cleanup(&pool, f.account_id).await;

    assert_eq!(persisted, (2, 4), "level 2 must carry its point");
    assert_eq!(sent.len(), 2, "XP bundle, then the ASP property: {sent:?}");
    // The XP bundle took reliable seq 1 (`make_connected_state` starts at 1).
    assert_eq!(
        sent[1],
        f.asp_packet(2, 4),
        "the ASP property must carry the new total, byte for byte"
    );

    let events = asp_earned_events(&capture);
    assert_eq!(events.len(), 1, "one asp_earned per grant: {events:#?}");
    for (k, v) in [
        ("account_id", ACCOUNT_ID.to_string()),
        ("player_id", f.player_id.to_string()),
        ("entity_id", f.entity_id.to_string()),
        ("level_before", "1".to_string()),
        ("level_after", "2".to_string()),
        ("asp_before", "3".to_string()),
        ("asp_after", "4".to_string()),
    ] {
        assert!(events[0].has_field(k, &v), "{k}={v}: {:#?}", events[0]);
    }
}

/// A catch-up grant across several levels earns one point for each, still in
/// one write and with one push carrying the final total.
#[tokio::test]
async fn live_db_multi_level_grant_earns_one_asp_per_level() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = fixture(&pool, 10, 1, 1, 0, 1).await;

    f.grant(&pool, 1_000).await;
    let (level, asp) = f.persisted(&pool).await;
    let sent = f.transport_typed.filter_to(f.addr);
    cleanup(&pool, f.account_id).await;

    assert!(level > 3, "test invariant: several levels, got {level}");
    assert_eq!(asp, level, "1 at level 1, plus one per level gained");
    assert_eq!(sent.len(), 2, "one ASP push for the whole grant");
    assert_eq!(sent[1], f.asp_packet(2, asp));
    let events = asp_earned_events(&capture);
    assert_eq!(events.len(), 1);
    assert!(events[0].has_field("level_after", &level.to_string()));
    assert!(events[0].has_field("asp_after", &asp.to_string()));
}

/// The ASP total saturates at `i32::MAX` (the column is `integer`): a
/// level-up on a character one below the maximum still commits the level
/// and pushes `i32::MAX`. Without the clamp Postgres raises "integer out of
/// range", the whole write rolls back and the level is lost.
#[tokio::test]
async fn live_db_asp_saturates_at_i32_max() {
    let pool = require_db_or_skip!();
    let f = fixture(&pool, 60, 1, 1, 0, i32::MAX - 1).await;

    // Several levels at once: MAX - 1 plus more than one point overflows.
    f.grant(&pool, 1_000).await;
    let (level, asp) = f.persisted(&pool).await;
    let sent = f.transport_typed.filter_to(f.addr);
    cleanup(&pool, f.account_id).await;

    assert!(level > 2, "the level-up must commit, got level {level}");
    assert_eq!(asp, i32::MAX, "ASP clamps at i32::MAX");
    assert_eq!(sent.len(), 2, "XP bundle, then the ASP property");
    assert_eq!(sent[1], f.asp_packet(2, i32::MAX));
}

/// XP that crosses no boundary earns nothing: no point, no event, no push.
#[tokio::test]
async fn live_db_xp_without_a_level_up_earns_no_asp() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = fixture(&pool, 20, 1, 1, 0, 5).await;

    f.grant(&pool, 50).await;
    let persisted = f.persisted(&pool).await;
    let sent = f.transport_typed.filter_to(f.addr);
    cleanup(&pool, f.account_id).await;

    assert_eq!(persisted, (1, 5));
    assert_eq!(sent.len(), 1, "the XP bundle only");
    assert!(asp_earned_events(&capture).is_empty());
}

/// At the level cap XP still accrues but no level and so no point.
#[tokio::test]
async fn live_db_grant_at_the_cap_earns_no_asp() {
    let pool = require_db_or_skip!();
    let f = fixture(&pool, 30, 50, 50, 26_525_000, 50).await;

    f.grant(&pool, 1_000_000).await;
    let persisted = f.persisted(&pool).await;
    cleanup(&pool, f.account_id).await;

    assert_eq!(persisted, (50, 50));
}

/// The levels gained are counted against the row, not the session cache:
/// when the row already holds the level the cache computes, the write earns
/// nothing, so a stale cache can never grant the same level's point twice.
#[tokio::test]
async fn live_db_levels_are_counted_against_the_persisted_level() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    // Row at level 2 with its 2 points; the cache still says level 1.
    let f = fixture(&pool, 40, 2, 1, 0, 2).await;

    f.grant(&pool, 101).await;
    let persisted = f.persisted(&pool).await;
    let sent = f.transport_typed.filter_to(f.addr);
    cleanup(&pool, f.account_id).await;

    assert_eq!(persisted, (2, 2), "level 2 was already paid for");
    assert_eq!(sent.len(), 1, "no ASP push when nothing was earned");
    assert!(asp_earned_events(&capture).is_empty());
}

/// A session whose character row is gone updates nothing: a WARN with the
/// paired `rows_affected` / `expected`, the `phase` and the identity, and no
/// `asp_earned`, no packet.
#[tokio::test]
async fn live_db_missing_row_is_a_persist_failed_warn_and_earns_nothing() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = fixture(&pool, 50, 1, 1, 0, 1).await;
    cleanup(&pool, f.account_id).await;

    f.grant(&pool, 101).await;

    assert_eq!(f.transport_typed.filter_to(f.addr).len(), 0);
    assert!(asp_earned_events(&capture).is_empty());
    let warn = capture
        .all()
        .into_iter()
        .find(|c| c.level == tracing::Level::WARN && c.has_field("event", "persist_failed"))
        .expect("persist_failed WARN");
    for (k, v) in [
        ("phase", "grant_xp_update".to_string()),
        ("reason", "rows_affected_zero".to_string()),
        ("rows_affected", "0".to_string()),
        ("expected", "1".to_string()),
        ("account_id", ACCOUNT_ID.to_string()),
        ("player_id", f.player_id.to_string()),
        ("entity_id", f.entity_id.to_string()),
    ] {
        assert!(warn.has_field(k, &v), "{k}={v}: {warn:#?}");
    }
}

/// Seed guard: every seeded character holds one point per level (all are
/// unspent), so a seeded character is indistinguishable from one created
/// through the client.
#[tokio::test]
async fn live_db_seeded_characters_hold_one_asp_per_level() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, i32, i32)> = sqlx::query_as(
        "SELECT player_id, level, applied_science_points FROM sgw_player \
         WHERE player_id BETWEEN 62 AND 70 ORDER BY player_id",
    )
    .fetch_all(&pool)
    .await
    .expect("read seeded characters");

    assert_eq!(rows.len(), 9, "the seed's nine characters: {rows:?}");
    for (player_id, level, asp) in rows {
        assert_eq!(asp, level, "seeded player {player_id}");
    }
}
