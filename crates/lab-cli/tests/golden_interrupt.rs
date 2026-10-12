//! A Ctrl+C that reached `lab golden record` while its `lab uat` child ran: the
//! child stops starting runs and ends with the ones it had (exit 0 when they
//! passed). No golden may be recorded from that short batch, and the daemon is
//! not asked. This file is its own test binary because the press is process-wide.

mod golden_support;
mod support;

use std::io;

use cimmeria_lab_cli::commands::golden::{ChildOutput, Deps};
use cimmeria_lab_cli::commands::uat::ctrlc;
use golden_support::{args, json_of, path_text, run, specs_dir};
use serde_json::json;
use support::StubServer;
use tempfile::TempDir;

#[test]
fn a_press_while_the_child_ran_records_no_golden_and_exits_130() {
    let (specs, batch) = (specs_dir(), TempDir::new().unwrap());
    let (stub, endpoint) = StubServer::start(vec![]);
    let line = json!({"ok": true, "runs": 2, "passed": 2, "failed": 0,
                      "batch": path_text(&batch)});
    let runner = |_: &[String]| -> io::Result<ChildOutput> {
        // The press arrives while the child runs; the child still exits 0.
        assert!(ctrlc::press(), "the first press is handled");
        Ok(ChildOutput {
            stdout: format!("{line}\n"),
            code: 0,
        })
    };
    let deps = Deps {
        run_uat: &runner,
        endpoint,
        default_specs: Some(specs.path().to_path_buf()),
    };
    let mut a = args("record", "first-session");
    a.json = true;
    let (code, out, _) = run(&a, &deps);
    assert_eq!(code, 130, "{out}");
    let body = json_of(&out);
    assert_eq!(body["ok"], json!(false));
    assert_eq!(body["exit"], json!(130));
    assert_eq!(
        body["error"],
        json!("interrupted by Ctrl+C; no golden recorded")
    );
    assert_eq!(body["batch"], json!(path_text(&batch)));
    assert!(
        stub.requests().is_empty(),
        "the daemon must not be asked: {:?}",
        stub.requests()
    );
}
