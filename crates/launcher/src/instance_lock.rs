//! The process's single-instance lock (`launcher.lock` beside the exe).
//!
//! `main` takes the lock before building any state and parks the handle
//! here, so the self-update handoff can let go of it from a worker thread
//! without waiting for the window to close: an egui frame only runs on
//! input or a repaint request, and a launcher that waited for one kept the
//! lock long after the new launcher had started (2026-09-29, updating from
//! `launcher-20260929-676f314`). See [`crate::self_update::handoff`].

use std::fs::File;
use std::sync::Mutex;

use fs4::FileExt;

/// The held lock file, or `None` once released (or never taken).
static HELD: Mutex<Option<File>> = Mutex::new(None);

/// Keep `file`, whose exclusive lock this process already holds, until
/// [`release`] or process exit.
pub fn hold(file: File) {
    let mut held = HELD.lock().unwrap_or_else(|p| p.into_inner());
    *held = Some(file);
}

/// Unlock and close the lock file. Returns false when no lock was held.
///
/// The explicit unlock matters: Windows releases a closed handle's locks
/// "when resources allow", not necessarily at once, and the new launcher
/// is polling for this lock right now.
pub fn release() -> bool {
    let file = HELD.lock().unwrap_or_else(|p| p.into_inner()).take();
    match file {
        Some(file) => {
            // Fully qualified for fs4's trait method, as in main.rs.
            let _ = FileExt::unlock(&file);
            drop(file);
            true
        }
        None => false,
    }
}
