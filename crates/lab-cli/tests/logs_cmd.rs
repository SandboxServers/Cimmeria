//! `lab logs`: the filter, the tail, `--follow` and the colours. The first
//! twelve tests port the logs checks of tools/lab/cli/test-logs-env.ps1 (the
//! PowerShell check is named in each comment); the rest are new.

mod support;

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cimmeria_lab_cli::commands::logs::{follow_file, paint_line};
use cimmeria_lab_cli::log_filter::{filter_lines, tail_lines, LogFilter};
use cimmeria_lab_cli::ui::Ui;
use support::{lab_home_for, run_lab_in};

fn hex() -> String {
    "ab".repeat(32)
}

/// The six lines of the PowerShell fixture.
fn log_lines() -> Vec<String> {
    vec![
        r#"2026-10-10T20:00:00.000001Z  INFO serve: instance=Some("p2") started"#.to_string(),
        "2026-10-10T20:00:01.000001Z  WARN serve: instance=None idle".to_string(),
        r#"2026-10-10T20:00:02.000001Z DEBUG serve: instance="P2" bearer abc123def"#.to_string(),
        format!("2026-10-10T20:00:03.000001Z ERROR serve: key {}", hex()),
        r#"2026-10-10T20:00:04.000001Z  TRACE serve: instance="default" ping"#.to_string(),
        "continuation line with no level".to_string(),
    ]
}

fn panic_lines() -> Vec<&'static str> {
    vec![
        r#"2026-10-10T20:00:05.000001Z ERROR serve: instance=Some("p2") panicked"#,
        "   at error handling here",
        "   stack frame 2",
        r#"2026-10-10T20:00:06.000001Z  INFO serve: instance=Some("p3") fine"#,
        "   at error handling there",
    ]
}

fn run_filter(instance: Option<&str>, level: Option<&str>) -> Vec<String> {
    let lines = log_lines();
    filter_lines(lines.iter().map(String::as_str), instance, level)
}

// 'instance p2 keeps instance=Some("p2") and instance="P2" (ignoring case)'
#[test]
fn instance_keeps_both_spellings_ignoring_case() {
    assert_eq!(run_filter(Some("p2"), None).len(), 2);
}

// 'instance matches the whole name, not a prefix'
#[test]
fn instance_matches_the_whole_name() {
    assert!(run_filter(Some("p"), None).is_empty());
}

// 'level warn keeps WARN and ERROR only'
#[test]
fn level_warn_keeps_warn_and_error_only() {
    let kept = run_filter(None, Some("warn"));
    assert_eq!(kept.len(), 2);
    assert!(kept[0].contains("WARN") && kept[1].contains("ERROR"));
}

// 'level info keeps INFO, WARN and ERROR'
#[test]
fn level_info_keeps_info_warn_and_error() {
    assert_eq!(run_filter(None, Some("info")).len(), 3);
}

// 'instance and level filters combine'
#[test]
fn instance_and_level_combine() {
    assert_eq!(run_filter(Some("p2"), Some("info")).len(), 1);
}

// 'no filter keeps every line, including the continuation line'
#[test]
fn no_filter_keeps_every_line() {
    let kept = run_filter(None, None);
    assert_eq!(kept.len(), 6);
    assert_eq!(kept[5], "continuation line with no level");
}

// 'a kept ERROR entry keeps its continuation lines'
#[test]
fn a_kept_error_entry_keeps_its_continuation_lines() {
    let kept = filter_lines(panic_lines(), None, Some("error"));
    assert_eq!(kept.len(), 3);
    assert!(kept[2].contains("frame 2"));
}

// 'a continuation line whose second word is a level name is not ranked on its own'
#[test]
fn a_continuation_line_is_never_ranked_on_its_own() {
    let kept = filter_lines(panic_lines(), None, Some("error"));
    assert!(kept.iter().all(|line| !line.contains("there")));
}

// 'an -Instance match keeps the entry continuation lines'
#[test]
fn an_instance_match_keeps_the_entry_continuation_lines() {
    assert_eq!(filter_lines(panic_lines(), Some("p2"), None).len(), 3);
}

// 'a bearer token is masked'
#[test]
fn a_bearer_token_is_masked() {
    let kept = run_filter(None, None);
    assert_eq!(
        kept[2],
        r#"2026-10-10T20:00:02.000001Z DEBUG serve: instance="P2" <redacted>"#
    );
    assert!(!kept[2].contains("abc123def"));
}

// 'a 64-hex token is masked'
#[test]
fn a_64_hex_token_is_masked() {
    let kept = run_filter(None, None);
    assert_eq!(
        kept[3],
        "2026-10-10T20:00:03.000001Z ERROR serve: key <redacted>"
    );
    assert!(!kept[3].contains(&hex()));
}

