//! OpenTelemetry exporter — ships tracing logs + spans to a SigNoz
//! deployment (or any other OTLP-compatible backend) via the OTEL
//! Collector.
//!
//! # SigNoz service split
//!
//! The OTLP log signal is split across **four** providers, each tagged
//! with its own `service.name` resource:
//!
//! - **`cimmeria-server`** — the high-signal index. Auth, content
//!   chains, combat, missions, inventory, vendor, abilities. Default
//!   operator view for triage. Receives WARN+ from every scope
//!   regardless of routing — elevated severity always lands here.
//! - **`cimmeria-network`** — the high-noise wire-level index. Every
//!   `mercury_packet` event, every bundle decrypt, every cell-arms
//!   dispatch, tick-sync heartbeats. Operators query this index when
//!   chasing wire-level issues; it never drowns the main view at
//!   normal severity.
//! - **`cimmeria-trace`** — every TRACE-level row that reaches an on-disk
//!   log file, plus the custom-target TRACE rows and the 1-in-N samples of
//!   the per-packet firehoses (NA25). The other two indexes never receive
//!   TRACE, so no record is indexed twice.
//! - **`cimmeria-client`** — what the players' machines sent: the injected
//!   `cimmeria-client-telemetry` DLL's events (`client.native`) and the
//!   game-log lines the launcher tails (`launcher.client_log`,
//!   `launcher.debug_log`, `launcher.session_meta`), replayed by the
//!   `/api/telemetry/upload-*` ingest. Every level, and only here: a
//!   client-side warning is not a server problem, so it does not land in
//!   `cimmeria-server`. The resource carries `cimmeria.source = client`
//!   (see [`client_resource_attributes`]).
//!
//! Routing is by level plus [`is_network_noise_target`] and
//! [`is_client_target`]; the filters and the routing table live in
//! `crates/server/src/logging/filters.rs`.
//!
//! # Architecture
//!
//! Three `tracing_subscriber::Layer`s composed alongside the file/broadcast
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
//! Launcher and client logs land here too — the
//! `/api/telemetry/upload-{chunk,bundle}` endpoints (see
//! [`cimmeria_admin_api::routes::telemetry`]) replay each uploaded
//! event through `tracing::*` so the OTLP layers ship them to the same
//! SigNoz store as the server's own logs and Mercury packet events, the
//! client-side ones under `cimmeria-client`.
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
//! | `CIMMERIA_DEPLOY_ENV` | Default `"dev"`. Sets `deployment.environment` and `cimmeria.deploy_env` on every signal. |
//!
//! # Deploy identity
//!
//! Every provider's resource carries (see [`identity_attributes`]):
//!
//! - `deployment.environment` and `cimmeria.deploy_env` — from
//!   `CIMMERIA_DEPLOY_ENV` (`colo` on the colo, `dev` elsewhere);
//! - `host.name` — the machine (or container) hostname;
//! - `service.version` — the git commit baked in at build time by
//!   `crates/server/build.rs` (`CIMMERIA_GIT_SHA` in the container build,
//!   `git rev-parse HEAD` otherwise, `"unknown"` when neither is available).
//!
//! Without these a colo row and a dev-laptop row were indistinguishable and
//! no row could be tied to a build (audit gap T2).
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

/// The span layer and one log layer per SigNoz log index. `init_logging`
/// gives each its own filter.
pub type OtelTraceLayer = OpenTelemetryLayer<Registry, SdkTracer>;
pub type OtelLogLayer = OpenTelemetryTracingBridge<SdkLoggerProvider, SdkLogger>;

/// Everything [`init`] hands to `init_logging`.
pub struct OtelLayers {
    /// Spans (`tracing::span!`) and the events inside them.
    pub trace: OtelTraceLayer,
    /// `service.name = cimmeria-server`.
    pub server_log: OtelLogLayer,
    /// `service.name = cimmeria-network`.
    pub network_log: OtelLogLayer,
    /// `service.name = cimmeria-trace`.
    pub trace_log: OtelLogLayer,
    /// `service.name = cimmeria-client`.
    pub client_log: OtelLogLayer,
}

/// Default service name for the high-signal index (auth, content,
/// combat, missions, etc.).
const DEFAULT_SERVICE_NAME: &str = "cimmeria-server";

