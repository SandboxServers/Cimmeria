//! Polling tailer for the Atera client log + BigWorld `sgwdebuglog*`.
//!
//! Stateful: each watched file gets a [`TailedFile`] that remembers
//! its last-read offset and inode (Windows: file index) so a
//! mid-stream rotation surfaces as "new file appeared" rather than
//! "file truncated, replay from zero." A `tick()` call returns all
//! lines that appeared since the previous tick across every watched
//! path.
//!
//! Polling (not `notify`) because the flush interval is generous
//! (~2 s) and avoids adding native deps for marginal latency.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The most one tick reads from one file. A file with more waiting (a
/// large log that appeared since the last tick) is read over several
/// ticks, so a tick never holds a whole log in memory.
pub const MAX_READ_PER_TICK: u64 = 1024 * 1024;

/// One file the tailer is watching. The (len, modified) pair is the
/// fast-path "did anything change?" check; a full read only happens
/// when the cheap check trips.
#[derive(Debug, Clone)]
struct TailedFile {
    /// Bytes consumed so far. New bytes start at this offset.
    pos: u64,
    /// Last observed size — a smaller size on the next tick means
    /// the file was truncated or rotated.
    last_len: u64,
    /// The file's creation time, when the platform has one. A rename keeps
    /// it, so a file that appears under a new name with a tracked file's
    /// creation time is that file, and keeps its offset.
    created: Option<SystemTime>,
}

impl TailedFile {
    fn fresh(len: u64, created: Option<SystemTime>) -> Self {
        Self {
            pos: len,
            last_len: len,
            created,
        }
    }
}

/// Where [`Tailer::scan`] starts a file it has not seen before.
#[derive(Debug, Clone, Copy)]
enum Start {
    /// At its end.
    End,
    /// At its start if it was created at or after this instant (it is the
    /// session's), otherwise at its end.
    NewSince(SystemTime),
}

/// One line read from a tailed file, with the source path so the
/// downstream parser can tag the resulting telemetry event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailedLine {
    pub source_file: String,
    pub line: String,
}

/// Polling file tailer. Construct with [`Tailer::new`], call
/// [`Tailer::tick`] on the flush cadence to drain everything new.
#[derive(Default)]
pub struct Tailer {
    files: HashMap<PathBuf, TailedFile>,
}

