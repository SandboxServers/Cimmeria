//! Live-DB integration tests for `handle_execute_trade`.
//!
//! Skip cleanly when `DATABASE_URL` is unset. Against the bundled
//! local Postgres they exercise the atomic happy-path, the rollback
//! semantics on insufficient cash / missing items, the bound-item
//! rejection guard, and the not-enough-slots rollback.
//!
//! Split into submodules by theme once the previous flat `tests.rs`
//! crossed the 700-line hard cap (CLAUDE.md §"File organization"):
//!
//! - [`commit`] — core commit/rollback tests (happy path, insufficient
//!   cash, missing item, bound item, buyback, duplicate instance,
//!   negative cash).
//! - [`container_whitelist`] — security-review regression guards for
//!   the tradeable-container whitelist (`INV_MAIN` only).
//! - [`no_db`] — DB-less early-return tests for the no-pool branch.
//! - [`slot_reservation`] — recipient-slot accounting regression guard
//!   for the full-bag-swap scenario.
//!
//! All submodules `use super::*;` to pick up the shared helpers
//! defined here.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::base::ConnectedClientState;
use crate::test_support::TestTransport;

mod commit;
mod container_whitelist;
mod no_db;
mod slot_reservation;

/// Sentinel base for trade live-DB tests. Distinct from prior live-DB
/// sentinels (purchase +0x0A00 / ammo +0x0B00 / vendor_data +0x0C00 /
/// player_load +0x0D00 / sync_bandolier +0x0E00).
pub(super) const TEST_BASE: i32 = 0x7000_0F00;

/// Pull two distinct, currently-existing item type ids from the live
/// `resources.items` seed at runtime. Hard-coded type-id constants
/// are explicitly discouraged by TESTING.md because they break when
/// seeds shift; the runtime lookup picks the two lowest-numbered ids
/// so the result is stable enough to be deterministic across CI runs
/// but not so brittle that a seed edit moves them.
///
/// Returns `(first, second)` in ascending order. Panics if the
/// seed has fewer than two rows — that's a precondition failure
/// for any trade test, since both sides of a trade need to offer
/// distinct items to exercise the swap.
pub(super) async fn tradeable_type_ids(pool: &PgPool) -> (i32, i32) {
    let ids: Vec<i32> =
        sqlx::query_scalar("SELECT item_id FROM resources.items ORDER BY item_id ASC LIMIT 2")
            .fetch_all(pool)
            .await
            .expect("query resources.items seed for tradeable type ids");
    assert!(
        ids.len() >= 2,
        "trade tests require at least 2 rows in `resources.items` — \
         the live-DB seed must be loaded before running this test"
    );
    (ids[0], ids[1])
}

pub(super) async fn cleanup(pool: &PgPool, accounts: &[i32], players: &[i32]) {
    for &p in players {
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(p)
            .execute(pool)
            .await;
    }
    for &a in accounts {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(a)
            .execute(pool)
            .await;
    }
}

pub(super) async fn insert_account_and_player(
    pool: &PgPool,
    account_id: i32,
    player_id: i32,
    naquadah: i32,
    label: &str,
) {
    sqlx::query(
        "INSERT INTO account (account_id, account_name, password) \
         VALUES ($1, $2, '')",
    )
    .bind(account_id)
    .bind(format!("trade-test-{account_id}-{label}"))
    .execute(pool)
    .await
    .expect("insert account");

    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
         ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                   0.0, 0.0, 0.0, 0, $4, 0)",
    )
    .bind(account_id)
    .bind(player_id)
    .bind(format!("trader-{player_id}"))
    .bind(naquadah)
    .execute(pool)
    .await
    .expect("insert player");
}

pub(super) async fn insert_item(
    pool: &PgPool,
    player_id: i32,
    type_id: i32,
    container_id: i32,
    slot_id: i32,
    bound: bool,
) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges) \
         VALUES ($1, $2, 1, $3, $4, $5, 100, 0) \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(slot_id)
    .bind(container_id)
    .bind(bound)
    .fetch_one(pool)
    .await
    .expect("insert item")
}

pub(super) async fn naquadah_of(pool: &PgPool, player_id: i32) -> i32 {
    sqlx::query_scalar("SELECT naquadah FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_one(pool)
        .await
        .expect("read naquadah")
}

pub(super) async fn owner_of(pool: &PgPool, item_id: i32) -> Option<i32> {
    sqlx::query_scalar("SELECT character_id FROM sgw_inventory WHERE item_id = $1")
        .bind(item_id)
        .fetch_optional(pool)
        .await
        .expect("read owner")
}

pub(super) async fn slot_id_of(pool: &PgPool, item_id: i32) -> Option<i32> {
    sqlx::query_scalar("SELECT slot_id FROM sgw_inventory WHERE item_id = $1")
        .bind(item_id)
        .fetch_optional(pool)
        .await
        .expect("read slot_id")
}

#[derive(Debug, Clone)]
pub(super) struct TradeFixtures {
    pub(super) account_a: i32,
    pub(super) account_b: i32,
    pub(super) player_a: i32,
    pub(super) player_b: i32,
    pub(super) entity_a: u32,
    pub(super) entity_b: u32,
}

pub(super) fn fixtures(salt: i32) -> TradeFixtures {
    TradeFixtures {
        account_a: TEST_BASE + salt,
        account_b: TEST_BASE + salt + 1,
        player_a: TEST_BASE + salt + 10,
        player_b: TEST_BASE + salt + 11,
        entity_a: (TEST_BASE + salt + 100) as u32,
        entity_b: (TEST_BASE + salt + 101) as u32,
    }
}

pub(super) fn make_state(
    entity_a: u32,
    entity_b: u32,
) -> (
    Arc<dyn Transport>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let addr_a: SocketAddr = "127.0.0.1:65534".parse().unwrap();
    let addr_b: SocketAddr = "127.0.0.1:65533".parse().unwrap();
    let mut m = HashMap::new();
    m.insert(entity_a, addr_a);
    m.insert(entity_b, addr_b);
    let entity_to_addr = Arc::new(Mutex::new(m));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    (transport, entity_to_addr, connected)
}
