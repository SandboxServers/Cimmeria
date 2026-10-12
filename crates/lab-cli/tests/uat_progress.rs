mod uat_support;

use cimmeria_lab_cli::commands::uat::golden::{GoldenGroup, GoldenSummary};
use cimmeria_lab_cli::commands::uat::progress::{
    cut, fancy_header, fancy_run_line, fancy_summary, fmt_duration, label_width, lane_notice,
    plain_header, progress_mode, quiet_line, rate_cell, write_line, ProgressMode,
};
use cimmeria_lab_cli::commands::uat::record::run_verdict;
use cimmeria_lab_cli::commands::uat::summary::{merge, HealEntry};
use cimmeria_lab_cli::ui::Ui;
use uat_support::{frost, good, ids, lane, lanes2, rec, row};

fn fancy_ui() -> Ui {
    Ui {
        color: false,
        fancy: true,
        width: None,
    }
}

#[test]
fn progress_goes_to_the_console_by_default() {
    // the removed PowerShell test file: 'progress goes to the console by default'
    assert_eq!(progress_mode(false, false, false), ProgressMode::Host);
}

#[test]
fn json_alone_silences_progress() {
    // the removed PowerShell test file: '-Json alone silences progress'
    assert_eq!(progress_mode(true, false, false), ProgressMode::None);
}

#[test]
fn quiet_silences_progress() {
    // the removed PowerShell test file: '-Quiet silences progress'
    assert_eq!(progress_mode(false, true, false), ProgressMode::None);
}

#[test]
fn progress_to_stderr_keeps_progress_under_json() {
    // the removed PowerShell test file: '-ProgressToStderr keeps progress under -Json, on stderr'
    assert_eq!(progress_mode(true, false, true), ProgressMode::Stderr);
}

#[test]
fn progress_to_stderr_wins_over_quiet() {
    // the removed PowerShell test file: '-ProgressToStderr wins over -Quiet'
    assert_eq!(progress_mode(true, true, true), ProgressMode::Stderr);
}

#[test]
fn stderr_mode_writes_the_line_to_stderr_and_nothing_to_stdout() {
    // the removed PowerShell test file: 'stderr mode writes the line to stderr, nothing to stdout'
    let (mut out, mut err) = (Vec::new(), Vec::new());
    write_line(
        ProgressMode::Stderr,
        "[default 1/5] PASS (90s)",
        &mut out,
        &mut err,
    );
    assert!(out.is_empty());
    assert_eq!(
        String::from_utf8(err).unwrap(),
        "[default 1/5] PASS (90s)\n"
    );
}

#[test]
fn host_mode_writes_to_stdout_and_none_mode_writes_nothing() {
    // the removed PowerShell test file: 'none mode writes nothing'
    let (mut out, mut err) = (Vec::new(), Vec::new());
    write_line(ProgressMode::Host, "hello", &mut out, &mut err);
    assert_eq!(String::from_utf8(out).unwrap(), "hello\n");
    assert!(err.is_empty());
    let (mut out, mut err) = (Vec::new(), Vec::new());
    write_line(ProgressMode::None, "hello", &mut out, &mut err);
    assert!(out.is_empty() && err.is_empty());
}

#[test]
fn the_plain_header_is_the_powershell_line() {
    assert_eq!(
        plain_header("first-session", &lanes2(), 3, &ids(&["FS-01", "FS-02"])),
        "lab uat first-session: 2 lease(s) x 3 run(s) = 6 run(s) on default, p2; rows FS-01, FS-02"
    );
}

#[test]
fn the_fancy_header_is_the_mock_up() {
    assert_eq!(
        fancy_header(
            &fancy_ui(),
            "first-session",
            2,
            2,
            3,
            "batch-20261011-013906"
        ),
        "▶ first-session · 2 rows · 2 lanes × 3 runs · batch-20261011-013906"
    );
    assert_eq!(
        fancy_header(&fancy_ui(), "s", 1, 1, 1, "batch-x"),
        "▶ s · 1 row · 1 lane × 1 run · batch-x"
    );
}

#[test]
fn durations_read_seconds_or_minutes_and_seconds() {
    assert_eq!(fmt_duration(0), "0s");
    assert_eq!(fmt_duration(59), "59s");
    assert_eq!(fmt_duration(60), "1m00s");
    assert_eq!(fmt_duration(62), "1m02s");
    assert_eq!(fmt_duration(252), "4m12s");
}

#[test]
fn cut_keeps_max_characters_ending_in_an_ellipsis() {
    assert_eq!(cut("abc", 3), "abc");
    assert_eq!(cut("abcdef", 4), "abc…");
}

