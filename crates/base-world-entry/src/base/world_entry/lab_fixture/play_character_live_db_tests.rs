//! Live-DB guard for the Play hook (lab-fixtures FX-02): the staged
//! fixture is written before `handle_play_character` reads the row.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cimmeria_base_session::base::world_entry::lab_fixture::{
    fixture_pending, stage_fixture, GM_ACCESS_LEVEL,
};
use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio::time::timeout;

use super::tests::{
    active, cleanup, fixture, mission_row, place, seed_player, steps_of, MISSION, PLAY_GM,
    PLAY_PLAYER, START,
};
use crate::base::world_entry::handle_play_character;
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::{require_db_or_skip, test_default_connected_client_state, TestTransport};

/// Run `handle_play_character` for `account_id`'s character at
/// `access_level`, answering the cell's `CreateEntity`. Returns the
/// position the cell was asked to create the player at and the session's
/// pending world entry position.
async fn play(pool: &PgPool, account_id: i32, access_level: u32) -> ([f32; 3], [f32; 3]) {
    let player_id = account_id + 1;
    let addr: SocketAddr = "127.0.0.1:55670".parse().expect("addr");
    let mut state = test_default_connected_client_state();
    state.account_id = account_id as u32;
    state.access_level = access_level;
    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
        Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let (cell_tx, mut cell_rx) = mpsc::channel(4);
    let task_connected = Arc::clone(&connected);
    let db = Some(Arc::new(pool.clone()));
    let handle = tokio::spawn(async move {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let mgr = Arc::new(Mutex::new(EntityManager::new()));
        handle_play_character(
            &transport,
            addr,
            [0u8; 32],
            account_id as u32,
            player_id,
            &task_connected,
            &db,
            &mgr,
            &Some(cell_tx),
        )
        .await
        .map_err(|e| e.to_string())
    });
    let msg = timeout(Duration::from_secs(5), cell_rx.recv())
        .await
        .expect("CreateEntity must not hang")
        .expect("CreateEntity expected");
    let BaseToCellMsg::CreateEntity {
        position, reply_tx, ..
    } = msg
    else {
        panic!("expected CreateEntity");
    };
    let _ = reply_tx.send(1);
    timeout(Duration::from_secs(5), handle)
        .await
        .expect("play must not hang")
        .expect("play task")
        .expect("play ok");
    let pending = connected
        .lock()
        .expect("connected")
        .get(&addr)
        .and_then(|c| c.pending_world_entry.as_ref().map(|w| w.pos))
        .expect("pending world entry");
    (position, pending)
}

/// **Guard: the Play hook writes the fixture before world entry reads
/// the row.** A GM's Play creates the player at the fixture's position
/// and saves its mission; a player's Play consumes the fixture and
/// writes nothing. Move the hook after `query_world_entry` and the GM's
/// entry comes up at the old position.
#[tokio::test]
async fn live_db_play_character_applies_a_staged_fixture_before_world_entry() {
    let pool = require_db_or_skip!();
    let gm_player = seed_player(&pool, PLAY_GM).await;
    let player = seed_player(&pool, PLAY_PLAYER).await;
    let steps = steps_of(&pool, MISSION).await;
    let target = [-319.0, 73.5, -212.5];
    for player_id in [gm_player, player] {
        let mut f = fixture(player_id, vec![active(MISSION, steps[1])]);
        f.position = Some(target);
        stage_fixture(f);
    }

    let (gm_created, gm_pending) = play(&pool, PLAY_GM, GM_ACCESS_LEVEL).await;
    let gm_mission = mission_row(&pool, gm_player, MISSION).await;
    let gm_pending_fixture = fixture_pending(gm_player);

    let (created, pending) = play(&pool, PLAY_PLAYER, 0).await;
    let mission = mission_row(&pool, player, MISSION).await;
    let player_place = place(&pool, player).await;
    let pending_fixture = fixture_pending(player);
    cleanup(&pool, PLAY_GM).await;
    cleanup(&pool, PLAY_PLAYER).await;

    assert_eq!(gm_created, target, "the cell creates the GM at the fixture");
    assert_eq!(
        gm_pending, target,
        "the session's world entry is the fixture's"
    );
    assert_eq!(
        gm_mission.map(|m| (m.status, m.current_step_id)),
        Some((1, Some(steps[1])))
    );
    assert!(!gm_pending_fixture, "the fixture is taken");

    assert_eq!(created, START, "a player's Play ignores the fixture");
    assert_eq!(pending, START);
    assert_eq!(mission, None, "nothing is written for a player");
    assert_eq!(
        [player_place.pos_x, player_place.pos_y, player_place.pos_z],
        START
    );
    assert!(!pending_fixture, "a refused fixture is still consumed");
}
