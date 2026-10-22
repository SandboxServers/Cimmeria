//! Unit tests for the launcher log line reader and the offsets file. Each test
//! runs in its own temp folder; no test reads the player's logs.
use super::*;
use std::io::Write;
use tempfile::TempDir;

const FILE: &str = "launcher.2026-10-22.log";
const HEAD: &str = "2026-10-22T12:00:00.000000Z";

/// Appends `text` to `name` in `dir`, as the shell's appender does.
fn append(dir: &Path, name: &str, text: &str) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(name))
        .unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

/// `collect` with the saved `offsets` and no cancel.
fn run(dir: &Path, offsets: &Offsets) -> Collected {
    collect(dir, offsets, &CancellationToken::new()).unwrap()
}

/// Sends `collected` the way the upload does: advance each line, then save.
fn send_all(dir: &Path, collected: &Collected) -> Offsets {
    let mut offsets = Offsets::load(dir);
    for line in &collected.lines {
        offsets.advance(line);
    }
    offsets.save(dir).unwrap();
    Offsets::load(dir)
}

fn messages(collected: &Collected) -> Vec<&str> {
    collected.lines.iter().map(|l| l.message.as_str()).collect()
}

#[test]
fn second_collect_after_advance_and_save_returns_only_new_lines() {
    let dir = TempDir::new().unwrap();
    let dir = dir.path();
    append(
        dir,
        FILE,
        &format!("{HEAD}  INFO a: one\n{HEAD}  INFO a: two\n"),
    );
    let first = run(dir, &Offsets::load(dir));
    assert_eq!(messages(&first), ["one", "two"]);
    let offsets = send_all(dir, &first);
    append(dir, FILE, &format!("{HEAD}  INFO a: three\n"));
    let second = run(dir, &offsets);
    assert_eq!(messages(&second), ["three"]);
    assert_eq!(second.lines_total, 1);
}

#[test]
fn half_written_line_waits_for_its_newline() {
    let dir = TempDir::new().unwrap();
    let dir = dir.path();
    append(
        dir,
        FILE,
        &format!("{HEAD}  INFO a: whole\n{HEAD}  INFO a: part"),
    );
    let first = run(dir, &Offsets::load(dir));
    assert_eq!(messages(&first), ["whole"]);
    let offsets = send_all(dir, &first);
    assert!(run(dir, &offsets).lines.is_empty());
    append(dir, FILE, "ial\n");
    assert_eq!(messages(&run(dir, &offsets)), ["partial"]);
}

#[test]
fn file_shorter_than_its_offset_restarts_at_zero() {
    let dir = TempDir::new().unwrap();
    let dir = dir.path();
    append(
        dir,
        FILE,
        &format!("{HEAD}  INFO a: old line one\n{HEAD}  INFO a: old line two\n"),
    );
    let offsets = send_all(dir, &run(dir, &Offsets::load(dir)));
    std::fs::write(dir.join(FILE), format!("{HEAD}  WARN a: new\n")).unwrap();
    let again = run(dir, &offsets);
    assert_eq!(messages(&again), ["new"]);
    assert_eq!(again.lines[0].level.as_deref(), Some("WARN"));
}

#[test]
fn continuation_line_folds_into_its_parent() {
    let dir = TempDir::new().unwrap();
    let dir = dir.path();
    let first_three = format!("{HEAD}  ERROR a: failed\n   at step two\nno timestamp here\n");
    append(dir, FILE, &format!("{first_three}{HEAD}  INFO a: next\n"));
    let lines = run(dir, &Offsets::load(dir)).lines;
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].message, "failed | at step two | no timestamp here");
    assert_eq!(lines[0].level.as_deref(), Some("ERROR"));
    assert_eq!(lines[0].line_no, 1);
    assert_eq!(lines[0].end, first_three.len() as u64);
    assert_eq!(lines[1].message, "next");
    assert_eq!(lines[1].line_no, 4);
}

#[test]
fn appender_line_sets_timestamp_level_and_target() {
    let dir = TempDir::new().unwrap();
    let dir = dir.path();
    append(
        dir,
        FILE,
        &format!("{HEAD}  WARN cimmeria_launcher_desktop::host: retry later\n"),
    );
    let lines = run(dir, &Offsets::load(dir)).lines;
    let expected = chrono::DateTime::parse_from_rfc3339(HEAD)
        .unwrap()
        .timestamp_millis();
    assert_eq!(lines[0].ts_ms, Some(expected));
    assert_eq!(lines[0].level.as_deref(), Some("WARN"));
    assert_eq!(
        lines[0].target.as_deref(),
        Some("cimmeria_launcher_desktop::host")
    );
    assert_eq!(lines[0].message, "retry later");
}

