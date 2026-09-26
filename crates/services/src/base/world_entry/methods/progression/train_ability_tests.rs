//! Live-DB regression guards for the trainer purchase `UPDATE` (AT-03).
//!
//! The bug shapes: a debit that ignores the node's cost (the pre-AT-03
//! `training_points - 1`), a purchase that never adds to
//! `tree_points_spent` (so no spend gate can ever open), a replayed
//! purchase that debits twice, and a guard failure that moves some of the
//! four fields but not the others.
//!
//! Sentinels: `0x7030_03xx` (AT-03), accounts and players share the id.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use super::tests::{cleanup, insert_test_account, insert_test_player, make_connected_state};
use super::{handle_train_ability, persist_purchase, PurchaseResult, TrainRequest};
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::require_db_or_skip;

const ABILITY: i32 = 597;
const STARTER: i32 = 1646;

/// `(abilities, trained_abilities, training_points, tree_points_spent)`.
type Row = (Vec<i32>, Vec<i32>, i32, i32);

async fn snapshot(pool: &sqlx::PgPool, player_id: i32) -> Row {
    sqlx::query_as(
        "SELECT abilities, trained_abilities, training_points, tree_points_spent \
           FROM sgw_player WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_one(pool)
    .await
    .expect("snapshot sgw_player row")
}

/// A player with `training_points` points, no spend, and `abilities` known
/// (as a starter grant would leave them: not in `trained_abilities`).
async fn setup(pool: &sqlx::PgPool, id: i32, training_points: i32, abilities: &[i32]) {
    cleanup(pool, id).await;
    insert_test_account(pool, id).await;
    insert_test_player(pool, id, id, 0).await;
    sqlx::query("UPDATE sgw_player SET training_points = $1, abilities = $2 WHERE player_id = $3")
        .bind(training_points)
        .bind(abilities)
        .bind(id)
        .execute(pool)
        .await
        .expect("seed points and starter abilities");
}

#[tokio::test]
async fn replayed_purchase_debits_cost_once_and_adds_spend_once() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0301;
    setup(&pool, ID, 5, &[STARTER]).await;

    let first = persist_purchase(&pool, ID, ABILITY, 2).await.unwrap();
    let replay = persist_purchase(&pool, ID, ABILITY, 2).await.unwrap();
    let row = snapshot(&pool, ID).await;
    cleanup(&pool, ID).await;

    assert_eq!(
        first,
        Some(PurchaseResult {
            training_points: 3,
            tree_points_spent: 2,
        }),
        "the first purchase debits the node cost and adds it to the spend"
    );
    assert_eq!(replay, None, "a replayed purchase matches no row");
    assert_eq!(
        row,
        (vec![STARTER, ABILITY], vec![ABILITY], 3, 2),
        "one debit, one spend increment, one append to each array"
    );
}

#[tokio::test]
async fn purchase_short_of_points_leaves_all_four_fields_untouched() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0302;
    setup(&pool, ID, 1, &[STARTER]).await;
    let before = snapshot(&pool, ID).await;

    let result = persist_purchase(&pool, ID, ABILITY, 2).await.unwrap();
    let after = snapshot(&pool, ID).await;
    cleanup(&pool, ID).await;

    assert_eq!(result, None, "cost 2 with 1 point must match no row");
    assert_eq!(after, before, "a failed guard moves no field");
}

/// v2 rule: only trainer purchases count as spend. A starter-granted
/// ability is already known, so buying it is refused, and it never enters
/// `trained_abilities` or `tree_points_spent`.
#[tokio::test]
async fn purchase_of_a_starter_ability_is_refused_and_adds_no_spend() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0303;
    setup(&pool, ID, 5, &[STARTER]).await;
    let before = snapshot(&pool, ID).await;

    let result = persist_purchase(&pool, ID, STARTER, 1).await.unwrap();
    let after = snapshot(&pool, ID).await;
    cleanup(&pool, ID).await;

    assert_eq!(result, None);
    assert_eq!(after, before, "the starter adds no spend and no provenance");
    assert_eq!((after.1, after.3), (Vec::<i32>::new(), 0));
}

async fn run_handler(
    pool: &sqlx::PgPool,
    id: i32,
    in_memory_points: u32,
    cost: i32,
) -> (Option<BaseToCellMsg>, Option<u32>) {
    run_handler_as(pool, id, id, in_memory_points, cost).await
}

/// `session_player` is the character the session is playing; `id` is the
/// one the cell validated.
async fn run_handler_as(
    pool: &sqlx::PgPool,
    id: i32,
    session_player: i32,
    in_memory_points: u32,
    cost: i32,
) -> (Option<BaseToCellMsg>, Option<u32>) {
    let entity_id: u32 = 9_300_300;
    let addr: SocketAddr = "127.0.0.1:65301".parse().unwrap();
    let mut state = make_connected_state(Some(session_player));
    state.player_training_points = Some(in_memory_points);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let (tx, mut rx) = mpsc::channel(4);

    handle_train_ability(
        TrainRequest {
            entity_id,
            player_id: id,
            ability_id: ABILITY,
            cost,
            tree_index: 1,
        },
        &Some(Arc::new(pool.clone())),
        &connected,
        &Some(tx),
        &entity_to_addr,
    )
    .await;

    let points = connected
        .lock()
        .unwrap()
        .get(&addr)
        .and_then(|s| s.player_training_points);
    (rx.try_recv().ok(), points)
}

#[tokio::test]
async fn handler_reports_both_counters_to_the_cell() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0304;
    setup(&pool, ID, 5, &[]).await;

    let (msg, in_memory) = run_handler(&pool, ID, 5, 2).await;
    cleanup(&pool, ID).await;

    match msg {
        Some(BaseToCellMsg::AbilityGranted {
            ability_id,
            training_points,
            tree_points_spent,
            ..
        }) => assert_eq!(
            (ability_id, training_points, tree_points_spent),
            (ABILITY, 3, 2)
        ),
        _ => panic!("expected AbilityGranted"),
    }
    assert_eq!(
        in_memory,
        Some(3),
        "the session's point cache follows the row"
    );
}

#[tokio::test]
async fn handler_refuses_a_negative_cost_without_touching_the_row() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0305;
    setup(&pool, ID, 5, &[]).await;
    let before = snapshot(&pool, ID).await;

    let (msg, _) = run_handler(&pool, ID, 5, -3).await;
    let after = snapshot(&pool, ID).await;
    cleanup(&pool, ID).await;

    assert!(msg.is_none(), "no AbilityGranted for a negative cost");
    assert_eq!(after, before);
}

/// A reused entity id: the session now plays another character. The base
/// must not debit the validated character or report a grant for it.
#[tokio::test]
async fn handler_refuses_when_the_session_plays_another_character() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0306;
    setup(&pool, ID, 5, &[]).await;
    let before = snapshot(&pool, ID).await;

    let (msg, in_memory) = run_handler_as(&pool, ID, ID + 1, 5, 2).await;
    let after = snapshot(&pool, ID).await;
    cleanup(&pool, ID).await;

    assert!(msg.is_none(), "no AbilityGranted for a mismatched session");
    assert_eq!(
        in_memory,
        Some(5),
        "the other character's cache is untouched"
    );
    assert_eq!(after, before);
}
