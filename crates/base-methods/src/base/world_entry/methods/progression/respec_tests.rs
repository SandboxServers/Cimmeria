//! Live-DB regression guards for the respec `UPDATE` (AT-08).
//!
//! The bug shapes: a respec that removes starter or quest grants (it must
//! remove `trained_abilities` only), a refund that is not exactly the spend,
//! a replay that charges twice, a short balance that still moves some
//! fields, and a reset whose spend is not zeroed so the root cannot be
//! bought back.
//!
//! Sentinels: `0x7030_08xx` (AT-08), accounts and players share the id.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use super::respec::persist_respec;
use super::tests::{cleanup, insert_test_account, insert_test_player, make_connected_state};
use super::{handle_reset_abilities, persist_purchase, PurchaseResult, RespecRequest};
use crate::ability_tree::{RespecOutcome, RESPEC_COST_NAQUADAH};
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::require_db_or_skip;

/// Starter grant (char creation), never trainer-bought.
const STARTER: i32 = 1646;
/// A content (quest) grant, never trainer-bought.
const QUEST: i32 = 900;
/// Two trainer purchases.
const ROOT: i32 = 597;
const NODE: i32 = 646;

/// `(abilities, trained_abilities, training_points, tree_points_spent, naquadah)`.
type Row = (Vec<i32>, Vec<i32>, i32, i32, i32);

async fn snapshot(pool: &sqlx::PgPool, player_id: i32) -> Row {
    sqlx::query_as(
        "SELECT abilities, trained_abilities, training_points, tree_points_spent, naquadah \
           FROM sgw_player WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_one(pool)
    .await
    .expect("snapshot sgw_player row")
}

/// A character who knows the starter and the quest grant, then bought
/// `ROOT` (cost 1) and `NODE` (cost 2) from a trainer: 3 points spent,
/// `points_left` unspent. The trainer abilities sit between the others in
/// `abilities`, so a respec that dropped the wrong entries or reordered the
/// survivors shows up.
async fn setup_trained(pool: &sqlx::PgPool, id: i32, points_left: i32, naquadah: i32) {
    cleanup(pool, id).await;
    insert_test_account(pool, id).await;
    insert_test_player(pool, id, id, naquadah).await;
    sqlx::query(
        "UPDATE sgw_player \
            SET abilities = $1, trained_abilities = $2, \
                training_points = $3, tree_points_spent = 3 \
          WHERE player_id = $4",
    )
    .bind(vec![STARTER, ROOT, QUEST, NODE])
    .bind(vec![ROOT, NODE])
    .bind(points_left)
    .bind(id)
    .execute(pool)
    .await
    .expect("seed a trained character");
}

#[tokio::test]
async fn live_db_respec_removes_only_trainer_abilities_refunds_the_spend_and_charges_once() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0801;
    // Enough naquadah for two respecs, so only the "something trainer-bought"
    // clause can hold the replay back.
    setup_trained(&pool, ID, 2, 2500).await;

    let first = persist_respec(&pool, ID, RESPEC_COST_NAQUADAH)
        .await
        .unwrap();
    let after_first = snapshot(&pool, ID).await;
    let replay = persist_respec(&pool, ID, RESPEC_COST_NAQUADAH)
        .await
        .unwrap();
    let after_replay = snapshot(&pool, ID).await;
    cleanup(&pool, ID).await;

    assert_eq!(
        first,
        Some(RespecOutcome::Reset {
            refunded: vec![ROOT, NODE],
            training_points: 5,
            naquadah: 1500,
        }),
        "the refund is exactly the 3 points spent; the charge is the price"
    );
    assert_eq!(
        after_first,
        (vec![STARTER, QUEST], vec![], 5, 0, 1500),
        "the starter and quest grants survive in order; provenance and spend reset"
    );
    assert_eq!(
        replay,
        Some(RespecOutcome::NothingToReset),
        "a replay finds nothing trainer-bought"
    );
    assert_eq!(
        after_replay, after_first,
        "a replay is free and moves nothing"
    );
}

#[tokio::test]
async fn live_db_respec_short_of_naquadah_leaves_every_field_untouched() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0802;
    setup_trained(&pool, ID, 2, RESPEC_COST_NAQUADAH - 1).await;
    let before = snapshot(&pool, ID).await;

    let result = persist_respec(&pool, ID, RESPEC_COST_NAQUADAH)
        .await
        .unwrap();
    let after = snapshot(&pool, ID).await;
    cleanup(&pool, ID).await;

    assert_eq!(
        result,
        Some(RespecOutcome::NotEnoughNaquadah {
            naquadah: RESPEC_COST_NAQUADAH - 1
        })
    );
    assert_eq!(after, before, "a failed guard moves no field");
}

