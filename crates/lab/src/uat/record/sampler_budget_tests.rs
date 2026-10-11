//! Sampler tests, continued: the per-tick error budget, the client entry,
//! failed and truncated snapshots, and the entity lookup caps and retries.

use serde_json::{json, Value};

use super::super::server::{MAX_LOOKUPS_PER_TICK, MAX_TRACKED_IDS};
use super::*;

/// An inbound `setTargetID` naming `id` (cell method 0, msg_id 0x80: the
/// 4-byte entity prefix, then the INT32 target). `decode_call` reads it by
/// method index, so a base msg_id (0xC0 and up) would name nothing.
fn target_row(id: u32) -> Value {
    let le: String = id
        .to_le_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    json!({ "dir": "in", "msg_id": 0x80, "method_index": 0, "msg_name": "setTargetID",
            "args_hex": format!("00000000{le}") })
}

fn tap_of(ids: impl IntoIterator<Item = u32>) -> ToolOutcome {
    let rows: Vec<Value> = ids.into_iter().map(target_row).collect();
    ok(json!({ "messages": rows }))
}

fn lookups(server: &FakeServer, id: u32) -> usize {
    server
        .calls()
        .iter()
        .filter(|c| c.0 == "server_entity_get" && c.1["entity_id"] == id)
        .count()
}

#[tokio::test]
async fn a_tick_keeps_four_errors_however_many_reads_fail() {
    let server = FakeServer::default()
        .answer(
            "server_packet_tap_read",
            vec![
                ok(json!({ "messages": [] })),
                ToolOutcome::err("lab-mcp: operation timed out"),
            ],
        )
        .answer(
            "server_db_query",
            vec![
                ok(json!({ "rows": [{ "player_id": 42 }] })),
                ToolOutcome::err("database not connected"),
            ],
        )
        .answer(
            "server_log_tail",
            vec![
                ok(json!({ "entries": [], "newest_ms": 1000 })),
                ToolOutcome::err("log ring unavailable"),
            ],
        )
        .answer(
            "server_packet_tap_stop",
            vec![ToolOutcome::err("lab-mcp: operation timed out")],
        );
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;
    server.gone.lock().unwrap().insert(7);
    client.camera_fails.store(true, Ordering::SeqCst);

    // finish fails six reads: tap, player, log, snapshot, tap stop, camera.
    let (t, _) = s.finish(1_500, "stop").await;
    assert_eq!(t.errors.len(), 4, "{:?}", t.errors);
    assert!(t.errors.iter().all(|e| e.chars().count() <= 120));
}

#[tokio::test]
async fn client_failures_share_one_entry() {
    let server = FakeServer::default();
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;
    client.pump_fails.store(true, Ordering::SeqCst);
    client.camera_fails.store(true, Ordering::SeqCst);
    client.ui_fails.store(true, Ordering::SeqCst);
    // A server failure in the same tick still gets a slot.
    server.gone.lock().unwrap().insert(7);

    let t = s.tick(1_500).await;
    let client_entries: Vec<&String> = t
        .errors
        .iter()
        .filter(|e| e.starts_with("client: "))
        .collect();
    assert_eq!(
        client_entries,
        ["client: pump: bridge: dispatch timeout"],
        "{:?}",
        t.errors
    );
    assert!(t.errors.iter().any(|e| e == "player: no entity 7"));
}

#[tokio::test]
async fn a_failed_pump_is_one_entry_and_the_tick_goes_on() {
    let server = FakeServer::default();
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;

    client.pump_fails.store(true, Ordering::SeqCst);
    let t = s.tick(1_500).await;
    assert_eq!(t.errors, ["client: pump: bridge: dispatch timeout"]);
    assert!(t.camera.is_some() && t.player.is_some() && t.windows.is_some());

    client.pump_fails.store(false, Ordering::SeqCst);
    client.pump_lua_error.store(true, Ordering::SeqCst);
    let t = s.tick(2_000).await;
    assert_eq!(t.errors, ["client: pump: attempt to index a nil value"]);
    assert!(t.camera.is_some() && t.player.is_some() && t.windows.is_some());
}

#[tokio::test]
async fn a_camera_without_yaw_gain_is_left_out() {
    let server = FakeServer::default();
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;
    client.no_yaw_gain.store(true, Ordering::SeqCst);
    let t = s.tick(1_500).await;
    assert!(t.camera.is_none(), "the unsigned gain is not a yaw_gain");
    assert_eq!(t.errors, ["client: camera: no yaw_gain"]);
}

