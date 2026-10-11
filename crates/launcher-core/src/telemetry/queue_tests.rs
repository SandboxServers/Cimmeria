//! Tests for the disk queue ([`DiskQueue`]).

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

/// **Opting out leaves no events behind, even after a crash.** A crash
/// between the write and the rename of a compaction or an offset update
/// leaves its temporary file; the compaction's holds queued events.
#[test]
fn discard_removes_the_temp_files_a_crash_left() {
    let dir = tempfile::tempdir().unwrap();
    let q = DiskQueue::new(dir.path());
    let line = serde_json::to_string(&sample(7)).unwrap();
    std::fs::write(
        dir.path().join("telemetry-queue.jsonl.tmp"),
        format!("{line}\n"),
    )
    .unwrap();
    std::fs::write(dir.path().join("telemetry-queue.inflight.offset.tmp"), "12").unwrap();
    assert!(q.discard().unwrap());
    let left: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

/// Take one batch of three and commit it, then overwrite the offset.
fn queue_with_offset(dir: &tempfile::TempDir, offset: &str) -> DiskQueue {
    let q = DiskQueue::new(dir.path());
    for i in 0..3 {
        q.enqueue(&sample(i)).unwrap();
    }
    let batch = q.take_batch::<Sample>(u64::MAX, 1).unwrap().unwrap();
    q.commit_batch(&batch).unwrap();
    let offset_path = dir.path().join("telemetry-queue.inflight.offset");
    assert!(offset_path.is_file());
    std::fs::write(offset_path, offset).unwrap();
    q
}

fn take_all(q: &DiskQueue) -> Vec<u64> {
    let mut seen = Vec::new();
    while let Some(batch) = q.take_batch::<Sample>(u64::MAX, 1_000).unwrap() {
        seen.extend(batch.events.iter().map(|s| s.seq));
        q.commit_batch(&batch).unwrap();
    }
    seen
}

/// **A bad offset resends; it never deletes unsent events.** An offset at
/// or past the end of the in-flight file, one that does not parse, and one
/// in the middle of a line all mean the whole file goes again.
#[test]
fn a_bad_offset_resends_the_whole_inflight_file() {
    let line = serde_json::to_vec(&sample(0)).unwrap().len() + 1;
    for offset in [
        "999999".to_string(),
        (line * 3).to_string(),
        "not-a-number".to_string(),
        "5".to_string(),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let q = queue_with_offset(&dir, &offset);
        assert_eq!(q.pending_count::<Sample>().unwrap(), 3, "offset {offset}");
        assert_eq!(take_all(&q), [0, 1, 2], "offset {offset}");
    }
}

/// A launcher killed between moving the queue in flight and sending any of
/// it leaves the in-flight file with no offset: the next run sends it
/// whole, ahead of what was queued since.
#[test]
fn an_inflight_file_with_no_offset_is_sent_whole() {
    let dir = tempfile::tempdir().unwrap();
    let q = DiskQueue::new(dir.path());
    for i in 0..3 {
        q.enqueue(&sample(i)).unwrap();
    }
    drop(q.take_batch::<Sample>(u64::MAX, 1).unwrap().unwrap());
    assert!(dir.path().join("telemetry-queue.inflight.jsonl").is_file());
    assert!(!dir.path().join("telemetry-queue.inflight.offset").exists());
    assert!(!queue_file(&dir).exists());

    let q = DiskQueue::new(dir.path());
    q.enqueue(&sample(3)).unwrap();
    assert_eq!(take_all(&q), [0, 1, 2, 3]);
}

/// An event queued while a batch is on the wire goes to a fresh queue file,
/// not the in-flight one, and is taken right after the in-flight file ends.
#[test]
fn an_enqueue_during_a_batch_lands_in_the_new_queue_and_goes_next() {
    let dir = tempfile::tempdir().unwrap();
    let q = DiskQueue::new(dir.path());
    for i in 0..2 {
        q.enqueue(&sample(i)).unwrap();
    }
    let inflight = dir.path().join("telemetry-queue.inflight.jsonl");
    let batch = q.take_batch::<Sample>(u64::MAX, 1_000).unwrap().unwrap();
    let before = std::fs::read(&inflight).unwrap();
    q.enqueue(&sample(2)).unwrap();
    assert_eq!(std::fs::read(&inflight).unwrap(), before);
    let queued: Vec<Sample> = read_back(&queue_file(&dir));
    assert_eq!(queued, [sample(2)]);

    q.commit_batch(&batch).unwrap();
    assert_eq!(batch.events, [sample(0), sample(1)]);
    assert!(!inflight.exists());
    assert_eq!(take_all(&q), [2]);
}

fn read_back(path: &std::path::Path) -> Vec<Sample> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}
