//! The `cimmeria-client` OTLP index: client-originated telemetry (the
//! in-game DLL's events and the client logs the launcher uploads) under
//! its own `service.name`, with typed attributes.
//!
//! [`otel::init`](crate::otel::init) builds a fourth logger provider for
//! it and installs [`ClientSink`] into
//! [`cimmeria_admin_api::routes::telemetry::client_sink`], which hands it
//! one [`ClientRecord`] per uploaded event. The tracing replay of those
//! events keeps feeding the file logs and the admin WebSocket, but
//! [`is_client_target`] keeps it out of the other three OTLP indexes, so
//! nothing is indexed twice.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cimmeria_admin_api::routes::telemetry::client_sink::{
    AttrValue, ClientLogSink, ClientRecord, ClientSeverity,
};
use opentelemetry::logs::{AnyValue, LogRecord, Logger, LoggerProvider, Severity};
use opentelemetry::InstrumentationScope;
use opentelemetry_sdk::logs::{SdkLogger, SdkLoggerProvider};

/// `service.name` of the client index.
pub const CLIENT_SERVICE_NAME: &str = "cimmeria-client";

/// Tracing targets whose events reach SigNoz through [`ClientSink`]
/// instead. Exactly the targets the telemetry ingest replays from an
/// upload; `launcher.ingest` and the bundle summary are the server's own
/// events and stay in `cimmeria-server`.
pub fn is_client_target(target: &str) -> bool {
    matches!(
        target,
        "client.native" | "launcher.client_log" | "launcher.debug_log" | "launcher.session_meta"
    )
}

/// Emits [`ClientRecord`]s through the `cimmeria-client` provider, one
/// logger (instrumentation scope) per record scope.
pub struct ClientSink {
    provider: SdkLoggerProvider,
    loggers: Mutex<HashMap<&'static str, SdkLogger>>,
}

impl ClientSink {
    pub fn new(provider: SdkLoggerProvider) -> Self {
        Self {
            provider,
            loggers: Mutex::new(HashMap::new()),
        }
    }
}

impl ClientLogSink for ClientSink {
    fn emit(&self, record: ClientRecord) {
        let logger = {
            let mut loggers = self.loggers.lock().unwrap_or_else(|p| p.into_inner());
            loggers
                .entry(record.scope)
                .or_insert_with(|| {
                    self.provider
                        .logger_with_scope(InstrumentationScope::builder(record.scope).build())
                })
                .clone()
        };
        let now = SystemTime::now();
        let mut out = logger.create_log_record();
        out.set_timestamp(timestamp(record.ts_ms, now));
        out.set_observed_timestamp(now);
        let (number, text) = severity(record.severity);
        out.set_severity_number(number);
        out.set_severity_text(text);
        out.set_body(AnyValue::from(record.body));
        for (key, value) in record.attributes {
            out.add_attribute(key, any_value(value));
        }
        logger.emit(out);
    }
}

/// The client's own timestamp when it sent one, else the time it arrived.
fn timestamp(ts_ms: i64, now: SystemTime) -> SystemTime {
    if ts_ms > 0 {
        UNIX_EPOCH + Duration::from_millis(ts_ms as u64)
    } else {
        now
    }
}

fn severity(s: ClientSeverity) -> (Severity, &'static str) {
    match s {
        ClientSeverity::Trace => (Severity::Trace, "TRACE"),
        ClientSeverity::Debug => (Severity::Debug, "DEBUG"),
        ClientSeverity::Info => (Severity::Info, "INFO"),
        ClientSeverity::Warn => (Severity::Warn, "WARN"),
        ClientSeverity::Error => (Severity::Error, "ERROR"),
    }
}

fn any_value(v: AttrValue) -> AnyValue {
    match v {
        AttrValue::Str(s) => AnyValue::from(s),
        AttrValue::I64(i) => AnyValue::Int(i),
        AttrValue::F64(f) => AnyValue::Double(f),
        AttrValue::Bool(b) => AnyValue::Boolean(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exactly the replayed upload targets move to the client index; the
    /// server's own ingest bookkeeping stays where it was.
    #[test]
    fn only_replayed_upload_targets_are_client_targets() {
        for t in [
            "client.native",
            "launcher.client_log",
            "launcher.debug_log",
            "launcher.session_meta",
        ] {
            assert!(is_client_target(t), "{t}");
        }
        for t in [
            "launcher.ingest",
            "launcher.key_dump",
            "cimmeria_admin_api::routes::dev_session::handlers",
            "mercury.packet",
        ] {
            assert!(!is_client_target(t), "{t}");
        }
    }

    #[test]
    fn client_time_is_used_when_sent() {
        let now = UNIX_EPOCH + Duration::from_secs(10);
        assert_eq!(timestamp(0, now), now);
        assert_eq!(
            timestamp(1_500, now),
            UNIX_EPOCH + Duration::from_millis(1_500)
        );
    }

    #[test]
    fn values_and_severities_map_one_to_one() {
        assert_eq!(any_value(AttrValue::I64(3)), AnyValue::Int(3));
        assert_eq!(any_value(AttrValue::Bool(true)), AnyValue::Boolean(true));
        assert_eq!(any_value(AttrValue::F64(0.5)), AnyValue::Double(0.5));
        assert_eq!(
            any_value(AttrValue::Str("x".into())),
            AnyValue::from("x".to_string())
        );
        assert_eq!(severity(ClientSeverity::Warn), (Severity::Warn, "WARN"));
        assert_eq!(severity(ClientSeverity::Error).1, "ERROR");
    }

    /// The sink emits without panicking through a real provider (no
    /// exporter: records are dropped, but the path runs end to end).
    #[test]
    fn sink_emits_through_a_provider() {
        let sink = ClientSink::new(SdkLoggerProvider::builder().build());
        sink.emit(ClientRecord {
            scope: "client.native",
            severity: ClientSeverity::Info,
            ts_ms: 1,
            body: "client.dll.attached".into(),
            attributes: vec![("seq".into(), AttrValue::I64(1))],
        });
        assert_eq!(sink.loggers.lock().unwrap().len(), 1);
    }
}
