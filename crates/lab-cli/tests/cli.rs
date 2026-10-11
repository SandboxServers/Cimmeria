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
}

#[test]
fn status_without_dispatcher_exits_1_not_installed() {
    let empty = tempfile::TempDir::new().expect("temp dir");
    let output = run_lab(&["status"], &[("CIMMERIA_LAB_CLI_DIR", empty.path())]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("not installed"), "stderr: {stderr}");
}
