//! OpenTelemetry exporter — ships tracing logs + spans to a SigNoz
//! deployment (or any other OTLP-compatible backend) via the OTEL
//! Collector.
//!
//! # Architecture
//!
//! A [`tracing_subscriber::Layer`] that the main subscriber composes
//! alongside the file/broadcast layers. The OTLP exporter itself runs
//! in a background task spawned by [`init`]; the layer is a thin
//! "is this enabled" wrapper.
//!
//! Launcher logs land here too — the
//! `/api/telemetry/upload-{chunk,bundle}` endpoints (see
//! [`cimmeria_admin_api::routes::telemetry`]) replay each launcher
//! event through `tracing::*` so the OTLP layer ships it to the same
//! SigNoz store as the server's own logs and Mercury packet events.
//!
//! # Environment variables
//!
//! | Variable | Description |
//! |---|---|
//! | `OTEL_EXPORTER_OTLP_ENDPOINT` | e.g. `http://otel-collector:4317`. If unset, OTLP is disabled. |
//! | `OTEL_EXPORTER_OTLP_PROTOCOL` | `grpc` (default) or `http/protobuf`. |
//! | `OTEL_SERVICE_NAME` | Defaults to `cimmeria-server`. Shows up in SigNoz's service map. |
//! | `OTEL_RESOURCE_ATTRIBUTES` | Comma-separated `k=v` pairs piped through to every event. Common keys: `deployment.environment`, `service.namespace`. |
//! | `OTEL_TRACES_SAMPLER` | `always_on` (default), `always_off`, or `traceidratio` with `OTEL_TRACES_SAMPLER_ARG`. |
//!
//! All env vars match the OpenTelemetry SDK spec — pinned so the
//! standard `opentelemetry-otlp` crate reads them directly without us
//! re-implementing the conventions.
//!
//! # Why the existing tracing layer is the right seam
//!
//! Every server log already flows through `tracing::*` macros. The
//! Mercury packet capture in `cimmeria_mercury::instrumentation` is a
//! plain `tracing::info!(target = "mercury.packet", ...)` call.
//! Hooking OTLP to the same subscriber means *nothing else has to
//! change* — every log, every packet event, every existing structured
//! field flows out automatically. No fan-out at the call sites, no
//! parallel infrastructure to maintain.

use std::env;

use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{Sampler, TracerProvider};
use opentelemetry_sdk::Resource;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::Registry;

/// Sentinel returned by [`init`] when the OTLP env vars aren't set.
/// The caller passes `None` to the subscriber so the layer is omitted
/// from the layered stack — zero cost, no spurious "failed to export"
/// retries when the integration just isn't enabled.
pub type OtelLayer = OpenTelemetryLayer<Registry, opentelemetry_sdk::trace::Tracer>;

