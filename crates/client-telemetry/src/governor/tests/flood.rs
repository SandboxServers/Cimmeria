//! Must-keep events survive a flood, through the real producer and ring.
//!
//! Before the governor, a hot stream and a must-keep event competed for
//! the same 4096 ring slots: a burst of the one dropped the other. These
//! tests fail if the governor is taken out of `governed_channel`.

use serde_json::json;

use crate::events::ClientNativeEvent;
use crate::governor::{GovernorConfig, ROLLUP_TARGET};
use crate::queue::{governed_channel, RING_CAPACITY};

fn drain(c: &crate::queue::Consumer) -> Vec<ClientNativeEvent> {
    std::iter::from_fn(|| c.try_recv()).collect()
}

#[test]
fn entity_lifecycle_survives_a_hot_flood_with_nothing_dropped() {
    let (p, c) = governed_channel(GovernorConfig::default());
    let floods = RING_CAPACITY * 50;
    let mut kept = 0;
    for i in 0..floods {
        p.try_emit(
            ClientNativeEvent::builder("client.engine.sequence_tick", "debug")
                .field("delta_time", json!(0.016)),
        );
        p.try_emit(
            ClientNativeEvent::builder("client.lua.pcall", "debug").field("nargs", json!(i % 4)),
        );
        if i % 1_000 == 0 {
            p.try_emit(
                ClientNativeEvent::builder("client.entity.create", "info")
                    .field("entity_id", json!(i)),
            );
            kept += 1;
        }
    }
    assert_eq!(p.dropped_total(), 0, "the flood never reached the ring");
    let got = drain(&c);
    let creates: Vec<_> = got
        .iter()
        .filter(|e| e.target == "client.entity.create")
        .collect();
    assert_eq!(creates.len(), kept);
    // In order, with their own fields.
    for (n, e) in creates.iter().enumerate() {
        assert_eq!(e.fields["entity_id"], json!(n * 1_000));
    }
    assert!(got.iter().all(|e| e.target == "client.entity.create"));

    // The flood is accounted for in the rollups at the next close.
    let mut batch = Vec::new();
    c.governor_finish(0, &mut batch);
    let total: u64 = batch
        .iter()
        .filter(|e| e.target == ROLLUP_TARGET)
        .map(|e| e.fields["count"].as_u64().unwrap())
        .sum();
    assert_eq!(total, 2 * floods as u64);
}

/// Warn/error and per-entity first sightings get through a budgeted flood
/// of distinct lines that would otherwise fill the ring.
#[test]
fn warnings_and_first_sightings_survive_a_budgeted_flood() {
    let (p, c) = governed_channel(GovernorConfig::default());
    for i in 0..(RING_CAPACITY * 10) {
        p.try_emit(
            ClientNativeEvent::builder("client.ui.cegui_log", "info")
                .field("text", json!(format!("line {i}"))),
        );
        if i % 500 == 0 {
            p.try_emit(
                ClientNativeEvent::builder("client.ui.cegui_log", "warn")
                    .field("text", json!(format!("problem {i}"))),
            );
            p.try_emit(
                ClientNativeEvent::builder("client.cme.event", "debug")
                    .field("name", json!("Event_NetIn_onDialogDisplay"))
                    .field("entity_id", json!(i)),
            );
        }
    }
    assert_eq!(p.dropped_total(), 0);
    let got = drain(&c);
    let warns = got.iter().filter(|e| e.level == "warn").count();
    let sightings = got
        .iter()
        .filter(|e| e.target == "client.cme.event")
        .count();
    let expected = (RING_CAPACITY * 10).div_ceil(500);
    assert_eq!(warns, expected);
    assert_eq!(sightings, expected);
}

/// `raw` forwards everything: the lab's deliberate full-volume mode.
#[test]
fn raw_mode_forwards_every_event() {
    let cfg = GovernorConfig {
        raw: true,
        ..GovernorConfig::default()
    };
    let (p, c) = governed_channel(cfg);
    for _ in 0..100 {
        p.try_emit(ClientNativeEvent::builder(
            "client.engine.sequence_tick",
            "debug",
        ));
    }
    assert_eq!(drain(&c).len(), 100);
}

/// A ring drop is reported: the health event carries the count and is a
/// warn, so it cannot itself be summarized away.
#[test]
fn ring_drops_are_reported_by_the_health_event() {
    let (p, c) = governed_channel(GovernorConfig::default());
    for i in 0..(RING_CAPACITY + 7) {
        // Must-keep, so every one of them tries for a ring slot.
        p.try_emit(
            ClientNativeEvent::builder("client.entity.create", "info").field("entity_id", json!(i)),
        );
    }
    assert_eq!(p.dropped_total(), 7);
    let mut batch = Vec::new();
    c.governor_finish(3, &mut batch);
    let health = batch
        .iter()
        .find(|e| e.target == crate::governor::HEALTH_TARGET)
        .unwrap();
    assert_eq!(health.level, "warn");
    assert_eq!(health.fields["ring_dropped_total"], json!(7));
    assert_eq!(health.fields["upload_dropped_total"], json!(3));
}
