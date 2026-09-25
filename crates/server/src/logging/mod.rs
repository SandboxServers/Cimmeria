//! Tracing subscriber setup. `main` calls [`init_logging`] once at startup and
//! must hold the returned `WorkerGuard`s for the process lifetime — dropping one
//! flushes and closes its log file.

use std::fs;
use std::path::Path;

use tokio::sync::broadcast;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;

use cimmeria_admin_api::ws::broadcast_layer::{BroadcastLayer, LogBuffer, LogEntry};

use crate::otel;

mod filters;
#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod target_scan_tests;

use filters::{
    otel_network_log_filter, otel_server_log_filter, otel_trace_log_filter, server_log_directives,
    FILE_LAYERS, OTEL_FILTER, WIRE_FIREHOSE_MUTED,
};

// ── Logging ──────────────────────────────────────────────────────────────────

/// Archive any `.log` files from a previous session into `logs/archive/<timestamp>/`.
fn archive_previous_logs() {
    archive_previous_logs_in(Path::new("logs"));
}

/// Archive any `.log` files in `logs_dir` into `<logs_dir>/archive/<timestamp>/`.
///
/// Split from [`archive_previous_logs`] (which pins the real `logs/` path) so the
/// archival behaviour — move only `*.log`, leave everything else, no-op on an
/// empty or missing directory — is unit-testable against a temp directory.
fn archive_previous_logs_in(logs_dir: &Path) {
    if !logs_dir.exists() {
        return;
    }

    let entries: Vec<_> = fs::read_dir(logs_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "log"))
        .collect();

    if entries.is_empty() {
        return;
    }

    // Second-resolution timestamps collide on two restarts in the same second;
    // on Windows `fs::rename` into an existing dir fails, so disambiguate.
    let ts = chrono_timestamp();
    let mut archive_dir = logs_dir.join("archive").join(&ts);
    let mut n = 1u32;
    while archive_dir.exists() {
        archive_dir = logs_dir.join("archive").join(format!("{ts}-{n:02}"));
        n += 1;
    }
    if let Err(e) = fs::create_dir_all(&archive_dir) {
        eprintln!(
            "Failed to create archive directory {}: {e}",
            archive_dir.display()
        );
        return;
    }

    for entry in &entries {
        let src = entry.path();
        let dst = archive_dir.join(entry.file_name());
        if let Err(e) = fs::rename(&src, &dst) {
            eprintln!("Failed to archive {}: {e}", src.display());
        }
    }

    eprintln!(
        "Archived {} log file(s) to {}",
        entries.len(),
        archive_dir.display()
    );
}

/// Generate a filesystem-safe timestamp like `2026-03-03T14-30-22`.
fn chrono_timestamp() -> String {
    use std::time::SystemTime;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Simple UTC breakdown (no chrono dependency needed).
    let secs_per_day: u64 = 86400;
    let days = now / secs_per_day;
    let day_secs = now % secs_per_day;
    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    let seconds = day_secs % 60;

    // Days since epoch to Y-M-D (simplified Gregorian).
    let (year, month, day) = days_to_ymd(days);

    format!(
        "{:04}-{:02}-{:02}T{:02}-{:02}-{:02}",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from Howard Hinnant's `civil_from_days`.
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u64, m, d)
}

