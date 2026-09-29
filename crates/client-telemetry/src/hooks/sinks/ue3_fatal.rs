//! The UE3 fatal-error sink: the message of `appErrorf`, `GError->Logf`
//! and a failed `appFailAssertFunc`, captured before the client shows its
//! error box and exits.
//!
//! # Where it hooks
//!
//! `FOutputDeviceWindowsError::Serialize(const TCHAR* Msg, EName Event)` at
//! `0x004ce3a0`, vtable slot 1 of that device (`vtable_FOutputDeviceWindowsError`
//! at `0x018150f8`). `GError` is this device; it is what UE3 calls for an
//! error it cannot continue from. The function (decompiled 2026-09-28):
//!
//! 1. calls `IsDebuggerPresent`, and if one is attached writes to a low
//!    address to force a fault (a deliberate break);
//! 2. on the first error only (`0x01ead7c4` is the "critical error" flag)
//!    copies the message into the 16 K-unit wide buffer at `0x01ea57a0`,
//!    appends `"\r\n\r\n"`, and reports it to the assertion manager;
//! 3. if a second flag (`0x01ead7cc`) is set, throws a C++ exception
//!    (`0x01c208e8`);
//! 4. calls virtual slot 4 of the device (`HandleError`, which shows the box
//!    and writes the crash log) and then `UGameEngine::unknown_004910b0(1)`,
//!    which ends the process.
//!
//! So the hook is the last chance to record why the client died: the event
//! is queued, and the message is also written to the local
//! `cimmeria-client-telemetry.log` synchronously, because the process may be
//! gone before the uploader thread's next flush.
//!
//! Static evidence only (2026-09-28, Ghidra); not yet seen from a live
//! client.

use serde_json::json;

use super::emit::Fields;

/// Entry of `FOutputDeviceWindowsError::Serialize`.
pub const ADDR_ERROR_SERIALIZE: usize = 0x004c_e3a0;

/// The fatal message is a full formatted UE3 buffer.
pub const MAX_LINE_UNITS: usize = 2048;

/// Telemetry target.
pub const TARGET: &str = "client.ue3.fatal_error";

/// The fields of one fatal error.
pub fn error_fields(category: &str, event: u32, message: &str, truncated: bool) -> Fields {
    let mut f: Fields = vec![
        ("category", json!(category)),
        ("event", json!(event)),
        ("message", json!(message)),
        ("fatal", json!(true)),
    ];
    if truncated {
        f.push(("truncated", json!(true)));
    }
    f
}

/// The line written to the local log: one line, so a multi-line message
/// (an assert's expression and stack) does not break the log's framing.
pub fn log_line(category: &str, message: &str) -> String {
    format!(
        "UE3 fatal error [{category}]: {}",
        message.replace(['\r', '\n'], " | ")
    )
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod x86 {
    use std::ffi::c_void;
    use std::panic::AssertUnwindSafe;
    use std::sync::OnceLock;

    use super::super::gnames;
    use super::super::mem;
    use super::super::text;
    use super::*;

    pub(in super::super) static TRAMPOLINE: OnceLock<usize> = OnceLock::new();

    /// Report the error. Never rate-limited: it is the last event of a
    /// session, and there is at most one (a second error while handling the
    /// first is the same death).
    fn observe(msg: *const u16, event: u32) {
        let category =
            gnames::resolve_here(event as i32).map_or_else(|| format!("name#{event}"), |n| n.name);
        let (message, truncated) = match mem::wide_at(msg as usize, MAX_LINE_UNITS) {
            Some(line) => {
                let (m, cut) = text::message_field(&text::decode_wide(&line.units));
                (m, cut || line.truncated)
            }
            None => ("<unreadable>".to_string(), false),
        };
        crate::log::line(log_line(&category, &message));
        super::super::emit::emit(
            TARGET,
            "error",
            "ue3.fatal",
            error_fields(&category, event, &message, truncated),
        );
    }

    /// Detour for `FOutputDeviceWindowsError::Serialize(Msg, Event)`.
    ///
    /// **Threads:** whichever thread raised the error. The original does not
    /// return normally (it ends the process, or throws), so an unwind is
    /// expected and passes straight through.
    #[allow(improper_ctypes_definitions)]
    pub(in super::super) unsafe extern "thiscall-unwind" fn detour(
        this: *mut c_void,
        msg: *const u16,
        event: u32,
    ) {
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| observe(msg, event)));

        let Some(t) = TRAMPOLINE.get() else {
            return;
        };
        let original: unsafe extern "thiscall-unwind" fn(*mut c_void, *const u16, u32) =
            unsafe { std::mem::transmute(*t) };
        original(this, msg, event);
    }

    #[cfg(test)]
    mod tests {
        use super::super::super::emit::take_captured;
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static SEEN: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];

        unsafe extern "thiscall-unwind" fn fake_original(
            this: *mut c_void,
            msg: *const u16,
            event: u32,
        ) {
            SEEN[0].store(this as usize, Ordering::SeqCst);
            SEEN[1].store(msg as usize, Ordering::SeqCst);
            SEEN[2].store(event as usize, Ordering::SeqCst);
        }

        #[test]
        fn the_detour_records_the_message_then_forwards() {
            TRAMPOLINE
                .set(fake_original as *const () as usize)
                .expect("only this test sets the trampoline");
            let msg: Vec<u16> = "Assertion failed: Foo != NULL\r\nStack: x"
                .encode_utf16()
                .chain(Some(0))
                .collect();
            let this = [0usize; 1];
            unsafe { detour(this.as_ptr() as *mut c_void, msg.as_ptr(), 5) };

            assert_eq!(SEEN[0].load(Ordering::SeqCst), this.as_ptr() as usize);
            assert_eq!(SEEN[1].load(Ordering::SeqCst), msg.as_ptr() as usize);
            assert_eq!(SEEN[2].load(Ordering::SeqCst), 5);

            let events = take_captured();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].target, TARGET);
            assert_eq!(events[0].level, "error");
            assert_eq!(events[0].get("fatal"), Some(&json!(true)));
            assert_eq!(
                events[0].get("message"),
                Some(&json!("Assertion failed: Foo != NULL\r\nStack: x"))
            );
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) use x86::{detour, TRAMPOLINE};

/// Install the sink.
///
/// # Safety
///
/// MinHook is initialised and the address is fingerprinted.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install(producer: &crate::queue::Producer) {
    super::install::inline(
        producer,
        "ue3_fatal",
        ADDR_ERROR_SERIALIZE,
        detour as *mut std::ffi::c_void,
        &TRAMPOLINE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_local_log_line_is_one_line() {
        assert_eq!(
            log_line("Error", "Assertion failed: x\r\nStack: a\nb"),
            "UE3 fatal error [Error]: Assertion failed: x |  | Stack: a | b"
        );
    }

    #[test]
    fn fields_mark_the_event_fatal() {
        let f = error_fields("Error", 5, "boom", false);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("fatal"), Some(json!(true)));
        assert_eq!(get("message"), Some(json!("boom")));
        assert_eq!(get("truncated"), None);
        let f = error_fields("Error", 5, "boom", true);
        assert!(f.contains(&("truncated", json!(true))));
    }

    #[test]
    fn the_hooked_address_is_the_ghidra_one() {
        assert_eq!(ADDR_ERROR_SERIALIZE, 0x004c_e3a0);
    }
}
