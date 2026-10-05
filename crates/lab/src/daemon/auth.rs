//! Bearer-token gate for the daemon, in front of the MCP service: a request
//! without `Authorization: Bearer <CIMMERIA_LAB_DAEMON_TOKEN>` never reaches
//! an MCP session. Same shape as `crates/lab-mcp/src/auth.rs` (constant-time
//! compare, one opaque 401 for every failure).

use std::sync::Arc;

use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::Response,
};

/// Constant-time equality: the loop always covers the full length, so the
/// time taken does not reveal where the first mismatch is.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

/// The token in an `Authorization: Bearer <token>` header, if any.
fn extract_bearer(value: Option<&HeaderValue>) -> Option<&str> {
    let raw = value?.to_str().ok()?;
    let rest = raw
        .strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))?;
    Some(rest.trim())
}

/// Axum middleware: 401 unless the bearer token matches.
pub async fn require_bearer(
    State(expected): State<Arc<str>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    match extract_bearer(request.headers().get(AUTHORIZATION)) {
        Some(t) if constant_time_eq(t.as_bytes(), expected.as_bytes()) => {
            Ok(next.run(request).await)
        }
        _ => {
            tracing::warn!(target: "lab.auth", "lab daemon request rejected: bad or missing bearer token");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_matches_and_mismatches() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }

    #[test]
    fn extract_bearer_parses_scheme() {
        let h = HeaderValue::from_static("Bearer s3cret");
        assert_eq!(extract_bearer(Some(&h)), Some("s3cret"));
        let h = HeaderValue::from_static("Basic s3cret");
        assert_eq!(extract_bearer(Some(&h)), None);
        assert_eq!(extract_bearer(None), None);
    }
}
