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
//! Only these four routes are exposed. Everything else under `/api`
//! (players, config, entities, the admin `/api/auth/login`) and the `/ws`
//! and Swagger surfaces stay on the admin listener only.

use axum::Router;
use tower_http::trace::{DefaultMakeSpan, DefaultOnFailure, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::routes::{dev_session, telemetry};

/// Router with exactly `/api/auth/dev-session`,
/// `/api/auth/dev-session/refresh`, `/api/telemetry/upload-chunk` and
/// `/api/telemetry/upload-bundle`, for the auth service to merge into its
/// login router.
///
/// The per-route body limits come with [`dev_session::routes`] and
/// [`telemetry::routes`]. The mint and refresh quotas read the peer
/// address, so the listener must serve with
/// `into_make_service_with_connect_info::<SocketAddr>()`, which both auth
/// listeners already do.
pub fn login_port_telemetry_router() -> Router {
    Router::new()
        .nest(
            "/api",
            Router::new()
                .nest("/auth", dev_session::routes())
                .nest("/telemetry", telemetry::routes()),
        )
        // Same per-request span as the admin router, so an upload through
        // the login port looks the same in SigNoz as one through 8443.
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO))
                .on_failure(DefaultOnFailure::new().level(Level::WARN)),
        )
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

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

    /// Send a bodyless request over raw HTTP/1.1 and return the status
    /// code. Raw TCP keeps the test free of an HTTP-client dependency.
    async fn status(addr: SocketAddr, method: &str, path: &str) -> u16 {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        let response = String::from_utf8_lossy(&response);
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
