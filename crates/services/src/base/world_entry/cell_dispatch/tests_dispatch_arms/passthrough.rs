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
        &None,
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
        &None,
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
        &None,
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
            notify_gm: false,
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

    assert!(typed_transport.is_empty());
    let event = capture
        .find_message(tracing::Level::WARN, "GrantXP: no address for entity")
        .expect("dispatch must reach handle_grant_xp's missing-addr branch");
    assert!(event.has_field("entity_id", "8888"));
}

/// `ExecuteTrade` is the cell→base atomic-commit handoff. The dispatcher
/// delegates directly to `super::methods::trade::handle_execute_trade`.
/// With `db_pool: None`, the handler short-circuits with
/// `warn!("ExecuteTrade: no DB pool — sending Cancelled to both")`. Pin
/// the routing via that log so a future regression that drops or
/// mis-routes the ExecuteTrade arm trips here. A pure
/// `transport.is_empty()` check would not work — the handler still emits
/// onTradeResults(Cancelled) packets attempting to fan out to both
/// players; the entity_to_addr map is empty in this test so those sends
/// are no-ops, but the deterministic signal of "routing landed in
/// handle_execute_trade" is the warn log.
///
/// Revert-verifier: replacing the `ExecuteTrade` arm body with a `()`
/// or routing it to `handle_grant_xp` causes the "no DB pool" log to
/// not fire from this entry, failing the assertion.
#[tokio::test]
async fn execute_trade_routes_to_handler_and_warns_when_no_db_pool() {
    let capture = LogCapture::install();
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let (connected, entity_to_addr) = empty_maps();

    handle_cell_message(
        CellToBaseMsg::ExecuteTrade {
            entity_id: 1234,
            player_id: 11,
            partner_entity_id: 5678,
            partner_player_id: 22,
            p1_item_instance_ids: vec![100],
            p1_cash: 50,
            p2_item_instance_ids: vec![200],
            p2_cash: 25,
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None, // db_pool: None — handler short-circuits
        &None,
        "127.0.0.1",
        7777,
        &None,
    )
    .await;

    let event = capture
        .find_message(
            tracing::Level::WARN,
            "ExecuteTrade: no DB pool — sending Cancelled to both",
        )
        .expect(
            "dispatch must reach handle_execute_trade's no-pool branch. \
             If this assertion fails, either (a) the ExecuteTrade match arm \
             was removed or mis-routed (e.g. to handle_grant_xp), or (b) the \
             handler's no-pool warn was downgraded/removed. Captured \
             events: see test output.",
        );
    // Pin the entity_id field so a regression that swaps the partner
    // into the primary slot (or omits the field) trips here.
    assert!(
        event.has_field("entity_id", "1234"),
        "ExecuteTrade warn must record entity_id=1234: {event:#?}"
    );
    assert!(
        event.has_field("partner_entity_id", "5678"),
        "ExecuteTrade warn must record partner_entity_id=5678: {event:#?}"
    );
}

/// `ContactListPresenceEvent` is a cell→base hop for contact-list presence
/// events that originate cell-side (currently: Death). The dispatcher routes
/// it to `contact_list_dispatch::route`, which spawns a fire-and-forget
/// `fanout_contact_event`. With `db_pool: None`, `fanout_contact_event`
/// short-circuits with `warn!("ContactList presence fanout: no DB pool")`.
///
/// Revert-verifier: removing or mis-routing the
/// `ContactListPresenceEvent` arm in `handle_cell_message` causes this
/// test to fail because the no-pool warning is never emitted from
/// `fanout_contact_event`.
#[tokio::test]
async fn contact_list_presence_event_routes_to_fanout_and_warns_when_no_pool() {
    use crate::base::contact_list::wire::EVENT_DEATH;
    let capture = LogCapture::install();
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let (connected, entity_to_addr) = empty_maps();

    handle_cell_message(
        CellToBaseMsg::ContactListPresenceEvent {
            player_name: "O'Neill".to_string(),
            event_id: EVENT_DEATH,
            data_value: 0,
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None, // db_pool: None — fanout short-circuits with warn
        &None,
        "127.0.0.1",
        7777,
        &None,
    )
    .await;

    // The spawn is fire-and-forget; yield to let the spawned task run.
    tokio::task::yield_now().await;

    assert!(
        typed_transport.is_empty(),
        "ContactListPresenceEvent is DB-only fan-out — no wire emit expected"
    );
    let event = capture
        .find_message(
            tracing::Level::WARN,
            "ContactList presence fanout: no DB pool",
        )
        .expect(
            "dispatch must reach fanout_contact_event's no-pool branch. \
             If this assertion fails, either (a) the ContactListPresenceEvent arm \
             was removed or mis-routed, or (b) the no-pool warn was renamed.",
        );
    assert!(
        event.has_field("player_name", "O'Neill"),
        "fanout warn must record player_name: {event:#?}"
    );
    assert!(
        event.has_field("event_id", &EVENT_DEATH.to_string()),
        "fanout warn must record event_id: {event:#?}"
    );
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
        &None,
    )
    .await;

    assert!(typed_transport.is_empty());
    let event = capture
        .find_message(tracing::Level::WARN, "TeleportPlayer: no client addr")
        .expect("dispatch must reach handle_teleport_player's missing-addr branch");
    assert!(event.has_field("entity_id", "7777"));
}
