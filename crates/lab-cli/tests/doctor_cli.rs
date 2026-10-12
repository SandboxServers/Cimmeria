//! `lab doctor` end to end: the binary against an empty lab home.

mod support;

use std::process::Command;

// ops: the exit-1-on-FAIL rule and the plain PASS/WARN/FAIL lines, end to end.
#[test]
fn doctor_prints_ten_plain_lines_and_exits_1_when_a_check_fails() {
    // A closed port: the daemon is "down", whatever runs on this machine.
    let home = support::isolated_lab_home();
    let output = Command::new(env!("CARGO_BIN_EXE_lab"))
        .arg("doctor")
        .env("CIMMERIA_LAB_HOME", home.path())
        .env("LOCALAPPDATA", home.path())
        .env_remove("CIMMERIA_LAB_PROFILE_ROOT")
        .env_remove("CIMMERIA_LAB_DAEMON_TOKEN")
        .env_remove("NO_COLOR")
        .output()
        .expect("lab runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 10, "stdout: {stdout}");
    assert!(
        lines
            .iter()
            .all(|l| l.starts_with("PASS  ") || l.starts_with("WARN  ") || l.starts_with("FAIL  ")),
        "stdout: {stdout}"
    );
    assert!(
        lines.contains(
            &"FAIL  daemon answers /status  not running, or the status endpoint is down; run lab start"
        ),
        "stdout: {stdout}"
    );
    assert!(
        lines.contains(
            &"FAIL  labd.env has CIMMERIA_LAB_INSTALL_DIR and SGW.exe is under it  CIMMERIA_LAB_INSTALL_DIR is not set in labd.env"
        ),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.contains('\u{1b}'),
        "redirected output has no colour"
    );
    assert_eq!(output.status.code(), Some(1));
}
