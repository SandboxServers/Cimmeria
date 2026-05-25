//! Crash-safe on-disk overflow queue for telemetry events.
//!
//! Append-only JSONL. Bounded at [`MAX_QUEUE_BYTES`]; once the file
//! exceeds the cap a compaction pass rewrites it with the most recent
//! tail (drop-oldest), incrementing a counter the bundle metadata
//! surfaces as `dropped_lines`.
//!
//! Persistence shape:
//!
//! - `<state_dir>/telemetry-queue.jsonl` — one event per line, UTF-8.
//! - `<state_dir>/telemetry-queue.dropped` — cumulative dropped-line
//!   counter, surfaces in bundle metadata.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

/// Default overflow cap: 100 MiB matches the spec. Crossing it
/// triggers a compaction that retains the most recent suffix.
pub const MAX_QUEUE_BYTES: u64 = 100 * 1024 * 1024;

/// After compaction, retain this much from the tail. Smaller than the
/// cap so successive overflows don't compact on every append.
pub const RETAIN_AFTER_COMPACT_BYTES: u64 = 80 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct DiskQueue {
    path: PathBuf,
    dropped_path: PathBuf,
    max_bytes: u64,
    retain_bytes: u64,
}

impl DiskQueue {
    pub fn new(state_dir: &Path) -> Self {
        Self::with_caps(state_dir, MAX_QUEUE_BYTES, RETAIN_AFTER_COMPACT_BYTES)
    }

    /// Testing seam — same shape as `new` but with explicit caps so a
    /// unit test can drive overflow / compaction at small sizes.
    pub fn with_caps(state_dir: &Path, max_bytes: u64, retain_bytes: u64) -> Self {
        Self {
            path: state_dir.join("telemetry-queue.jsonl"),
            dropped_path: state_dir.join("telemetry-queue.dropped"),
            max_bytes,
            retain_bytes,
        }
    }

    /// Append one event. Triggers compaction if the file would cross
    /// [`max_bytes`]. Each event becomes exactly one line terminated
    /// by `\n`.
    pub fn enqueue<T: Serialize>(&self, event: &T) -> Result<(), QueueError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut line = serde_json::to_vec(event)?;
        line.push(b'\n');

        let current_len = self.path.metadata().map(|m| m.len()).unwrap_or(0);
        if current_len + line.len() as u64 > self.max_bytes {
            self.compact_to_retain_tail()?;
        }
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        f.write_all(&line)?;
        Ok(())
    }

    /// Read every event currently on disk, deserialize, and remove the
    /// file. Caller treats the returned events as in-flight — if the
    /// downstream upload fails the events should be re-enqueued
    /// individually rather than left on disk (the file is already gone).
    pub fn drain<T: DeserializeOwned>(&self) -> Result<Vec<T>, QueueError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let f = std::fs::File::open(&self.path)?;
        let mut out = Vec::new();
        for line in BufReader::new(f).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            // Skip lines we can't parse rather than aborting the whole
            // drain — a partial-write at the tail from a crash mid-
            // append shouldn't strand the whole queue.
            if let Ok(ev) = serde_json::from_str::<T>(&line) {
                out.push(ev);
            } else {
                tracing::warn!(
                    bytes = line.len(),
                    "skipping unparseable telemetry-queue line"
                );
            }
        }
        std::fs::remove_file(&self.path)?;
        Ok(out)
    }

    pub fn dropped_count(&self) -> u64 {
        std::fs::read_to_string(&self.dropped_path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    }

    fn add_dropped(&self, n: u64) -> Result<(), QueueError> {
        let prev = self.dropped_count();
        let next = prev.saturating_add(n);
        if let Some(parent) = self.dropped_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.dropped_path, next.to_string().as_bytes())?;
        Ok(())
    }

    /// Rewrite the queue file keeping only the most-recent
    /// `retain_bytes` worth of complete lines. Lines that straddle the
    /// retention cut-off are dropped whole so the file is always
    /// valid JSONL after a compaction.
    fn compact_to_retain_tail(&self) -> Result<(), QueueError> {
        let original = std::fs::read(&self.path)?;
        if original.len() as u64 <= self.retain_bytes {
            return Ok(());
        }
        let cut = original.len() - self.retain_bytes as usize;
        // Advance to the next newline so we drop straddling lines.
        let keep_from = match original[cut..].iter().position(|&b| b == b'\n') {
            Some(off) => cut + off + 1,
            None => original.len(),
        };
        let dropped_lines = count_lines_in(&original[..keep_from]);
        let tmp = self.path.with_extension("jsonl.tmp");
        std::fs::write(&tmp, &original[keep_from..])?;
        std::fs::rename(&tmp, &self.path)?;
        self.add_dropped(dropped_lines)?;
        Ok(())
    }
}

