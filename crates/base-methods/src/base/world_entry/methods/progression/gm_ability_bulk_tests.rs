//! Live-DB guards for the GM bulk ability commands (AB-N2):
//! `gmResetAbilities` (153) and `gmGiveAllAbilities` (154).
//!
//! Bug shapes: a reset that keeps quest or GM grants (it would mirror the
//! trainer respec, which removes only trainer purchases), drops a starter,
//! or forgets the refund; a reset that holds its row lock while waiting for
//! a second pool connection; a give-all that duplicates an id, debits
//! points or marks the grants as trainer-bought; and a write for a
//! character the session no longer plays.
//!
//! Sentinels: `0x7030_0Bxx`. Players and accounts use `0x01..0x0F`; the
//! non-starter ability ids (quest grant, trainer purchases, give-all ids)
//! use `0x40..0x5F`, so no assertion leans on a production seed id. The
//! test character is a Soldier (`archetype = 1`, `insert_test_player`); its
//! starter set is read from `resources.char_creation_abilities` at run time.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use super::gm_ability_bulk::{persist_bulk, starter_abilities, BulkWrite};
use super::handle_gm_ability_bulk;
use super::tests::{cleanup, insert_test_account, insert_test_player, make_connected_state};
use crate::cell::messages::{
    BaseToCellMsg, GmAbilitiesChanged, GmAbilityBulk, GmAbilityChange, GmAbilitySource,
};
use crate::test_support::{database_url, require_db_or_skip, TestTransport};

const GM: u32 = 9_302_001;
/// A quest grant and two trainer purchases, none of them starters.
const QUEST: i32 = 0x7030_0B40;
const TRAINED: [i32; 2] = [0x7030_0B41, 0x7030_0B42];
/// Give-all ids the row does not hold yet.
const NEW_A: i32 = 0x7030_0B50;
const NEW_B: i32 = 0x7030_0B51;

/// `(abilities, trained_abilities, training_points, tree_points_spent)`.
type Row = (Vec<i32>, Vec<i32>, i32, i32);

/// The Soldier's starters, read by enum label rather than through
/// [`starter_abilities`]'s ordinal mapping, so the two check each other.
/// Asserts there are at least two (one test drops one).
async fn soldier_starters(pool: &sqlx::PgPool) -> Vec<i32> {
    let starters: Vec<i32> = sqlx::query_scalar(
        "SELECT DISTINCT ca.ability_id \
           FROM resources.char_creation_abilities ca \
           JOIN resources.char_creation cc USING (char_def_id) \
          WHERE cc.archetype = 'ARCHETYPE_Soldier' \
          ORDER BY 1",
    )
    .fetch_all(pool)
    .await
    .expect("read the Soldier starters");
    assert!(
        starters.len() >= 2,
        "fixture: the seed must give a Soldier 2+ starters"
    );
    for id in [QUEST, TRAINED[0], TRAINED[1], NEW_A, NEW_B] {
        assert!(
            !starters.contains(&id),
            "sentinel {id:#x} collides with a starter"
        );
    }
    starters
}

async fn setup(pool: &sqlx::PgPool, id: i32, abilities: &[i32], trained: &[i32]) {
    cleanup(pool, id).await;
    insert_test_account(pool, id).await;
    insert_test_player(pool, id, id, 5_000).await;
    let r = sqlx::query(
        "UPDATE sgw_player SET abilities = $1, trained_abilities = $2, \
                training_points = 3, tree_points_spent = $3 WHERE player_id = $4",
    )
    .bind(abilities)
    .bind(trained)
    .bind(trained.len() as i32)
    .bind(id)
    .execute(pool)
    .await
    .expect("seed abilities");
    assert_eq!(r.rows_affected(), 1, "fixture row must exist");
}

async fn row(pool: &sqlx::PgPool, id: i32) -> Row {
    sqlx::query_as(
        "SELECT abilities, trained_abilities, training_points, tree_points_spent \
           FROM sgw_player WHERE player_id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("read player row")
}

/// Every starter plus a quest grant plus two trainer purchases.
fn full_kit(starters: &[i32]) -> Vec<i32> {
    let mut v = starters.to_vec();
    v.push(QUEST);
    v.extend(TRAINED);
    v
}

#[tokio::test]
async fn live_db_starter_abilities_are_the_archetypes_char_creation_set() {
    let pool = require_db_or_skip!();
    let starters = soldier_starters(&pool).await;
    assert_eq!(starter_abilities(&pool, 1).await.unwrap(), starters);
    assert!(
        starter_abilities(&pool, 999).await.unwrap().is_empty(),
        "an archetype ordinal with no enum label has no starters"
    );
}

/// **Guard: reset leaves exactly the starters.** The quest grant and the
/// trainer purchases go, the 2 spent points come back (3 + 2), the tree
/// provenance clears. On revert to the respec's `UPDATE` the quest grant
/// survives; on a missing refund `training_points` stays 3.
#[tokio::test]
async fn live_db_gm_reset_leaves_the_starters_and_refunds_the_spend() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0B01;
    let starters = soldier_starters(&pool).await;
    setup(&pool, ID, &full_kit(&starters), &TRAINED).await;

    let write = persist_bulk(&pool, ID, GmAbilityChange::Reset, &[])
        .await
        .unwrap()
        .expect("reset writes");
    assert_eq!(write.after, starters);
    assert_eq!(write.training_points, 5);
    assert_eq!(row(&pool, ID).await, (starters, vec![], 5, 0));
    cleanup(&pool, ID).await;
}