/// Missions read fine; the `n`th snapshot statement (2 or 3) fails.
async fn snapshot_with_failed_statement(n: usize) -> Tick {
    let mut replies = vec![
        ok(json!({ "rows": [{ "player_id": 42 }] })),
        ok(json!({ "rows": [{ "mission_id": 622, "status": 1, "step": 2113, "repeats": 0 }] })),
    ];
    if n == 3 {
        replies.push(ok(json!({ "rows": [
            { "level": 1, "exp": 0, "naquadah": 0, "world_location": "Castle_CellBlock" },
        ] })));
    }
    replies.push(ToolOutcome::err(
        "canceling statement due to statement timeout",
    ));
    let server = FakeServer::default().answer("server_db_query", replies);
    let client = FakeClient::default();
    let (_, _, t0) = begin(&server, &client).await;
    t0
}

#[tokio::test]
async fn a_failed_later_statement_drops_the_whole_snapshot() {
    for n in [2, 3] {
        let t0 = snapshot_with_failed_statement(n).await;
        assert!(t0.snapshot.is_none(), "statement {n}");
        assert!(
            t0.errors.iter().any(|e| e.starts_with("snapshot: ")),
            "statement {n}: {:?}",
            t0.errors
        );
    }
}

#[tokio::test]
async fn a_truncated_query_is_a_failed_snapshot() {
    let server = FakeServer::default().answer(
        "server_db_query",
        vec![
            ok(json!({ "rows": [{ "player_id": 42 }] })),
            ok(json!({ "truncated": true, "rows": [
                { "mission_id": 622, "status": 1, "step": 2113, "repeats": 0 },
            ] })),
            ok(json!({ "rows": [
                { "level": 1, "exp": 0, "naquadah": 0, "world_location": "Castle_CellBlock" },
            ] })),
            ok(json!({ "rows": [] })),
        ],
    );
    let client = FakeClient::default();
    let (_, _, t0) = begin(&server, &client).await;
    assert!(t0.snapshot.is_none());
    assert!(
        t0.errors
            .iter()
            .any(|e| e == "snapshot: server_db_query: truncated result"),
        "{:?}",
        t0.errors
    );
}

#[tokio::test]
async fn entity_lookups_are_capped_per_tick() {
    let ids: Vec<u32> = (1001..1011).collect();
    let server = FakeServer::default().answer(
        "server_packet_tap_read",
        vec![
            ok(json!({ "messages": [] })),
            tap_of(ids.clone()),
            ok(json!({ "messages": [] })),
        ],
    );
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;

    let t = s.tick(1_500).await;
    assert_eq!(t.entities.len(), MAX_LOOKUPS_PER_TICK);
    assert_eq!(
        t.errors,
        ["entities: 2 lookups wait for the next tick"],
        "one entry for the overflow"
    );
    let t = s.tick(2_000).await;
    assert_eq!(t.entities.len(), 2);
    assert!(t.errors.is_empty(), "{:?}", t.errors);
    assert!(ids.iter().all(|&id| lookups(&server, id) == 1));
}

#[tokio::test]
async fn no_new_lookups_past_the_tracked_id_cap() {
    let server = FakeServer::default().answer(
        "server_packet_tap_read",
        vec![ok(json!({ "messages": [] })), tap_of([FROST])],
    );
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;
    s.seen.extend((0..).take(MAX_TRACKED_IDS));
    assert!(s.seen.len() >= MAX_TRACKED_IDS);

    let t = s.tick(1_500).await;
    assert!(t.entities.is_empty());
    assert_eq!(t.errors, ["entities: 2048 ids tracked, no new lookups"]);
    // Reported once per recording.
    let t = s.tick(2_000).await;
    assert!(t.errors.is_empty(), "{:?}", t.errors);
    assert_eq!(lookups(&server, FROST), 0);
}

#[tokio::test]
async fn a_failed_lookup_is_retried_on_later_ticks() {
    let server = FakeServer::default().answer(
        "server_packet_tap_read",
        vec![
            ok(json!({ "messages": [] })),
            tap_of([FROST]),
            ok(json!({ "messages": [] })),
        ],
    );
    server.flaky.lock().unwrap().insert(FROST, 2);
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;

    assert!(s.tick(1_500).await.entities.is_empty());
    assert!(s.tick(2_000).await.entities.is_empty());
    let t = s.tick(2_500).await;
    assert_eq!(t.entities.len(), 1);
    assert_eq!(t.entities[0].entity_id, FROST);
    s.tick(3_000).await;
    assert_eq!(lookups(&server, FROST), 3);
}

#[tokio::test]
async fn a_lookup_gives_up_after_three_retries() {
    let server = FakeServer::default().answer(
        "server_packet_tap_read",
        vec![ok(json!({ "messages": [] })), tap_of([FROST])],
    );
    server.flaky.lock().unwrap().insert(FROST, 1_000);
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;

    let mut gave_up = 0;
    for i in 0..8 {
        let t = s.tick(1_500 + i * 500).await;
        gave_up += t
            .errors
            .iter()
            .filter(|e| e.starts_with(&format!("entity {FROST}:")))
            .count();
    }
    // The first try and three retries, though every tick names Frost.
    assert_eq!(lookups(&server, FROST), 4);
    assert_eq!(gave_up, 1);
}