impl Tailer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of files currently being tracked. Useful in tests and
    /// for a "Telemetry: tailing N files" status line in the UI.
    pub fn watched_count(&self) -> usize {
        self.files.len()
    }

    /// Walk `root` for files matching `predicate`, add any not yet
    /// seen, and drop any that no longer match (e.g. were deleted).
    /// Tail position for a newly-discovered file starts at *end of
    /// file* so we don't ship the entire history of a pre-existing
    /// log on first tick — the legacy data goes into the end-of-
    /// session bundle instead.
    ///
    /// One call per tick covers every root: the files a walk did not
    /// match are dropped, so two calls over different roots drop each
    /// other's files. Give one root and a predicate that matches them all.
    pub fn refresh(&mut self, root: &Path, predicate: impl Fn(&Path) -> bool) {
        self.scan(root, predicate, Start::End);
    }

    /// Like [`Tailer::refresh`], except that a newly-discovered file
    /// created at or after `baseline` is read from its *start*. For a
    /// session whose pre-existing files were taken in by a `refresh` at
    /// `baseline`, before the game started: a file created after that was
    /// written by this session, so its first lines belong in the stream
    /// too. An older file that only now matches (an earlier log rotated
    /// by rename, say) starts at its end, so its history is not sent. A
    /// platform with no creation time starts every new file at its end.
    pub fn follow(&mut self, root: &Path, predicate: impl Fn(&Path) -> bool, baseline: SystemTime) {
        self.scan(root, predicate, Start::NewSince(baseline));
    }

    fn scan(&mut self, root: &Path, predicate: impl Fn(&Path) -> bool, start: Start) {
        let walker = match walkdir::WalkDir::new(root)
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(entries) => entries,
            Err(_) => return,
        };
        let mut seen = std::collections::HashSet::new();
        let mut discovered = Vec::new();
        for entry in walker {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_path_buf();
            if !predicate(&path) {
                continue;
            }
            seen.insert(path.clone());
            if !self.files.contains_key(&path) {
                discovered.push(path);
            }
        }
        // Tracked files that are gone, or whose name now holds a shorter
        // file: the ones a new name can be a rename of. Two live files
        // created in the same clock tick share a creation time, so only
        // these are candidates.
        let moved: Vec<TailedFile> = if discovered.is_empty() {
            Vec::new()
        } else {
            self.files
                .iter()
                .filter(|(path, tracked)| {
                    !seen.contains(*path)
                        || std::fs::metadata(path).is_ok_and(|m| m.len() < tracked.last_len)
                })
                .map(|(_, tracked)| tracked.clone())
                .collect()
        };
        for path in discovered {
            let meta = std::fs::metadata(&path).ok();
            let len = meta.as_ref().map_or(0, |m| m.len());
            let created = meta.and_then(|m| m.created().ok());
            // A tracked file renamed (a rotated log): the same file, so it
            // carries on from where it was. NTFS also gives a name
            // re-created right after the rename the old creation time; the
            // file at that name has shrunk, and `tick` reads it from 0.
            // A rename keeps the content, so the file holds at least what
            // was read of it: that rules out a different file created in
            // the same coarse clock tick.
            let renamed = created.and_then(|created| {
                moved
                    .iter()
                    .find(|tracked| tracked.created == Some(created) && len >= tracked.pos)
                    .cloned()
            });
            let file = renamed.unwrap_or_else(|| match start {
                Start::NewSince(baseline) if created.is_some_and(|c| c >= baseline) => {
                    TailedFile::fresh(0, created)
                }
                _ => TailedFile::fresh(len, created),
            });
            self.files.insert(path, file);
        }
        // Drop tracking for paths that disappeared so a long-running
        // session doesn't leak memory.
        self.files.retain(|p, _| seen.contains(p));
    }

    /// Read every byte newly appended since the previous tick across
    /// every watched file. Splits on `\n`, trims `\r`. Returns each
    /// line tagged with its source file name (just the file name, not
    /// the full path — the launcher's install_dir is private to the
    /// dev).
    pub fn tick(&mut self) -> Vec<TailedLine> {
        let mut out = Vec::new();
        for (path, tf) in self.files.iter_mut() {
            let meta = match std::fs::metadata(path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let len = meta.len();
            if len < tf.last_len {
                // File shrank — typical rotation pattern truncates
                // the existing path or replaces it. Reset to the new
                // start so we don't try to read a stale offset.
                tf.pos = 0;
                tf.last_len = 0;
            }
            if len == tf.pos {
                tf.last_len = len;
                continue;
            }
            let mut f = match std::fs::File::open(path) {
                Ok(f) => f,
                Err(_) => continue,
            };
            if f.seek(SeekFrom::Start(tf.pos)).is_err() {
                continue;
            }
            let want = (len - tf.pos).min(MAX_READ_PER_TICK);
            let mut buf = Vec::with_capacity(want as usize);
            if (&mut f).take(want).read_to_end(&mut buf).is_err() {
                continue;
            }
            let source = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            for line in split_complete_lines(&buf) {
                out.push(TailedLine {
                    source_file: source.clone(),
                    line,
                });
            }
            // Advance only over the bytes that ended in a newline —
            // a partial final line gets replayed on the next tick so
            // its full content lands.
            let mut advance = complete_line_bytes(&buf);
            if advance == 0 && buf.len() as u64 == MAX_READ_PER_TICK {
                // A whole read with no newline: not a log line, and waiting
                // for its end would stall this file for good. Skip it.
                advance = MAX_READ_PER_TICK;
            }
            tf.pos += advance;
            tf.last_len = len;
        }
        out
    }
}

/// Yield each `\n`-terminated line, trimming any trailing `\r`. A
/// partial final line (no terminator) is NOT yielded.
fn split_complete_lines(bytes: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for chunk in bytes.split_inclusive(|&b| b == b'\n') {
        if chunk.last() != Some(&b'\n') {
            continue;
        }
        let stripped = chunk.strip_suffix(b"\n").unwrap_or(chunk);
        let stripped = stripped.strip_suffix(b"\r").unwrap_or(stripped);
        out.push(String::from_utf8_lossy(stripped).into_owned());
    }
    out
}

