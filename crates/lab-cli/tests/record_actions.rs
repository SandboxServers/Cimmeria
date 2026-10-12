//! `lab record` start and stop over a stubbed daemon (ports test-record.ps1,
//! "usage and no daemon", "start writes the state file; stop reads it").

mod record_support;

use std::fs;
use std::path::Path;

use cimmeria_lab_cli::commands::record::action::Outcome;
use cimmeria_lab_cli::commands::record::json::{body_get, compact_json};
use cimmeria_lab_cli::commands::record::state::{read_state, save_state};
use serde_json::{json, Value};
use tempfile::TempDir;

use record_support::{args, joined, ok, run, tool_err, Recorded, Reply, ID, SPECS, UAT};

const FAILED_BODY: &str = r#"{"ok":false,"exit":1,"error":"C:\\lab\\records\\x; see transform-error.txt; transform: boom"}"#;

fn start_reply() -> Reply {
    ok(json!({"ok":true,"recording_id":ID,"instance":"p2","section":"first-session"}))
}

fn stop_reply() -> Reply {
    ok(
        json!({"ok":true,"draft":"first-session-20261012-1430.toml","rows":3,"steps":9,"todo":1,"dropped":0,"raw":ID}),
    )
}

/// `lab record start first-session -Instance p2 -MaxMinutes 20`.
fn start_p2(home: &Path) -> (Outcome, Vec<Recorded>) {
    let mut a = args("start", Some("first-session"));
    a.instance = Some("p2".into());
    a.max_minutes = Some("20".into());
    run(home, &a, vec![start_reply()], true)
}

fn stop_p2() -> cimmeria_lab_cli::commands::record::Args {
    let mut a = args("stop", None);
    a.instance = Some("p2".into());
    a
}

fn save_p2(home: &Path) {
    save_state(&home.join("record-p2.json"), ID, "p2", SPECS).expect("save state");
}

// Check: 'a usage error exits 2 before any call'
#[test]
fn a_usage_error_exits_2_before_any_call() {
    let dir = TempDir::new().expect("temp dir");
    let (o, calls) = run(dir.path(), &args("start", Some("BAD")), vec![], true);
    assert_eq!(o.exit, 2);
    assert!(calls.is_empty());
}

// Check: 'no daemon exits 3'
#[test]
fn no_daemon_exits_3() {
    let dir = TempDir::new().expect("temp dir");
    let (o, calls) = run(dir.path(), &args("status", None), vec![], false);
    assert_eq!(o.exit, 3);
    assert!(o.text.contains("not answering"));
    assert!(calls.is_empty());
}

// Check: 'transform without the daemon says to start it'
#[test]
fn transform_without_the_daemon_says_to_start_it() {
    let dir = TempDir::new().expect("temp dir");
    let (o, _) = run(dir.path(), &args("transform", Some(ID)), vec![], false);
    assert_eq!(o.exit, 3);
    assert!(o
        .text
        .contains("runs in the lab daemon; start it with `lab start`"));
}

// Check: 'start calls lab_record_start'
#[test]
fn start_calls_lab_record_start() {
    let dir = TempDir::new().expect("temp dir");
    let (o, calls) = start_p2(dir.path());
    assert_eq!(o.exit, 0);
    assert_eq!(calls[0].name, "lab_record_start");
}

// Check: 'start sends section, instance, max_minutes and the spec folder'
#[test]
fn start_sends_section_instance_max_minutes_and_the_spec_folder() {
    let dir = TempDir::new().expect("temp dir");
    let (_, calls) = start_p2(dir.path());
    let sent = &calls[0].args;
    assert_eq!(sent["section"], json!("first-session"));
    assert_eq!(sent["instance"], json!("p2"));
    assert_eq!(sent["max_minutes"], json!(20));
    assert!(sent["specs_dir"]
        .as_str()
        .expect("specs_dir")
        .ends_with("uat-specs"));
}

// Check: 'start sends no character unless given'
#[test]
fn start_sends_no_character_unless_given() {
    let dir = TempDir::new().expect("temp dir");
    let (_, calls) = start_p2(dir.path());
    assert!(calls[0].args.get("character").is_none());
}

// Check: 'start saves {recording_id, instance}'
#[test]
fn start_saves_recording_id_and_instance() {
    let dir = TempDir::new().expect("temp dir");
    start_p2(dir.path());
    let state = read_state(&dir.path().join("record-p2.json")).expect("state file");
    assert_eq!(state.recording_id, ID);
    assert_eq!(state.instance.as_deref(), Some("p2"));
}

