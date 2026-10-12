//! Runs the built `lab` binary and checks exit codes and output.

mod support;

use std::process::{Command, Output};

/// Runs `lab` with `args` and an optional environment override. Every run gets
/// an isolated lab home (`CIMMERIA_LAB_HOME` and `LOCALAPPDATA`), so no run
/// reaches the machine's real lab daemon. `env` may override the home.
fn run_lab(args: &[&str], env: &[(&str, &std::path::Path)]) -> Output {
    let home = support::isolated_lab_home();
    let mut command = Command::new(env!("CARGO_BIN_EXE_lab"));
    command.args(args);
    command.env("CIMMERIA_LAB_HOME", home.path());
    command.env("LOCALAPPDATA", home.path());
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().expect("the lab binary should start")
}

#[test]
fn no_arguments_prints_usage_and_exits_0() {
    let output = run_lab(&[], &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("Usage"), "stdout: {stdout}");
}

#[test]
fn unknown_command_exits_2_with_suggestion() {
    let output = run_lab(&["stauts"], &[]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("did you mean"), "stderr: {stderr}");
}

#[test]
fn version_exits_0() {
    let output = run_lab(&["version"], &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("lab "), "stdout: {stdout}");
    assert!(!stdout.contains(''), "redirected output has no colour");
}

#[test]
fn version_with_a_stray_argument_is_a_usage_error() {
    assert_eq!(run_lab(&["version", "extra"], &[]).status.code(), Some(2));
}

#[test]
fn help_forms_print_the_root_help() {
    for args in [&["help"][..], &["--help"], &["-h"], &["-?"], &["/?"]] {
        let output = run_lab(args, &[]);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(0), "{args:?}");
        assert!(stdout.contains("Usage"), "{args:?}: {stdout}");
    }
}

#[test]
fn help_for_an_unknown_name_exits_2() {
    let output = run_lab(&["help", "stauts"], &[]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("unknown command 'stauts'"), "{stderr}");
    assert!(stderr.contains("did you mean"), "{stderr}");
}

#[test]
fn help_for_a_converted_name_prints_its_clap_page() {
    let output = run_lab(&["help", "status"], &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout.contains("Examples:"), "stdout: {stdout}");
    assert!(stdout.contains("lab status"), "stdout: {stdout}");
}

#[test]
fn help_for_completions_prints_its_clap_page() {
    let output = run_lab(&["help", "completions"], &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout.contains("Examples:"), "{stdout}");
    assert!(stdout.contains("powershell"), "{stdout}");
}

#[test]
fn unknown_command_without_a_near_name_hints_at_help_only() {
    let output = run_lab(&["zzzzzz"], &[]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2));
    assert!(!stderr.contains("did you mean"), "{stderr}");
    assert!(stderr.contains("run: lab help"), "{stderr}");
}

#[test]
fn command_help_flag_prints_examples_and_exits_0() {
    for (args, flag) in [
        (&["status", "--help"][..], "Examples:"),
        (&["logs", "-h"], "--lines"),
        (&["logs", "-?"], "--lines"),
        (&["clients", "--help"], "--force"),
    ] {
        let output = run_lab(args, &[]);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(0), "{args:?}");
        assert!(stdout.contains(flag), "{args:?}: {stdout}");
        assert!(
            !stdout.contains('\u{1b}'),
            "{args:?}: redirected help has no colour"
        );
    }
}

#[test]
fn unknown_flag_is_a_usage_error_exit_2() {
    assert_eq!(run_lab(&["status", "--bogus"], &[]).status.code(), Some(2));
    assert_eq!(
        run_lab(&["logs", "--level", "loud"], &[]).status.code(),
        Some(2)
    );
}

#[test]
fn powershell_spelling_is_accepted_by_a_native_command() {
    // -Lines 5 is rewritten to --lines 5 and parses (a bad spelling would
    // exit 2); logs then finds no log in the empty home and exits 1.
    let home = tempfile::TempDir::new().expect("temp dir");
    let output = run_lab(
        &["logs", "-Lines", "5"],
        &[("CIMMERIA_LAB_HOME", home.path())],
    );
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no daemon log yet"), "stderr: {stderr}");
}

#[test]
fn uat_help_pages_list_rows() {
    for args in [
        &["help", "uat"][..],
        &["uat", "-h"],
        &["uat", "--help"],
        &["uat", "-?"],
    ] {
        let output = run_lab(args, &[]);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(0), "{args:?}");
        assert!(stdout.contains("--rows"), "{args:?}: {stdout}");
    }
}

#[test]
fn uat_unknown_flag_exits_2() {
    assert_eq!(run_lab(&["uat", "--bogus"], &[]).status.code(), Some(2));
}

#[test]
fn every_converted_command_has_an_examples_section() {
    for name in [
        "uat",
        "watch",
        "record",
        "calibrate",
        "golden",
        "status",
        "start",
        "stop",
        "restart",
        "logs",
        "doctor",
        "instances",
        "clients",
        "env",
        "install",
        "setup",
        "local-setup",
        "completions",
    ] {
        let output = run_lab(&[name, "--help"], &[]);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(0), "{name}");
        assert!(stdout.contains("Examples:"), "{name}: {stdout}");
    }
}

/// PowerShell-style flags reach the native `uat`: `-Rows A,B -PlanOnly` is
/// parsed (not a usage error, exit 2). run_lab's home names the closed port 9,
/// so the plan finds no daemon and stops before anything is driven: exit 3
/// (pre-flight failure), not a run (0 or 1).
#[test]
fn powershell_spelling_of_uat_flags_reaches_the_native_command() {
    let output = run_lab(
        &["uat", "first-session", "-Rows", "FS-01,FS-02", "-PlanOnly"],
        &[],
    );
    assert_eq!(
        output.status.code(),
        Some(3),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `-Value:-x` reaches the native `env set` and the value lands in labd.env.
#[test]
fn powershell_value_colon_form_reaches_the_native_env_set() {
    let home = support::isolated_lab_home();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_lab"))
            .args(args)
            .env("CIMMERIA_LAB_HOME", home.path())
            .env("LOCALAPPDATA", home.path())
            .output()
            .expect("the lab binary should start")
    };
    let set = run(&["env", "set", "K", "-Value:-x"]);
    assert_eq!(
        set.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&set.stderr)
    );
    let got = run(&["env", "get", "K"]);
    assert_eq!(String::from_utf8_lossy(&got.stdout).trim(), "-x");
}
