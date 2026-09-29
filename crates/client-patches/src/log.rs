//! The DLL's log: one line per event, to two places.
//!
//! - `OutputDebugStringW`, always, so a debugger or Sysinternals DebugView
//!   sees it with no setup.
//! - `cimmeria-client-patches.log` next to `SGW.exe`, rewritten at each
//!   launch, when the directory is writable (the launcher already writes
//!   `sessions/` there).
//!
//! Lines are prefixed `[cimmeria-client-patches +<ms>]`. After
//! [`MAX_LINES`] lines the log stops, so a bad server cannot grow it
//! without bound; repeated per-call events are further thinned by
//! [`crate::counters::is_log_worthy`].

use std::fmt::Display;
use std::io::Write;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Instant;

/// Lines written before the log goes quiet.
pub const MAX_LINES: u32 = 2_000;

/// File name, created next to `SGW.exe`.
pub const FILE_NAME: &str = "cimmeria-client-patches.log";

const PREFIX: &str = "cimmeria-client-patches";

static STARTED: OnceLock<Instant> = OnceLock::new();
static LINES: AtomicU32 = AtomicU32::new(0);
static FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// Start the log. `dir` is where the file goes; `None`, or a directory
/// that cannot be written, leaves only the debugger output.
pub fn init(dir: Option<&std::path::Path>) {
    init_as(dir, None);
}

/// [`init`] for a named lab instance: two clients on one machine would
/// otherwise both `File::create` the same log and overwrite each other's
/// lines, so an instance writes `cimmeria-client-patches-<instance>.log` instead. `None` is the
/// default file name.
pub fn init_as(dir: Option<&std::path::Path>, instance: Option<&str>) {
    let _ = STARTED.set(Instant::now());
    let file = dir.and_then(|d| std::fs::File::create(d.join(file_name(instance))).ok());
    *FILE.lock().unwrap_or_else(PoisonError::into_inner) = file;
}

/// The log's file name: [`FILE_NAME`], or with `-<instance>` before the
/// extension for a named lab instance.
pub fn file_name(instance: Option<&str>) -> String {
    match instance {
        Some(i) => format!("{}-{i}.log", FILE_NAME.trim_end_matches(".log")),
        None => FILE_NAME.to_string(),
    }
}

/// Write one line.
pub fn line(message: impl Display) {
    if LINES.fetch_add(1, Ordering::Relaxed) >= MAX_LINES {
        return;
    }
    let elapsed_ms = STARTED.get().map_or(0, |s| s.elapsed().as_millis());
    let text = format_line(elapsed_ms, &message.to_string());

    #[cfg(all(windows, target_arch = "x86"))]
    {
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        wide.push(0);
        // SAFETY: a NUL-terminated UTF-16 buffer that outlives the call.
        unsafe {
            windows_sys::Win32::System::Diagnostics::Debug::OutputDebugStringW(wide.as_ptr());
        }
    }

    let mut file = FILE.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(f) = file.as_mut() {
        // Best effort: a full disk must not take the game down.
        let _ = f.write_all(text.as_bytes());
        let _ = f.flush();
    }
}

/// `[cimmeria-client-patches +<ms>] <message>` and a CRLF. Control
/// characters in the message are replaced, so a hostile string from the
/// server cannot forge extra lines.
pub fn format_line(elapsed_ms: u128, message: &str) -> String {
    let clean: String = message
        .chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .collect();
    format!("[{PREFIX} +{elapsed_ms}ms] {clean}\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_log_gets_its_own_file_name() {
        assert_eq!(file_name(None), FILE_NAME);
        assert_eq!(
            file_name(Some("p2")),
            format!("{}-p2.log", FILE_NAME.trim_end_matches(".log"))
        );
    }

    #[test]
    fn line_has_prefix_elapsed_and_crlf() {
        assert_eq!(
            format_line(1234, "hooks installed"),
            "[cimmeria-client-patches +1234ms] hooks installed\r\n"
        );
    }

    #[test]
    fn control_characters_cannot_forge_lines() {
        assert_eq!(
            format_line(0, "a\r\n[cimmeria-client-patches +0ms] fake"),
            "[cimmeria-client-patches +0ms] a??[cimmeria-client-patches +0ms] fake\r\n"
        );
    }
}