/// Initialize the OTLP exporter and return a [`tracing_subscriber`]
/// layer that ships events through it. Returns `None` (and logs a
/// `debug!`) when `OTEL_EXPORTER_OTLP_ENDPOINT` is unset.
///
/// The returned guard ([`OtelGuard`]) must be held for the lifetime of
/// the process — dropping it triggers `TracerProvider::shutdown` which
/// flushes the in-flight batch to the collector. Without this flush,
/// the last few seconds of telemetry before a clean shutdown are lost.
pub fn init() -> Option<(OtelLayer, OtelGuard)> {
    let endpoint = match env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            // Not configured — silently no-op. We don't `eprintln!` here
            // because the absence of OTLP is the default state and
            // logging it on every cold start would be noise.
            return None;
        }
    };

    // Propagator: parses W3C tracecontext headers off any inbound HTTP
    // request so a trace started by the client (or the launcher) chains
    // through to our spans. Mandatory for distributed traces to look
    // contiguous in the SigNoz UI.
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    // Resource attributes — composed from the standard OTel env vars
    // plus a hardcoded `service.name` fallback. SigNoz's service map
    // groups by `service.name`, so leaving it unset would coalesce
    // every server's events into a single "unknown_service" bucket.
    let service_name =
        env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "cimmeria-server".to_string());
    let resource = Resource::new(vec![KeyValue::new("service.name", service_name.clone())]);
    // OTEL_RESOURCE_ATTRIBUTES is parsed by `opentelemetry_sdk` itself
    // when present, so we don't need to manually split-and-merge it
    // here — the SDK union-merges over our explicit Resource above.

    // Sampler defaults to `always_on` — Mercury packet logs are the
    // analytical surface we care about, sampling would defeat the
    // purpose. Tune with OTEL_TRACES_SAMPLER on a per-deployment basis.
    let sampler = match env::var("OTEL_TRACES_SAMPLER").as_deref() {
        Ok("always_off") => Sampler::AlwaysOff,
        Ok("traceidratio") => {
            let ratio = env::var("OTEL_TRACES_SAMPLER_ARG")
                .ok()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(1.0);
            Sampler::TraceIdRatioBased(ratio)
        }
        _ => Sampler::AlwaysOn,
    };

    let protocol = env::var("OTEL_EXPORTER_OTLP_PROTOCOL").unwrap_or_else(|_| "grpc".to_string());

    let exporter_result = match protocol.as_str() {
        "http/protobuf" | "http" => opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(&endpoint)
            .build(),
        _ => opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(&endpoint)
            .build(),
    };

    let exporter = match exporter_result {
        Ok(e) => e,
        Err(err) => {
            // Loud: if OTEL_* env vars are set we expect connectivity.
            // Falling back to "no OTLP" silently would mask deployment
            // misconfiguration. eprintln! (not tracing::error!) because
            // the tracing subscriber isn't installed yet when we run.
            eprintln!("[otel] Exporter init failed ({err}); telemetry will not ship");
            return None;
        }
    };

    let provider = TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_sampler(sampler)
        .with_resource(resource)
        .build();

    let tracer = provider.tracer(service_name);
    // Set as the global provider so any code path that grabs
    // `opentelemetry::global::tracer(...)` picks up our exporter.
    opentelemetry::global::set_tracer_provider(provider.clone());

    let layer = tracing_opentelemetry::layer().with_tracer(tracer);
    eprintln!("[otel] Streaming to {endpoint} (protocol={protocol})");

    Some((layer, OtelGuard { provider }))
}

/// RAII guard — when dropped, flushes the in-flight batch to the
/// collector via `TracerProvider::shutdown`. Holding this in `main()`
/// until after the orchestrator's `stop_all` returns keeps the last
/// few hundred ms of shutdown telemetry from being dropped.
pub struct OtelGuard {
    provider: TracerProvider,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        // shutdown() blocks until the in-flight batch flushes. ~250ms
        // worst-case; usually much faster. We do this on the main
        // thread, after `stop_all` returns, so the cost is on a path
        // where we're already serial-shutting-down anyway.
        if let Err(e) = self.provider.shutdown() {
            eprintln!("[otel] Shutdown flush failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // OTEL env-var reads contend on a single process-global state, so
    // serialise the test cases that touch them. `unwrap_or_else` on
    // PoisonError keeps a panicking test from cascading into the next.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Without `OTEL_EXPORTER_OTLP_ENDPOINT`, `init()` must return
    /// `None` rather than failing — telemetry is opt-in.
    ///
    /// Note: we intentionally do NOT have a paired "with endpoint set,
    /// init returns Some" test. The OTLP exporter builder (tonic-based)
    /// needs a live tokio runtime at construction time; in a sync test
    /// without `#[tokio::test]` the builder panics inside hyper-util.
    /// The realistic init path is exercised by booting cimmeria-server
    /// with `OTEL_EXPORTER_OTLP_ENDPOINT` set and the `otel-smoke`
    /// docker profile from `compose.signoz.yml` (smoke test, not unit).
    #[test]
    fn init_returns_none_when_endpoint_unset() {
        let _lock = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
        assert!(init().is_none(), "no endpoint → no layer");
    }
}
