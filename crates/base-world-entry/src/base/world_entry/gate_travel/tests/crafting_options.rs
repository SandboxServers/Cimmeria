//! Gate travel is a world entry for the crafting options: the stations in
//! reach belong to the origin world, so they are forgotten, and the
//! change sends wait for the destination's login send. Both must happen
//! before the cell is asked for the destination entity, so no station
//! report of the new world lands before the reset and is wiped by it.

use super::*;
use crate::cell::messages::BaseToCellMsg;
use cimmeria_wire::crafting::CraftingOptions;
use std::time::Duration;
use tokio::time::timeout;

const ENTITY_ID: u32 = 42;

/// Regression shape: without the reset, the destination's login bundle
/// names the origin world's station (900) until the destination's first
/// station report, and change sends go out before the client has created
/// the new player entity.
#[tokio::test]
async fn gate_travel_forgets_origin_stations_before_the_cell_create() {
    let addr: SocketAddr = "127.0.0.1:55790".parse().unwrap();
    let mut state = make_state();
    state.crafting_options.stations = [Some(900), None, None, Some(901)];
    state.crafting_options.armed = true;
    state.crafting_options.last_sent = Some(CraftingOptions::default());
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY_ID, addr)])));
    let transport: Arc<dyn Transport> = Arc::new(crate::test_support::TestTransport::new());
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(8);

    let (c, e, t) = (
        Arc::clone(&connected),
        Arc::clone(&entity_to_addr),
        Arc::clone(&transport),
    );
    let travel = tokio::spawn(async move {
        handle_gate_travel(
            ENTITY_ID,
            "Castle_CellBlock",
            [1.0, 2.0, 3.0],
            [0.0; 3],
            None,
            None,
            &t,
            &c,
            &e,
            &Some(cell_tx),
            &None,
        )
        .await
    });

    let msg = timeout(Duration::from_secs(2), cell_rx.recv())
        .await
        .expect("CreateEntity must not hang")
        .expect("CreateEntity expected");
    let BaseToCellMsg::CreateEntity { reply_tx, .. } = msg else {
        panic!("expected CreateEntity as the first base->cell message");
    };
    // Read at the moment the cell is asked for the entity.
    let at_create = connected.lock().unwrap()[&addr].crafting_options.clone();
    let _ = reply_tx.send(0x0001_0001);
    timeout(Duration::from_secs(2), travel)
        .await
        .expect("gate travel must not hang")
        .unwrap()
        .expect("gate travel completes");

    assert_eq!(at_create.stations, [None; 4], "origin stations forgotten");
    assert!(!at_create.armed, "change sends wait for the login send");
    assert_eq!(at_create.last_sent, None);
}
