//! Launcher telemetry routes for the public SOAP login port.
//!
//! The admin API listener is private and unauthenticated (#439), so a
//! player's launcher on another machine cannot reach it. The SOAP login
//! listener (`LOGON_PORT`, 8081) is already internet-facing: every player
//! sends their password to it over plain HTTP. Decision (@Cadacious,
//! 2026-09-29): mount the four launcher telemetry routes on that listener
//! too, at the same paths as on the admin port, so a remote launcher needs
//! no config and no secret.
//!
//! A fifth route, `/api/telemetry/launcher-summary`, is merged below for
//! the desktop launcher's attempt summaries. It is outside the recorded
//! four-route decision: serving it publicly needs the maintainer's explicit
//! yes before a build carrying this merge is deployed. No shipped launcher
//! calls it (the exporter's endpoint is unset).
//!
//! Only these routes are exposed. Everything else under `/api`
//! (players, config, entities, the admin `/api/auth/login`) and the `/ws`
//! and Swagger surfaces stay on the admin listener only.

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use tower_http::trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer};
use tracing::{Level, Span};

use crate::routes::{dev_session, telemetry};

/// Router with exactly `/api/auth/dev-session`,
/// `/api/auth/dev-session/refresh`, `/api/telemetry/upload-chunk`,
/// `/api/telemetry/upload-bundle` and `/api/telemetry/launcher-summary`,
/// for the auth service to merge into its login router.
///
/// The per-route body limits come with [`dev_session::routes`],
/// [`telemetry::routes`] and [`telemetry::launcher_summary_routes`]. The
/// mint, refresh and summary quotas read the peer
/// address, so the listener must serve with
/// `into_make_service_with_connect_info::<SocketAddr>()`, which both auth
/// listeners already do.
pub fn login_port_telemetry_router() -> Router {
    Router::new()
        .nest(
            "/api",
            Router::new().nest("/auth", dev_session::routes()).nest(
                "/telemetry",
                telemetry::routes().merge(telemetry::launcher_summary_routes()),
            ),
        )
        // The same per-request span as the admin router, except that it
        // leaves the query string out: see `request_span`.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(request_span)
                .on_response(DefaultOnResponse::new().level(Level::INFO))
                .on_failure(DefaultOnFailure::new().level(Level::WARN)),
        )
}