/// Total bytes ending in a `\n` — the offset the tailer should
/// advance to. Anything after the last newline is a partial line
/// that the next tick must reread.
fn complete_line_bytes(bytes: &[u8]) -> u64 {
    match bytes.iter().rposition(|&b| b == b'\n') {
        Some(i) => (i + 1) as u64,
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn append(path: &Path, s: &str) {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        f.write_all(s.as_bytes()).unwrap();
    }

    #[test]
    fn fresh_tail_skips_existing_content_on_first_tick() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.log");
        append(&path, "old line 1\nold line 2\n");
        let mut t = Tailer::new();
        t.refresh(dir.path(), |p| p.extension().is_some_and(|e| e == "log"));
        let lines = t.tick();
        assert!(
            lines.is_empty(),
            "first tick must skip pre-existing content; got {lines:?}"
        );
    }

    #[test]
    fn tick_emits_lines_appended_since_last_tick() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.log");
        append(&path, "preexisting\n");
        let mut t = Tailer::new();
        t.refresh(dir.path(), |p| p.extension().is_some_and(|e| e == "log"));
        let _ = t.tick();
        append(&path, "fresh-1\nfresh-2\n");
        let lines = t.tick();
        let messages: Vec<_> = lines.iter().map(|l| l.line.as_str()).collect();
        assert_eq!(messages, vec!["fresh-1", "fresh-2"]);
        assert_eq!(lines[0].source_file, "session.log");
    }

    // Trailing CRLF gets stripped — the client logs use Windows line
    // endings.
    #[test]
    fn tick_strips_crlf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.log");
        std::fs::write(&path, b"").unwrap();
        let mut t = Tailer::new();
        t.refresh(dir.path(), |p| p.extension().is_some_and(|e| e == "log"));
        append(&path, "line-1\r\nline-2\r\n");
        let lines = t.tick();
        let messages: Vec<_> = lines.iter().map(|l| l.line.as_str()).collect();
        assert_eq!(messages, vec!["line-1", "line-2"]);
    }

    // Partial final line (no trailing newline) is held back until the
    // next tick completes it — so we never ship half a log line.
    #[test]
    fn tick_holds_back_partial_final_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.log");
        std::fs::write(&path, b"").unwrap();
        let mut t = Tailer::new();
        t.refresh(dir.path(), |p| p.extension().is_some_and(|e| e == "log"));
        append(&path, "complete\nincompl");
        let lines = t.tick();
        let messages: Vec<_> = lines.iter().map(|l| l.line.as_str()).collect();
        assert_eq!(messages, vec!["complete"]);
        append(&path, "ete-now\n");
        let lines = t.tick();
        let messages: Vec<_> = lines.iter().map(|l| l.line.as_str()).collect();
        assert_eq!(messages, vec!["incomplete-now"]);
    }

    // Rotation: file shrinks (truncate / replace). Tailer resets to 0
    // and reads the new content from the top.
    #[test]
    fn tick_recovers_after_truncation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.log");
        append(&path, "old content\n");
        let mut t = Tailer::new();
        t.refresh(dir.path(), |p| p.extension().is_some_and(|e| e == "log"));
        let _ = t.tick();
        // Truncate to shorter content than the original (12 < 12: use a
        // genuinely shorter payload so the shrink-trigger fires).
        std::fs::write(&path, b"rot\n").unwrap();
        let lines = t.tick();
        let messages: Vec<_> = lines.iter().map(|l| l.line.as_str()).collect();
        assert_eq!(messages, vec!["rot"]);
    }

    // Atera rolls per minute — a new file appears alongside the old.
    // refresh() picks it up; the old file's position is preserved so
    // we don't re-ship its earlier content. New files start at
    // end-of-file so their pre-rotation content isn't double-shipped
    // (it goes into the end-of-session bundle instead).
    #[test]
    fn refresh_picks_up_new_files_without_disturbing_existing_state() {
        let dir = tempfile::tempdir().unwrap();
        let p1 = dir.path().join("2026-05-25_12-00.log");
        append(&p1, "line in first file\n");
        let mut t = Tailer::new();
        t.refresh(dir.path(), |p| p.extension().is_some_and(|e| e == "log"));
        let _ = t.tick();
        // Minute rolls; a new (empty) file appears.
        let p2 = dir.path().join("2026-05-25_12-01.log");
        std::fs::write(&p2, b"").unwrap();
        t.refresh(dir.path(), |p| p.extension().is_some_and(|e| e == "log"));
        assert_eq!(t.watched_count(), 2);
        // Append to both files post-refresh — both produce lines on
        // this tick.
        append(&p1, "later in first file\n");
        append(&p2, "line in rolled file\n");
        let lines = t.tick();
        let mut messages: Vec<_> = lines.iter().map(|l| l.line.as_str()).collect();
        messages.sort();
        assert_eq!(messages, vec!["later in first file", "line in rolled file"]);
    }

    // A file that appears after the baseline is this session's: `follow`
    // ships it from its first line, and a file already tracked keeps its
    // offset (the baseline's pre-existing content stays unsent).
    #[test]
    fn follow_reads_a_new_file_from_its_start_and_keeps_known_offsets() {
        let dir = tempfile::tempdir().unwrap();
        if !has_creation_times(dir.path()) {
            return;
        }
        let old = dir.path().join("before.log");
        append(&old, "from an earlier session\n");
        let baseline = past_the_clock_tick();
        let mut t = Tailer::new();
        t.refresh(dir.path(), is_log);
        let new = dir.path().join("during.log");
        append(&new, "first line of the session\n");
        append(&old, "appended this session\n");
        t.follow(dir.path(), is_log, baseline);
        let mut messages: Vec<_> = t.tick().into_iter().map(|l| l.line).collect();
        messages.sort();
        assert_eq!(
            messages,
            vec!["appended this session", "first line of the session"]
        );
        t.follow(dir.path(), is_log, baseline);
        assert!(t.tick().is_empty(), "nothing ships twice");
    }

    // Rotation by rename: the client's debug log renamed aside (and a new
    // one started), and an older non-log file renamed into the pattern.
    // Neither is new to the session, so neither ships its history; only
    // what is written after the rename goes.
    #[test]
    fn follow_never_resends_a_renamed_log_from_before_the_session() {
        let dir = tempfile::tempdir().unwrap();
        if !has_creation_times(dir.path()) {
            return;
        }
        let debug = dir.path().join("SGWDebugLog.log");
        append(&debug, &"earlier session history\n".repeat(50));
        append(&dir.path().join("archive.bak"), "an archived log\n");
        let baseline = past_the_clock_tick();
        let mut t = Tailer::new();
        t.refresh(dir.path(), is_log);
        assert!(t.tick().is_empty());

        let rotated = dir.path().join("SGWDebugLog_old.log");
        std::fs::rename(&debug, &rotated).unwrap();
        append(&rotated, "written to the old log after the rename\n");
        append(&debug, "the new log's first line\n");
        std::fs::rename(
            dir.path().join("archive.bak"),
            dir.path().join("archive.log"),
        )
        .unwrap();
        let mut messages = Vec::new();
        for _ in 0..3 {
            t.follow(dir.path(), is_log, baseline);
            messages.extend(t.tick().into_iter().map(|l| l.line));
        }
        messages.sort();
        assert_eq!(
            messages,
            vec![
                "the new log's first line",
                "written to the old log after the rename"
            ]
        );
    }

    // A large file that appeared during the session is read a capped amount
    // per tick, over several ticks, with every line exactly once.
    #[test]
    fn a_large_new_file_is_read_across_ticks_with_every_line_once() {
        let dir = tempfile::tempdir().unwrap();
        if !has_creation_times(dir.path()) {
            return;
        }
        let baseline = past_the_clock_tick();
        let mut t = Tailer::new();
        t.refresh(dir.path(), is_log);
        let expected: Vec<String> = (0..300_000).map(|n| format!("line {n:06}")).collect();
        let mut text = expected.join("\n");
        text.push('\n');
        assert!(text.len() as u64 > 3 * MAX_READ_PER_TICK);
        append(&dir.path().join("big.log"), &text);

        let mut got = Vec::new();
        let mut reads = 0;
        loop {
            t.follow(dir.path(), is_log, baseline);
            let lines = t.tick();
            if lines.is_empty() {
                break;
            }
            reads += 1;
            got.extend(lines.into_iter().map(|l| l.line));
        }
        assert!(reads >= 4, "read in {reads} ticks");
        assert_eq!(got, expected);
    }

    fn is_log(path: &Path) -> bool {
        path.extension().is_some_and(|e| e == "log")
    }

    /// `follow` needs creation times; a filesystem without them starts
    /// every file at its end, which these tests do not cover.
    fn has_creation_times(dir: &Path) -> bool {
        std::fs::metadata(dir).is_ok_and(|m| m.created().is_ok())
    }

    /// A baseline with a pause either side: a filesystem stamps creation
    /// times from a coarse clock, so a file made just before or after a
    /// precise `now()` could land on the wrong side of it.
    fn past_the_clock_tick() -> SystemTime {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let baseline = SystemTime::now();
        std::thread::sleep(std::time::Duration::from_millis(50));
        baseline
    }

    #[test]
    fn refresh_drops_files_that_no_longer_match() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.log");
        append(&path, "x\n");
        let mut t = Tailer::new();
        t.refresh(dir.path(), |p| p.extension().is_some_and(|e| e == "log"));
        assert_eq!(t.watched_count(), 1);
        std::fs::remove_file(&path).unwrap();
        t.refresh(dir.path(), |p| p.extension().is_some_and(|e| e == "log"));
        assert_eq!(t.watched_count(), 0);
    }
}
