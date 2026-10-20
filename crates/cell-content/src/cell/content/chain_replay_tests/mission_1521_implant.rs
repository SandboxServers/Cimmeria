//! Mission 1521 implant item (CS-03d, OD-1 Option A): chain 1386 grants the
//! Immature Goa'uld Symbiote (5169) into container 2 once, on the
//! `mission_completed 1521` replay of a real completion in world 8. The
//! rows are the seeded ones in `castle_symbiote_chains.sql`.
//!
//! The cell sends the grant and cannot see the container, so the full-bag
//! mail fallback is base-side and is not pinned here.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::missions::{MissionInstance, MISSION_COMPLETED};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::event_dispatch::fire_mission_completed;
use super::super::fire_dialog_choice;
use super::mission_1521_entry::{
    completed_704, engine_with, mission_at, mission_of, staged, CASTLE, ENTITY, MISSION, PLAYER,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::require_db_or_skip;

const STEP_CLOSE: i32 = 4458;
const OBJECTIVE_CLOSE: i32 = 5178;
const IMPLANT: i32 = 5169;
const CONTAINER: i32 = 2;

/// The messages that matter here, in the order they were sent.
#[derive(Debug, PartialEq, Eq)]
enum Sent {
    Completed,
    Grant {
        item: i32,
        container: i32,
        count: i32,
    },
    Other,
}

/// The one grant chain 1386 is meant to make.
fn implant() -> Sent {
    Sent::Grant {
        item: IMPLANT,
        container: CONTAINER,
        count: 1,
    }
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<Sent> {
    let mut sent = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        sent.push(match msg {
            CellToBaseMsg::GrantItem {
                item_id,
                container_id,
                count,
                ..
            } => Sent::Grant {
                item: item_id,
                container: container_id,
                count,
            },
            CellToBaseMsg::MissionUpdate { status, .. } if status == MISSION_COMPLETED => {
                Sent::Completed
            }
            _ => Sent::Other,
        });
    }
    sent
}

/// Mission 1521 already completed, the state the completion replay sees.
fn completed_1521() -> MissionInstance {
    let mut mission = mission_at(MISSION, None, &[]);
    mission.status = MISSION_COMPLETED;
    mission
}

/// 1521 active at the close step 4458, holding objective 5178.
async fn at_step_close(pool: &PgPool) -> SpaceManager {
    staged(
        pool,
        CASTLE,
        Some(8),
        vec![
            completed_704(),
            mission_at(MISSION, Some(STEP_CLOSE), &[OBJECTIVE_CLOSE]),
        ],
    )
    .await
}

/// Replay `mission_completed` for `mission` through the real dispatcher.
async fn replay_completion(
    engine: &ChainEngine,
    mgr: &mut SpaceManager,
    mission: i32,
) -> Vec<Sent> {
    let (tx, mut rx) = mpsc::channel(64);
    fire_mission_completed(ENTITY, PLAYER, mission, engine, &tx, mgr).await;
    drain(&mut rx)
}

/// Tank-side close of dialog 4991, which runs chain 1377 and its completion.
async fn close_4991(engine: &ChainEngine, mgr: &mut SpaceManager) -> Vec<Sent> {
    let (tx, mut rx) = mpsc::channel(64);
    fire_dialog_choice(ENTITY, PLAYER, 4991, -1, engine, &tx, mgr).await;
    drain(&mut rx)
}

/// **Chain 1386 is seeded as the Option A row: one enabled mission-1521
/// chain, one `mission_completed '1521'` trigger, the world and status
/// conditions, and one `add_item 5169` into container 2.**
#[tokio::test]
async fn live_db_chain_1386_loads_with_the_implant_grant() {
    let pool = require_db_or_skip!();
    let chain = load_single_chain_for_test(&pool, 1386)
        .await
        .expect("DB query for chain 1386 must succeed")
        .expect("chain 1386 must exist in seeded content_chains");
    assert!(
        matches!(
            chain.actions.as_slice(),
            [Action::GrantItem {
                item_id: IMPLANT,
                container_id: Some(CONTAINER),
                ..
            }]
        ),
        "{:?}",
        chain.actions
    );

    let rows = sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
        "SELECT \
         (SELECT COUNT(*) FROM resources.content_chains WHERE chain_id = 1386 AND enabled \
          AND scope_type = 'mission' AND scope_id = 1521), \
         (SELECT COUNT(*) FROM resources.content_triggers WHERE chain_id = 1386 \
          AND event_type = 'mission_completed' AND event_key = '1521'), \
         (SELECT COUNT(*) FROM resources.content_conditions WHERE chain_id = 1386), \
         (SELECT COUNT(*) FROM resources.content_actions WHERE chain_id = 1386), \
         (SELECT COUNT(*) FROM resources.content_actions WHERE chain_id = 1386 AND delay_ms <> 0)",
    )
    .fetch_one(&pool)
    .await
    .expect("chain 1386 row counts");
    assert_eq!(
        rows,
        (1, 1, 2, 1, 0),
        "chain 1386 rows (last: delayed actions)"
    );

    let conditions = sqlx::query_as::<_, (String, i32, Option<String>, String, Option<String>)>(
        "SELECT condition_type, target_id, target_key, operator, value \
         FROM resources.content_conditions WHERE chain_id = 1386 ORDER BY sort_order",
    )
    .fetch_all(&pool)
    .await
    .expect("chain 1386 conditions");
    assert_eq!(
        conditions,
        vec![
            ("world".to_string(), 8, None, "eq".to_string(), None),
            (
                "mission_status".to_string(),
                1521,
                None,
                "eq".to_string(),
                Some("completed".to_string())
            ),
        ],
        "chain 1386 conditions, in order"
    );
}

