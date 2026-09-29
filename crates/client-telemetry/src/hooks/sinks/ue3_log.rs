//! The UE3 `GLog` sink: every line sent to the engine's log, with its
//! category (`Log`, `Warning`, `ScriptWarning`, `DevNet`, ...).
//!
//! # Where it hooks
//!
//! `FOutputDeviceRedirector::Serialize(const TCHAR* V, EName Event)` at
//! `0x004ce0b0`, vtable slot 1 of the redirector (RTTI
//! `.?AVFOutputDeviceRedirector@@`, vtable `0x01815188`, type descriptor
//! `0x01dafeac`). `GLog` is that redirector. `debugf`, `warnf`, `Logf` and
//! UnrealScript's `Log()` all end here: it serializes to every registered
//! device (the file device that writes `Launch.log`, the debug device that
//! calls `OutputDebugStringW`, the console) from the game thread, and queues
//! a copy for the game thread when called from another. So it sees each line
//! exactly once, on the thread that produced it, before any device's own
//! category filter.
//!
//! `debugf` and `warnf` are compiled out of this build (the client's
//! `Launch.log` holds only "Log file open" and "Release logging
//! initialized."), so most of the engine's own chatter never reaches this
//! hook. What does reach it: lines a subsystem sends through `GLog`/`GWarn`
//! unconditionally, UnrealScript `Log()` and `warn()`, and the log open and
//! close markers. Expect a low volume; the hook costs nothing when idle.
//!
//! # Category and suppression
//!
//! `Event` is an `FName` index. The name resolves through `FName::Names`
//! ([`gnames`](super::gnames)). The log devices skip a line whose name entry
//! has flag `0x1000` (`RF_Suppress`): that is how a category is silenced
//! (`FOutputDeviceDebug::Serialize` at `0x004cc9f0` tests it, and skips
//! names `0x5a` and `0x314` outright). The sink reports the line either way,
//! with `suppressed_category` set, and with `unfilter` on clears the flag
//! first so the client's own devices write it too.
//!
//! Static evidence only (2026-09-28, Ghidra); not yet seen from a live
//! client.

use serde_json::json;

use super::emit::Fields;
use super::text;

/// Entry of `FOutputDeviceRedirector::Serialize`.
pub const ADDR_REDIRECTOR_SERIALIZE: usize = 0x004c_e0b0;

/// Longest line read, in UTF-16 units. A UE3 log line is at most a
/// 1024-unit formatted buffer.
pub const MAX_LINE_UNITS: usize = 1024;

/// Telemetry target.
pub const TARGET: &str = "client.ue3.log";

/// What a category name says about a line's severity.
pub fn telemetry_level(category: &str) -> &'static str {
    let c = category.to_ascii_lowercase();
    if matches!(c.as_str(), "error" | "critical" | "fatal" | "scripterror") {
        "error"
    } else if c.contains("warning") || c == "warn" {
        "warn"
    } else if c.starts_with("dev") {
        "debug"
    } else {
        "info"
    }
}

/// Rate-limit key. A warning or error is keyed on its text, so two
/// different ones never hide each other; chatty categories share a bucket
/// per category.
pub fn throttle_key(category: &str, message: &str) -> String {
    match telemetry_level(category) {
        "warn" | "error" => format!("{category}:{}", text::message_shape(message)),
        _ => category.to_string(),
    }
}

