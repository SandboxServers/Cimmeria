//! Production HTTP sender + Discord `Retry-After` header parsing.

use std::time::Duration;

use serde_json::Value;

use super::handle::{DiscordSender, SendError};
use super::REQUEST_TIMEOUT;

/// Production HTTP sender. Owns a `reqwest::Client` configured with a
/// 10 s per-request timeout. Honours Discord's `Retry-After` 429 header
/// by sleeping in-task before returning the 429.
pub struct HttpDiscordSender {
    client: reqwest::Client,
}

impl HttpDiscordSender {
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(Duration::from_secs(5))
            .build()?;
        Ok(Self { client })
    }
}

impl Default for HttpDiscordSender {
    fn default() -> Self {
        Self::new().expect("reqwest client construction cannot fail with default config")
    }
}

#[async_trait::async_trait]
impl DiscordSender for HttpDiscordSender {
    async fn send(&self, url: &str, body: &Value) -> Result<(), SendError> {
        let resp = self
            .client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| SendError::Network(e.to_string()))?;
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        // 429: honour Retry-After in-task. We sleep here so the sender
        // loop above sees it as a single retryable error rather than
        // racing other channels' buckets.
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = parse_retry_after(&resp).unwrap_or(Duration::from_secs(1));
            tracing::debug!(
                target: "cimmeria_discord",
                wait_ms = retry_after.as_millis() as u64,
                "Discord 429 — honouring Retry-After"
            );
            tokio::time::sleep(retry_after).await;
            return Err(SendError::HttpStatus(429, "rate limited".to_string()));
        }
        let body_text = resp
            .text()
            .await
            .unwrap_or_else(|_| "(no body)".to_string());
        Err(SendError::HttpStatus(status.as_u16(), body_text))
    }
}

fn parse_retry_after(resp: &reqwest::Response) -> Option<Duration> {
    let s = resp.headers().get("Retry-After")?.to_str().ok()?;
    parse_retry_after_str(s)
}

