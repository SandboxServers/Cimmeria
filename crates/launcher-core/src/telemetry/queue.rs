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
//! - `<state_dir>/telemetry-queue.inflight.jsonl` — the queue a flush took
//!   ([`DiskQueue::take_batch`] renames the queue to it, so new events start
//!   a fresh queue), and `telemetry-queue.inflight.offset`, the bytes of it
//!   the server has already taken. Both go only when the last chunk of it
//!   is acknowledged, so a launcher closed or killed mid-flush loses
//!   nothing: the next flush resumes at the offset (at worst one chunk is
//!   sent twice; the server keys chunks by `seq`). The 100 MiB cap applies
//!   to each file, so the two together can briefly hold twice that.
//!
//! Every file operation takes one process-wide lock, so an append can
//! never land between a flush's read and its rename, and no lock is held
//! while a chunk is on the wire.

use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

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

/// Short file operations only; never held across an await.
static FILE_LOCK: Mutex<()> = Mutex::new(());

fn file_lock() -> MutexGuard<'static, ()> {
    FILE_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

pub struct DiskQueue {
    path: PathBuf,
    dropped_path: PathBuf,
    inflight_path: PathBuf,
    offset_path: PathBuf,
    max_bytes: u64,
    retain_bytes: u64,
}

/// One chunk's worth of events read from the in-flight file by
/// [`DiskQueue::take_batch`]. Pass it to [`DiskQueue::commit_batch`] once
/// the server has taken (or permanently refused) it; dropping it instead
/// leaves the events queued.
#[derive(Debug)]
pub struct Batch<T> {
    pub events: Vec<T>,
    /// Offset in the in-flight file just past this batch's last line.
    end: u64,
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
            inflight_path: state_dir.join("telemetry-queue.inflight.jsonl"),
            offset_path: state_dir.join("telemetry-queue.inflight.offset"),
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

        let _files = file_lock();
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

    /// Read every event on disk (the unsent rest of the in-flight file,
    /// then the queue), deserialize, and remove both. The events are the
    /// caller's now: nothing is left to resend. Tests and tools only; a
    /// flush uses [`DiskQueue::take_batch`], which loses nothing on a crash.
    pub fn drain<T: DeserializeOwned>(&self) -> Result<Vec<T>, QueueError> {
        let _files = file_lock();
        let mut out = Vec::new();
        if self.inflight_path.exists() {
            read_events(
                &self.inflight_path,
                self.offset(),
                u64::MAX,
                usize::MAX,
                &mut out,
            )?;
            self.finish_inflight()?;
        }
        if self.path.exists() {
            read_events(&self.path, 0, u64::MAX, usize::MAX, &mut out)?;
            std::fs::remove_file(&self.path)?;
        }
        Ok(out)
    }

    /// The next chunk to send: at most `max_bytes` of lines (one line
    /// whatever its size) and `max_rows` events, from the in-flight file,
    /// first moving the queue there when nothing is in flight. `None` when
    /// nothing is queued. Unparseable lines (a write torn by a crash, bytes
    /// that are not UTF-8) are skipped, never fatal. The events stay on disk
    /// until [`DiskQueue::commit_batch`].
    pub fn take_batch<T: DeserializeOwned>(
        &self,
        max_bytes: u64,
        max_rows: usize,
    ) -> Result<Option<Batch<T>>, QueueError> {
        let _files = file_lock();
        loop {
            if !self.inflight_path.exists() {
                if !self.path.exists() {
                    return Ok(None);
                }
                remove_if_present(&self.offset_path)?;
                std::fs::rename(&self.path, &self.inflight_path)?;
            }
            let start = self.offset();
            let mut events = Vec::new();
            let end = read_events(&self.inflight_path, start, max_bytes, max_rows, &mut events)?;
            if !events.is_empty() {
                return Ok(Some(Batch { events, end }));
            }
            // Nothing parseable left past `start`: this file is done.
            self.commit_locked(end)?;
        }
    }

    /// The server took `batch` (or refused it for good): advance past it,
    /// and delete the in-flight file once all of it has gone. A no-op when
    /// the queue was discarded meanwhile.
    pub fn commit_batch<T>(&self, batch: &Batch<T>) -> Result<(), QueueError> {
        let _files = file_lock();
        self.commit_locked(batch.end)
    }

    /// How many events wait on disk, in flight and queued.
    pub fn pending_count<T: DeserializeOwned>(&self) -> Result<u64, QueueError> {
        let _files = file_lock();
        let mut n = 0u64;
        for (path, start) in [(&self.inflight_path, self.offset()), (&self.path, 0)] {
            if path.exists() {
                let mut events: Vec<T> = Vec::new();
                read_events(path, start, u64::MAX, usize::MAX, &mut events)?;
                n = n.saturating_add(events.len() as u64);
            }
        }
        Ok(n)
    }