/// Service name for the high-noise wire-level index (mercury packets,
/// bundle decode, cell-arms dispatch, tick-sync heartbeats). Splitting
/// these into their own SigNoz service stops the volume from drowning
/// the high-signal events in the main index — operators query
/// `service.name = 'cimmeria-network'` only when chasing wire-level
/// issues, and keep `service.name = 'cimmeria-server'` as the default
/// triage view.
///
/// WARN and ERROR events from network-noise scopes are NOT split —
/// elevated severity always lands in the `cimmeria-server` index so a
/// real wire problem surfaces in the operator's primary view without
/// dual-querying. See [`is_network_noise_target`] for the routing
/// predicate.
const NETWORK_SERVICE_NAME: &str = "cimmeria-network";

/// Service name for the TRACE-level index (NA25). Parity with the on-disk
/// files: every TRACE row a `logs/*.log` layer keeps is exported here, and
/// only here. Query `service.name = 'cimmeria-trace'`.
const TRACE_SERVICE_NAME: &str = "cimmeria-trace";

/// Service name for what the players' machines uploaded: the injected
/// client DLL's events and the launcher-tailed game logs. Query
/// `service.name = 'cimmeria-client'`.
pub const CLIENT_SERVICE_NAME: &str = "cimmeria-client";

/// Tracing targets the telemetry ingest replays client-side data under.
/// [`is_client_target`] matches each one and its dotted children.
///
/// `launcher.ingest`, `launcher.bundle` and the dev-session mint rows are
/// deliberately absent: they are the server's own account of an upload
/// (counts, refusals, caps), so they stay in `cimmeria-server`. And
/// `launcher.key_dump` stays off everywhere (see `OTLP_EXCLUDED_TARGETS`).
pub const CLIENT_TARGETS: &[&str] = &[
    "client.native",
    "launcher.client_log",
    "launcher.debug_log",
    "launcher.session_meta",
];

/// Git commit this binary was built from, or `"unknown"`. Set by
/// `crates/server/build.rs`.
pub const BUILD_SHA: &str = env!("CIMMERIA_BUILD_SHA");

/// Resource attributes that identify *which deployment and which build*
/// emitted a signal. Shared by every provider (both log indexes, traces and
/// metrics) so a query can filter on them whichever signal it starts from.
///
/// Explicit builder attributes win over `OTEL_RESOURCE_ATTRIBUTES` in the
/// SDK's resource merge, so these are authoritative.
fn identity_attributes(
    deploy_env: &str,
    host_name: &str,
    version: &str,
) -> Vec<opentelemetry::KeyValue> {
    use opentelemetry::KeyValue;
    vec![
        KeyValue::new("deployment.environment", deploy_env.to_string()),
        KeyValue::new("cimmeria.deploy_env", deploy_env.to_string()),
        KeyValue::new("host.name", host_name.to_string()),
        KeyValue::new("service.version", version.to_string()),
    ]
}

/// Resource attributes for the `cimmeria-client` provider.
///
/// A resource is process-wide, so it can only say what is true of every
/// client row this server ingests: where it came from and which server
/// ingested it. Per-session facts (`session_id`, `session_kind`, the DLL
/// build) ride on each log record instead; see the replay in
/// `cimmeria_admin_api::routes::telemetry`.
///
/// `host.name` and `service.version` are deliberately *not* set: on a
/// client row they would name the ingesting server and its build, which a
/// reader would take for the player's machine and client build. The
/// server's identity is kept under `cimmeria.ingest_host` and
/// `cimmeria.ingest_version` instead.
pub fn client_resource_attributes(
    deploy_env: &str,
    host_name: &str,
    version: &str,
) -> Vec<opentelemetry::KeyValue> {
    use opentelemetry::KeyValue;
    vec![
        KeyValue::new("deployment.environment", deploy_env.to_string()),
        KeyValue::new("cimmeria.deploy_env", deploy_env.to_string()),
        KeyValue::new("cimmeria.source", "client"),
        KeyValue::new("cimmeria.ingest_host", host_name.to_string()),
        KeyValue::new("cimmeria.ingest_version", version.to_string()),
    ]
}

/// The `cimmeria-client` resource: [`CLIENT_SERVICE_NAME`] plus
/// [`client_resource_attributes`]. Public so the ingest round-trip test
/// builds the same resource production does.
pub fn client_resource(deploy_env: &str, host_name: &str, version: &str) -> Resource {
    Resource::builder()
        .with_service_name(CLIENT_SERVICE_NAME)
        .with_attributes(client_resource_attributes(deploy_env, host_name, version))
        .build()
}

