//! Every command's `--json` line is pure ASCII: a reason with an accent or an
//! emoji is written as `\uXXXX` escapes (a surrogate pair above U+FFFF), so a
//! PowerShell pipe on any console code page reads it back as it was.

mod golden_support;

use std::io;

use cimmeria_lab_cli::commands::calibrate::outcome::stop_json;
use cimmeria_lab_cli::commands::golden::Deps;
use cimmeria_lab_cli::commands::record::json::{compact_json as record_json, failure};
use cimmeria_lab_cli::commands::uat::json::compact_json as uat_json;
use cimmeria_lab_cli::commands::uat::summary::{FailureGroup, Summary};
use cimmeria_lab_cli::commands::watch::json::error_line;
use cimmeria_lab_cli::daemon::status::Endpoint;
use golden_support::{args, run, specs_dir};
use serde_json::Value;

const WHY: &str = "caf\u{e9} \u{1F600} fell over";

/// The line is ASCII, parses, and holds `WHY` (somewhere in a string) exactly.
fn assert_ascii_round_trip(line: &str, what: &str) {
    assert!(line.is_ascii(), "{what}: not ASCII: {line}");
    assert!(line.contains("\\u00e9"), "{what}: {line}");
    assert!(line.contains("\\ud83d\\ude00"), "{what}: {line}");
    let value: Value = serde_json::from_str(line.trim()).expect(what);
    let back = serde_json::to_string(&value).unwrap();
    assert!(back.contains(WHY), "{what}: {back}");
}

#[test]
fn uat_json_is_ascii() {
    let summary = Summary {
        runs: 1,
        failed: 1,
        failed_new: 1,
        failures: vec![FailureGroup {
            signature: format!("FS-P2 FAIL: {WHY}"),
            count: 1,
            issue: None,
            runs: vec!["default#1".into()],
        }],
        ..Default::default()
    };
    assert_ascii_round_trip(&uat_json(&summary, "C:\\b"), "uat");
}

#[test]
fn calibrate_json_is_ascii() {
    assert_ascii_round_trip(&stop_json(3, WHY), "calibrate");
}

#[test]
fn record_json_is_ascii() {
    assert_ascii_round_trip(&record_json(&failure(3, WHY)), "record");
}

#[test]
fn watch_json_is_ascii() {
    assert_ascii_round_trip(&error_line(3, WHY), "watch");
}

#[test]
fn golden_json_is_ascii() {
    let specs = specs_dir();
    let run_uat = |_: &[String]| -> io::Result<_> { Err(io::Error::other(WHY)) };
    let deps = Deps {
        run_uat: &run_uat,
        endpoint: Endpoint {
            base_url: "http://localhost:1".to_string(),
            token: None,
        },
        default_specs: Some(specs.path().to_path_buf()),
    };
    let mut a = args("record", "first-session");
    a.json = true;
    let (code, out, _) = run(&a, &deps);
    assert_eq!(code, 3, "{out}");
    assert_ascii_round_trip(&out, "golden");
}
