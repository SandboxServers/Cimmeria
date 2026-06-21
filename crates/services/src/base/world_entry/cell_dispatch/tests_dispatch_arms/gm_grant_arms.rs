//! Routing tests for the three GM-command dispatch arms:
//! `GrantExpertise`, `GrantAppliedSciencePoints`, and `GmSpawnNpc`.
//!
//! These arms delegate directly to a handler that owns its own DB-touching
//! tests. The dispatcher's only job is to route the right `CellToBaseMsg`
//! variant to the right handler with the right fields. We pin the routing by
//! driving the real `handle_cell_message` with `db_pool: None`, which makes
//! each handler short-circuit on a distinctive no-pool `warn!`. `LogCapture`
//! asserts the log fired *and* that the message's fields were forwarded
//! verbatim — so a regression that drops the arm, mis-routes it (e.g.
//! GrantExpertise → handle_grant_cash), or swaps a field trips here.
//!
//! Revert-verifier: deleting any of the three arm bodies (or routing them to
//! the wrong handler) stops the matching no-pool warn from firing through this
//! entry, failing the corresponding `.expect`.

use super::super::*;
use super::empty_maps;
use crate::test_support::{LogCapture, TestTransport};

#[tokio::test]
async fn grant_expertise_routes_to_handler_and_warns_when_no_pool() {
    let capture = LogCapture::install();
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let (connected, entity_to_addr) = empty_maps();

    handle_cell_message(
        CellToBaseMsg::GrantExpertise {
            entity_id: 4242,
            player_id: 77,
            discipline_id: 3,
            amount: 40,
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

    assert!(
        typed_transport.is_empty(),
        "no-pool GrantExpertise drops the grant — no wire"
    );
    let event = capture
        .find_message(
            tracing::Level::WARN,
            "GrantExpertise: no DB pool, dropping grant",
        )
        .expect(
            "dispatch must reach handle_grant_expertise's no-pool branch. If this \
             fails the GrantExpertise arm was removed or mis-routed.",
        );
    assert!(
        event.has_field("player_id", "77"),
        "GrantExpertise warn must forward player_id: {event:#?}"
    );
    assert!(
        event.has_field("discipline_id", "3"),
        "GrantExpertise warn must forward discipline_id: {event:#?}"
    );
    assert!(
        event.has_field("amount", "40"),
        "GrantExpertise warn must forward amount: {event:#?}"
    );
}

#[tokio::test]
async fn grant_applied_science_routes_to_handler_and_warns_when_no_pool() {
    let capture = LogCapture::install();
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let (connected, entity_to_addr) = empty_maps();

    handle_cell_message(
        CellToBaseMsg::GrantAppliedSciencePoints {
            entity_id: 4243,
            player_id: 88,
            amount: 12,
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
        .find_message(
            tracing::Level::WARN,
            "GrantAppliedSciencePoints: no DB pool, dropping grant",
        )
        .expect(
            "dispatch must reach handle_grant_applied_science's no-pool branch. \
             If this fails the GrantAppliedSciencePoints arm was removed or mis-routed.",
        );
    assert!(
        event.has_field("player_id", "88"),
        "GrantAppliedSciencePoints warn must forward player_id: {event:#?}"
    );
    assert!(
        event.has_field("amount", "12"),
        "GrantAppliedSciencePoints warn must forward amount: {event:#?}"
    );
}

#[tokio::test]
async fn gm_spawn_npc_routes_to_handler_and_warns_when_no_pool() {
    let capture = LogCapture::install();
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let (connected, entity_to_addr) = empty_maps();
    // A cell channel is present so a regression that *did* reach the reply path
    // (with a pool) would be observable; with no pool the handler short-circuits
    // before touching it.
    let (cell_tx, mut cell_rx) = mpsc::channel::<BaseToCellMsg>(1);

    handle_cell_message(
        CellToBaseMsg::GmSpawnNpc {
            entity_id: 4244,
            template_id: 9001,
            space_id: 5,
            world_name: "Castle".into(),
            position: [1.0, 2.0, 3.0],
        },
        &transport,
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
        &None, // db_pool: None — handler cannot resolve the template
        &None,
        "127.0.0.1",
        7777,
        &None,
    )
    .await;

    assert!(
        cell_rx.try_recv().is_err(),
        "no-pool GmSpawnNpc must not reply GmSpawnNpcReady to the cell"
    );
    let event = capture
        .find_message(
            tracing::Level::WARN,
            "GmSpawnNpc: no DB pool, cannot resolve template",
        )
        .expect(
            "dispatch must reach handle_gm_spawn_npc's no-pool branch. If this \
             fails the GmSpawnNpc arm was removed or mis-routed.",
        );
    assert!(
        event.has_field("template_id", "9001"),
        "GmSpawnNpc warn must forward template_id: {event:#?}"
    );
}
