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
    otel_trace_layer: Option<otel::OtelTraceLayer>,
    otel_log_layer: Option<otel::OtelLogLayer>,
    otel_network_log_layer: Option<otel::OtelLogLayer>,
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
    // The per-packet wire stream belongs in protocol.log + OTLP at full fidelity,
    // not in human-facing sinks (console, server.log, admin WS). Mute it in each.
    const WIRE_FIREHOSE_MUTED: &str =
        "mercury.packet=warn,wire.in=warn,wire.out=warn,mercury.retransmit=warn";
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
            .with_filter(EnvFilter::new(format!("info,{WIRE_FIREHOSE_MUTED}"))),
    ));

    // ── Per-system log files ─────────────────────────────────────────────
    layers.push(log_layer!("auth.log", "off,cimmeria_services::auth=trace"));

    layers.push(log_layer!(
        "base.log",
        "off,\
         cimmeria_services::base::service=trace,\
         cimmeria_services::base::connect_loop=trace,\
         cimmeria_services::base::login=trace,\
         cimmeria_services::base::tick_sync=trace,\
         cimmeria_services::base::helpers=trace"
    ));

    layers.push(log_layer!(
        "world_entry.log",
        "off,\
         cimmeria_services::base::world_entry=trace,\
         cimmeria_services::base::world_entry_player=trace,\
         cimmeria_services::base::world_entry_appearance=trace"
    ));

    layers.push(log_layer!(
        "character.log",
        "off,\
         cimmeria_services::base::character=trace,\
         cimmeria_services::base::character_create=trace,\
         cimmeria_services::base::chardef=trace,\
         cimmeria_services::base::cooked_data=trace,\
         cimmeria_services::base::resources=trace"
    ));

    layers.push(log_layer!(
        "protocol.log",
        "off,\
         cimmeria_services::mercury=trace,\
         cimmeria_mercury=trace,\
         mercury.packet=info,\
         wire.in=info,wire.out=info,\
         mercury.retransmit=info"
    ));

    layers.push(log_layer!(
        "aoi.log",
        "off,\
         cimmeria_services::cell::service=trace,\
         cimmeria_services::cell::space_manager=trace"
    ));

    layers.push(log_layer!(
        "combat.log",
        "off,\
         cimmeria_services::cell::combat=trace,\
         cimmeria_services::cell::abilities=trace"
    ));

    layers.push(log_layer!(
        "content.log",
        "off,cimmeria_services::cell::content=trace"
    ));

    layers.push(log_layer!(
        "missions.log",
        "off,cimmeria_services::cell::missions=trace"
    ));

    layers.push(log_layer!(
        "interactions.log",
        "off,\
         cimmeria_services::cell::interactions=trace,\
         cimmeria_services::cell::chat=trace,\
         cimmeria_services::cell::mail=trace"
    ));

    layers.push(log_layer!(
        "spawner.log",
        "off,\
         cimmeria_services::cell::spawner=trace,\
         cimmeria_services::cell::gate_travel=trace,\
         cimmeria_services::cell::ring_transport=trace"
    ));

    layers.push(log_layer!(
        "dispatch.log",
        "off,\
         cimmeria_services::cell::dispatch=trace,\
         cimmeria_services::base::dispatch=trace"
    ));

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
    // it's the load-bearing analytical surface here, so muting/sampling it would
    // defeat the purpose. The trace and log layers share one filter.
    let otel_filter = "info,\
                cimmeria_services=debug,\
                cimmeria_mercury=debug,\
                mercury.packet=info,\
                mercury.retransmit=info,\
                mercury.backpressure=warn,\
                wire.in=info,wire.out=info,\
                aoi.entity_enter=debug,aoi.entity_leave=debug,\
                movement.npc=debug,movement.player=debug,\
                npc_ai=debug,\
                threat=info,\
                auth=info,\
                world_entry=info,\
                vendor=info,mail=info,progression=info,inventory=info,mission=info,\
                sqlx::query=debug,\
                tungstenite=off,tokio_tungstenite=off,hyper=off,\
                h2=off,tower=off,tonic=off,reqwest=off,opentelemetry=off";

    if let Some(layer) = otel_trace_layer {
        layers.push(Box::new(layer.with_filter(EnvFilter::new(otel_filter))));
    }
    // The log signal splits across TWO OTLP providers (cimmeria-server +
    // cimmeria-network — see `otel::init`). Per-layer FilterFn routes:
    //
    // - `cimmeria-server` layer: receive everything EXCEPT TRACE/DEBUG/
    //   INFO from network-noise scopes. WARN+ from network-noise scopes
    //   still goes here so elevated severity surfaces in the operator's
    //   primary view without dual-querying.
    // - `cimmeria-network` layer: receive ONLY TRACE/DEBUG/INFO from
    //   network-noise scopes. WARN+ is suppressed (it's already in the
    //   server index).
    //
    // Composition: each branch is `EnvFilter::new(otel_filter)` AND a
    // `FilterFn` doing the noise routing. Together they cover all events
    // with no overlap (a single event lands in exactly one log index).
    //
    // We use `tracing_subscriber::filter::FilterExt::and` to combine the
    // two filters, and `Box::new` at the end because the layer push
    // signature wants a homogeneous trait object.
    if let Some(layer) = otel_log_layer {
        use tracing_subscriber::filter::{filter_fn, FilterExt};
        let routing = filter_fn(|meta| {
            // Server index: non-noise OR severity is WARN/ERROR.
            !otel::is_network_noise_target(meta.target()) || *meta.level() <= tracing::Level::WARN
        });
        layers.push(Box::new(
            layer.with_filter(EnvFilter::new(otel_filter).and(routing)),
        ));
    }
    if let Some(layer) = otel_network_log_layer {
        use tracing_subscriber::filter::{filter_fn, FilterExt};
        let routing = filter_fn(|meta| {
            // Network index: noise scopes only, at non-elevated severity.
            // `level <= Level::WARN` is "WARN or more severe" (numerically
            // smaller); the negation here means "less severe than WARN"
            // i.e. INFO/DEBUG/TRACE.
            otel::is_network_noise_target(meta.target()) && *meta.level() > tracing::Level::WARN
        });
        layers.push(Box::new(
            layer.with_filter(EnvFilter::new(otel_filter).and(routing)),
        ));
    }

    // Assemble the subscriber — one `.with()` call on the whole Vec.
    tracing_subscriber::registry().with(layers).init();

    guards
}

#[cfg(test)]
mod tests {
    use super::{archive_previous_logs_in, chrono_timestamp, days_to_ymd};
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
}
