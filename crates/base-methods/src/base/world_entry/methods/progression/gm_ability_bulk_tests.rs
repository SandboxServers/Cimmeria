//! Live-DB guards for the GM bulk ability commands (AB-N2):
//! `gmResetAbilities` (153) and `gmGiveAllAbilities` (154).
//!
//! Bug shapes: a reset that keeps quest or GM grants (it would mirror the
//! trainer respec, which removes only trainer purchases), drops a starter,
//! or forgets the refund; a give-all that duplicates an id, debits points or
//! marks the grants as trainer-bought; and a write for a character the
//! session no longer plays.
//!
//! Sentinels: `0x7030_0Bxx`, accounts and players share the id. The test
//! character is a Soldier (`archetype = 1`, `insert_test_player`); every
//! seeded CharDef grants 592, 594, 597, 1218 and 1646.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use super::gm_ability_bulk::{persist_bulk, starter_abilities, BulkWrite};
use super::handle_gm_ability_bulk;
use super::tests::{cleanup, insert_test_account, insert_test_player, make_connected_state};
use crate::cell::messages::{BaseToCellMsg, GmAbilitiesChanged, GmAbilityBulk, GmAbilityChange};
use crate::test_support::{require_db_or_skip, TestTransport};

const GM: u32 = 9_302_001;
const STARTERS: [i32; 5] = [592, 594, 597, 1218, 1646];
/// A quest grant and two trainer purchases, none of them starters.
const QUEST: i32 = 2826;
const TRAINED: [i32; 2] = [1643, 1644];

/// `(abilities, trained_abilities, training_points, tree_points_spent)`.
type Row = (Vec<i32>, Vec<i32>, i32, i32);

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
fn full_kit() -> Vec<i32> {
    let mut v = STARTERS.to_vec();
    v.push(QUEST);
    v.extend(TRAINED);
    v
}

#[tokio::test]
async fn live_db_starter_abilities_are_the_archetypes_char_creation_set() {
    let pool = require_db_or_skip!();
    assert_eq!(
        starter_abilities(&pool, 1).await.unwrap(),
        STARTERS.to_vec()
    );
    assert!(
        starter_abilities(&pool, 999).await.unwrap().is_empty(),
        "an archetype with no CharDef has no starters"
    );
}

/// **Guard: reset leaves exactly the starters.** The quest grant and the
/// trainer purchases go, the 2 spent points come back (3 + 2), the tree
/// provenance clears. On revert to the respec's `UPDATE` the quest grant
/// 2826 survives; on a missing refund `training_points` stays 3.
#[tokio::test]
async fn live_db_gm_reset_leaves_the_starters_and_refunds_the_spend() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0B01;
    setup(&pool, ID, &full_kit(), &TRAINED).await;

    let write = persist_bulk(&pool, ID, GmAbilityChange::Reset, &[])
        .await
        .unwrap()
        .expect("reset writes");
    assert_eq!(write.after, STARTERS.to_vec());
    assert_eq!(write.training_points, 5);
    assert_eq!(row(&pool, ID).await, (STARTERS.to_vec(), vec![], 5, 0));
    cleanup(&pool, ID).await;
}

/// **Guard: a starter the row lost comes back.** A character missing 597
/// gets it again from the seed, not from its own row.
#[tokio::test]
async fn live_db_gm_reset_restores_a_missing_starter() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0B02;
    setup(&pool, ID, &[592, 594, 1218, 1646, QUEST], &[]).await;

    persist_bulk(&pool, ID, GmAbilityChange::Reset, &[])
        .await
        .unwrap()
        .expect("reset writes");
    assert_eq!(row(&pool, ID).await.0, STARTERS.to_vec());
    cleanup(&pool, ID).await;
}

/// **Guard: give-all appends only what is missing, in the order sent, and
/// is not a purchase.** 592 is already known and repeated in the request;
/// it is neither duplicated nor reordered, and the points, the spend and
/// `trained_abilities` are untouched.
#[tokio::test]
async fn live_db_gm_give_all_appends_missing_ids_without_debiting() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0B03;
    setup(&pool, ID, &[592, 1643], &[1643]).await;

    let BulkWrite { before, after, .. } =
        persist_bulk(&pool, ID, GmAbilityChange::GrantAll, &[700, 592, 701, 700])
            .await
            .unwrap()
            .expect("give-all writes");
    assert_eq!(before, vec![592, 1643]);
    assert_eq!(after, vec![592, 1643, 700, 701]);
    assert_eq!(
        row(&pool, ID).await,
        (vec![592, 1643, 700, 701], vec![1643], 3, 1)
    );
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

/// The handler tells the cell what changed: the reset's removals in row
/// order and the refunded points, ready for the one mirror burst.
#[tokio::test]
async fn live_db_gm_reset_handler_reports_the_diff_to_the_cell() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0B04;
    setup(&pool, ID, &full_kit(), &TRAINED).await;
    let msg = GmAbilityBulk {
        entity_id: GM,
        player_id: ID,
        account_id: None,
        change: GmAbilityChange::Reset,
        ability_ids: vec![],
    };

    let to_cell = run(&pool, msg, ID).await;
    let expected = GmAbilitiesChanged {
        entity_id: GM,
        player_id: ID,
        change: GmAbilityChange::Reset,
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
    setup(&pool, ID, &full_kit(), &TRAINED).await;
    let msg = GmAbilityBulk {
        entity_id: GM,
        player_id: ID,
        account_id: None,
        change: GmAbilityChange::Reset,
        ability_ids: vec![],
    };

    let to_cell = run(&pool, msg, ID + 1).await;
    assert!(to_cell.is_empty(), "nothing to the cell");
    assert_eq!(row(&pool, ID).await.0, full_kit(), "row untouched");
    cleanup(&pool, ID).await;
}
