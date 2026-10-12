//! `lab version`: the daemon line in both modes and the binary's output.

mod support;

use cimmeria_lab_cli::commands::version::daemon_line;
use cimmeria_lab_cli::daemon::status::parse_status;
use cimmeria_lab_cli::ui::Ui;
use support::{closed_bind, lab_home_for, run_lab_in, StubServer};

const BODY: &str = r#"{"daemon":{"pid":4242,"version":"9.9.9","started_at":"2026-10-11T00:00:00Z","uptime_s":5},"instances":[]}"#;

#[test]
fn daemon_line_plain_is_the_powershell_text() {
    let reply = parse_status(&serde_json::from_str(BODY).unwrap()).expect("shape");
    assert_eq!(daemon_line(&Ui::plain(), Some(&reply)), "daemon 9.9.9");
    assert_eq!(daemon_line(&Ui::plain(), None), "daemon: not running");
}

#[test]
fn daemon_line_fancy_uses_the_glyphs() {
    let ui = Ui {
        color: false,
        fancy: true,
        width: None,
    };
    let reply = parse_status(&serde_json::from_str(BODY).unwrap()).expect("shape");
    assert_eq!(daemon_line(&ui, Some(&reply)), "daemon  ● 9.9.9");
    assert_eq!(daemon_line(&ui, None), "daemon  ○ not running");
}

#[test]
fn binary_prints_the_daemon_version_when_it_answers() {
    let (_server, endpoint) = StubServer::start(vec![("GET", "/status", 200, BODY)]);
    let bind = endpoint.base_url.trim_start_matches("http://").to_string();
    let home = lab_home_for(&bind);
    let output = run_lab_in(home.path(), &["version"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(lines.len(), 2, "stdout: {stdout}");
    assert!(lines[0].starts_with("lab "), "stdout: {stdout}");
    assert_eq!(lines[1], "daemon 9.9.9");
}

#[test]
fn binary_says_not_running_and_still_exits_0() {
    let home = lab_home_for(&closed_bind());
    let output = run_lab_in(home.path(), &["version"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(lines.len(), 2, "stdout: {stdout}");
    assert_eq!(lines[1], "daemon: not running");
}
