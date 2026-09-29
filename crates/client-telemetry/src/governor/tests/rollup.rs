//! Rollup math: every number in a rollup event is exact.

use serde_json::{json, Value};

use super::{admit, ev, governor, rollups_for};
use crate::governor::rollup::{Rollup, RollupReason, MAX_KEYS, TOP_N};
use crate::governor::{ExternalDrops, GovernorConfig};

fn f(r: &Rollup, k: &str) -> Value {
    r.to_fields(0, 10_000, "window")[k].clone()
}

#[test]
fn counts_timestamps_and_rate_are_exact() {
    let mut r = Rollup::new("client.lua.pcall", RollupReason::HotStream, None);
    for ts in [500, 100, 9_000] {
        r.add(&ev("client.lua.pcall", "debug", ts, &[]), None);
    }
    assert_eq!(f(&r, "count"), json!(3));
    assert_eq!(f(&r, "first_ts_ms"), json!(100));
    assert_eq!(f(&r, "last_ts_ms"), json!(9_000));
    assert_eq!(f(&r, "window_ms"), json!(10_000));
    // 3 events over 10 s.
    assert_eq!(f(&r, "rate_per_sec"), json!(0.3));
    assert_eq!(f(&r, "reason"), json!("hot_stream"));
    assert_eq!(f(&r, "rollup_target"), json!("client.lua.pcall"));
}

/// Top-N is exact, ordered by count then key, and the keys outside it are
/// reported so the list plus the remainder adds back to the total.
#[test]
fn top_keys_are_exact_and_the_remainder_adds_up() {
    let target = "client.engine.static_load_object";
    let mut r = Rollup::new(target, RollupReason::HotStream, Some("package_name"));
    let mut total = 0u64;
    // Key k<i> appears i times, for i in 1..=15.
    for i in 1..=15u64 {
        for _ in 0..i {
            r.add(
                &ev(
                    target,
                    "debug",
                    1,
                    &[("package_name", json!(format!("k{i:02}")))],
                ),
                None,
            );
            total += 1;
        }
    }
    // A tie at the boundary resolves by key.
    r.add(
        &ev(target, "debug", 2, &[("package_name", json!("a_tie"))]),
        None,
    );
    total += 1;

    let top = r.top_keys();
    assert_eq!(top.len(), TOP_N);
    assert_eq!(top[0], ("k15".to_string(), 15));
    assert_eq!(top[9], ("k06".to_string(), 6));
    let fields = r.to_fields(0, 1_000, "window");
    let top_sum: u64 = top.iter().map(|(_, n)| n).sum();
    assert_eq!(fields["other_key_events"], json!(total - top_sum));
    assert_eq!(fields["distinct_keys"], json!(16));
    assert_eq!(fields["distinct_keys_capped"], json!(false));
    assert_eq!(fields["key_field"], json!("package_name"));
    assert_eq!(fields["last_key"], json!("a_tie"));
    assert_eq!(fields["top_keys"][0], json!({"key": "k15", "count": 15}));
}

#[test]
fn numeric_fields_carry_min_max_sum_and_n() {
    let mut r = Rollup::new("client.lua.call", RollupReason::HotStream, None);
    for (nargs, nresults) in [(2, 0), (5, 1), (-1, 3)] {
        r.add(
            &ev(
                "client.lua.call",
                "debug",
                1,
                &[("nargs", json!(nargs)), ("nresults", json!(nresults))],
            ),
            None,
        );
    }
    let fields = r.to_fields(0, 1_000, "window");
    assert_eq!(
        fields["numeric"]["nargs"],
        json!({"n": 3, "min": -1.0, "max": 5.0, "sum": 6.0})
    );
    assert_eq!(
        fields["numeric"]["nresults"],
        json!({"n": 3, "min": 0.0, "max": 3.0, "sum": 4.0})
    );
    // The last event survives as an exemplar.
    assert_eq!(fields["last_fields"], json!({"nargs": -1, "nresults": 3}));
}

