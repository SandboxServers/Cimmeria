//! The launcher's tracing subscriber (C9). A daily rolling log file keeps every
//! line at INFO and above; a forward layer sends `adoption` records to the
//! forwarder thread. Nothing here blocks: the file writer is non-blocking and the
//! forward layer only `try_send`s.
use crate::host::NativeHost;
use cimmeria_launcher_engine::game_telemetry::health::{Recovery, SharedHealth, TelemetryHealth};
use cimmeria_launcher_engine::game_telemetry::launcher_event::wire_target;
use cimmeria_launcher_engine::game_telemetry::launcher_session::{LauncherSender, SendOutcome};
use std::fmt;
use std::io;
use std::path::Path;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{Builder, Rotation};
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{Layer, Registry};

pub(crate) use cimmeria_launcher_engine::game_telemetry::launcher_event::{
    LauncherRecord, Redaction,
};

pub(crate) const LOG_FILE_PREFIX: &str = "launcher"; // launcher.<yyyy-mm-dd>.log
pub(crate) const LOG_FILES_KEPT: usize = 7;
pub(crate) const FORWARD_CAPACITY: usize = 512; // sync_channel bound
pub(crate) const FORWARD_BATCH: usize = 64;
/// This crate's name as a tracing target (`-` written as `_`).
const SHELL_TARGET: &str = "cimmeria_launcher_desktop";

/// What `init` hands back: the file writer's guard (kept alive for the whole run)
/// and the records the forward layer sends.
pub(crate) struct Telemetry {
    pub guard: Option<tracing_appender::non_blocking::WorkerGuard>,
    pub records: Receiver<LauncherRecord>,
}

/// Install the global subscriber: the file layer when the log folder can be made,
/// and the forward layer always. Never panics; a second install is ignored.
pub(crate) fn init(log_dir: &Path, redaction: Redaction) -> Telemetry {
    let (tx, records) = std::sync::mpsc::sync_channel(FORWARD_CAPACITY);
    let (layer, guard) = match file_layer::<Registry>(log_dir, redaction) {
        Some((layer, guard)) => (Some(layer), Some(guard)),
        None => (None, None),
    };
    let _ = tracing::subscriber::set_global_default(
        Registry::default().with(layer).with(ForwardLayer::new(tx)),
    );
    Telemetry { guard, records }
}

/// The daily log file layer, or None when the folder or the appender cannot be
/// made (the user is told nothing). Every line passes through `Redacting`.
pub(crate) fn file_layer<S>(
    log_dir: &Path,
    redaction: Redaction,
) -> Option<(impl Layer<S>, WorkerGuard)>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    std::fs::create_dir_all(log_dir).ok()?;
    let appender = Builder::new()
        .rotation(Rotation::DAILY)
        .filename_prefix(LOG_FILE_PREFIX)
        .filename_suffix("log")
        .max_log_files(LOG_FILES_KEPT)
        .build(log_dir)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(move || Redacting {
            inner: writer.clone(),
            redaction: redaction.clone(),
        })
        .with_filter(file_filter());
    Some((layer, guard))
}

/// INFO for the launcher crates, `adoption` and the `launcher` prefix (the file-only
/// `launcher.pipeline` target); WARN for everything else.
fn file_filter() -> Targets {
    Targets::new()
        .with_default(LevelFilter::WARN)
        .with_target("cimmeria_launcher_engine", LevelFilter::INFO)
        .with_target("cimmeria_launcher_core", LevelFilter::INFO)
        .with_target(SHELL_TARGET, LevelFilter::INFO)
        .with_target("adoption", LevelFilter::INFO)
        .with_target("launcher", LevelFilter::INFO)
}

/// A writer that takes the home folder out of every line before it goes on.
pub(crate) struct Redacting<W> {
    pub inner: W,
    pub redaction: Redaction,
}

impl<W: io::Write> io::Write for Redacting<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        let scrubbed = self.redaction.replace_home(&text);
        self.inner.write_all(scrubbed.as_bytes())?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Queues records on a launcher wire target (C1) at INFO and above for the forwarder.
/// A full queue drops the record: this layer never blocks the UI.
pub(crate) struct ForwardLayer {
    tx: SyncSender<LauncherRecord>,
}

impl ForwardLayer {
    pub(crate) fn new(tx: SyncSender<LauncherRecord>) -> Self {
        Self { tx }
    }
}

impl<S: Subscriber> Layer<S> for ForwardLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let Some(wire) = wire_target(metadata.target()) else {
            return;
        };
        if *metadata.level() > Level::INFO {
            return;
        }
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);
        let record = LauncherRecord {
            ts_ms: now_ms(),
            level: metadata.level().to_string().to_ascii_lowercase(),
            target: wire,
            fields: visitor.fields,
        };
        let _ = self.tx.try_send(record);
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}

/// Collects an event's fields into a JSON map.
#[derive(Default)]
struct FieldVisitor {
    fields: serde_json::Map<String, serde_json::Value>,
}

