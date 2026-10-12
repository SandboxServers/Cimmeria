//! `lab record` JSON line, the binary's exit codes, and the fancy output
//! (ports the removed PowerShell test file, "compact JSON" and "the script itself").

mod record_support;
mod support;

use std::process::Command;

use cimmeria_lab_cli::commands::record::action::{render, Outcome};
use cimmeria_lab_cli::commands::record::json::{compact_json, failure, Body};
use cimmeria_lab_cli::ui::Ui;
use serde_json::{json, Value};
use tempfile::TempDir;

use record_support::{args, fancy_ui, joined, ok, run, run_with, tool_err, ID, SPECS, UAT};

fn lab(dir: &TempDir, tokens: &[&str]) -> std::process::Output {
    // The home names a closed port: a run that did reach the daemon step
    // would find none, never the machine's real daemon.
    support::write_closed_port_pid(dir.path());
    Command::new(env!("CARGO_BIN_EXE_lab"))
        .args(tokens)
        .env("CIMMERIA_LAB_HOME", dir.path())
        .env_remove("NO_COLOR")
        .output()
        .expect("run lab")
}

// Check: 'nulls and empty strings are dropped, zeros kept'
#[test]
fn nulls_and_empty_strings_are_dropped_zeros_kept() {
    let body: Body = vec![
        ("ok".into(), json!(true)),
        ("draft".into(), json!("a.toml")),
        ("ended".into(), Value::Null),
        ("note".into(), json!("")),
        ("rows".into(), json!(0)),
    ];
    assert_eq!(
        compact_json(&body),
        r#"{"ok":true,"draft":"a.toml","rows":0}"#
    );
}

