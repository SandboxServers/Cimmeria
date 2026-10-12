//! LX-15: the `lab uat` pre-flight with a stub /status server and a stub MCP
//! call. Ports the plan, usage and -Chaos checks of the removed PowerShell test file
//! and the removed PowerShell test file that reach the daemon. The `--plan-only` output and
//! the real-run hand-off are in uat_plan_only.rs.

mod support;
mod uat_preflight_support;

use std::fs;

use cimmeria_lab_cli::commands::uat::Outcome;
use cimmeria_lab_cli::daemon::mcp::McpError;
use cimmeria_lab_cli::ui::Ui;
use serde_json::json;
use support::StubServer;
use uat_preflight_support::{
    one_free, ready, run, run_with, specs_dir, status_body, with_chaos, FANCY,
};

// ---- usage errors exit 2 before the daemon is asked ----

// the removed PowerShell test file: 'argument checks need no daemon'; the removed PowerShell test file:
// '-Chaos cpu -Leases 2 exits 2 before the daemon is asked'
#[test]
fn usage_errors_exit_2_before_the_daemon_is_asked() {
    for tokens in [
        vec!["first-session", "--leases", "9"],
        vec!["first-session", "--leases", "2", "--heal"],
        vec!["first-session", "--chaos", "cpu", "--leases", "2"],
        vec!["first-session", "--leases", "2", "--instance", "p2"],
    ] {
        let (stub, ep) = StubServer::start(vec![("GET", "/status", 200, "{}")]);
        let r = run_with(&tokens, &ep, None, 0, None, Ui::plain(), vec![]);
        assert_eq!(r.code(), Some(2), "{tokens:?}");
        assert!(r.calls.is_empty(), "{tokens:?}");
        assert!(
            stub.requests().is_empty(),
            "the daemon was asked for {tokens:?}"
        );
        assert!(r.out.is_empty());
        assert!(!r.err.is_empty());
    }
}

// new: a missing section is the usage line, exit 2, on stderr (plain) or as the --json object
#[test]
fn a_missing_section_prints_the_usage_line() {
    let usage = "usage: lab uat <section> [--rows FS-01,...] [--leases 1-5] [--runs-per-lease 1-20] [--instance p2] [--fixtures auto|always|never] [--diff-golden] [--chaos spec,...] [--heal] [--plan-only] [--json]";
    let r = run(&[], vec![]);
    assert_eq!(r.code(), Some(2));
    assert_eq!(r.err, format!("{usage}\n"));
    assert!(r.out.is_empty());
    let r = run(&["--json"], vec![]);
    assert_eq!(
        r.out,
        format!("{{\"ok\":false,\"exit\":2,\"error\":\"{usage}\"}}\n")
    );
    assert!(r.err.is_empty());
}

// new: fancy mode puts a red `error:` label in front (no colour here, so plain text)
#[test]
fn a_fancy_usage_error_leads_with_error() {
    let body = one_free();
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let r = run_with(
        &["first-session", "--leases", "9"],
        &ep,
        None,
        0,
        None,
        FANCY,
        vec![],
    );
    assert_eq!(r.err, "error: --leases must be 1 to 5\n");
}

// ---- the golden check (the removed PowerShell test file) ----

// '-Chaos with no golden exits 2 before the daemon is asked'
#[test]
fn chaos_without_a_golden_exits_2_before_the_daemon_is_asked() {
    let specs = specs_dir(false);
    let (stub, ep) = StubServer::start(vec![("GET", "/status", 200, "{}")]);
    let dir = specs.path().to_string_lossy().into_owned();
    let r = run_with(
        &["first-session", "--chaos", "flush:pack", "--json"],
        &ep,
        Some(&dir),
        0,
        None,
        Ui::plain(),
        vec![],
    );
    assert_eq!(r.code(), Some(2));
    assert!(stub.requests().is_empty() && r.calls.is_empty());
    assert_eq!(
        r.out,
        "{\"ok\":false,\"exit\":2,\"error\":\"no golden for first-session; --chaos reports against it, so record one first with lab golden record (lab-golden GD-10)\"}\n"
    );
}

// '-Chaos -PlanOnly with no golden plans and exits 0'
#[test]
fn chaos_plan_only_needs_no_golden() {
    let specs = specs_dir(false);
    let body = one_free();
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let dir = specs.path().to_string_lossy().into_owned();
    let reply = with_chaos(ready(&["FS-01"]), &["chaos=flush:pack seed=<per run>"]);
    let r = run_with(
        &[
            "first-session",
            "--plan-only",
            "--chaos",
            "flush:pack",
            "--json",
        ],
        &ep,
        Some(&dir),
        0,
        None,
        Ui::plain(),
        vec![Ok(reply)],
    );
    assert_eq!(r.code(), Some(0));
    assert_eq!(r.calls.len(), 1);
    assert_eq!(r.calls[0].1["plan_only"], true);
}

