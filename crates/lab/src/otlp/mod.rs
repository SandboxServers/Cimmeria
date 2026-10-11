//! labd's log export to SigNoz (TG-PIPE-17, owner decision D-TG2).
//!
//! The shared daemon (`--http`) keeps writing `labd.log` and, when an
//! endpoint is configured, also ships its rows over OTLP to the colo
//! collector as `service.name = cimmeria-lab`. That makes the lab's side of
//! a session visible next to the server's: a watchdog relaunch, an orphaned
//! `SGW.exe` (#1342), a lease that ran out.
//!
//! Built like the server's exporter (`crates/server/src/otel.rs`): the same
//! OpenTelemetry crates and versions, a batch logger provider per service,
//! the `opentelemetry-appender-tracing` bridge as a filtered layer. Logs only.
//!
//! What differs, and why:
//!
//! - **Its own variables.** [`ENDPOINT_ENV`] and [`PROTOCOL_ENV`], not the
//!   standard `OTEL_EXPORTER_OTLP_*`: those are often set user-wide for other
//!   tools (Claude Code's own telemetry), and labd must not start exporting
//!   because something else configured a collector. Set them in `labd.env`.
//! - **Paths are redacted.** Every string a row carries goes through
//!   [`redact::Redactor`]: a local path becomes the instance it belongs to,
//!   or `<path>`, and the Windows user name becomes `<user>`. `labd.log`
//!   keeps the full text.
//! - **Off by default.** No endpoint, no exporter, no layer.
//!
//! The batch processor runs on its own thread with a bounded queue that
//! drops when full, so an unreachable collector never blocks labd; shutdown
//! waits at most [`SHUTDOWN_TIMEOUT`].

use std::time::Duration;

use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::Resource;
use tracing_subscriber::EnvFilter;

mod logger;
mod redact;

#[cfg(test)]
mod tests;

pub(crate) use logger::{RedactingLogger, RedactingProvider};
pub(crate) use redact::Redactor;

/// The collector endpoint, e.g. `http://<collector>:4317`. Unset or empty:
/// no export.
pub(crate) const ENDPOINT_ENV: &str = "CIMMERIA_LAB_OTLP_ENDPOINT";
/// `grpc` (default) or `http/protobuf`, as `OTEL_EXPORTER_OTLP_PROTOCOL`.
pub(crate) const PROTOCOL_ENV: &str = "CIMMERIA_LAB_OTLP_PROTOCOL";
/// `deployment.environment` on the resource, as the server's exporter.
const DEPLOY_ENV_ENV: &str = "CIMMERIA_DEPLOY_ENV";

/// The SigNoz service labd's rows land in.
pub(crate) const SERVICE_NAME: &str = "cimmeria-lab";

/// What the exporter ships: labd's own rows at INFO, hand-named `lab.*`
/// targets (`lab.lease`, `lab.instance`, `lab.uat`, `lab.client` from
/// TG-PIPE-05) and module-path rows (`cimmeria_lab::supervisor::watchdog`),
/// and only WARN+ from dependencies. The exporter's own stack is off so a
/// failing export cannot log about itself into the export.
pub(crate) const OTLP_FILTER: &str = "warn,cimmeria_lab=info,lab=info,opentelemetry=off,\
opentelemetry_sdk=off,opentelemetry_otlp=off,tonic=off,h2=off,hyper=off,hyper_util=off,\
tower=off,reqwest=off";

/// How long shutdown waits for the last batch. Short: a daemon restart must
/// not hang on an unreachable collector.
pub(crate) const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

/// The tracing layer that exports through a [`RedactingProvider`].
pub(crate) type OtlpLayer = OpenTelemetryTracingBridge<RedactingProvider, RedactingLogger>;

/// Flushes the last batch when dropped.
pub(crate) struct OtlpGuard {
    provider: SdkLoggerProvider,
}

impl Drop for OtlpGuard {
    fn drop(&mut self) {
        if let Err(e) = self.provider.shutdown_with_timeout(SHUTDOWN_TIMEOUT) {
            eprintln!("cimmeria-lab: OTLP log shutdown flush failed: {e}");
        }
    }
}

/// The filter the export layer runs under ([`OTLP_FILTER`]).
pub(crate) fn filter() -> EnvFilter {
    EnvFilter::new(OTLP_FILTER)
}

/// The export layer and its guard, or `None` when [`ENDPOINT_ENV`] is unset
/// or the exporter cannot be built (said on stderr, which the scheduled task
/// keeps; the subscriber is not installed yet). Call inside the tokio
/// runtime: the gRPC exporter's channel needs one.
pub(crate) fn init() -> Option<(OtlpLayer, OtlpGuard)> {
    let endpoint = std::env::var(ENDPOINT_ENV)
        .ok()
        .filter(|v| !v.trim().is_empty())?;
    let protocol = std::env::var(PROTOCOL_ENV).unwrap_or_else(|_| "grpc".to_string());
    let deploy_env = std::env::var(DEPLOY_ENV_ENV).unwrap_or_else(|_| "dev".to_string());

    let exporter = match protocol.as_str() {
        "http/protobuf" | "http" => opentelemetry_otlp::LogExporter::builder()
            .with_http()
            .with_endpoint(&endpoint)
            .build(),
        _ => opentelemetry_otlp::LogExporter::builder()
            .with_tonic()
            .with_endpoint(&endpoint)
            .build(),
    };
    let exporter = match exporter {
        Ok(e) => e,
        Err(err) => {
            eprintln!("cimmeria-lab: OTLP log exporter init failed ({err}); labd.log only");
            return None;
        }
    };
    let provider = SdkLoggerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource(&deploy_env))
        .build();
    let layer = OpenTelemetryTracingBridge::new(&RedactingProvider::new(
        provider.clone(),
        Redactor::from_env(),
    ));
    eprintln!("cimmeria-lab: exporting logs to {endpoint} (protocol={protocol}, service.name={SERVICE_NAME})");
    Some((layer, OtlpGuard { provider }))
}

/// `service.name = cimmeria-lab` plus the server's deploy identity keys.
/// No `host.name`: on a dev box it is often the owner's own name, and the
/// deployment environment already separates the colo lab from a laptop.
fn resource(deploy_env: &str) -> Resource {
    use opentelemetry::KeyValue;
    Resource::builder()
        .with_service_name(SERVICE_NAME)
        .with_attributes([
            KeyValue::new("deployment.environment", deploy_env.to_string()),
            KeyValue::new("cimmeria.deploy_env", deploy_env.to_string()),
            KeyValue::new("cimmeria.source", "lab"),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        ])
        .build()
}
