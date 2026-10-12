//! `lab golden record`: through the built binary (refusals that stop before the
//! child runs, and the real spawn of `lab uat`), and in-process with a stubbed
//! child process and a stub MCP daemon.

mod golden_support;
mod support;

use std::cell::{Cell, RefCell};
use std::io;
use std::process::Command;

use cimmeria_lab_cli::commands::golden::{ChildOutput, Deps};
use cimmeria_lab_cli::daemon::status::Endpoint;
use golden_support::{args, json_of, lab, path_text, put_golden, run, specs_dir, GOLDEN, SHA};
use serde_json::{json, Value};
use support::{closed_port_endpoint, StubServer, STUB_TOKEN};
use tempfile::TempDir;

// PS L179
#[test]
fn an_existing_golden_without_rebless_exits_2() {
    let (home, specs) = (TempDir::new().unwrap(), specs_dir());
    put_golden(&specs, "first-session.json", GOLDEN);
    let sp = path_text(&specs);
    let (code, out, _) = lab(
        &home,
        &[
            "golden",
            "record",
            "first-session",
            "--server-version",
            SHA,
            "--specs-dir",
            sp.as_str(),
            "--json",
        ],
    );
    assert_eq!(code, 2);
    assert_eq!(
        json_of(&out)["error"],
        json!("a golden exists; re-blessing needs --rebless and five fresh runs")
    );
}

// PS L189
#[test]
fn record_v1_2_sees_golden_v1_2_json_exists() {
    let (home, specs) = (TempDir::new().unwrap(), specs_dir());
    put_golden(&specs, "v1_2.json", GOLDEN);
    let sp = path_text(&specs);
    let (code, out, _) = lab(
        &home,
        &[
            "golden",
            "record",
            "v1.2",
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
        .starts_with("a golden exists"));
}

// New: the real spawn. The child is `<this exe> uat <PowerShell-form args>`: a
// fake `lab.ps1` (the PowerShell dispatcher `uat` still falls back to) records
// the tokens it is given, writes one progress line to stderr and answers with a
// `lab uat --json` refusal. `record` must forward those tokens in order, report
// the child's own error and exit code, show the child's stderr live, and keep
// stdout to one JSON line. (When `uat` goes native this test must follow it.)
#[test]
fn record_really_spawns_this_exe_with_uat() {
    let (home, specs, fake_cli, seen) = (
        TempDir::new().unwrap(),
        specs_dir(),
        TempDir::new().unwrap(),
        TempDir::new().unwrap(),
    );
    std::fs::write(
        fake_cli.path().join("lab.ps1"),
        "$args | Set-Content -LiteralPath $env:LAB_GOLDEN_TEST_ARGS\n\
         [Console]::Error.WriteLine('uat progress line')\n\
         [Console]::Out.WriteLine('{\"ok\":false,\"exit\":2,\"error\":\"stub uat refused\"}')\n\
         exit 2\n",
    )
    .unwrap();
    let args_file = seen.path().join("args.txt");
    let sp = path_text(&specs);
    let out = Command::new(env!("CARGO_BIN_EXE_lab"))
        .args([
            "golden",
            "record",
            "first-session",
            "--server-version",
            SHA,
            "--specs-dir",
            sp.as_str(),
            "--json",
        ])
        .env("CIMMERIA_LAB_HOME", home.path())
        .env("CIMMERIA_LAB_CLI_DIR", fake_cli.path())
        .env("LAB_GOLDEN_TEST_ARGS", &args_file)
        .output()
        .expect("run lab");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(2), "{stdout}");
    assert_eq!(
        json_of(&stdout),
        json!({"ok": false, "exit": 2, "error": "stub uat refused"})
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("uat progress line"));
    let given: Vec<String> = std::fs::read_to_string(&args_file)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    let expected = [
        "uat",
        "first-session",
        "-Leases",
        "1",
        "-RunsPerLease",
        "5",
        "-ServerVersion",
        SHA,
        "-SpecsDir",
        sp.as_str(),
        "-Fingerprint",
        "-MaxConsecutiveFailures",
        "1",
        "-ProgressToStderr",
        "-Json",
    ];
    assert_eq!(given, expected);
}

// ----- record, with the child and the daemon stubbed -----