impl Visit for FieldVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields.insert(field.name().to_owned(), value.into());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields.insert(field.name().to_owned(), value.into());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields.insert(field.name().to_owned(), value.into());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields.insert(field.name().to_owned(), value.into());
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        let number = serde_json::Number::from_f64(value)
            .map_or(serde_json::Value::Null, serde_json::Value::Number);
        self.fields.insert(field.name().to_owned(), number);
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.fields
            .insert(field.name().to_owned(), format!("{value:?}").into());
    }
}

static HEALTH: OnceLock<SharedHealth> = OnceLock::new();

/// The process-wide pipeline health, recorded by the forwarder (launcher-telemetry C7).
pub(crate) fn health() -> SharedHealth {
    HEALTH
        .get_or_init(|| Arc::new(Mutex::new(TelemetryHealth::default())))
        .clone()
}

/// Records one send's outcome and logs the recovery when it ends a run of failures.
/// Returns the recovery, if any.
pub(crate) fn record_outcome(
    health: &SharedHealth,
    outcome: &SendOutcome,
    now_ms: u64,
) -> Option<Recovery> {
    let mut state = health.lock().unwrap_or_else(PoisonError::into_inner);
    let recovery = state.record(outcome, now_ms)?;
    let last_reason = state.last_reason;
    drop(state);
    pipeline_recovered(&recovery, last_reason);
    Some(recovery)
}

/// The sends are back after a run of failures (launcher-telemetry C7).
fn pipeline_recovered(recovery: &Recovery, last_reason: Option<&'static str>) {
    tracing::info!(
        target: "launcher.telemetry",
        event = "pipeline_recovered",
        mint_failed = recovery.failures.mint_failed,
        held = recovery.failures.held,
        dropped = recovery.failures.dropped,
        outage_ms = recovery.outage_ms,
        last_reason = last_reason.unwrap_or("none"),
        "launcher telemetry: sends resumed; expected every batch delivered, some were lost while the server was unreachable"
    );
}

/// Start the forwarder thread. It batches records, builds the events on the
/// host, and sends them with `LauncherSender`. A spawn error is ignored.
pub(crate) fn spawn_forwarder(
    host: Arc<NativeHost>,
    records: Receiver<LauncherRecord>,
    redaction: Redaction,
) {
    let _ = std::thread::Builder::new()
        .name("launcher-telemetry".into())
        .spawn(move || {
            let sender = LauncherSender::new();
            while let Ok(first) = records.recv() {
                let mut batch = vec![first];
                while batch.len() < FORWARD_BATCH {
                    match records.try_recv() {
                        Ok(r) => batch.push(r),
                        Err(_) => break,
                    }
                }
                if let Some((target, events)) = host.launcher_batch(&batch, &redaction) {
                    let outcome = tauri::async_runtime::block_on(sender.send(&target, &events));
                    record_outcome(&health(), &outcome, u64::try_from(now_ms()).unwrap_or(0));
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::sync_channel;
    use tracing::subscriber::with_default;
    use tracing::{debug, info};

    #[test]
    fn a_launcher_install_info_event_is_forwarded_on_its_wire_target() {
        let (tx, rx) = sync_channel(16);
        with_default(Registry::default().with(ForwardLayer::new(tx)), || {
            info!(target: "launcher.install", operation = 7u64, "install finished");
        });
        let records: Vec<LauncherRecord> = rx.try_iter().collect();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].target, "client.launcher.install");
        assert_eq!(records[0].level, "info");
        assert_eq!(records[0].fields["operation"], 7);
    }

    #[test]
    fn a_file_only_target_and_a_debug_event_are_not_forwarded() {
        let (tx, rx) = sync_channel(16);
        with_default(Registry::default().with(ForwardLayer::new(tx)), || {
            info!(target: "launcher.pipeline", "file only");
            debug!(target: "launcher.install", "debug only");
        });
        assert_eq!(rx.try_iter().count(), 0);
    }

    #[test]
    fn the_file_filter_keeps_launcher_info_in_the_local_log() {
        let filter = file_filter();
        assert!(filter.would_enable("launcher.pipeline", &Level::INFO));
        assert!(!filter.would_enable("launcher.pipeline", &Level::DEBUG));
    }

    #[test]
    fn two_failures_then_a_success_emit_exactly_one_pipeline_recovered() {
        let health: SharedHealth = Arc::new(Mutex::new(TelemetryHealth::default()));
        let outcomes = [
            SendOutcome::MintFailed {
                reason: "transport",
            },
            SendOutcome::Dropped,
            SendOutcome::Sent,
            SendOutcome::Sent,
        ];
        let ((), captured) = crate::test_capture::capture(|| {
            for (at, outcome) in outcomes.iter().enumerate() {
                record_outcome(&health, outcome, 100 * at as u64);
            }
        });
        let recovered: Vec<_> = captured
            .iter()
            .filter(|c| c.field("event") == Some("pipeline_recovered"))
            .collect();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].target, "launcher.telemetry");
        assert_eq!(recovered[0].field("mint_failed"), Some("1"));
        assert_eq!(recovered[0].field("dropped"), Some("1"));
        assert_eq!(recovered[0].field("last_reason"), Some("transport"));
    }
}
