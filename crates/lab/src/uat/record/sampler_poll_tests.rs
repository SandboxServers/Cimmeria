//! Sampler tests, continued: the known-server-event poll, the caps and
//! the snapshot SQL.

use std::sync::Arc;

use serde_json::{json, Value};

use super::*;

#[tokio::test]
async fn known_server_events_are_polled_every_second_tick() {
    let line = |ts: u64, event: &str| {
        json!({ "timestamp_ms": ts, "target": "aoi.cinematic_hold",
                "message": "Cinematic AoI hold: released", "fields": { "event": event } })
    };
    let server = FakeServer::default().answer(
        "server_log_tail",
        vec![
            // The mark at begin, then tick 0's poll.
            ok(json!({ "entries": [], "newest_ms": 1000 })),
            ok(json!({ "entries": [] })),
            // Tick 2: an old line, a wrong event, and the release.
            ok(json!({ "entries": [line(900, "hold_released"), line(1100, "hold_started"), line(1200, "hold_released")] })),
            ok(json!({ "entries": [] })),
        ],
    );
    let client = FakeClient::default();
    let (mut s, _, _) = begin(&server, &client).await;
    let t1 = s.tick(1_500).await;
    assert!(t1.server_events.is_empty());
    let t2 = s.tick(2_000).await;
    assert_eq!(t2.server_events, ["cinematic-hold"]);
    s.tick(2_500).await;
    s.tick(3_000).await;

    let polls: Vec<Value> = server
        .calls()
        .into_iter()
        .filter(|c| c.0 == "server_log_tail")
        .map(|c| c.1)
        .collect();
    // Mark, tick 0, tick 2, tick 4: odd ticks do not poll.
    assert_eq!(polls.len(), 4, "{polls:?}");
    assert_eq!(polls[0]["target"], "aoi.cinematic_hold");
    assert_eq!(polls[1]["since_ms"], 1000);
    // The matched line moved the mark; the wrong event did not.
    assert_eq!(polls[3]["since_ms"], 1200);
}

#[tokio::test]
async fn the_caps_stop_a_recording() {
    let server = FakeServer::default();
    let client = FakeClient::default();
    let cfg = SamplerConfig {
        max_ticks: 3,
        ..Default::default()
    };
    let (mut s, ..) = Sampler::begin(
        &server,
        &client,
        cfg,
        Arc::default(),
        "s",
        "p2",
        "Labone",
        1,
        "d",
    )
    .await
    .unwrap();
    s.tick(2).await;
    assert_eq!(s.full(), None);
    s.tick(3).await;
    assert_eq!(s.full(), Some("max_minutes"));

    let cfg = SamplerConfig {
        max_bytes: 1,
        ..Default::default()
    };
    let (s, ..) = Sampler::begin(
        &server,
        &client,
        cfg,
        Arc::default(),
        "s",
        "p2",
        "Labone",
        1,
        "d",
    )
    .await
    .unwrap();
    assert_eq!(s.full(), Some("size_cap"));
}

#[test]
fn snapshot_sql_is_the_contracts() {
    let [m, p, i] = snapshot_sql(42);
    assert_eq!(m, "SELECT mission_id, status, current_step_id AS step, repeats FROM sgw_mission WHERE player_id = 42 ORDER BY mission_id");
    assert_eq!(
        p,
        "SELECT level, exp, naquadah, world_location FROM sgw_player WHERE player_id = 42"
    );
    assert_eq!(i, "SELECT i.type_id, i.container_id, coalesce(sum(coalesce(i.stack_size,1)),0)::int AS count, min(t.name) AS name FROM sgw_inventory i LEFT JOIN resources.items t ON t.item_id = i.type_id WHERE i.character_id = 42 GROUP BY i.type_id, i.container_id ORDER BY 1, 2");
}
