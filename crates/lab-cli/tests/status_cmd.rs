//! `lab status`: lease and seed text, the plain and fancy renders, the
//! daemon-down listing and the binary's exit codes. The `Format-Ago` test
//! ports the seven checks of the retired tools/lab/cli/test-common.ps1; the rest are new.

mod support;

use cimmeria_lab_cli::commands::status::{
    daemon_line, format_ago, instance_labels, is_seeded, lease_text, render,
};
use cimmeria_lab_cli::daemon::env_file;
use cimmeria_lab_cli::daemon::status::{parse_status, StatusReply};
use cimmeria_lab_cli::ui::Ui;
use serde_json::{json, Value};
use support::{closed_bind, lab_home_for, run_lab_in, StubServer};

const UP: &str = r#"{"daemon":{"pid":41236,"version":"0.1.0","started_at":"2026-10-11T00:00:00Z","uptime_s":7380},
"instances":[
{"instance":"default","account":"lab1","bridge_port":8790,"client_pid":12044,
 "lease":{"held":false,"lease":null,"recent":[],"rules":{}},"uat":null},
{"instance":"p2","account":"lab2","bridge_port":8791,"client_pid":null,
 "lease":{"held":true,"lease":{"lease_id":"deadbeefdeadbeef","owner":"claude-a1","purpose":"FS UAT","remaining_s":720},"recent":[],"rules":{}},"uat":null}]}"#;

fn up_reply() -> StatusReply {
    let body: Value = serde_json::from_str(UP).expect("fixture json");
    parse_status(&body).expect("fixture shape")
}

/// A profile root where only `default` is seeded.
fn root_with_default_seeded() -> tempfile::TempDir {
    let root = tempfile::TempDir::new().expect("temp dir");
    std::fs::create_dir_all(
        root.path()
            .join("default/profile/Documents/My Games/Firesky/SGWGame"),
    )
    .expect("mkdir");
    root
}

fn fancy_no_colour() -> Ui {
    Ui {
        color: false,
        fancy: true,
        width: None,
    }
}

// 'Format-Ago 0 is 0s', '59 is 59s', '61 is 1m', '7380 is 2h 3m',
// '90 is 1m (truncates, not rounds)', '3599 is 59m', '5400 is 1h 30m'
#[test]
fn format_ago_matches_the_powershell_checks() {
    for (seconds, want) in [
        (0, "0s"),
        (59, "59s"),
        (61, "1m"),
        (7380, "2h 3m"),
        (90, "1m"),
        (3599, "59m"),
        (5400, "1h 30m"),
    ] {
        assert_eq!(format_ago(seconds), want, "{seconds} s");
    }
}

#[test]
fn lease_text_is_free_when_missing_or_not_held() {
    assert_eq!(lease_text(None, false), "free");
    let not_held = json!({"held": false, "lease": null});
    assert_eq!(lease_text(Some(&not_held), false), "free");
    assert_eq!(lease_text(Some(&not_held), true), "free");
}

#[test]
fn lease_text_names_owner_purpose_and_time_left_but_never_the_id() {
    let held = json!({"held": true, "lease": {
        "lease_id": "deadbeefdeadbeef", "owner": "claude-a1", "purpose": "FS UAT", "remaining_s": 720}});
    let plain = lease_text(Some(&held), false);
    let fancy = lease_text(Some(&held), true);
    assert_eq!(plain, "claude-a1 (FS UAT, 12m left)");
    assert_eq!(fancy, "claude-a1 (FS UAT) · 12m left");
    assert!(!plain.contains("deadbeef") && !fancy.contains("deadbeef"));
}

#[test]
fn lease_text_held_without_a_lease_object_is_held() {
    let held = json!({"held": true, "lease": null});
    assert_eq!(lease_text(Some(&held), false), "held");
}

#[test]
fn lease_text_missing_fields_are_dashes_and_negative_time_is_zero() {
    let held = json!({"held": true, "lease": {"remaining_s": 5}});
    assert_eq!(lease_text(Some(&held), false), "- (-, 5s left)");
    let past = json!({"held": true, "lease": {"owner": "o", "purpose": "p", "remaining_s": -3}});
    assert_eq!(lease_text(Some(&past), false), "o (p, 0s left)");
}

#[test]
fn seeded_means_the_sgwgame_folder_exists() {
    let root = root_with_default_seeded();
    assert!(is_seeded(root.path(), "default"));
    assert!(!is_seeded(root.path(), "p2"));
    // The profile folder alone is not a seed.
    std::fs::create_dir_all(root.path().join("p3/profile")).expect("mkdir");
    assert!(!is_seeded(root.path(), "p3"));
}

