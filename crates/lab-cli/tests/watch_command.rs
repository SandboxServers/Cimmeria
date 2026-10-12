//! `lab watch` as a process: exit codes, one snapshot, usage errors. Ports the
//! 'command' checks of the removed PowerShell test file (each is named in a comment).

mod support;
mod watch_support;

use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;
use tempfile::TempDir;
use watch_support::{temp_fixtures, NEW_BATCH};

/// A temp lab home (labd.pid at a closed port) beside the fixtures' copy.
struct Rig {
    tmp: TempDir,
}

impl Rig {
    fn new() -> Rig {
        let tmp = temp_fixtures();
        fs::create_dir_all(tmp.path().join("home")).unwrap();
        fs::create_dir_all(tmp.path().join("no-batches")).unwrap();
        support::write_closed_port_pid(&tmp.path().join("home"));
        Rig { tmp }
    }

    fn p(&self, name: &str) -> String {
        self.tmp.path().join(name).to_string_lossy().into_owned()
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_lab"));
        c.arg("watch").args(args);
        c.env("CIMMERIA_LAB_HOME", self.p("home"))
            .env("CIMMERIA_LAB_UAT_DIR", self.p("no-batches"))
            .env("LOCALAPPDATA", self.p("local"))
            .env_remove("CIMMERIA_LAB_DAEMON_TOKEN");
        c
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("lab starts")
    }
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn lines(o: &Output) -> Vec<String> {
    out(o).lines().map(str::to_string).collect()
}

const LEASE: &str = "0123456789abcdef";

// '-Once -Json exits 0 with one JSON line'
#[test]
fn once_json_exits_0_with_one_json_line() {
    let rig = Rig::new();
    let o = rig.run(&[
        "--once",
        "--json",
        "--batch",
        &rig.p(NEW_BATCH),
        "--status-file",
        &rig.p("status-two-lanes.json"),
        "--log-file",
        &rig.p("labd-tail.log"),
    ]);
    assert_eq!(
        o.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    let l = lines(&o);
    assert_eq!(l.len(), 1, "{l:?}");
    assert!(serde_json::from_str::<Value>(&l[0]).is_ok());
}

// 'the JSON has a state of the five and lanes'
#[test]
fn the_json_has_a_state_of_the_five_and_lanes() {
    let rig = Rig::new();
    let o = rig.run(&[
        "--once",
        "--json",
        "--batch",
        &rig.p(NEW_BATCH),
        "--status-file",
        &rig.p("status-two-lanes.json"),
        "--log-file",
        &rig.p("labd-tail.log"),
    ]);
    let v: Value = serde_json::from_str(&lines(&o)[0]).unwrap();
    assert!(
        ["live", "done", "interrupted", "unknown", "none"].contains(&v["state"].as_str().unwrap())
    );
    assert!(v.get("lanes").is_some());
}

// 'the command's JSON is under 500 characters with no lease id'
#[test]
fn the_commands_json_is_under_500_characters_with_no_lease_id() {
    let rig = Rig::new();
    let o = rig.run(&[
        "--json",
        "--batch",
        &rig.p(NEW_BATCH),
        "--status-file",
        &rig.p("status-two-lanes.json"),
        "--log-file",
        &rig.p("labd-tail.log"),
    ]);
    let l = lines(&o);
    assert!(
        l.len() == 1 && l[0].len() < 500 && !l[0].contains(LEASE),
        "{l:?}"
    );
}

// '-Once prints the text with its INSTANCE header'
#[test]
fn once_prints_the_text_with_its_instance_header() {
    let rig = Rig::new();
    let o = rig.run(&[
        "--once",
        "--batch",
        &rig.p(NEW_BATCH),
        "--status-file",
        &rig.p("status-two-lanes.json"),
        "--log-file",
        &rig.p("labd-tail.log"),
    ]);
    assert_eq!(o.status.code(), Some(0));
    assert_eq!(
        lines(&o)
            .iter()
            .filter(|l| l.starts_with("INSTANCE "))
            .count(),
        1
    );
}

// '-Once text: 120 columns, no escape codes, no lease id'
#[test]
fn once_text_is_120_columns_with_no_escape_codes_and_no_lease_id() {
    let rig = Rig::new();
    let o = rig.run(&[
        "--once",
        "--batch",
        &rig.p(NEW_BATCH),
        "--status-file",
        &rig.p("status-two-lanes.json"),
        "--log-file",
        &rig.p("labd-tail.log"),
    ]);
    let text = out(&o);
    assert!(lines(&o).iter().all(|l| l.chars().count() <= 120));
    assert!(!text.contains('\u{1b}') && !text.contains(LEASE));
}

// '-IntervalSeconds 0 exits 2'
#[test]
fn interval_seconds_0_exits_2() {
    let rig = Rig::new();
    let o = rig.run(&[
        "--once",
        "--interval-seconds",
        "0",
        "--status-file",
        &rig.p("status-two-lanes.json"),
    ]);
    assert_eq!(o.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("--interval-seconds must be 1 to 60 (got 0)")
    );
}

// '-IntervalSeconds 61 -Json exits 2 with lab uat's error shape'
#[test]
fn interval_seconds_61_with_json_exits_2_with_the_error_shape() {
    let rig = Rig::new();
    let o = rig.run(&[
        "--json",
        "--interval-seconds",
        "61",
        "--status-file",
        &rig.p("status-two-lanes.json"),
    ]);
    let l = lines(&o);
    assert_eq!(o.status.code(), Some(2));
    assert!(
        l.len() == 1 && l[0].starts_with("{\"ok\":false,\"exit\":2,"),
        "{l:?}"
    );
}

// New: the PowerShell spelling still works (argv rewrite before clap).
#[test]
fn the_powershell_spelling_still_works() {
    let rig = Rig::new();
    let o = rig.run(&[
        "-Once",
        "-Json",
        "-IntervalSeconds",
        "61",
        "-StatusFile",
        &rig.p("status-two-lanes.json"),
    ]);
    assert_eq!(o.status.code(), Some(2));
    let o = rig.run(&[
        "-Once",
        "-Json",
        "-Batch",
        &rig.p(NEW_BATCH),
        "-StatusFile",
        &rig.p("status-two-lanes.json"),
        "-LogFile",
        &rig.p("labd-tail.log"),
    ]);
    assert_eq!(o.status.code(), Some(0));
}

// 'a -Batch folder with neither plan.json nor batch.jsonl exits 2'
#[test]
fn a_batch_folder_with_neither_plan_json_nor_batch_jsonl_exits_2() {
    let rig = Rig::new();
    fs::create_dir(rig.tmp.path().join("empty-batch")).unwrap();
    let o = rig.run(&[
        "--once",
        "--batch",
        &rig.p("empty-batch"),
        "--status-file",
        &rig.p("status-two-lanes.json"),
    ]);
    assert_eq!(o.status.code(), Some(2));
    let o = rig.run(&[
        "--once",
        "--batch",
        &rig.p("not-there"),
        "--status-file",
        &rig.p("status-two-lanes.json"),
    ]);
    assert_eq!(o.status.code(), Some(2));
}

// 'a missing -StatusFile exits 2'
#[test]
fn a_missing_status_file_exits_2() {
    let rig = Rig::new();
    let o = rig.run(&["--once", "--status-file", &rig.p("no-such-status.json")]);
    assert_eq!(o.status.code(), Some(2));
}

// New: a usage error never prints a lease id, even one in a path the user typed.
#[test]
fn a_usage_error_hides_a_lease_id_in_the_path() {
    let rig = Rig::new();
    let bad = rig.p("lease-0123456789abcdef");
    for json in [false, true] {
        let mut args = vec!["--once", "--batch", bad.as_str()];
        if json {
            args.push("--json");
        }
        let o = rig.run(&args);
        assert_eq!(o.status.code(), Some(2));
        let shown = format!("{}{}", out(&o), String::from_utf8_lossy(&o.stderr));
        assert!(!shown.contains(LEASE), "{shown}");
        assert!(shown.contains("lease-<redacted>"), "{shown}");
    }
}

// New: an empty --status-file, --batch or --log-file is not given (as in PowerShell),
// not a usage error.
#[test]
fn an_empty_path_argument_reads_as_not_given() {
    let rig = Rig::new();
    let o = rig.run(&[
        "--json",
        "--batch",
        "",
        "--status-file",
        "",
        "--log-file",
        "",
    ]);
    // The rig's daemon is closed and its UAT root holds no batch: down, no batch.
    assert_eq!(o.status.code(), Some(1), "{}", out(&o));
    assert_eq!(
        out(&o).trim_end(),
        "{\"ok\":true,\"daemon\":\"down\",\"state\":\"none\"}"
    );
}

// 'daemon down and no batch exits 1'
#[test]
fn daemon_down_and_no_batch_exits_1() {
    let rig = Rig::new();
    fs::write(rig.tmp.path().join("status-down.json"), "null").unwrap();
    let o = rig.run(&[
        "--json",
        "--status-file",
        &rig.p("status-down.json"),
        "--log-file",
        &rig.p("labd-tail.log"),
    ]);
    assert_eq!(o.status.code(), Some(1));
    assert_eq!(
        out(&o).trim_end(),
        "{\"ok\":true,\"daemon\":\"down\",\"state\":\"none\"}"
    );
}

// 'an unreadable labd.log exits 3 with the error shape' (Windows: a file held open with no sharing)
#[cfg(windows)]
#[test]
fn an_unreadable_labd_log_exits_3_with_the_error_shape() {
    use std::os::windows::fs::OpenOptionsExt;
    let rig = Rig::new();
    let log = rig.tmp.path().join("locked.log");
    fs::write(&log, "x\n").unwrap();
    let _held = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&log)
        .unwrap();
    let o = rig.run(&[
        "--json",
        "--batch",
        &rig.p(NEW_BATCH),
        "--status-file",
        &rig.p("status-two-lanes.json"),
        "--log-file",
        &rig.p("locked.log"),
    ]);
    assert_eq!(o.status.code(), Some(3), "{}", out(&o));
    assert!(
        out(&o).starts_with("{\"ok\":false,\"exit\":3,"),
        "{}",
        out(&o)
    );
}

// New (every OS): an unreadable batch.jsonl exits 3 with the error shape, not 1 or 0.
#[test]
fn an_unreadable_batch_jsonl_exits_3_with_the_error_shape() {
    let rig = Rig::new();
    let batch = rig.tmp.path().join("batch-unreadable");
    fs::create_dir(&batch).unwrap();
    fs::copy(
        rig.tmp.path().join(NEW_BATCH).join("plan.json"),
        batch.join("plan.json"),
    )
    .unwrap();
    fs::create_dir(batch.join("batch.jsonl")).unwrap();
    let o = rig.run(&[
        "--json",
        "--batch",
        &rig.p("batch-unreadable"),
        "--status-file",
        &rig.p("status-two-lanes.json"),
    ]);
    assert_eq!(o.status.code(), Some(3), "{}", out(&o));
    assert!(
        out(&o).starts_with("{\"ok\":false,\"exit\":3,\"error\":\"could not read the lab state: "),
        "{}",
        out(&o)
    );
}

// 'redirected output prints one snapshot and exits' (no --once; a 20 s guard fails
// the test instead of hanging the job if a live loop is ever entered)
#[test]
fn redirected_output_prints_one_snapshot_and_exits() {
    let rig = Rig::new();
    let mut child = rig
        .command(&[
            "--batch",
            &rig.p(NEW_BATCH),
            "--status-file",
            &rig.p("status-two-lanes.json"),
            "--log-file",
            &rig.p("labd-tail.log"),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("lab starts");
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if start.elapsed() > Duration::from_secs(20) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("still running after 20 s: it entered the live loop");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let o = child.wait_with_output().unwrap();
    assert_eq!(o.status.code(), Some(0));
    assert!(out(&o).contains("INSTANCE") && !out(&o).contains('\u{1b}'));
}

// New: without --batch the newest batch under the UAT root is shown.
#[test]
fn without_batch_the_newest_batch_under_the_uat_root_is_shown() {
    let rig = Rig::new();
    let o = rig
        .command(&[
            "--json",
            "--status-file",
            &rig.p("status-two-lanes.json"),
            "--log-file",
            &rig.p("labd-tail.log"),
        ])
        .env("CIMMERIA_LAB_UAT_DIR", rig.tmp.path()) // the fixtures' root holds both batch folders
        .output()
        .unwrap();
    let v: Value = serde_json::from_str(&lines(&o)[0]).unwrap();
    assert_eq!(v["batch"], NEW_BATCH);
}

// 'watch.ps1 calls no /mcp tool and takes no lease' (the command is read-only)
#[test]
fn the_command_calls_no_mcp_tool_and_takes_no_lease() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("commands")
        .join("watch");
    let banned = [
        "/mcp",
        "McpClient",
        "call_tool",
        "lab_uat_run",
        "lease_acquire",
        "lab_lease",
    ];
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let source = fs::read_to_string(&path).unwrap();
        for word in banned {
            assert!(!source.contains(word), "{} mentions {word}", path.display());
        }
    }
}

// 'the lab guide has no --once' (now: the guide's lab watch paragraph documents the --flag form)
#[test]
fn the_guide_documents_the_flag_form() {
    let guide = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("docs")
        .join("guides")
        .join("live-research-lab.md");
    let text = fs::read_to_string(guide).unwrap();
    let start = text
        .find("**`lab watch`.**")
        .expect("the lab watch paragraph");
    let end = text[start..]
        .find("**`lab calibrate`.**")
        .expect("the next paragraph")
        + start;
    let section = &text[start..end];
    for flag in [
        "--once",
        "--json",
        "--batch",
        "--interval-seconds",
        "--status-file",
        "--log-file",
    ] {
        assert!(
            section.contains(flag),
            "the guide's lab watch paragraph lacks {flag}"
        );
    }
    assert!(!section.contains("test-watch.ps1"));
}
