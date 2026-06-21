//! Fallible-handler error-log seams.
//!
//! `GateTravel` and `ReanchorPlayer` wrap their handlers in
//! `if let Err(e) = handle_X(...) { tracing::error!(...) }`. The
//! `tracing::error!` call is the only observable consequence of the
//! handler returning `Err` — without it, the failure would be a silent
//! swallow. Pin both the level (ERROR) and the message substring so a
//! future refactor that downgrades the log to WARN, drops the log
//! entirely, or silently swallows the `Err` trips here.

use super::super::*;
use super::empty_maps;
use crate::test_support::{LogCapture, TestTransport};

/// `GateTravel` whose `entity_id` has no `entity_to_addr` mapping makes
/// `handle_gate_travel` return `Err("Gate travel: no client addr...")`.
/// The dispatcher arm wraps the call in `if let Err(e) = ...` and emits
/// `tracing::error!("Gate travel failed: ...")`.
#[tokio::test]
async fn gate_travel_logs_error_when_entity_has_no_session() {
    let capture = LogCapture::install();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let (connected, entity_to_addr) = empty_maps();

    handle_cell_message(
        CellToBaseMsg::GateTravel {
            entity_id: 9991,
            target_world_name: "Castle_CellBlock".into(),
            position: [0.0; 3],
            rotation: [0.0; 3],
            destination_ring_id: None,
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
        &None,
    )
    .await;

    let event = capture
        .find_message(tracing::Level::ERROR, "Gate travel failed")
        .expect("ERROR log must fire on gate_travel handler Err");
    assert!(
        event.has_field("entity_id", "9991"),
        "error log must carry the entity_id field: {event:#?}"
    );
}

/// `ReanchorPlayer` whose `entity_id` has no session triggers the same
/// `if let Err(e) = handle_reanchor_player(...)` -> `tracing::error!`
/// seam as gate-travel.
#[tokio::test]
async fn reanchor_player_logs_error_when_entity_has_no_session() {
    let capture = LogCapture::install();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let (connected, entity_to_addr) = empty_maps();

    handle_cell_message(
        CellToBaseMsg::ReanchorPlayer {
            entity_id: 9992,
            space_id: 1,
            position: [0.0; 3],
            rotation: [0.0; 3],
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
        &None,
    )
    .await;

    let event = capture
        .find_message(tracing::Level::ERROR, "Reanchor player failed")
        .expect("ERROR log must fire on reanchor handler Err");
    assert!(
        event.has_field("entity_id", "9992"),
        "error log must carry the entity_id field: {event:#?}"
    );
}
