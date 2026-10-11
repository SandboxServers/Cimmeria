//! Runs the built `lab` binary and checks exit codes and output.

use std::process::{Command, Output};

/// Runs `lab` with `args` and an optional environment override.
fn run_lab(args: &[&str], env: &[(&str, &std::path::Path)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lab"));
    command.args(args);
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
fn help_for_a_name_not_in_clap_yet_goes_to_the_fallback() {
    // uat moves to clap in LX-02b, which replaces this test with `completions`.
    let empty = tempfile::TempDir::new().expect("temp dir");
    let output = run_lab(&["help", "uat"], &[("CIMMERIA_LAB_CLI_DIR", empty.path())]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("not installed"), "stderr: {stderr}");
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
fn status_without_dispatcher_exits_1_not_installed() {
    let empty = tempfile::TempDir::new().expect("temp dir");
    let output = run_lab(&["status"], &[("CIMMERIA_LAB_CLI_DIR", empty.path())]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("not installed"), "stderr: {stderr}");
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
fn powershell_spelling_is_accepted_before_the_fallback() {
    // -Lines 5 is rewritten to --lines 5, parses, then the fallback reports
    // the missing dispatcher (exit 1), not a usage error (exit 2).
    let empty = tempfile::TempDir::new().expect("temp dir");
    let output = run_lab(
        &["logs", "-Lines", "5"],
        &[("CIMMERIA_LAB_CLI_DIR", empty.path())],
    );
    assert_eq!(output.status.code(), Some(1));
}
