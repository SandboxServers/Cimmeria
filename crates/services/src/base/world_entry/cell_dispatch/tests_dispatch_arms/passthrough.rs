//! Pure-passthrough dispatch arms.
//!
//! These arms delegate directly to a sibling handler that owns its own
//! tests. The dispatcher's only job is to route the message to the
//! correct function — not to validate or transform. The tests here pin
//! the routing decision by triggering a deterministic short-circuit
//! inside the handler (no-pool / no-addr) and asserting the resulting
//! log line. A regression that mis-routes (e.g. SpaceData being sent
//! through a transport emit, or GrantXP going to handle_grant_cash)
//! would either fire the wrong log or fail to fire the expected one.

use super::super::*;
use super::empty_maps;
use crate::test_support::{LogCapture, TestTransport};

/// `SpaceData` is a fire-and-forget startup notification. The registry
/// is a private singleton, so the only observable proof the arm reached
/// `register_space` is its `debug!("Registered space")` log line. Pin
/// it via `LogCapture` so a regression that re-routes `SpaceData`
/// elsewhere (or drops the log entirely) trips here.
#[tokio::test]
async fn space_data_routes_to_register_space_and_logs_world_name() {
    let capture = LogCapture::install();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let (connected, entity_to_addr) = empty_maps();

    handle_cell_message(
        CellToBaseMsg::SpaceData {
            space_id: 424242,
            world_name: "CellDispatchTestWorld".into(),
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

    let event = capture
        .find_message(tracing::Level::DEBUG, "Registered space")
        .expect("SpaceData arm must reach register_space (logs at DEBUG)");
    assert!(
        event.has_field("world", "CellDispatchTestWorld"),
        "register_space must record the world_name field exactly: {event:#?}"
    );
    assert!(
        event.has_field("space_id", "424242"),
        "register_space must record the space_id field exactly: {event:#?}"
    );
}

/// `MissionUpdate` is a pure DB-persist message — the dispatcher delegates
/// directly to `handle_mission_update`. With `db_pool: None` the handler
/// short-circuits with a `debug!("MissionUpdate: no DB pool")`. Pin
/// that path so a future regression that mis-routes `MissionUpdate`
/// (e.g. to a transport emit) trips here via the missing log + a
/// non-empty transport.
#[tokio::test]
async fn mission_update_routes_to_handler_and_returns_without_emitting_when_no_pool() {
    let capture = LogCapture::install();
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let (connected, entity_to_addr) = empty_maps();
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(1);

    handle_cell_message(
        CellToBaseMsg::MissionUpdate {
            player_id: 77,
            mission_id: 1234,
            status: 1,
            current_step_id: Some(2),
            completed_step_ids: vec![1],
            completed_objective_ids: vec![],
            active_objective_ids: vec![10, 11],
            failed_objective_ids: vec![],
            repeats: 0,
        },
        &transport,
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    assert!(
        typed_transport.is_empty(),
        "mission update is DB-only — no wire"
    );
    assert!(
        cell_rx.try_recv().is_err(),
        "mission update is one-way base→DB"
    );
    let event = capture
        .find_message(tracing::Level::DEBUG, "MissionUpdate: no DB pool")
        .expect("dispatch must reach handle_mission_update's no-pool branch");
    assert!(
        event.has_field("player_id", "77"),
        "handler must record player_id: {event:#?}"
    );
    assert!(
        event.has_field("mission_id", "1234"),
        "handler must record mission_id: {event:#?}"
    );
}

/// `MailRequest` is a direct passthrough to `handle_mail_request`. With
/// `db_pool: None`, the handler logs `debug!("Mail request: no DB pool")`.
/// Pin the routing via that log + no-wire / no-cell-reply assertions.
#[tokio::test]
async fn mail_request_routes_to_handler_and_returns_without_emitting_when_no_pool() {
    use crate::cell::messages::MailOp;
    let capture = LogCapture::install();
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let (connected, entity_to_addr) = empty_maps();

    handle_cell_message(
        CellToBaseMsg::MailRequest {
            entity_id: 55,
            player_id: 22,
            op: MailOp::RequestHeaders { b_archive: 0 },
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

    assert!(typed_transport.is_empty());
    let event = capture
        .find_message(tracing::Level::DEBUG, "Mail request: no DB pool")
        .expect("dispatch must reach handle_mail_request's no-pool branch");
    assert!(event.has_field("entity_id", "55"));
    assert!(event.has_field("player_id", "22"));
}

/// `GrantXP` is delegated to `handle_grant_xp`. When `entity_to_addr`
/// has no mapping for the entity, the handler short-circuits with
/// `warn!("GrantXP: no address for entity")`. Pin the routing via
/// that log so a future regression that drops the GrantXP arm (or
/// re-routes it elsewhere) trips here.
#[tokio::test]
async fn grant_xp_routes_to_handler_and_warns_when_entity_unknown() {
    let capture = LogCapture::install();
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let (connected, entity_to_addr) = empty_maps();

    handle_cell_message(
        CellToBaseMsg::GrantXP {
            entity_id: 8888,
            xp_amount: 250,
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

    assert!(typed_transport.is_empty());
    let event = capture
        .find_message(tracing::Level::WARN, "GrantXP: no address for entity")
        .expect("dispatch must reach handle_grant_xp's missing-addr branch");
    assert!(event.has_field("entity_id", "8888"));
}

/// `TeleportPlayer` is delegated to `handle_teleport_player`. With no
/// entity_to_addr mapping, the handler logs
/// `warn!("TeleportPlayer: no client addr for entity")` and returns
/// without touching the transport.
#[tokio::test]
async fn teleport_player_routes_to_handler_and_warns_when_entity_unknown() {
    let capture = LogCapture::install();
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let (connected, entity_to_addr) = empty_maps();

    handle_cell_message(
        CellToBaseMsg::TeleportPlayer {
            entity_id: 7777,
            space_id: 1,
            position: [10.0, 20.0, 30.0],
            prev_pos: [0.0, 0.0, 0.0],
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

    assert!(typed_transport.is_empty());
    let event = capture
        .find_message(tracing::Level::WARN, "TeleportPlayer: no client addr")
        .expect("dispatch must reach handle_teleport_player's missing-addr branch");
    assert!(event.has_field("entity_id", "7777"));
}
