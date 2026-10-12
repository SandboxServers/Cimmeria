//! The `lab clients stop` flow with the daemon and the stopper faked. Ports
//! the exit-code checks of tools/lab/cli/test-ops.ps1 and adds the rest of the
//! exit-code contract.

use super::targets::{DAEMON_DOWN, FORCE_NEEDS_NAME, USAGE};
use super::{execute, Args, StopOutcome, Stream};
use crate::daemon::status::{parse_status, StatusReply};
use crate::ui::Ui;

fn fake_status() -> StatusReply {
    let body = serde_json::json!({
        "daemon": {"pid": 4242, "version": "0.1.0", "uptime_s": 60},
        "instances": [
            {"instance": "default", "account": "lab", "bridge_port": 8770, "client_pid": 101,
             "lease": {"held": true, "lease": {"id": "LEASE-ID-0001", "owner": "lp07-lab", "purpose": "UAT", "remaining_s": 600}}},
            {"instance": "p2", "account": "lab2", "bridge_port": 8771, "client_pid": 202,
             "lease": {"held": false}},
            {"instance": "p3", "account": null, "bridge_port": 8772, "client_pid": null,
             "lease": {"held": false}}
        ]
    });
    parse_status(&body).expect("shape")
}

fn stopped(_pid: u32) -> Result<StopOutcome, String> {
    Ok(StopOutcome::Stopped)
}
fn not_sgw(_pid: u32) -> Result<StopOutcome, String> {
    Ok(StopOutcome::NotSgw)
}
fn failed(_pid: u32) -> Result<StopOutcome, String> {
    Ok(StopOutcome::Failed)
}
fn windows_only(_pid: u32) -> Result<StopOutcome, String> {
    Err("Windows only".to_string())
}

fn fancy() -> Ui {
    // Glyphs on, colour off: the text is exact and has no escape codes.
    Ui {
        color: false,
        fancy: true,
        width: None,
    }
}

struct Run {
    code: u8,
    out: Vec<String>,
    err: Vec<String>,
    stopped: Vec<u32>,
    asked_daemon: bool,
}

fn go(
    verb: Option<&str>,
    which: Option<&str>,
    force: bool,
    status: Option<StatusReply>,
    stopper: fn(u32) -> Result<StopOutcome, String>,
    ui: &Ui,
) -> Run {
    let args = Args {
        verb: verb.map(str::to_string),
        which: which.map(str::to_string),
        force,
    };
    let (mut out, mut err, mut pids, mut asked) = (Vec::new(), Vec::new(), Vec::new(), false);
    let code = execute(
        &args,
        ui,
        || {
            asked = true;
            status
        },
        &mut |pid| {
            pids.push(pid);
            stopper(pid)
        },
        &mut |stream, line| match stream {
            Stream::Out => out.push(line),
            Stream::Err => err.push(line),
        },
    );
    Run {
        code,
        out,
        err,
        stopped: pids,
        asked_daemon: asked,
    }
}

fn stop(which: Option<&str>, force: bool, stopper: fn(u32) -> Result<StopOutcome, String>) -> Run {
    go(
        Some("stop"),
        which,
        force,
        Some(fake_status()),
        stopper,
        &Ui::plain(),
    )
}

// ports: "'clients stop all -Force' exits 2"
#[test]
fn stop_all_with_force_exits_2_before_the_daemon_is_asked() {
    let run = go(Some("stop"), Some("all"), true, None, stopped, &Ui::plain());
    assert_eq!(run.code, 2);
    assert!(!run.asked_daemon, "the daemon must not be asked");
    assert_eq!(run.err, vec![FORCE_NEEDS_NAME.to_string()]);
    assert!(run.stopped.is_empty());
}

// ports: "'clients stop -Force' with no instance exits 2"
#[test]
fn stop_with_force_and_no_instance_exits_2_before_the_daemon_is_asked() {
    let run = go(Some("stop"), None, true, None, stopped, &Ui::plain());
    assert_eq!(run.code, 2);
    assert!(!run.asked_daemon);
    assert_eq!(run.err, vec![FORCE_NEEDS_NAME.to_string()]);
}

// ports: 'a verb other than stop exits 2'
#[test]
fn a_verb_other_than_stop_exits_2_with_the_usage_line() {
    for verb in [Some("start"), None] {
        let run = go(verb, None, false, None, stopped, &Ui::plain());
        assert_eq!(run.code, 2);
        assert!(!run.asked_daemon);
        assert_eq!(run.err, vec![USAGE.to_string()]);
    }
}

// new: exit 1
#[test]
fn no_status_exits_1_and_stops_nothing() {
    let run = go(Some("stop"), Some("p2"), false, None, stopped, &Ui::plain());
    assert_eq!(run.code, 1);
    assert!(run.asked_daemon);
    assert_eq!(run.err, vec![DAEMON_DOWN.to_string()]);
    assert!(run.stopped.is_empty());
}

