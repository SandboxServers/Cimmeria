//! `lab record` exit-code mapping, status and transform over a stubbed daemon
//! (ports test-record.ps1, "exit-code mapping" and "status and transform").

mod record_support;

use std::path::Path;

use cimmeria_lab_cli::commands::record::json::body_get;
use cimmeria_lab_cli::daemon::mcp::McpError;
use serde_json::{json, Value};
use tempfile::TempDir;

use record_support::{args, joined, ok, run, tool_err, ID};

fn start_p3() -> cimmeria_lab_cli::commands::record::Args {
    let mut a = args("start", Some("first-session"));
    a.instance = Some("p3".into());
    a
}

// Check: 'a busy answer (exit 4) exits 4, and the body says so'
#[test]
fn a_busy_answer_exits_4_and_the_body_says_so() {
    let dir = TempDir::new().expect("temp dir");
    let busy = tool_err(
        r#"{"ok":false,"exit":4,"error":"the lab is leased to lab_uat (lease-0123456789abcdef). Wait for it"}"#,
    );
    let (o, _) = run(dir.path(), &start_p3(), vec![busy], true);
    assert_eq!(o.exit, 4);
    assert_eq!(body_get(&o.body, "exit"), Some(&json!(4)));
}

// Check: 'lease ids are hidden'
#[test]
fn lease_ids_are_hidden() {
    let dir = TempDir::new().expect("temp dir");
    let busy = tool_err(
        r#"{"ok":false,"exit":4,"error":"the lab is leased to lab_uat (lease-0123456789abcdef). Wait for it"}"#,
    );
    let (o, _) = run(dir.path(), &start_p3(), vec![busy], true);
    assert!(!o.text.contains("lease-0123456789abcdef"));
    assert!(o.text.contains("lease-<redacted>"));
}

