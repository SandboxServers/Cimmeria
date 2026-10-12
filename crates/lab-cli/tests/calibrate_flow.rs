//! LX-12b: the calibrate flow with a stub /status server and a stub MCP call.

mod support;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use cimmeria_lab_cli::commands::calibrate::{self, Deps};
use cimmeria_lab_cli::commands::{Cli, Command as Cmd};
use cimmeria_lab_cli::daemon::mcp::McpError;
use cimmeria_lab_cli::daemon::status::Endpoint;
use cimmeria_lab_cli::ui::Ui;
use clap::Parser;
use serde_json::{json, Value};
use support::StubServer;
use tempfile::TempDir;

const DIFF: &str = "--- a/docs/guides/uat-specs/x.toml\n+++ b/docs/guides/uat-specs/x.toml\n@@ -1,3 +1,3 @@\n a\n-b\n+B\n c\n";

struct Ran {
    code: u8,
    out: String,
    err: String,
    calls: Vec<(String, Value)>,
    /// The timeout each call was given.
    waits: Vec<Duration>,
}

fn args(tokens: &[&str]) -> calibrate::Args {
    let mut all = vec!["lab", "calibrate"];
    all.extend_from_slice(tokens);
    match Cli::try_parse_from(all).expect("parses").command {
        Some(Cmd::Calibrate(a)) => a,
        _ => panic!("not calibrate"),
    }
}

/// `[(instance, leased, client_pid)]` as the daemon's /status body.
fn status_body(rows: &[(&str, bool, Option<u32>)]) -> String {
    let instances: Vec<Value> = rows
        .iter()
        .map(|(name, held, pid)| json!({"instance": name, "account": null, "client_pid": pid, "lease": {"held": held}}))
        .collect();
    json!({"daemon": {"pid": 1, "version": "x"}, "instances": instances}).to_string()
}

fn run_flow(
    tokens: &[&str],
    ep: &Endpoint,
    repo: Option<PathBuf>,
    sgw: Vec<u32>,
    replies: Vec<Result<Value, McpError>>,
) -> Ran {
    run_flow_hooked(tokens, ep, repo, sgw, replies, &|_| {})
}

/// [`run_flow`], with `hook` run (with the tool's name) as each MCP call starts.
fn run_flow_hooked(
    tokens: &[&str],
    ep: &Endpoint,
    repo: Option<PathBuf>,
    sgw: Vec<u32>,
    replies: Vec<Result<Value, McpError>>,
    hook: &dyn Fn(&str),
) -> Ran {
    let queue = RefCell::new(VecDeque::from(replies));
    let calls = RefCell::new(Vec::<(String, Value)>::new());
    let waits = RefCell::new(Vec::<Duration>::new());
    let mut call = |name: &str, a: Value, t: Duration| -> Result<Value, McpError> {
        calls.borrow_mut().push((name.to_string(), a));
        waits.borrow_mut().push(t);
        hook(name);
        queue
            .borrow_mut()
            .pop_front()
            .expect("an unexpected MCP call")
    };
    let sgw_fn = move || sgw.clone();
    let mut deps = Deps {
        ep,
        repo,
        specs: None,
        sgw_pids: &sgw_fn,
        call: &mut call,
    };
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = calibrate::execute(&args(tokens), &Ui::plain(), &mut deps, &mut out, &mut err);
    Ran {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
        calls: calls.into_inner(),
        waits: waits.into_inner(),
    }
}

fn frost_reply(diff: &str) -> Value {
    json!({"run_dir": "RUN", "outcomes": [
        {"ok": true, "pose": "frost", "score": 8312, "confirmed": 3, "changed": {"bearing_deg": [322.5, 330.0]}, "diff_path": diff}]})
}

fn git(repo: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git");
    assert!(o.status.success(), "git {args:?}");
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn scratch_repo() -> (TempDir, PathBuf, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    fs::create_dir_all(repo.join("docs/guides/uat-specs")).unwrap();
    git(&repo, &["init", "-q", "."]);
    for kv in [
        ["user.email", "t@example.invalid"],
        ["user.name", "t"],
        ["core.autocrlf", "false"],
        ["commit.gpgsign", "false"],
    ] {
        git(&repo, &["config", kv[0], kv[1]]);
    }
    let spec = repo.join("docs/guides/uat-specs/x.toml");
    fs::write(&spec, "a\nb\nc\n").unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "init"]);
    (tmp, repo, spec)
}

