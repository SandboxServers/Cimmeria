//! Witness-broadcast arm fan-out byte tests.
//!
//! `WitnessEntityMethod` and `EntityInvisible` fan a single packet out
//! to exactly one address — the witness's. The bug shape these tests
//! pin is wrong-recipient routing: a regression that swapped
//! `witness_id` for `entity_id` (the observee) would send the packet
//! to the wrong client. With both addresses present in `entity_to_addr`,
//! a swap would show up as a `send_count_to(observee_addr) > 0` failure.

use super::super::*;
use super::test_default_connected_client_state;
use crate::test_support::TestTransport;

/// `WitnessEntityMethod` routes the method packet to exactly the
/// witness's address — never to the observee entity's address, never
/// to any other session. Witness-cardinality regression guard.
#[tokio::test]
async fn witness_entity_method_routes_one_packet_to_witness_addr_only() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let witness_id = 900u32;
    let observee_id = 901u32; // the ghost entity the method is called on
    let witness_addr: SocketAddr = "127.0.0.1:55900".parse().unwrap();
    let observee_addr: SocketAddr = "127.0.0.1:55901".parse().unwrap();

    let connected = Arc::new(Mutex::new(HashMap::from([
        (witness_addr, test_default_connected_client_state()),
        (observee_addr, test_default_connected_client_state()),
    ])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (witness_id, witness_addr),
        (observee_id, observee_addr),
    ])));

    handle_cell_message(
        CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id: observee_id,
            method_index: 0x20,
            args: vec![0xDE, 0xAD],
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    assert_eq!(
        typed_transport.send_count_to(witness_addr),
        1,
        "exactly one packet to the witness"
    );
    assert_eq!(
        typed_transport.send_count_to(observee_addr),
        0,
        "observee must NOT receive — only the witness does"
    );
    assert_eq!(typed_transport.len(), 1, "no traffic to any other address");
}

/// `EntityInvisible` routes the visibility-hide packet to exactly the
/// witness's address. Same fan-out shape as `WitnessEntityMethod`,
/// different wire bytes — pin the routing so a regression that swaps
/// the witness_id for entity_id (a wrong-recipient bug class) trips.
#[tokio::test]
async fn entity_invisible_routes_one_packet_to_witness_addr_only() {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let witness_id = 910u32;
    let observee_id = 911u32;
    let witness_addr: SocketAddr = "127.0.0.1:55910".parse().unwrap();
    let observee_addr: SocketAddr = "127.0.0.1:55911".parse().unwrap();

    let connected = Arc::new(Mutex::new(HashMap::from([
        (witness_addr, test_default_connected_client_state()),
        (observee_addr, test_default_connected_client_state()),
    ])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
        (witness_id, witness_addr),
        (observee_id, observee_addr),
    ])));

    handle_cell_message(
        CellToBaseMsg::EntityInvisible {
            witness_id,
            entity_id: observee_id,
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    assert_eq!(typed_transport.send_count_to(witness_addr), 1);
    assert_eq!(typed_transport.send_count_to(observee_addr), 0);
    assert_eq!(typed_transport.len(), 1);
}