/// A batch folder holding `batch.jsonl` with `n` passing run records.
fn batch_dir(n: usize) -> TempDir {
    let dir = TempDir::new().unwrap();
    let lines: String = (1..=n)
        .map(|i| {
            format!(
                "{}\n",
                json!({"Lane": 1, "Instance": "default", "Index": i, "Ok": true,
                       "Rows": [], "RunDir": format!("C:\\runs\\run-{i}"), "Error": null})
            )
        })
        .collect();
    std::fs::write(dir.path().join("batch.jsonl"), lines).unwrap();
    dir
}

fn uat_ok(batch: &TempDir) -> ChildOutput {
    let line = json!({"ok": true, "runs": 5, "passed": 5, "failed": 0,
                      "batch": path_text(batch)});
    ChildOutput {
        stdout: format!("{line}\n"),
        code: 0,
    }
}

/// The body every stub POST /mcp answers: a tools/call result whose text is `reply`.
fn mcp_body(reply: &Value) -> String {
    json!({"jsonrpc": "2.0", "id": 2,
           "result": {"content": [{"type": "text", "text": reply.to_string()}]}})
    .to_string()
}

fn ok_reply() -> Value {
    json!({"ok": true, "path": "C:\\specs\\golden\\first-session.json",
           "rows": 7, "events": 312, "bytes": 21456, "runs": 5})
}

fn deps<'a>(
    run_uat: &'a dyn Fn(&[String]) -> io::Result<ChildOutput>,
    endpoint: Endpoint,
    specs: &TempDir,
) -> Deps<'a> {
    Deps {
        run_uat,
        endpoint,
        default_specs: Some(specs.path().to_path_buf()),
    }
}

/// The `tools/call` request the stub received, as JSON.
fn tool_call(stub: &StubServer) -> Value {
    let requests = stub.requests();
    let call = requests
        .iter()
        .find(|r| r.method == "POST" && r.body.contains("tools/call"))
        .expect("a tools/call request");
    assert_eq!(
        call.header("authorization"),
        Some(format!("Bearer {STUB_TOKEN}").as_str())
    );
    serde_json::from_str(&call.body).unwrap()
}

#[test]
fn record_success_prints_the_announcement_and_the_golden_line() {
    let (specs, batch) = (specs_dir(), batch_dir(5));
    let body = mcp_body(&ok_reply());
    let (stub, endpoint) = StubServer::start(vec![("POST", "/mcp", 200, body.as_str())]);
    let runner = |_: &[String]| Ok(uat_ok(&batch));
    let (code, out, err) = run(
        &args("record", "first-session"),
        &deps(&runner, endpoint, &specs),
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        "lab golden record first-session: 5 fingerprint run(s) on 1 lane(s), then lab_golden_record (a run takes a few minutes)\ngolden first-session: 7 rows, 312 events, 21456 bytes from 5 runs; C:\\specs\\golden\\first-session.json\n"
    );
    assert_eq!(err, "");
    let call = tool_call(&stub);
    assert_eq!(call["params"]["name"], json!("lab_golden_record"));
    let tool = &call["params"]["arguments"];
    assert_eq!(tool["section"], json!("first-session"));
    assert_eq!(tool["rebless"], json!(false));
    assert_eq!(tool["run_dirs"].as_array().unwrap().len(), 5);
    assert_eq!(tool["run_dirs"][0], json!("C:\\runs\\run-1"));
    assert!(tool.get("rows").is_none());
    assert_eq!(tool["specs_dir"], json!(path_text(&specs)));
}

#[test]
fn record_json_prints_one_line_and_no_announcement() {
    let (specs, batch) = (specs_dir(), batch_dir(5));
    let body = mcp_body(&ok_reply());
    let (_stub, endpoint) = StubServer::start(vec![("POST", "/mcp", 200, body.as_str())]);
    let runner = |_: &[String]| Ok(uat_ok(&batch));
    let mut a = args("record", "first-session");
    a.json = true;
    let (code, out, _) = run(&a, &deps(&runner, endpoint, &specs));
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "{\"ok\":true,\"exit\":0,\"path\":\"C:\\\\specs\\\\golden\\\\first-session.json\",\"rows\":7,\"events\":312,\"bytes\":21456,\"runs\":5}\n"
    );
}

