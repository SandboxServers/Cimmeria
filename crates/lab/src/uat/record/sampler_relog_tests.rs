//! Sampler tests, continued: resolving the player by `player_id`, tap
//! hygiene (replaced, vanished, relog, the stop guard) and the instance
//! guard.

use std::sync::Arc;

use serde_json::{json, Value};

use super::*;

fn tap_starts(server: &FakeServer) -> Vec<Value> {
    server
        .calls()
        .into_iter()
        .filter(|c| c.0 == "server_packet_tap_start")
        .map(|c| c.1)
        .collect()
}

/// Let the guard's spawned stop run.
async fn settle() {
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
}

#[tokio::test]
async fn begin_resolves_the_session_by_player_id_not_name() {
    // "Bob" is a whole word of another player's name: a name match would
    // tap Bob Jones.
    let server = FakeServer::default().answer(
        "server_sessions",
        vec![ok(
            json!({ "sessions": [{ "entity_id": 11, "name": "Bob Jones", "player_id": 7 }] }),
        )],
    );
    let client = FakeClient::default();
    let err = begin_as(&server, &client, "Bob")
        .await
        .err()
        .expect("no session carries player_id 42");
    assert!(err.contains("player_id 42"), "{err}");
    assert_eq!(server.count("server_packet_tap_start"), 0);
}

#[tokio::test]
async fn a_replaced_tap_is_reported_at_begin_and_on_a_relog() {
    let server = FakeServer::default()
        .answer(
            "server_packet_tap_start",
            vec![
                ok(json!({ "replaced_existing": true })),
                ok(json!({ "replaced_existing": true })),
            ],
        )
        .answer(
            "server_sessions",
            vec![
                ok(json!({ "sessions": [{ "entity_id": 7, "player_id": 42 }] })),
                ok(json!({ "sessions": [{ "entity_id": 11, "player_id": 42 }] })),
            ],
        );
    let client = FakeClient::default();
    let (mut s, _, t0) = begin(&server, &client).await;
    assert!(
        t0.errors
            .iter()
            .any(|e| e == "tap replaced an existing tap"),
        "{:?}",
        t0.errors
    );
    server.gone.lock().unwrap().insert(7);
    let t = s.tick(1_500).await;
    assert!(
        t.errors.iter().any(|e| e == "relog: tap restarted on 11"),
        "{:?}",
        t.errors
    );
    assert!(
        t.errors.iter().any(|e| e == "tap replaced an existing tap"),
        "{:?}",
        t.errors
    );
}

#[tokio::test]
async fn a_vanished_tap_is_restarted_on_the_same_entity() {
    let server = FakeServer::default().answer(
        "server_packet_tap_read",
        vec![
            ok(json!({ "messages": [] })),
            ToolOutcome::err("lab-mcp: operation timed out"),
            ToolOutcome::err(
                "no active packet tap for entity 7 (start one with server_packet_tap_start)",
            ),
            ok(json!({ "messages": [] })),
        ],
    );
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;

    // A timeout is not a vanished tap: nothing restarts.
    let t = s.tick(1_500).await;
    assert!(
        !t.errors.iter().any(|e| e == "tap restarted"),
        "{:?}",
        t.errors
    );
    assert_eq!(tap_starts(&server).len(), 1);

    let t = s.tick(2_000).await;
    assert!(
        t.errors.iter().any(|e| e == "tap restarted"),
        "{:?}",
        t.errors
    );
    let starts = tap_starts(&server);
    assert_eq!(starts.len(), 2);
    assert_eq!(starts[1]["entity_id"], 7);
    assert_eq!(s.player_entity(), 7);
    assert_eq!(server.count("server_packet_tap_stop"), 0);
}

