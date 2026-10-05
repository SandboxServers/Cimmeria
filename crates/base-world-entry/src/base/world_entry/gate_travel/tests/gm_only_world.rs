//! D-DA4: a non-GM's cross-world transfer into a GM-only world (the Debug
//! Area) arrives at the faction start instead, and is told why on arrival;
//! a GM's goes through. Every cross-world route (GM `.summon` and
//! `.gotolocation`, content teleports, rings, gates, a respawner in another
//! world) reaches the cell through `handle_gate_travel`.
//!
//! Revert proof: drop the `gm_only_gate_redirect` call and the player's
//! `CreateEntity` names `DebugArea`.

use super::*;
use crate::cell::messages::BaseToCellMsg;
use cimmeria_base_session::base::world_entry::gm_only_worlds::{take_redirect_line, REDIRECT_LINE};
use std::time::Duration;
use tokio::time::timeout;

const ENTITY_ID: u32 = 43;

/// One trip to `DebugArea` at `access_level`; returns the world and
/// position the cell was asked to create the entity in.
async fn travel_to_debug_area(
    access_level: u32,
    alignment: Option<i32>,
    port: u16,
) -> (String, [f32; 3], i32) {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    let mut state = make_state();
    state.access_level = access_level;
    state.player_alignment = alignment;
    // A player id per test: the owed-line store is process-wide.
    state.active_player_id = Some(i32::from(port));
    let player_id = i32::from(port);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY_ID, addr)])));
    let transport: Arc<dyn Transport> = Arc::new(crate::test_support::TestTransport::new());
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(8);
    let task = tokio::spawn(async move {
        handle_gate_travel(
            ENTITY_ID,
            "DebugArea",
            [251.0, 8.0, -962.0],
            [0.0; 3],
            None,
            None,
            &transport,
            &connected,
            &entity_to_addr,
            &Some(cell_tx),
            &None,
        )
        .await
    });
    let msg = timeout(Duration::from_secs(2), cell_rx.recv())
        .await
        .expect("CreateEntity must not hang")
        .expect("CreateEntity expected");
    let BaseToCellMsg::CreateEntity {
        world_name,
        position,
        reply_tx,
        ..
    } = msg
    else {
        panic!("expected CreateEntity as the first base->cell message");
    };
    let _ = reply_tx.send(0x0001_0001);
    timeout(Duration::from_secs(2), task)
        .await
        .expect("gate travel must not hang")
        .unwrap()
        .expect("gate travel completes");
    (world_name, position, player_id)
}

#[tokio::test]
async fn a_player_bound_for_the_debug_area_arrives_at_the_faction_start() {
    let (world, position, player_id) = travel_to_debug_area(0, None, 55796).await;
    assert_eq!(world, "Castle_CellBlock", "no database: Praxis start");
    assert_eq!(position, [-334.231, 73.472, -228.026]);
    assert_eq!(
        take_redirect_line(player_id),
        Some(REDIRECT_LINE),
        "the arrival owes the player the reason"
    );
}

#[tokio::test]
async fn a_gm_enters_the_debug_area() {
    let (world, position, player_id) = travel_to_debug_area(2, None, 55797).await;
    assert_eq!(world, "DebugArea");
    assert_eq!(position, [251.0, 8.0, -962.0]);
    assert_eq!(take_redirect_line(player_id), None);
}

#[tokio::test]
async fn an_sgu_player_arrives_at_the_sgu_start() {
    let (world, position, player_id) = travel_to_debug_area(1, Some(2), 55798).await;
    assert_eq!(world, "SGC_W1", "a moderator is not a GM; SGU start");
    assert_eq!(position, [201.5, 1.31, 49.724]);
    assert_eq!(take_redirect_line(player_id), Some(REDIRECT_LINE));
}
