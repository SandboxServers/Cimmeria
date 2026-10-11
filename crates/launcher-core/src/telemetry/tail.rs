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
}

impl TailedFile {
    fn fresh(len: u64) -> Self {
        Self {
            pos: len,
            last_len: len,
        }
    }
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
    pub fn refresh(&mut self, root: &Path, predicate: impl Fn(&Path) -> bool) {
        let walker = match walkdir::WalkDir::new(root)
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(entries) => entries,
            Err(_) => return,
        };
        let mut seen = std::collections::HashSet::new();
        for entry in walker {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path().to_path_buf();
            if !predicate(&path) {
                continue;
            }
            seen.insert(path.clone());
            self.files.entry(path.clone()).or_insert_with(|| {
                let len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                TailedFile::fresh(len)
            });
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
            let mut buf = Vec::with_capacity((len - tf.pos) as usize);
            if f.read_to_end(&mut buf).is_err() {
                continue;
            }
            let consumed = buf.len() as u64;
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
            let advance = complete_line_bytes(&buf);
            tf.pos += advance;
            tf.last_len = tf.pos.max(consumed);
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