// 'with no local spec folder -DiffGolden alone still exits 2'
#[test]
fn diff_golden_without_a_spec_folder_exits_2() {
    let r = run(&["first-session", "--diff-golden"], vec![]);
    assert_eq!(r.code(), Some(2));
    assert_eq!(
        r.err,
        "--diff-golden needs --specs-dir (no spec folder found next to this checkout)\n"
    );
    assert!(r.calls.is_empty());
}

// 'with no local spec folder a chaos run is not refused for -SpecsDir'
#[test]
fn chaos_without_a_spec_folder_is_left_to_the_daemon() {
    let r = run(
        &["first-session", "--chaos", "cpu"],
        vec![Err(McpError::Tool("stub: no run".into()))],
    );
    assert_eq!(r.code(), Some(3));
    assert_eq!(r.calls.len(), 1);
    assert!(r.calls[0].1.get("specs_dir").is_none());
}

// new: --diff-golden with a golden present passes and asks for fingerprints
#[test]
fn diff_golden_with_a_golden_plans_with_fingerprints() {
    let specs = specs_dir(true);
    let body = one_free();
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let dir = specs.path().to_string_lossy().into_owned();
    let r = run_with(
        &["first-session", "--diff-golden"],
        &ep,
        Some(&dir),
        0,
        Some(vec![]),
        Ui::plain(),
        vec![Ok(ready(&["FS-01"]))],
    );
    assert!(matches!(r.outcome, Outcome::Drive(_)));
    assert_eq!(r.calls[0].1["fingerprint"], true);
}

// new: a golden that is not a golden is exit 2 naming the section
#[test]
fn an_unreadable_golden_exits_2() {
    let specs = specs_dir(false);
    fs::create_dir_all(specs.path().join("golden")).unwrap();
    fs::write(specs.path().join("golden").join("first-session.json"), "{").unwrap();
    let body = one_free();
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let dir = specs.path().to_string_lossy().into_owned();
    let r = run_with(
        &["first-session", "--diff-golden"],
        &ep,
        Some(&dir),
        0,
        None,
        Ui::plain(),
        vec![],
    );
    assert_eq!(r.code(), Some(2));
    assert!(
        r.err
            .starts_with("golden for first-session cannot be read: "),
        "{}",
        r.err
    );
}

// ---- the request body (the removed PowerShell test file) ----

// 'two -Chaos values both reach the request body'; '-Chaos implies
// -Fingerprint: the request asks for fingerprint capture (CH-09)'
#[test]
fn two_chaos_values_reach_the_plan_call_and_imply_a_fingerprint() {
    let r = run(
        &[
            "first-session",
            "--plan-only",
            "--chaos",
            "cpu",
            "--chaos",
            "flush:pack",
        ],
        vec![Ok(ready(&["FS-01"]))],
    );
    assert_eq!(r.code(), Some(0));
    assert_eq!(r.calls[0].0, "lab_uat_run");
    assert_eq!(
        r.calls[0].1,
        json!({"sections": ["first-session"], "chaos": ["cpu", "flush:pack"], "fingerprint": true, "plan_only": true, "instance": "default"})
    );
}

// 'a quoted multi-knob spec reaches the body as one element'
#[test]
fn a_multi_knob_spec_is_one_element() {
    let r = run(
        &[
            "first-session",
            "--plan-only",
            "--chaos",
            "net:latency=120,loss=1",
        ],
        vec![Ok(ready(&["FS-01"]))],
    );
    assert_eq!(r.calls[0].1["chaos"], json!(["net:latency=120,loss=1"]));
}

// 'without -Chaos the body has no chaos or fingerprint key'
#[test]
fn without_chaos_the_body_has_no_chaos_or_fingerprint() {
    let r = run(
        &["first-session", "--plan-only"],
        vec![Ok(ready(&["FS-01"]))],
    );
    assert_eq!(
        r.calls[0].1,
        json!({"sections": ["first-session"], "plan_only": true, "instance": "default"})
    );
}

// new: every other option reaches the plan call
#[test]
fn rows_specs_server_version_and_fixtures_reach_the_plan_call() {
    let specs = specs_dir(false);
    let dir = specs.path().to_string_lossy().into_owned();
    let r = run(
        &[
            "first-session",
            "--plan-only",
            "--rows",
            "FS-01,FS-02,FS-01",
            "--specs-dir",
            &dir,
            "--server-version",
            "abc1234",
            "--fixtures",
            "always",
        ],
        vec![Ok(ready(&["FS-01", "FS-02"]))],
    );
    assert_eq!(
        r.calls[0].1,
        json!({"sections": ["first-session"], "rows": ["FS-01", "FS-02"], "specs_dir": dir, "server_version": "abc1234", "fixtures": "always", "plan_only": true, "instance": "default"})
    );
}