    /// End the queue with a newline if a crash tore its last line, so the
    /// next append starts a line of its own rather than joining the torn
    /// one (which would lose the new event with it).
    pub fn repair_torn_tail(&self) -> Result<(), QueueError> {
        let _files = file_lock();
        let mut f = match std::fs::OpenOptions::new()
            .read(true)
            .append(true)
            .open(&self.path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        let len = f.metadata()?.len();
        if len == 0 {
            return Ok(());
        }
        f.seek(SeekFrom::Start(len - 1))?;
        let mut last = [0u8; 1];
        std::io::Read::read_exact(&mut f, &mut last)?;
        if last[0] != b'\n' {
            f.write_all(b"\n")?;
        }
        Ok(())
    }

    /// Delete every queue file unsent: the queue, the in-flight file and
    /// its offset, and the dropped-lines counter. Returns whether anything
    /// was there. For a player who opted out.
    pub fn discard(&self) -> Result<bool, QueueError> {
        let _files = file_lock();
        let mut removed = false;
        for path in [
            &self.path,
            &self.inflight_path,
            &self.offset_path,
            &self.dropped_path,
        ] {
            removed |= remove_if_present(path)?;
        }
        Ok(removed)
    }

    /// Bytes of the in-flight file already acknowledged; 0 when unknown (a
    /// torn write resends rather than skips).
    fn offset(&self) -> u64 {
        std::fs::read_to_string(&self.offset_path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    }

    fn commit_locked(&self, end: u64) -> Result<(), QueueError> {
        let len = match self.inflight_path.metadata() {
            Ok(m) => m.len(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e.into()),
        };
        if end >= len {
            return self.finish_inflight();
        }
        let tmp = self.offset_path.with_extension("offset.tmp");
        std::fs::write(&tmp, end.to_string().as_bytes())?;
        std::fs::rename(&tmp, &self.offset_path)?;
        Ok(())
    }

    fn finish_inflight(&self) -> Result<(), QueueError> {
        remove_if_present(&self.inflight_path)?;
        remove_if_present(&self.offset_path)?;
        Ok(())
    }

    pub fn dropped_count(&self) -> u64 {
        std::fs::read_to_string(&self.dropped_path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    }

    /// Clear the cumulative dropped-line counter — called by the
    /// session runner after a successful bundle upload so the next
    /// session's metadata reports drops attributable to *that*
    /// session, not the union of all prior runs.
    pub fn dropped_counter_reset(&self) -> Result<(), QueueError> {
        let _files = file_lock();
        remove_if_present(&self.dropped_path)?;
        Ok(())
    }

    pub fn add_dropped(&self, n: u64) -> Result<(), QueueError> {
        let _files = file_lock();
        self.add_dropped_locked(n)
    }

    fn add_dropped_locked(&self, n: u64) -> Result<(), QueueError> {
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
        self.add_dropped_locked(dropped_lines)?;
        Ok(())
    }
}

fn count_lines_in(bytes: &[u8]) -> u64 {
    bytes.iter().filter(|&&b| b == b'\n').count() as u64
}

/// Whether `path` was there to remove.
fn remove_if_present(path: &Path) -> Result<bool, QueueError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Parse lines of `path` from byte `start` into `out`, stopping before the
/// line that would take the batch past `max_bytes` (the first line always
/// goes) or once `max_rows` events are in. Returns the offset just past the
/// last line read. Lines are raw bytes: a line that is not UTF-8 or not
/// JSON is skipped with a warning, so it cannot strand the lines after it.
fn read_events<T: DeserializeOwned>(
    path: &Path,
    start: u64,
    max_bytes: u64,
    max_rows: usize,
    out: &mut Vec<T>,
) -> Result<u64, QueueError> {
    let mut f = std::fs::File::open(path)?;
    f.seek(SeekFrom::Start(start))?;
    let mut reader = BufReader::new(f);
    let mut offset = start;
    let mut taken = 0u64;
    let mut rows = 0usize;
    let mut line = Vec::new();
    loop {
        line.clear();
        let n = reader.read_until(b'\n', &mut line)? as u64;
        if n == 0 {
            break;
        }
        if taken > 0 && taken.saturating_add(n) > max_bytes {
            break;
        }
        taken += n;
        offset += n;
        let text = line.trim_ascii();
        if text.is_empty() {
            continue;
        }
        match serde_json::from_slice::<T>(text) {
            Ok(ev) => {
                out.push(ev);
                rows += 1;
                if rows >= max_rows {
                    break;
                }
            }
            Err(_) => tracing::warn!(
                bytes = text.len(),
                "skipping unparseable telemetry-queue line"
            ),
        }
    }
    Ok(offset)
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

    /// **A line that is not UTF-8 does not block the queue.** A torn write
    /// can leave any bytes; the events either side of it still go.
    #[test]
    fn a_non_utf8_line_is_skipped_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::new(dir.path());
        q.enqueue(&sample(0)).unwrap();
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(queue_file(&dir))
            .unwrap();
        f.write_all(b"\xff\xfe{\"seq\":\xc3\n").unwrap();
        drop(f);
        q.enqueue(&sample(1)).unwrap();
        assert_eq!(q.pending_count::<Sample>().unwrap(), 2);
        let batch = q.take_batch::<Sample>(u64::MAX, 1_000).unwrap().unwrap();
        assert_eq!(batch.events, [sample(0), sample(1)]);
        q.commit_batch(&batch).unwrap();
        assert!(q.take_batch::<Sample>(u64::MAX, 1_000).unwrap().is_none());
    }

    /// **Nothing leaves the disk before the server has it.** A batch taken
    /// and never committed (the launcher closed mid-POST) is taken again,
    /// whole, by the next flush, ahead of anything queued since; a
    /// committed one is not.
    #[test]
    fn an_uncommitted_batch_is_taken_again_after_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::new(dir.path());
        for i in 0..5 {
            q.enqueue(&sample(i)).unwrap();
        }
        let first = q.take_batch::<Sample>(u64::MAX, 2).unwrap().unwrap();
        assert_eq!(first.events, [sample(0), sample(1)]);
        q.commit_batch(&first).unwrap();
        let lost = q.take_batch::<Sample>(u64::MAX, 2).unwrap().unwrap();
        assert_eq!(lost.events, [sample(2), sample(3)]);
        drop(lost);
        q.enqueue(&sample(5)).unwrap();

        // A new process: only the files remain.
        let q = DiskQueue::new(dir.path());
        assert_eq!(q.pending_count::<Sample>().unwrap(), 4);
        let mut seen = Vec::new();
        while let Some(batch) = q.take_batch::<Sample>(u64::MAX, 2).unwrap() {
            seen.extend(batch.events.iter().map(|s| s.seq));
            q.commit_batch(&batch).unwrap();
        }
        assert_eq!(seen, [2, 3, 4, 5]);
        assert!(!dir.path().join("telemetry-queue.inflight.jsonl").exists());
        assert!(!dir.path().join("telemetry-queue.inflight.offset").exists());
    }

    /// A batch stops before the line that would cross `max_bytes`, but the
    /// first line always goes, however big.
    #[test]
    fn a_batch_respects_max_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::new(dir.path());
        for i in 0..4 {
            q.enqueue(&sample(i)).unwrap();
        }
        let line = serde_json::to_vec(&sample(0)).unwrap().len() as u64 + 1;
        let batch = q.take_batch::<Sample>(line * 2, 1_000).unwrap().unwrap();
        assert_eq!(batch.events.len(), 2);
        q.commit_batch(&batch).unwrap();
        let tiny = q.take_batch::<Sample>(1, 1_000).unwrap().unwrap();
        assert_eq!(tiny.events, [sample(2)]);
    }

