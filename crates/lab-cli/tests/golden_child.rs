//! `lab golden record` must not leave its child `lab uat` running when Ctrl+C
//! ends it: it handles Ctrl+C itself and waits for the child. No console here, so
//! this checks what can be checked without one: the handler is installed (once)
//! and the child is awaited to its end with its output.

#![cfg(windows)]

use std::process::Command;
use std::time::{Duration, Instant};

use cimmeria_lab_cli::commands::golden::wait_for_child;
use cimmeria_lab_cli::commands::uat::ctrlc;

#[test]
fn golden_installs_the_handler_and_waits_for_the_child() {
    assert!(!ctrlc::installed());
    let mut child = Command::new("powershell.exe");
    child.args([
        "-NoProfile",
        "-Command",
        "Start-Sleep -Milliseconds 700; Write-Output '{\"ok\":true}'",
    ]);
    let started = Instant::now();
    let out = wait_for_child(child).expect("the child runs");
    assert!(
        started.elapsed() >= Duration::from_millis(600),
        "returned before the child ended"
    );
    assert_eq!(out.code, 0);
    assert!(out.stdout.contains("{\"ok\":true}"), "{:?}", out.stdout);
    assert!(ctrlc::installed(), "the handler was not installed");
    // a second wait adds nothing (installed once, no panic)
    let mut again = Command::new("powershell.exe");
    again.args(["-NoProfile", "-Command", "exit 7"]);
    assert_eq!(wait_for_child(again).unwrap().code, 7);
}
