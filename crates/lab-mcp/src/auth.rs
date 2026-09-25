//! Bearer-token authentication for the lab MCP endpoint.
//!
//! A single shared token (from `CIMMERIA_LAB_MCP_TOKEN`) gates every request.
//! The check is a constant-time byte compare so a timing side-channel can't be
//! used to recover the token one byte at a time. This is an axum middleware
//! layered in front of the MCP tower service — auth is enforced at the HTTP
//! edge, before any MCP session or tool dispatch runs.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, State},
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::Next,
    response::Response,
};

tokio::task_local! {
    /// Peer address of the request currently being served, set by
    /// [`require_bearer`] and read by [`crate::audit::emit`] to fill the
    /// `caller` field of the per-tool-call audit event. Task-local because a
    /// tool method has no direct handle to the socket; the auth middleware and
    /// the tool dispatch run within the same request task for a POST tool call.
    pub static CALLER: String;
}

/// Constant-time equality for two byte slices.
///
/// The loop always runs over the full length and accumulates differences
/// rather than short-circuiting, so the time taken does not depend on *where*
/// the first mismatch is. The length is compared first — token length is not a
/// meaningful secret, and a fixed 32+ byte token makes this moot in practice.
#[must_use]
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Extract the token from an `Authorization: Bearer <token>` header value.
/// Returns `None` if the header is absent, non-UTF-8, or not a Bearer scheme.
fn extract_bearer(value: Option<&axum::http::HeaderValue>) -> Option<&str> {
    let raw = value?.to_str().ok()?;
    // Case-insensitive scheme, per RFC 7235; the token is the remainder.
    let rest = raw
        .strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))?;
    Some(rest.trim())
}

/// Axum middleware: reject any request whose bearer token doesn't match the
/// configured shared token. The token is held in an `Arc<str>` so cloning it
/// into the layer state is cheap.
pub async fn require_bearer(
    State(expected): State<Arc<str>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let presented = extract_bearer(request.headers().get(AUTHORIZATION));
    match presented {
        Some(tok) if constant_time_eq(tok.as_bytes(), expected.as_bytes()) => {
            // Bind the caller address for the duration of this request so the
            // per-tool-call audit event can name it.
            Ok(CALLER.scope(peer.to_string(), next.run(request)).await)
        }
        _ => {
            // Do not echo the presented token or say whether it was the length
            // or the value that failed — one opaque 401 for every failure.
            tracing::warn!(target: "lab.auth", "lab MCP request rejected: bad or missing bearer token");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn constant_time_eq_matches_and_mismatches() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(!constant_time_eq(b"", b"x"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn extract_bearer_parses_scheme() {
        assert_eq!(
            extract_bearer(Some(&HeaderValue::from_static("Bearer secret123"))),
            Some("secret123")
        );
        assert_eq!(
            extract_bearer(Some(&HeaderValue::from_static("bearer secret123"))),
            Some("secret123")
        );
        assert_eq!(
            extract_bearer(Some(&HeaderValue::from_static("Basic abcdef"))),
            None
        );
        assert_eq!(extract_bearer(None), None);
    }
}
