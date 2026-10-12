//! `lab clients` as the built binary: the refusals that exit before the daemon
//! is asked, and the daemon-down exit. Ports the three `clients stop` checks of
//! the removed PowerShell test file. The lab home is a temp folder whose labd.pid
//! points at a closed port, so a command that did reach the daemon step would
//! find none (exit 1), never the real daemon.

mod support;

use std::process::{Command, Output};

use tempfile::TempDir;

fn lab(args: &[&str]) -> (Output, TempDir) {
    let home = support::isolated_lab_home();
    let output = Command::new(env!("CARGO_BIN_EXE_lab"))
        .args(args)
        .env("CIMMERIA_LAB_HOME", home.path())
        .env_remove("NO_COLOR")
        .output()
        .expect("the lab binary should start");
    (output, home)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

// ports: "'clients stop all -Force' exits 2" (PowerShell spelling, through argv::rewrite)
#[test]
fn stop_all_with_the_powershell_force_spelling_exits_2() {
    let (output, _home) = lab(&["clients", "stop", "all", "-Force"]);
    assert_eq!(output.status.code(), Some(2), "stderr: {}", stderr(&output));
    assert!(stderr(&output).contains("--force needs a named instance"));
}

#[test]
fn stop_all_with_force_exits_2() {
    let (output, _home) = lab(&["clients", "stop", "all", "--force"]);
    assert_eq!(output.status.code(), Some(2), "stderr: {}", stderr(&output));
}

// ports: "'clients stop -Force' with no instance exits 2"
#[test]
fn stop_with_force_and_no_instance_exits_2() {
    let (output, _home) = lab(&["clients", "stop", "-Force"]);
    assert_eq!(output.status.code(), Some(2), "stderr: {}", stderr(&output));
    assert!(stderr(&output).contains("--force needs a named instance"));
}

// ports: 'a verb other than stop exits 2'
#[test]
fn a_verb_other_than_stop_exits_2() {
    let (output, _home) = lab(&["clients", "start"]);
    assert_eq!(output.status.code(), Some(2), "stderr: {}", stderr(&output));
    assert!(stderr(&output).contains("usage: lab clients stop"));
}

#[test]
fn no_verb_exits_2() {
    let (output, _home) = lab(&["clients"]);
    assert_eq!(output.status.code(), Some(2), "stderr: {}", stderr(&output));
}

// new: the daemon-down exit, with nothing stopped
#[test]
fn stop_with_no_daemon_exits_1() {
    let (output, _home) = lab(&["clients", "stop", "p2"]);
    assert_eq!(output.status.code(), Some(1), "stderr: {}", stderr(&output));
    assert!(stderr(&output).contains("daemon not running"));
}
