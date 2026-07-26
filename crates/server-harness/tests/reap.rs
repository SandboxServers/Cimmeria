//! Reaping guards for [`ChildGuard`].
//!
//! These deliberately use a trivial long-lived child rather than a real
//! `cimmeria-server`. The property under test is "the guard kills what it
//! owns, even when the test body panics" — that property is independent of
//! which program was spawned, and decoupling it means the guard stays
//! covered on a machine that has never built the server binary.
//!
//! Every test here probes the OS for liveness rather than trusting the
//! guard's own bookkeeping. Asserting on a "reaped" flag the guard sets
//! itself would pass even if `kill()` silently failed.
//!
//! # Why the `Drop`-layer tests pass `OrphanProtection::DropOnly`
//!
//! `ChildGuard` reaps through two independent mechanisms: `Drop` and a
//! kernel-level kill-on-parent-death job object. They are redundant *by
//! design*, and that redundancy defeats a naive test: with the job object
//! active, deleting the entire `Drop` impl still leaves every child correctly
//! reaped, because closing the job handle kills it anyway. Verified by
//! experiment, not assumed.
//!
//! So the layers are guarded separately. Tests that name `Drop` in their
//! assertion spawn with [`OrphanProtection::DropOnly`], which is the only way
//! a broken `Drop` becomes observable. One test then covers the default
//! (both-layers) configuration, and one covers the kernel layer alone.

use std::process::Command;
use std::time::{Duration, Instant};

use cimmeria_server_harness::{is_process_alive, ChildGuard, OrphanProtection};

/// A child that stays alive long enough to observe, without depending on a
/// shell (a `cmd /c` wrapper would be killed while its grandchild survived,
/// which would make these tests assert on the wrong process).
fn long_lived_command() -> Command {
    if cfg!(windows) {
        // ~120s of pinging localhost, as a single direct process.
        let mut c = Command::new("ping.exe");
        c.args(["-n", "120", "127.0.0.1"]);
        c
    } else {
        let mut c = Command::new("sleep");
        c.arg("120");
        c
    }
}

