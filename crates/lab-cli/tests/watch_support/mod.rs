//! Fixtures and builders shared by the lab watch tests.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use cimmeria_lab_cli::batch::RunRecord;
use cimmeria_lab_cli::commands::watch::snapshot::{build, Inputs, InstanceView, Snapshot};
use serde_json::Value;
use tempfile::TempDir;

/// The live two-lane batch (3 records and a half-written 4th line).
pub const NEW_BATCH: &str = "batch-20261011-013906";
/// The batch from before plan.json (2 records, a `batch.json`, no plan).
pub const OLD_BATCH: &str = "batch-20261010-193906";
/// The text a half-written last line of NEW_BATCH's batch.jsonl starts with.
pub const HALF_LINE: &str = "{\"Lane\":2,\"Instance\":\"p2\",\"Ind";

/// The fixed clock of every snapshot test: 2026-10-11T01:41:23Z.
pub fn now() -> DateTime<Utc> {
    "2026-10-11T01:41:23Z".parse().expect("fixed clock")
}

pub fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("watch")
}

pub fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("mkdir");
    for entry in fs::read_dir(from).expect("read_dir") {
        let entry = entry.expect("entry");
        let dest = to.join(entry.file_name());
        if entry.file_type().expect("type").is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).expect("copy");
        }
    }
}

/// A temp copy of the fixtures, so a test may change them.
pub fn temp_fixtures() -> TempDir {
    let tmp = TempDir::new().expect("temp dir");
    copy_dir(&fixture_dir(), tmp.path());
    tmp
}

pub fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("read")).expect("json")
}

pub fn two_lanes(tmp: &Path) -> Value {
    read_json(&tmp.join("status-two-lanes.json"))
}

pub fn old_daemon(tmp: &Path) -> Value {
    read_json(&tmp.join("status-old-daemon.json"))
}

pub fn log_lines(tmp: &Path) -> Vec<String> {
    fs::read_to_string(tmp.join("labd-tail.log"))
        .expect("log")
        .lines()
        .map(str::to_string)
        .collect()
}

pub fn alive(_pid: u32) -> bool {
    true
}

pub fn dead(_pid: u32) -> bool {
    false
}

/// A snapshot at [`now`].
pub fn snap(
    status: Option<&Value>,
    batch: Option<&Path>,
    log: &[String],
    sgw: &[u32],
    is_alive: &dyn Fn(u32) -> bool,
) -> Snapshot {
    build(&Inputs {
        status,
        batch_dir: batch,
        log_lines: log,
        sgw_pids: sgw,
        now: now(),
        is_alive,
    })
    .expect("snapshot")
}

pub fn instance<'a>(s: &'a Snapshot, name: &str) -> &'a InstanceView {
    s.instances
        .iter()
        .find(|i| i.instance == name)
        .unwrap_or_else(|| panic!("no instance {name}"))
}

/// The instance names, joined with `,`.
pub fn names(s: &Snapshot) -> String {
    s.instances
        .iter()
        .map(|i| i.instance.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

/// `<tmp>/<name>`: NEW_BATCH's plan.json, and its batch.jsonl with the
/// half-written last line cut off and `extra` appended as one more record.
pub fn batch_with(tmp: &Path, name: &str, extra: &RunRecord) -> PathBuf {
    let dir = tmp.join(name);
    fs::create_dir_all(&dir).expect("mkdir");
    fs::copy(tmp.join(NEW_BATCH).join("plan.json"), dir.join("plan.json")).expect("plan");
    let text = fs::read_to_string(tmp.join(NEW_BATCH).join("batch.jsonl")).expect("jsonl");
    let cut = text.rfind(HALF_LINE).expect("the fixture's half line");
    let mut out = text[..cut].to_string();
    out.push_str(&serde_json::to_string(extra).expect("record"));
    out.push('\n');
    fs::write(dir.join("batch.jsonl"), out).expect("write");
    dir
}

/// p2's brake record of the PowerShell test: run 2 failed with an error, braked.
pub fn brake_record() -> RunRecord {
    RunRecord {
        lane: 2,
        instance: "p2".into(),
        index: 2,
        ok: false,
        error: Some("lab_uat_run failed: bridge down".into()),
        seconds: 3,
        braked: true,
        ..Default::default()
    }
}

/// p2's lane-death record: index 0, error `lane: bridge never came up`, braked.
pub fn death_record() -> RunRecord {
    RunRecord {
        lane: 2,
        instance: "p2".into(),
        index: 0,
        ok: false,
        error: Some("lane: bridge never came up".into()),
        seconds: 1,
        braked: true,
        ..Default::default()
    }
}