#[test]
fn a_passing_run_reads_as_the_mock_up_line() {
    let mut r = rec(
        "default",
        1,
        true,
        vec![row("FS-01", "PASS", ""), row("FS-02", "PASS", "")],
        None,
    );
    r.seconds = 252;
    let width = label_width(&lanes2(), 3);
    assert_eq!(width, 11);
    assert_eq!(
        fancy_run_line(&fancy_ui(), &r, 3, width),
        "  default 1/3  ✓ pass  2/2 rows  4m12s"
    );
}

#[test]
fn a_failing_run_reads_as_the_mock_up_line_with_the_known_issue() {
    let rows = vec![row(
        "FS-02",
        "BLOCKED",
        "Login_PasswordEdit holds 0 characters after typing, expected 4",
    )];
    let mut r = rec("p2", 1, false, rows.clone(), None);
    r.verdict = Some(run_verdict(&rows, &[]));
    r.seconds = 62;
    assert_eq!(
        fancy_run_line(&fancy_ui(), &r, 3, 11),
        "  p2 1/3       ✗ fail  FS-02: Login_PasswordEdit holds 0 ch…  [#1343]  1m02s"
    );
}

#[test]
fn an_errored_run_reads_as_an_error_line() {
    let mut r = rec(
        "p2",
        2,
        false,
        vec![],
        Some("lab_uat_run failed: bridge down"),
    );
    r.seconds = 3;
    assert_eq!(
        fancy_run_line(&fancy_ui(), &r, 3, 11),
        "  p2 2/3       ✗ error  lab_uat_run failed: bridge down  3s"
    );
}

#[test]
fn a_lane_notice_is_plain_or_fancy() {
    let ui = fancy_ui();
    assert_eq!(
        lane_notice(&ui, false, "p2", "stopped after 3 failed runs in a row"),
        "[p2] stopped after 3 failed runs in a row"
    );
    assert_eq!(
        lane_notice(&ui, true, "p2", "stopped after 3 failed runs in a row"),
        "  ! p2  stopped after 3 failed runs in a row"
    );
}

#[test]
fn pass_rates_are_green_at_100_red_at_0_yellow_between_dim_with_no_runs() {
    let ui = Ui {
        color: true,
        fancy: true,
        width: None,
    };
    assert_eq!(rate_cell(&ui, 3, 3), "\x1b[32m3/3  100%\x1b[0m");
    assert_eq!(rate_cell(&ui, 0, 3), "\x1b[1;31m0/3  0%\x1b[0m");
    assert_eq!(rate_cell(&ui, 2, 3), "\x1b[33m2/3  66%\x1b[0m");
    assert_eq!(rate_cell(&ui, 0, 0), "\x1b[2m0/0  0%\x1b[0m");
}

#[test]
fn the_fancy_summary_is_a_lane_table_a_row_table_and_the_failure_groups() {
    let runs = vec![
        rec("default", 1, true, good(), None),
        rec("default", 2, true, good(), None),
        rec("p2", 1, false, frost(), None),
        rec("p2", 2, true, good(), None),
    ];
    let s = merge(&runs, &lanes2(), &[]);
    let text = fancy_summary(&fancy_ui(), &s, "lab uat first-session", "C:\\b");
    let expected = [
        "✗ lab uat first-session: FAIL (3/4 runs passed)",
        "Failed: 1 (1 known issue, 0 new)",
        "",
        "LANE  INSTANCE  STOPPED  PASSED",
        "1     default   -        2/2  100%",
        "2     p2        -        1/2  50%",
        "",
        "ROW    PASSED",
        "FS-P1  4/4  100%",
        "FS-P2  3/4  75%",
        "",
        "Failures, grouped by the first failing row:",
        "  1x FS-P2 FAIL: clause movie-over failed  [#1341]  runs: p2#1",
        "",
        "Batch: C:\\b",
        "",
    ]
    .join("\n");
    assert_eq!(text, expected);
}

#[test]
fn a_braked_lane_shows_yes_and_a_clean_batch_has_no_failure_block() {
    let runs = vec![rec("default", 1, true, good(), None)];
    let s = merge(&runs, &[lane(1, "default")], &["default".to_string()]);
    let text = fancy_summary(&fancy_ui(), &s, "t", "C:\\b");
    assert!(
        text.starts_with("✓ t: PASS (1/1 runs passed)\n\nLANE"),
        "{text}"
    );
    assert!(text.contains("yes"));
    assert!(!text.contains("Failed:") && !text.contains("Failures,"));
}

#[test]
fn the_quiet_line_names_the_passed_count_and_the_folder() {
    let s = merge(
        &[
            rec("default", 1, true, good(), None),
            rec("default", 2, false, frost(), None),
        ],
        &[lane(1, "default")],
        &[],
    );
    assert_eq!(
        quiet_line("lab uat first-session", &s, "C:\\b"),
        "lab uat first-session: 1/2 passed; C:\\b"
    );
}

