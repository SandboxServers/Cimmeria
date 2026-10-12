//! The files `lab watch` reads: a saved `/status` and the tail of `labd.log`.

use std::fs;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use serde_json::Value;

use crate::daemon::status::has_status_shape;

/// How many log lines a snapshot looks at.
pub const LOG_TAIL_LINES: usize = 4000;

/// How far back from the end of the log the tail reads (2 MiB holds far more
/// than 4000 lines).
const TAIL_WINDOW: u64 = 2 * 1024 * 1024;

/// A saved `/status`, held to the shape the daemon sends: anything else (a
/// missing or unreadable file, `null`, not JSON, another object) reads as the
/// daemon down, as it would live.
pub fn read_status_file(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    let body: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    has_status_shape(&body).then_some(body)
}

/// The last `max_lines` lines of a file (it reads the last 2 MiB at most). An
/// error is an unreadable file (locked, denied).
pub fn read_tail_lines(path: &Path, max_lines: usize) -> io::Result<Vec<String>> {
    let mut file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(TAIL_WINDOW);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let text = String::from_utf8_lossy(&bytes);
    let mut lines: Vec<&str> = text.lines().collect();
    if start > 0 && !lines.is_empty() {
        lines.remove(0); // the first line of a window cut mid-file is partial
    }
    let skip = lines.len().saturating_sub(max_lines);
    Ok(lines[skip..].iter().map(|l| (*l).to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn a_status_file_without_the_daemons_shape_reads_as_down() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("s.json");
        for body in [
            "null",
            "{}",
            "not json",
            "[]",
            "{\"daemon\":null,\"instances\":[]}",
            "{\"daemon\":{}}",
        ] {
            fs::write(&p, body).unwrap();
            assert_eq!(read_status_file(&p), None, "{body}");
        }
        assert_eq!(read_status_file(&tmp.path().join("missing.json")), None);
    }

    #[test]
    fn a_status_file_with_the_daemons_shape_reads() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("s.json");
        fs::write(&p, "\u{feff}{\"daemon\":{\"pid\":1},\"instances\":[]}").unwrap();
        assert!(read_status_file(&p).is_some());
    }

    #[test]
    fn the_tail_keeps_the_last_n_lines() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("l.log");
        let text: String = (1..=10).map(|n| format!("line {n}\r\n")).collect();
        fs::write(&p, text).unwrap();
        assert_eq!(
            read_tail_lines(&p, 3).unwrap(),
            ["line 8", "line 9", "line 10"]
        );
        assert_eq!(read_tail_lines(&p, 50).unwrap().len(), 10);
    }

    #[test]
    fn a_missing_file_is_an_error() {
        let tmp = TempDir::new().unwrap();
        assert!(read_tail_lines(&tmp.path().join("nope.log"), 5).is_err());
    }

    #[test]
    fn a_big_file_is_read_from_its_end_and_drops_the_partial_first_line() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("big.log");
        let line = format!("{}\n", "x".repeat(99)); // 100 bytes
        let text = line.repeat(30_000); // 3,000,000 bytes, over the 2 MiB window
        fs::write(&p, &text).unwrap();
        let lines = read_tail_lines(&p, LOG_TAIL_LINES).unwrap();
        assert_eq!(lines.len(), LOG_TAIL_LINES);
        assert!(lines.iter().all(|l| l.len() == 99));
    }
}