/// The fields of one `client.ue3.log`.
pub fn line_fields(
    category: &str,
    event: i32,
    message: &str,
    truncated: bool,
    suppressed_category: bool,
    dropped: u64,
) -> Fields {
    let mut f: Fields = vec![
        ("category", json!(category)),
        ("event", json!(event)),
        ("message", json!(message)),
    ];
    if suppressed_category {
        f.push(("suppressed_category", json!(true)));
    }
    if dropped > 0 {
        f.push(("suppressed", json!(dropped)));
    }
    if truncated {
        f.push(("truncated", json!(true)));
    }
    f
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod x86 {
    use std::ffi::c_void;
    use std::panic::AssertUnwindSafe;
    use std::sync::OnceLock;

    use super::super::gnames::{self, FLAG_SUPPRESS};
    use super::super::mem;
    use super::super::throttle::SinkThrottle;
    use super::*;
    use crate::hooks::name_throttle::Decision;

    pub(in super::super) static TRAMPOLINE: OnceLock<usize> = OnceLock::new();

    static THROTTLE: SinkThrottle = SinkThrottle::new();

    /// Report one line; with `unfilter`, clear the category's suppress flag.
    fn observe(v: *const u16, event: i32) {
        let name = gnames::resolve_here(event);
        if let Some(n) = name.as_ref().filter(|n| n.is_suppressed()) {
            if crate::capture::unfilter() {
                // SAFETY: `entry` was read through the process reader a
                // moment ago, so it is mapped; the flags word is a plain
                // aligned `u32` the log devices only read.
                unsafe {
                    std::ptr::write_volatile(
                        (n.entry + gnames::ENTRY_FLAGS_OFFSET) as *mut u32,
                        n.flags & !FLAG_SUPPRESS,
                    );
                }
            }
        }

        let Some(line) = mem::wide_at(v as usize, MAX_LINE_UNITS) else {
            return;
        };
        let raw = text::decode_wide(&line.units);
        let (message, cut) = text::message_field(&raw);
        let category = name
            .as_ref()
            .map_or_else(|| format!("name#{event}"), |n| n.name.clone());

        let Decision::Emit { suppressed } = THROTTLE.check(&throttle_key(&category, &message))
        else {
            return;
        };
        super::super::emit::emit(
            TARGET,
            telemetry_level(&category),
            "ue3.log",
            line_fields(
                &category,
                event,
                &message,
                cut || line.truncated,
                name.as_ref().is_some_and(|n| n.is_suppressed()),
                suppressed,
            ),
        );
    }

    /// Detour for `FOutputDeviceRedirector::Serialize(V, Event)`.
    ///
    /// **Threads:** any. The redirector takes a critical section around the
    /// original; this runs before it and touches no engine state.
    #[allow(improper_ctypes_definitions)]
    pub(in super::super) unsafe extern "thiscall-unwind" fn detour(
        this: *mut c_void,
        v: *const u16,
        event: i32,
    ) {
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| observe(v, event)));

        let Some(t) = TRAMPOLINE.get() else {
            return;
        };
        // The debug device the redirector forwards to calls
        // `OutputDebugStringW`; that is this line again.
        let _sink = super::super::nesting::SinkGuard::enter();
        let original: unsafe extern "thiscall-unwind" fn(*mut c_void, *const u16, i32) =
            unsafe { std::mem::transmute(*t) };
        original(this, v, event);
    }

    #[cfg(test)]
    mod tests {
        use super::super::super::emit::take_captured;
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static SEEN: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];

        unsafe extern "thiscall-unwind" fn fake_original(
            this: *mut c_void,
            v: *const u16,
            event: i32,
        ) {
            SEEN[0].store(this as usize, Ordering::SeqCst);
            SEEN[1].store(v as usize, Ordering::SeqCst);
            SEEN[2].store(event as usize, Ordering::SeqCst);
        }

        /// In a test process there is no name table, so the category falls
        /// back to `name#<index>`; the line and both arguments still go
        /// through.
        #[test]
        fn the_detour_reports_the_line_and_forwards_both_arguments() {
            TRAMPOLINE
                .set(fake_original as *const () as usize)
                .expect("only this test sets the trampoline");
            let line: Vec<u16> = "Script call stack:\r\n"
                .encode_utf16()
                .chain(Some(0))
                .collect();
            let this = [0usize; 1];
            unsafe { detour(this.as_ptr() as *mut c_void, line.as_ptr(), 0x301) };

            assert_eq!(SEEN[0].load(Ordering::SeqCst), this.as_ptr() as usize);
            assert_eq!(SEEN[1].load(Ordering::SeqCst), line.as_ptr() as usize);
            assert_eq!(SEEN[2].load(Ordering::SeqCst), 0x301);

            let events = take_captured();
            assert_eq!(events.len(), 1);
            let e = &events[0];
            assert_eq!(e.target, TARGET);
            assert_eq!(e.bridge_kind, "ue3.log");
            assert_eq!(e.get("message"), Some(&json!("Script call stack:")));
            assert_eq!(e.get("event"), Some(&json!(0x301)));
            assert_eq!(e.get("category"), Some(&json!("name#769")));
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
        "ue3_log",
        ADDR_REDIRECTOR_SERIALIZE,
        detour as *mut std::ffi::c_void,
        &TRAMPOLINE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_map_to_levels() {
        assert_eq!(telemetry_level("Error"), "error");
        assert_eq!(telemetry_level("Critical"), "error");
        assert_eq!(telemetry_level("Warning"), "warn");
        assert_eq!(telemetry_level("ScriptWarning"), "warn");
        assert_eq!(telemetry_level("DevNet"), "debug");
        assert_eq!(telemetry_level("DevNetTraffic"), "debug");
        assert_eq!(telemetry_level("Log"), "info");
        assert_eq!(telemetry_level("ScriptLog"), "info");
        assert_eq!(telemetry_level("Init"), "info");
        assert_eq!(telemetry_level("name#5"), "info");
    }

    /// Warnings are keyed on their text (two different ones never hide each
    /// other); chatty categories share one bucket.
    #[test]
    fn warnings_are_keyed_by_text_and_chatter_by_category() {
        assert_ne!(
            throttle_key("Warning", "Accessed None 'Foo'"),
            throttle_key("Warning", "Divide by zero")
        );
        // The same warning with different numbers is one bucket.
        assert_eq!(
            throttle_key("Warning", "Actor 12 not found"),
            throttle_key("Warning", "Actor 99 not found")
        );
        assert_eq!(throttle_key("Log", "a"), throttle_key("Log", "b"));
        assert_eq!(throttle_key("Log", "a"), "Log");
    }

    #[test]
    fn fields_flag_a_suppressed_category() {
        let f = line_fields("DevNet", 0x30a, "x", false, true, 0);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("category"), Some(json!("DevNet")));
        assert_eq!(get("event"), Some(json!(0x30a)));
        assert_eq!(get("suppressed_category"), Some(json!(true)));
        assert_eq!(get("suppressed"), None);
        let f = line_fields("Log", 1, "x", true, false, 4);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("suppressed_category"), None);
        assert_eq!(get("suppressed"), Some(json!(4)));
        assert_eq!(get("truncated"), Some(json!(true)));
    }

    #[test]
    fn the_hooked_address_is_the_ghidra_one() {
        assert_eq!(ADDR_REDIRECTOR_SERIALIZE, 0x004c_e0b0);
    }
}
