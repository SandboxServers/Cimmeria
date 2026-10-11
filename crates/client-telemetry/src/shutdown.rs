//! The final flush at process exit (TG-PIPE-03).
//!
//! Until this module the uploader ran `|| false` for the life of the process,
//! so whatever was still queued when SGW.exe quit (the governor's last window,
//! the closing health row) never left the machine. Two exit hooks in
//! [`crate::hooks`] call [`flush_at_exit`]: it raises a stop flag the uploader
//! polls, then waits a bounded time for the uploader to post its last batch.
//!
//! # Where the client exits (SGW.exe QA build, headless Ghidra, 2026-10-11)
//!
//! - Quit request: `UGameEngine__unknown_004910b0(force)` @ `0x004910b0`
//!   calls `ExitProcess(1)` when `force != 0`; otherwise it posts `WM_QUIT`
//!   (`PostQuitMessage(0)`) and sets `DAT_01ead7ec`. The twin at `0x004cc107`
//!   (inside an unnamed function) tests the same flag: a second quit request
//!   goes to `ExitProcess(0)`. Both call through the `ExitProcess` IAT slot
//!   `0x017ef238`.
//! - Normal return: `WM_QUIT` ends the message loop, `FUN_004161d0` (WinMain)
//!   returns, and `___tmainCRTStartup` @ `0x012376e3` calls `exit(code)`
//!   @ `0x01237832` through the MSVCR80 import slot `0x017ef9a8`. Hooking
//!   `ExitProcess` alone misses this path, because `exit` reaches
//!   `ExitProcess` from inside MSVCR80, not through SGW.exe's slot.
//! - "Failed to load Login_Map. Exiting." also ends in `exit(0)` (`0x00417fe0`).
//! - `TerminateProcess` has one SGW.exe caller, `___report_gsfailure`; a crash
//!   path is not a clean exit and is left to the exception sink.
//!
//! # Why the flush is bounded
//!
//! Both hooks run on the game's own thread, before `ExitProcess` takes the
//! loader lock, so waiting there cannot deadlock the loader. `DllMain`
//! `DLL_PROCESS_DETACH` does *not* get this work: it runs under the loader
//! lock after every other thread is gone, so the uploader thread cannot run
//! and a wait would only stall exit. The wait is capped at
//! [`FINAL_FLUSH_BUDGET`]; a stuck network never holds the process open longer.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// The longest an exit hook waits for the uploader's last POST. The
/// uploader's own final POST has a shorter limit
/// ([`crate::uploader::FINAL_POST_TIMEOUT`]) so it normally finishes inside
/// this.
pub const FINAL_FLUSH_BUDGET: Duration = Duration::from_millis(1500);

/// Which exit entry the client took. TG-CLI-04 turns this into the
/// `reason` of `client.session.end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitReason {
    /// SGW.exe called `ExitProcess` itself (forced or repeated quit).
    ExitProcess,
    /// The CRT's `exit`, reached after the message loop ended.
    CrtExit,
}

impl ExitReason {
    /// The value for the `reason` field.
    pub const fn as_str(self) -> &'static str {
        match self {
            ExitReason::ExitProcess => "exit_process",
            ExitReason::CrtExit => "crt_exit",
        }
    }

    const fn code(self) -> u8 {
        match self {
            ExitReason::ExitProcess => 1,
            ExitReason::CrtExit => 2,
        }
    }

    const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(ExitReason::ExitProcess),
            2 => Some(ExitReason::CrtExit),
            _ => None,
        }
    }
}

/// How a [`flush_at_exit`] call ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlushOutcome {
    /// The uploader posted its last batch (or had nothing to post) in time.
    Flushed,
    /// The budget ran out first; the process exits without the last batch.
    TimedOut,
    /// No uploader is running (no session, or the spawn failed).
    NotRunning,
}

/// The stop flag, the uploader's "I'm done" signal and the exit reason.
/// One process-wide instance ([`global`]); tests make their own.
#[derive(Debug, Default)]
pub struct Shutdown {
    armed: AtomicBool,
    stop: AtomicBool,
    reason: AtomicU8,
    flushed: Mutex<bool>,
    flushed_cv: Condvar,
}

impl Shutdown {
    pub const fn new() -> Self {
        Self {
            armed: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            reason: AtomicU8::new(0),
            flushed: Mutex::new(false),
            flushed_cv: Condvar::new(),
        }
    }

    /// An uploader is about to run. Without this, [`Self::flush`] returns
    /// at once instead of waiting for a thread that does not exist.
    pub fn arm(&self) {
        self.armed.store(true, Ordering::SeqCst);
    }

    /// The uploader could not start; exit hooks have nothing to wait for.
    pub fn disarm(&self) {
        self.armed.store(false, Ordering::SeqCst);
    }

