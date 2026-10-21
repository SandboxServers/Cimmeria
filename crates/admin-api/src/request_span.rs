//! The per-request tracing span, shared by the admin router
//! ([`crate::build_router`]) and the login-port telemetry router
//! ([`crate::login_port_telemetry_router`]).

use std::net::SocketAddr;
use std::time::Instant;

use axum::body::Body;
use axum::extract::{ConnectInfo, MatchedPath};
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use tracing::{Level, Span};

/// The span around one request: tower-http's `DefaultMakeSpan` at INFO
/// (name `request`; fields `method`, `uri`, `version`), except that `uri`
/// is the path alone.
///
/// The default records the whole request target. Every row a handler
/// writes sits inside this span, and the console and `server.log` layers
/// print span fields on every event line, so the default would put two
/// pieces of caller-chosen text beside each of the 161 rows one
/// launcher-summary request can write:
///
/// - a query string, up to 64 KiB of it;
/// - the scheme and authority of an absolute-form target
///   (`POST http://host/api/… HTTP/1.1`), which is served like the bare
///   path.
///
/// The login port is public, and the admin port is reachable by anyone who
/// can route TCP to it (#439), so both routers use this span. No route on
/// the login port reads a query string. Some admin routes do (the login
/// audit list, for one); their filters are no longer in the span, and a
/// handler that wants one in its logs records it itself.
///
/// The method and the path are still the caller's. A request whose path or
/// method matches no route gets a 404 or 405 and writes no rows, so that
/// text appears once per request, not once per row.
///
/// The target is spelled out as tower-http's own. The OTLP filter turns
/// `tower` off (`OTEL_FILTER` in `crates/server/src/logging/filters.rs`), so
/// tower-http's span is not exported; under this module's path the span
/// would start being exported, and the console and file layers would file
/// it under a different target than before.
pub(crate) fn request_span(request: &Request<Body>) -> Span {
    tracing::info_span!(
        target: "tower_http::trace::make_span",
        "request",
        method = %request.method(),
        uri = %request.uri().path(),
        version = ?request.version(),
    )
}

/// The `admin.request` row: one per request to the admin router, written
/// inside the request span by [`admin_request_row`]'s middleware.
///
/// `path` is the matched route (`"unmatched"` when no route matched, so a
/// probe cannot write free text), `peer` is the IP alone (absent on a
/// telemetry or dev-session path, see [`is_telemetry_path`]), and a 5xx
/// status is a WARN. Tower-http's own events are off, so this row is the
/// only request row the admin router writes.
///
/// A middleware rather than a `TraceLayer` `on_response` closure: that
/// closure gets the response and the span but not the request, so it has
/// no method, route or peer to write.
pub(crate) async fn admin_request_row(request: Request<Body>, next: Next) -> Response {
    let method = request.method().as_str().to_owned();
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("unmatched", MatchedPath::as_str)
        .to_owned();
    // A telemetry uploader is named by its token's ids, so its row never
    // carries the address; other admin paths are operator traffic and keep it.
    let peer = if is_telemetry_path(&path) {
        None
    } else {
        request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(addr)| addr.ip().to_string())
    };
    let started = Instant::now();
    let response = next.run(request).await;
    let latency_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let status = response.status().as_u16();
    // `tracing` needs a constant level per callsite, so the level choice
    // (`row_level`) picks between two callsites.
    if row_level(status) == Level::WARN {
        tracing::warn!(
            target: "admin.request",
            method = %method,
            path = %path,
            status,
            latency_ms,
            peer = peer.as_deref(),
            "admin request failed"
        );
    } else {
        tracing::info!(
            target: "admin.request",
            method = %method,
            path = %path,
            status,
            latency_ms,
            peer = peer.as_deref(),
            "admin request"
        );
    }
    response
}

/// The level of an `admin.request` row: WARN for a 5xx status, INFO otherwise.
pub(crate) fn row_level(status: u16) -> Level {
    if status >= 500 {
        Level::WARN
    } else {
        Level::INFO
    }
}

/// Whether a matched `path` is a telemetry or dev-session route, whose
/// `admin.request` rows carry no peer address.
pub(crate) fn is_telemetry_path(path: &str) -> bool {
    path.starts_with("/api/telemetry") || path.starts_with("/api/auth/dev-session")
}