// Check: 'a long failure is cut under 400, shape kept'
#[test]
fn a_long_failure_is_cut_under_400_shape_kept() {
    let long = format!(r"C:\{}x", r"deep\".repeat(120));
    let line = compact_json(&failure(1, &long));
    assert!(line.chars().count() < 400, "{}", line.len());
    let back: Value = serde_json::from_str(&line).expect("json");
    assert_eq!(back["ok"], json!(false));
    assert_eq!(back["exit"], json!(1));
    assert!(back["error"]
        .as_str()
        .expect("error")
        .starts_with(r"C:\deep\"));
}

// Check: 'the failure shape is {ok, exit, error} in that order'
#[test]
fn the_failure_shape_is_ok_exit_error_in_that_order() {
    let line = compact_json(&failure(1, "boom"));
    assert!(line.starts_with(r#"{"ok":false,"exit":1,"error":""#));
}

// Check: 'a start answer stays under 400'
#[test]
fn a_start_answer_stays_under_400() {
    let dir = TempDir::new().expect("temp dir");
    let mut a = args("start", Some("first-session"));
    a.instance = Some("p2".into());
    let reply = ok(json!({"ok":true,"recording_id":ID,"instance":"p2","section":"first-session"}));
    let (o, _) = run(dir.path(), &a, vec![reply], true);
    assert!(compact_json(&o.body).chars().count() < 400);
}

// Check: 'record.ps1 -Json prints one failure line and exits 2'
#[test]
fn record_json_prints_one_failure_line_and_exits_2() {
    let dir = TempDir::new().expect("temp dir");
    let out = lab(&dir, &["record", "start", "Bad Section", "-Json"]);
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(lines.len(), 1, "{text}");
    let back: Value = serde_json::from_str(lines[0]).expect("json");
    assert_eq!(back["exit"], json!(2));
}

// Check: 'an unknown action exits 2'
#[test]
fn an_unknown_action_exits_2() {
    let dir = TempDir::new().expect("temp dir");
    let out = lab(&dir, &["record", "bogus"]);
    assert_eq!(out.status.code(), Some(2));
}

// Check: '-MaxMinutes abc exits 2 with one JSON line'
#[test]
fn max_minutes_abc_exits_2_with_one_json_line() {
    let dir = TempDir::new().expect("temp dir");
    let out = lab(
        &dir,
        &[
            "record",
            "start",
            "first-session",
            "-MaxMinutes",
            "abc",
            "-Json",
        ],
    );
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(lines.len(), 1, "{text}");
    let back: Value = serde_json::from_str(lines[0]).expect("json");
    assert_eq!(back["error"], json!("-MaxMinutes must be 1 to 30"));
}

// Extra: --json goes to stdout whatever the exit, lease ids hidden.
#[test]
fn json_goes_to_stdout_even_on_failure_with_lease_ids_hidden() {
    let dir = TempDir::new().expect("temp dir");
    let busy =
        tool_err(r#"{"ok":false,"exit":4,"error":"leased to lab_uat (lease-0123456789abcdef)"}"#);
    let (o, _) = run(dir.path(), &args("status", None), vec![busy], true);
    let out = render(&Ui::plain(), true, &o);
    assert!(!out.to_stderr);
    assert_eq!(
        out.text,
        r#"{"ok":false,"exit":4,"error":"leased to lab_uat (lease-<redacted>)"}"#
    );
}

// Extra: plain failures go to stderr, plain successes to stdout.
#[test]
fn plain_failure_goes_to_stderr_and_success_to_stdout() {
    let failed = Outcome {
        exit: 2,
        body: failure(2, "x"),
        text: "x".into(),
    };
    let out = render(&Ui::plain(), false, &failed);
    assert!(out.to_stderr);
    assert_eq!(out.text, "x");
    let fine = Outcome {
        exit: 0,
        body: Vec::new(),
        text: "fine".into(),
    };
    assert!(!render(&Ui::plain(), false, &fine).to_stderr);
}

// Extra: fancy failure paints the error label.
#[test]
fn fancy_failure_paints_the_error_label() {
    let failed = Outcome {
        exit: 2,
        body: failure(2, "x"),
        text: "x".into(),
    };
    let out = render(&fancy_ui(), false, &failed);
    assert!(out.to_stderr);
    assert_eq!(out.text, "\x1b[1;31merror:\x1b[0m x");
}

// Extra: fancy start line.
#[test]
fn fancy_start_prints_the_recording_line() {
    let dir = TempDir::new().expect("temp dir");
    let mut a = args("start", Some("first-session"));
    a.instance = Some("p2".into());
    a.max_minutes = Some("30".into());
    let reply = ok(json!({"ok":true,"recording_id":ID,"instance":"p2","section":"first-session"}));
    let (o, _) = run_with(&fancy_ui(), dir.path(), &a, vec![reply], true);
    assert_eq!(
        o.text,
        "\x1b[32m●\x1b[0m recording p2 · section first-session · stops by itself in 30m\n\x1b[2m  stop with: lab record stop -Instance p2\x1b[0m"
    );
}

// Extra: fancy status block and the empty status.
#[test]
fn fancy_status_prints_a_key_value_block() {
    let dir = TempDir::new().expect("temp dir");
    let mut a = args("status", None);
    a.instance = Some("p2".into());
    let reply = ok(
        json!({"ok":true,"recording":true,"recording_id":ID,"instance":"p2","section":"fs","age_s":75,"ticks":150,"anchors":2}),
    );
    let (o, _) = run_with(&fancy_ui(), dir.path(), &a, vec![reply], true);
    let dim = |key: &str| format!("\x1b[2m{key:<9}\x1b[0m");
    let want = format!(
        "\x1b[32m●\x1b[0m recording {ID}\n  {}  p2\n  {}  fs\n  {}  1m 15s\n  {}  150\n  {}  2",
        dim("instance"),
        dim("section"),
        dim("age"),
        dim("ticks"),
        dim("anchors")
    );
    assert_eq!(o.text, want);
    let reply = ok(json!({"ok":true,"recording":false,"instance":"default"}));
    let (o, _) = run_with(
        &fancy_ui(),
        dir.path(),
        &args("status", None),
        vec![reply],
        true,
    );
    assert_eq!(o.text, "\x1b[2m○ no recording on default\x1b[0m");
}

// Extra: fancy stop prints `✓ draft written: <path>` with the path in cyan.
#[test]
fn fancy_stop_prints_draft_written() {
    let dir = TempDir::new().expect("temp dir");
    record_support_save(dir.path());
    let mut a = args("stop", None);
    a.instance = Some("p2".into());
    let reply = ok(
        json!({"ok":true,"draft":"first-session-20261012-1430.toml","rows":3,"steps":9,"todo":1,"dropped":0,"raw":ID}),
    );
    let (o, _) = run_with(&fancy_ui(), dir.path(), &a, vec![reply], true);
    let draft = joined(SPECS, &["drafts", "first-session-20261012-1430.toml"]);
    let raw = joined(UAT, &["records", ID, "record.jsonl"]);
    assert_eq!(
        o.text,
        format!("\x1b[32m✓\x1b[0m draft written: \x1b[36m{draft}\x1b[0m\n\x1b[2m  3 row(s), 9 step(s), 1 TODO(s), 0 dropped\x1b[0m\n\x1b[2m  raw   {raw}\x1b[0m")
    );
}

// Extra: lease ids in a daemon's success answer are hidden in the line printed,
// with and without --json (the answer is not a refusal, so only `render` hides them).
#[test]
fn lease_ids_in_a_success_answer_are_hidden_when_printed() {
    let dir = TempDir::new().expect("temp dir");
    let reply = ok(
        json!({"ok":true,"recording":true,"recording_id":ID,"instance":"p2","section":"lease-0123456789abcdef","age_s":1,"ticks":2,"anchors":3}),
    );
    let (o, _) = run(dir.path(), &args("status", None), vec![reply], true);
    let plain = render(&Ui::plain(), false, &o);
    assert!(
        !plain.text.contains("lease-0123456789abcdef"),
        "{}",
        plain.text
    );
    assert!(plain.text.contains("lease-<redacted>"));
    let json = render(&Ui::plain(), true, &o);
    assert!(
        !json.text.contains("lease-0123456789abcdef"),
        "{}",
        json.text
    );
    assert!(json.text.contains("lease-<redacted>"));
}

// Extra: a success answer prints as the daemon's keys in the order it sent
// them (alphabetical), nulls and empty strings dropped, zeros kept.
#[test]
fn a_success_answer_prints_in_the_daemons_key_order() {
    let dir = TempDir::new().expect("temp dir");
    let reply: Value = serde_json::from_str(
        r#"{"ok":true,"recording_id":"r1","instance":"p2","section":"fs","ended":null,"note":"","ticks":0}"#,
    )
    .expect("json");
    let (o, _) = run(dir.path(), &args("status", None), vec![ok(reply)], true);
    assert_eq!(
        render(&Ui::plain(), true, &o).text,
        r#"{"instance":"p2","ok":true,"recording_id":"r1","section":"fs","ticks":0}"#
    );
}

// Extra: fancy status shows a fractional age as minutes and seconds, never blank.
#[test]
fn fancy_status_formats_a_fractional_age() {
    let dir = TempDir::new().expect("temp dir");
    let reply = ok(
        json!({"ok":true,"recording":true,"recording_id":ID,"instance":"p2","section":"fs","age_s":75.5,"ticks":150,"anchors":2}),
    );
    let (o, _) = run_with(
        &fancy_ui(),
        dir.path(),
        &args("status", None),
        vec![reply],
        true,
    );
    assert!(o.text.contains("1m 15s"), "{}", o.text);
}

// Extra: fancy stop with no draft and an early end.
#[test]
fn fancy_stop_with_no_draft_prints_the_tick_count_and_the_early_end() {
    let dir = TempDir::new().expect("temp dir");
    record_support_save(dir.path());
    let mut a = args("stop", None);
    a.instance = Some("p2".into());
    a.no_draft = true;
    let reply = ok(json!({"ok":true,"recording_id":ID,"ticks":4,"raw":ID,"ended":"lease_lost"}));
    let (o, _) = run_with(&fancy_ui(), dir.path(), &a, vec![reply], true);
    let raw = joined(UAT, &["records", ID, "record.jsonl"]);
    assert_eq!(
        o.text,
        format!("\x1b[32m✓\x1b[0m stopped; 4 tick(s), no draft\n\x1b[2m  raw   {raw}\x1b[0m\n\x1b[33m!\x1b[0m the recorder stopped early: lease_lost")
    );
}
fn record_support_save(home: &std::path::Path) {
    cimmeria_lab_cli::commands::record::state::save_state(
        &home.join("record-p2.json"),
        ID,
        "p2",
        SPECS,
    )
    .expect("save state");
}
