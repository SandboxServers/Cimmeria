//! Live-DB guard: the seeded playtest characters are what character
//! creation writes.
//!
//! Bug shape: the seeded characters (`db/sgw/Players/Seed/sgw_player.sql`,
//! player ids 62-70) drift from what character creation writes, and the colo
//! rebuilds its database from that seed on every deploy, so the playtesters'
//! characters were the broken ones. Since Class Start v6 CS-02 they are
//! debug-kit Praxis Commandos (lock L2): the reference is created through
//! the real handler with the debug kit forced on (a test-only parameter,
//! never access level).
//!
//! The per-profile matrix guards are in `profile_live_db_tests`.

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::sync::Arc;

use super::live_db_tests::{
    build_create_character_payload, cleanup, insert_account, make_connected,
};
use super::*;
use crate::test_support::{require_db_or_skip, TestTransport};

/// Sentinel accounts for these tests; the next free block after
/// `live_db_tests`' `0x7000_1A00` and the `0x7000_1B00`-`0x7000_1D00`
/// neighbours. Fits in `i32`.
const PARITY_ACCOUNT: i32 = 0x7000_1E01;

/// Praxis Commando, male human: what every seeded character is.
const PRAXIS_COMMANDO_MALE: i32 = 3;

/// The seeded playtest characters.
const SEEDED_PLAYER_IDS: std::ops::RangeInclusive<i32> = 62..=70;

/// The first choice of every `VIS_Optional` group of `char_def_id`: the
/// payload a player who clicks straight through the creator sends.
pub(super) async fn default_choices(pool: &PgPool, char_def_id: i32) -> Vec<(i32, i32)> {
    sqlx::query_as::<_, (i32, i32)>(
        "SELECT vg.vis_group_id, MIN(c.choice_id) \
         FROM resources.char_creation_visgroups vg \
         JOIN resources.char_creation_choices c ON c.vis_group_id = vg.vis_group_id \
         WHERE vg.char_def_id = $1 AND vg.vis_type = 'VIS_Optional' \
         GROUP BY vg.vis_group_id ORDER BY vg.vis_group_id",
    )
    .bind(char_def_id)
    .fetch_all(pool)
    .await
    .expect("read default visual choices")
}

/// Create one character through the real handler and return its player id.
/// `debug_kit` forces the debug kit on (the test-only override).
pub(super) async fn create(
    pool: &PgPool,
    account_id: i32,
    access_level: u32,
    char_def_id: i32,
    name: &str,
    debug_kit: bool,
) -> i32 {
    super::live_db_tests::register_start_worlds(pool).await;
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:55811".parse().unwrap();
    let connected = make_connected(addr, account_id as u32, access_level);
    let db_pool = Some(Arc::new(pool.clone()));
    let choices = default_choices(pool, char_def_id).await;
    let payload = build_create_character_payload(name, "", char_def_id, &choices, 0);

    create_character(
        &dyn_transport,
        addr,
        [0u8; 32],
        account_id as u32,
        &payload,
        &connected,
        &db_pool,
        debug_kit,
    )
    .await
    .unwrap_or_else(|e| panic!("createCharacter char_def {char_def_id}: {e}"));

    sqlx::query_scalar(
        "SELECT player_id FROM sgw_player WHERE account_id = $1 AND player_name = $2",
    )
    .bind(account_id)
    .bind(name)
    .fetch_optional(pool)
    .await
    .expect("read created player")
    .unwrap_or_else(|| {
        panic!("char_def {char_def_id}: no character row (the handler refused the payload)")
    })
}

/// Every char_def a client can pick, read from `resources.char_creation` so a
/// new char_def is covered without an edit here.
pub(super) async fn char_defs(pool: &PgPool) -> Vec<i32> {
    sqlx::query_scalar("SELECT char_def_id FROM resources.char_creation ORDER BY char_def_id")
        .fetch_all(pool)
        .await
        .expect("read char_defs")
}

/// Columns of `table` whose value depends on when the row was written: a
/// clock default (`now()`, `CURRENT_TIMESTAMP`, `clock_timestamp()`, ...).
/// A seed can't reproduce the reference's creation time, so these are left
/// out of the comparison. A column the handler fills from the clock itself
/// (no default) has to be added to the `- '...'` list of the shape query by
/// hand.
async fn clock_columns(pool: &PgPool, table: &str) -> Vec<String> {
    sqlx::query_scalar(
        "SELECT column_name::text FROM information_schema.columns \
         WHERE table_schema = current_schema() AND table_name = $1 \
           AND (column_default ILIKE '%now()%' \
                OR column_default ILIKE '%current_timestamp%' \
                OR column_default ILIKE '%clock_timestamp%' \
                OR column_default ILIKE '%statement_timestamp%' \
                OR column_default ILIKE '%transaction_timestamp%' \
                OR column_default ILIKE '%current_date%' \
                OR column_default ILIKE '%localtimestamp%')",
    )
    .bind(table)
    .fetch_all(pool)
    .await
    .expect("read clock-default columns")
}