/// The recorder the request-span tests share.
#[cfg(test)]
pub(crate) mod recorder {
    use std::sync::{Arc, Mutex};

    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id, Record};
    use tracing::{Event, Subscriber};
    use tracing_subscriber::layer::{Context, Layer};

    /// Everything a subscriber is shown, as `name=value` lines: each span's
    /// name, target and fields (at creation and when recorded later) and
    /// each event's target and fields. The capture layers elsewhere in this crate record
    /// event fields only, which is why they could not see what a request
    /// span holds.
    #[derive(Clone, Default)]
    pub(crate) struct Shown(Arc<Mutex<Vec<String>>>);

    struct Lines<'a>(&'a mut Vec<String>);

    impl Visit for Lines<'_> {
        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.0.push(format!("{}={value:?}", field.name()));
        }
        fn record_str(&mut self, field: &Field, value: &str) {
            self.0.push(format!("{}={value}", field.name()));
        }
    }

    impl<S: Subscriber> Layer<S> for Shown {
        fn on_new_span(&self, attrs: &Attributes<'_>, _: &Id, _: Context<'_, S>) {
            let mut lines = self.0.lock().unwrap();
            let metadata = attrs.metadata();
            lines.push(format!(
                "span={} target={}",
                metadata.name(),
                metadata.target()
            ));
            attrs.record(&mut Lines(&mut lines));
        }
        fn on_record(&self, _: &Id, values: &Record<'_>, _: Context<'_, S>) {
            values.record(&mut Lines(&mut self.0.lock().unwrap()));
        }
        fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
            let mut lines = self.0.lock().unwrap();
            lines.push(format!("event target={}", event.metadata().target()));
            lines.push(format!("event level={}", event.metadata().level()));
            event.record(&mut Lines(&mut lines));
        }
    }

    impl Shown {
        pub(crate) fn lines(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::Arc;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::broadcast;
    use tracing::subscriber::DefaultGuard;
    use tracing::Level;
    use tracing_subscriber::layer::SubscriberExt;

    use super::recorder::Shown;
    use super::{is_telemetry_path, row_level};
    use crate::ws::broadcast_layer::LogBuffer;
    use cimmeria_services::audit::LoginEventBuffer;
    use cimmeria_services::orchestrator::Orchestrator;

    /// A plain `GET` over raw HTTP/1.1, returning the whole response. Raw
    /// TCP keeps the test free of an HTTP-client dependency.
    async fn get(addr: SocketAddr, path: &str) -> String {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        String::from_utf8_lossy(&response).into_owned()
    }

    /// Records everything shown to a subscriber on this thread, and serves
    /// the real admin router with connect info (so the peer is recorded).
    /// The returned guards keep the recorder the thread default until dropped.
    async fn serve_recorded() -> (SocketAddr, Shown, (tracing::Dispatch, DefaultGuard)) {
        let shown = Shown::default();
        // Two live subscribers stop a cached "nobody is listening" from
        // hiding the row; the thread default sees the listener's tasks.
        let second = tracing::Dispatch::new(tracing_subscriber::registry());
        let default =
            tracing::subscriber::set_default(tracing_subscriber::registry().with(shown.clone()));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        // The orchestrator is only constructed, never started; no database
        // is needed for the roster route to answer 200.
        let app = crate::build_router(
            Arc::new(Orchestrator::new(Default::default())),
            broadcast::channel(1).0,
            LogBuffer::new(),
            broadcast::channel(1).0,
            LoginEventBuffer::default(),
        );
        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
        (addr, shown, (second, default))
    }

    /// One `admin.request` row for `GET /api/players`, from the real admin
    /// router served with connect info (so the peer is recorded). Reverting
    /// the middleware layer in `build_router` leaves no such row.
    #[tokio::test]
    async fn admin_request_row_names_method_path_and_status() {
        let (addr, shown, _guards) = serve_recorded().await;

        let response = get(addr, "/api/players").await;
        assert!(response.starts_with("HTTP/1.1 200"), "{response:?}");

        let lines = shown.lines();
        let row = lines
            .iter()
            .position(|line| line == "event target=admin.request")
            .unwrap_or_else(|| panic!("no admin.request row in {lines:#?}"));
        assert_eq!(lines[row + 1], "event level=INFO", "{lines:#?}");
        let fields = &lines[row..];
        for wanted in [
            "method=GET",
            "path=/api/players",
            "status=200",
            "peer=127.0.0.1",
        ] {
            assert!(
                fields.iter().any(|line| line == wanted),
                "no `{wanted}` after the row in {lines:#?}"
            );
        }
        let rows = lines
            .iter()
            .filter(|line| *line == "event target=admin.request")
            .count();
        assert_eq!(rows, 1, "{lines:#?}");
    }

    /// A probe to no route: its row says `path=unmatched`, not the probe's
    /// text, and the query string is written nowhere. The request span's own
    /// `uri` is the path alone, so `zz-probe` appears in that one line only.
    #[tokio::test]
    async fn unmatched_probe_row_writes_no_raw_uri() {
        let (addr, shown, _guards) = serve_recorded().await;

        let response = get(addr, "/zz-probe?x=1").await;
        assert!(response.starts_with("HTTP/1.1 404"), "{response:?}");

        let lines = shown.lines();
        let row = lines
            .iter()
            .position(|line| line == "event target=admin.request")
            .unwrap_or_else(|| panic!("no admin.request row in {lines:#?}"));
        let fields = &lines[row..];
        assert!(
            fields.iter().any(|line| line == "path=unmatched"),
            "{lines:#?}"
        );
        assert!(fields.iter().any(|line| line == "status=404"), "{lines:#?}");
        let probes: Vec<&String> = lines
            .iter()
            .filter(|line| line.contains("zz-probe"))
            .collect();
        assert_eq!(probes, [&"uri=/zz-probe".to_owned()], "{lines:#?}");
        assert!(lines.iter().all(|line| !line.contains("x=1")), "{lines:#?}");
    }

    /// A telemetry request's row has no `peer`, though the connect-info peer
    /// is there. The `GET` reaches the route's method guard (405), which is
    /// enough for the matched path to be the telemetry route.
    #[tokio::test]
    async fn telemetry_request_row_has_no_peer() {
        let (addr, shown, _guards) = serve_recorded().await;

        let response = get(addr, "/api/telemetry/upload-chunk").await;
        assert!(response.starts_with("HTTP/1.1 405"), "{response:?}");

        let lines = shown.lines();
        let row = lines
            .iter()
            .position(|line| line == "event target=admin.request")
            .unwrap_or_else(|| panic!("no admin.request row in {lines:#?}"));
        let fields = &lines[row..];
        assert!(
            fields
                .iter()
                .any(|line| line == "path=/api/telemetry/upload-chunk"),
            "{lines:#?}"
        );
        assert!(fields.iter().any(|line| line == "status=405"), "{lines:#?}");
        assert!(
            fields.iter().all(|line| !line.starts_with("peer=")),
            "{lines:#?}"
        );
        assert!(
            fields.iter().all(|line| !line.contains("127.0.0.1")),
            "{lines:#?}"
        );
    }

    /// The telemetry and dev-session prefixes are the only paths without a
    /// peer; the rest of the admin router keeps it.
    #[test]
    fn only_telemetry_and_dev_session_paths_drop_the_peer() {
        assert!(is_telemetry_path("/api/telemetry/upload-chunk"));
        assert!(is_telemetry_path("/api/telemetry/launcher-summary"));
        assert!(is_telemetry_path("/api/auth/dev-session"));
        assert!(is_telemetry_path("/api/auth/dev-session/refresh"));
        assert!(!is_telemetry_path("/api/players"));
        assert!(!is_telemetry_path("/api/auth/login-audit"));
        assert!(!is_telemetry_path("unmatched"));
    }

    /// The row level is INFO below 500 and WARN from 500 up.
    #[test]
    fn row_level_is_warn_only_for_a_5xx_status() {
        assert_eq!(row_level(200), Level::INFO);
        assert_eq!(row_level(404), Level::INFO);
        assert_eq!(row_level(499), Level::INFO);
        assert_eq!(row_level(500), Level::WARN);
        assert_eq!(row_level(503), Level::WARN);
    }
}
