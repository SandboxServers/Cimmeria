//! Live-DB guards for the starter kit and the seeded playtest characters.
//!
//! Bug shapes:
//!
//! 1. A class spawns unable to fire Pistol Shot (592 needs one round in the
//!    active bandolier slot, else NoAmmo) or without the starter heals
//!    (597 Heal Focus, 1646 Health Heal, 1218 Recuperation). Maintainer
//!    report 2026-10-04: "starting classes should all spawn with at least
//!    pistol shot, focus regen and health regen abilities".
//! 2. The seeded characters (`db/sgw/Players/Seed/sgw_player.sql`, player
//!    ids 62-70) drift from what character creation writes. They used to be
//!    hand-written SGU Soldiers with three of the five starter abilities and
//!    no weapon, and the colo rebuilds its database from that seed on every
//!    deploy, so the playtesters' characters were the broken ones.
//!
//! Both tests drive the real `handle_create_character`, so a change to
//! creation (a new starter ability, a new column default) fails the parity
//! test until the seed is updated to match.

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
const KIT_ACCOUNT: i32 = 0x7000_1E00;
const PARITY_ACCOUNT: i32 = 0x7000_1E01;

/// Praxis Commando, male human: what every seeded character is.
const PRAXIS_COMMANDO_MALE: i32 = 3;

/// The seeded playtest characters.
const SEEDED_PLAYER_IDS: std::ops::RangeInclusive<i32> = 62..=70;

/// What the maintainer asked every class to spawn with: Pistol Shot, Heal
/// Focus (focus regen), Health Heal and Recuperation (health regen).
const REQUIRED_STARTERS: [i32; 4] = [592, 597, 1646, 1218];

/// The starter pistol (`char_creation_items`): SI 3 9mm Pistol.
const STARTER_PISTOL: i32 = 55;

const INV_BANDOLIER: i32 = 3;

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
async fn create(
    pool: &PgPool,
    account_id: i32,
    access_level: u32,
    char_def_id: i32,
    name: &str,
) -> i32 {
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:55811".parse().unwrap();
    let connected = make_connected(addr, account_id as u32, access_level);
    let db_pool = Some(Arc::new(pool.clone()));
    let choices = default_choices(pool, char_def_id).await;
    let payload = build_create_character_payload(name, "", char_def_id, &choices, 0);

    handle_create_character(
        &dyn_transport,
        addr,
        [0u8; 32],
        account_id as u32,
        &payload,
        &connected,
        &db_pool,
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
async fn char_defs(pool: &PgPool) -> Vec<i32> {
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

/// **Regression guard (maintainer report 2026-10-04).** Every char_def,
/// created with its default visual choices, starts with Pistol Shot, Heal
/// Focus, Health Heal and Recuperation, and holds the starter pistol in
/// bandolier slot 0 (the active slot) with a full magazine. Reverting the
/// `char_creation_items` seed fails `pistol in bandolier slot 0`; reverting
/// the magazine load in `insert_starter_inventory` fails `loaded`.
#[tokio::test]
async fn every_char_def_starts_armed_with_the_starter_set_live_db() {
    let pool = require_db_or_skip!();
    cleanup(&pool, KIT_ACCOUNT).await;
    insert_account(&pool, KIT_ACCOUNT, 0).await;

    let clip_size: i32 =
        sqlx::query_scalar("SELECT clip_size FROM resources.items WHERE item_id = $1")
            .bind(STARTER_PISTOL)
            .fetch_one(&pool)
            .await
            .expect("starter pistol is seeded");
    assert!(clip_size > 0, "the starter pistol has a magazine");

    let defs = char_defs(&pool).await;
    assert!(
        defs.len() >= 23,
        "the 23 seeded char_defs at least: {defs:?}"
    );
    for char_def_id in defs {
        let name = format!("Starter Kit {char_def_id:02}");
        let player_id = create(&pool, KIT_ACCOUNT, 0, char_def_id, &name).await;

        let (abilities, bandolier_slot): (Vec<i32>, i32) =
            sqlx::query_as("SELECT abilities, bandolier_slot FROM sgw_player WHERE player_id = $1")
                .bind(player_id)
                .fetch_one(&pool)
                .await
                .expect("read abilities");
        let known: BTreeSet<i32> = abilities.into_iter().collect();
        for id in REQUIRED_STARTERS {
            assert!(
                known.contains(&id),
                "char_def {char_def_id} must start knowing ability {id}; has {known:?}"
            );
        }
        assert_eq!(bandolier_slot, 0, "char_def {char_def_id}: active slot 0");

        let weapon: Option<(i32, i32, i32, bool)> = sqlx::query_as(
            "SELECT i.type_id, i.ammo, i.durability, \
                    i.ammo_type = COALESCE(ri.default_ammo_type, 'AMMO_NONE') \
                      AND i.ammo_types = ri.ammo_types \
             FROM sgw_inventory i JOIN resources.items ri ON ri.item_id = i.type_id \
             WHERE i.character_id = $1 AND i.container_id = $2 AND i.slot_id = 0",
        )
        .bind(player_id)
        .bind(INV_BANDOLIER)
        .fetch_optional(&pool)
        .await
        .expect("read bandolier slot 0");
        let (type_id, ammo, durability, design_ammo) = weapon.unwrap_or_else(|| {
            panic!("char_def {char_def_id}: no pistol in bandolier slot 0 (Pistol Shot = NoAmmo)")
        });
        assert_eq!(
            type_id, STARTER_PISTOL,
            "char_def {char_def_id}: the starter pistol in bandolier slot 0"
        );
        assert_eq!(
            ammo, clip_size,
            "char_def {char_def_id}: the starter pistol is loaded (ammo = clip_size), \
             or the first Pistol Shot is refused with NoAmmo"
        );
        // The row is the one the item-grant path writes (durability 100,
        // the design's ammo types), so the starter pistol repairs and loads
        // like the tutorial's copy of item 55.
        assert_eq!(
            durability, 100,
            "char_def {char_def_id}: granted durability"
        );
        assert!(
            design_ammo,
            "char_def {char_def_id}: ammo_type / ammo_types copied from the design"
        );
    }

    cleanup(&pool, KIT_ACCOUNT).await;
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
