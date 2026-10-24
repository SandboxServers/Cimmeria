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
    let next = install_with(
        || Some(HTML_PAGE.to_vec()),
        |_| {
            ran.set(true);
            InstallOutcome::Exited
        },
        || true,
    );
    assert!(!ran.get(), "the HTML page reached the installer step");
    assert_eq!(next, Next::FallBack);
}

#[test]
fn a_failed_download_falls_back_without_running_anything() {
    let ran = Cell::new(false);
    let next = install_with(
        || None,
        |_| {
            ran.set(true);
            InstallOutcome::Exited
        },
        || true,
    );
    assert!(!ran.get());
    assert_eq!(next, Next::FallBack);
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
        Next::Continue
    );
    assert_eq!(runs.get(), 1);
    assert_eq!(
        install_with(|| Some(EXECUTABLE.to_vec()), run, || false),
        Next::FallBack
    );
    assert_eq!(runs.get(), 2);
}

#[test]
fn the_bootstrapper_gets_silent_first() {
    assert_eq!(BOOTSTRAPPER_ARGS, ["/silent", "/install"]);
}
