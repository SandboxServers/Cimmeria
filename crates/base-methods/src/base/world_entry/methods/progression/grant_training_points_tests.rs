//! Live-DB guards for the `gmGiveTrainingPoints` grant.
//!
//! Bug shapes: a grant that never reaches the row; a grant the next level-up
//! erases (`handle_grant_xp` writes `training_points` as an absolute value
//! from the session cache); an add past `i32::MAX`; and a non-positive or
//! misdirected grant that moves the row anyway.
//!
//! Sentinels: `0x7030_09xx`, accounts and players share the id.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use tokio::sync::mpsc;

use super::grant_training_points::persist_training_points_grant;
use super::tests::{cleanup, insert_test_account, insert_test_player, make_connected_state};
use super::{handle_grant_training_points, handle_grant_xp, TrainingPointsGrant};
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{require_db_or_skip, TestTransport};

const ENTITY: u32 = 9_300_900;

async fn setup(pool: &sqlx::PgPool, id: i32, training_points: i32) {
    cleanup(pool, id).await;
    insert_test_account(pool, id).await;
    insert_test_player(pool, id, id, 0).await;
    let r = sqlx::query("UPDATE sgw_player SET training_points = $1 WHERE player_id = $2")
        .bind(training_points)
        .bind(id)
        .execute(pool)
        .await
        .expect("seed training points");
    assert_eq!(r.rows_affected(), 1, "fixture row must exist");
}

async fn persisted(pool: &sqlx::PgPool, id: i32) -> i32 {
    sqlx::query_scalar("SELECT training_points FROM sgw_player WHERE player_id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("read training_points")
}

/// One session playing `session_player`, with a cached point count.
struct Session {
    addr: SocketAddr,
    transport: Arc<TestTransport>,
    connected: Arc<Mutex<HashMap<SocketAddr, super::ConnectedClientState>>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

impl Session {
    fn new(session_player: i32, cached_points: u32) -> Self {
        let addr: SocketAddr = "127.0.0.1:65390".parse().unwrap();
        let mut state = make_connected_state(Some(session_player));
        state.player_training_points = Some(cached_points);
        Self {
            addr,
            transport: Arc::new(TestTransport::new()),
            connected: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            entity_to_addr: Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)]))),
        }
    }

    fn cached_points(&self) -> Option<u32> {
        self.connected
            .lock()
            .unwrap()
            .get(&self.addr)
            .and_then(|s| s.player_training_points)
    }

    /// Run the handler; return what it sent the cell.
    async fn grant(&self, pool: &sqlx::PgPool, player_id: i32, amount: i32) -> Vec<BaseToCellMsg> {
        let transport: Arc<dyn Transport> = self.transport.clone();
        let (tx, mut rx) = mpsc::channel(4);
        handle_grant_training_points(
            TrainingPointsGrant {
                entity_id: ENTITY,
                player_id,
                amount,
                gm_feedback_to: Some(ENTITY),
            },
            &Some(Arc::new(pool.clone())),
            &transport,
            &self.connected,
            &self.entity_to_addr,
            &Some(tx),
        )
        .await;
        std::iter::from_fn(|| rx.try_recv().ok()).collect()
    }
}