/// Initialise the layered tracing subscriber (console + per-system `logs/*.log`
/// files + admin-WS broadcast + optional OTLP). The returned `WorkerGuard`s must
/// outlive the process.
///
/// Most events carry `entity_id` / `player_id` / `witness_id`, so a player can be
/// followed across files with `grep entity_id=42 logs/*.log`.
pub(crate) fn init_logging(
    log_tx: broadcast::Sender<LogEntry>,
    log_buffer: LogBuffer,
    otel_layers: Option<otel::OtelLayers>,
) -> Vec<WorkerGuard> {
    // Move previous session's logs into archive/.
    archive_previous_logs();

    // Ensure logs/ directory exists.
    let _ = fs::create_dir_all("logs");

    let mut guards = Vec::new();

    type BoxLayer = Box<dyn tracing_subscriber::Layer<tracing_subscriber::Registry> + Send + Sync>;

    // Boxed (not nested generics) because the deeply-nested layer type otherwise
    // makes the type-checker consume 50+ GB of RAM.
    macro_rules! log_layer {
        ($filename:expr, $filter:expr) => {{
            let file = tracing_appender::rolling::never("logs", $filename);
            let (writer, guard) = tracing_appender::non_blocking(file);
            guards.push(guard);
            Box::new(
                fmt::layer()
                    .with_writer(writer)
                    .with_ansi(false)
                    .with_target(true)
                    .with_filter(EnvFilter::new($filter)),
            ) as BoxLayer
        }};
    }

    // ── Collect all layers into a Vec<BoxLayer> ─────────────────────────
    let mut layers: Vec<BoxLayer> = Vec::new();

    // ── Console (stdout, coloured, RUST_LOG or info) ─────────────────────
    // Default to `info` but mute the per-packet wire firehose — `mercury.packet`
    // (one INFO event per UDP datagram), the `wire.in`/`wire.out` method-call
    // stream, and retransmit noise. These stay at full fidelity in the file
    // layers (protocol.log) and OTLP; they just don't belong on an operator's
    // console. `RUST_LOG`, when set, overrides this entirely.
    // See `filters::WIRE_FIREHOSE_MUTED`.
    let console_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("info,{WIRE_FIREHOSE_MUTED}")));
    layers.push(Box::new(fmt::layer().with_filter(console_filter)));

    // ── server.log (JSON, all modules, info) ─────────────────────────────
    let server_file = tracing_appender::rolling::never("logs", "server.log");
    let (server_writer, guard) = tracing_appender::non_blocking(server_file);
    guards.push(guard);
    layers.push(Box::new(
        fmt::layer()
            .json()
            .with_writer(server_writer)
            .with_target(true)
            .with_filter(EnvFilter::new(server_log_directives())),
    ));

    // ── Per-system log files ─────────────────────────────────────────────
    // One layer per `FILE_LAYERS` row; the parity test walks the same table.
    for l in FILE_LAYERS {
        layers.push(log_layer!(l.file, l.directives));
    }

    // ── WebSocket broadcast (debug+, all modules) ─────────────────────
    layers.push(Box::new(
        BroadcastLayer::new(log_tx, log_buffer).with_filter(EnvFilter::new(format!(
            "debug,tungstenite=info,tokio_tungstenite=info,hyper=info,{WIRE_FIREHOSE_MUTED}"
        ))),
    ));

    // ── Discord notifications (optional — config-gated) ──────────────
    // The layer harvests `warn!`/`error!` events with structured fields
    // and posts them to the configured Discord channels. Disabled events
    // and missing channels short-circuit before the queue, so the layer
    // is cheap when Discord is off.
    if let Some(rt) = cimmeria_discord::global() {
        layers.push(Box::new(
            cimmeria_discord::DiscordLayer::new(rt.handle.clone(), rt.config.handle())
                // Layer applies its own per-event toggle gating inside
                // on_event — no env-filter needed here. Keep the
                // env-filter wide so we don't pre-filter out events the
                // user might want to enable at runtime via toggle.
                .with_filter(EnvFilter::new("warn")),
        ));
    }

    // ── OpenTelemetry → SigNoz (optional) ─────────────────────────────
    // Unlike the human-facing sinks above, OTLP keeps `mercury.packet` at info —
    // it's the load-bearing analytical surface here, so muting it would defeat
    // the purpose.
    //
    // Spans go through `OTEL_FILTER`. Log records split across THREE providers
    // (see `otel::init` and the routing table in `filters`), each filter
    // disjoint from the other two so a record lands in exactly one index:
    //
    // - `cimmeria-server`: DEBUG and above, minus DEBUG/INFO from
    //   network-noise scopes. WARN+ from those scopes stays here.
    // - `cimmeria-network`: DEBUG/INFO from network-noise scopes.
    // - `cimmeria-trace`: every TRACE row a file layer keeps, plus the
    //   custom targets and the sampled firehose rows. NA25.
    if let Some(otel) = otel_layers {
        layers.push(Box::new(
            otel.trace.with_filter(EnvFilter::new(OTEL_FILTER)),
        ));
        layers.push(Box::new(
            otel.server_log.with_filter(otel_server_log_filter()),
        ));
        layers.push(Box::new(
            otel.network_log.with_filter(otel_network_log_filter()),
        ));
        layers.push(Box::new(
            otel.trace_log.with_filter(otel_trace_log_filter()),
        ));
    }

    // Assemble the subscriber — one `.with()` call on the whole Vec.
    tracing_subscriber::registry().with(layers).init();

    guards
}