    /// The uploader polls this as its `should_stop` callback.
    pub fn stop_requested(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    /// The first exit reason recorded, if an exit has been requested.
    pub fn reason(&self) -> Option<ExitReason> {
        ExitReason::from_code(self.reason.load(Ordering::SeqCst))
    }

    /// The uploader has posted its last batch (or is gone). Wakes waiters.
    pub fn mark_flushed(&self) {
        *self.flushed.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.flushed_cv.notify_all();
    }

    /// Ask the uploader to stop and wait up to `budget` for it to finish.
    /// Safe to call more than once (`exit` then `ExitProcess`): the first
    /// reason wins and later calls return as soon as the flush is done.
    ///
    /// TG-CLI-04 seam: emit `client.session.end {reason}` here, before the
    /// stop flag is raised, so it is in the final batch.
    pub fn flush(&self, reason: ExitReason, budget: Duration) -> FlushOutcome {
        if !self.armed.load(Ordering::SeqCst) {
            return FlushOutcome::NotRunning;
        }
        let _ = self
            .reason
            .compare_exchange(0, reason.code(), Ordering::SeqCst, Ordering::SeqCst);
        self.stop.store(true, Ordering::SeqCst);

        let deadline = Instant::now() + budget;
        let mut flushed = self.flushed.lock().unwrap_or_else(PoisonError::into_inner);
        while !*flushed {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                return FlushOutcome::TimedOut;
            };
            let (guard, timeout) = self
                .flushed_cv
                .wait_timeout(flushed, left)
                .unwrap_or_else(PoisonError::into_inner);
            flushed = guard;
            if timeout.timed_out() && !*flushed {
                return FlushOutcome::TimedOut;
            }
        }
        FlushOutcome::Flushed
    }
}

static GLOBAL: Shutdown = Shutdown::new();

/// The process-wide instance the boot code and the exit hooks share.
pub fn global() -> &'static Shutdown {
    &GLOBAL
}

/// What the exit hooks call: flush within [`FINAL_FLUSH_BUDGET`], record
/// the outcome in the local log, and return. Never panics.
pub fn flush_at_exit(reason: ExitReason) -> FlushOutcome {
    let outcome = global().flush(reason, FINAL_FLUSH_BUDGET);
    crate::log::line(format_args!(
        "exit via {}: final flush {:?}",
        reason.as_str(),
        outcome
    ));
    outcome
}

/// Marks the global shutdown flushed when dropped, so a panicking uploader
/// thread still releases the exit hooks.
pub struct FlushedOnDrop;

impl Drop for FlushedOnDrop {
    fn drop(&mut self) {
        global().mark_flushed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn an_unarmed_shutdown_does_not_wait() {
        let s = Shutdown::new();
        let started = Instant::now();
        assert_eq!(
            s.flush(ExitReason::CrtExit, Duration::from_secs(5)),
            FlushOutcome::NotRunning
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(!s.stop_requested(), "no uploader: nothing to stop");
    }

    /// The hook raises the stop flag and returns once the uploader reports
    /// done. Without the stop flag the fake uploader never finishes and the
    /// call times out.
    #[test]
    fn flush_raises_the_stop_flag_and_waits_for_the_uploader() {
        let s = Arc::new(Shutdown::new());
        s.arm();
        let uploader = {
            let s = s.clone();
            thread::spawn(move || {
                while !s.stop_requested() {
                    thread::sleep(Duration::from_millis(5));
                }
                s.mark_flushed();
            })
        };
        assert_eq!(
            s.flush(ExitReason::ExitProcess, Duration::from_secs(5)),
            FlushOutcome::Flushed
        );
        uploader.join().unwrap();
        assert_eq!(s.reason(), Some(ExitReason::ExitProcess));
    }

    /// A stuck uploader holds the exit for the budget and no longer.
    #[test]
    fn flush_gives_up_when_the_budget_runs_out() {
        let s = Shutdown::new();
        s.arm();
        let started = Instant::now();
        let outcome = s.flush(ExitReason::CrtExit, Duration::from_millis(80));
        assert_eq!(outcome, FlushOutcome::TimedOut);
        assert!(started.elapsed() >= Duration::from_millis(80));
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    /// `exit` runs first, then `ExitProcess`: the first reason is kept and
    /// the second call does not wait again.
    #[test]
    fn the_first_exit_reason_wins_and_a_second_call_returns_at_once() {
        let s = Shutdown::new();
        s.arm();
        s.mark_flushed();
        assert_eq!(
            s.flush(ExitReason::CrtExit, Duration::from_secs(5)),
            FlushOutcome::Flushed
        );
        let started = Instant::now();
        assert_eq!(
            s.flush(ExitReason::ExitProcess, Duration::from_secs(5)),
            FlushOutcome::Flushed
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(s.reason(), Some(ExitReason::CrtExit));
    }

    #[test]
    fn the_reason_values_are_pinned() {
        assert_eq!(ExitReason::ExitProcess.as_str(), "exit_process");
        assert_eq!(ExitReason::CrtExit.as_str(), "crt_exit");
    }
}