// Check: 'start says how to stop'
#[test]
fn start_says_how_to_stop() {
    let dir = TempDir::new().expect("temp dir");
    let (o, _) = start_p2(dir.path());
    assert_eq!(
        o.text,
        format!("recording {ID} on p2 (section first-session); play, then: lab record stop -Instance p2")
    );
}

// Check: 'a failed stop prints the failure body unchanged'
#[test]
fn a_failed_stop_prints_the_failure_body_unchanged() {
    let dir = TempDir::new().expect("temp dir");
    save_p2(dir.path());
    let (o, _) = run(dir.path(), &stop_p2(), vec![tool_err(FAILED_BODY)], true);
    assert_eq!(o.exit, 1);
    assert_eq!(compact_json(&o.body), FAILED_BODY);
}

// Check: 'stop sends the saved id and instance'
#[test]
fn stop_sends_the_saved_id_and_instance() {
    let dir = TempDir::new().expect("temp dir");
    save_p2(dir.path());
    let (_, calls) = run(dir.path(), &stop_p2(), vec![tool_err(FAILED_BODY)], true);
    assert_eq!(calls[0].name, "lab_record_stop");
    assert_eq!(calls[0].args["recording_id"], json!(ID));
    assert_eq!(calls[0].args["instance"], json!("p2"));
}

// Check: 'a failed stop keeps the state file'
#[test]
fn a_failed_stop_keeps_the_state_file() {
    let dir = TempDir::new().expect("temp dir");
    save_p2(dir.path());
    run(dir.path(), &stop_p2(), vec![tool_err(FAILED_BODY)], true);
    assert!(dir.path().join("record-p2.json").exists());
}

// Check: 'a failed stop offers the retry'
#[test]
fn a_failed_stop_offers_the_retry() {
    let dir = TempDir::new().expect("temp dir");
    save_p2(dir.path());
    let (o, _) = run(dir.path(), &stop_p2(), vec![tool_err(FAILED_BODY)], true);
    assert!(o
        .text
        .ends_with("recording is kept; retry with lab record stop -Instance p2"));
}

// Check: 'a refused stop exits 2 and keeps the state file'
#[test]
fn a_refused_stop_exits_2_and_keeps_the_state_file() {
    let dir = TempDir::new().expect("temp dir");
    save_p2(dir.path());
    let refused = tool_err(r#"{"ok":false,"exit":2,"error":"character_expr: bad"}"#);
    let (o, _) = run(dir.path(), &stop_p2(), vec![refused], true);
    assert_eq!(o.exit, 2);
    assert!(dir.path().join("record-p2.json").exists());
}

// Check: 'a stop that succeeds deletes the state file'
#[test]
fn a_stop_that_succeeds_deletes_the_state_file() {
    let dir = TempDir::new().expect("temp dir");
    save_p2(dir.path());
    let mut a = stop_p2();
    a.character_expr = Some("Px${run_id}".into());
    a.profile = Some("fresh".into());
    let (o, _) = run(dir.path(), &a, vec![stop_reply()], true);
    assert_eq!(o.exit, 0);
    assert!(!dir.path().join("record-p2.json").exists());
}

// Check: 'stop passes -CharacterExpr and -Profile'
#[test]
fn stop_passes_character_expr_and_profile() {
    let dir = TempDir::new().expect("temp dir");
    save_p2(dir.path());
    let mut a = stop_p2();
    a.character_expr = Some("Px${run_id}".into());
    a.profile = Some("fresh".into());
    let (_, calls) = run(dir.path(), &a, vec![stop_reply()], true);
    assert_eq!(calls[0].args["character_expr"], json!("Px${run_id}"));
    assert_eq!(calls[0].args["profile"], json!("fresh"));
}

// Check: 'stop prints the draft and raw log paths'
#[test]
fn stop_prints_the_draft_and_raw_log_paths() {
    let dir = TempDir::new().expect("temp dir");
    save_p2(dir.path());
    let (o, _) = run(dir.path(), &stop_p2(), vec![stop_reply()], true);
    let draft = joined(SPECS, &["drafts", "first-session-20261012-1430.toml"]);
    let raw = joined(UAT, &["records", ID, "record.jsonl"]);
    assert!(o.text.contains(&draft), "{}", o.text);
    assert!(o.text.contains(&raw), "{}", o.text);
    assert_eq!(
        o.text,
        format!("draft {draft}\n  3 row(s), 9 step(s), 1 TODO(s), 0 dropped\nraw   {raw}")
    );
}

// Check: 'stop prints the counts'
#[test]
fn stop_prints_the_counts() {
    let dir = TempDir::new().expect("temp dir");
    save_p2(dir.path());
    let (o, _) = run(dir.path(), &stop_p2(), vec![stop_reply()], true);
    assert!(o.text.contains("3 row(s), 9 step(s), 1 TODO(s), 0 dropped"));
}

// Check: 'stop without a state file exits 2 and calls nothing'
#[test]
fn stop_without_a_state_file_exits_2_and_calls_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let (o, calls) = run(dir.path(), &stop_p2(), vec![], true);
    assert_eq!(o.exit, 2);
    assert!(o
        .text
        .starts_with("no recording started here for instance p2"));
    assert!(calls.is_empty());
}

