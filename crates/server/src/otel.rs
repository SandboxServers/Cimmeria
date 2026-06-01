//! OpenTelemetry exporter — ships tracing logs + spans to a SigNoz
//! deployment (or any other OTLP-compatible backend) via the OTEL
//! Collector.
//!
//! # Architecture
//!
//! Two `tracing_subscriber::Layer`s composed alongside the file/broadcast
//! layers, plus a third metrics provider registered with the OTel global
//! state:
//!
//! 1. **Trace layer** ([`tracing_opentelemetry::OpenTelemetryLayer`]) —
//!    converts `tracing::span!` spans into OpenTelemetry spans. Span
//!    fields and events fired *inside* an active span become attributes
//!    and nested span events.
//! 2. **Log layer** ([`OpenTelemetryTracingBridge`]) — captures *every*
//!    `tracing` event as an OpenTelemetry log record, including events
//!    fired at the top level outside any active span.
//! 3. **Metrics provider** ([`SdkMeterProvider`]) — registered as the
//!    global Meter so the [`cimmeria_observability`] facade's
//!    `counter!`/`histogram!`/`gauge_add!` macros emit through the
//!    same OTLP endpoint as traces + logs. SigNoz ingests all three
//!    via the same collector. Per the instrumentation-discipline ADR
//!    (`docs/architecture/instrumentation-discipline.md`), labels are
//!    enumerated low-cardinality strings — never `entity_id`/`player_id`.
//!
//! Both are needed: without the log layer, every `tracing::info!` that
//! fires outside a span (the entire `mercury.packet` event stream
//! lives here) gets silently dropped by the trace exporter and never
//! reaches SigNoz. Wiring the appender bridges those orphan events to
//! the OTLP log signal so they show up in SigNoz's Logs view.
//!
//! Launcher logs land here too — the
//! `/api/telemetry/upload-{chunk,bundle}` endpoints (see
//! [`cimmeria_admin_api::routes::telemetry`]) replay each launcher
//! event through `tracing::*` so the OTLP layers ship them to the same
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
//! | `CIMMERIA_DEPLOY_ENV` | Default `"dev"`. Sets `deployment.environment` on every signal — overridable by `OTEL_RESOURCE_ATTRIBUTES`. |
//!
//! All env vars match the OpenTelemetry SDK spec — pinned so the
//! standard `opentelemetry-otlp` crate reads them directly without us
//! re-implementing the conventions.

use std::env;

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::logs::{SdkLogger, SdkLoggerProvider};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{Sampler, SdkTracer, SdkTracerProvider};
use opentelemetry_sdk::Resource;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::Registry;

/// Composed return type for [`init`] — the trace layer (spans) and the
/// log layer (root-level events) flow through different SDK paths, so
/// `main()` adds them both to the layered subscriber when present.
pub type OtelTraceLayer = OpenTelemetryLayer<Registry, SdkTracer>;
pub type OtelLogLayer = OpenTelemetryTracingBridge<SdkLoggerProvider, SdkLogger>;