/// True if `target` is client-side data replayed by the telemetry ingest,
/// which lands in `cimmeria-client` and nowhere else. Matches each of
/// [`CLIENT_TARGETS`] and its dotted children (`client.native.x`), never a
/// sibling that merely shares the prefix (`client.nativex`).
pub fn is_client_target(target: &str) -> bool {
    CLIENT_TARGETS.iter().any(|t| {
        target
            .strip_prefix(t)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
    })
}

/// This host's name, or `"unknown"` if the OS will not say.
fn host_name() -> String {
    let name = gethostname::gethostname().to_string_lossy().into_owned();
    if name.is_empty() {
        "unknown".to_string()
    } else {
        name
    }
}

/// True if `target` (which OTel surfaces as `scope_name`) is a
/// high-volume wire-level event that should land in the
/// `cimmeria-network` index rather than `cimmeria-server`.
///
/// The list is conservative — anything not explicitly enumerated
/// stays in the main index. Add to this list, not subtract, when a
/// new high-volume scope appears.
///
/// Source of the listed targets:
/// - `mercury.packet` / `mercury.retransmit` / `mercury.backpressure`
///   — explicit `target = "mercury.*"` strings in
///   `crates/mercury/src/instrumentation.rs`, `channel/mod.rs`,
///   `transport.rs`. Per-packet wire-level instrumentation.
/// - `cimmeria_base::base::connect_loop::encrypted` —
///   bundle/decrypt DEBUG logs that fire per inbound packet.
/// - `cimmeria_base::base::connect_loop::cell_arms` — cell-method
///   dispatch debug logs. Both were `cimmeria_services::base::connect_loop::…`
///   until wave B4 of the crate split moved the connect loop to
///   `cimmeria-base`.
/// - `cimmeria_base_session::base::tick_sync` — tick-sync heartbeats and
///   retransmit RTO notices. It was `cimmeria_services::base::tick_sync`
///   until wave B1 of the crate split moved it to `cimmeria-base-session`.
///
/// `cimmeria_mercury::` is the transport crate only. The services-side packet
/// builders in `cimmeria_wire::mercury` (moved out of `cimmeria-services` in
/// wave W3a of the crate split) share the module name but log per map load
/// and per appearance or tint call, not per datagram, so they stay in
/// `cimmeria-server` as they did before the move.
pub fn is_network_noise_target(target: &str) -> bool {
    target.starts_with("mercury.")
        || target == "cimmeria_base::base::connect_loop::encrypted"
        || target == "cimmeria_base::base::connect_loop::cell_arms"
        || target.starts_with("cimmeria_base_session::base::tick_sync")
        || target.starts_with("cimmeria_mercury::")
}