/// D-AT11: a character with no trainer purchases (every character created
/// before AT-01) pays nothing and loses nothing.
#[tokio::test]
async fn live_db_respec_with_nothing_trained_is_free() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0803;
    cleanup(&pool, ID).await;
    insert_test_account(&pool, ID).await;
    insert_test_player(&pool, ID, ID, 1500).await;
    sqlx::query("UPDATE sgw_player SET abilities = $1, training_points = 4 WHERE player_id = $2")
        .bind(vec![STARTER, QUEST])
        .bind(ID)
        .execute(&pool)
        .await
        .unwrap();
    let before = snapshot(&pool, ID).await;

    let result = persist_respec(&pool, ID, RESPEC_COST_NAQUADAH)
        .await
        .unwrap();
    let after = snapshot(&pool, ID).await;
    cleanup(&pool, ID).await;

    assert_eq!(result, Some(RespecOutcome::NothingToReset));
    assert_eq!(after, before);
}

/// Cross-task (AT-03): after a respec the spend is 0 and the points are
/// back, so the root the character bought before is buyable again at once.
#[tokio::test]
async fn live_db_respecced_character_can_rebuy_the_root_immediately() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0804;
    setup_trained(&pool, ID, 0, 1500).await;

    let blocked = persist_purchase(&pool, ID, ROOT, 1).await.unwrap();
    persist_respec(&pool, ID, RESPEC_COST_NAQUADAH)
        .await
        .unwrap();
    let rebuy = persist_purchase(&pool, ID, ROOT, 1).await.unwrap();
    let row = snapshot(&pool, ID).await;
    cleanup(&pool, ID).await;

    assert_eq!(blocked, None, "before the respec the root is already known");
    assert_eq!(
        rebuy,
        Some(PurchaseResult {
            training_points: 2,
            tree_points_spent: 1,
        }),
        "the spend restarts from 0 and the refunded points pay for the root"
    );
    assert_eq!(row, (vec![STARTER, QUEST, ROOT], vec![ROOT], 2, 1, 500));
}

async fn run_handler(
    pool: &sqlx::PgPool,
    id: i32,
    session_player: i32,
    cost: i32,
) -> (Option<BaseToCellMsg>, Option<u32>) {
    let entity_id: u32 = 9_300_800;
    let addr: SocketAddr = "127.0.0.1:65308".parse().unwrap();
    let mut state = make_connected_state(Some(session_player));
    state.player_training_points = Some(2);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let (tx, mut rx) = mpsc::channel(4);

    handle_reset_abilities(
        RespecRequest {
            entity_id,
            player_id: id,
            cost,
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
async fn live_db_handler_reports_the_reset_and_refreshes_the_point_cache() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0805;
    setup_trained(&pool, ID, 2, 1500).await;

    let (msg, in_memory) = run_handler(&pool, ID, ID, RESPEC_COST_NAQUADAH).await;
    cleanup(&pool, ID).await;

    match msg {
        Some(BaseToCellMsg::AbilitiesReset { outcome, .. }) => assert_eq!(
            outcome,
            RespecOutcome::Reset {
                refunded: vec![ROOT, NODE],
                training_points: 5,
                naquadah: 500,
            }
        ),
        _ => panic!("expected AbilitiesReset"),
    }
    assert_eq!(
        in_memory,
        Some(5),
        "the session's point cache follows the row"
    );
}

/// Too little naquadah: the cell still hears about it, so the player gets
/// feedback on the first press.
#[tokio::test]
async fn live_db_handler_reports_a_short_balance_to_the_cell() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0806;
    setup_trained(&pool, ID, 2, 10).await;

    let (msg, in_memory) = run_handler(&pool, ID, ID, RESPEC_COST_NAQUADAH).await;
    cleanup(&pool, ID).await;

    match msg {
        Some(BaseToCellMsg::AbilitiesReset { outcome, .. }) => {
            assert_eq!(outcome, RespecOutcome::NotEnoughNaquadah { naquadah: 10 })
        }
        _ => panic!("expected AbilitiesReset"),
    }
    assert_eq!(in_memory, Some(2), "nothing refunded, cache untouched");
}

/// A reused entity id: the session now plays another character. The base
/// must not reset the validated one.
#[tokio::test]
async fn live_db_handler_refuses_when_the_session_plays_another_character() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0807;
    setup_trained(&pool, ID, 2, 1500).await;
    let before = snapshot(&pool, ID).await;

    let (msg, _) = run_handler(&pool, ID, ID + 1, RESPEC_COST_NAQUADAH).await;
    let after = snapshot(&pool, ID).await;
    cleanup(&pool, ID).await;

    assert!(msg.is_none(), "no AbilitiesReset for a mismatched session");
    assert_eq!(after, before);
}

#[tokio::test]
async fn live_db_handler_refuses_a_negative_cost_without_touching_the_row() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0808;
    setup_trained(&pool, ID, 2, 1500).await;
    let before = snapshot(&pool, ID).await;

    let (msg, _) = run_handler(&pool, ID, ID, -1000).await;
    let after = snapshot(&pool, ID).await;
    cleanup(&pool, ID).await;

    assert!(msg.is_none());
    assert_eq!(after, before, "a negative cost must not pay the player");
}
