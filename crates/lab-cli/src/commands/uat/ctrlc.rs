//! Ctrl+C during a batch (and during `lab golden record`, which drives one).
//!
//! The first press sets [`STOP`]: no new run starts, the run under way finishes.
//! The second sets [`ABORT`]: calls in flight are given up on (see [`abortable`]),
//! the summary is written from the records already on disk and the process exits
//! 130. A third press is left to the system's default, which ends the process
//! at once. On other systems than Windows nothing is installed: the default
//! Ctrl+C ends the process.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::Duration;

/// Set by the first Ctrl+C press.
pub static STOP: AtomicBool = AtomicBool::new(false);
/// Set by the second Ctrl+C press.
pub static ABORT: AtomicBool = AtomicBool::new(false);
static PRESSES: AtomicU32 = AtomicU32::new(0);
static INSTALLED: AtomicBool = AtomicBool::new(false);
static QUIET: AtomicBool = AtomicBool::new(false);

/// The exit code of a batch a second Ctrl+C ended (128 + SIGINT).
pub const EXIT_INTERRUPTED: u8 = 130;

/// Whether Ctrl+C was pressed.
pub fn stop_requested() -> bool {
    STOP.load(Ordering::SeqCst)
}

/// Whether Ctrl+C was pressed twice.
pub fn abort_requested() -> bool {
    ABORT.load(Ordering::SeqCst)
}

/// What one press does: the first sets [`STOP`] and the second [`ABORT`], each
/// returning `true` (handled); every later one returns `false` (let the default
/// handler end the process).
pub fn press() -> bool {
    match PRESSES.fetch_add(1, Ordering::SeqCst) {
        0 => {
            STOP.store(true, Ordering::SeqCst);
            true
        }
        1 => {
            // The second press implies the first, whichever order the flags are read in.
            STOP.store(true, Ordering::SeqCst);
            ABORT.store(true, Ordering::SeqCst);
            true
        }
        _ => false,
    }
}

/// Runs `work` on its own thread and waits for it, giving up when `abort` is
/// set: `None` then (the thread is left to finish on its own; it is killed with
/// the process). A `work` that panics panics the caller, so a lane's panic guard
/// sees it as it would a call made in place.
pub fn abortable<T: Send + 'static>(
    abort: &AtomicBool,
    work: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(work());
    });
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(value) => return Some(value),
            Err(RecvTimeoutError::Timeout) if abort.load(Ordering::SeqCst) => return None,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => panic!("the call thread panicked"),
        }
    }
}

#[cfg(windows)]
mod win {
    use std::io::Write;
    use std::sync::Once;

    use std::sync::atomic::Ordering;

    use super::{press, PRESSES, QUIET};

    static INSTALLED: Once = Once::new();

    type HandlerRoutine = unsafe extern "system" fn(u32) -> i32;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetConsoleCtrlHandler(handler: Option<HandlerRoutine>, add: i32) -> i32;
    }

    const CTRL_C_EVENT: u32 = 0;
    const CTRL_BREAK_EVENT: u32 = 1;

    unsafe extern "system" fn on_event(kind: u32) -> i32 {
        if (kind == CTRL_C_EVENT || kind == CTRL_BREAK_EVENT) && press() {
            // `eprintln!` panics when stderr is closed; a panic here would abort.
            let text = if PRESSES.load(Ordering::SeqCst) <= 1 {
                "Ctrl+C: no new run will start; the run under way finishes first (Ctrl+C again stops now and still writes the summary)"
            } else {
                "Ctrl+C: stopping now; writing the summary from the runs that finished (Ctrl+C once more ends the process)"
            };
            if !QUIET.load(Ordering::SeqCst) {
                let _ = writeln!(std::io::stderr(), "{text}");
            }
            1
        } else {
            0
        }
    }

    /// Registers the handler once per process (a second call adds nothing).
    pub fn install() {
        INSTALLED.call_once(|| {
            // SAFETY: `on_event` has the `PHANDLER_ROUTINE` signature and lives for
            // the whole process; the call only registers it.
            unsafe {
                SetConsoleCtrlHandler(Some(on_event), 1);
            }
        });
    }

    /// Takes the handler off again (the default Ctrl+C ends the process).
    pub fn uninstall() {
        // SAFETY: removes the routine `install` registered; removing one that is
        // not registered only returns 0.
        unsafe {
            SetConsoleCtrlHandler(Some(on_event), 0);
        }
    }
}

/// Installs the handler (Windows); nothing elsewhere. Once per process.
pub fn install() {
    INSTALLED.store(true, Ordering::SeqCst);
    #[cfg(windows)]
    win::install();
}

/// [`install`] for a process that only waits for a child `lab uat` on the same
/// console (`lab golden record`): the child says what Ctrl+C does, so this
/// process prints nothing of its own.
pub fn install_silent() {
    QUIET.store(true, Ordering::SeqCst);
    install();
}

/// Takes the handler off (Windows) so the default Ctrl+C ends the process
/// again: `lab golden record` does this once its child has ended.
pub fn uninstall() {
    #[cfg(windows)]
    win::uninstall();
}

/// Whether a handler was installed in this process.
pub fn installed() -> bool {
    INSTALLED.load(Ordering::SeqCst)
}