/// String-input slice of [`parse_retry_after`] — exposed so unit tests
/// can pin each header-format branch without constructing a full
/// `reqwest::Response`. Discord sends fractional seconds (e.g.,
/// `"0.345"`) but the integer form is the RFC 7231 default, so we try
/// both. Negative values clamp to zero (defensive — RFC says ≥ 0 but
/// a hostile server could lie); huge values clamp to 60 s.
fn parse_retry_after_str(s: &str) -> Option<Duration> {
    if let Ok(secs) = s.parse::<f64>() {
        if !secs.is_finite() {
            return None;
        }
        let cap = secs.clamp(0.0, 60.0);
        return Some(Duration::from_secs_f64(cap));
    }
    if let Ok(secs) = s.parse::<u64>() {
        return Some(Duration::from_secs(secs.min(60)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── HttpDiscordSender (wiremock) integration tests ─────────────────
    //
    // The MockSender path in `task.rs` tests the retry / fan-out logic in
    // isolation. These tests exercise the *real* HttpDiscordSender —
    // reqwest client, JSON serialisation, status handling, Retry-After
    // parsing — against a wiremock HTTP server. Catches reqwest API
    // drift and 429/header-parsing regressions that the in-memory
    // mock can't see.

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// 204 No Content → `Ok(())`. Discord's documented success code
    /// for webhook posts.
    #[tokio::test]
    async fn http_sender_success_204_returns_ok() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/webhook"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let sender = HttpDiscordSender::default();
        let url = format!("{}/webhook", server.uri());
        let body = serde_json::json!({ "content": "hi" });

        sender.send(&url, &body).await.expect("204 → Ok");
        let received = server.received_requests().await.unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].method.as_str(), "POST");
    }

    /// 400 Bad Request → `Err(HttpStatus(400, body))`. The retry layer
    /// uses this code to short-circuit (no point retrying a malformed
    /// payload).
    #[tokio::test]
    async fn http_sender_4xx_returns_status_with_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(400).set_body_string("invalid embed"))
            .mount(&server)
            .await;

        let sender = HttpDiscordSender::default();
        let body = serde_json::json!({});
        let err = sender
            .send(&server.uri(), &body)
            .await
            .expect_err("4xx → Err");
        match err {
            SendError::HttpStatus(code, msg) => {
                assert_eq!(code, 400);
                assert!(
                    msg.contains("invalid embed"),
                    "body should be passed through, got {msg}"
                );
            }
            other => panic!("expected HttpStatus, got {other:?}"),
        }
    }

    /// 429 with a fractional-seconds Retry-After: parse it, sleep, and
    /// return `Err(HttpStatus(429, _))`. The slow-path (integer +
    /// clamp + missing) Retry-After variants are pinned directly via
    /// `parse_retry_after_str` below so we don't pay multiple
    /// real-time seconds per test.
    ///
    /// Real-time wait (no `start_paused`): wiremock uses real I/O so
    /// pausing tokio's time advances breaks the request/response
    /// loop. 123ms is short enough not to slow CI noticeably while
    /// still proving the header was honoured.
    #[tokio::test]
    async fn http_sender_429_float_retry_after_honoured() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("Retry-After", "0.123")
                    .set_body_string("rate limited"),
            )
            .mount(&server)
            .await;

        let sender = HttpDiscordSender::default();
        let body = serde_json::json!({});
        let start = std::time::Instant::now();
        let err = sender
            .send(&server.uri(), &body)
            .await
            .expect_err("429 → Err");
        let elapsed = start.elapsed();

        assert!(matches!(err, SendError::HttpStatus(429, _)), "got {err:?}");
        assert!(
            elapsed >= Duration::from_millis(100),
            "expected ≥100ms sleep, got {elapsed:?}"
        );
    }

    /// `parse_retry_after_str` covers the header-parsing branches that
    /// the wiremock test above only exercises one of. Direct unit
    /// tests so each variant is paid for in microseconds, not seconds.
    #[test]
    fn parse_retry_after_str_fractional_seconds() {
        let d = parse_retry_after_str("0.500").unwrap();
        assert_eq!(d, Duration::from_millis(500));
    }

    #[test]
    fn parse_retry_after_str_integer_seconds() {
        let d = parse_retry_after_str("3").unwrap();
        assert_eq!(d, Duration::from_secs(3));
    }

    #[test]
    fn parse_retry_after_str_clamps_to_60s_ceiling() {
        // A malicious / misbehaving server can't pin the task for
        // hours by returning a huge Retry-After. parse_retry_after
        // clamps to 60 s.
        let d_float = parse_retry_after_str("999999.5").unwrap();
        assert_eq!(d_float, Duration::from_secs(60));
        let d_int = parse_retry_after_str("999999").unwrap();
        assert_eq!(d_int, Duration::from_secs(60));
    }

    #[test]
    fn parse_retry_after_str_negative_clamps_to_zero() {
        // RFC 7231 says Retry-After is non-negative; defensively
        // clamp anyway so a negative float can't produce a NaN /
        // panic in Duration::from_secs_f64.
        let d = parse_retry_after_str("-5").unwrap();
        assert_eq!(d, Duration::ZERO);
    }

    #[test]
    fn parse_retry_after_str_garbage_returns_none() {
        assert!(parse_retry_after_str("not-a-number").is_none());
        assert!(parse_retry_after_str("").is_none());
    }

    /// Connection refused / DNS / TLS errors surface as `Network(...)`,
    /// which IS retryable in the layer above. Tested here with a closed
    /// port that nothing is listening on.
    #[tokio::test]
    async fn http_sender_network_error_returns_network_variant() {
        let sender = HttpDiscordSender::default();
        // Reserved-for-documentation address (RFC 5737) — guaranteed
        // not to route. Bypasses any local proxy that might absorb a
        // 127.0.0.1:closed-port probe.
        let body = serde_json::json!({});
        let err = sender
            .send("http://192.0.2.1:1/webhook", &body)
            .await
            .expect_err("unreachable host → Err");
        assert!(
            matches!(err, SendError::Network(_)),
            "expected Network variant, got {err:?}"
        );
    }
}