// F-LX7: a relative --specs-dir reaches the plan call as an absolute path.
#[test]
fn a_relative_specs_dir_reaches_the_plan_call_absolute() {
    let r = run(
        &["first-session", "--plan-only", "--specs-dir", "lx18-specs"],
        vec![Ok(ready(&["FS-01"]))],
    );
    let want = std::env::current_dir()
        .unwrap()
        .join("lx18-specs")
        .display()
        .to_string();
    assert_eq!(r.calls[0].1["specs_dir"], json!(want));
}

// ---- the plan call's answer (the removed PowerShell test file) ----

// "the plan prints the runner's header in place of the typed one"
#[test]
fn the_plan_prints_the_runners_header_in_place_of_the_typed_one() {
    let reply = with_chaos(
        ready(&["FS-01"]),
        &["chaos=cpu:safe seed=<per run>", "chaos=flush:pack seed=5"],
    );
    let r = run(
        &[
            "first-session",
            "--plan-only",
            "--json",
            "--chaos",
            "cpu",
            "--chaos",
            "flush:pack",
        ],
        vec![Ok(reply)],
    );
    assert_eq!(r.code(), Some(0));
    assert_eq!(
        r.out,
        "{\"ok\":true,\"plan_only\":true,\"rows\":[\"FS-01\"],\"lanes\":[\"default\"],\"chaos\":\"chaos=cpu:safe seed=<per run>; chaos=flush:pack seed=5\"}\n"
    );
}

// new: no runner header, the typed one stays
#[test]
fn the_typed_header_stays_when_the_runner_returns_none() {
    let r = run(
        &["first-session", "--plan-only", "--chaos", "cpu"],
        vec![Ok(ready(&["FS-01"]))],
    );
    assert_eq!(r.out, "chaos=cpu\nplan: 1 row(s) ready on default: FS-01\n");
}

// 'a spec the runner refuses fails the plan call with exit 3';
// '-Chaos burst -PlanOnly with no golden still exits 3 from the plan call';
// 'an unknown profile with -PlanOnly and no golden still exits 3'
#[test]
fn a_spec_the_runner_refuses_fails_the_plan_call_with_exit_3() {
    for text in [
        "Invalid params: chaos \"burst\": burst is not wired yet; it waits on CH-13 (.spawnrandom ids, .despawn <id>)",
        "Invalid params: chaos \"storm\": unknown profile",
    ] {
        let r = run(&["first-session", "--plan-only", "--json", "--chaos", "x"], vec![Err(McpError::Tool(text.into()))]);
        assert_eq!(r.code(), Some(3));
        assert_eq!(r.out, format!("{{\"ok\":false,\"exit\":3,\"error\":\"the plan call failed: {}\"}}\n", text.replace('"', "\\\"")));
    }
}

// 'a real run with -Chaos sends it to the plan call'
#[test]
fn a_real_run_with_chaos_sends_it_to_the_plan_call() {
    let r = run(
        &["first-session", "--chaos", "cpu"],
        vec![Err(McpError::Tool("stub: no run".into()))],
    );
    assert_eq!(r.code(), Some(3));
    assert_eq!(r.calls[0].1["chaos"], json!(["cpu"]));
    assert_eq!(r.calls[0].1["plan_only"], true);
    assert_eq!(r.err, "the plan call failed: stub: no run\n");
}

// new: only the first line of the failure, with lease ids hidden
#[test]
fn the_plan_failure_keeps_the_first_line_and_hides_lease_ids() {
    let r = run(
        &["first-session"],
        vec![Err(McpError::Tool(
            "boom lease-0123456789abcdef\nsecond line".into(),
        ))],
    );
    assert_eq!(r.err, "the plan call failed: boom lease-<redacted>\n");
}

// new: a daemon that is down
#[test]
fn a_daemon_that_is_down_exits_3() {
    let ep = support::closed_port_endpoint();
    let r = run_with(
        &["first-session", "--plan-only"],
        &ep,
        None,
        0,
        None,
        Ui::plain(),
        vec![],
    );
    assert_eq!(r.code(), Some(3));
    assert_eq!(
        r.err,
        "the lab daemon is not answering (lab status; lab start)\n"
    );
    assert!(r.calls.is_empty());
}

// new: the plan's own checks
#[test]
fn a_requested_row_the_section_lacks_exits_2() {
    let r = run(
        &["first-session", "--rows", "FS-01,fs-02", "--plan-only"],
        vec![Ok(ready(&["FS-01", "FS-02"]))],
    );
    assert_eq!(r.code(), Some(2));
    assert_eq!(
        r.err,
        "section first-session has no row(s) fs-02 (row ids are exact and case sensitive)\n"
    );
}

