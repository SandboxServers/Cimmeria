//! Cosmos DB log sink.
//!
//! A tracing layer that sends filtered log entries to an Azure Cosmos DB
//! `server-logs` container for queryable, time-limited storage. Documents
//! auto-expire via the container's TTL setting (24 hours).
//!
//! # Architecture
//!
//! [`CosmosLogLayer`] captures tracing events synchronously and sends them
//! through an unbounded mpsc channel. A background tokio task
//! ([`cosmos_log_writer`]) reads from the channel and POSTs documents to
//! the Cosmos DB REST API using master-key HMAC-SHA256 auth.
//!
//! # Environment variables
//!
//! | Variable | Description |
//! |---|---|
//! | `COSMOS_LOG_ENDPOINT` | e.g. `https://cimmeria-cosmos.documents.azure.com` |
//! | `COSMOS_LOG_KEY` | Cosmos DB master key (base64) |
//!
//! If either is missing the layer is simply not created.

use std::fmt::Write as FmtWrite;
use std::time::SystemTime;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use tokio::sync::mpsc;
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

type HmacSha256 = Hmac<Sha256>;

// ── Document ─────────────────────────────────────────────────────────────

/// A single log document written to Cosmos DB.
#[derive(serde::Serialize)]
struct LogDoc {
    id: String,
    session_id: String,
    timestamp_ms: u64,
    level: String,
    target: String,
    message: String,
    fields: serde_json::Value,
    /// Cosmos DB per-document TTL in seconds.
    ttl: i32,
}

// ── Layer ────────────────────────────────────────────────────────────────

/// Tracing layer that captures events and forwards them to Cosmos DB via
/// an mpsc channel.
pub struct CosmosLogLayer {
    session_id: String,
    tx: mpsc::UnboundedSender<LogDoc>,
    counter: std::sync::atomic::AtomicU64,
}

/// Opaque handle for the receiving end of the log channel.
pub struct CosmosLogReceiver(mpsc::UnboundedReceiver<LogDoc>);

/// Create a paired layer + receiver.
///
/// The layer is added to the tracing subscriber; the receiver is passed to
/// [`cosmos_log_writer`] running in a background task.
pub fn create_cosmos_log_sink(session_id: String) -> (CosmosLogLayer, CosmosLogReceiver) {
    let (tx, rx) = mpsc::unbounded_channel();
    let layer = CosmosLogLayer {
        session_id,
        tx,
        counter: std::sync::atomic::AtomicU64::new(0),
    };
    (layer, CosmosLogReceiver(rx))
}

impl<S> Layer<S> for CosmosLogLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();

        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let timestamp_ms = now.as_millis() as u64;

        let seq = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let id = format!("{}-{:06}", timestamp_ms, seq);

        let doc = LogDoc {
            id,
            session_id: self.session_id.clone(),
            timestamp_ms,
            level: metadata.level().to_string(),
            target: metadata.target().to_string(),
            message: visitor.message,
            fields: serde_json::Value::Object(visitor.fields),
            ttl: 86400, // 24 hours
        };

        let _ = self.tx.send(doc);
    }
}

// ── Field visitor ────────────────────────────────────────────────────────

#[derive(Default)]
struct FieldVisitor {
    message: String,
    fields: serde_json::Map<String, serde_json::Value>,
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(format!("{:?}", value)),
            );
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::Bool(value));
    }
}

// ── Background writer ────────────────────────────────────────────────────

