//! The HTTP half of the uploader: the ureq agent, the gzip + NDJSON body,
//! one POST and its error. The loop that decides when to POST is
//! [`super`]; the retry decision is [`crate::upload_gate`].

use std::io::Write;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use flate2::write::GzEncoder;
use flate2::Compression;

use crate::events::{ClientNativeEvent, TelemetryEvent};
use crate::queue::Consumer;

/// Build the ureq agent we use for every POST. Single instance per
/// thread — ureq keeps the underlying TCP connection alive across
/// calls, which matters for the rapid-flush case where we ship
/// every 2 s.
pub(super) fn build_agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .timeout_write(Duration::from_secs(30))
        .build()
}

/// Header carrying this process's id on every chunk. One lab session
/// spans several launches, so the server needs the pid to tell them apart.
pub(super) const CLIENT_PID_HEADER: &str = "X-Cimmeria-Client-Pid";

/// The DLL's wall clock when this POST was built, unix ms, on the same clock
/// as each row's `ts_ms`. admin-api measures the clock skew from it.
pub(super) const CLIENT_SENT_MS_HEADER: &str = "X-Cimmeria-Client-Sent-Ms";

/// Serialize as NDJSON, gzip, POST with the Bearer `token` to `endpoint`,
/// with an optional whole-request time limit (the final flush at exit
/// passes one; the ordinary cadence does not). Returns the HTTP status on a
/// 2xx answer.
///
/// Returns `Err` on any network or non-2xx HTTP response. The
/// caller (the uploader loop) retains the batch in memory on
/// failure and re-attempts when the gate allows, so the
/// delivery shape is at-least-once *with possible duplicates* if
/// the server actually processed the batch before its response
/// failed (since we cannot distinguish that case from a true POST
/// failure). The server does **not** absorb those duplicates:
/// `/api/telemetry/upload-chunk` replays every row it parses and keeps
/// no `(session_id, seq)` set (see `admin-api` `routes/telemetry/mod.rs`),
/// so a retried batch the server had already taken shows up twice in
/// SigNoz, with the same `seq`.
pub(super) fn post_batch_within(
    client: &ureq::Agent,
    endpoint: &str,
    token: &str,
    batch: &[ClientNativeEvent],
    limit: Option<Duration>,
) -> Result<u16, PostError> {
    let body = encode_batch(batch)?;
    // Stamped per attempt: a retry calls this again and carries a fresh clock.
    let sent_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let mut req = client
        .post(endpoint)
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/x-ndjson")
        .set("Content-Encoding", "gzip")
        .set(CLIENT_PID_HEADER, &std::process::id().to_string())
        .set(CLIENT_SENT_MS_HEADER, &sent_ms.to_string());
    if let Some(limit) = limit {
        req = req.timeout(limit);
    }
    // ureq 2 reports a 4xx/5xx answer as `Error::Status`, not as a
    // response; keep its status (and any `Retry-After`) so the health row
    // can say what the server answered and the gate how long to wait.
    let resp = match req.send_bytes(&body) {
        Ok(resp) => resp,
        Err(ureq::Error::Status(status, resp)) => {
            let retry_after = resp.header("Retry-After").and_then(parse_retry_after);
            return Err(PostError::Http {
                status,
                retry_after,
                body: resp.into_string().unwrap_or_default(),
            });
        }
        Err(e) => return Err(PostError::Network(e.to_string())),
    };
    let status = resp.status();
    if !(200..300).contains(&status) {
        let retry_after = resp.header("Retry-After").and_then(parse_retry_after);
        return Err(PostError::Http {
            status,
            retry_after,
            body: resp.into_string().unwrap_or_default(),
        });
    }
    Ok(status)
}

/// A `Retry-After` value in whole seconds. The HTTP-date form is not worth
/// a date parser here: it is ignored, and the backoff applies.
pub(super) fn parse_retry_after(v: &str) -> Option<Duration> {
    v.trim().parse::<u64>().ok().map(Duration::from_secs)
}

/// Count a POST's outcome on the consumer (TG-PIPE-13): a failure bumps
/// `upload_failures_total`, and every answer sets `last_upload_status`.
pub(super) fn record(consumer: &Consumer, result: Result<u16, PostError>) -> Result<(), PostError> {
    match result {
        Ok(status) => {
            consumer.record_upload_ok(status);
            Ok(())
        }
        Err(e) => {
            consumer.record_upload_failure(e.status());
            Err(e)
        }
    }
}

/// NDJSON encode → gzip. Each event becomes one
/// `TelemetryEvent::ClientNative(...)` JSON object on its own line.
pub(super) fn encode_batch(batch: &[ClientNativeEvent]) -> Result<Vec<u8>, PostError> {
    let mut gz = GzEncoder::new(
        Vec::with_capacity(batch.len() * 200),
        Compression::default(),
    );
    for ev in batch {
        let wrapped = TelemetryEvent::ClientNative(ev.clone());
        serde_json::to_writer(&mut gz, &wrapped).map_err(PostError::Serialize)?;
        gz.write_all(b"\n").map_err(PostError::Io)?;
    }
    gz.finish().map_err(PostError::Io)
}

#[derive(Debug, thiserror::Error)]
pub(super) enum PostError {
    #[error("network: {0}")]
    Network(String),
    #[error("http {status}: {body}")]
    Http {
        status: u16,
        /// The answer's `Retry-After`, in whole seconds, when it had one.
        retry_after: Option<Duration>,
        body: String,
    },
    #[error("serialize: {0}")]
    Serialize(#[source] serde_json::Error),
    #[error("io: {0}")]
    Io(#[source] std::io::Error),
}

impl PostError {
    /// The server's HTTP status, when it answered at all.
    pub(super) fn status(&self) -> Option<u16> {
        match self {
            PostError::Http { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// The status when the server refused the credentials (401, 403): the
    /// cases a re-read of the session file can fix.
    pub(super) fn auth_status(&self) -> Option<u16> {
        self.status().filter(|s| matches!(s, 401 | 403))
    }

    /// How long the server asked us to wait, if it did.
    pub(super) fn retry_after(&self) -> Option<Duration> {
        match self {
            PostError::Http { retry_after, .. } => *retry_after,
            _ => None,
        }
    }
}