/// Past MAX_KEYS distinct keys the count stays exact; only the key detail
/// stops growing, and the rollup says so.
#[test]
fn the_key_table_is_capped_but_the_count_is_not() {
    let mut r = Rollup::new("t", RollupReason::OverBudget, Some("name"));
    let n = MAX_KEYS + 50;
    for i in 0..n {
        r.add(&ev("t", "debug", 1, &[("name", json!(i))]), None);
    }
    let fields = r.to_fields(0, 1_000, "window");
    assert_eq!(fields["count"], json!(n));
    assert_eq!(fields["distinct_keys"], json!(MAX_KEYS));
    assert_eq!(fields["distinct_keys_capped"], json!(true));
    let top_sum: u64 = r.top_keys().iter().map(|(_, c)| c).sum();
    assert_eq!(fields["other_key_events"], json!(n as u64 - top_sum));
}

/// Through the governor: a hot stream forwards nothing and the window
/// closes into one rollup with the exact count.
#[test]
fn a_hot_stream_becomes_one_rollup_per_window() {
    let mut g = governor();
    for i in 0..1_000 {
        let out = admit(
            &mut g,
            ev(
                "client.engine.sequence_tick",
                "debug",
                i,
                &[("delta_time", json!(0.016))],
            ),
        );
        assert!(out.is_empty(), "a hot event is never forwarded");
    }
    let mut out = Vec::new();
    g.tick(5_000, ExternalDrops::default(), &mut out);
    assert!(out.is_empty(), "the window is not due yet");
    g.tick(10_000, ExternalDrops::default(), &mut out);
    let rolls: Vec<_> = rollups_for(&out, "client.engine.sequence_tick").collect();
    assert_eq!(rolls.len(), 1);
    assert_eq!(rolls[0].fields["count"], json!(1_000));
    assert_eq!(rolls[0].fields["rate_per_sec"], json!(100.0));
    assert_eq!(rolls[0].fields["trigger"], json!("window"));
    assert_ne!(rolls[0].seq, 0, "generated events are stamped");
}

/// A scene change closes the window early so two maps never share a rollup.
#[test]
fn a_scene_change_closes_the_window() {
    let mut g = governor();
    admit(
        &mut g,
        ev(
            "client.streaming.update",
            "info",
            0,
            &[("level_name", json!("Castle"))],
        ),
    );
    for i in 0..10 {
        admit(&mut g, ev("client.lua.pcall", "debug", i, &[]));
    }
    let out = admit(
        &mut g,
        ev(
            "client.streaming.update",
            "info",
            20,
            &[("level_name", json!("Harset"))],
        ),
    );
    let rolls: Vec<_> = rollups_for(&out, "client.lua.pcall").collect();
    assert_eq!(rolls.len(), 1);
    assert_eq!(rolls[0].fields["count"], json!(10));
    assert_eq!(rolls[0].fields["trigger"], json!("scene_change"));
    // The streaming event itself still comes out, after the rollup.
    assert_eq!(out.last().unwrap().target, "client.streaming.update");
}

/// A budgeted target forwards its burst, then its sustained rate, and
/// rolls up the rest with the exact count.
#[test]
fn over_budget_events_are_rolled_up_not_dropped() {
    let cfg = GovernorConfig::default();
    let mut g = governor();
    let mut forwarded = 0u64;
    let n = 1_000u64;
    for i in 0..n {
        // Distinct fields so the collapse does not absorb them.
        let out = admit(
            &mut g,
            ev("client.ui.cegui_log", "info", 0, &[("i", json!(i))]),
        );
        forwarded += out.len() as u64;
    }
    assert_eq!(forwarded, u64::from(cfg.budget_burst));
    let mut out = Vec::new();
    g.tick(cfg.window_ms, ExternalDrops::default(), &mut out);
    let roll = rollups_for(&out, "client.ui.cegui_log").next().unwrap();
    assert_eq!(roll.fields["count"], json!(n - forwarded));
    assert_eq!(roll.fields["reason"], json!("over_budget"));
}
