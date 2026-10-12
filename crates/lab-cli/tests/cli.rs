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
fn help_for_completions_goes_to_the_fallback() {
    // completions has no clap subcommand until LX-18.
    let empty = tempfile::TempDir::new().expect("temp dir");
    let output = run_lab(
        &["help", "completions"],
        &[("CIMMERIA_LAB_CLI_DIR", empty.path())],
    );
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
    ] {
        let output = run_lab(&[name, "--help"], &[]);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(output.status.code(), Some(0), "{name}");
        assert!(stdout.contains("Examples:"), "{name}: {stdout}");
    }
}

/// Runs `lab` against a stand-in `lab.ps1` that prints the tokens it was given,
/// joined with `|`. `None` when `pwsh` is not on the PATH.
fn tokens_the_fallback_receives(args: &[&str]) -> Option<String> {
    Command::new("pwsh").arg("-Version").output().ok()?;
    let dir = tempfile::TempDir::new().expect("temp dir");
    std::fs::write(
        dir.path().join("lab.ps1"),
        "[Console]::Out.WriteLine(($args -join '|'))\n",
    )
    .expect("write the stand-in dispatcher");
    let output = run_lab(args, &[("CIMMERIA_LAB_CLI_DIR", dir.path())]);
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// clap parses the tokens as either spelling, but the PowerShell dispatcher
/// gets them in its own form: `--lines` and `-Lines` both arrive as `-Lines`,
/// and `--runs-per-lease` as `-RunsPerLease`.
#[test]
fn the_fallback_receives_the_original_tokens() {
    // `instances` stays on the fallback until LX-18 (`logs` is native now).
    let Some(seen) = tokens_the_fallback_receives(&[
        "instances",
        "status",
        "-Lines",
        "5",
        "--runs-per-lease",
        "5",
    ]) else {
        eprintln!("pwsh is not on the PATH; skipped");
        return;
    };
    assert_eq!(seen, "instances|status|-Lines|5|-RunsPerLease|5");
    let seen = tokens_the_fallback_receives(&[
        "instances",
        "status",
        "--lines",
        "5",
        "-RunsPerLease",
        "5",
    ])
    .expect("pwsh");
    assert_eq!(seen, "instances|status|-Lines|5|-RunsPerLease|5");
}

/// `completions` has no clap subcommand: it reaches the dispatcher with its
/// tokens in PowerShell form, and `lab help completions` asks it for `--help`
/// (sent as `-Help`).
#[test]
fn completions_goes_to_the_fallback_with_its_tokens() {
    let Some(seen) = tokens_the_fallback_receives(&["completions", "bash"]) else {
        eprintln!("pwsh is not on the PATH; skipped");
        return;
    };
    assert_eq!(seen, "completions|bash");
    let seen = tokens_the_fallback_receives(&["completions", "--shell", "bash"]).expect("pwsh");
    assert_eq!(seen, "completions|-Shell|bash");
    let seen = tokens_the_fallback_receives(&["help", "completions"]).expect("pwsh");
    assert_eq!(seen, "completions|-Help");
}

#[test]
fn powershell_spelling_of_uat_flags_is_accepted() {
    let empty = tempfile::TempDir::new().expect("temp dir");
    let output = run_lab(
        &["uat", "first-session", "-Rows", "FS-01,FS-02", "-PlanOnly"],
        &[("CIMMERIA_LAB_CLI_DIR", empty.path())],
    );
    // Parsed, then the fallback finds no dispatcher: exit 1, not a usage error.
    assert_eq!(output.status.code(), Some(1));
}
