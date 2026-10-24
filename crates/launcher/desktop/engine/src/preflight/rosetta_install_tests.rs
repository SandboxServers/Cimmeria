use super::*;
use crate::preflight::events::{dependency_install_started, Dependency};
use crate::test_capture::{capture, Captured};

fn only_event(events: &[Captured]) -> &Captured {
    assert_eq!(events.len(), 1, "expected one event, got {events:?}");
    &events[0]
}

#[test]
fn a_zero_exit_with_rosetta_still_missing_is_a_failure() {
    assert_eq!(
        classify_install(Some(0), "", RosettaProbe::Missing),
        InstallOutcome::Failed { exit_code: Some(0) }
    );
}

#[test]
fn a_cancelled_admin_prompt_is_declined() {
    assert_eq!(
        classify_install(
            Some(1),
            "0:73: execution error: User canceled. (-128)",
            RosettaProbe::Missing
        ),
        InstallOutcome::Declined
    );
}

#[test]
fn rosetta_present_after_a_failing_exit_is_installed() {
    assert_eq!(
        classify_install(Some(1), "", RosettaProbe::Present),
        InstallOutcome::Installed
    );
}

#[test]
fn a_probe_that_cannot_answer_after_install_is_a_failure() {
    assert_eq!(
        classify_install(None, "", RosettaProbe::TimedOut),
        InstallOutcome::Failed { exit_code: None }
    );
}

#[test]
fn the_install_command_is_the_fixed_constant() {
    assert_eq!(OSASCRIPT, "/usr/bin/osascript");
    assert_eq!(
        INSTALL_SCRIPT,
        r#"do shell script "/usr/sbin/softwareupdate --install-rosetta --agree-to-license" with administrator privileges"#
    );
}

#[test]
fn the_stderr_tail_keeps_only_the_last_bytes() {
    let mut buf = vec![b'a'; STDERR_TAIL_BYTES + 10];
    buf.extend_from_slice(b"(-128)");
    keep_tail(&mut buf);
    assert_eq!(buf.len(), STDERR_TAIL_BYTES);
    assert!(buf.ends_with(b"(-128)"));
}

#[test]
fn the_started_event_names_the_dependency_and_its_source() {
    let ((), events) = capture(|| dependency_install_started(Dependency::Rosetta, None));
    let event = only_event(&events);
    assert_eq!(event.target, "launcher.install");
    assert_eq!(event.level, tracing::Level::INFO);
    assert_eq!(event.field("event"), Some("dependency_install_started"));
    assert_eq!(event.field("dependency"), Some("rosetta"));
    assert_eq!(event.field("source"), Some("apple_software_update"));
    assert_eq!(event.field("download_bytes"), None);

    let ((), events) = capture(|| dependency_install_started(Dependency::WineRuntime, Some(42)));
    let event = only_event(&events);
    assert_eq!(event.field("source"), Some("wowsilicon_release"));
    assert_eq!(event.field("download_bytes"), Some("42"));
}

#[test]
fn an_installed_outcome_logs_info_without_a_failure_code() {
    let ((), events) = capture(|| emit_finished(InstallOutcome::Installed, Some(1), 1200));
    let event = only_event(&events);
    assert_eq!(event.target, "launcher.install");
    assert_eq!(event.level, tracing::Level::INFO);
    assert_eq!(event.field("event"), Some("dependency_install_finished"));
    assert_eq!(event.field("dependency"), Some("rosetta"));
    assert_eq!(event.field("outcome"), Some("installed"));
    assert_eq!(event.field("failure_code"), None);
    assert_eq!(event.field("elapsed_ms"), Some("1200"));
    assert_eq!(event.field("exit_code"), Some("1"));
}

#[test]
fn unsuccessful_outcomes_log_warn_with_their_failure_code() {
    let cases = [
        (
            InstallOutcome::Failed { exit_code: Some(1) },
            Some(1),
            "failed",
            "rosetta_install_failed",
            Some("1"),
        ),
        (
            InstallOutcome::Declined,
            Some(1),
            "declined",
            "rosetta_install_declined",
            Some("1"),
        ),
        (
            InstallOutcome::TimedOut,
            None,
            "timed_out",
            "rosetta_install_failed",
            None,
        ),
    ];
    for (outcome, exit_code_in, code, failure, exit_code) in cases {
        let ((), events) = capture(|| emit_finished(outcome, exit_code_in, 7));
        let event = only_event(&events);
        assert_eq!(event.target, "launcher.install");
        assert_eq!(event.level, tracing::Level::WARN, "{outcome:?}");
        assert_eq!(event.field("event"), Some("dependency_install_finished"));
        assert_eq!(event.field("outcome"), Some(code));
        assert_eq!(event.field("failure_code"), Some(failure));
        assert_eq!(event.field("elapsed_ms"), Some("7"));
        assert_eq!(event.field("exit_code"), exit_code, "{outcome:?}");
    }
}

#[cfg(not(target_os = "macos"))]
#[test]
fn off_macos_nothing_is_installed() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let (outcome, events) = capture(|| runtime.block_on(install_rosetta()));
    assert_eq!(outcome, InstallOutcome::Failed { exit_code: None });
    assert!(events.is_empty());
}

#[test]
fn a_timeout_with_rosetta_present_after_is_installed() {
    assert_eq!(
        classify_timeout(RosettaProbe::Present),
        InstallOutcome::Installed
    );
    for probe in [
        RosettaProbe::Missing,
        RosettaProbe::TimedOut,
        RosettaProbe::Error,
    ] {
        assert_eq!(
            classify_timeout(probe),
            InstallOutcome::TimedOut,
            "{probe:?}"
        );
    }
}

#[test]
fn only_one_install_holds_the_gate_at_a_time() {
    let gate = InstallGate::new();
    let now = std::time::Instant::now();
    let permit = gate.try_acquire(now).expect("a free gate is taken");
    assert!(
        gate.try_acquire(now).is_none(),
        "a second install must wait"
    );
    drop(permit);
    assert!(
        gate.try_acquire(now).is_some(),
        "dropping the permit frees the gate"
    );
}

#[test]
fn a_cancelled_install_frees_the_gate() {
    let gate = InstallGate::new();
    let now = std::time::Instant::now();
    let install = async {
        let _permit = gate.try_acquire(now);
        std::future::pending::<()>().await;
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    runtime.block_on(async {
        let timed_out = tokio::time::timeout(std::time::Duration::from_millis(1), install).await;
        assert!(timed_out.is_err());
    });
    assert!(gate.try_acquire(now).is_some());
}

#[test]
fn a_timed_out_install_holds_retries_off_for_the_cooldown() {
    let gate = InstallGate::new();
    let now = std::time::Instant::now();
    let until = now + TIMED_OUT_COOLDOWN;
    gate.try_acquire(now)
        .expect("a free gate is taken")
        .release_with_cooldown(until);
    assert!(
        gate.try_acquire(now).is_none(),
        "retry during the cool-down"
    );
    assert!(gate
        .try_acquire(until - std::time::Duration::from_secs(1))
        .is_none());
    assert!(gate.try_acquire(until).is_some(), "the cool-down ends");
}
