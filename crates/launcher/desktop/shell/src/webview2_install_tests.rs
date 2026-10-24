use super::*;
use std::cell::Cell;

const HTML_PAGE: &[u8] = b"<!DOCTYPE html><html><body>Sign in to the network</body></html>";
const EXECUTABLE: &[u8] = b"MZ\x90\x00\x03\x00\x00\x00";

#[test]
fn only_an_mz_header_is_an_executable() {
    assert!(is_executable(EXECUTABLE));
    assert!(!is_executable(HTML_PAGE));
    assert!(!is_executable(b""));
    assert!(!is_executable(b"M"));
}

#[test]
fn startup_continues_only_after_an_exit_that_left_a_runtime() {
    assert_eq!(after_install(true, InstallOutcome::Exited), Next::Continue);
    assert_eq!(after_install(false, InstallOutcome::Exited), Next::FallBack);
    assert_eq!(
        after_install(true, InstallOutcome::TimedOut),
        Next::FallBack
    );
    assert_eq!(
        after_install(false, InstallOutcome::TimedOut),
        Next::FallBack
    );
    assert_eq!(after_install(true, InstallOutcome::Failed), Next::FallBack);
    assert_eq!(after_install(false, InstallOutcome::Failed), Next::FallBack);
}

#[test]
fn a_served_html_page_is_never_run() {
    let ran = Cell::new(false);
    let result = install_with(
        || Some(HTML_PAGE.to_vec()),
        |_| {
            ran.set(true);
            InstallOutcome::Exited
        },
        || true,
    );
    assert!(!ran.get(), "the HTML page reached the installer step");
    assert_eq!(result, Err(Cause::NotAnExecutable));
}

#[test]
fn a_failed_download_falls_back_without_running_anything() {
    let ran = Cell::new(false);
    let result = install_with(
        || None,
        |_| {
            ran.set(true);
            InstallOutcome::Exited
        },
        || true,
    );
    assert!(!ran.get());
    assert_eq!(result, Err(Cause::DownloadFailed));
}

#[test]
fn an_executable_runs_once_and_the_registry_decides() {
    let runs = Cell::new(0);
    let run = |bytes: &[u8]| {
        assert_eq!(bytes, EXECUTABLE);
        runs.set(runs.get() + 1);
        InstallOutcome::Exited
    };
    assert_eq!(
        install_with(|| Some(EXECUTABLE.to_vec()), run, || true),
        Ok(())
    );
    assert_eq!(runs.get(), 1);
    assert_eq!(
        install_with(|| Some(EXECUTABLE.to_vec()), run, || false),
        Err(Cause::NoRuntimeAfterExit)
    );
    assert_eq!(runs.get(), 2);
}

#[test]
fn the_bootstrapper_gets_silent_first() {
    assert_eq!(BOOTSTRAPPER_ARGS, ["/silent", "/install"]);
}

#[test]
fn a_timeout_or_a_failed_run_names_its_cause() {
    let timed_out = install_with(
        || Some(EXECUTABLE.to_vec()),
        |_| InstallOutcome::TimedOut,
        || true,
    );
    assert_eq!(timed_out, Err(Cause::TimedOut));
    let failed = install_with(
        || Some(EXECUTABLE.to_vec()),
        |_| InstallOutcome::Failed,
        || false,
    );
    assert_eq!(failed, Err(Cause::CouldNotStart));
}

#[test]
fn every_cause_has_its_own_sentence() {
    assert_eq!(
        Cause::DownloadFailed.sentence(),
        "The installer could not be downloaded."
    );
    assert_eq!(
        Cause::NotAnExecutable.sentence(),
        "The download was not a Windows installer."
    );
    assert_eq!(
        Cause::CouldNotStart.sentence(),
        "The installer could not be started."
    );
    assert_eq!(
        Cause::TimedOut.sentence(),
        "The installer was still running after 5 minutes."
    );
    assert_eq!(
        Cause::NoRuntimeAfterExit.sentence(),
        "The installer finished, but the runtime is still missing."
    );
}

#[test]
fn only_a_lock_held_elsewhere_stops_a_start() {
    assert_eq!(setup_gate(SetupLock::Created), Gate::Proceed);
    assert_eq!(setup_gate(SetupLock::Unavailable), Gate::Proceed);
    assert_eq!(setup_gate(SetupLock::AlreadyExists), Gate::AlreadyRunning);
}

#[test]
fn with_notice_returns_the_work_result() {
    assert_eq!(with_notice(|| 7, || {}, || {}).ok(), Some(7));
}

#[test]
fn the_notice_comes_back_when_dismissed_early() {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let mut shows = 0;
    let result = with_notice(
        move || rx.recv_timeout(Duration::from_secs(5)).is_ok(),
        || {
            shows += 1;
            if shows == 3 {
                let _ = tx.send(());
            }
        },
        || {},
    );
    assert_eq!(result.ok(), Some(true));
    assert!(shows >= 3);
}

#[test]
fn the_worker_closes_a_notice_that_is_still_up() {
    use std::sync::{Condvar, Mutex};
    let state = (Mutex::new(false), Condvar::new());
    let mut woken = false;
    let result = with_notice(
        || std::thread::sleep(Duration::from_millis(50)),
        || {
            let (lock, cv) = &state;
            let (guard, timeout) = cv
                .wait_timeout_while(lock.lock().unwrap(), Duration::from_secs(5), |closed| {
                    !*closed
                })
                .unwrap();
            woken = *guard && !timeout.timed_out();
        },
        || {
            let (lock, cv) = &state;
            *lock.lock().unwrap() = true;
            cv.notify_all();
        },
    );
    assert!(result.is_ok());
    assert!(woken, "the notice was not closed by the worker");
}

#[test]
fn a_panicking_install_still_ends_the_notice() {
    let result = with_notice(|| -> u8 { panic!("install blew up") }, || {}, || {});
    assert!(result.is_err());
}

#[test]
fn the_notice_texts_say_what_they_promise() {
    assert!(INSTALLING_NOTICE.contains("closes by itself"));
    assert!(ALREADY_SETTING_UP.contains("already setting up"));
}