fn count_lines_in(bytes: &[u8]) -> u64 {
    bytes.iter().filter(|&&b| b == b'\n').count() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    struct Sample {
        seq: u64,
        msg: String,
    }

    fn sample(i: u64) -> Sample {
        Sample {
            seq: i,
            msg: format!("event-{i}"),
        }
    }

    fn queue_file(dir: &tempfile::TempDir) -> std::path::PathBuf {
        dir.path().join("telemetry-queue.jsonl")
    }

    #[test]
    fn enqueue_then_drain_returns_events_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::new(dir.path());
        for i in 0..5 {
            q.enqueue(&sample(i)).unwrap();
        }
        let drained: Vec<Sample> = q.drain().unwrap();
        assert_eq!(drained.len(), 5);
        assert_eq!(drained[0].seq, 0);
        assert_eq!(drained[4].seq, 4);
        assert!(!queue_file(&dir).exists(), "drain removes the queue file");
    }

    // Empty queue → empty Vec without an error so first-launch and
    // post-drain calls don't have to special-case missing-file.
    #[test]
    fn drain_on_missing_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::new(dir.path());
        let drained: Vec<Sample> = q.drain().unwrap();
        assert!(drained.is_empty());
    }

    // A partial-write at the tail (mid-line crash) should not strand
    // the whole queue — drain skips the unparseable suffix and returns
    // the rest.
    #[test]
    fn drain_skips_unparseable_lines() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::new(dir.path());
        q.enqueue(&sample(0)).unwrap();
        // Append a partial line directly.
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(queue_file(&dir))
            .unwrap();
        writeln!(f, "{{not valid json").unwrap();
        q.enqueue(&sample(1)).unwrap();
        let drained: Vec<Sample> = q.drain().unwrap();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].seq, 0);
        assert_eq!(drained[1].seq, 1);
    }

    // Compaction: with caps low enough to trigger, oldest entries are
    // dropped and the dropped counter increments. Retained tail must
    // still parse cleanly.
    #[test]
    fn enqueue_compacts_on_overflow_dropping_oldest() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::with_caps(dir.path(), 200, 100);
        for i in 0..100 {
            q.enqueue(&sample(i)).unwrap();
        }
        assert!(
            q.dropped_count() > 0,
            "compaction should have happened; dropped={}",
            q.dropped_count()
        );
        let drained: Vec<Sample> = q.drain().unwrap();
        assert!(!drained.is_empty());
        // Retained tail is the most-recent events — last seq is the
        // freshest one.
        let last = drained.last().unwrap();
        assert_eq!(last.seq, 99);
    }

    // Compaction must not produce a file with a partial leading line
    // — the retained slice starts at the byte AFTER a newline so the
    // first line is always complete.
    #[test]
    fn compaction_starts_at_line_boundary() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::with_caps(dir.path(), 200, 100);
        for i in 0..50 {
            q.enqueue(&sample(i)).unwrap();
        }
        // Verify by inspecting the file content before drain — drain
        // itself tolerates unparseable lines, so we need the raw bytes
        // to prove compaction left a clean line boundary.
        let raw = std::fs::read(queue_file(&dir)).unwrap();
        if let Some(first_newline) = raw.iter().position(|&b| b == b'\n') {
            let first_line = &raw[..first_newline];
            assert!(
                serde_json::from_slice::<Sample>(first_line).is_ok(),
                "first line after compaction must parse: {:?}",
                std::str::from_utf8(first_line)
            );
        }
    }

    #[test]
    fn dropped_counter_persists_across_compactions() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::with_caps(dir.path(), 200, 100);
        for i in 0..200 {
            q.enqueue(&sample(i)).unwrap();
        }
        let dropped_after_first_compactions = q.dropped_count();
        for i in 200..300 {
            q.enqueue(&sample(i)).unwrap();
        }
        assert!(
            q.dropped_count() >= dropped_after_first_compactions,
            "dropped counter is cumulative across compactions"
        );
    }
}
