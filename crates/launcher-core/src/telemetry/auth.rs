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
    #[error("reply larger than {MAX_REPLY_BYTES} bytes")]
    ReplyTooLarge,
}

/// The most of a mint or refresh reply (a few hundred bytes) the launcher
/// reads; a bigger one is refused unread.
pub const MAX_REPLY_BYTES: usize = 16 * 1024;

/// `resp`'s body, read a network chunk at a time; `None` once it passes
/// `max` bytes, so a hostile or broken server cannot make the launcher
/// buffer an unbounded reply.
pub(super) async fn read_capped(
    mut resp: reqwest::Response,
    max: usize,
) -> Result<Option<Vec<u8>>, reqwest::Error> {
    let mut bytes = Vec::new();
    while let Some(chunk) = resp.chunk().await? {
        if bytes.len() + chunk.len() > max {
            return Ok(None);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(Some(bytes))
}

/// An error reply's body for [`AuthError::Status`] and its kin: at most
/// [`MAX_REPLY_BYTES`], empty when longer or unreadable.
pub(super) async fn error_body(resp: reqwest::Response) -> String {
    match read_capped(resp, MAX_REPLY_BYTES).await {
        Ok(Some(bytes)) => String::from_utf8_lossy(&bytes).into_owned(),
        _ => String::new(),
    }
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
    /// The game account the player logs in with (TG-NET-18, D-TG14). The
    /// server signs it into the token so world entry can find this
    /// session's client telemetry. Left out of the body when `None`, so the
    /// request is what it was before the field existed and an older server
    /// is not handed a key it does not know.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
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
        let bytes = read_capped(resp, MAX_REPLY_BYTES)
            .await?
            .ok_or(AuthError::ReplyTooLarge)?;
        return Ok(serde_json::from_slice::<DevSessionResponse>(&bytes)?);
    }
    if status.as_u16() == 503 {
        return Err(AuthError::KillSwitch {
            retry_after_secs: super::chunk::parse_retry_after(&resp),
        });
    }
    let body = error_body(resp).await;
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
            account_name: None,
        }
    }

    /// No account: the body has no `account_name` key, byte-for-byte the
    /// body an older launcher sent. An account goes out under that key,
    /// which the server's `DevSessionRequest` reads.
    #[test]
    fn account_name_is_sent_only_when_set() {
        let body = serde_json::to_value(fake_request()).unwrap();
        assert!(body.get("account_name").is_none(), "{body}");
        let body = serde_json::to_value(DevSessionRequest {
            account_name: Some("sgc_login".into()),
            ..fake_request()
        })
        .unwrap();
        assert_eq!(body["account_name"], "sgc_login");
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

    /// A refresh reply past [`MAX_REPLY_BYTES`] is refused, not buffered
    /// whole; an error body that long is kept to nothing.
    #[tokio::test]
    async fn an_oversized_reply_is_refused() {
        let server = MockServer::start().await;
        let mut big = fake_response_body("new-tok", 1);
        big["padding"] = serde_json::json!("x".repeat(MAX_REPLY_BYTES));
        Mock::given(method("POST"))
            .and(path("/auth/dev-session/refresh"))
            .respond_with(ResponseTemplate::new(200).set_body_json(big))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/auth/dev-session"))
            .respond_with(
                ResponseTemplate::new(500).set_body_string("y".repeat(MAX_REPLY_BYTES + 1)),
            )
            .mount(&server)
            .await;
        let http = reqwest::Client::new();
        let err = refresh_dev_session(&http, &server.uri(), "old")
            .await
            .unwrap_err();
        assert!(matches!(err, AuthError::ReplyTooLarge), "{err:?}");
        match fetch_dev_session(&http, &server.uri(), &fake_request()).await {
            Err(AuthError::Status { status: 500, body }) => assert!(body.is_empty()),
            other => panic!("expected an empty Status body, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_503_retry_after_is_clamped_to_an_hour() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/auth/dev-session"))
            .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "86400000"))
            .mount(&server)
            .await;
        let err = fetch_dev_session(&reqwest::Client::new(), &server.uri(), &fake_request())
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                AuthError::KillSwitch {
                    retry_after_secs: 3_600
                }
            ),
            "{err:?}"
        );
    }
}