/// Reads log documents from the channel and POSTs them to Cosmos DB.
///
/// Uses the REST API with master-key HMAC-SHA256 authentication. Errors
/// are printed to stderr (not tracing — that would cause recursion).
pub async fn cosmos_log_writer(receiver: CosmosLogReceiver, endpoint: String, key: String) {
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;

    let master_key = match B64.decode(&key) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("[cosmos-log] Invalid COSMOS_LOG_KEY (bad base64): {e}");
            return;
        }
    };

    let client = reqwest::Client::new();
    let resource_link = "dbs/cimmeria/colls/server-logs";
    let url = format!("{endpoint}/{resource_link}/docs");

    let mut rx = receiver.0;
    let mut consecutive_errors: u32 = 0;

    while let Some(doc) = rx.recv().await {
        let body = match serde_json::to_string(&doc) {
            Ok(b) => b,
            Err(_) => continue,
        };

        let date = rfc1123_now();
        let sig = auth_signature(&master_key, "post", "docs", resource_link, &date);
        let auth = auth_header(&sig);

        let result = client
            .post(&url)
            .header("Authorization", &auth)
            .header("x-ms-date", &date)
            .header("x-ms-version", "2018-12-31")
            .header(
                "x-ms-documentdb-partitionkey",
                format!("[\"{}\"]", doc.session_id),
            )
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => {
                consecutive_errors = 0;
            }
            Ok(resp) if resp.status().as_u16() == 429 => {
                // Rate limited — silently drop this entry.
                if consecutive_errors == 0 {
                    eprintln!("[cosmos-log] Rate limited (429), dropping entries");
                }
                consecutive_errors += 1;
            }
            Ok(resp) => {
                if consecutive_errors < 3 {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    eprintln!("[cosmos-log] HTTP {status}: {text}");
                }
                consecutive_errors += 1;
            }
            Err(e) => {
                if consecutive_errors < 3 {
                    eprintln!("[cosmos-log] Network error: {e}");
                }
                consecutive_errors += 1;
            }
        }
    }
}

// ── Cosmos DB REST API auth ──────────────────────────────────────────────

/// Build the URL-encoded Authorization header value.
fn auth_header(signature: &str) -> String {
    let raw = format!("type=master&ver=1.0&sig={}", signature);
    percent_encode(&raw)
}

/// Compute the HMAC-SHA256 auth signature for a Cosmos DB request.
fn auth_signature(
    master_key: &[u8],
    verb: &str,
    resource_type: &str,
    resource_link: &str,
    date: &str,
) -> String {
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;

    let payload = format!(
        "{}\n{}\n{}\n{}\n\n",
        verb.to_lowercase(),
        resource_type.to_lowercase(),
        resource_link,
        date.to_lowercase(),
    );

    let mut mac = HmacSha256::new_from_slice(master_key).expect("HMAC accepts any key size");
    mac.update(payload.as_bytes());
    B64.encode(mac.finalize().into_bytes())
}

/// Percent-encode a string per RFC 3986 (unreserved chars pass through).
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                let _ = write!(out, "%{:02X}", b);
            }
        }
    }
    out
}

// ── Date helpers ─────────────────────────────────────────────────────────

/// RFC 1123 / RFC 7231 date for the current instant.
///
/// e.g. `"Thu, 13 Mar 2026 22:56:15 GMT"`
fn rfc1123_now() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let secs_per_day: u64 = 86400;
    let days = now / secs_per_day;
    let day_secs = now % secs_per_day;
    let hours = day_secs / 3600;
    let minutes = (day_secs % 3600) / 60;
    let seconds = day_secs % 60;

    let (year, month, day) = days_to_ymd(days);
    let dow = day_of_week(year, month, day);

    const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    format!(
        "{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
        DAYS[dow as usize],
        day,
        MONTHS[(month - 1) as usize],
        year,
        hours,
        minutes,
        seconds
    )
}

/// Howard Hinnant's `civil_from_days` — days since Unix epoch to (Y, M, D).
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u64, m, d)
}

/// Tomohiko Sakamoto's day-of-week (0 = Sunday).
fn day_of_week(year: u64, month: u64, day: u64) -> u64 {
    const T: [u64; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    (y + y / 4 - y / 100 + y / 400 + T[(month - 1) as usize] + day) % 7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc1123_format_known_date() {
        // 2026-03-13 is a Friday.
        // epoch day 20525 = 2026-01-01 (day 20454) + 71 days (31 Jan + 28 Feb + 12 Mar)
        // 20525 * 86400 + 22h56m15s = 1773360000 + 82575 = 1773442575
        let secs = 1_773_442_575_u64;
        let days = secs / 86400;
        let (y, m, d) = days_to_ymd(days);
        assert_eq!((y, m, d), (2026, 3, 13));
        assert_eq!(day_of_week(y, m, d), 5); // Friday
    }

    #[test]
    fn percent_encode_auth() {
        let encoded = percent_encode("type=master&ver=1.0&sig=abc+/=");
        assert_eq!(encoded, "type%3Dmaster%26ver%3D1.0%26sig%3Dabc%2B%2F%3D");
    }
}
