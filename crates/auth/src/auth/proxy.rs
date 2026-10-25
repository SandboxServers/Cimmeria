//! Reverse-proxy listener middleware and the `/auth/v1` HTTPS gate.
//!
//! Trust comes from the listener, never from a header. [`proxy_layer`] is
//! layered only on the `AUTH_PROXY_ADDR` listener, where a TLS-terminating
//! reverse proxy is the only peer. The plain and in-process TLS listeners never
//! run it, so their `X-Forwarded-*` headers mean nothing.

use std::net::{IpAddr, SocketAddr};

use axum::extract::{ConnectInfo, Request};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::TlsConn;

const X_FORWARDED_PROTO: &str = "x-forwarded-proto";
const X_FORWARDED_FOR: &str = "x-forwarded-for";

/// Middleware for the proxy listener. A request must say it arrived over
/// HTTPS (`X-Forwarded-Proto: https`) and carry a client address in
/// `X-Forwarded-For`; otherwise it gets an empty `400`. A valid request has
/// its `ConnectInfo` replaced by the rightmost forwarded address (port 0), so
/// handlers see the same extensions as on the other listeners, and gets the
/// [`TlsConn`] marker.
///
/// The rightmost entry is the one the proxy itself appended. The leftmost is
/// whatever the client sent, so it is never trusted.
pub(super) async fn proxy_layer(mut req: Request, next: Next) -> Response {
    let https = req
        .headers()
        .get(X_FORWARDED_PROTO)
        .and_then(|v| v.to_str().ok())
        == Some("https");
    if !https {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let Some(ip) = rightmost_forwarded_for(req.headers()) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::new(ip, 0)));
    req.extensions_mut().insert(TlsConn);
    next.run(req).await
}

/// The rightmost `X-Forwarded-For` entry, trimmed, when it parses as an IP
/// address (IPv4 or IPv6). A missing header or a garbage entry is `None`.
fn rightmost_forwarded_for(headers: &HeaderMap) -> Option<IpAddr> {
    let value = headers
        .get_all(X_FORWARDED_FOR)
        .iter()
        .next_back()?
        .to_str()
        .ok()?;
    value.rsplit(',').next()?.trim().parse().ok()
}

/// Middleware for the `/auth/v1` routes on every listener. A request without
/// the [`TlsConn`] marker (it did not arrive over TLS) gets `403` with the
/// JSON body `{"error":"https_required"}`.
pub(super) async fn require_tls(req: Request, next: Next) -> Response {
    if req.extensions().get::<TlsConn>().is_some() {
        return next.run(req).await;
    }
    (
        StatusCode::FORBIDDEN,
        [(header::CONTENT_TYPE, "application/json")],
        r#"{"error":"https_required"}"#,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::middleware::from_fn;
    use axum::routing::get;
    use axum::{Extension, Router};
    use tower::ServiceExt; // oneshot

    /// Echoes the peer address and whether the TLS marker is present.
    async fn echo(
        ConnectInfo(peer): ConnectInfo<SocketAddr>,
        tls: Option<Extension<TlsConn>>,
    ) -> String {
        format!("{}|{}", peer.ip(), tls.is_some())
    }

    fn app() -> Router {
        Router::new()
            .route("/echo", get(echo))
            .layer(from_fn(proxy_layer))
    }

    async fn send(headers: &[(&str, &str)]) -> (StatusCode, String) {
        let mut builder = axum::http::Request::builder().uri("/echo");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let response = app()
            .oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    /// Without `X-Forwarded-Proto: https` the request is refused with an
    /// empty 400, whether the header is missing or says `http`. Reverting
    /// the proto check lets both through.
    #[tokio::test]
    async fn proxy_requires_https_proto() {
        let fwd = ("x-forwarded-for", "203.0.113.9");
        let (status, body) = send(&[fwd]).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.is_empty());

        let (status, _) = send(&[fwd, ("x-forwarded-proto", "http")]).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        let (status, _) = send(&[fwd, ("x-forwarded-proto", "https")]).await;
        assert_eq!(status, StatusCode::OK);
    }

    /// The client address is the rightmost `X-Forwarded-For` entry, so a
    /// spoofed leftmost entry is ignored. The request also carries `TlsConn`.
    /// Reverting to the leftmost entry fails the address assertion.
    #[tokio::test]
    async fn proxy_takes_the_rightmost_forwarded_for() {
        let (status, body) = send(&[
            ("x-forwarded-proto", "https"),
            ("x-forwarded-for", "203.0.113.9, 198.51.100.7"),
        ])
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "198.51.100.7|true");

        // IPv6 is accepted too.
        let (_, body) = send(&[
            ("x-forwarded-proto", "https"),
            ("x-forwarded-for", "203.0.113.9,  2001:db8::7 "),
        ])
        .await;
        assert_eq!(body, "2001:db8::7|true");
    }

    /// A missing or unparsable `X-Forwarded-For` (here `nonsense`, and an
    /// empty trailing entry) gets an empty 400, never a fallback address.
    #[tokio::test]
    async fn proxy_refuses_a_bad_forwarded_for() {
        let proto = ("x-forwarded-proto", "https");
        let (status, body) = send(&[proto]).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.is_empty());

        let (status, body) = send(&[proto, ("x-forwarded-for", "nonsense")]).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.is_empty());

        let (status, _) = send(&[proto, ("x-forwarded-for", "203.0.113.9, ")]).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    /// `require_tls` answers a request without the `TlsConn` marker with 403
    /// and the `https_required` JSON body, and passes a marked one through.
    /// Reverting the gate lets the plain request reach the handler.
    #[tokio::test]
    async fn require_tls_refuses_plain() {
        let app = Router::new()
            .route("/ping", get(|| async { "pong" }))
            .layer(from_fn(require_tls));

        let plain = axum::http::Request::builder()
            .uri("/ping")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(plain).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let bytes = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        assert_eq!(&bytes[..], br#"{"error":"https_required"}"#);

        let mut tls = axum::http::Request::builder()
            .uri("/ping")
            .body(Body::empty())
            .unwrap();
        tls.extensions_mut().insert(TlsConn);
        let response = app.oneshot(tls).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