#[cfg(test)]
mod tests {
    use super::{archive_previous_logs_in, chrono_timestamp, days_to_ymd, OTEL_FILTER};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    // A unique temp directory per call — process id + a monotonic counter — so
    // tests don't collide under either `cargo test` (threads) or `cargo nextest`
    // (processes), and without pulling in a temp-dir dependency.
    fn unique_tempdir() -> PathBuf {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "cimmeria-log-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // `days_to_ymd` implements Howard Hinnant's `civil_from_days`. These pin the
    // boundaries that historically break naive date math: month rollover, the
    // non-leap-year February, a leap-year February 29th, and year rollovers —
    // including the year-2000 epoch the algorithm internally shifts to.
    #[test]
    fn days_to_ymd_epoch_and_month_boundaries() {
        assert_eq!(days_to_ymd(0), (1970, 1, 1)); // Unix epoch
        assert_eq!(days_to_ymd(30), (1970, 1, 31)); // last day of January
        assert_eq!(days_to_ymd(31), (1970, 2, 1)); // first day of February
        assert_eq!(days_to_ymd(59), (1970, 3, 1)); // 31 (Jan) + 28 (Feb 1970, non-leap)
    }

    #[test]
    fn days_to_ymd_year_and_leap_boundaries() {
        assert_eq!(days_to_ymd(365), (1971, 1, 1)); // 1970 is not a leap year
        assert_eq!(days_to_ymd(789), (1972, 2, 29)); // 1972 IS a leap year
        assert_eq!(days_to_ymd(790), (1972, 3, 1)); // the day after Feb 29
        assert_eq!(days_to_ymd(10957), (2000, 1, 1)); // Hinnant era anchor
    }

    // `chrono_timestamp` reads the wall clock, so we can't assert an exact value.
    // We can assert the filesystem-safe shape (no ':' that Windows rejects in a
    // path) and that the embedded date/time fields parse into sane ranges — which
    // exercises the full seconds→Y-M-D-H-M-S breakdown.
    #[test]
    fn chrono_timestamp_is_filesystem_safe_and_in_range() {
        let ts = chrono_timestamp();
        assert!(!ts.contains(':'), "timestamp must be path-safe: {ts}");

        let (date, time) = ts.split_once('T').expect("expected 'T' separator");
        let d: Vec<u64> = date.split('-').map(|p| p.parse().unwrap()).collect();
        let t: Vec<u64> = time.split('-').map(|p| p.parse().unwrap()).collect();
        assert_eq!(d.len(), 3);
        assert_eq!(t.len(), 3);
        assert!(d[0] >= 2020, "year looks wrong: {}", d[0]); // sanity vs. epoch math
        assert!((1..=12).contains(&d[1]), "month out of range: {}", d[1]);
        assert!((1..=31).contains(&d[2]), "day out of range: {}", d[2]);
        assert!(
            t[0] < 24 && t[1] < 60 && t[2] < 60,
            "time out of range: {time}"
        );
    }

    #[test]
    fn archive_missing_dir_is_noop() {
        let dir = unique_tempdir();
        let missing = dir.join("does-not-exist");
        archive_previous_logs_in(&missing); // must not panic
        assert!(!missing.exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn archive_empty_dir_creates_no_archive() {
        let dir = unique_tempdir();
        archive_previous_logs_in(&dir);
        assert!(!dir.join("archive").exists(), "no .log files -> no archive");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn archive_moves_only_log_files() {
        let dir = unique_tempdir();
        fs::write(dir.join("server.log"), b"old").unwrap();
        fs::write(dir.join("auth.log"), b"old").unwrap();
        fs::write(dir.join("notes.txt"), b"keep").unwrap(); // non-.log: must stay

        archive_previous_logs_in(&dir);

        // The two .log files moved out of the top level...
        assert!(!dir.join("server.log").exists());
        assert!(!dir.join("auth.log").exists());
        // ...the non-.log file stayed put...
        assert!(dir.join("notes.txt").exists());
        // ...and exactly one timestamped archive subdir was created holding them.
        let archive = dir.join("archive");
        let stamps: Vec<_> = fs::read_dir(&archive)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(stamps.len(), 1, "one timestamped archive subdir");
        let moved: Vec<_> = fs::read_dir(stamps[0].path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(moved.len(), 2, "both .log files archived: {moved:?}");
        fs::remove_dir_all(&dir).ok();
    }

    // A custom `target:` absent from `OTEL_FILTER` inherits the leading
    // `info`, which drops its DEBUG events before they reach SigNoz. These
    // seams exist to diagnose field bugs from SigNoz, so each must be named at
    // the level it emits.
    #[test]
    fn otel_filter_exports_the_debug_level_aoi_seams() {
        for directive in [
            "aoi.entity_enter=debug",
            "aoi.entity_leave=debug",
            "aoi.create_emit=debug",
            // NA00 / audit gap T1: DEBUG seams that never reached SigNoz.
            "wire.out.avatar_update=debug",
            // NA02: every FORCED_POSITION sent. Needs its own directive:
            // `wire.out=info` would otherwise drop it.
            "wire.out.forced_position=debug",
            "movement.navmesh=debug",
            "cover=debug",
            "spawner=debug",
            // Covers `npc_ai.transition`, `npc_ai.aggro` and every other
            // `npc_ai.*` target by prefix -- see the behavioural test below.
            "npc_ai=debug",
            // `content` WARN rows (`set_aggression_tag_miss`) and INFO
            // rows would pass at the default `info`; named so the target
            // is explicit and a later global raise cannot drop it.
            "content=info",
        ] {
            assert!(
                OTEL_FILTER.split(',').any(|d| d.trim() == directive),
                "OTEL_FILTER must carry `{directive}` or the seam never reaches SigNoz"
            );
        }
        OTEL_FILTER
            .parse::<tracing_subscriber::EnvFilter>()
            .expect("OTEL_FILTER must stay a valid EnvFilter directive string");
    }

    /// Records the target of every event the filter lets through.
    struct TargetLog(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

    impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for TargetLog {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            self.0
                .lock()
                .unwrap()
                .push(event.metadata().target().to_string());
        }
    }

    /// Behavioural pin for the prefix rule the filter relies on: the new
    /// `npc_ai.*` targets reach the exporter through `npc_ai=debug` without
    /// their own directives, the more specific `wire.out.avatar_update`
    /// beats `wire.out=info`, and a sibling `wire.out.*` DEBUG row is still
    /// dropped. Fails if `tracing-subscriber` ever switches to exact-target
    /// matching, or if one of these directives is removed.
    #[test]
    fn otel_filter_prefix_matching_exports_npc_ai_children() {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::{EnvFilter, Layer};

        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry()
            .with(TargetLog(seen.clone()).with_filter(EnvFilter::new(OTEL_FILTER)));
        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!(target: "npc_ai.transition", "t");
            tracing::info!(target: "npc_ai.aggro", "a");
            tracing::debug!(target: "npc_ai.leash", "l");
            tracing::debug!(target: "wire.out.avatar_update", "w");
            tracing::debug!(target: "movement.navmesh", "m");
            tracing::debug!(target: "cover.selection", "c");
            tracing::debug!(target: "spawner.npc_behaviour", "s");
            tracing::warn!(target: "content", "x");
            // Must be filtered out:
            tracing::debug!(target: "wire.out.other", "dropped");
            tracing::debug!(target: "threat", "dropped");
        });
        let seen = seen.lock().unwrap().clone();
        assert_eq!(
            seen,
            [
                "npc_ai.transition",
                "npc_ai.aggro",
                "npc_ai.leash",
                "wire.out.avatar_update",
                "movement.navmesh",
                "cover.selection",
                "spawner.npc_behaviour",
                "content",
            ],
            "OTEL_FILTER passed the wrong set of targets"
        );
    }

    /// NA02: every detector target, at the level it emits, reaches the
    /// exporter. Most ride a prefix directive (`npc_ai=debug`,
    /// `cover=debug`, `spawner=debug`, `movement.npc=debug`, `threat=info`);
    /// `wire.out.forced_position` needs its own. Removing any directive that
    /// one of these depends on fails here.
    #[test]
    fn otel_filter_exports_every_na02_detector_target() {
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::{EnvFilter, Layer};

        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry()
            .with(TargetLog(seen.clone()).with_filter(EnvFilter::new(OTEL_FILTER)));
        tracing::subscriber::with_default(subscriber, || {
            tracing::debug!(target: "npc_ai.path", "request");
            tracing::warn!(target: "npc_ai.path_fail", "partial");
            tracing::debug!(target: "npc_ai.los", "blocked");
            tracing::info!(target: "npc_ai.leash", "enter");
            tracing::warn!(target: "npc_ai.leash", "loop");
            tracing::debug!(target: "npc_ai.leash", "damage_ignored");
            tracing::debug!(target: "npc_ai.aggro_scan", "candidate_rejected");
            tracing::debug!(target: "npc_ai.idle", "unticked");
            tracing::info!(target: "npc_ai.idle_parked", "idle_parked");
            tracing::warn!(target: "npc_ai", "stuck");
            tracing::debug!(target: "npc_ai", "no_cover");
            tracing::warn!(target: "movement.npc", "stale_velocity");
            tracing::info!(target: "cover.coverage", "space_summary");
            tracing::debug!(target: "cover.selection", "picked");
            tracing::warn!(target: "spawner.npc_behaviour", "spawn_off_mesh");
            tracing::warn!(target: "threat", "cleared_without_exit");
            tracing::debug!(target: "wire.out.forced_position", "forced");
        });
        let seen = seen.lock().unwrap().clone();
        assert_eq!(
            seen.len(),
            17,
            "every NA02 detector row must pass OTEL_FILTER; got {seen:?}"
        );
    }
}