#[test]
fn the_child_gets_the_rows_runs_leases_build_and_specs_and_the_tool_gets_the_rows() {
    let (specs, batch) = (specs_dir(), batch_dir(10));
    let body = mcp_body(&ok_reply());
    let (stub, endpoint) = StubServer::start(vec![("POST", "/mcp", 200, body.as_str())]);
    let seen: RefCell<Vec<String>> = RefCell::new(Vec::new());
    let runner = |a: &[String]| {
        *seen.borrow_mut() = a.to_vec();
        Ok(uat_ok(&batch))
    };
    let mut a = args("record", "first-session");
    a.rows = vec!["FS-01,FS-P1".to_string(), "FS-01".to_string()];
    a.runs = 10;
    a.leases = 2;
    let (code, _, err) = run(&a, &deps(&runner, endpoint, &specs));
    assert_eq!(code, 0, "{err}");
    let expected: Vec<String> = [
        "first-session",
        "-Rows",
        "FS-01,FS-P1",
        "-Leases",
        "2",
        "-RunsPerLease",
        "5",
        "-ServerVersion",
        SHA,
        "-SpecsDir",
        path_text(&specs).as_str(),
        "-Fingerprint",
        "-MaxConsecutiveFailures",
        "1",
        "-ProgressToStderr",
        "-Json",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(*seen.borrow(), expected);
    assert_eq!(
        tool_call(&stub)["params"]["arguments"]["rows"],
        json!(["FS-01", "FS-P1"])
    );
}

#[test]
fn a_disagreement_is_exit_6_and_names_the_batch() {
    let (specs, batch) = (specs_dir(), batch_dir(5));
    let body = mcp_body(&json!({"ok": false, "code": "disagree", "error": "run b differs"}));
    let (_stub, endpoint) = StubServer::start(vec![("POST", "/mcp", 200, body.as_str())]);
    let runner = |_: &[String]| Ok(uat_ok(&batch));
    let (code, out, err) = run(
        &args("record", "first-session"),
        &deps(&runner, endpoint, &specs),
    );
    assert_eq!(code, 6);
    assert!(!out.contains("golden first-session:"));
    assert_eq!(
        err,
        format!(
            "disagree: run b differs; the runs are in {}\n",
            path_text(&batch)
        )
    );
}

#[test]
fn a_json_refusal_has_the_fixed_key_order() {
    let (specs, batch) = (specs_dir(), batch_dir(5));
    let body = mcp_body(&json!({"ok": false, "code": "disagree", "error": "run b differs"}));
    let (_stub, endpoint) = StubServer::start(vec![("POST", "/mcp", 200, body.as_str())]);
    let runner = |_: &[String]| Ok(uat_ok(&batch));
    let mut a = args("record", "first-session");
    a.json = true;
    let (code, out, _) = run(&a, &deps(&runner, endpoint, &specs));
    assert_eq!(code, 6);
    assert_eq!(
        out,
        format!(
            "{{\"ok\":false,\"exit\":6,\"code\":\"disagree\",\"error\":\"run b differs\",\"batch\":{}}}\n",
            json!(path_text(&batch))
        )
    );
}

#[test]
fn each_refusal_code_becomes_its_exit_code() {
    for (code_text, exit) in [("exists", 2u8), ("not_passing", 1), ("cannot_record", 7)] {
        let (specs, batch) = (specs_dir(), batch_dir(5));
        let body = mcp_body(&json!({"ok": false, "code": code_text, "error": "x"}));
        let (_stub, endpoint) = StubServer::start(vec![("POST", "/mcp", 200, body.as_str())]);
        let runner = |_: &[String]| Ok(uat_ok(&batch));
        let (code, _, _) = run(
            &args("record", "first-session"),
            &deps(&runner, endpoint, &specs),
        );
        assert_eq!(code, exit, "{code_text}");
    }
}

#[test]
fn a_braked_batch_is_exit_1_and_the_daemon_is_never_asked() {
    let specs = specs_dir();
    let body = mcp_body(&ok_reply());
    let (stub, endpoint) = StubServer::start(vec![("POST", "/mcp", 200, body.as_str())]);
    let line = json!({"ok": false, "runs": 2, "passed": 1, "failed": 1,
        "failures": [{"n": 1, "why": "FS-P2 FAIL: clause movie-over failed", "runs": ["default#2"]}],
        "batch": "C:\\b"})
    .to_string();
    let runner = |_: &[String]| {
        Ok(ChildOutput {
            stdout: format!("{line}\n"),
            code: 4,
        })
    };
    let (code, _, err) = run(
        &args("record", "first-session"),
        &deps(&runner, endpoint, &specs),
    );
    assert_eq!(code, 1);
    assert_eq!(
        err,
        "every run must pass to record a golden; 1/2 passed, first failure: FS-P2 FAIL: clause movie-over failed (runs: default#2); C:\\b\n"
    );
    assert!(stub.requests().is_empty());
}

#[test]
fn a_child_that_cannot_start_is_exit_3() {
    let specs = specs_dir();
    let runner = |_: &[String]| Err(io::Error::new(io::ErrorKind::NotFound, "no such file"));
    let (code, _, err) = run(
        &args("record", "first-session"),
        &deps(&runner, closed_port_endpoint(), &specs),
    );
    assert_eq!(code, 3);
    assert_eq!(err, "lab uat could not start: no such file\n");
}

#[test]
fn a_batch_without_a_run_folder_is_exit_7() {
    let (specs, batch) = (specs_dir(), TempDir::new().unwrap());
    // No batch.jsonl at all: no records, so no run folders.
    let runner = |_: &[String]| Ok(uat_ok(&batch));
    let (code, _, err) = run(
        &args("record", "first-session"),
        &deps(&runner, closed_port_endpoint(), &specs),
    );
    assert_eq!(code, 7);
    assert_eq!(
        err,
        format!(
            "the batch passed but a run has no run folder; {}\n",
            path_text(&batch)
        )
    );
}

#[test]
fn no_token_is_exit_7_with_the_batch_folder() {
    let (specs, batch) = (specs_dir(), batch_dir(5));
    let runner = |_: &[String]| Ok(uat_ok(&batch));
    let endpoint = Endpoint {
        base_url: "http://127.0.0.1:9".to_string(),
        token: None,
    };
    let (code, _, err) = run(
        &args("record", "first-session"),
        &deps(&runner, endpoint, &specs),
    );
    assert_eq!(code, 7);
    assert_eq!(
        err,
        format!(
            "the lab daemon is not answering (lab status); the runs are in {}\n",
            path_text(&batch)
        )
    );
}

#[test]
fn an_unreachable_daemon_is_exit_7_with_the_batch_folder() {
    let (specs, batch) = (specs_dir(), batch_dir(5));
    let runner = |_: &[String]| Ok(uat_ok(&batch));
    let (code, _, err) = run(
        &args("record", "first-session"),
        &deps(&runner, closed_port_endpoint(), &specs),
    );
    assert_eq!(code, 7);
    assert!(err.starts_with("lab_golden_record failed: "), "{err}");
    assert!(
        err.trim_end()
            .ends_with(&format!("; the runs are in {}", path_text(&batch))),
        "{err}"
    );
}

#[test]
fn an_existing_golden_stops_before_the_child_runs_unless_rebless() {
    let (specs, batch) = (specs_dir(), batch_dir(5));
    put_golden(&specs, "first-session.json", GOLDEN);
    let body = mcp_body(&ok_reply());
    let (_stub, endpoint) = StubServer::start(vec![("POST", "/mcp", 200, body.as_str())]);
    let called = Cell::new(0u32);
    let runner = |_: &[String]| {
        called.set(called.get() + 1);
        Ok(uat_ok(&batch))
    };
    let d = deps(&runner, endpoint, &specs);
    let (code, _, err) = run(&args("record", "first-session"), &d);
    assert_eq!((code, called.get()), (2, 0));
    assert_eq!(
        err,
        "a golden exists; re-blessing needs --rebless and five fresh runs\n"
    );
    let mut a = args("record", "first-session");
    a.rebless = true;
    let (code, _, _) = run(&a, &d);
    assert_eq!((code, called.get()), (0, 1));
}

#[test]
fn lease_ids_never_leave_the_command() {
    let (specs, batch) = (specs_dir(), batch_dir(5));
    let body = mcp_body(
        &json!({"ok": false, "code": "cannot_record", "error": "lease-0123456789abcdef is held"}),
    );
    let (_stub, endpoint) = StubServer::start(vec![("POST", "/mcp", 200, body.as_str())]);
    let runner = |_: &[String]| Ok(uat_ok(&batch));
    let (code, _, err) = run(
        &args("record", "first-session"),
        &deps(&runner, endpoint, &specs),
    );
    assert_eq!(code, 7);
    assert!(
        err.contains("lease-<redacted>") && !err.contains("0123456789abcdef"),
        "{err}"
    );
}