// 'logs -Instance p2 -Lines 1 prints the last p2 line, masked' (library half)
#[test]
fn tail_keeps_the_last_matching_line_masked() {
    let text = log_lines().join("\n");
    let tail = tail_lines(&text, Some("p2"), None, 1);
    assert_eq!(
        tail,
        vec![r#"2026-10-10T20:00:02.000001Z DEBUG serve: instance="P2" <redacted>"#.to_string()]
    );
    assert!(tail_lines(&text, None, None, 0).is_empty());
    assert_eq!(tail_lines(&text, None, None, 100).len(), 6);
}

// 'logs -Instance p2 -Lines 1 prints the last p2 line, masked' (process half)
#[test]
fn binary_prints_the_last_p2_line_masked() {
    let home = lab_home_for("127.0.0.1:9");
    std::fs::write(
        home.path().join("labd.log"),
        log_lines().join("\r\n") + "\r\n",
    )
    .expect("write log");
    let output = run_lab_in(home.path(), &["logs", "-Instance", "p2", "-Lines", "1"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout,
        "2026-10-10T20:00:02.000001Z DEBUG serve: instance=\"P2\" <redacted>\n"
    );
}

// New: logs.ps1 exits 1 with 'no daemon log yet: <path>' on stderr.
#[test]
fn binary_without_a_log_exits_1_and_names_the_path() {
    let home = lab_home_for("127.0.0.1:9");
    let output = run_lab_in(home.path(), &["logs"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1));
    let want = format!(
        "no daemon log yet: {}",
        home.path().join("labd.log").display()
    );
    assert_eq!(stderr.trim_end(), want);
    assert!(output.stdout.is_empty());
}

// New: before the first entry, a continuation line has nothing to follow, so
// with a filter on it is dropped (PowerShell starts with $keep = $false).
#[test]
fn a_leading_continuation_line_is_dropped_when_a_filter_is_on() {
    let mut filter = LogFilter::new(None, Some("info"));
    assert_eq!(filter.accept("   at the top of a rotated file"), None);
    let mut open = LogFilter::new(None, None);
    assert!(open.accept("   at the top of a rotated file").is_some());
}

// New: the level comes from the second field only; an unlisted word drops the entry.
#[test]
fn an_entry_with_no_known_level_word_is_dropped_by_a_level_filter() {
    let mut filter = LogFilter::new(None, Some("trace"));
    assert_eq!(filter.accept("2026-10-10T20:00:00Z NOTICE serve: x"), None);
    assert!(filter
        .accept("2026-10-10T20:00:00Z trace serve: x")
        .is_some());
}

// ---- colours ----

fn colour_ui() -> Ui {
    Ui {
        color: true,
        fancy: true,
        width: None,
    }
}

#[test]
fn fancy_colours_the_level_word_and_dims_the_timestamp() {
    let ui = colour_ui();
    assert_eq!(
        paint_line(&ui, "2026-10-10T20:00:01.000001Z  WARN serve: x"),
        "\x1b[2m2026-10-10T20:00:01.000001Z\x1b[0m  \x1b[33mWARN\x1b[0m serve: x"
    );
    assert_eq!(
        paint_line(&ui, "2026-10-10T20:00:03.000001Z ERROR serve: x"),
        "\x1b[2m2026-10-10T20:00:03.000001Z\x1b[0m \x1b[1;31mERROR\x1b[0m serve: x"
    );
    assert_eq!(
        paint_line(&ui, "2026-10-10T20:00:02.000001Z DEBUG serve: x"),
        "\x1b[2m2026-10-10T20:00:02.000001Z\x1b[0m \x1b[2mDEBUG\x1b[0m serve: x"
    );
    assert_eq!(
        paint_line(&ui, "2026-10-10T20:00:00.000001Z  INFO serve: x"),
        "\x1b[2m2026-10-10T20:00:00.000001Z\x1b[0m  INFO serve: x"
    );
}

#[test]
fn continuation_lines_and_plain_mode_are_not_coloured() {
    assert_eq!(
        paint_line(&colour_ui(), "   at error handling"),
        "   at error handling"
    );
    let line = "2026-10-10T20:00:01.000001Z  WARN serve: x";
    assert_eq!(paint_line(&Ui::plain(), line), line);
}

// ---- --follow ----

#[derive(Clone)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().expect("buf lock").extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct Follow {
    stop: Arc<AtomicBool>,
    out: SharedBuf,
    handle: Option<JoinHandle<()>>,
}

impl Follow {
    fn start(
        path: PathBuf,
        instance: Option<&'static str>,
        level: Option<&'static str>,
        tail: usize,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let out = SharedBuf(Arc::new(Mutex::new(Vec::new())));
        let handle = {
            let stop = Arc::clone(&stop);
            let mut out = out.clone();
            std::thread::spawn(move || {
                let mut filter = LogFilter::new(instance, level);
                follow_file(
                    &path,
                    &Ui::plain(),
                    &mut filter,
                    tail,
                    &stop,
                    Duration::from_millis(20),
                    &mut out,
                )
                .expect("follow_file");
            })
        };
        Follow {
            stop,
            out,
            handle: Some(handle),
        }
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.out.0.lock().expect("buf lock")).into_owned()
    }

    fn wait_for(&self, needle: &str) {
        for _ in 0..250 {
            if self.text().contains(needle) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("timed out waiting for {needle:?}; saw {:?}", self.text());
    }

    /// Sets the stop flag (what Ctrl+C does) and returns the final output.
    fn finish(mut self) -> String {
        self.stop.store(true, Ordering::SeqCst);
        self.handle
            .take()
            .expect("handle")
            .join()
            .expect("follow thread");
        self.text()
    }
}

fn append(path: &std::path::Path, text: &str) {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .expect("open log");
    file.write_all(text.as_bytes()).expect("append");
}

// 'Get-Content -Tail N -Wait | Select-LabLogLine' (the tail is taken first,
// then filtered; new complete lines follow; a partial line waits for its newline)
#[test]
fn follow_prints_the_filtered_tail_then_new_complete_lines() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let path = dir.path().join("labd.log");
    std::fs::write(
        &path,
        "2026-10-10T20:00:00Z  INFO serve: a\n2026-10-10T20:00:01Z  WARN serve: b\n2026-10-10T20:00:02Z ERROR serve: c\n",
    )
    .expect("write log");
    let follow = Follow::start(path.clone(), None, Some("warn"), 50);
    follow.wait_for("ERROR serve: c");
    assert!(follow.text().contains("WARN serve: b"));
    assert!(!follow.text().contains("INFO serve: a"));

    append(
        &path,
        "2026-10-10T20:01:00Z ERROR serve: late\n2026-10-10T20:01:01Z  WARN serve: par",
    );
    follow.wait_for("ERROR serve: late");
    assert!(
        !follow.text().contains("serve: par"),
        "a line without its newline is held back: {}",
        follow.text()
    );
    append(&path, "tial\n2026-10-10T20:01:02Z  INFO serve: quiet\n");
    follow.wait_for("WARN serve: partial");
    let text = follow.finish();
    assert_eq!(text.matches("serve: late").count(), 1);
    assert!(!text.contains("quiet"), "INFO is below warn: {text}");
}