/// Initialize the OTLP exporters and return the tracing layers that ship
/// events through them. Returns `None` (silently) when
/// `OTEL_EXPORTER_OTLP_ENDPOINT` is unset — telemetry is opt-in.
///
/// The returned [`OtelGuard`] must be held for the lifetime of the
/// process — dropping it shuts down both providers, flushing the
/// in-flight batches to the collector. Without this flush, the last
/// few seconds of telemetry before a clean shutdown are lost.
pub fn init() -> Option<(OtelLayers, OtelGuard)> {
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
        env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| DEFAULT_SERVICE_NAME.to_string());
    let deploy_env = env::var("CIMMERIA_DEPLOY_ENV").unwrap_or_else(|_| "dev".to_string());
    let host = host_name();
    let identity = identity_attributes(&deploy_env, &host, BUILD_SHA);
    let resource = Resource::builder()
        .with_service_name(service_name.clone())
        .with_attributes(identity.clone())
        .build();
    // High-noise wire-level events ride a separate provider with
    // `service.name = cimmeria-network`. Same deployment.environment
    // so the dev/colo split still applies, but SigNoz indexes them as
    // a distinct service so operators can isolate the noise stream
    // when triaging. See `is_network_noise_target` for the routing
    // predicate; the per-layer filter is applied in `main.rs`.
    let network_resource = Resource::builder()
        .with_service_name(NETWORK_SERVICE_NAME)
        .with_attributes(identity.clone())
        .build();
    // TRACE rows get a third service for the same reason: their volume is
    // an order of magnitude above everything else, and an operator asks for
    // them explicitly.
    let trace_resource = Resource::builder()
        .with_service_name(TRACE_SERVICE_NAME)
        .with_attributes(identity)
        .build();
    // Client-side uploads get a fourth: they describe the players'
    // machines, not this server, and the operator reads them apart.
    let client_resource = client_resource(&deploy_env, &host, BUILD_SHA);
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

    // ── Log exporters (root-level events), one per SigNoz log index ──
    //
    // Same OTLP endpoint, different `service.name` resource, so SigNoz shows
    // each as its own service. Routing happens in the per-layer filters that
    // `init_logging` puts on the bridges. Each provider costs one batch
    // exporter and one channel — small against the volume it isolates.
    let logger_provider = log_provider(&protocol, &endpoint, resource.clone(), "Log")?;
    let network_logger_provider =
        log_provider(&protocol, &endpoint, network_resource, "Network log")?;
    let trace_logger_provider = log_provider(&protocol, &endpoint, trace_resource, "Trace log")?;
    let client_logger_provider = log_provider(&protocol, &endpoint, client_resource, "Client log")?;

    let log_layer = OpenTelemetryTracingBridge::new(&logger_provider);
    let network_log_layer = OpenTelemetryTracingBridge::new(&network_logger_provider);
    let trace_log_layer = OpenTelemetryTracingBridge::new(&trace_logger_provider);
    let client_log_layer = OpenTelemetryTracingBridge::new(&client_logger_provider);

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
        "[otel] Streaming to {endpoint} (protocol={protocol}, signals=traces+logs{metrics}, deployment.environment={deploy_env}, host.name={host}, service.version={BUILD_SHA})",
        metrics = if meter_provider.is_some() { "+metrics" } else { "" },
    );

    Some((
        OtelLayers {
            trace: trace_layer,
            server_log: log_layer,
            network_log: network_log_layer,
            trace_log: trace_log_layer,
            client_log: client_log_layer,
        },
        OtelGuard {
            tracer_provider,
            logger_provider,
            network_logger_provider,
            trace_logger_provider,
            client_logger_provider,
            meter_provider,
        },
    ))
}

/// Build one batch-exporting logger provider for `resource`.
///
/// `None` on exporter failure, and [`init`] then ships nothing at all: the
/// bridge layer has no cheap no-op form, so partial telemetry would need an
/// `Option` threaded through every layer. Full-or-nothing, loudly — the
/// operator fixes the config. `eprintln!` because the subscriber is not
/// installed yet.
fn log_provider(
    protocol: &str,
    endpoint: &str,
    resource: Resource,
    label: &str,
) -> Option<SdkLoggerProvider> {
    let exporter = match protocol {
        "http/protobuf" | "http" => opentelemetry_otlp::LogExporter::builder()
            .with_http()
            .with_endpoint(endpoint)
            .build(),
        _ => opentelemetry_otlp::LogExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build(),
    };
    match exporter {
        Ok(e) => Some(
            SdkLoggerProvider::builder()
                .with_batch_exporter(e)
                .with_resource(resource)
                .build(),
        ),
        Err(err) => {
            eprintln!("[otel] {label} exporter init failed ({err}); telemetry will not ship");
            None
        }
    }
}

/// RAII guard — when dropped, flushes the in-flight batches to the
/// collector via the SDK provider shutdown calls. Holding this in
/// `main()` until after the orchestrator's `stop_all` returns keeps the
/// last few hundred ms of shutdown telemetry from being dropped.
pub struct OtelGuard {
    tracer_provider: SdkTracerProvider,
    logger_provider: SdkLoggerProvider,
    /// Second logger provider for the high-noise `cimmeria-network`
    /// index. Same shutdown discipline as `logger_provider` — its
    /// batches must flush before the gRPC channel closes or the last
    /// wire-level packets get dropped on a clean shutdown.
    network_logger_provider: SdkLoggerProvider,
    /// Third logger provider, for the `cimmeria-trace` index.
    trace_logger_provider: SdkLoggerProvider,
    /// Fourth logger provider, for the `cimmeria-client` index.
    client_logger_provider: SdkLoggerProvider,
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
        if let Err(e) = self.network_logger_provider.shutdown() {
            eprintln!("[otel] Network logger shutdown flush failed: {e}");
        }
        if let Err(e) = self.trace_logger_provider.shutdown() {
            eprintln!("[otel] Trace logger shutdown flush failed: {e}");
        }
        if let Err(e) = self.client_logger_provider.shutdown() {
            eprintln!("[otel] Client logger shutdown flush failed: {e}");
        }
    }
}

#[cfg(test)]
#[path = "otel_tests.rs"]
mod tests;
