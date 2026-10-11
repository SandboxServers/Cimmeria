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
//! - **Nothing from `OTEL_*`.** The resource is built empty, not by the
//!   SDK's detectors (which would ship `OTEL_RESOURCE_ATTRIBUTES` as it is),
//!   and the exporter sends no header the environment named
//!   (`OTEL_EXPORTER_OTLP_HEADERS`, `OTEL_EXPORTER_OTLP_LOGS_HEADERS`): the
//!   SDK merges those in unconditionally, so the gRPC path clears the request
//!   metadata in an interceptor that runs after the merge, and the HTTP path
//!   wraps its client to drop every header but the body's type and encoding.
//!   Batch sizes and timeouts may still come from `OTEL_*`; they carry no
//!   data.
//! - **Rows are redacted.** Every string a row carries goes through
//!   [`redact::Redactor`]: a local path becomes the instance it belongs to,
//!   or `<path>`; the Windows user name becomes `<user>`, the machine name
//!   `<host>`, a lab token `<secret>`. The lease's free text (`owner`,
//!   `purpose`, a `lab.lease` `reason`) and credential-named fields are left
//!   off ([`logger`]). `labd.log` keeps the full text.
//! - **Off by default.** No endpoint, no exporter, no layer.
//!
//! The batch processor runs on its own thread with a bounded queue that
//! drops when full, so an unreachable collector never blocks labd; shutdown
//! waits at most [`SHUTDOWN_TIMEOUT`].

use std::sync::Arc;
use std::time::Duration;

use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_http::HttpClient;
use opentelemetry_otlp::{Protocol, WithExportConfig, WithHttpConfig, WithTonicConfig};
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::Resource;
use tracing_subscriber::EnvFilter;

mod headers;
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
///
/// `EnvFilter` matches a target by string prefix: `tower=off` (tonic's
/// transport) also covers `tower_http`, which is not in the export path, so
/// `tower_http=warn` (the longer, more specific directive wins) keeps it at
/// the dependency default. `hyper=off` covering `hyper_util` and
/// `opentelemetry=off` covering `opentelemetry_*` are intended.
pub(crate) const OTLP_FILTER: &str = "warn,cimmeria_lab=info,lab=info,opentelemetry=off,\
opentelemetry_sdk=off,opentelemetry_otlp=off,tonic=off,h2=off,hyper=off,hyper_util=off,\
tower=off,tower_http=warn,reqwest=off";

/// Request timeout of the HTTP exporter's client: the SDK's own default
/// (`OTEL_EXPORTER_OTLP_TIMEOUT`'s), which a client labd builds itself has
/// to restate.
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

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
    init_with(&|k| std::env::var(k).ok(), None)
}

/// [`init`] over an explicit variable lookup, and optionally the HTTP client
/// the exporter sends through (tests capture the request there). Only the
/// lab's own variables come from `var`; the SDK still reads the process
/// environment, which is what the no-`OTEL_*` guarantees are about.
pub(crate) fn init_with(
    var: &dyn Fn(&str) -> Option<String>,
    http_client: Option<Arc<dyn HttpClient>>,
) -> Option<(OtlpLayer, OtlpGuard)> {
    let endpoint = var(ENDPOINT_ENV).filter(|v| !v.trim().is_empty())?;
    let protocol = var(PROTOCOL_ENV).unwrap_or_else(|| "grpc".to_string());
    let deploy_env = var(DEPLOY_ENV_ENV).unwrap_or_else(|| "dev".to_string());

    let exporter = match build_exporter(&protocol, &endpoint, http_client) {
        Ok(e) => e,
        Err(err) => {
            eprintln!("cimmeria-lab: OTLP log exporter init failed ({err}); labd.log only");
            return None;
        }
    };
    let provider = SdkLoggerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource(&deploy_env, &instance_name(var)))
        .build();
    let layer = OpenTelemetryTracingBridge::new(&RedactingProvider::new(
        provider.clone(),
        Redactor::from_vars(var),
    ));
    eprintln!("cimmeria-lab: exporting logs to {endpoint} (protocol={protocol}, service.name={SERVICE_NAME})");
    Some((layer, OtlpGuard { provider }))
}

fn is_http(protocol: &str) -> bool {
    matches!(protocol, "http/protobuf" | "http")
}

/// The OTLP log exporter, with no header from the environment on the wire
/// (module docs). The protocol is set explicitly too, so an
/// `OTEL_EXPORTER_OTLP_PROTOCOL` set for another tool cannot make the HTTP
/// builder refuse.
fn build_exporter(
    protocol: &str,
    endpoint: &str,
    http_client: Option<Arc<dyn HttpClient>>,
) -> Result<opentelemetry_otlp::LogExporter, String> {
    if is_http(protocol) {
        let inner = match http_client {
            Some(c) => c,
            None => headers::blocking_client(HTTP_TIMEOUT)?,
        };
        opentelemetry_otlp::LogExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .with_endpoint(endpoint)
            .with_headers(Default::default())
            .with_http_client(headers::NoEnvHeaders::new(inner))
            .build()
            .map_err(|e| e.to_string())
    } else {
        opentelemetry_otlp::LogExporter::builder()
            .with_tonic()
            .with_protocol(Protocol::Grpc)
            .with_endpoint(endpoint)
            .with_metadata(Default::default())
            .with_interceptor(headers::clear_metadata)
            .build()
            .map_err(|e| e.to_string())
    }
}

/// The instance label(s) this daemon hosts, comma-joined, or `default`.
/// Validated names only, so nothing local leaks through them.
fn instance_name(var: &dyn Fn(&str) -> Option<String>) -> String {
    let labels = redact::instance_labels(var);
    if labels.is_empty() {
        "default".to_string()
    } else {
        labels.join(",")
    }
}

/// The resource, built from nothing: `service.name = cimmeria-lab`,
/// `service.version`, the instance name, and the server's deploy identity
/// keys (from `CIMMERIA_DEPLOY_ENV`, ours). Not `Resource::builder()`: its
/// detectors would ship `OTEL_RESOURCE_ATTRIBUTES` and `OTEL_SERVICE_NAME`
/// as they are. No `host.name`: on a dev box it is often the owner's own
/// name, and the deployment environment already separates the colo lab
/// from a laptop.
pub(crate) fn resource(deploy_env: &str, instance: &str) -> Resource {
    use opentelemetry::KeyValue;
    Resource::builder_empty()
        .with_service_name(SERVICE_NAME)
        .with_attributes([
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("cimmeria.lab.instance", instance.to_string()),
            KeyValue::new("deployment.environment", deploy_env.to_string()),
            KeyValue::new("cimmeria.deploy_env", deploy_env.to_string()),
            KeyValue::new("cimmeria.source", "lab"),
        ])
        .build()
}
