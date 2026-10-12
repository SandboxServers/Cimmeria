//! LX-15: what `lab uat --plan-only` prints (plain, fancy, `--json`, `--quiet`,
//! `--progress-to-stderr`) and what a ready real run hands to LX-16.

mod support;
mod uat_preflight_support;

use cimmeria_lab_cli::commands::uat::Outcome;
use cimmeria_lab_cli::ui::Ui;
use support::StubServer;
use uat_preflight_support::{one_free, ready, run, run_with, status_body, with_chaos, FANCY};
// ---- --plan-only output ----

// the removed PowerShell test file / the removed PowerShell uat command: "plan: N row(s) ready on <instances>: <rows>"
#[test]
fn plan_only_plain_prints_one_line_and_exits_0() {
    let body = status_body(&[("default", false, None), ("p2", false, None)]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let r = run_with(
        &["first-session", "--plan-only", "--leases", "2"],
        &ep,
        None,
        0,
        None,
        Ui::plain(),
        vec![Ok(ready(&["FS-01", "FS-02"]))],
    );
    assert_eq!(r.code(), Some(0));
    assert_eq!(r.out, "plan: 2 row(s) ready on default, p2: FS-01, FS-02\n");
    assert!(r.err.is_empty());
}

// new: the runner's header leads the plain line
#[test]
fn plan_only_plain_puts_the_chaos_header_first() {
    let reply = with_chaos(ready(&["FS-01"]), &["chaos=cpu:safe seed=<per run>"]);
    let r = run(
        &["first-session", "--plan-only", "--chaos", "cpu"],
        vec![Ok(reply)],
    );
    assert_eq!(
        r.out,
        "chaos=cpu:safe seed=<per run>\nplan: 1 row(s) ready on default: FS-01\n"
    );
}

// new: fancy mode marks the line with a check (no colour here, so only the glyph shows)
#[test]
fn plan_only_fancy_leads_with_a_check_mark() {
    let body = one_free();
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let r = run_with(
        &["first-session", "--plan-only"],
        &ep,
        None,
        0,
        None,
        FANCY,
        vec![Ok(ready(&["FS-01"]))],
    );
    assert_eq!(r.out, "\u{2713} plan: 1 row(s) ready on default: FS-01\n");
}

// new: --json is the one object, no progress
#[test]
fn plan_only_json_is_one_object() {
    let r = run(
        &["first-session", "--plan-only", "--json"],
        vec![Ok(ready(&["FS-01", "FS-02"]))],
    );
    assert_eq!(
        r.out,
        "{\"ok\":true,\"plan_only\":true,\"rows\":[\"FS-01\",\"FS-02\"],\"lanes\":[\"default\"]}\n"
    );
    assert!(r.err.is_empty());
}

// new: --quiet prints nothing; --progress-to-stderr moves the line to stderr, under --json too
#[test]
fn plan_only_progress_goes_where_the_flags_say() {
    let r = run(
        &["first-session", "--plan-only", "--quiet"],
        vec![Ok(ready(&["FS-01"]))],
    );
    assert_eq!(
        (r.code(), r.out.as_str(), r.err.as_str()),
        (Some(0), "", "")
    );
    let r = run(
        &["first-session", "--plan-only", "--progress-to-stderr"],
        vec![Ok(ready(&["FS-01"]))],
    );
    assert_eq!(
        (r.out.as_str(), r.err.as_str()),
        ("", "plan: 1 row(s) ready on default: FS-01\n")
    );
    // under --json the object is stdout; --json has no progress line to move
    let r = run(
        &[
            "first-session",
            "--plan-only",
            "--json",
            "--progress-to-stderr",
        ],
        vec![Ok(ready(&["FS-01"]))],
    );
    assert!(r.out.starts_with("{\"ok\":true"));
    assert!(r.err.is_empty());
}

// ---- a real run ----

// new: a ready real run returns Drive with the plan, and made exactly one call
#[test]
fn a_ready_real_run_returns_drive_after_one_plan_call() {
    let body = status_body(&[("default", false, None), ("p2", false, None)]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let r = run_with(
        &["first-session", "--leases", "2", "--runs-per-lease", "3"],
        &ep,
        None,
        0,
        Some(vec![]),
        Ui::plain(),
        vec![Ok(ready(&["FS-01", "FS-02"]))],
    );
    let Outcome::Drive(ready) = &r.outcome else {
        panic!("expected Drive")
    };
    assert_eq!(ready.section, "first-session");
    assert_eq!(ready.rows, ["FS-01", "FS-02"]);
    assert_eq!(
        ready
            .lanes
            .iter()
            .map(|l| l.instance.as_str())
            .collect::<Vec<_>>(),
        ["default", "p2"]
    );
    assert_eq!(ready.chaos, None);
    assert!(!ready.diff_golden);
    assert_eq!(r.calls.len(), 1);
    assert_eq!(r.calls[0].1["instance"], "default");
    assert!(
        r.out.is_empty() && r.err.is_empty(),
        "a real run prints nothing before it drives"
    );
}
