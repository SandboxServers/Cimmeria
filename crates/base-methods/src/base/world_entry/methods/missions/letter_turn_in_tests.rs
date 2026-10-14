use std::sync::Arc;

use sqlx::PgPool;

use super::handle_mission_update;
use super::letter_turn_in::{
    commit_letter_turn_in, commit_with_test_failure, LetterTurnInRefusal, LetterTurnInVerdict,
};
use crate::test_support::require_db_or_skip;

pub(super) const TEST_BASE: i32 = 0x7000_3600;
pub(super) const SESSION: &str = "0123456789abcdef0123456789abcdef";

pub(super) async fn cleanup(pool: &PgPool, id: i32) {
    sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(id)
        .execute(pool)
        .await
        .expect("clean test account");
}

pub(super) async fn player(pool: &PgPool, offset: i32) -> i32 {
    let id = TEST_BASE + offset;
    cleanup(pool, id).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(id)
        .bind(format!("letter-turn-in-{id}"))
        .execute(pool)
        .await
        .expect("insert account");
    sqlx::query(
        "INSERT INTO sgw_player (account_id, player_id, level, alignment, archetype, gender, \
         player_name, extra_name, world_location, bodyset, pos_x, pos_y, pos_z, \
         skin_color_id, naquadah) \
         VALUES ($1, $1, 1, 0, 1, 1, $2, '', 'CombatSim', \
                 'BS_HumanMale.BS_HumanMale', 0.0, 0.0, 0.0, 0, 0)",
    )
    .bind(id)
    .bind(format!("letter-{id}"))
    .execute(pool)
    .await
    .expect("insert player");
    id
}

pub(super) async fn accept_and_advance(pool: &PgPool, id: i32) {
    let db_pool = Some(Arc::new(pool.clone()));
    handle_mission_update(id, 1360, 1, Some(4037), &[], &[], &[4650], &[], 0, &db_pool).await;
    assert_eq!(mission_snapshot(pool, id).await.1, Some(4037));
    handle_mission_update(
        id,
        1360,
        1,
        Some(4038),
        &[4037],
        &[4650],
        &[4651],
        &[],
        0,
        &db_pool,
    )
    .await;
    assert_eq!(mission_snapshot(pool, id).await.1, Some(4038));
}

pub(super) async fn put_item(
    pool: &PgPool,
    player_id: i32,
    type_id: i32,
    container_id: i32,
    slot_id: i32,
    stack_size: i32,
) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, \
         container_id, bound, durability, charges) \
         VALUES ($1, $2, $3, $4, $5, false, 100, 0) RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(stack_size)
    .bind(slot_id)
    .bind(container_id)
    .fetch_one(pool)
    .await
    .expect("insert item")
}

pub(super) type MissionSnapshot = (
    i32,
    Option<i32>,
    Vec<i32>,
    Vec<i32>,
    Vec<i32>,
    Vec<i32>,
    i32,
);

pub(super) async fn mission_snapshot(pool: &PgPool, id: i32) -> MissionSnapshot {
    sqlx::query_as(
        "SELECT status, current_step_id, completed_step_ids, completed_objective_ids, \
         active_objective_ids, failed_objective_ids, repeats \
         FROM sgw_mission WHERE player_id = $1 AND mission_id = 1360",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("mission snapshot")
}

pub(super) async fn receipt_count(pool: &PgPool, id: i32) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM sgw_mission_1360_receipt WHERE player_id = $1 AND mission_id = 1360",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .expect("receipt count")
}

#[tokio::test]
async fn live_db_letter_turn_in_commits_exact_postimage_once() {
    let pool = require_db_or_skip!();
    let id = player(&pool, 1).await;
    accept_and_advance(&pool, id).await;
    let item_id = put_item(&pool, id, 3730, 1, 0, 2).await;

    let first = commit_letter_turn_in(&pool, id, "first-attempt", SESSION)
        .await
        .expect("commit result");
    let LetterTurnInVerdict::Committed(first) = first else {
        panic!("expected first commit, got {first:?}");
    };
    assert_eq!((first.item_id, first.container_id), (item_id, 1));
    assert_eq!((first.stack_before, first.stack_after), (2, 1));
    assert_eq!(first.mission.status, 2);
    assert_eq!(first.mission.current_step_id, None);
    assert_eq!(first.mission.completed_step_ids, [4037, 4038]);
    assert_eq!(first.mission.completed_objective_ids, [4650, 4651]);
    assert_eq!(first.mission.active_objective_ids, [4651]);
    assert_eq!(first.mission.repeats, 1);
    assert_eq!(
        mission_snapshot(&pool, id).await,
        (
            2,
            None,
            vec![4037, 4038],
            vec![4650, 4651],
            vec![4651],
            vec![],
            1
        )
    );
    let stack: i32 = sqlx::query_scalar("SELECT stack_size FROM sgw_inventory WHERE item_id = $1")
        .bind(item_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stack, 1);
    assert_eq!(receipt_count(&pool, id).await, 1);
    let receipt: (String, String, i32, i32, i32, i32) = sqlx::query_as(
        "SELECT attempt_key, session_token, item_id, container_id, stack_before, stack_after \
         FROM sgw_mission_1360_receipt WHERE player_id = $1 AND mission_id = 1360",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        receipt,
        ("first-attempt".into(), SESSION.into(), item_id, 1, 2, 1)
    );

    let retry = commit_letter_turn_in(&pool, id, "first-attempt", SESSION)
        .await
        .expect("retry result");
    assert!(matches!(retry, LetterTurnInVerdict::AlreadyCommitted(_)));
    let changed_attempt = commit_letter_turn_in(&pool, id, "different-attempt", SESSION)
        .await
        .expect("other attempt result");
    assert!(matches!(
        changed_attempt,
        LetterTurnInVerdict::Refused(LetterTurnInRefusal::AlreadyCompleted)
    ));
    let changed_session = commit_letter_turn_in(
        &pool,
        id,
        "first-attempt",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )
    .await
    .expect("other session result");
    assert!(matches!(
        changed_session,
        LetterTurnInVerdict::Refused(LetterTurnInRefusal::AlreadyCompleted)
    ));
    assert_eq!(receipt_count(&pool, id).await, 1);
    let duplicate = sqlx::query(
        "INSERT INTO sgw_mission_1360_receipt \
         (player_id, mission_id, attempt_key, session_token, item_id, container_id, \
          stack_before, stack_after) \
         VALUES ($1, 1360, 'duplicate', $2, $3, 1, 2, 1)",
    )
    .bind(id)
    .bind(SESSION)
    .bind(item_id)
    .execute(&pool)
    .await;
    assert!(
        duplicate.is_err(),
        "one-shot receipt PK must reject another row"
    );
    cleanup(&pool, id).await;
}