#[test]
fn a_section_with_no_rows_exits_2() {
    let r = run(
        &["first-session", "--plan-only"],
        vec![Ok(json!({"rows": []}))],
    );
    assert_eq!(r.err, "no rows matched in section first-session\n");
    assert_eq!(r.code(), Some(2));
}

#[test]
fn a_row_that_is_not_ready_exits_3() {
    let reply = json!({"rows": [{"row": "FS-01", "result": "BLOCKED", "reasons": ["no tool lease-0123456789abcdef"]}]});
    let r = run(&["first-session", "--plan-only"], vec![Ok(reply)]);
    assert_eq!(r.code(), Some(3));
    assert_eq!(
        r.err,
        "not ready: FS-01 BLOCKED: no tool lease-<redacted>\n"
    );
}

// ---- instances and foreign clients ----

// the removed PowerShell test file planning, through the daemon's /status: the first free instance,
// a leased -Instance, an unknown one, the client cap
#[test]
fn the_lane_is_the_first_free_instance() {
    let body = status_body(&[("default", true, None), ("p2", false, None)]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let r = run_with(
        &["first-session", "--plan-only"],
        &ep,
        None,
        0,
        None,
        Ui::plain(),
        vec![Ok(ready(&["FS-01"]))],
    );
    assert_eq!(r.calls[0].1["instance"], "p2");
    assert_eq!(r.out, "plan: 1 row(s) ready on p2: FS-01\n");
    let r = run_with(
        &["first-session", "--instance", "default"],
        &ep,
        None,
        0,
        None,
        Ui::plain(),
        vec![],
    );
    assert_eq!(
        (r.code(), r.err.as_str()),
        (
            Some(3),
            "instance default is leased by another session; wait for it or pick another\n"
        )
    );
    let r = run_with(
        &["first-session", "--instance", "nope"],
        &ep,
        None,
        0,
        None,
        Ui::plain(),
        vec![],
    );
    assert_eq!(
        (r.code(), r.err.as_str()),
        (
            Some(2),
            "unknown instance 'nope'; the daemon hosts: default, p2\n"
        )
    );
    let r = run_with(
        &["first-session", "--leases", "2"],
        &ep,
        None,
        0,
        None,
        Ui::plain(),
        vec![],
    );
    assert_eq!(
        r.err,
        "only 1 free instance(s) for 2 lease(s); leased by other sessions: default\n"
    );
    assert!(r.calls.is_empty());
}

#[test]
fn the_daemons_client_cap_limits_leases() {
    let body = status_body(&[
        ("default", false, None),
        ("p2", false, None),
        ("p3", false, None),
    ]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let r = run_with(
        &["first-session", "--leases", "3", "--plan-only"],
        &ep,
        None,
        2,
        None,
        Ui::plain(),
        vec![],
    );
    assert_eq!(
        r.err,
        "--leases 3 is more than the lab allows (2: 3 hosted, max clients 2)\n"
    );
}

// new: SGW.exe outside the lab stops a real run before the plan call
#[test]
fn a_foreign_sgw_stops_a_real_run_with_exit_3() {
    let body = status_body(&[("default", false, Some(100))]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let r = run_with(
        &["first-session"],
        &ep,
        None,
        0,
        Some(vec![100, 7, 9]),
        Ui::plain(),
        vec![],
    );
    assert_eq!(r.code(), Some(3));
    assert_eq!(r.err, "SGW.exe runs outside the lab (pid 7, 9); close it first, or the lab cannot start its clients\n");
    assert!(r.calls.is_empty());
}

// new: --plan-only drives nothing, so it never looks at SGW.exe (the closure panics if called)
#[test]
fn plan_only_does_not_scan_for_sgw() {
    let body = one_free();
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    // `None`: the scan closure panics when called, so a scan fails the test.
    let r = run_with(
        &["first-session", "--plan-only"],
        &ep,
        None,
        0,
        None,
        Ui::plain(),
        vec![Ok(ready(&["FS-01"]))],
    );
    assert_eq!(r.code(), Some(0));
    // A foreign SGW.exe does not stop a plan either (a real run exits 3 on it).
    let r = run_with(
        &["first-session", "--plan-only"],
        &ep,
        None,
        0,
        Some(vec![7]),
        Ui::plain(),
        vec![Ok(ready(&["FS-01"]))],
    );
    assert_eq!(r.code(), Some(0));
}

// new: the lab's own client is not foreign
#[test]
fn the_labs_own_client_is_not_foreign() {
    let body = status_body(&[("default", false, Some(100))]);
    let (_stub, ep) = StubServer::start(vec![("GET", "/status", 200, body.as_str())]);
    let r = run_with(
        &["first-session"],
        &ep,
        None,
        0,
        Some(vec![100]),
        Ui::plain(),
        vec![Ok(ready(&["FS-01"]))],
    );
    assert!(matches!(r.outcome, Outcome::Drive(_)));
}