/// The span around one request on the login port: tower-http's
/// `DefaultMakeSpan` at INFO (name `request`; fields `method`, `uri`,
/// `version`), except that `uri` is the path alone.
///
/// The default records the whole URI. This listener is public, and every
/// row a handler writes sits inside this span, so a query string would put
/// up to 64 KiB of caller-chosen text beside each of the 161 rows one
/// launcher-summary request can write (the console and `server.log` layers
/// print span fields on every event line). No route here reads a query
/// string, so nothing is lost by dropping it.
///
/// The method and the path are still the caller's. A request whose path or
/// method matches no route gets a 404 or 405 and writes no rows, so that
/// text appears once per request, not once per row.
///
/// The target is spelled out as tower-http's own. The OTLP filter turns
/// `tower` off (`OTEL_FILTER` in `crates/server/src/logging/filters.rs`), so
/// tower-http's span is not exported; under this module's path the span
/// would start being exported, and the console and file layers would file
/// it under a different target than the admin router's.
fn request_span(request: &Request<Body>) -> Span {
    tracing::info_span!(
        target: "tower_http::trace::make_span",
        "request",
        method = %request.method(),
        uri = %request.uri().path(),
        version = ?request.version(),
    )
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tracing::field::{Field, Visit};
    use tracing::span::{Attributes, Id, Record};
    use tracing::{Event, Subscriber};
    use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

    use super::login_port_telemetry_router;

    /// Serve the router on an ephemeral loopback port the way the auth
    /// listeners do (with connect info), and return its address.
    async fn serve() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = login_port_telemetry_router();
        tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        });
        addr
    }

    /// Send a bodyless request over raw HTTP/1.1 and return the whole
    /// response, head and body. Raw TCP keeps the test free of an
    /// HTTP-client dependency.
    async fn exchange(addr: SocketAddr, method: &str, path: &str) -> String {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        String::from_utf8_lossy(&response).into_owned()
    }

    /// The status code of a bodyless request.
    async fn status(addr: SocketAddr, method: &str, path: &str) -> u16 {
        let response = exchange(addr, method, path).await;
        let status_line = response.lines().next().unwrap_or_default();
        status_line
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or_else(|| panic!("no status in response to {method} {path}: {response:?}"))
    }

    #[tokio::test]
    async fn login_port_router_answers_the_four_telemetry_routes() {
        let addr = serve().await;

        // A bodyless POST with no Content-Type is refused by the Json
        // extractor (415), which runs only once the route matched and
        // ConnectInfo was extracted.
        assert_eq!(status(addr, "POST", "/api/auth/dev-session").await, 415);
        assert_eq!(
            status(addr, "POST", "/api/auth/dev-session/refresh").await,
            415
        );
        // The chunk handler checks the bearer token first: 401 proves the
        // handler itself ran.
        assert_eq!(
            status(addr, "POST", "/api/telemetry/upload-chunk").await,
            401
        );
        // Multipart rejects a request without a multipart boundary before
        // the handler body; anything but 404/405 proves the route is there.
        let bundle = status(addr, "POST", "/api/telemetry/upload-bundle").await;
        assert!(
            bundle != 404 && bundle != 405,
            "upload-bundle not mounted: {bundle}"
        );
    }

    /// The launcher-summary ingest is the fifth route. Its handler checks
    /// the kill switch and then the bearer token, so a bodyless POST gets a
    /// 401 (or a 503 while another test holds the kill switch on); either
    /// proves the handler ran. Dropping the merge above makes it a 404.
    #[tokio::test]
    async fn login_port_router_answers_the_launcher_summary_route() {
        let addr = serve().await;

        let summary = status(addr, "POST", "/api/telemetry/launcher-summary").await;
        assert!(
            summary == 401 || summary == 503,
            "launcher-summary not mounted: {summary}"
        );
        // The route takes POST only, and nothing else was mounted beside it.
        assert_eq!(
            status(addr, "GET", "/api/telemetry/launcher-summary").await,
            405
        );
        assert_eq!(
            status(addr, "POST", "/api/telemetry/launcher-summaries").await,
            404
        );
    }

    /// Everything a subscriber is shown, as `name=value` lines: each span's
    /// name, target and fields (at creation and when recorded later) and
    /// each event's fields. The capture layers elsewhere in this crate record
    /// event fields only, which is why they could not see what a request
    /// span holds.
    #[derive(Clone, Default)]
    struct Shown(Arc<Mutex<Vec<String>>>);

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
            event.record(&mut Lines(&mut self.0.lock().unwrap()));
        }
    }

    impl Shown {
        fn lines(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }
    }

    /// **No echo through the request span.** A query string on the summary
    /// route reaches no span field, no event field and no response byte.
    /// The request span is recorded with the path alone.
    ///
    /// Putting `DefaultMakeSpan` back fails the first log assertion: its
    /// `uri` field is the whole URI, marker included.
    #[tokio::test]
    async fn a_query_string_reaches_no_span_no_event_and_no_response() {
        const MARKER: &str = "ZZMARKER";
        const PATH: &str = "/api/telemetry/launcher-summary";

        let shown = Shown::default();
        // tracing caches each callsite's interest for the whole process.
        // While exactly one subscriber is alive it asks the current
        // thread's default, so under `cargo test` another login-port test
        // could reach the request span first, on its own subscriber-less
        // thread, and cache it as disabled. With a second subscriber alive
        // tracing asks every live subscriber instead.
        let _second = tracing::Dispatch::new(tracing_subscriber::registry());
        // `#[tokio::test]` runs the listener's tasks on this thread, so the
        // thread's default subscriber sees the handler side as well.
        let _default =
            tracing::subscriber::set_default(tracing_subscriber::registry().with(shown.clone()));

        let addr = serve().await;
        let response = exchange(addr, "POST", &format!("{PATH}?{MARKER}")).await;
        let lines = shown.lines();

        assert!(!response.contains(MARKER), "{response:?}");
        assert!(
            lines.iter().all(|line| !line.contains(MARKER)),
            "{lines:#?}"
        );
        // Not vacuous: the handler ran (401, or 503 while another test
        // holds the kill switch on), and the request span was recorded
        // with its three fields, the path among them, under the target
        // tower-http's own span has.
        let status_line = response.lines().next().unwrap_or_default();
        assert!(
            status_line.contains(" 401 ") || status_line.contains(" 503 "),
            "{response:?}"
        );
        for wanted in [
            "span=request target=tower_http::trace::make_span".to_string(),
            "method=POST".to_string(),
            format!("uri={PATH}"),
            "version=HTTP/1.1".to_string(),
        ] {
            assert!(lines.contains(&wanted), "no `{wanted}` in {lines:#?}");
        }

        // Control: the same recorder sees a marker placed in a span field,
        // whether the field is set at creation or recorded later.
        let span = tracing::info_span!("control", echo = MARKER, late = tracing::field::Empty);
        span.record("late", MARKER);
        let lines = shown.lines();
        for wanted in [format!("echo={MARKER}"), format!("late={MARKER}")] {
            assert!(lines.contains(&wanted), "no `{wanted}` in {lines:#?}");
        }
    }

    #[tokio::test]
    async fn login_port_router_does_not_expose_admin_routes() {
        let addr = serve().await;

        for (method, path) in [
            ("POST", "/api/auth/login"),
            ("POST", "/api/auth/logout"),
            ("GET", "/api/auth/me"),
            ("GET", "/api/players"),
            ("GET", "/api/config"),
            ("GET", "/api/entities"),
            ("GET", "/api/spaces"),
            ("GET", "/api/audit/logins"),
            ("GET", "/ws/logs"),
            ("GET", "/swagger-ui/"),
            ("GET", "/api-docs/openapi.json"),
        ] {
            assert_eq!(
                status(addr, method, path).await,
                404,
                "{method} {path} must not be served on the login port"
            );
        }
    }
}