fn granted(msgs: &[BaseToCellMsg]) -> Vec<(u32, i32)> {
    msgs.iter()
        .filter_map(|m| match m {
            BaseToCellMsg::TrainingPointsGranted {
                entity_id,
                training_points,
            } => Some((*entity_id, *training_points)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn grant_persists_and_tells_the_cell_and_the_gm() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0901;
    setup(&pool, ID, 3).await;
    let session = Session::new(ID, 3);

    let to_cell = session.grant(&pool, ID, 5).await;

    assert_eq!(persisted(&pool, ID).await, 8, "3 + 5 persisted");
    assert_eq!(
        granted(&to_cell),
        vec![(ENTITY, 8)],
        "exactly one TrainingPointsGranted, carrying the RETURNING total"
    );
    assert_eq!(to_cell.len(), 1, "nothing else to the cell");
    assert_eq!(session.cached_points(), Some(8), "session cache refreshed");
    assert_eq!(
        session.transport.send_count_to(session.addr),
        1,
        "the GM gets the feedback line"
    );
    cleanup(&pool, ID).await;
}

/// `handle_grant_xp` writes `training_points = cache + levels`. Without the
/// cache refresh the first level-up after a GM grant writes the pre-grant
/// value back and the granted points vanish.
#[tokio::test]
async fn a_later_level_up_keeps_the_granted_points() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0902;
    setup(&pool, ID, 0).await;
    let session = Session::new(ID, 0);

    session.grant(&pool, ID, 5).await;
    let transport: Arc<dyn Transport> = session.transport.clone();
    handle_grant_xp(
        ENTITY,
        1_000,
        None,
        &Some(Arc::new(pool.clone())),
        &transport,
        &session.connected,
        &session.entity_to_addr,
        &None,
    )
    .await;

    let level: i32 = sqlx::query_scalar("SELECT level FROM sgw_player WHERE player_id = $1")
        .bind(ID)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(level > 1, "test invariant: the XP grant levelled up");
    assert_eq!(
        persisted(&pool, ID).await,
        5 + (level - 1),
        "the 5 granted points survive the level-up (one point per level)"
    );
    cleanup(&pool, ID).await;
}

/// The unguarded add raises a Postgres `integer out of range` error, so the
/// guard's job is to hold the row back cleanly (`Ok(None)`) instead.
#[tokio::test]
async fn grant_past_i32_max_is_held_back_and_changes_nothing() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0903;
    setup(&pool, ID, i32::MAX - 2).await;

    assert_eq!(
        persist_training_points_grant(&pool, ID, 3)
            .await
            .expect("the guard must hold the row back, not raise an overflow"),
        None
    );
    assert_eq!(
        persist_training_points_grant(&pool, ID, 2).await.unwrap(),
        Some(i32::MAX),
        "exactly reaching i32::MAX is allowed"
    );

    // Through the handler: refused, nothing to the cell, cache untouched,
    // and the GM still hears about it.
    let session = Session::new(ID, 7);
    let to_cell = session.grant(&pool, ID, 1).await;
    assert_eq!(persisted(&pool, ID).await, i32::MAX);
    assert!(to_cell.is_empty(), "a refused grant tells the cell nothing");
    assert_eq!(session.cached_points(), Some(7));
    assert_eq!(
        session.transport.send_count_to(session.addr),
        1,
        "the refusal is answered"
    );
    cleanup(&pool, ID).await;
}

#[tokio::test]
async fn non_positive_amount_is_refused_at_the_base_and_changes_nothing() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0904;
    setup(&pool, ID, 4).await;
    let session = Session::new(ID, 4);

    for amount in [0, -3] {
        let to_cell = session.grant(&pool, ID, amount).await;
        assert!(to_cell.is_empty(), "amount {amount} tells the cell nothing");
    }
    assert_eq!(persisted(&pool, ID).await, 4, "the row never moved");
    assert_eq!(session.cached_points(), Some(4));
    assert_eq!(
        session.transport.send_count_to(session.addr),
        2,
        "each refusal is answered"
    );
    cleanup(&pool, ID).await;
}

/// The session moved on to another character between the cell's send and
/// the base: the resolved character must not be credited.
#[tokio::test]
async fn grant_for_a_character_the_session_no_longer_plays_is_refused() {
    let pool = require_db_or_skip!();
    const ID: i32 = 0x7030_0905;
    setup(&pool, ID, 1).await;
    let session = Session::new(ID + 1, 1);

    let to_cell = session.grant(&pool, ID, 5).await;

    assert_eq!(persisted(&pool, ID).await, 1);
    assert!(to_cell.is_empty());
    assert_eq!(session.cached_points(), Some(1));
    cleanup(&pool, ID).await;
}
