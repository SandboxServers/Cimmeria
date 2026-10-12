//! Ctrl+C during a batch. The first press sets [`STOP`] (no new run starts) and is
//! handled; the second press is left to the system's default, which ends the
//! process. On other systems than Windows nothing is installed: the default
//! Ctrl+C ends the process.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Set by the first Ctrl+C press.
pub static STOP: AtomicBool = AtomicBool::new(false);
static PRESSES: AtomicU32 = AtomicU32::new(0);

/// Whether Ctrl+C was pressed.
pub fn stop_requested() -> bool {
    STOP.load(Ordering::SeqCst)
}

/// What one press does: the first sets [`STOP`] and returns `true` (handled);
/// every later one returns `false` (let the default handler end the process).
pub fn press() -> bool {
    if PRESSES.fetch_add(1, Ordering::SeqCst) == 0 {
        STOP.store(true, Ordering::SeqCst);
        true
    } else {
        false
    }
}

#[cfg(windows)]
mod win {
    use std::io::Write;
    use std::sync::Once;

    use super::press;

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
            let _ = writeln!(
                std::io::stderr(),
                "Ctrl+C: no new run will start; the runs under way finish first (Ctrl+C again stops now)"
            );
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
}

/// Installs the handler (Windows); nothing elsewhere.
pub fn install() {
    #[cfg(windows)]
    win::install();
}