// Check: '-RecordingId stops without a state file'
#[test]
fn recording_id_stops_without_a_state_file() {
    let dir = TempDir::new().expect("temp dir");
    let mut a = stop_p2();
    a.recording_id = Some(ID.into());
    a.no_draft = true;
    let reply = ok(json!({"ok":true,"recording_id":ID,"ticks":4,"raw":ID}));
    let (o, calls) = run(dir.path(), &a, vec![reply], true);
    assert_eq!(o.exit, 0);
    assert_eq!(calls[0].args["recording_id"], json!(ID));
    assert_eq!(calls[0].args["no_draft"], json!(true));
}

// Check: '-NoDraft says no draft'
#[test]
fn no_draft_says_no_draft() {
    let dir = TempDir::new().expect("temp dir");
    let mut a = stop_p2();
    a.recording_id = Some(ID.into());
    a.no_draft = true;
    let reply = ok(json!({"ok":true,"recording_id":ID,"ticks":4,"raw":ID}));
    let (o, _) = run(dir.path(), &a, vec![reply], true);
    assert!(o.text.starts_with("stopped; 4 tick(s), no draft"));
}

// Check: 'an unrouted start saves record-default.json'
#[test]
fn an_unrouted_start_saves_record_default_json() {
    let dir = TempDir::new().expect("temp dir");
    let reply =
        ok(json!({"ok":true,"recording_id":"x1","instance":"default","section":"first-session"}));
    let (_, calls) = run(
        dir.path(),
        &args("start", Some("first-session")),
        vec![reply],
        true,
    );
    assert!(calls[0].args.get("instance").is_none());
    let state = read_state(&dir.path().join("record-default.json")).expect("state file");
    assert_eq!(state.recording_id, "x1");
}

// Check: 'a bad instance in the answer writes no file'
#[test]
fn a_bad_instance_in_the_answer_writes_no_file() {
    let dir = TempDir::new().expect("temp dir");
    let reply = ok(json!({"ok":true,"recording_id":"x2","instance":r"..\evil"}));
    let (o, _) = run(
        dir.path(),
        &args("start", Some("first-session")),
        vec![reply],
        true,
    );
    assert_eq!(o.exit, 1);
    assert!(!dir.path().join("..").join("evil").exists());
    let names: Vec<_> = fs::read_dir(dir.path()).expect("read dir").collect();
    assert!(names.is_empty(), "no file was written: {names:?}");
}

// Check: 'a state save failure after start exits 1'
#[test]
fn a_state_save_failure_after_start_exits_1() {
    let dir = TempDir::new().expect("temp dir");
    // A lab home under a regular file: create_dir_all fails.
    let blocker = dir.path().join("blocker");
    fs::write(&blocker, "x").expect("write");
    let home = blocker.join("lab");
    let mut a = args("start", Some("first-session"));
    a.instance = Some("p4".into());
    let reply = ok(json!({"ok":true,"recording_id":ID,"instance":"p4","section":"first-session"}));
    let (o, _) = run(&home, &a, vec![reply], true);
    assert_eq!(o.exit, 1);
    assert_eq!(body_get(&o.body, "ok"), Some(&Value::Bool(false)));
    assert_eq!(body_get(&o.body, "exit"), Some(&json!(1)));
}

// Check: 'the save failure names the recording and the stop'
#[test]
fn the_save_failure_names_the_recording_and_the_stop() {
    let dir = TempDir::new().expect("temp dir");
    let blocker = dir.path().join("blocker");
    fs::write(&blocker, "x").expect("write");
    let home = blocker.join("lab");
    let mut a = args("start", Some("first-session"));
    a.instance = Some("p4".into());
    let reply = ok(json!({"ok":true,"recording_id":ID,"instance":"p4","section":"first-session"}));
    let (o, _) = run(&home, &a, vec![reply], true);
    assert!(o.text.contains(ID));
    assert!(o
        .text
        .contains(&format!("lab record stop -Instance p4 -RecordingId {ID}")));
    assert!(o.text.contains("state file could not be written ("));
}

