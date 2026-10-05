//! Ability-mechanics AB-T7: the ability telemetry's log targets reach SigNoz
//! at the level they emit, and no ability row logs under a module path.
//!
//! The ability rows ride three `OTEL_FILTER` rows: the `abilities=debug`
//! prefix row (`abilities`, `abilities.qr`, `.effect`, `.ledger`, `.pulse`,
//! `.wire`, `.sequence`, `.snapshot`, `.gm`, and AB-N1's `.debug`),
//! `vitals=debug` (regen) and `base.entity_method=debug` (the base's
//! delivery of a cell's client method, AB-T4). The `cast_id` forensics query
//! in `docs/gameplay/ability-system.md` ("Reading one cast") reads exactly
//! these targets, so a row that stops reaching the exporter is a hole in
//! every cast's story.
//!
//! A row with no `target:` logs under its module path
//! (`cimmeria_cell_combat::cell::abilities::…`), which the forensics query
//! and the SigNoz views do not select. [`no_ability_row_logs_under_a_module_path`]
//! fails on one.

use std::collections::BTreeSet;
use std::path::Path;

use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Layer};

use super::filters::{FILE_LAYERS, OTEL_FILTER};
use super::parity_tests::{harness, sinks_for, OTLP_LOG_SINKS};
use super::target_scan_tests::{
    crates_dir, emitted_targets, is_test_path, rs_files, strip_test_module,
};

/// The targets the forensics query reads.
fn is_ability_target(target: &str) -> bool {
    target == "abilities"
        || target.starts_with("abilities.")
        || target == "vitals"
        || target == "base.entity_method"
}

/// Source directories that hold ability rows, relative to `crates/`. Every
/// tracing event in them names its target.
const ABILITY_SOURCE_DIRS: &[&str] = &[
    "cell-combat/src/cell/abilities",
    "cell-combat/src/cell/effects",
    "cell-world/src/cell/effects",
    "cell-effect-scripts/src",
];

/// Every `(target, level)` an ability row is emitted at reaches exactly one
/// OTLP log index. Each site is found by the source scan, so a new ability
/// target or level is covered the day it is written.
#[test]
fn every_ability_row_reaches_one_otlp_index_at_its_level() {
    let (dispatch, hits) = harness(FILE_LAYERS);
    let mut bad = Vec::new();
    let mut seen = 0;
    for ((target, level), site) in emitted_targets() {
        if !is_ability_target(&target) {
            continue;
        }
        seen += 1;
        let sinks = sinks_for(&dispatch, &hits, &target, level);
        let n = OTLP_LOG_SINKS
            .iter()
            .filter(|s| sinks.contains(**s))
            .count();
        if n != 1 {
            bad.push(format!(
                "{target} at {level} ({site}) reaches {n} OTLP indexes"
            ));
        }
    }
    assert!(
        seen >= 15,
        "the scan found only {seen} ability (target, level) pairs"
    );
    assert!(
        bad.is_empty(),
        "ability rows SigNoz misses:\n{}",
        bad.join("\n")
    );
}

/// The scan finds the rows the forensics query depends on, at the level
/// they are written, so the guard above cannot pass on an empty list.
#[test]
fn scan_finds_every_ability_target() {
    let sites = emitted_targets();
    let missing: Vec<_> = [
        ("abilities", Level::DEBUG),
        ("abilities", Level::INFO),
        ("abilities", Level::WARN),
        ("abilities.qr", Level::DEBUG),
        ("abilities.effect", Level::DEBUG),
        ("abilities.ledger", Level::DEBUG),
        ("abilities.pulse", Level::DEBUG),
        ("abilities.wire", Level::DEBUG),
        ("abilities.wire", Level::WARN),
        ("abilities.sequence", Level::DEBUG),
        ("abilities.snapshot", Level::INFO),
        ("abilities.gm", Level::INFO),
        ("vitals", Level::DEBUG),
        ("base.entity_method", Level::DEBUG),
        ("base.entity_method", Level::WARN),
    ]
    .into_iter()
    .filter(|(t, l)| !sites.contains_key(&(t.to_string(), *l)))
    .collect();
    assert!(missing.is_empty(), "the scan found no row for {missing:?}");
}

/// Records the target and level of every event the filter lets through.
struct Seen(std::sync::Arc<std::sync::Mutex<Vec<(String, Level)>>>);

impl<S: tracing::Subscriber> Layer<S> for Seen {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let m = event.metadata();
        self.0
            .lock()
            .unwrap()
            .push((m.target().to_string(), *m.level()));
    }
}

