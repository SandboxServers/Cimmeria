use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use cimmeria_lab_cli::commands::uat::ctrlc::{abort_requested, abortable, press, stop_requested};

#[test]
fn the_first_press_stops_new_runs_the_second_aborts_a_third_is_left_to_the_default() {
    assert!(!stop_requested() && !abort_requested());
    assert!(press(), "the first press is handled");
    assert!(stop_requested());
    assert!(!abort_requested(), "one press does not abort");
    assert!(press(), "the second press is handled too");
    assert!(abort_requested());
    assert!(!press(), "the third press is not handled: the process ends");
}

#[test]
fn abortable_returns_the_value_of_work_that_finishes() {
    let abort = AtomicBool::new(false);
    assert_eq!(abortable(&abort, || 7), Some(7));
}

#[test]
fn abortable_gives_up_on_work_that_outlasts_the_abort() {
    let abort = AtomicBool::new(false);
    let started = Instant::now();
    let got = std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(200));
            abort.store(true, Ordering::SeqCst);
        });
        abortable(&abort, || {
            std::thread::sleep(Duration::from_secs(30));
            1
        })
    });
    assert_eq!(got, None);
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
#[should_panic(expected = "call thread panicked")]
fn abortable_passes_a_panic_of_the_work_on() {
    let abort = AtomicBool::new(false);
    let _ = abortable(&abort, || -> u32 { panic!("boom") });
}