/// Initialize the OTLP exporters and return the pair of tracing layers
/// that ship events through them. Returns `None` (silently) when
/// `OTEL_EXPORTER_OTLP_ENDPOINT` is unset — telemetry is opt-in.
///
/// The returned [`OtelGuard`] must be held for the lifetime of the
/// process — dropping it shuts down both providers, flushing the
/// in-flight batches to the collector. Without this flush, the last
/// few seconds of telemetry before a clean shutdown are lost.
pub fn init() -> Option<(OtelTraceLayer, OtelLogLayer, OtelGuard)> {
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
    //
    // `deployment.environment` defaults to `"dev"` and is set from the
    // `CIMMERIA_DEPLOY_ENV` env var (e.g. `colo`, `staging`, `dev`).
    // SigNoz dashboards use this to split aggregates between
    // dev-laptop noise and the colo's production-shaped traffic — if
    // it's unset the colo dashboards would silently include local
    // events and skew every aggregate.
    let service_name =
        env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "cimmeria-server".to_string());
    let deploy_env = env::var("CIMMERIA_DEPLOY_ENV").unwrap_or_else(|_| "dev".to_string());
    let resource = Resource::builder()
        .with_service_name(service_name.clone())
        .with_attribute(opentelemetry::KeyValue::new(
            "deployment.environment",
            deploy_env.clone(),
        ))
        .build();
    // OTEL_RESOURCE_ATTRIBUTES is parsed by `opentelemetry_sdk` itself
    // when present, so we don't need to manually split-and-merge it
    // here — the SDK union-merges over our explicit Resource above.
    // An operator who sets `OTEL_RESOURCE_ATTRIBUTES=deployment.environment=colo`
    // overrides the `CIMMERIA_DEPLOY_ENV` default per the SDK's
    // env-var merge precedence.

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

    // ── Trace exporter (spans) ────────────────────────────────────────
    let span_exporter_result = match protocol.as_str() {
        "http/protobuf" | "http" => opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(&endpoint)
            .build(),
        _ => opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(&endpoint)
            .build(),
    };

    let span_exporter = match span_exporter_result {
        Ok(e) => e,
        Err(err) => {
            // Loud: if OTEL_* env vars are set we expect connectivity.
            // Falling back to "no OTLP" silently would mask deployment
            // misconfiguration. eprintln! (not tracing::error!) because
            // the tracing subscriber isn't installed yet when we run.
            eprintln!("[otel] Span exporter init failed ({err}); telemetry will not ship");
            return None;
        }
    };

    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(span_exporter)
        .with_sampler(sampler)
        .with_resource(resource.clone())
        .build();

    let tracer = tracer_provider.tracer(service_name.clone());
    // Set as the global provider so any code path that grabs
    // `opentelemetry::global::tracer(...)` picks up our exporter.
    opentelemetry::global::set_tracer_provider(tracer_provider.clone());

    let trace_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    // ── Log exporter (root-level events) ──────────────────────────────
    let log_exporter_result = match protocol.as_str() {
        "http/protobuf" | "http" => opentelemetry_otlp::LogExporter::builder()
            .with_http()
            .with_endpoint(&endpoint)
            .build(),
        _ => opentelemetry_otlp::LogExporter::builder()
            .with_tonic()
            .with_endpoint(&endpoint)
            .build(),
    };

    let log_exporter = match log_exporter_result {
        Ok(e) => e,
        Err(err) => {
            // Fail-loud and bail entirely — the bridge layer type isn't
            // trivially constructable as a no-op, so partial telemetry
            // (traces only) would require keeping a parallel "log layer
            // is `Option<...>`" path through the rest of init. Cleaner
            // to ship full-or-nothing and let the operator fix config.
            eprintln!("[otel] Log exporter init failed ({err}); telemetry will not ship");
            return None;
        }
    };

    let logger_provider = SdkLoggerProvider::builder()
        .with_batch_exporter(log_exporter)
        .with_resource(resource.clone())
        .build();

    let log_layer = OpenTelemetryTracingBridge::new(&logger_provider);

    // ── Metrics exporter (counters + histograms) ──────────────────────
    //
    // The metrics SDK is initialised AFTER logs so a metrics-exporter
    // failure logs through the (already-installed) trace + log pipe
    // before the process exits. Soft-fail by design: telemetry stays
    // enabled with traces+logs even when metrics flake — the missing
    // counters surface as gaps in the SigNoz dashboard, not as a dead
    // server.
    let metric_exporter_result = match protocol.as_str() {
        "http/protobuf" | "http" => opentelemetry_otlp::MetricExporter::builder()
            .with_http()
            .with_endpoint(&endpoint)
            .build(),
        _ => opentelemetry_otlp::MetricExporter::builder()
            .with_tonic()
            .with_endpoint(&endpoint)
            .build(),
    };

    let meter_provider = match metric_exporter_result {
        Ok(exporter) => {
            // PeriodicReader ships a metric batch every 60s by default.
            // That's the OTLP spec default and matches what SigNoz
            // dashboards expect — finer granularity costs more storage
            // for marginal observability benefit.
            let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter).build();
            let provider = SdkMeterProvider::builder()
                .with_reader(reader)
                .with_resource(resource)
                .build();
            opentelemetry::global::set_meter_provider(provider.clone());
            // Initialise the cimmeria-observability facade so the
            // counter!/histogram! macros pick up the global Meter on
            // the first emission. Idempotent — a second otel::init in
            // the same process is a no-op on the facade.
            if let Err(e) = cimmeria_observability::init("cimmeria-server") {
                eprintln!("[otel] observability facade init returned {e}; metrics may not ship");
            }
            Some(provider)
        }
        Err(err) => {
            eprintln!(
                "[otel] Metric exporter init failed ({err}); traces+logs continue, metrics OFF"
            );
            None
        }
    };

    eprintln!(
        "[otel] Streaming to {endpoint} (protocol={protocol}, signals=traces+logs{metrics}, deployment.environment={deploy_env})",
        metrics = if meter_provider.is_some() { "+metrics" } else { "" },
    );

    Some((
        trace_layer,
        log_layer,
        OtelGuard {
            tracer_provider,
            logger_provider,
            meter_provider,
        },
    ))
}

/// RAII guard — when dropped, flushes the in-flight batches to the
/// collector via the SDK provider shutdown calls. Holding this in
/// `main()` until after the orchestrator's `stop_all` returns keeps the
/// last few hundred ms of shutdown telemetry from being dropped.
pub struct OtelGuard {
    tracer_provider: SdkTracerProvider,
    logger_provider: SdkLoggerProvider,
    /// `None` when the metric exporter failed to construct — traces +
    /// logs still flush on shutdown, metrics path was never wired so
    /// nothing to drain.
    meter_provider: Option<SdkMeterProvider>,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        // shutdown() blocks until each in-flight batch flushes. ~250ms
        // worst-case per provider; usually much faster. We do this on
        // the main thread, after `stop_all` returns, so the cost is on
        // a path where we're already serial-shutting-down anyway.
        //
        // Metric shutdown runs FIRST so the final batch ships through
        // before tracer/logger shutdown closes the gRPC tonic channel
        // they share — out-of-order shutdown silently loses the last
        // metric batch on a 60s emit cadence (so up to ~60s of counter
        // emissions if shutdown lands mid-cycle).
        if let Some(provider) = &self.meter_provider {
            if let Err(e) = provider.shutdown() {
                eprintln!("[otel] Meter shutdown flush failed: {e}");
            }
        }
        if let Err(e) = self.tracer_provider.shutdown() {
            eprintln!("[otel] Tracer shutdown flush failed: {e}");
        }
        if let Err(e) = self.logger_provider.shutdown() {
            eprintln!("[otel] Logger shutdown flush failed: {e}");
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
    /// with `OTEL_EXPORTER_OTLP_ENDPOINT` set against a live SigNoz
    /// (smoke test, not unit).
    #[test]
    fn init_returns_none_when_endpoint_unset() {
        let _lock = ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");
        assert!(init().is_none(), "no endpoint → no layer");
    }
}