/// The row without the columns that name it or carry a clock default: the
/// parity comparand. `to_jsonb` covers every column, so a new one is
/// compared without an edit, and `jsonb`'s text form is canonical (keys
/// sorted), so two rows compare equal as text exactly when they are equal.
async fn player_shape(pool: &PgPool, player_id: i32) -> String {
    let skip = clock_columns(pool, "sgw_player").await;
    sqlx::query_scalar(
        "SELECT (to_jsonb(p) - 'player_id' - 'account_id' - 'player_name' - 'extra_name' \
                 - $2::text[])::text \
         FROM sgw_player p WHERE player_id = $1",
    )
    .bind(player_id)
    .bind(&skip)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|e| panic!("read player {player_id}: {e}"))
}

/// The character's inventory without the instance and owner ids (or a clock
/// default, see [`clock_columns`]), in container/slot order.
async fn inventory_shape(pool: &PgPool, player_id: i32) -> Vec<String> {
    let skip = clock_columns(pool, "sgw_inventory").await;
    sqlx::query_scalar(
        "SELECT (to_jsonb(i) - 'item_id' - 'character_id' - $2::text[])::text \
         FROM sgw_inventory i WHERE character_id = $1 ORDER BY container_id, slot_id",
    )
    .bind(player_id)
    .bind(&skip)
    .fetch_all(pool)
    .await
    .unwrap_or_else(|e| panic!("read inventory of {player_id}: {e}"))
}

/// **Regression guard (seed drift).** Every seeded playtest character is the
/// Praxis Commando that `createCharacter` makes today, column for column
/// (ids, names and account aside), with the same inventory. The reference
/// is created on an account at the seeded accounts' access level, as a
/// playtester's own character would be. Reverting `sgw_player.sql` to the
/// old SGU Soldier rows (three starter abilities, SGC_W1, no pistol) fails
/// the player comparison; reverting `sgw_inventory.sql` fails the inventory
/// comparison.
#[tokio::test]
async fn seeded_characters_match_a_fresh_praxis_commando_live_db() {
    let pool = require_db_or_skip!();

    let seeded: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT p.player_id, a.accesslevel FROM sgw_player p \
         JOIN account a ON a.account_id = p.account_id \
         WHERE p.player_id BETWEEN $1 AND $2 ORDER BY p.player_id",
    )
    .bind(*SEEDED_PLAYER_IDS.start())
    .bind(*SEEDED_PLAYER_IDS.end())
    .fetch_all(&pool)
    .await
    .expect("read seeded characters");
    assert_eq!(
        seeded.len(),
        SEEDED_PLAYER_IDS.count(),
        "every seeded character (62-70) is present"
    );
    let levels: BTreeSet<i32> = seeded.iter().map(|&(_, level)| level).collect();
    assert_eq!(
        levels.len(),
        1,
        "the seeded accounts share one access level, so one reference fits all: {levels:?}"
    );
    let access_level = *levels.first().unwrap();

    cleanup(&pool, PARITY_ACCOUNT).await;
    insert_account(&pool, PARITY_ACCOUNT, access_level).await;
    let reference = create(
        &pool,
        PARITY_ACCOUNT,
        access_level as u32,
        PRAXIS_COMMANDO_MALE,
        "Seed Parity Ref",
        true,
    )
    .await;
    let want_player = player_shape(&pool, reference).await;
    let want_inventory = inventory_shape(&pool, reference).await;
    assert!(
        !want_inventory.is_empty(),
        "a fresh Praxis Commando has starter items"
    );

    for &(player_id, _) in &seeded {
        assert_eq!(
            player_shape(&pool, player_id).await,
            want_player,
            "seeded player {player_id} must be the sgw_player row createCharacter writes \
             for a Praxis Commando (char_def {PRAXIS_COMMANDO_MALE}); update \
             db/sgw/Players/Seed/sgw_player.sql to match"
        );
        let got_inventory = inventory_shape(&pool, player_id).await;
        assert_eq!(
            got_inventory.len(),
            want_inventory.len(),
            "seeded player {player_id} has {} starter items, a fresh Praxis Commando {}; \
             update db/sgw/Inventory/Seed/sgw_inventory.sql",
            got_inventory.len(),
            want_inventory.len()
        );
        // Row by row, so a failure prints the one row that differs in full.
        for (got, want) in got_inventory.iter().zip(&want_inventory) {
            assert_eq!(
                got, want,
                "seeded player {player_id} must start with the inventory createCharacter \
                 gives a Praxis Commando; update db/sgw/Inventory/Seed/sgw_inventory.sql"
            );
        }
    }

    cleanup(&pool, PARITY_ACCOUNT).await;
}
