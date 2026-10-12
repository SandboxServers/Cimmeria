//! LX-12b: `lab calibrate` through the real binary. Ports checks [45]-[47] of
//! tools/lab/cli/test-calibrate.ps1.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn fake_home() -> TempDir {
    let home = TempDir::new().unwrap();
    std::fs::write(
        home.path().join("labd.pid"),
        r#"{"pid":1,"bind":"127.0.0.1:9"}"#,
    )
    .unwrap();
    home
}

fn run_lab(args: &[&str], home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lab"))
        .args(args)
        .env("CIMMERIA_LAB_HOME", home)
        .env_remove("CIMMERIA_LAB_DAEMON_TOKEN")
        .env_remove("NO_COLOR")
        .output()
        .expect("run lab")
}

fn last_json(output: &Output) -> (String, Value) {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let line = stdout.lines().last().unwrap_or("").to_string();
    let value: Value =
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("not JSON ({e}): {stdout:?}"));
    (line, value)
}

// [45] 'no row exits 2 with one JSON object'
#[test]
fn no_row_exits_2_with_one_json_object() {
    let home = fake_home();
    let o = run_lab(&["calibrate", "first-session", "--json"], home.path());
    let (_, v) = last_json(&o);
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(
        (v["ok"].clone(), v["exit"].clone()),
        (Value::Bool(false), Value::from(2))
    );
    assert!(v["error"]
        .as_str()
        .unwrap()
        .starts_with("usage: lab calibrate"));
}

// [46] '-BudgetMinutes 31 exits 2' (the PowerShell spelling goes through argv::rewrite too)
#[test]
fn a_budget_of_31_exits_2() {
    let home = fake_home();
    for args in [
        [
            "calibrate",
            "first-session",
            "FS-P3",
            "--budget-minutes",
            "31",
            "--json",
        ],
        [
            "calibrate",
            "first-session",
            "FS-P3",
            "-BudgetMinutes",
            "31",
            "-Json",
        ],
    ] {
        let o = run_lab(&args, home.path());
        let (_, v) = last_json(&o);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert_eq!(v["error"], "--budget-minutes must be 1 to 30");
    }
}

// [47] '-Apply -Json passes the argument checks and stops at the pre-flight'
#[test]
fn apply_with_json_passes_the_argument_checks_and_stops_at_the_pre_flight() {
    let home = fake_home();
    let o = run_lab(
        &["calibrate", "no-such-section", "FS-P3", "--apply", "--json"],
        home.path(),
    );
    let (line, v) = last_json(&o);
    assert_eq!(o.status.code(), Some(3));
    assert_eq!(v["exit"], 3);
    assert!(v["error"]
        .as_str()
        .unwrap()
        .contains("daemon is not answering"));
    assert!(line.chars().count() < 400);
}

// New: plain mode prints the PowerShell text on stderr, nothing on stdout.
#[test]
fn a_plain_usage_error_goes_to_stderr() {
    let home = fake_home();
    let o = run_lab(&["calibrate", "first-session"], home.path());
    assert_eq!(o.status.code(), Some(2));
    assert!(o.stdout.is_empty());
    assert!(String::from_utf8_lossy(&o.stderr).starts_with("usage: lab calibrate"));
}
