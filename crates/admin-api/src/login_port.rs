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
//! the desktop launcher's attempt summaries. Unlike the other four it is
//! anonymous: it takes no token, and never reads an `Authorization`
//! header. Anyone who can reach this port can post correctly shaped rows
//! within the rate limit (12 requests a minute per address by default,
//! `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP`), and anything that is not the
//! exact schema-1 JSON payload is refused. The rows are self-reported:
//! they are useful for spotting failure patterns and must never drive
//! server state, alerts, success-rate claims or SLOs. The strict schema
//! means nothing but closed enum values, bounded integers, UUIDs and a
//! version triple can ever be stored.
//!
//! The owner approved serving that route on this port on 2026-10-04,
//! which extends the four-route decision of 2026-09-29 to it as a fifth,
//! and set its rate limit at 12 requests a minute per address the same
//! day. No shipped launcher calls it: the exporter's endpoint is unset,
//! and it accepts only `https://` (or `http://` to loopback), so this
//! plain-HTTP port cannot be its endpoint.
//!
//! Only these routes are exposed. Everything else under `/api`
//! (players, config, entities, the admin `/api/auth/login`) and the `/ws`
//! and Swagger surfaces stay on the admin listener only.

use axum::Router;
use tower_http::trace::{DefaultOnFailure, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::request_span::request_span;
use crate::routes::{dev_session, telemetry};

/// Router with exactly `/api/auth/dev-session`,
/// `/api/auth/dev-session/refresh`, `/api/telemetry/upload-chunk`,
/// `/api/telemetry/upload-bundle` and `/api/telemetry/launcher-summary`,
/// for the auth service to merge into its login router.
///
/// The body limits of the first four routes come with
/// [`dev_session::routes`] and [`telemetry::routes`], as layers. The
/// summary route has no such layer: its handler takes the request unread
/// and enforces its own 64 KiB cap while it reads the body, after the kill
/// switch and the quota (see [`telemetry::launcher_summary_routes`]).
///
/// The first four need a dev-session token; the summary route is anonymous.
/// The mint, refresh and summary quotas read the peer
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
        // The same per-request span as the admin router: the path
        // without the query string. See `request_span`.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(request_span)
                .on_response(DefaultOnResponse::new().level(Level::INFO))
                .on_failure(DefaultOnFailure::new().level(Level::WARN)),
        )
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tracing_subscriber::layer::SubscriberExt;

    use super::login_port_telemetry_router;
    use crate::request_span::recorder::Shown;

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
        exchange_with(addr, method, path, "").await
    }

    /// [`exchange`] with extra header lines, each ending in `\r\n`.
    async fn exchange_with(addr: SocketAddr, method: &str, path: &str, headers: &str) -> String {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {addr}\r\n{headers}Content-Length: 0\r\nConnection: close\r\n\r\n"
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

    /// True if `response` is the summary handler's own answer to a
    /// bodyless request with no `Content-Type`: its 415 with its static
    /// body. A 503 (another test in this process holds the kill switch on)
    /// or a 429 (the tests in this process have used up loopback's
    /// allowance) also comes only from that handler.
    fn summary_handler_ran(response: &str) -> bool {
        let status_line = response.lines().next().unwrap_or_default();
        (status_line.contains(" 415 ")
            && response.ends_with("Content-Type must be application/json"))
            || status_line.contains(" 503 ")
            || status_line.contains(" 429 ")
    }

    /// The launcher-summary ingest is the fifth route, and the only
    /// anonymous one. Its handler checks the kill switch, the quota and
    /// then the content type, so a bodyless POST with no token and no
    /// `Content-Type` gets the handler's own 415, not a 401. Dropping the
    /// merge above makes it a 404.
    #[tokio::test]
    async fn login_port_router_answers_the_launcher_summary_route() {
        let addr = serve().await;

        let summary = exchange(addr, "POST", "/api/telemetry/launcher-summary").await;
        assert!(
            summary_handler_ran(&summary),
            "launcher-summary not mounted: {summary:?}"
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

    /// **No echo through the request span.** A query string on the summary
    /// route, an `Authorization` header the anonymous route never reads,
    /// and the host of an absolute-form request target
    /// (`POST http://zzhost.example/api/…`) reach no span field, no event
    /// field and no response byte. The request span is recorded with the
    /// path alone, and with no header.
    ///
    /// Putting `DefaultMakeSpan` back on this router fails the log
    /// assertions: its `uri` field is the whole request target, marker or
    /// host included. The admin router's copy of this test is
    /// `the_admin_request_span_holds_the_path_alone` in
    /// `routes/telemetry/launcher_summary/tests/routes.rs`.
    #[tokio::test]
    async fn a_query_string_reaches_no_span_no_event_and_no_response() {
        const MARKER: &str = "ZZMARKER";
        const HOST: &str = "zzhost";
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
        let response = exchange_with(
            addr,
            "POST",
            &format!("{PATH}?{MARKER}"),
            &format!("Authorization: Bearer {MARKER}\r\n"),
        )
        .await;
        let absolute = exchange(addr, "POST", &format!("http://{HOST}.example{PATH}")).await;
        let lines = shown.lines();

        for text in [MARKER, HOST] {
            assert!(!response.contains(text), "{text} in {response:?}");
            assert!(!absolute.contains(text), "{text} in {absolute:?}");
            assert!(
                lines.iter().all(|line| !line.contains(text)),
                "{text} in {lines:#?}"
            );
        }
        // Not vacuous: the handler ran both times (its 415 for the missing
        // content type; see `summary_handler_ran`), so the absolute-form
        // target was routed like the bare path, and each request's span
        // was recorded with its three fields, the path among them, under
        // the target tower-http's own span has.
        assert!(summary_handler_ran(&response), "{response:?}");
        assert!(summary_handler_ran(&absolute), "{absolute:?}");
        for wanted in [
            "span=request target=tower_http::trace::make_span".to_string(),
            "method=POST".to_string(),
            format!("uri={PATH}"),
            "version=HTTP/1.1".to_string(),
        ] {
            let count = lines.iter().filter(|line| **line == wanted).count();
            assert_eq!(count, 2, "`{wanted}` in {lines:#?}");
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