// Check: 'a state file with no instance stops unrouted'
#[test]
fn a_state_file_with_no_instance_stops_unrouted() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(
        dir.path().join("record-default.json"),
        format!("{{\"recording_id\":\"{ID}\"}}"),
    )
    .expect("write");
    let reply = ok(
        json!({"ok":true,"draft":"fs-20261012-1430.toml","rows":1,"steps":1,"todo":0,"dropped":0,"raw":ID}),
    );
    let (o, calls) = run(dir.path(), &args("stop", None), vec![reply], true);
    assert_eq!(o.exit, 0);
    assert_eq!(calls[0].args["recording_id"], json!(ID));
    assert!(calls[0].args.get("instance").is_none());
}

// Check: 'a spec folder unknown here prints <specs>, never a guessed path'
#[test]
fn a_spec_folder_unknown_here_prints_specs_placeholder() {
    let dir = TempDir::new().expect("temp dir");
    fs::write(
        dir.path().join("record-default.json"),
        format!("{{\"recording_id\":\"{ID}\"}}"),
    )
    .expect("write");
    let reply = ok(
        json!({"ok":true,"draft":"fs-20261012-1430.toml","rows":1,"steps":1,"todo":0,"dropped":0,"raw":ID}),
    );
    let (o, _) = run(dir.path(), &args("stop", None), vec![reply], true);
    assert!(o
        .text
        .starts_with("draft <specs>/drafts/fs-20261012-1430.toml"));
}

// Check: 'exit 3 on stop offers the retry too'
#[test]
fn exit_3_on_stop_offers_the_retry_too() {
    use cimmeria_lab_cli::daemon::mcp::McpError;
    let dir = TempDir::new().expect("temp dir");
    save_p2(dir.path());
    let gone = Err(McpError::Unreachable("No connection could be made".into()));
    let (o, _) = run(dir.path(), &stop_p2(), vec![gone], true);
    assert_eq!(o.exit, 3);
    assert!(dir.path().join("record-p2.json").exists());
    assert!(o
        .text
        .ends_with("recording is kept; retry with lab record stop -Instance p2"));
}

// Extra: the state file is one compact line, keys in order, and the folder
// `start` sent is where `stop` says the draft went.
#[test]
fn the_state_file_is_one_ordered_line_and_stop_uses_its_spec_folder() {
    let dir = TempDir::new().expect("temp dir");
    let mut a = args("start", Some("first-session"));
    a.instance = Some("p2".into());
    a.specs_dir = Some(r"D:\specs".into());
    run(dir.path(), &a, vec![start_reply()], true);
    let text = fs::read_to_string(dir.path().join("record-p2.json")).expect("state file");
    assert_eq!(
        text,
        format!(
            "{{\"recording_id\":\"{ID}\",\"instance\":\"p2\",\"specs_dir\":\"D:\\\\specs\"}}\n"
        )
    );
    let (o, _) = run(dir.path(), &stop_p2(), vec![stop_reply()], true);
    let draft = joined(r"D:\specs", &["drafts", "first-session-20261012-1430.toml"]);
    assert!(o.text.starts_with(&format!("draft {draft}")), "{}", o.text);
}

// Extra: a start that sent no spec folder leaves the key out of the state file.
#[test]
fn a_start_with_no_spec_folder_leaves_specs_dir_out_of_the_state_file() {
    let dir = TempDir::new().expect("temp dir");
    save_state(&dir.path().join("record-p2.json"), ID, "p2", "").expect("save state");
    let text = fs::read_to_string(dir.path().join("record-p2.json")).expect("state file");
    assert_eq!(
        text,
        format!("{{\"recording_id\":\"{ID}\",\"instance\":\"p2\"}}\n")
    );
}

// Extra: a good stop keeps the state file of another instance.
#[test]
fn a_good_stop_deletes_only_its_own_state_file() {
    let dir = TempDir::new().expect("temp dir");
    save_p2(dir.path());
    save_state(&dir.path().join("record-p3.json"), "other", "p3", "").expect("save state");
    run(dir.path(), &stop_p2(), vec![stop_reply()], true);
    assert!(!dir.path().join("record-p2.json").exists());
    assert!(dir.path().join("record-p3.json").exists());
}
// Extra (not a ported check): the PowerShell paths on Windows, byte for byte.
#[cfg(windows)]
#[test]
fn stop_paths_use_backslashes_on_windows() {
    let dir = TempDir::new().expect("temp dir");
    save_p2(dir.path());
    let (o, _) = run(dir.path(), &stop_p2(), vec![stop_reply()], true);
    assert!(o
        .text
        .contains(r"draft C:\repo\docs\guides\uat-specs\drafts\first-session-20261012-1430.toml"));
    assert!(o
        .text
        .contains(r"raw   C:\lab\uat-runs\records\20261012-143000-p2\record.jsonl"));
}