/// `OTEL_FILTER` itself exports each ability target at DEBUG and above. A
/// literal-target pin, independent of the scan: drop the `abilities=debug`,
/// `vitals=debug` or `base.entity_method=debug` row and this fails even if
/// the scan stops seeing a site. `abilities.debug` is AB-N1's combat-debug
/// target, pinned before its emitter lands.
#[test]
fn otel_filter_exports_the_ability_targets_from_debug() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry()
        .with(Seen(seen.clone()).with_filter(EnvFilter::new(OTEL_FILTER)));
    tracing::subscriber::with_default(subscriber, || {
        tracing::trace!(target: "abilities", "below the filter");
        tracing::debug!(target: "abilities", event = "ability_launched", "a");
        tracing::warn!(target: "abilities", event = "use_ability_not_known", "a");
        tracing::debug!(target: "abilities.qr", event = "qr_rolled", "q");
        tracing::debug!(target: "abilities.effect", event = "effect_planned", "e");
        tracing::debug!(target: "abilities.ledger", event = "ledger", "l");
        tracing::debug!(target: "abilities.pulse", event = "pulse_ticked", "p");
        tracing::debug!(target: "abilities.wire", event = "wire_sent", "w");
        tracing::warn!(target: "abilities.wire", event = "wire_send_failed", "w");
        tracing::debug!(target: "abilities.sequence", "s");
        tracing::info!(target: "abilities.snapshot", event = "ability_snapshot", "s");
        tracing::info!(target: "abilities.gm", "g");
        tracing::debug!(target: "abilities.debug", "d");
        tracing::debug!(target: "vitals", event = "regen_skipped", "v");
        tracing::debug!(target: "base.entity_method", event = "client_sent", "b");
    });
    let got: Vec<_> = seen.lock().unwrap().clone();
    let want: Vec<(String, Level)> = [
        ("abilities", Level::DEBUG),
        ("abilities", Level::WARN),
        ("abilities.qr", Level::DEBUG),
        ("abilities.effect", Level::DEBUG),
        ("abilities.ledger", Level::DEBUG),
        ("abilities.pulse", Level::DEBUG),
        ("abilities.wire", Level::DEBUG),
        ("abilities.wire", Level::WARN),
        ("abilities.sequence", Level::DEBUG),
        ("abilities.snapshot", Level::INFO),
        ("abilities.gm", Level::INFO),
        ("abilities.debug", Level::DEBUG),
        ("vitals", Level::DEBUG),
        ("base.entity_method", Level::DEBUG),
    ]
    .into_iter()
    .map(|(t, l)| (t.to_string(), l))
    .collect();
    assert_eq!(
        got, want,
        "OTEL_FILTER must export every ability target from DEBUG"
    );
}

/// Every `<level>!(` call in `src` whose first argument is not `target:`,
/// with its line.
fn untargeted(src: &str) -> Vec<(usize, String)> {
    const LEVELS: [&str; 5] = ["trace", "debug", "info", "warn", "error"];
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = src[from..].find("!(") {
        let bang = from + i;
        from = bang + 2;
        let name_start = src[..bang]
            .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .map_or(0, |p| p + 1);
        let name = &src[name_start..bang];
        if !LEVELS.contains(&name) {
            continue;
        }
        if src[from..].trim_start().starts_with("target:") {
            continue;
        }
        let line = src[..bang].matches('\n').count() + 1;
        out.push((line, name.to_string()));
    }
    out
}

/// **The guard.** No ability row logs under a module path: each names
/// `abilities`, an `abilities.*` child, or another named target. A failure
/// names the file and line; give the row a `target:` (see the target table
/// in `docs/analysis/ability-mechanics/lab-uat-and-telemetry.md`).
#[test]
fn no_ability_row_logs_under_a_module_path() {
    let root = crates_dir();
    let mut bad = BTreeSet::new();
    let mut files_seen = 0;
    for dir in ABILITY_SOURCE_DIRS {
        let mut files = Vec::new();
        rs_files(&root.join(dir), &mut files);
        assert!(!files.is_empty(), "{dir} holds no sources: moved?");
        for f in files {
            let rel = f.strip_prefix(&root).unwrap().to_path_buf();
            if is_test_path(&rel) {
                continue;
            }
            files_seen += 1;
            let src = std::fs::read_to_string(&f).unwrap();
            for (line, level) in untargeted(strip_test_module(&src)) {
                bad.insert(format!("{}:{line} {level}!", rel.display()));
            }
        }
    }
    assert!(
        files_seen > 50,
        "only {files_seen} ability source files scanned"
    );
    assert!(
        bad.is_empty(),
        "ability rows logging under a module path:\n{}",
        bad.into_iter().collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn untargeted_finds_module_path_rows_only() {
    let src = r#"
        tracing::debug!(target: "abilities", "named");
        tracing::warn!(
            target: "abilities.wire",
            "named"
        );
        tracing::info!(entity_id, "module path");
        debug!(
            x = 1,
            "module path"
        );
        debug_assert!(true);
        let _ = tracing::info_span!("span");
    "#;
    let got: Vec<_> = untargeted(src).into_iter().map(|(_, l)| l).collect();
    assert_eq!(got, ["info", "debug"]);
}

/// The directory list still points at real code.
#[test]
fn ability_source_dirs_exist() {
    for dir in ABILITY_SOURCE_DIRS {
        assert!(
            Path::new(&crates_dir().join(dir)).is_dir(),
            "{dir} is gone: update ABILITY_SOURCE_DIRS"
        );
    }
}