#[test]
fn a_proposal_prints_the_line_the_diff_and_the_apply_hint() {
    let body = status_body(&[("default", false, None)]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let tmp = TempDir::new().unwrap();
    let diff = tmp.path().join("x.diff");
    fs::write(&diff, DIFF).unwrap();
    let d = diff.to_string_lossy().into_owned();
    let r = run_flow(
        &["first-session", "FS-P3"],
        &ep,
        None,
        vec![],
        vec![Ok(frost_reply(&d))],
    );
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(r.calls.len(), 1);
    assert_eq!(r.calls[0].0, "lab_uat_calibrate");
    assert_eq!(
        r.calls[0].1,
        json!({"section": "first-session", "row": "FS-P3", "instance": "default"})
    );
    assert_eq!(
        r.out,
        format!(
            "calibrate first-session FS-P3 on default (budget 15 min)\nfrost: score 8312, confirmed 3/3, bearing 322.5 -> 330.0; diff {d}\n\n{DIFF}\nto apply: git apply \"{d}\" in the checkout (or rerun with --apply)\n"
        )
    );
}

#[test]
fn json_prints_one_object_and_quiet_hides_the_diff() {
    let body = status_body(&[("default", false, None)]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let r = run_flow(
        &[
            "first-session",
            "FS-P3",
            "--pose",
            "frost",
            "--budget-minutes",
            "8",
            "--json",
        ],
        &ep,
        None,
        vec![],
        vec![Ok(frost_reply("D"))],
    );
    assert_eq!(r.code, 0);
    assert_eq!(r.out, "{\"ok\":true,\"section\":\"first-session\",\"row\":\"FS-P3\",\"outcomes\":[{\"pose\":\"frost\",\"ok\":true,\"score\":8312,\"confirmed\":3,\"changed\":{\"bearing\":\"322.5 -> 330.0\"},\"diff\":\"D\"}]}\n");
    assert_eq!(r.calls[0].1["pose"], "frost");
    assert_eq!(r.calls[0].1["budget_minutes"], 8);
}

#[test]
fn no_outcomes_exits_1_and_names_the_run_folder() {
    let body = status_body(&[("default", false, None)]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let r = run_flow(
        &["first-session", "FS-P3"],
        &ep,
        None,
        vec![],
        vec![Ok(json!({"run_dir": "RUN", "outcomes": []}))],
    );
    assert_eq!(r.code, 1);
    assert!(r
        .out
        .contains("lab_uat_calibrate returned no outcomes; run folder RUN\n"));
}

#[test]
fn a_failed_calibrate_call_exits_3() {
    let body = status_body(&[("default", false, None)]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let r = run_flow(
        &["first-session", "FS-P3"],
        &ep,
        None,
        vec![],
        vec![Err(McpError::Unreachable("boom".into()))],
    );
    assert_eq!(r.code, 3);
    assert_eq!(
        r.err,
        "lab_uat_calibrate failed: cannot reach the lab daemon: boom\n"
    );
}

#[test]
fn a_failing_prepare_row_stops_with_exit_3_before_the_sweep() {
    let body = status_body(&[("default", false, None)]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let rows = json!({"rows": [{"row": "FS-01", "result": "PASS", "reasons": []}, {"row": "FS-02", "result": "FAIL", "reasons": ["boom lease-0123456789abcdef"]}]});
    let r = run_flow(
        &["first-session", "FS-P3", "--prepare", "FS-01,FS-02"],
        &ep,
        None,
        vec![],
        vec![Ok(rows)],
    );
    assert_eq!(r.code, 3);
    assert_eq!(r.err, "prepare row FS-02 FAIL: boom lease-<redacted>\n");
    assert_eq!(r.calls.len(), 1, "the sweep never ran");
    assert_eq!(r.calls[0].0, "lab_uat_run");
    assert_eq!(
        r.calls[0].1,
        json!({"sections": ["first-session"], "rows": ["FS-01", "FS-02"], "instance": "default", "end_session": false})
    );
    assert!(r.out.starts_with("prepare: FS-01, FS-02 on default\n"));
}

#[test]
fn a_prepare_that_passes_runs_the_sweep_next() {
    let body = status_body(&[("default", false, None)]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let rows = json!({"rows": [{"row": "FS-01", "result": "PASS", "reasons": []}]});
    let r = run_flow(
        &["first-session", "FS-P3", "--prepare", "FS-01", "--json"],
        &ep,
        None,
        vec![],
        vec![Ok(rows), Ok(frost_reply(""))],
    );
    assert_eq!(
        r.calls.iter().map(|c| c.0.as_str()).collect::<Vec<_>>(),
        ["lab_uat_run", "lab_uat_calibrate"]
    );
    assert_eq!(r.code, 0);
}

#[test]
fn a_foreign_sgw_stops_with_exit_3_and_names_the_pids() {
    let body = status_body(&[("default", false, Some(100))]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let r = run_flow(
        &["first-session", "FS-P3"],
        &ep,
        None,
        vec![100, 7, 9],
        vec![],
    );
    assert_eq!(r.code, 3);
    assert_eq!(r.err, "SGW.exe runs outside the lab (pid 7, 9); close it first, or the lab cannot start its clients\n");
    assert!(r.calls.is_empty());
}

#[test]
fn instance_choice_follows_the_plan_rules() {
    let body = status_body(&[("default", true, None), ("p2", false, None)]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let r = run_flow(
        &["first-session", "FS-P3"],
        &ep,
        None,
        vec![],
        vec![Ok(frost_reply(""))],
    );
    assert_eq!(r.calls[0].1["instance"], "p2", "the first free instance");
    let r = run_flow(
        &["first-session", "FS-P3", "--instance", "default"],
        &ep,
        None,
        vec![],
        vec![],
    );
    assert_eq!(
        (r.code, r.err.as_str()),
        (
            3,
            "instance default is leased by another session; wait for it or pick another\n"
        )
    );
    let r = run_flow(
        &["first-session", "FS-P3", "--instance", "nope"],
        &ep,
        None,
        vec![],
        vec![],
    );
    assert_eq!(
        (r.code, r.err.as_str()),
        (
            2,
            "unknown instance 'nope'; the daemon hosts: default, p2\n"
        )
    );
}

#[test]
fn a_daemon_that_does_not_answer_exits_3() {
    let r = run_flow(
        &["first-session", "FS-P3", "--json"],
        &support::closed_port_endpoint(),
        None,
        vec![],
        vec![],
    );
    assert_eq!(r.code, 3);
    assert_eq!(r.out, "{\"ok\":false,\"exit\":3,\"error\":\"the lab daemon is not answering (lab status; lab start)\"}\n");
}

#[test]
fn apply_on_a_dirty_spec_is_refused_before_the_daemon_is_asked() {
    let (_tmp, repo, spec) = scratch_repo();
    fs::write(&spec, "a\nb\nc\nlocal\n").unwrap();
    let (stub, ep) = StubServer::start(vec![]);
    let r = run_flow(&["x", "FS-P3", "--apply"], &ep, Some(repo), vec![], vec![]);
    assert_eq!(r.code, 2);
    assert_eq!(r.err, "docs/guides/uat-specs/x.toml has uncommitted changes or is not tracked; --apply would clobber them (commit or revert first, or drop --apply)\n");
    assert!(stub.requests().is_empty() && r.calls.is_empty());
}

#[test]
fn apply_on_a_clean_spec_applies_and_never_stages_or_commits() {
    let (tmp, repo, spec) = scratch_repo();
    let body = status_body(&[("default", false, None)]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let diff = tmp.path().join("x.diff");
    fs::write(&diff, DIFF).unwrap();
    let head = git(&repo, &["rev-parse", "HEAD"]);
    let r = run_flow(
        &["x", "FS-P3", "--apply", "--quiet"],
        &ep,
        Some(repo.clone()),
        vec![],
        vec![Ok(frost_reply(&diff.to_string_lossy()))],
    );
    assert_eq!(r.code, 0, "{}", r.err);
    assert_eq!(fs::read_to_string(&spec).unwrap(), "a\nB\nc\n");
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(git(&repo, &["diff", "--cached", "--name-only"]), "");
    assert!(r.out.contains(&format!(
        "applied to the checkout ({}); review it with git diff, nothing is staged or committed\n",
        repo.display()
    )));
}

#[test]
fn a_failed_apply_under_json_keeps_the_proposal_and_exits_2() {
    let (tmp, repo, spec) = scratch_repo();
    let body = status_body(&[("default", false, None)]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let stale = tmp.path().join("stale.diff");
    fs::write(&stale, DIFF.replace("-b", "-zzz")).unwrap();
    let d = stale.to_string_lossy().into_owned();
    let r = run_flow(
        &["x", "FS-P3", "--apply", "--json"],
        &ep,
        Some(repo),
        vec![],
        vec![Ok(frost_reply(&d))],
    );
    assert_eq!(r.code, 2);
    let v: Value = serde_json::from_str(r.out.trim()).unwrap();
    assert_eq!(
        (v["ok"].clone(), v["exit"].clone()),
        (json!(false), json!(2))
    );
    assert!(v["error"].as_str().unwrap().starts_with(&d));
    assert_eq!(
        (v["diff"].as_str(), v["run_dir"].as_str()),
        (Some(d.as_str()), Some("RUN"))
    );
    assert_eq!(fs::read_to_string(&spec).unwrap(), "a\nb\nc\n");
}

#[test]
fn the_call_timeouts_are_the_budget_plus_5_minutes_and_15_minutes_for_prepare() {
    let body = status_body(&[("default", false, None)]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let rows = json!({"rows": [{"row": "FS-01", "result": "PASS", "reasons": []}]});
    let r = run_flow(
        &[
            "first-session",
            "FS-P3",
            "--prepare",
            "FS-01",
            "--budget-minutes",
            "8",
            "--json",
        ],
        &ep,
        None,
        vec![],
        vec![Ok(rows), Ok(frost_reply(""))],
    );
    assert_eq!(
        r.waits,
        [Duration::from_secs(15 * 60), Duration::from_secs(13 * 60)]
    );
    let r = run_flow(
        &["first-session", "FS-P3", "--json"],
        &ep,
        None,
        vec![],
        vec![Ok(frost_reply(""))],
    );
    assert_eq!(r.waits, [Duration::from_secs(20 * 60)]);
}

#[test]
fn apply_without_a_checkout_is_refused_before_the_daemon_is_asked() {
    let (stub, ep) = StubServer::start(vec![]);
    let r = run_flow(
        &["x", "FS-P3", "--apply", "--json"],
        &ep,
        None,
        vec![],
        vec![],
    );
    assert_eq!(r.code, 2);
    let v: Value = serde_json::from_str(r.out.trim()).unwrap();
    assert_eq!(v["exit"], 2);
    assert!(v["error"]
        .as_str()
        .unwrap()
        .starts_with("no checkout was found"));
    assert!(stub.requests().is_empty() && r.calls.is_empty());
}

#[test]
fn apply_on_an_untracked_spec_is_refused_before_the_daemon_is_asked() {
    let (_tmp, repo, _spec) = scratch_repo();
    fs::write(repo.join("docs/guides/uat-specs/y.toml"), "a\nb\nc\n").unwrap();
    let (stub, ep) = StubServer::start(vec![]);
    let r = run_flow(&["y", "FS-P3", "--apply"], &ep, Some(repo), vec![], vec![]);
    assert_eq!(r.code, 2);
    assert!(
        r.err
            .starts_with("docs/guides/uat-specs/y.toml has uncommitted changes or is not tracked"),
        "{}",
        r.err
    );
    assert!(stub.requests().is_empty() && r.calls.is_empty());
}

#[test]
fn a_spec_edited_during_the_sweep_is_refused_again_before_git_apply() {
    let (tmp, repo, spec) = scratch_repo();
    let body = status_body(&[("default", false, None)]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let diff = tmp.path().join("x.diff");
    fs::write(&diff, DIFF).unwrap();
    let d = diff.to_string_lossy().into_owned();
    // The spec is clean when the flow starts and edited while the sweep runs.
    let edit = |name: &str| {
        if name == "lab_uat_calibrate" {
            fs::write(&spec, "a\nb\nc\nlocal\n").unwrap();
        }
    };
    let r = run_flow_hooked(
        &["x", "FS-P3", "--apply"],
        &ep,
        Some(repo.clone()),
        vec![],
        vec![Ok(frost_reply(&d))],
        &edit,
    );
    assert_eq!(r.code, 2);
    assert!(
        r.err.starts_with(&format!(
            "{d}: docs/guides/uat-specs/x.toml has uncommitted changes"
        )),
        "{}",
        r.err
    );
    assert_eq!(fs::read_to_string(&spec).unwrap(), "a\nb\nc\nlocal\n");
    assert_eq!(git(&repo, &["diff", "--cached", "--name-only"]), "");
}
