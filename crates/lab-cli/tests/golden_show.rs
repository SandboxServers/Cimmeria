//! `lab golden show` through the built binary and in-process, plus the
//! argument-parsing checks shared by `golden record` (exit codes before anything
//! runs, and the PowerShell spelling reaching the native command).

mod golden_support;
mod support;

use std::io;

use cimmeria_lab_cli::commands::golden::{ChildOutput, Deps};
use golden_support::{args, json_of, lab, path_text, put_golden, run, specs_dir, GOLDEN, SHA};
use serde_json::json;
use support::closed_port_endpoint;
use tempfile::TempDir;

// PS L172
#[test]
fn runs_3_exits_2_before_anything_runs() {
    let (home, specs) = (TempDir::new().unwrap(), specs_dir());
    let sp = path_text(&specs);
    let (code, out, _) = lab(
        &home,
        &[
            "golden",
            "record",
            "first-session",
            "--runs",
            "3",
            "--server-version",
            SHA,
            "--specs-dir",
            sp.as_str(),
            "--json",
        ],
    );
    assert_eq!(code, 2);
    assert!(json_of(&out)["error"]
        .as_str()
        .unwrap()
        .starts_with("--runs must be 5"));
}

// PS L174
#[test]
fn record_without_server_version_exits_2() {
    let (home, specs) = (TempDir::new().unwrap(), specs_dir());
    let sp = path_text(&specs);
    let (code, out, _) = lab(
        &home,
        &[
            "golden",
            "record",
            "first-session",
            "--specs-dir",
            sp.as_str(),
            "--json",
        ],
    );
    assert_eq!(code, 2);
    assert!(json_of(&out)["error"]
        .as_str()
        .unwrap()
        .starts_with("--server-version"));
}

// PS L176
#[test]
fn show_without_a_golden_exits_2() {
    let (home, specs) = (TempDir::new().unwrap(), specs_dir());
    let sp = path_text(&specs);
    let (code, out, _) = lab(
        &home,
        &[
            "golden",
            "show",
            "first-session",
            "--specs-dir",
            sp.as_str(),
            "--json",
        ],
    );
    assert_eq!(code, 2);
    assert!(json_of(&out)["error"]
        .as_str()
        .unwrap()
        .starts_with("no golden for first-session"));
}

// PS L182
#[test]
fn show_prints_the_committed_golden_as_json() {
    let (home, specs) = (TempDir::new().unwrap(), specs_dir());
    put_golden(&specs, "first-session.json", GOLDEN);
    let sp = path_text(&specs);
    let (code, out, _) = lab(
        &home,
        &[
            "golden",
            "show",
            "first-session",
            "--specs-dir",
            sp.as_str(),
            "--json",
        ],
    );
    let shown = json_of(&out);
    assert_eq!(code, 0);
    assert_eq!(shown["ok"], json!(true));
    assert_eq!(shown["events"], json!(3));
    assert_eq!(shown["runs"], json!(["r1", "r2"]));
}

// PS L183
#[test]
fn show_json_has_no_nulls() {
    let (home, specs) = (TempDir::new().unwrap(), specs_dir());
    put_golden(&specs, "first-session.json", GOLDEN);
    let sp = path_text(&specs);
    let (_, out, _) = lab(
        &home,
        &[
            "golden",
            "show",
            "first-session",
            "--specs-dir",
            sp.as_str(),
            "--json",
        ],
    );
    assert!(!out.contains("null"), "{out}");
}

// PS L187
#[test]
fn show_v1_2_reads_golden_v1_2_json() {
    let (home, specs) = (TempDir::new().unwrap(), specs_dir());
    put_golden(&specs, "v1_2.json", GOLDEN);
    let sp = path_text(&specs);
    let (code, out, _) = lab(
        &home,
        &[
            "golden",
            "show",
            "v1.2",
            "--specs-dir",
            sp.as_str(),
            "--json",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(json_of(&out)["ok"], json!(true));
}

// PS L194
#[test]
fn an_unreadable_golden_exits_7_not_1() {
    let (home, specs) = (TempDir::new().unwrap(), specs_dir());
    put_golden(&specs, "broken.json", "{ not json");
    let sp = path_text(&specs);
    let (code, out, _) = lab(
        &home,
        &[
            "golden",
            "show",
            "broken",
            "--specs-dir",
            sp.as_str(),
            "--json",
        ],
    );
    let o = json_of(&out);
    assert_eq!(code, 7);
    assert_eq!(o["exit"], json!(7));
    assert!(o["error"].as_str().unwrap().starts_with("cannot read"));
}

// New: the PowerShell spelling reaches the native command through argv::rewrite.
#[test]
fn the_powershell_spelling_is_accepted() {
    let (home, specs) = (TempDir::new().unwrap(), specs_dir());
    let sp = path_text(&specs);
    let (code, out, _) = lab(
        &home,
        &[
            "golden",
            "record",
            "first-session",
            "-Runs",
            "3",
            "-ServerVersion",
            SHA,
            "-SpecsDir",
            sp.as_str(),
            "-Json",
        ],
    );
    assert_eq!(code, 2);
    assert!(json_of(&out)["error"]
        .as_str()
        .unwrap()
        .starts_with("--runs must be 5"));
}

// New: plain `show` text through the binary.
#[test]
fn show_prints_the_plain_block() {
    let (home, specs) = (TempDir::new().unwrap(), specs_dir());
    put_golden(&specs, "first-session.json", GOLDEN);
    let sp = path_text(&specs);
    let (code, out, _) = lab(
        &home,
        &[
            "golden",
            "show",
            "first-session",
            "--specs-dir",
            sp.as_str(),
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "golden first-session: 2 rows, 3 events\nrows:   FS-01, FS-P1\nserver: 1a2b3c4\nsgw:    cd\nwhen:   2026-10-11T10:00:00Z\nruns:   r1, r2\n"
    );
}

#[test]
fn show_json_goes_to_stdout_and_a_missing_spec_folder_is_exit_2() {
    let runner = |_: &[String]| -> io::Result<ChildOutput> { unreachable!("show never runs uat") };
    let d = Deps {
        run_uat: &runner,
        endpoint: closed_port_endpoint(),
        default_specs: None,
    };
    let mut a = args("show", "first-session");
    a.json = true;
    let (code, out, err) = run(&a, &d);
    assert_eq!(code, 2);
    assert_eq!(err, "");
    assert_eq!(
        out,
        "{\"ok\":false,\"exit\":2,\"error\":\"no spec folder here; pass --specs-dir\"}\n"
    );
}