#[tokio::test]
async fn live_db_letter_turn_in_refuses_unowned_or_inaccessible_letter() {
    let pool = require_db_or_skip!();
    let id = player(&pool, 10).await;
    let other = player(&pool, 11).await;
    accept_and_advance(&pool, id).await;
    let banked = put_item(&pool, id, 3730, 17, 0, 1).await;
    let zero = put_item(&pool, id, 3730, 1, 0, 0).await;
    let foreign = put_item(&pool, other, 3730, 1, 0, 1).await;
    assert!(matches!(
        commit_letter_turn_in(&pool, id, "bad-token", "ABCDEF0123456789ABCDEF0123456789")
            .await
            .unwrap(),
        LetterTurnInVerdict::Refused(LetterTurnInRefusal::InvalidCorrelation)
    ));
    let result = commit_letter_turn_in(&pool, id, "missing", SESSION)
        .await
        .expect("refusal result");
    assert!(matches!(
        result,
        LetterTurnInVerdict::Refused(LetterTurnInRefusal::NoAccessibleLetter)
    ));
    assert_eq!(mission_snapshot(&pool, id).await.1, Some(4038));
    assert_eq!(receipt_count(&pool, id).await, 0);
    let rows: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT item_id, stack_size FROM sgw_inventory WHERE item_id = ANY($1) ORDER BY item_id",
    )
    .bind(&[banked, zero, foreign][..])
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows, vec![(banked, 1), (zero, 0), (foreign, 1)]);
    cleanup(&pool, id).await;
    cleanup(&pool, other).await;
}

#[tokio::test]
async fn live_db_letter_turn_in_rolls_back_after_item_and_mission_mutation() {
    let pool = require_db_or_skip!();
    for (offset, after_mission) in [(20, false), (21, true)] {
        let id = player(&pool, offset).await;
        accept_and_advance(&pool, id).await;
        let item_id = put_item(&pool, id, 3730, 1, 0, 1).await;
        let before = mission_snapshot(&pool, id).await;
        assert!(
            commit_with_test_failure(&pool, id, "rollback", SESSION, after_mission)
                .await
                .is_err()
        );
        assert_eq!(mission_snapshot(&pool, id).await, before);
        let stack: i32 =
            sqlx::query_scalar("SELECT stack_size FROM sgw_inventory WHERE item_id = $1")
                .bind(item_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stack, 1);
        assert_eq!(receipt_count(&pool, id).await, 0);
        cleanup(&pool, id).await;
    }
}

#[tokio::test]
async fn live_db_letter_turn_in_fences_stale_writers() {
    let pool = require_db_or_skip!();
    let id = player(&pool, 30).await;
    let db_pool = Some(Arc::new(pool.clone()));
    handle_mission_update(
        id,
        1360,
        1,
        Some(4038),
        &[4037],
        &[4650],
        &[4651],
        &[],
        0,
        &db_pool,
    )
    .await;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sgw_mission WHERE player_id = $1 AND mission_id = 1360",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0, "B requires prior A");
    accept_and_advance(&pool, id).await;
    let b = mission_snapshot(&pool, id).await;
    handle_mission_update(id, 1360, 1, Some(4038), &[], &[], &[4651], &[], 0, &db_pool).await;
    handle_mission_update(id, 1360, 1, Some(4037), &[], &[], &[4650], &[], 0, &db_pool).await;
    handle_mission_update(
        id,
        1360,
        2,
        None,
        &[4037, 4038],
        &[4650, 4651],
        &[4651],
        &[],
        1,
        &db_pool,
    )
    .await;
    handle_mission_update(id, 1360, 0, None, &[], &[], &[], &[], 0, &db_pool).await;
    assert_eq!(mission_snapshot(&pool, id).await, b);
    put_item(&pool, id, 3730, 1, 0, 1).await;
    assert!(matches!(
        commit_letter_turn_in(&pool, id, "fence", SESSION)
            .await
            .unwrap(),
        LetterTurnInVerdict::Committed(_)
    ));
    let terminal = mission_snapshot(&pool, id).await;
    handle_mission_update(
        id,
        1360,
        1,
        Some(4038),
        &[4037],
        &[4650],
        &[4651],
        &[],
        0,
        &db_pool,
    )
    .await;
    handle_mission_update(id, 1360, 1, Some(4037), &[], &[], &[4650], &[], 0, &db_pool).await;
    handle_mission_update(id, 1360, 0, None, &[], &[], &[], &[], 0, &db_pool).await;
    assert_eq!(mission_snapshot(&pool, id).await, terminal);
    assert_eq!(receipt_count(&pool, id).await, 1);
    cleanup(&pool, id).await;
}