/// Poll until the process disappears, up to `limit`.
///
/// `kill()` requests termination; the OS reaps asynchronously, so a bare
/// assertion immediately after the drop would be flaky. This bounds the wait
/// on an observed condition rather than assuming a fixed settle time.
fn wait_until_dead(pid: u32, limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if !is_process_alive(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    !is_process_alive(pid)
}

/// The headline guarantee: a panicking body still reaps the child, via `Drop`
/// running during unwind.
///
/// Spawned `DropOnly` so this isolates the `Drop` layer — deleting the `Drop`
/// impl makes this fail with a surviving process. Under the default
/// `Kernel` protection it would keep passing regardless, which is what
/// `child_is_reaped_on_panic_with_default_protection` covers instead.
#[test]
fn drop_alone_reaps_the_child_when_the_test_body_panics() {
    let guard = ChildGuard::spawn_with(
        long_lived_command(),
        "panic-reap-drop-only",
        OrphanProtection::DropOnly,
    )
    .expect("spawn child");
    let pid = guard.pid();

    // Positive control: the probe must be able to see a live process, or the
    // post-panic assertion below would pass vacuously.
    assert!(
        is_process_alive(pid),
        "child should be alive before the panic; if not, the liveness probe \
         is broken and the reap assertion would be meaningless"
    );

    let result = std::panic::catch_unwind(move || {
        // Moved in, so it is dropped as the unwind passes through this frame.
        let _owned = guard;
        panic!("simulated test-body failure");
    });

    assert!(result.is_err(), "the body was supposed to panic");
    assert!(
        wait_until_dead(pid, Duration::from_secs(10)),
        "child pid {pid} survived a panicking test body — Drop did not reap it"
    );
}

/// The ordinary path, isolating the `Drop` layer.
#[test]
fn drop_alone_reaps_the_child_on_normal_scope_exit() {
    let guard = ChildGuard::spawn_with(
        long_lived_command(),
        "normal-reap-drop-only",
        OrphanProtection::DropOnly,
    )
    .expect("spawn child");
    let pid = guard.pid();
    assert!(is_process_alive(pid), "child should be alive before drop");

    drop(guard);

    assert!(
        wait_until_dead(pid, Duration::from_secs(10)),
        "child pid {pid} survived a normal drop"
    );
}

/// The configuration a real harness actually uses: both layers active, body
/// panics. This is the end-to-end promise callers rely on, independent of
/// which layer delivers it.
#[test]
fn child_is_reaped_on_panic_with_default_protection() {
    let guard = ChildGuard::spawn(long_lived_command(), "panic-reap-default").expect("spawn child");
    let pid = guard.pid();
    assert!(
        is_process_alive(pid),
        "child should be alive before the panic"
    );

    let result = std::panic::catch_unwind(move || {
        let _owned = guard;
        panic!("simulated test-body failure");
    });

    assert!(result.is_err(), "the body was supposed to panic");
    assert!(
        wait_until_dead(pid, Duration::from_secs(10)),
        "child pid {pid} survived a panicking test body under default protection"
    );
}

/// The kernel layer on its own. `Drop` reaps before the job handle closes in
/// the normal case, so this cannot observe the job object killing a live
/// child; what it does pin is that requesting `Kernel` protection never
/// *prevents* a reap — a job-object misconfiguration that wedged `kill()` or
/// left the child running would surface here.
#[test]
fn kernel_protection_does_not_interfere_with_reaping() {
    let guard = ChildGuard::spawn_with(
        long_lived_command(),
        "kernel-protected",
        OrphanProtection::Kernel,
    )
    .expect("spawn child");
    let pid = guard.pid();
    assert!(is_process_alive(pid), "child should be alive before drop");

    drop(guard);

    assert!(
        wait_until_dead(pid, Duration::from_secs(10)),
        "child pid {pid} survived drop under kernel orphan protection"
    );
}

/// `Drop` must not double-kill or hang after an explicit reap.
#[test]
fn explicit_reap_then_drop_is_safe_and_idempotent() {
    let mut guard =
        ChildGuard::spawn(long_lived_command(), "idempotent-reap").expect("spawn child");
    let pid = guard.pid();

    guard.kill_and_reap();
    assert!(
        wait_until_dead(pid, Duration::from_secs(10)),
        "explicit kill_and_reap should have killed pid {pid}"
    );

    // The implicit Drop below must be a no-op, not a second kill of a
    // recycled pid.
    guard.kill_and_reap();
    drop(guard);
}

/// Each guard must reap only its own child. A shared-state bug that reaped
/// the most recently spawned process would pass the single-child tests above.
///
/// `DropOnly` so the assertion is about `Drop`'s per-guard bookkeeping rather
/// than a shared job object.
#[test]
fn guards_reap_independently() {
    let first = ChildGuard::spawn_with(
        long_lived_command(),
        "independent-a",
        OrphanProtection::DropOnly,
    )
    .expect("spawn a");
    let second = ChildGuard::spawn_with(
        long_lived_command(),
        "independent-b",
        OrphanProtection::DropOnly,
    )
    .expect("spawn b");
    let (first_pid, second_pid) = (first.pid(), second.pid());
    assert_ne!(first_pid, second_pid, "children must be distinct processes");

    drop(first);

    assert!(
        wait_until_dead(first_pid, Duration::from_secs(10)),
        "the dropped guard's child ({first_pid}) should be dead"
    );
    assert!(
        is_process_alive(second_pid),
        "the surviving guard's child ({second_pid}) must NOT be reaped by its sibling"
    );

    drop(second);
    assert!(
        wait_until_dead(second_pid, Duration::from_secs(10)),
        "second child ({second_pid}) should be reaped by its own guard"
    );
}

/// A child that exits on its own must be reaped without error, and reported
/// as exited. This is what lets the readiness loop fail fast on a server that
/// died during startup instead of polling until timeout.
#[test]
fn self_exited_child_is_observed_and_reaped_cleanly() {
    let command = if cfg!(windows) {
        let mut c = Command::new("ping.exe");
        c.args(["-n", "1", "127.0.0.1"]);
        c
    } else {
        Command::new("true")
    };

    let mut guard = ChildGuard::spawn(command, "self-exit").expect("spawn child");
    let pid = guard.pid();

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut status = None;
    while Instant::now() < deadline {
        if let Some(s) = guard.try_exit_status().expect("try_wait must not error") {
            status = Some(s);
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(
        status.is_some(),
        "a short-lived child should be observed exiting via try_exit_status"
    );

    drop(guard);
    assert!(
        wait_until_dead(pid, Duration::from_secs(10)),
        "an already-exited child must still be fully reaped, not left a zombie"
    );
}