fn colour_ui() -> Ui {
    Ui {
        color: true,
        fancy: true,
        width: None,
    }
}

#[test]
fn a_fancy_run_line_shows_the_golden_value_styled() {
    let mut r = rec("default", 1, true, good(), None);
    r.seconds = 45;
    r.golden = Some("match".to_string());
    assert_eq!(
        fancy_run_line(&fancy_ui(), &r, 3, 11),
        "  default 1/3  ✓ pass  2/2 rows  45s; golden: match"
    );
    // green for a match, yellow for a divergence, red for a diff that could not be made
    let mut line = |value: &str| {
        r.golden = Some(value.to_string());
        fancy_run_line(&colour_ui(), &r, 3, 11)
    };
    assert!(line("match").ends_with("\x1b[32m; golden: match\x1b[0m"));
    assert!(line("FS-P1 event 4: expected a, got b")
        .ends_with("\x1b[33m; golden: FS-P1 event 4: expected a, got b\x1b[0m"));
    assert!(line("error: no golden").ends_with("\x1b[1;31m; golden: error: no golden\x1b[0m"));
    // no value, no suffix
    r.golden = None;
    assert!(!fancy_run_line(&fancy_ui(), &r, 3, 11).contains("golden"));
}

fn group(line: &str, count: u32, runs: &[&str]) -> GoldenGroup {
    GoldenGroup {
        line: line.to_string(),
        count,
        runs: runs.iter().map(|r| r.to_string()).collect(),
    }
}

#[test]
fn the_fancy_summary_has_the_golden_chaos_divergence_and_heal_sections() {
    let runs = vec![rec("default", 1, true, good(), None)];
    let mut s = merge(&runs, &[lane(1, "default")], &[]);
    s.chaos = Some("chaos=cpu; chaos=flush:pack".to_string());
    s.golden = Some(GoldenSummary {
        matched: 2,
        diverged: 1,
        errors: 1,
        groups: vec![
            group("FS-P1 event 4: expected a, got b", 1, &["default#2"]),
            group("error: no golden", 1, &["default#3"]),
        ],
    });
    s.chaos_divergence = Some(GoldenSummary {
        matched: 0,
        diverged: 1,
        errors: 0,
        groups: vec![group(
            "FS-P2 event 1: expected c, got d",
            1,
            &["default#1 seed=7"],
        )],
    });
    s.heal = vec![HealEntry {
        row: "FS-P3".to_string(),
        pose: "frost".to_string(),
        diff: "C:\\b\\heal-FS-P3-frost.diff".to_string(),
    }];
    let text = fancy_summary(&fancy_ui(), &s, "lab uat first-session", "C:\\b");
    let expected = [
        "chaos=cpu; chaos=flush:pack",
        "✓ lab uat first-session: PASS (1/1 runs passed)",
        "",
        "LANE  INSTANCE  STOPPED  PASSED",
        "1     default   -        1/1  100%",
        "",
        "ROW    PASSED",
        "FS-P1  1/1  100%",
        "FS-P2  1/1  100%",
        "",
        "Golden: 2 match, 1 diverged, 1 could not be compared",
        "  1x FS-P1 event 4: expected a, got b  runs: default#2",
        "  1x error: no golden  runs: default#3",
        "",
        "Chaos divergence from the golden: 0 match, 1 diverged",
        "  1x FS-P2 event 1: expected c, got d  runs: default#1 seed=7",
        "",
        "Heal proposals (spec diffs to review; nothing was applied):",
        "  FS-P3 frost: C:\\b\\heal-FS-P3-frost.diff",
        "",
        "Batch: C:\\b",
        "",
    ]
    .join("\n");
    assert_eq!(text, expected);

    // the headings and the counts are styled
    let styled = fancy_summary(&colour_ui(), &s, "t", "C:\\b");
    assert!(styled.contains("\x1b[1;33mGolden\x1b[0m: \x1b[32m2 match\x1b[0m, \x1b[33m1 diverged\x1b[0m, \x1b[1;31m1 could not be compared\x1b[0m"), "{styled}");
    assert!(styled
        .contains("\x1b[1;33mHeal proposals (spec diffs to review; nothing was applied):\x1b[0m"));
    assert!(styled.contains("\x1b[36mC:\\b\\heal-FS-P3-frost.diff\x1b[0m"));
}

#[test]
fn a_clean_fancy_summary_has_none_of_those_sections() {
    let s = merge(
        &[rec("default", 1, true, good(), None)],
        &[lane(1, "default")],
        &[],
    );
    let text = fancy_summary(&fancy_ui(), &s, "t", "C:\\b");
    assert!(!text.contains("Golden") && !text.contains("Chaos") && !text.contains("Heal"));
}