/// **The `mission_completed 1521` replay in world 8 grants 5169 once, into
/// container 2, and nothing else.**
#[tokio::test]
async fn live_db_chain_1386_grants_5169_once_on_completion() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1386]).await;
    let mut mgr = staged(
        &pool,
        CASTLE,
        Some(8),
        vec![completed_704(), completed_1521()],
    )
    .await;
    let sent = replay_completion(&engine, &mut mgr, MISSION).await;
    assert_eq!(sent, vec![implant()], "{sent:?}");
}

/// **Chain 1386 grants nothing in another world, for another mission, or
/// while 1521 is still active.** Each case removes one of its guards.
#[tokio::test]
async fn live_db_chain_1386_grants_nothing_for_the_wrong_world_or_mission_or_state() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1386]).await;

    let mut other_world = staged(
        &pool,
        "Castle_CellBlock",
        Some(8),
        vec![completed_704(), completed_1521()],
    )
    .await;
    assert_eq!(
        replay_completion(&engine, &mut other_world, MISSION).await,
        Vec::new(),
        "1521 completed outside world 8"
    );

    let mut completed = staged(
        &pool,
        CASTLE,
        Some(8),
        vec![completed_704(), completed_1521()],
    )
    .await;
    for other in [1520, 1522] {
        assert_eq!(
            replay_completion(&engine, &mut completed, other).await,
            Vec::new(),
            "mission_completed {other} must not reach the 1521 chain"
        );
    }

    let mut active = at_step_close(&pool).await;
    assert_eq!(
        replay_completion(&engine, &mut active, MISSION).await,
        Vec::new(),
        "1521 still active"
    );
}

/// **Dialog 4991's close runs 1377 and then 1386: one grant after the
/// completion, and a second close on the completed mission grants nothing.**
#[tokio::test]
async fn live_db_chain_1386_follows_the_1377_close_and_never_reopens_1521() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1377, 1386]).await;
    let mut mgr = at_step_close(&pool).await;

    let sent = close_4991(&engine, &mut mgr).await;
    assert_eq!(
        mission_of(&mgr, MISSION).map(|m| m.status),
        Some(MISSION_COMPLETED),
        "1377 must complete 1521"
    );
    let done = sent.iter().position(|s| *s == Sent::Completed);
    let grant = sent.iter().position(|s| matches!(s, Sent::Grant { .. }));
    assert!(
        matches!((done, grant), (Some(d), Some(g)) if d < g),
        "the grant must follow the completion: {sent:?}"
    );
    assert_eq!(
        sent.iter()
            .filter(|s| matches!(s, Sent::Grant { .. }))
            .count(),
        1,
        "{sent:?}"
    );

    let again = close_4991(&engine, &mut mgr).await;
    assert!(
        !again.iter().any(|s| matches!(s, Sent::Grant { .. })),
        "a second close must not grant again: {again:?}"
    );
}

/// **Documents the stack-1 edge: a second completion event (reachable only
/// after a GM reset) grants a second copy, because the server does not merge
/// or refuse a unique item.**
#[tokio::test]
async fn live_db_chain_1386_second_completion_event_grants_a_second_copy() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &[1386]).await;
    let mut mgr = staged(
        &pool,
        CASTLE,
        Some(8),
        vec![completed_704(), completed_1521()],
    )
    .await;
    assert_eq!(
        replay_completion(&engine, &mut mgr, MISSION).await,
        vec![implant()]
    );
    assert_eq!(
        replay_completion(&engine, &mut mgr, MISSION).await,
        vec![implant()],
        "a stack-1 item is not merged"
    );
}
