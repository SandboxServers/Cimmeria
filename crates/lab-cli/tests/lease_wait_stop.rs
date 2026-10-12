//! LX-18c1 (F-LX10): Ctrl+C during the lease wait ends it at once. The core runs
//! on a fake clock; one test runs the real wait against a stub `/status`.

mod support;

use std::cell::Cell;
use std::time::{Duration, Instant};

use cimmeria_lab_cli::daemon::mcp::{
    wait_lease_free_stoppable_every, wait_lease_free_until, LeaseClock, STOP_SLICE,
};
use serde_json::{json, Value};
use support::StubServer;

/// A clock that moves time without sleeping and calls `on_sleep(n)` after the
/// n-th sleep (1-based).
struct FakeClock<'a> {
    now: Duration,
    sleeps: Vec<Duration>,
    on_sleep: Box<dyn FnMut(usize) + 'a>,
}

impl LeaseClock for FakeClock<'_> {
    fn elapsed(&mut self) -> Duration {
        self.now
    }

    fn sleep(&mut self, d: Duration) {
        self.now += d;
        self.sleeps.push(d);
        (self.on_sleep)(self.sleeps.len());
    }
}

fn clock<'a>(on_sleep: impl FnMut(usize) + 'a) -> FakeClock<'a> {
    FakeClock {
        now: Duration::ZERO,
        sleeps: Vec::new(),
        on_sleep: Box::new(on_sleep),
    }
}

fn status(instance: &str, held: bool) -> Option<Value> {
    Some(json!({"instances": [{"instance": instance, "lease": {"held": held}}]}))
}

const TEN: Duration = Duration::from_secs(10);
const TEN_MINUTES: Duration = Duration::from_secs(600);

#[test]
fn ctrl_c_ends_the_wait_within_one_slice() {
    let stop = Cell::new(false);
    let polls = Cell::new(0u32);
    let mut poll = || {
        polls.set(polls.get() + 1);
        status("i1", true)
    };
    // Ctrl+C arrives during the fifth slice of the first 10 s sleep.
    let mut fake = clock(|n| {
        if n == 5 {
            stop.set(true);
        }
    });
    let cleared =
        wait_lease_free_until(&mut poll, "i1", TEN_MINUTES, TEN, &|| stop.get(), &mut fake);
    assert!(!cleared);
    assert_eq!(polls.get(), 1, "no second poll after Ctrl+C");
    assert_eq!(fake.sleeps.len(), 5, "no sleep after Ctrl+C");
    assert_eq!(fake.now, STOP_SLICE * 5, "one second, not the full 10 s");
}

#[test]
fn a_stop_set_before_the_wait_polls_nothing() {
    let polls = Cell::new(0u32);
    let mut poll = || {
        polls.set(polls.get() + 1);
        status("i1", true)
    };
    let mut fake = clock(|_| {});
    assert!(!wait_lease_free_until(
        &mut poll,
        "i1",
        TEN_MINUTES,
        TEN,
        &|| true,
        &mut fake
    ));
    assert_eq!(polls.get(), 0);
    assert!(fake.sleeps.is_empty());
}

#[test]
fn without_a_stop_the_wait_sleeps_in_slices_until_the_lease_clears() {
    let polls = Cell::new(0u32);
    let mut poll = || {
        polls.set(polls.get() + 1);
        status("i1", polls.get() < 4) // held for three polls, then free
    };
    let mut fake = clock(|_| {});
    assert!(wait_lease_free_until(
        &mut poll,
        "i1",
        TEN_MINUTES,
        TEN,
        &|| false,
        &mut fake
    ));
    assert_eq!(polls.get(), 4);
    assert_eq!(fake.sleeps.len(), 150, "three 10 s sleeps of 50 slices");
    assert!(fake.sleeps.iter().all(|s| *s <= STOP_SLICE));
    assert_eq!(fake.now, Duration::from_secs(30));
}

#[test]
fn the_budget_still_bounds_the_wait() {
    let polls = Cell::new(0u32);
    let mut poll = || {
        polls.set(polls.get() + 1);
        status("i1", true)
    };
    let mut fake = clock(|_| {});
    assert!(!wait_lease_free_until(
        &mut poll,
        "i1",
        Duration::from_secs(25),
        TEN,
        &|| false,
        &mut fake
    ));
    assert_eq!(
        fake.now,
        Duration::from_secs(25),
        "10 + 10 + 5, never past max"
    );
    assert_eq!(polls.get(), 4, "polls at 0, 10, 20 and 25 s");
}

#[test]
fn the_real_wait_stops_on_the_flag_without_waiting_out_its_budget() {
    let body = r#"{"daemon":{"pid":1,"version":"x"},"instances":[{"instance":"i1","lease":{"held":true}}]}"#;
    let (stub, ep) = StubServer::start(vec![("GET", "/status", 200, body)]);
    let started = Instant::now();
    let cleared = wait_lease_free_stoppable_every(
        &ep.base_url,
        ep.token.as_deref(),
        "i1",
        TEN_MINUTES,
        Duration::from_millis(100),
        &|| stub.requests().len() >= 2, // Ctrl+C after the second poll
    );
    assert!(!cleared);
    assert!(started.elapsed() < Duration::from_secs(10));
    assert_eq!(stub.requests().len(), 2, "it polled twice and stopped");
}

#[test]
fn the_batch_and_the_heal_wait_take_the_ctrl_c_flag() {
    // The wiring is two closures in `execute` and `heal_batch`; this scan keeps
    // them from going back to the wait that cannot be stopped.
    let uat = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("commands")
        .join("uat");
    for file in ["batch_run.rs", "heal.rs"] {
        let text = std::fs::read_to_string(uat.join(file)).unwrap();
        assert!(
            text.contains("wait_lease_free_stoppable"),
            "{file} must wait with the stop check"
        );
        assert!(
            !text.contains("mcp::wait_lease_free("),
            "{file} still waits with the plain wait"
        );
    }
}