#[test]
fn twenty_five_thousand_lines_keep_the_newest_twenty_thousand() {
    let dir = TempDir::new().unwrap();
    let dir = dir.path();
    let mut text = String::new();
    for n in 0..25_000 {
        text.push_str(&format!("{HEAD}  INFO a: line {n}\n"));
    }
    append(dir, FILE, &text);
    let collected = run(dir, &Offsets::load(dir));
    assert_eq!(collected.lines_total, 25_000);
    assert_eq!(collected.lines.len(), MAX_LINES);
    assert_eq!(collected.lines[0].message, "line 5000");
    assert_eq!(collected.lines[0].line_no, 5_001);
}

#[test]
fn addresses_and_home_paths_are_scrubbed() {
    let dir = TempDir::new().unwrap();
    let dir = dir.path();
    append(
        dir,
        FILE,
        &format!(
            "{HEAD}  WARN a: connect 10.0.0.1:443 failed\n{HEAD}  INFO a: opened C:\\Users\\tester\\x.txt\n"
        ),
    );
    let lines = run(dir, &Offsets::load(dir)).lines;
    assert!(lines[0].message.contains("[ip]"));
    assert!(!lines[0].message.contains("10.0.0.1"));
    assert!(!lines[1].message.contains("tester"));
}

#[test]
fn older_file_is_read_first_and_a_long_message_is_cut() {
    let dir = TempDir::new().unwrap();
    let dir = dir.path();
    append(
        dir,
        "launcher.2026-10-21.log",
        &format!("{HEAD}  INFO a: older\n"),
    );
    append(dir, FILE, &format!("{HEAD}  INFO a: newer\n"));
    let long = "é".repeat(1_500);
    append(dir, FILE, &format!("{HEAD}  INFO a: {long}\n"));
    let collected = run(dir, &Offsets::load(dir));
    assert_eq!(collected.lines[0].file, "launcher.2026-10-21.log");
    assert_eq!(collected.lines[1].message, "newer");
    let cut = &collected.lines[2];
    assert!(cut.truncated);
    assert!(cut.message.len() <= MAX_MESSAGE_BYTES);
    assert!(cut.message.chars().all(|c| c == 'é'));
}

#[test]
fn cancel_and_a_missing_folder_are_handled() {
    let dir = TempDir::new().unwrap();
    let token = CancellationToken::new();
    token.cancel();
    assert!(matches!(
        collect(dir.path(), &Offsets::default(), &token),
        Err(LogUploadError::CollectFailed)
    ));
    let missing = dir.path().join("absent");
    let collected = run(&missing, &Offsets::default());
    assert_eq!(collected, Collected::default());
}

#[test]
fn offsets_round_trip_never_move_back_and_an_unreadable_file_loads_empty() {
    let dir = TempDir::new().unwrap();
    let dir = dir.path();
    let line = LogLine {
        file: FILE.to_string(),
        line_no: 1,
        end: 42,
        ts_ms: None,
        level: None,
        target: None,
        message: String::new(),
        truncated: false,
    };
    let mut offsets = Offsets::default();
    offsets.advance(&line);
    offsets.advance(&LogLine { end: 10, ..line });
    offsets.save(dir).unwrap();
    let loaded = Offsets::load(dir);
    assert_eq!(loaded.version, 2);
    assert_eq!(loaded.files.get(FILE), Some(&42));
    std::fs::write(dir.join("uploaded-log-offsets.json"), b"{not json").unwrap();
    assert!(Offsets::load(dir).files.is_empty());
}

#[test]
fn offsets_keep_only_the_newest_files() {
    use crate::storage::log_upload::offsets::MAX_FILES;
    let total = MAX_FILES + 88;
    let key = |n: usize| format!("launcher.2026-{:02}-{:02}.log", 1 + n / 28, 1 + n % 28);
    let mut offsets = Offsets::default();
    for n in 0..total {
        let line = LogLine {
            file: format!("launcher.2026-{:02}-{:02}.log", 1 + n / 28, 1 + n % 28),
            line_no: 1,
            end: 10,
            ts_ms: None,
            level: None,
            target: None,
            message: String::new(),
            truncated: false,
        };
        offsets.advance(&line);
    }
    // The limit is enforced after a read, not on each advance (A7).
    offsets.prune(
        &std::collections::HashMap::new(),
        &std::collections::HashSet::new(),
    );
    assert_eq!(offsets.files.len(), MAX_FILES);
    assert!(offsets.files.contains_key(&key(total - 1)));
    assert!(!offsets.files.contains_key(&key(0)));
}
