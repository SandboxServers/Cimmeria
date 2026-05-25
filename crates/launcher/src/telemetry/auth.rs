//! Launcher → cimmeria-server `/auth/dev-session` client.
//!
//! One call mints a session token + the upload endpoint and chunking
//! defaults; the launcher uses the returned `(token, upload_endpoint)`
//! pair for the rest of the session. A 401 from any subsequent chunk
//! POST triggers `refresh` to extend the token without re-handshaking
//! identity.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Server returned {status}: {body}")]
    Status { status: u16, body: String },
    #[error("Kill switch active (503) — retry after {retry_after_secs}s")]
    KillSwitch { retry_after_secs: u64 },
}

/// Request body for `POST /api/auth/dev-session`. Matches the
/// server-side `DevSessionRequest` shape; any field rename here is a
/// wire-format breaking change.
#[derive(Debug, Clone, Serialize)]
pub struct DevSessionRequest {
    pub install_id: String,
    pub machine_id: String,
    pub branch: String,
    pub git_sha: String,
    pub launcher_version: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DevSessionResponse {
    pub session_id: String,
    pub token: String,
    pub expires_at_ms: i64,
    pub upload_endpoint: String,
    pub chunk_max_bytes: u64,
    pub flush_interval_ms: u64,
}

/// Threshold (% of TTL) past which the launcher proactively refreshes
/// rather than waiting for a 401 from the chunk endpoint. 75% leaves
/// a comfortable window for the refresh round-trip before any client
/// would see an expired token.
pub const REFRESH_AT_PERCENT_REMAINING: u64 = 25;

pub async fn fetch_dev_session(
    http: &reqwest::Client,
    auth_base_url: &str,
    req: &DevSessionRequest,
) -> Result<DevSessionResponse, AuthError> {
    let url = format!("{}/auth/dev-session", auth_base_url.trim_end_matches('/'));
    send_and_parse(http.post(&url).json(req)).await
}

pub async fn refresh_dev_session(
    http: &reqwest::Client,
    auth_base_url: &str,
    token: &str,
) -> Result<DevSessionResponse, AuthError> {
    let url = format!(
        "{}/auth/dev-session/refresh",
        auth_base_url.trim_end_matches('/')
    );
    let body = serde_json::json!({ "token": token });
    send_and_parse(http.post(&url).json(&body)).await
}

/// True when the token's remaining lifetime is below the proactive-
/// refresh threshold. Pure function exposed for the orchestrator's
/// "should I refresh now?" decision so the policy is unit-testable
/// without spinning up an HTTP mock.
pub fn should_refresh(expires_at_ms: i64, now_ms: i64, issued_at_ms: i64) -> bool {
    let total = expires_at_ms.saturating_sub(issued_at_ms).max(1);
    let remaining = expires_at_ms.saturating_sub(now_ms);
    let pct = (remaining * 100 / total).max(0);
    (pct as u64) <= REFRESH_AT_PERCENT_REMAINING
}

async fn send_and_parse(req: reqwest::RequestBuilder) -> Result<DevSessionResponse, AuthError> {
    let resp = req.send().await?;
    let status = resp.status();
    if status.is_success() {
        return Ok(resp.json::<DevSessionResponse>().await?);
    }
    if status.as_u16() == 503 {
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60);
        return Err(AuthError::KillSwitch {
            retry_after_secs: retry_after,
        });
    }
    let body = resp.text().await.unwrap_or_default();
    Err(AuthError::Status {
        status: status.as_u16(),
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn fake_request() -> DevSessionRequest {
        DevSessionRequest {
            install_id: "i".into(),
            machine_id: "m".into(),
            branch: "main".into(),
            git_sha: "abc".into(),
            launcher_version: "0.1.0".into(),
            tags: vec![],
        }
    }

    fn fake_response_body(token: &str, exp_ms: i64) -> serde_json::Value {
        serde_json::json!({
            "session_id": "sid-1",
            "token": token,
            "expires_at_ms": exp_ms,
            "upload_endpoint": "https://x.example/api",
            "chunk_max_bytes": 1_048_576,
            "flush_interval_ms": 2_000,
        })
    }

    // should_refresh policy: at issuance, plenty of remaining → false.
    // Past 75% of TTL → true. Beyond expiry → true.
    #[test]
    fn should_refresh_below_threshold_returns_true() {
        let issued = 0i64;
        let exp = 8 * 60 * 60 * 1000;
        // 80% elapsed → 20% remaining → refresh
        assert!(should_refresh(exp, issued + (exp * 80 / 100), issued));
        // Fully expired → refresh
        assert!(should_refresh(exp, exp + 1, issued));
    }

    #[test]
    fn should_refresh_above_threshold_returns_false() {
        let issued = 0i64;
        let exp = 8 * 60 * 60 * 1000;
        // 50% elapsed → 50% remaining → no refresh
        assert!(!should_refresh(exp, issued + (exp * 50 / 100), issued));
        // Just issued → no refresh
        assert!(!should_refresh(exp, issued, issued));
    }

    #[tokio::test]
    async fn fetch_dev_session_happy_path() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/auth/dev-session"))
            .respond_with(ResponseTemplate::new(200).set_body_json(fake_response_body("tok", 1)))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let resp = fetch_dev_session(&http, &server.uri(), &fake_request())
            .await
            .unwrap();
        assert_eq!(resp.token, "tok");
        assert_eq!(resp.session_id, "sid-1");
        assert_eq!(resp.upload_endpoint, "https://x.example/api");
    }

    #[tokio::test]
    async fn refresh_dev_session_posts_existing_token() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/auth/dev-session/refresh"))
            .and(body_json(serde_json::json!({"token": "old"})))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(fake_response_body("new-tok", 2)),
            )
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let resp = refresh_dev_session(&http, &server.uri(), "old")
            .await
            .unwrap();
        assert_eq!(resp.token, "new-tok");
    }

    // Kill switch (503 with Retry-After) surfaces with the parsed
    // retry value so the orchestrator can honor it.
    #[tokio::test]
    async fn fetch_dev_session_503_surfaces_kill_switch() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/auth/dev-session"))
            .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "180"))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let err = fetch_dev_session(&http, &server.uri(), &fake_request())
            .await
            .unwrap_err();
        match err {
            AuthError::KillSwitch { retry_after_secs } => assert_eq!(retry_after_secs, 180),
            other => panic!("expected KillSwitch, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn fetch_dev_session_5xx_other_surfaces_status() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/auth/dev-session"))
            .respond_with(ResponseTemplate::new(500).set_body_string("oops"))
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let err = fetch_dev_session(&http, &server.uri(), &fake_request())
            .await
            .unwrap_err();
        match err {
            AuthError::Status { status, .. } => assert_eq!(status, 500),
            other => panic!("expected Status, got {other:?}"),
        }
    }
}