/// **Guard: a starter the row lost comes back.** A character missing its
/// first starter gets it again from the seed, not from its own row.
#[tokio::test]
async fn live_db_gm_reset_restores_a_missing_starter() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0B02;
    let starters = soldier_starters(&pool).await;
    let mut held = starters[1..].to_vec();
    held.push(QUEST);
    setup(&pool, ID, &held, &[]).await;

    persist_bulk(&pool, ID, GmAbilityChange::Reset, &[])
        .await
        .unwrap()
        .expect("reset writes");
    assert_eq!(row(&pool, ID).await.0, starters);
    cleanup(&pool, ID).await;
}

/// **Guard: the reset runs on one connection.** With a one-connection pool,
/// a starter lookup outside the row-locking transaction waits for a second
/// connection that never comes and the reset fails on the acquire timeout;
/// two GMs resetting at once on a busy pool would deadlock it the same way.
#[tokio::test]
async fn live_db_gm_reset_needs_only_one_pool_connection() {
    let _guard_pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0B06;
    let starters = soldier_starters(&_guard_pool).await;
    setup(&_guard_pool, ID, &full_kit(&starters), &TRAINED).await;
    let one = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(2))
        .connect(&database_url().expect("the gate saw a URL"))
        .await
        .expect("connect a one-connection pool");

    let write = persist_bulk(&one, ID, GmAbilityChange::Reset, &[])
        .await
        .expect("the reset must not need a second connection")
        .expect("reset writes");
    assert_eq!(write.after, starters);
    one.close().await;
    cleanup(&_guard_pool, ID).await;
}

/// **Guard: give-all appends only what is missing, in the order sent, and
/// is not a purchase.** The first held id is repeated in the request and a
/// new one twice; neither is duplicated or reordered, and the points, the
/// spend and `trained_abilities` are untouched.
#[tokio::test]
async fn live_db_gm_give_all_appends_missing_ids_without_debiting() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0B03;
    let held = [QUEST, TRAINED[0]];
    setup(&pool, ID, &held, &[TRAINED[0]]).await;

    let BulkWrite { before, after, .. } = persist_bulk(
        &pool,
        ID,
        GmAbilityChange::GrantAll,
        &[NEW_A, QUEST, NEW_B, NEW_A],
    )
    .await
    .unwrap()
    .expect("give-all writes");
    assert_eq!(before, held.to_vec());
    let expected = vec![QUEST, TRAINED[0], NEW_A, NEW_B];
    assert_eq!(after, expected);
    assert_eq!(row(&pool, ID).await, (expected, vec![TRAINED[0]], 3, 1));
    cleanup(&pool, ID).await;
}

fn sessions(
    player: i32,
) -> (
    Arc<TestTransport>,
    Arc<Mutex<HashMap<SocketAddr, super::super::super::super::ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let addr: SocketAddr = "127.0.0.1:65393".parse().unwrap();
    (
        Arc::new(TestTransport::new()),
        Arc::new(Mutex::new(HashMap::from([(
            addr,
            make_connected_state(Some(player)),
        )]))),
        Arc::new(Mutex::new(HashMap::from([(GM, addr)]))),
    )
}

async fn run(pool: &sqlx::PgPool, msg: GmAbilityBulk, session_player: i32) -> Vec<BaseToCellMsg> {
    let (transport, connected, entity_to_addr) = sessions(session_player);
    let transport: Arc<dyn Transport> = transport;
    let (tx, mut rx) = mpsc::channel(4);
    handle_gm_ability_bulk(
        msg,
        &Some(Arc::new(pool.clone())),
        &transport,
        &connected,
        &entity_to_addr,
        &Some(tx),
    )
    .await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn reset_msg(player_id: i32) -> GmAbilityBulk {
    GmAbilityBulk {
        entity_id: GM,
        player_id,
        account_id: None,
        change: GmAbilityChange::Reset,
        source: GmAbilitySource::Command,
        ability_ids: vec![],
    }
}

/// The handler tells the cell what changed: the reset's removals in row
/// order and the refunded points, ready for the one mirror burst.
#[tokio::test]
async fn live_db_gm_reset_handler_reports_the_diff_to_the_cell() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0B04;
    let starters = soldier_starters(&pool).await;
    setup(&pool, ID, &full_kit(&starters), &TRAINED).await;

    let to_cell = run(&pool, reset_msg(ID), ID).await;
    let expected = GmAbilitiesChanged {
        entity_id: GM,
        player_id: ID,
        change: GmAbilityChange::Reset,
        source: GmAbilitySource::Command,
        added: vec![],
        removed: vec![QUEST, TRAINED[0], TRAINED[1]],
        training_points: 5,
    };
    assert!(
        matches!(to_cell.as_slice(), [BaseToCellMsg::GmAbilitiesChanged(c)] if *c == expected),
        "one GmAbilitiesChanged with the diff ({} messages)",
        to_cell.len()
    );
    cleanup(&pool, ID).await;
}

/// **Guard: a recycled entity writes nothing.** The GM's entity now plays
/// another character, so the row and the cell are left alone.
#[tokio::test]
async fn live_db_gm_bulk_refuses_a_session_playing_another_character() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0B05;
    let starters = soldier_starters(&pool).await;
    setup(&pool, ID, &full_kit(&starters), &TRAINED).await;

    let to_cell = run(&pool, reset_msg(ID), ID + 1).await;
    assert!(to_cell.is_empty(), "nothing to the cell");
    assert_eq!(row(&pool, ID).await.0, full_kit(&starters), "row untouched");
    cleanup(&pool, ID).await;
}
