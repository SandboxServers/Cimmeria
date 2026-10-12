//! Shared helpers for the `lab golden` tests: the golden fixture, a spec folder,
//! the built `lab` binary and the in-process `execute` runner.
#![allow(dead_code)]

use std::process::Command;

use cimmeria_lab_cli::commands::golden::{execute, Args, Deps};
use cimmeria_lab_cli::ui::Ui;
use serde_json::Value;
use tempfile::TempDir;

pub const SHA: &str = "1a2b3c4";

pub const GOLDEN: &str = r#"{"schema":1,"section":"first-session","rows":["FS-01","FS-P1"],"recorded_utc":"2026-10-11T10:00:00Z",
 "spec_sha256":"ab","build":{"server":"1a2b3c4","sgw_sha256":"cd"},
 "evidence":[{"run_id":"r1","run_dir":"d1"},{"run_id":"r2","run_dir":"d2"}],
 "fingerprint":{"schema":1,"section":"first-session","rows":[
   {"row":"FS-01","result":"PASS","tap":"full","events":["c2s a","s2c b"],"variance":{"late_bind_settle_ms":1000}},
   {"row":"FS-P1","result":"PASS","tap":"full","events":["c2s c"],"variance":{"late_bind_settle_ms":1000}}]}}"#;

/// A spec folder with an empty `golden/` inside.
pub fn specs_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir_all(dir.path().join("golden")).unwrap();
    dir
}

pub fn put_golden(specs: &TempDir, file: &str, text: &str) {
    std::fs::write(specs.path().join("golden").join(file), text).unwrap();
}

pub fn path_text(dir: &TempDir) -> String {
    dir.path().to_string_lossy().into_owned()
}

/// Runs the built `lab`: (exit code, stdout, stderr).
pub fn lab(home: &TempDir, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_lab"))
        .args(args)
        .env("CIMMERIA_LAB_HOME", home.path())
        .output()
        .expect("run lab");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

pub fn json_of(text: &str) -> Value {
    serde_json::from_str(text.trim()).unwrap_or_else(|e| panic!("not JSON ({e}): {text}"))
}

/// Arguments for one `golden` action on one section, as the CLI parses them.
pub fn args(action: &str, section: &str) -> Args {
    Args {
        action: Some(action.to_string()),
        section: Some(section.to_string()),
        rows: Vec::new(),
        runs: 5,
        leases: 1,
        server_version: Some(SHA.to_string()),
        specs_dir: None,
        rebless: false,
        json: false,
    }
}

/// Runs `execute` with a plain UI: (exit code, stdout, stderr).
pub fn run(a: &Args, deps: &Deps) -> (u8, String, String) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = execute(a, &Ui::plain(), deps, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}