// Check: 'an exit-2 body that mentions busy words stays 2 (no text rewrite)'
#[test]
fn an_exit_2_body_that_mentions_busy_words_stays_2() {
    let dir = TempDir::new().expect("temp dir");
    let refused = tool_err(r#"{"ok":false,"exit":2,"error":"the lab is leased; already running"}"#);
    let (o, _) = run(dir.path(), &start_p3(), vec![refused], true);
    assert_eq!(o.exit, 2);
    assert_eq!(body_get(&o.body, "exit"), Some(&json!(2)));
}

// Check: 'a wrong id exits 2'
#[test]
fn a_wrong_id_exits_2() {
    let dir = TempDir::new().expect("temp dir");
    let mut a = args("stop", None);
    a.recording_id = Some("zz".into());
    let wrong = tool_err(
        r#"{"ok":false,"exit":2,"error":"that recording_id is not the running recording on instance default"}"#,
    );
    let (o, _) = run(dir.path(), &a, vec![wrong], true);
    assert_eq!(o.exit, 2);
    assert_eq!(body_get(&o.body, "exit"), Some(&json!(2)));
}

// Check: 'a plain tool error exits 1, whatever it says'
#[test]
fn a_plain_tool_error_exits_1_whatever_it_says() {
    let dir = TempDir::new().expect("temp dir");
    let plain = tool_err("the call needs a lease; the lab is leased to someone");
    let (o, _) = run(dir.path(), &args("status", None), vec![plain], true);
    assert_eq!(o.exit, 1);
    assert_eq!(body_get(&o.body, "ok"), Some(&Value::Bool(false)));
}

// Check: 'any other tool error exits 1'
#[test]
fn any_other_tool_error_exits_1() {
    let dir = TempDir::new().expect("temp dir");
    let (o, _) = run(
        dir.path(),
        &args("status", None),
        vec![tool_err("invalid params")],
        true,
    );
    assert_eq!(o.exit, 1);
}

// Check: 'a transport error exits 3'
#[test]
fn a_transport_error_exits_3() {
    let dir = TempDir::new().expect("temp dir");
    let refused = Err(McpError::Unreachable(
        "No connection could be made because the target machine actively refused it.".into(),
    ));
    let (o, _) = run(dir.path(), &args("status", None), vec![refused], true);
    assert_eq!(o.exit, 3);
    assert!(o.text.starts_with("the lab daemon did not answer"));
}

// Check: 'status shows a recording that stopped by itself'
#[test]
fn status_shows_a_recording_that_stopped_by_itself() {
    let dir = TempDir::new().expect("temp dir");
    let mut a = args("status", None);
    a.instance = Some("p2".into());
    let reply = ok(
        json!({"ok":true,"recording":false,"recording_id":ID,"instance":"p2","section":"fs","age_s":75,"ticks":150,"anchors":2}),
    );
    let (o, _) = run(dir.path(), &a, vec![reply], true);
    assert_eq!(o.exit, 0);
    assert_eq!(
        o.text,
        format!("stopped by itself, waiting for lab record stop: {ID} on p2 (section fs): 75 s, 150 ticks, 2 anchors")
    );
}

// Check: 'status with nothing armed'
#[test]
fn status_with_nothing_armed() {
    let dir = TempDir::new().expect("temp dir");
    let reply = ok(json!({"ok":true,"recording":false,"instance":"default"}));
    let (o, _) = run(dir.path(), &args("status", None), vec![reply], true);
    assert_eq!(o.text, "no recording on default");
}

fn transform_other(
    dir: &Path,
) -> (
    cimmeria_lab_cli::commands::record::action::Outcome,
    Vec<record_support::Recorded>,
) {
    let mut a = args("transform", Some(ID));
    a.section = Some("other-section".into());
    a.specs_dir = Some(r"D:\specs".into());
    let reply = ok(
        json!({"ok":true,"draft":"other-section-20261012-1430.toml","rows":1,"steps":2,"todo":0,"dropped":0,"raw":ID}),
    );
    run(dir, &a, vec![reply], true)
}

// Check: 'transform sends a bare id, -Section and -SpecsDir'
#[test]
fn transform_sends_a_bare_id_section_and_specs_dir() {
    let dir = TempDir::new().expect("temp dir");
    let (_, calls) = transform_other(dir.path());
    assert_eq!(calls[0].name, "lab_record_transform");
    assert_eq!(calls[0].args["dir"], json!(ID));
    assert_eq!(calls[0].args["section"], json!("other-section"));
    assert_eq!(calls[0].args["specs_dir"], json!(r"D:\specs"));
}

// Check: 'transform is not routed'
#[test]
fn transform_is_not_routed() {
    let dir = TempDir::new().expect("temp dir");
    let (_, calls) = transform_other(dir.path());
    assert!(calls[0].args.get("instance").is_none());
}

// Check: 'transform prints the draft path'
#[test]
fn transform_prints_the_draft_path() {
    let dir = TempDir::new().expect("temp dir");
    let (o, _) = transform_other(dir.path());
    let draft = joined(r"D:\specs", &["drafts", "other-section-20261012-1430.toml"]);
    assert!(o.text.starts_with(&format!("draft {draft}")), "{}", o.text);
}

// Check: 'a relative folder is sent in full'
#[test]
fn a_relative_folder_is_sent_in_full() {
    let dir = TempDir::new().expect("temp dir");
    std::fs::create_dir(dir.path().join("rel-rec")).expect("mkdir");
    // The test context's working folder is the temp dir.
    let reply = ok(json!({"ok":true,"draft":"d.toml","raw":"rel-rec"}));
    let (_, calls) = run(
        dir.path(),
        &args("transform", Some("rel-rec")),
        vec![reply],
        true,
    );
    let want = std::path::absolute(dir.path().join("rel-rec")).expect("absolute");
    assert_eq!(calls[0].args["dir"], json!(want.display().to_string()));
}

// Extra (not a ported check): a running recording's status line.
#[test]
fn status_shows_a_running_recording() {
    let dir = TempDir::new().expect("temp dir");
    let reply = ok(
        json!({"ok":true,"recording":true,"recording_id":ID,"instance":"p2","section":"fs","age_s":5,"ticks":10,"anchors":0}),
    );
    let (o, _) = run(dir.path(), &args("status", None), vec![reply], true);
    assert_eq!(
        o.text,
        format!("recording {ID} on p2 (section fs): 5 s, 10 ticks, 0 anchors")
    );
}
