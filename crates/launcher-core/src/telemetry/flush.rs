//! Flushing the on-disk queue as chunks: split by size and rows, a 4xx
//! rejection (413, 400, ...) dropped as poison, a 429/503 honoured as a
//! back-off.
//!
//! The queue can hold far more than one chunk (a server outage, or a
//! launcher killed mid-session, leaves a backlog that survives restarts),
//! so it is posted as several chunks of at most the mint's
//! `chunk_max_bytes` of NDJSON and [`chunk::MAX_CHUNK_ROWS`] rows.

use super::chunk::{self, ChunkError};
use super::events::TelemetryEvent;
use super::queue::DiskQueue;
use super::{map_chunk_err, TelemetryError};

/// The chunk size used when the mint did not say (`chunk_max_bytes` 0).
const DEFAULT_CHUNK_MAX_BYTES: u64 = 1_048_576;

/// Where [`flush_queue`] posts, and how big a chunk may be.
pub(super) struct FlushTarget<'a> {
    pub(super) endpoint: &'a str,
    pub(super) token: &'a str,
    /// The mint's `chunk_max_bytes`; 0 means [`DEFAULT_CHUNK_MAX_BYTES`].
    pub(super) max_bytes: u64,
}

/// [`Telemetry::flush`] with its queue, target and back-off clock passed
/// in, for tests.
pub(super) async fn flush_queue(
    http: &reqwest::Client,
    queue: &DiskQueue,
    target: &FlushTarget<'_>,
    retry_not_before_ms: &std::sync::atomic::AtomicI64,
) -> Result<u64, TelemetryError> {
    use std::sync::atomic::Ordering;

    let now_ms = chrono::Utc::now().timestamp_millis();
    if now_ms < retry_not_before_ms.load(Ordering::Relaxed) {
        return Ok(0);
    }
    let events: Vec<TelemetryEvent> = queue.drain()?;
    if events.is_empty() {
        return Ok(0);
    }
    let max_bytes = match target.max_bytes {
        0 => DEFAULT_CHUNK_MAX_BYTES,
        n => n,
    };
    let batches = chunk::split_batches(&events, max_bytes, chunk::MAX_CHUNK_ROWS)?;
    let mut sent = 0u64;
    for batch in batches {
        let n = batch.len() as u64;
        match chunk::post_chunk(http, target.endpoint, target.token, &events[batch.clone()]).await {
            Ok(()) => sent += n,
            Err(ChunkError::Rejected { status, .. }) => {
                tracing::warn!(
                    status,
                    dropped = n,
                    "telemetry chunk rejected by the server; its events are dropped"
                );
                if let Err(e) = queue.add_dropped(n) {
                    tracing::warn!(error = %e, "telemetry dropped-lines counter update failed");
                }
            }
            Err(e) => {
                if let Some(secs) = e.retry_after_secs() {
                    let wait_ms = i64::try_from(secs.saturating_mul(1000)).unwrap_or(i64::MAX / 2);
                    retry_not_before_ms.store(now_ms.saturating_add(wait_ms), Ordering::Relaxed);
                }
                // Best-effort re-enqueue; failures here surface to the
                // tracing log but don't override the underlying chunk
                // error.
                for ev in &events[batch.start..] {
                    if let Err(re) = queue.enqueue(ev) {
                        tracing::warn!(error = %re, "telemetry re-enqueue after chunk failure");
                        break;
                    }
                }
                return Err(map_chunk_err(e));
            }
        }
    }
    Ok(sent)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicI64;

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::telemetry::events::ClientLogEvent;

    fn ev(seq: u64) -> TelemetryEvent {
        TelemetryEvent::ClientLog(ClientLogEvent {
            ts_ms: 1,
            seq,
            source_file: "x.log".into(),
            level: "info".into(),
            category: "raw".into(),
            packet_no: None,
            message: format!("event-{seq}"),
        })
    }

    fn queue_with(dir: &tempfile::TempDir, n: u64) -> DiskQueue {
        let q = DiskQueue::new(dir.path());
        for i in 0..n {
            q.enqueue(&ev(i)).unwrap();
        }
        q
    }

    async fn server(status: u16, retry_after: Option<&str>, expect: u64) -> MockServer {
        let server = MockServer::start().await;
        let mut resp = ResponseTemplate::new(status);
        if let Some(v) = retry_after {
            resp = resp.insert_header("retry-after", v);
        }
        Mock::given(method("POST"))
            .and(path("/api/upload-chunk"))
            .respond_with(resp)
            .expect(expect)
            .mount(&server)
            .await;
        server
    }

    async fn flush(
        server: &MockServer,
        q: &DiskQueue,
        clock: &AtomicI64,
    ) -> Result<u64, TelemetryError> {
        let endpoint = format!("{}/api", server.uri());
        let target = FlushTarget {
            endpoint: &endpoint,
            token: "t",
            max_bytes: 0,
        };
        flush_queue(&reqwest::Client::new(), q, &target, clock).await
    }

    /// A backlog is posted as several chunks of at most 1,000 rows.
    #[tokio::test]
    async fn a_backlog_is_posted_in_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let q = queue_with(&dir, 2_500);
        let server = server(200, None, 3).await;
        let sent = flush(&server, &q, &AtomicI64::new(0)).await.unwrap();
        assert_eq!(sent, 2_500);
        assert!(q.drain::<TelemetryEvent>().unwrap().is_empty());
    }

    /// **A 413 is poison.** The refused events are dropped and counted, not
    /// re-queued: re-queueing them would send the same refused chunk on
    /// every flush, forever.
    #[tokio::test]
    async fn a_413_drops_the_chunk_instead_of_requeueing_it() {
        let dir = tempfile::tempdir().unwrap();
        let q = queue_with(&dir, 1_500);
        let server = server(413, None, 2).await;
        let sent = flush(&server, &q, &AtomicI64::new(0)).await.unwrap();
        assert_eq!(sent, 0);
        assert!(q.drain::<TelemetryEvent>().unwrap().is_empty());
        assert_eq!(q.dropped_count(), 1_500);
    }

    /// A 503 keeps the events and holds further flushes off for its
    /// `Retry-After`: the second flush sends nothing.
    #[tokio::test]
    async fn a_503_requeues_and_backs_off_for_retry_after() {
        let dir = tempfile::tempdir().unwrap();
        let q = queue_with(&dir, 10);
        let server = server(503, Some("120"), 1).await;
        let clock = AtomicI64::new(0);
        assert!(flush(&server, &q, &clock).await.is_err());
        assert_eq!(flush(&server, &q, &clock).await.unwrap(), 0);
        assert_eq!(q.drain::<TelemetryEvent>().unwrap().len(), 10);
    }

    #[test]
    fn batches_respect_rows_and_bytes() {
        let events: Vec<_> = (0..25).map(ev).collect();
        let by_rows = chunk::split_batches(&events, u64::MAX, 10).unwrap();
        assert_eq!(by_rows, vec![0..10, 10..20, 20..25]);
        let one = serde_json::to_vec(&events[0]).unwrap().len() as u64 + 1;
        let by_bytes = chunk::split_batches(&events, one * 4, 1_000).unwrap();
        assert!(by_bytes.iter().all(|b| b.len() <= 4), "{by_bytes:?}");
        assert_eq!(by_bytes.iter().map(|b| b.len()).sum::<usize>(), 25);
        // An event bigger than the cap goes alone rather than being lost.
        assert_eq!(
            chunk::split_batches(&events[..2], 1, 1_000).unwrap(),
            vec![0..1, 1..2]
        );
    }
}