    /// A crash mid-append leaves a line with no newline; startup repair
    /// ends it so the next event is not glued onto it.
    #[test]
    fn repair_keeps_the_next_event_off_a_torn_line() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::new(dir.path());
        q.enqueue(&sample(0)).unwrap();
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(queue_file(&dir))
            .unwrap();
        f.write_all(b"{\"seq\":1,\"ms").unwrap();
        drop(f);
        q.repair_torn_tail().unwrap();
        q.enqueue(&sample(2)).unwrap();
        let drained: Vec<Sample> = q.drain().unwrap();
        assert_eq!(drained, [sample(0), sample(2)]);
    }

    #[test]
    fn discard_removes_every_queue_file() {
        let dir = tempfile::tempdir().unwrap();
        let q = DiskQueue::new(dir.path());
        for i in 0..3 {
            q.enqueue(&sample(i)).unwrap();
        }
        let batch = q.take_batch::<Sample>(u64::MAX, 1).unwrap().unwrap();
        q.commit_batch(&batch).unwrap();
        q.enqueue(&sample(3)).unwrap();
        q.add_dropped(2).unwrap();
        assert!(q.discard().unwrap());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        assert!(!q.discard().unwrap());
        // A commit for a batch taken before the discard writes nothing.
        q.commit_batch(&batch).unwrap();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