// new: exit 2 for an unknown instance
#[test]
fn an_unknown_instance_exits_2_and_lists_the_hosted_ones() {
    let run = stop(Some("nope"), false, stopped);
    assert_eq!(run.code, 2);
    assert_eq!(
        run.err,
        vec!["unknown instance 'nope'; the daemon hosts: default, p2, p3".to_string()]
    );
    assert!(run.stopped.is_empty());
}

// new: exit 3, and the output order (none, refused, then stops)
#[test]
fn all_stops_the_free_client_and_refuses_the_leased_one_with_exit_3() {
    let run = stop(Some("all"), false, stopped);
    assert_eq!(run.code, 3);
    assert_eq!(run.stopped, vec![202]);
    assert_eq!(
        run.out,
        vec![
            "p3: no client".to_string(),
            "default: leased to lp07-lab (UAT); not stopped (use --force to override)".to_string(),
            "p2: stopped pid 202".to_string(),
        ]
    );
    assert!(run.err.is_empty());
}

// new: a leased instance named without --force is refused and never stopped
#[test]
fn a_leased_instance_by_name_without_force_exits_3_and_stops_nothing() {
    let run = stop(Some("default"), false, stopped);
    assert_eq!(run.code, 3);
    assert!(
        run.stopped.is_empty(),
        "a leased client must not be stopped"
    );
    assert_eq!(
        run.out,
        vec![
            "default: leased to lp07-lab (UAT); not stopped (use --force to override)".to_string()
        ]
    );
}

// new: the refusal and the forced warning name the holder, never the lease id
#[test]
fn no_line_carries_the_lease_id() {
    let mut lines = stop(Some("all"), false, stopped).out;
    lines.extend(stop(Some("default"), true, stopped).out);
    let fancy_run = go(
        Some("stop"),
        Some("default"),
        true,
        Some(fake_status()),
        stopped,
        &fancy(),
    );
    lines.extend(fancy_run.out);
    assert!(lines.iter().any(|l| l.contains("lp07-lab")));
    assert!(
        lines.iter().all(|l| !l.contains("LEASE-ID")),
        "lines: {lines:?}"
    );
}

// new: --force with `all` in any case exits 2 before the daemon is asked
#[test]
fn stop_all_in_capitals_with_force_exits_2_before_the_daemon_is_asked() {
    let run = go(Some("STOP"), Some("ALL"), true, None, stopped, &Ui::plain());
    assert_eq!(run.code, 2);
    assert!(!run.asked_daemon);
    assert!(run.stopped.is_empty());
}

// new: exit 0 with the forced warning
#[test]
fn a_leased_instance_by_name_with_force_warns_stops_and_exits_0() {
    let run = stop(Some("default"), true, stopped);
    assert_eq!(run.code, 0);
    assert_eq!(run.stopped, vec![101]);
    assert_eq!(
        run.out,
        vec![
            "default: leased to lp07-lab (UAT); stopping anyway (--force). The holder loses the client and the watchdog may relaunch it.".to_string(),
            "default: stopped pid 101".to_string(),
        ]
    );
}

// new: case-insensitive instance name
#[test]
fn an_instance_name_in_another_case_is_stopped() {
    let run = stop(Some("P2"), false, stopped);
    assert_eq!((run.code, run.stopped), (0, vec![202]));
}

// new: exit 4 beats exit 3
#[test]
fn a_client_that_stays_alive_exits_4_even_when_another_was_refused() {
    let run = stop(Some("all"), false, failed);
    assert_eq!(run.code, 4);
    assert_eq!(run.err, vec!["p2: could not stop pid 202".to_string()]);
}

// new: a stop that cannot be tried counts as a failure
#[test]
fn a_stop_that_cannot_be_tried_exits_4_with_the_reason() {
    let run = stop(Some("p2"), false, windows_only);
    assert_eq!(run.code, 4);
    assert_eq!(
        run.err,
        vec!["p2: could not stop pid 202: Windows only".to_string()]
    );
}

// new: a pid that is not SGW is reported, not failed
#[test]
fn a_pid_that_is_not_sgw_prints_no_client_and_exits_0() {
    let run = stop(Some("p2"), false, not_sgw);
    assert_eq!(run.code, 0);
    assert_eq!(
        run.out,
        vec!["p2: no client (pid 202 is not a running SGW)".to_string()]
    );
}

// new: fancy mode, one glyph line per instance
#[test]
fn fancy_mode_prints_one_glyph_line_per_instance() {
    let run = go(
        Some("stop"),
        Some("all"),
        false,
        Some(fake_status()),
        stopped,
        &fancy(),
    );
    assert_eq!(
        run.out,
        vec![
            "○ p3: no client".to_string(),
            "✗ refused default: leased by lp07-lab (UAT)  use --force to override".to_string(),
            "✓ stopped p2 (pid 202)".to_string(),
        ]
    );
}

#[test]
fn fancy_errors_carry_the_error_label_and_a_hint() {
    let run = go(Some("stop"), Some("all"), true, None, stopped, &fancy());
    assert_eq!(
        run.err,
        vec![
            "error: --force needs a named instance".to_string(),
            "  hint: name the instance: lab clients stop p2 --force".to_string(),
        ]
    );
}
