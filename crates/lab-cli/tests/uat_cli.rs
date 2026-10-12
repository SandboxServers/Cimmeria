//! LX-15: `lab uat` through the real binary. Ports the retired test-uat-chaos.ps1's two
//! "reaches the daemon" guards, which ran the retired uat.ps1 with every daemon call stubbed.
//! Every call passes `--plan-only` (or `-PlanOnly`): a real run goes to the
//! PowerShell script until LX-16b, and a test must never start that.

mod support;

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use support::isolated_lab_home as fake_home;

/// Runs the binary with `home` as both the lab home and `%LOCALAPPDATA%`, so
/// no child reads the machine's real lab state or reaches a real daemon.
fn run_lab(args: &[&str], home: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lab"))
        .args(args)
        .env("CIMMERIA_LAB_HOME", home)
        .env("LOCALAPPDATA", home)
        .env("CIMMERIA_LAB_TEST_SGW_PIDS", "")
        .env_remove("CIMMERIA_LAB_DAEMON_TOKEN")
        .env_remove("NO_COLOR")
        .output()
        .expect("run lab")
}

fn last_json(output: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let line = stdout.lines().last().unwrap_or("").to_string();
    serde_json::from_str(&line).unwrap_or_else(|e| panic!("not JSON ({e}): {stdout:?}"))
}

// test-uat-chaos.ps1: '-Chaos cpu -Leases 2 exits 2 before the daemon is asked';
// the PowerShell spelling goes through argv::rewrite too
#[test]
fn chaos_cpu_with_two_leases_exits_2_in_either_spelling() {
    let home = fake_home();
    for args in [
        [
            "uat",
            "first-session",
            "--chaos",
            "cpu",
            "--leases",
            "2",
            "--plan-only",
            "--json",
        ],
        [
            "uat",
            "first-session",
            "-Chaos",
            "cpu",
            "-Leases",
            "2",
            "-PlanOnly",
            "-Json",
        ],
    ] {
        let o = run_lab(&args, home.path());
        let v = last_json(&o);
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        assert_eq!(
            (v["ok"].clone(), v["exit"].clone()),
            (Value::Bool(false), Value::from(2))
        );
        assert!(v["error"]
            .as_str()
            .unwrap()
            .starts_with("--chaos cpu loads the whole machine"));
    }
}

// new: no section is the usage line on stderr (plain), nothing on stdout
#[test]
fn no_section_prints_the_usage_on_stderr() {
    let home = fake_home();
    let o = run_lab(&["uat", "--plan-only"], home.path());
    assert_eq!(o.status.code(), Some(2));
    assert!(o.stdout.is_empty());
    assert!(String::from_utf8_lossy(&o.stderr).starts_with("usage: lab uat <section>"));
}

// new: a daemon that does not answer is exit 3 for --plan-only, as one JSON object
#[test]
fn plan_only_with_no_daemon_exits_3() {
    let home = fake_home();
    let o = run_lab(
        &["uat", "first-session", "--plan-only", "--json"],
        home.path(),
    );
    let v = last_json(&o);
    assert_eq!(o.status.code(), Some(3));
    assert_eq!(v["exit"], 3);
    assert_eq!(
        v["error"],
        "the lab daemon is not answering (lab status; lab start)"
    );
}

// new: a negative number is clap's usage error, exit 2, before the daemon
#[test]
fn a_negative_stagger_is_a_usage_error() {
    let home = fake_home();
    let o = run_lab(
        &["uat", "first-session", "--stagger-seconds=-5"],
        home.path(),
    );
    assert_eq!(o.status.code(), Some(2));
}

// a clap usage error under --json is the one JSON object on stdout, exit 2, in
// both spellings and for every command that has --json
#[test]
fn a_usage_error_under_json_is_one_json_object() {
    let home = fake_home();
    let cases: &[&[&str]] = &[
        &["uat", "first-session", "--fixtures", "Always", "--json"],
        &["uat", "first-session", "-Fixtures", "Always", "-Json"],
        &["uat", "first-session", "--leases", "-1", "--json"],
        &[
            "golden",
            "record",
            "first-session",
            "--runs",
            "many",
            "--json",
        ],
        &["calibrate", "--no-such-flag", "--json"],
        &["record", "--no-such-flag", "--json"],
        &["watch", "--no-such-flag", "--json"],
    ];
    for args in cases {
        let o = run_lab(args, home.path());
        assert_eq!(o.status.code(), Some(2), "{args:?}");
        let v = last_json(&o);
        assert_eq!(v["ok"], false, "{args:?}");
        assert_eq!(v["exit"], 2, "{args:?}");
        let error = v["error"].as_str().unwrap();
        assert!(
            !error.is_empty() && !error.contains('\n'),
            "{args:?}: {error}"
        );
        assert!(!error.starts_with("error:"), "{error}");
        assert_eq!(String::from_utf8_lossy(&o.stdout).lines().count(), 1);
        assert!(
            o.stderr.is_empty(),
            "{:?}",
            String::from_utf8_lossy(&o.stderr)
        );
    }
    let o = run_lab(
        &["uat", "first-session", "--fixtures", "Always", "--json"],
        home.path(),
    );
    assert!(last_json(&o)["error"].as_str().unwrap().contains("Always"));
}

// without --json the usage error stays clap's text on stderr, nothing on stdout
#[test]
fn a_usage_error_without_json_stays_text_on_stderr() {
    let home = fake_home();
    let o = run_lab(
        &["uat", "first-session", "--fixtures", "Always"],
        home.path(),
    );
    assert_eq!(o.status.code(), Some(2));
    assert!(o.stdout.is_empty());
    assert!(String::from_utf8_lossy(&o.stderr).contains("--fixtures"));
}

// --chaos takes one profile per flag: a comma list without '=' is exit 2
#[test]
fn a_chaos_list_is_a_usage_error() {
    let home = fake_home();
    let o = run_lab(
        &[
            "uat",
            "first-session",
            "--chaos",
            "cpu,flush:pack",
            "--plan-only",
            "--json",
        ],
        home.path(),
    );
    assert_eq!(o.status.code(), Some(2));
    assert_eq!(
        last_json(&o)["error"],
        "repeat --chaos for each profile (got 'cpu,flush:pack')"
    );
}
