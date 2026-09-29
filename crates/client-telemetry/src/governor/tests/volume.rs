//! The measured mix: one lab client, one hour, SigNoz `client.native`
//! grouped by `client_target` (2026-09-29). Replayed through the governor
//! on the uploader's 2 s cadence, it must:
//!
//! 1. shrink by at least 90%;
//! 2. forward every must-keep event, unchanged;
//! 3. account for every event: per target, rows forwarded + repeat counts
//!    + rollup counts = events raised.

use std::collections::{BTreeMap, HashSet};

use serde_json::{json, Value};

use super::governor;
use crate::events::ClientNativeEvent;
use crate::governor::{classify, Class, ExternalDrops, ROLLUP_TARGET};

const HOUR_MS: i64 = 3_600_000;
const T0: i64 = 1_790_000_000_000;

/// (target, level, events per hour).
const MIX: &[(&str, &str, u64)] = &[
    ("client.engine.sequence_tick", "debug", 286_492),
    ("client.lua.pcall", "debug", 24_708),
    ("client.lua.call", "debug", 17_108),
    ("client.engine.async_archive_serialize", "debug", 2_185),
    // From the broken #1088 hook; its volume is not the governor's to fix,
    // but its events are must-keep and must all survive.
    ("client.mercury.error", "warn", 633),
    ("client.mercury.packet_in", "debug", 588),
    ("client.engine.static_load_object", "debug", 506),
    ("client.engine.tick", "debug", 320),
    ("client.cme.event", "debug", 223),
    ("client.entity.appearance_request", "info", 138),
    ("client.ui.cegui_log", "info", 126),
    ("client.mercury.entity_method", "debug", 94),
    ("client.net.out", "debug", 54),
    ("client.entity.create", "info", 31),
    ("client.entity.enter", "info", 28),
    ("client.entity.entered_world", "info", 25),
    ("client.streaming.update", "info", 4),
];

/// Deterministic, realistic-enough fields for the `i`th event of a stream.
fn fields_for(target: &str, i: u64) -> BTreeMap<String, Value> {
    let pairs: Vec<(&str, Value)> = match target {
        "client.engine.sequence_tick" => vec![("delta_time", json!(0.016))],
        "client.lua.pcall" | "client.lua.call" => {
            vec![("nargs", json!(i % 4)), ("nresults", json!(i % 2))]
        }
        "client.engine.async_archive_serialize" => vec![("length", json!(64 + i % 4096))],
        "client.engine.static_load_object" => vec![
            ("package_name", json!(format!("SGW_Pkg_{:02}", i % 40))),
            ("load_flags", json!(0)),
        ],
        "client.cme.event" => vec![
            ("name", json!(format!("Event_NetIn_{}", i % 12))),
            ("entity_id", json!(100 + i % 30)),
        ],
        "client.mercury.entity_method" => vec![
            ("entity_id", json!(100 + i % 30)),
            ("msg_id", json!(0x80 + i % 6)),
            ("path", json!("direct")),
        ],
        "client.net.out" => vec![("method", json!(i % 5)), ("entity_id", json!(1))],
        "client.mercury.packet_in" => vec![("len", json!(40 + i % 200))],
        // Mostly the same few lines: what the collapse is for.
        "client.ui.cegui_log" => vec![("text", json!(format!("line {}", i / 20)))],
        "client.streaming.update" => {
            vec![("level_name", json!(["Castle", "Harset"][(i % 2) as usize]))]
        }
        t if t.starts_with("client.entity.") => vec![("entity_id", json!(100 + i))],
        _ => vec![("n", json!(i))],
    };
    pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

#[test]
fn the_measured_mix_shrinks_by_90_percent_and_keeps_every_must_keep_event() {
    let mut g = governor();
    // Next index per stream; events of stream s are evenly spaced.
    let mut next = vec![0u64; MIX.len()];
    let ts_of = |s: usize, i: u64| T0 + (i as i64) * HOUR_MS / MIX[s].2 as i64;

    let mut raised: BTreeMap<String, u64> = BTreeMap::new();
    let mut must_keep: HashSet<u64> = HashSet::new();
    let mut out = Vec::new();
    let mut outputs = 0u64;
    let mut forwarded_kept: HashSet<u64> = HashSet::new();
    let mut accounted: BTreeMap<String, u64> = BTreeMap::new();
    let mut next_tick = T0 + 2_000;
    let mut seq = 0u64;

    let account = |out: &mut Vec<ClientNativeEvent>,
                   outputs: &mut u64,
                   forwarded_kept: &mut HashSet<u64>,
                   accounted: &mut BTreeMap<String, u64>| {
        for e in out.drain(..) {
            *outputs += 1;
            if e.target == crate::governor::HEALTH_TARGET {
                continue;
            }
            if e.target == ROLLUP_TARGET {
                let t = e.fields["rollup_target"].as_str().unwrap().to_string();
                *accounted.entry(t).or_default() += e.fields["count"].as_u64().unwrap();
            } else if let Some(n) = e.fields.get("repeat_count") {
                *accounted.entry(e.target).or_default() += n.as_u64().unwrap();
            } else {
                forwarded_kept.insert(e.seq);
                *accounted.entry(e.target).or_default() += 1;
            }
        }
    };

    loop {
        // The stream with the earliest pending event.
        let pick = (0..MIX.len())
            .filter(|&s| next[s] < MIX[s].2)
            .min_by_key(|&s| (ts_of(s, next[s]), s));
        let Some(s) = pick else { break };
        let (target, level, _) = MIX[s];
        let ts = ts_of(s, next[s]);
        while ts >= next_tick {
            g.tick(next_tick, ExternalDrops::default(), &mut out);
            next_tick += 2_000;
        }
        let fields = fields_for(target, next[s]);
        next[s] += 1;
        if matches!(classify(target, level, &fields).class, Class::MustKeep(_)) {
            must_keep.insert(seq);
        }
        *raised.entry(target.to_string()).or_default() += 1;
        g.admit(
            ClientNativeEvent {
                ts_ms: ts,
                seq,
                target: target.to_string(),
                level: level.to_string(),
                fields,
            },
            &mut out,
        );
        seq += 1;
        account(&mut out, &mut outputs, &mut forwarded_kept, &mut accounted);
    }
    g.finish(T0 + HOUR_MS, ExternalDrops::default(), &mut out);
    account(&mut out, &mut outputs, &mut forwarded_kept, &mut accounted);

    let input: u64 = raised.values().sum();
    let reduction = 100.0 * (1.0 - outputs as f64 / input as f64);
    eprintln!(
        "measured mix: {input} events in, {outputs} out ({reduction:.2}% reduction), {} must-keep",
        must_keep.len()
    );

    assert!(
        outputs * 10 <= input,
        "{outputs} of {input} is less than a 90% cut"
    );
    let lost: Vec<_> = must_keep.difference(&forwarded_kept).collect();
    assert!(lost.is_empty(), "must-keep events lost: {}", lost.len());
    assert_eq!(accounted, raised, "every event is accounted for");
    // 633 + 138 + 31 + 28 + 25 must-keep in the mix.
    assert_eq!(must_keep.len(), 855);
}
