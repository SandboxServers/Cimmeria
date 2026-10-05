//! The Debug Area gate (stargate 29, `debug_dial_hub`, DA-07) never reaches a
//! persisted address book: not through the arrival unlock when a traveller
//! leaves the Debug Area (origin half) or arrives in it via `.gotolocation`
//! (destination half), and not through the content grant's append.
//!
//! Live-DB: the filter is a `NOT EXISTS` against `resources.stargates`, so
//! only the real seed row can show it working. Each test fails if the
//! `debug_dial_hub` clause is dropped from its statement.

use super::super::address_grant::append_known_stargate;
use super::super::persist_arrival;
use crate::test_support::require_db_or_skip;
use cimmeria_entity::cell_entity::PlayerIdentity;
use sqlx::PgPool;

/// Sentinel range for DA-07. Cleanup deletes by exact id, never by range.
const TEST_BASE: i32 = 0x70DA_0700;

/// The Debug Area gate, world 1300.
const HUB: i32 = 29;
/// Harset's gate, world 57.
const HARSET_GATE: i32 = 3;

async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
    let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(account_id)
        .execute(pool)
        .await;
}

/// `extra_name` and `bodyset` are NOT NULL with a `NULL::varchar` default,
/// so they are named even though nothing here reads them.
async fn seed(pool: &PgPool, account_id: i32, player_id: i32, world: &str) {
    cleanup(pool, account_id, player_id).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(account_id)
        .bind(format!("da07-{account_id}"))
        .execute(pool)
        .await
        .expect("insert sentinel account");
    sqlx::query(
        "INSERT INTO sgw_player \
           (account_id, player_id, player_name, extra_name, bodyset, world_location, \
            alignment, archetype, gender, pos_x, pos_y, pos_z, skin_color_id, \
            known_stargates) \
         VALUES ($1, $2, $3, '', 'BS_HumanMale.BS_HumanMale', $4, \
                 1, 1, 1, 0, 0, 0, 0, '{}'::integer[])",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("DA07Pin{player_id}"))
    .bind(world)
    .execute(pool)
    .await
    .expect("insert sentinel player");
}

async fn book(pool: &PgPool, player_id: i32) -> (Vec<i32>, String) {
    sqlx::query_as("SELECT known_stargates, world_location FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .expect("sentinel player row must exist")
}

/// Precondition for every test here: the seed really has the hub row.
async fn assert_hub_seeded(pool: &PgPool) {
    let hub: Option<(bool, String)> = sqlx::query_as(
        "SELECT s.debug_dial_hub, w.world::text FROM resources.stargates s \
           JOIN resources.worlds w ON w.world_id = s.world_id WHERE s.stargate_id = $1",
    )
    .bind(HUB)
    .fetch_optional(pool)
    .await
    .expect("query must succeed");
    assert_eq!(
        hub,
        Some((true, "DebugArea".to_string())),
        "fixture assumption: stargate 29 is the DebugArea dial hub"
    );
}

/// Leaving the Debug Area through its gate: the origin half of the unlock
/// resolves the pre-update `world_location` (DebugArea) to its gates, which
/// is exactly the hub. The traveller learns Harset (the destination half)
/// and never 29.
#[tokio::test]
async fn live_db_leaving_the_debug_area_never_learns_its_gate() {
    let pool = require_db_or_skip!();
    assert_hub_seeded(&pool).await;
    let (account_id, player_id) = (TEST_BASE, TEST_BASE + 10);
    seed(&pool, account_id, player_id, "DebugArea").await;

    persist_arrival(
        &Some(std::sync::Arc::new(pool.clone())),
        player_id,
        account_id as u32,
        "Harset",
        [0.0, -67.0, 38.0],
        &[HARSET_GATE],
        PlayerIdentity::UNKNOWN,
    )
    .await;

    let (known, world) = book(&pool, player_id).await;
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(world, "Harset", "control: the arrival was persisted");
    assert_eq!(
        known,
        vec![HARSET_GATE],
        "the origin half must not teach the Debug Area gate"
    );
}

/// Arriving in the Debug Area (`.gotolocation DebugArea` rides the same
/// transfer): `destination_gates` is the world's gate list — which must keep
/// the hub, because it is also the client's `worldStargateList` — and the
/// statement must still not learn it. Departs a gateless world so nothing
/// else can contribute.
#[tokio::test]
async fn live_db_arriving_in_the_debug_area_never_learns_its_gate() {
    let pool = require_db_or_skip!();
    assert_hub_seeded(&pool).await;
    let (account_id, player_id) = (TEST_BASE + 1, TEST_BASE + 11);
    seed(&pool, account_id, player_id, "Castle_CellBlock").await;

    persist_arrival(
        &Some(std::sync::Arc::new(pool.clone())),
        player_id,
        account_id as u32,
        "DebugArea",
        [251.0, 8.0, -962.0],
        &[HUB],
        PlayerIdentity::UNKNOWN,
    )
    .await;

    let (known, world) = book(&pool, player_id).await;
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(world, "DebugArea", "control: the arrival was persisted");
    assert!(
        known.is_empty(),
        "the destination half must not teach the Debug Area gate: {known:?}"
    );
}

/// The content grant's append refuses the hub even if a cell sent it.
#[tokio::test]
async fn live_db_the_grant_append_never_persists_the_hub() {
    let pool = require_db_or_skip!();
    assert_hub_seeded(&pool).await;
    let (account_id, player_id) = (TEST_BASE + 2, TEST_BASE + 12);
    seed(&pool, account_id, player_id, "Castle_CellBlock").await;

    let returned = append_known_stargate(&pool, player_id, account_id, HUB)
        .await
        .expect("statement must succeed");
    let control = append_known_stargate(&pool, player_id, account_id, HARSET_GATE)
        .await
        .expect("statement must succeed");

    let (known, _) = book(&pool, player_id).await;
    cleanup(&pool, account_id, player_id).await;
    assert_eq!(returned, Some(vec![]), "the hub must not be appended");
    assert_eq!(control, Some(vec![HARSET_GATE]), "control: Harset appends");
    assert_eq!(known, vec![HARSET_GATE]);
}
