//! The DLL's local log: one line per boot step, to two places.
//!
//! - `OutputDebugStringW`, always, so a debugger or Sysinternals DebugView
//!   sees it with no setup.
//! - `cimmeria-client-telemetry.log` next to `SGW.exe`, rewritten at each
//!   launch, when the directory is writable.
//!
//! Everything the DLL reports goes to SigNoz through the uploader, but
//! that needs a session file, a token and a reachable server. This log
//! needs none of them, so "the DLL loaded but nothing arrived" can be
//! told apart from "the DLL never loaded" on the machine itself.
//!
//! Lines are prefixed `[cimmeria-client-telemetry +<ms>ms]`, the same
//! shape as the client-patches DLL's log. After [`MAX_LINES`] lines the
//! log stops, so a noisy hook cannot grow it without bound.

use std::fmt::Display;
use std::io::Write;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Instant;

/// Lines written before the log goes quiet.
pub const MAX_LINES: u32 = 2_000;

/// File name, created next to `SGW.exe`.
pub const FILE_NAME: &str = "cimmeria-client-telemetry.log";

const PREFIX: &str = "cimmeria-client-telemetry";

static STARTED: OnceLock<Instant> = OnceLock::new();
static LINES: AtomicU32 = AtomicU32::new(0);
static FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// Start the log. `dir` is where the file goes; `None`, or a directory
/// that cannot be written, leaves only the debugger output.
pub fn init(dir: Option<&std::path::Path>) {
    let _ = STARTED.set(Instant::now());
    let file = dir.and_then(|d| std::fs::File::create(d.join(FILE_NAME)).ok());
    *FILE.lock().unwrap_or_else(PoisonError::into_inner) = file;
}

/// Write one line.
pub fn line(message: impl Display) {
    if LINES.fetch_add(1, Ordering::Relaxed) >= MAX_LINES {
        return;
    }
    let elapsed_ms = STARTED.get().map_or(0, |s| s.elapsed().as_millis());
    let text = format_line(elapsed_ms, &message.to_string());

    #[cfg(windows)]
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

/// `[cimmeria-client-telemetry +<ms>ms] <message>` and a CRLF. Control
/// characters in the message are replaced, so a hostile string (a module
/// name, a server-supplied value) cannot forge extra lines.
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
    fn line_has_prefix_elapsed_and_crlf() {
        assert_eq!(
            format_line(1234, "hooks installed"),
            "[cimmeria-client-telemetry +1234ms] hooks installed\r\n"
        );
    }

    #[test]
    fn control_characters_cannot_forge_lines() {
        assert_eq!(
            format_line(0, "a\r\n[cimmeria-client-telemetry +0ms] fake"),
            "[cimmeria-client-telemetry +0ms] a??[cimmeria-client-telemetry +0ms] fake\r\n"
        );
    }
}