#[tokio::test]
async fn a_tap_read_error_finds_a_relog() {
    // The player entity still answers (the server has not reaped it yet):
    // only the tap read's failure leads to the relog check.
    let server = FakeServer::default()
        .answer(
            "server_packet_tap_read",
            vec![
                ok(json!({ "messages": [] })),
                ToolOutcome::err("lab-mcp: operation timed out"),
                ok(json!({ "messages": [] })),
            ],
        )
        .answer(
            "server_sessions",
            vec![
                ok(json!({ "sessions": [{ "entity_id": 7, "player_id": 42 }] })),
                ok(json!({ "sessions": [{ "entity_id": 11, "player_id": 42 }] })),
            ],
        );
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;

    let t = s.tick(1_500).await;
    assert!(
        t.errors.iter().any(|e| e == "relog: tap restarted on 11"),
        "{:?}",
        t.errors
    );
    assert_eq!(s.player_entity(), 11);
    // The same tick's player read already follows the new entity.
    assert_eq!(t.player.unwrap().entity_id, 11);
}

#[tokio::test]
async fn a_recycled_player_id_is_a_relog() {
    let server = FakeServer::default().answer(
        "server_sessions",
        vec![
            ok(json!({ "sessions": [{ "entity_id": 7, "player_id": 42 }] })),
            ok(json!({ "sessions": [{ "entity_id": 11, "player_id": 42 }] })),
        ],
    );
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;
    // Entity 7 now belongs to someone else.
    server
        .renamed
        .lock()
        .unwrap()
        .insert(7, "Somebody".to_string());

    let t = s.tick(1_500).await;
    assert!(t.player.is_none(), "another player's entity is not sampled");
    assert!(
        t.errors
            .iter()
            .any(|e| e == "player: entity 7 is now \"Somebody\""),
        "{:?}",
        t.errors
    );
    assert_eq!(s.player_entity(), 11);

    // The match is case-insensitive.
    server
        .renamed
        .lock()
        .unwrap()
        .insert(11, "LABONE".to_string());
    let t = s.tick(2_000).await;
    assert_eq!(t.player.unwrap().entity_id, 11);
}

#[tokio::test]
async fn the_guard_stops_the_tap_unless_finish_did() {
    // Dropped without finish (an aborted or panicking task).
    let server = Arc::new(FakeServer::default());
    let client = FakeClient::default();
    let guard = TapStopGuard::new(server.clone() as Arc<dyn ServerInvoker>);
    let (s, ..) = Sampler::begin(
        &*server,
        &client,
        SamplerConfig::default(),
        guard.cell(),
        "s",
        "p2",
        "Labone",
        1,
        "d",
    )
    .await
    .unwrap();
    drop(s);
    assert_eq!(server.count("server_packet_tap_stop"), 0);
    drop(guard);
    settle().await;
    let stops: Vec<Value> = server
        .calls()
        .into_iter()
        .filter(|c| c.0 == "server_packet_tap_stop")
        .map(|c| c.1)
        .collect();
    assert_eq!(stops, [json!({ "entity_id": 7 })]);

    // After finish, the guard has nothing left to stop.
    let server = Arc::new(FakeServer::default());
    let guard = TapStopGuard::new(server.clone() as Arc<dyn ServerInvoker>);
    let (mut s, ..) = Sampler::begin(
        &*server,
        &client,
        SamplerConfig::default(),
        guard.cell(),
        "s",
        "p2",
        "Labone",
        1,
        "d",
    )
    .await
    .unwrap();
    s.finish(2, "stop").await;
    drop(s);
    drop(guard);
    settle().await;
    assert_eq!(server.count("server_packet_tap_stop"), 1);
}

#[tokio::test]
async fn a_bad_instance_is_refused_before_any_call() {
    let server = FakeServer::default();
    let client = FakeClient::default();
    for bad in ["", "../p2", "p 2", "p2/x", "p2\\x"] {
        let err = Sampler::begin(
            &server,
            &client,
            SamplerConfig::default(),
            Arc::default(),
            "s",
            bad,
            "Labone",
            1,
            "d",
        )
        .await
        .err()
        .expect("refused");
        assert!(err.contains("[A-Za-z0-9_-]+"), "{bad:?}: {err}");
    }
    assert!(server.names().is_empty(), "{:?}", server.names());
    assert!(recording_id(1, "lab_2-b").unwrap().ends_with("-lab_2-b"));
}