// New: --lines bounds the raw tail before the filter runs.
#[test]
fn follow_tail_counts_raw_lines() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let path = dir.path().join("labd.log");
    std::fs::write(
        &path,
        "2026-10-10T20:00:00Z ERROR serve: first\n2026-10-10T20:00:01Z  INFO serve: second\n",
    )
    .expect("write log");
    // Tail of 1 raw line is the INFO line; the error filter then drops it.
    let follow = Follow::start(path.clone(), None, Some("error"), 1);
    std::thread::sleep(Duration::from_millis(150));
    append(&path, "2026-10-10T20:00:02Z ERROR serve: third\n");
    follow.wait_for("serve: third");
    let text = follow.finish();
    assert!(!text.contains("serve: first"), "{text}");
}

// New: a file that shrinks (rotation) is read again from the top.
#[test]
fn follow_rereads_a_file_that_shrinks() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let path = dir.path().join("labd.log");
    std::fs::write(
        &path,
        "2026-10-10T20:00:00Z  INFO serve: one\n2026-10-10T20:00:01Z  INFO serve: two\n2026-10-10T20:00:02Z  INFO serve: three\n",
    )
    .expect("write log");
    let follow = Follow::start(path.clone(), None, None, 0);
    std::thread::sleep(Duration::from_millis(150));
    std::fs::write(&path, "2026-10-11T21:00:00Z  INFO serve: fresh\n").expect("rotate");
    follow.wait_for("serve: fresh");
    follow.finish();
}

// New: setting the stop flag (Ctrl+C) ends the loop with Ok.
#[test]
fn follow_stops_when_the_flag_is_set() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let path = dir.path().join("labd.log");
    std::fs::write(&path, "2026-10-10T20:00:00Z  INFO serve: a\n").expect("write log");
    let follow = Follow::start(path, None, None, 50);
    follow.wait_for("serve: a");
    let text = follow.finish();
    assert_eq!(text, "2026-10-10T20:00:00Z  INFO serve: a\n");
}