#[test]
fn plain_status_two_instances_one_leased() {
    let root = root_with_default_seeded();
    let (text, code) = render(&Ui::plain(), Some(&up_reply()), &[], root.path());
    let want = [
        "daemon: pid 41236, up 2h 3m, version 0.1.0",
        "",
        "INSTANCE ACCOUNT CLIENT PORT LEASE                        SEED",
        "-------- ------- ------ ---- -----                        ----",
        "default  lab1    12044  8790 free                         seeded",
        "p2       lab2    -      8791 claude-a1 (FS UAT, 12m left) -",
        "",
        "",
    ]
    .join("\n");
    assert_eq!(text, want);
    assert_eq!(code, 0);
}

#[test]
fn fancy_status_matches_the_mock_up() {
    let root = root_with_default_seeded();
    let (text, code) = render(&fancy_no_colour(), Some(&up_reply()), &[], root.path());
    let want = [
        "lab daemon  ● running  pid 41236 · up 2h 3m · 0.1.0",
        "",
        "INSTANCE  ACCOUNT  CLIENT  PORT  LEASE                          PROFILE",
        "default   lab1     12044   8790  free                           seeded",
        "p2        lab2     -       8791  claude-a1 (FS UAT) · 12m left  not seeded",
        "",
    ]
    .join("\n");
    assert_eq!(text, want);
    assert_eq!(code, 0);
}

#[test]
fn daemon_lines_in_both_modes() {
    let reply = up_reply();
    assert_eq!(
        daemon_line(&Ui::plain(), None),
        "daemon: not running (or status endpoint unavailable)"
    );
    assert_eq!(
        daemon_line(&fancy_no_colour(), None),
        "lab daemon  ○ not running  (or status endpoint unavailable)"
    );
    assert_eq!(
        daemon_line(&Ui::plain(), Some(&reply)),
        "daemon: pid 41236, up 2h 3m, version 0.1.0"
    );
}

#[test]
fn daemon_down_lists_the_labd_env_instances_with_the_seed_column_and_exits_1() {
    let root = root_with_default_seeded();
    let env = env_file::parse("CIMMERIA_LAB_INSTANCES=default, p2 ,\r\n");
    let (text, code) = render(&Ui::plain(), None, &env, root.path());
    let want = [
        "daemon: not running (or status endpoint unavailable)",
        "",
        "INSTANCE ACCOUNT CLIENT PORT LEASE SEED",
        "-------- ------- ------ ---- ----- ----",
        "default  -       -      -    -     seeded",
        "p2       -       -      -    -     -",
        "",
        "",
    ]
    .join("\n");
    assert_eq!(text, want);
    assert_eq!(code, 1);
}

#[test]
fn no_labd_env_instances_means_default() {
    assert_eq!(instance_labels(&[]), vec!["default".to_string()]);
    let blank = env_file::parse("CIMMERIA_LAB_INSTANCES= , ,\r\n");
    assert_eq!(instance_labels(&blank), vec!["default".to_string()]);
    let two = env_file::parse("cimmeria_lab_instances=default,p2\r\n");
    assert_eq!(
        instance_labels(&two),
        vec!["default".to_string(), "p2".to_string()]
    );
}

#[test]
fn binary_daemon_down_exits_1_and_lists_default() {
    let home = lab_home_for(&closed_bind());
    let output = run_lab_in(home.path(), &["status"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
    let want = [
        "daemon: not running (or status endpoint unavailable)",
        "",
        "INSTANCE ACCOUNT CLIENT PORT LEASE SEED",
        "-------- ------- ------ ---- ----- ----",
        "default  -       -      -    -     -",
        "",
        "",
    ]
    .join("\n");
    assert_eq!(stdout, want);
}

#[test]
fn binary_against_a_stub_daemon_prints_its_row_and_exits_0() {
    let body = r#"{"daemon":{"pid":4242,"version":"9.9.9","started_at":"2026-10-11T00:00:00Z","uptime_s":5},
"instances":[{"instance":"default","account":"lab1","bridge_port":8790,"client_pid":null,
"lease":{"held":false,"lease":null,"recent":[],"rules":{}},"uat":null}]}"#;
    let (server, endpoint) = StubServer::start(vec![("GET", "/status", 200, body)]);
    let bind = endpoint.base_url.trim_start_matches("http://").to_string();
    let home = lab_home_for(&bind);
    let output = run_lab_in(home.path(), &["status"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "stdout: {stdout}");
    let want = [
        "daemon: pid 4242, up 5s, version 9.9.9",
        "",
        "INSTANCE ACCOUNT CLIENT PORT LEASE SEED",
        "-------- ------- ------ ---- ----- ----",
        "default  lab1    -      8790 free  -",
        "",
        "",
    ]
    .join("\n");
    assert_eq!(stdout, want);
    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/status");
}
