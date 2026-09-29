//! The Windows debug-output sink: everything `SGW.exe` passes to
//! `OutputDebugStringA` and `OutputDebugStringW`.
//!
//! # Where it hooks
//!
//! The two import slots in `SGW.exe`'s IAT: `OutputDebugStringW` at
//! `0x017ef230` and `OutputDebugStringA` at `0x017ef32c` (from the PE import
//! directory; each is checked against what `kernel32.dll` exports before it
//! is swapped, like every IAT hook here).
//!
//! # What it adds
//!
//! The client's two known log sinks both end here: BigWorld's default
//! message output (`0x00a352f0`) calls `OutputDebugStringA`, and UE3's
//! `FOutputDeviceDebug` calls `OutputDebugStringW`. The sinks in
//! [`bw_message`](super::bw_message) and [`ue3_log`](super::ue3_log) report
//! those lines with their priority or category, so this hook skips anything
//! reported while a known sink is running ([`nesting`](super::nesting)).
//! What is left is what only this hook can see: the engine's own
//! `appOutputDebugString` callers (`UnrealScript` stack dumps, `check`
//! output in debug paths), the wx and library code statically linked into
//! the client, and any third-party DLL routing through the client's IAT.
//!
//! Rate-limited per message shape: a debug-string flood is usually one
//! message with a changing number.

use serde_json::json;

use super::emit::Fields;

/// IAT slot of `OutputDebugStringA`.
pub const IAT_OUTPUT_DEBUG_STRING_A: usize = 0x017e_f32c;

/// IAT slot of `OutputDebugStringW`.
pub const IAT_OUTPUT_DEBUG_STRING_W: usize = 0x017e_f230;

/// Longest string read, in units.
pub const MAX_UNITS: usize = 1024;

/// Telemetry target.
pub const TARGET: &str = "client.os.debug_string";

/// The fields of one debug string. `wide` says which API it came through.
pub fn string_fields(message: &str, truncated: bool, wide: bool, suppressed: u64) -> Fields {
    let mut f: Fields = vec![
        ("message", json!(message)),
        ("api", json!(if wide { "W" } else { "A" })),
    ];
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    if truncated {
        f.push(("truncated", json!(true)));
    }
    f
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod x86 {
    use std::panic::AssertUnwindSafe;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::super::install::SlotImport;
    use super::super::mem;
    use super::super::nesting;
    use super::super::text;
    use super::super::throttle::SinkThrottle;
    use super::*;
    use crate::hooks::name_throttle::Decision;

    pub(in super::super) static ORIG_A: AtomicUsize = AtomicUsize::new(0);
    pub(in super::super) static ORIG_W: AtomicUsize = AtomicUsize::new(0);

    static THROTTLE: SinkThrottle = SinkThrottle::new();

    pub(in super::super) const IMPORT_A: SlotImport = SlotImport {
        slot: IAT_OUTPUT_DEBUG_STRING_A,
        module: "kernel32.dll",
        symbol: c"OutputDebugStringA",
    };
    pub(in super::super) const IMPORT_W: SlotImport = SlotImport {
        slot: IAT_OUTPUT_DEBUG_STRING_W,
        module: "kernel32.dll",
        symbol: c"OutputDebugStringW",
    };

    fn report(raw: &str, truncated: bool, wide: bool) {
        // A known sink's own output: reported there, with its priority.
        if nesting::in_known_sink() {
            return;
        }
        let (message, cut) = text::message_field(raw);
        if message.is_empty() {
            return;
        }
        let Decision::Emit { suppressed } = THROTTLE.check(&text::message_shape(&message)) else {
            return;
        };
        super::super::emit::emit(
            TARGET,
            "info",
            "os.debug_string",
            string_fields(&message, cut || truncated, wide, suppressed),
        );
    }

    fn observe_a(s: *const u8) {
        if let Some(read) = mem::ansi_at(s as usize, MAX_UNITS) {
            report(&text::decode_ansi(&read.units), read.truncated, false);
        }
    }

    fn observe_w(s: *const u16) {
        if let Some(read) = mem::wide_at(s as usize, MAX_UNITS) {
            report(&text::decode_wide(&read.units), read.truncated, true);
        }
    }

    /// `void WINAPI OutputDebugStringA(LPCSTR)`.
    #[allow(improper_ctypes_definitions)]
    pub(in super::super) unsafe extern "stdcall-unwind" fn detour_a(s: *const u8) {
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| observe_a(s)));
        let orig = ORIG_A.load(Ordering::Acquire);
        if orig == 0 {
            return;
        }
        let original: unsafe extern "stdcall-unwind" fn(*const u8) =
            unsafe { std::mem::transmute(orig) };
        original(s);
    }

    /// `void WINAPI OutputDebugStringW(LPCWSTR)`.
    #[allow(improper_ctypes_definitions)]
    pub(in super::super) unsafe extern "stdcall-unwind" fn detour_w(s: *const u16) {
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| observe_w(s)));
        let orig = ORIG_W.load(Ordering::Acquire);
        if orig == 0 {
            return;
        }
        let original: unsafe extern "stdcall-unwind" fn(*const u16) =
            unsafe { std::mem::transmute(orig) };
        original(s);
    }

    #[cfg(test)]
    mod tests {
        use super::super::super::emit::take_captured;
        use super::*;

        static SEEN_A: AtomicUsize = AtomicUsize::new(0);
        static SEEN_W: AtomicUsize = AtomicUsize::new(0);

        unsafe extern "stdcall-unwind" fn fake_a(s: *const u8) {
            SEEN_A.store(s as usize, Ordering::SeqCst);
        }
        unsafe extern "stdcall-unwind" fn fake_w(s: *const u16) {
            SEEN_W.store(s as usize, Ordering::SeqCst);
        }

        /// One test owns both originals: they are process-wide.
        #[test]
        fn a_debug_string_is_reported_unless_a_known_sink_is_running() {
            ORIG_A.store(fake_a as *const () as usize, Ordering::SeqCst);
            ORIG_W.store(fake_w as *const () as usize, Ordering::SeqCst);
            let _ = take_captured();

            let a = c"loose narrow string\n";
            unsafe { detour_a(a.as_ptr().cast()) };
            assert_eq!(SEEN_A.load(Ordering::SeqCst), a.as_ptr() as usize);

            let w: Vec<u16> = "loose wide string".encode_utf16().chain(Some(0)).collect();
            unsafe { detour_w(w.as_ptr()) };
            assert_eq!(SEEN_W.load(Ordering::SeqCst), w.as_ptr() as usize);

            let events = take_captured();
            assert_eq!(events.len(), 2);
            assert_eq!(events[0].target, TARGET);
            assert_eq!(
                events[0].get("message"),
                Some(&json!("loose narrow string"))
            );
            assert_eq!(events[0].get("api"), Some(&json!("A")));
            assert_eq!(events[1].get("message"), Some(&json!("loose wide string")));
            assert_eq!(events[1].get("api"), Some(&json!("W")));

            // The same strings from inside a known sink are the sink's to
            // report; the original is still called.
            {
                let _sink = nesting::SinkGuard::enter();
                unsafe { detour_a(c"bw default output".as_ptr().cast()) };
                let w2: Vec<u16> = "ue3 debug device".encode_utf16().chain(Some(0)).collect();
                unsafe { detour_w(w2.as_ptr()) };
                assert_eq!(SEEN_W.load(Ordering::SeqCst), w2.as_ptr() as usize);
            }
            assert!(take_captured().is_empty());

            // A null pointer is forwarded and reports nothing.
            unsafe { detour_a(std::ptr::null()) };
            assert_eq!(SEEN_A.load(Ordering::SeqCst), 0);
            assert!(take_captured().is_empty());
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) use x86::{detour_a, detour_w, IMPORT_A, IMPORT_W, ORIG_A, ORIG_W};

/// Install both hooks.
///
/// # Safety
///
/// The IAT addresses are the QA build's (a per-slot check guards them).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install(producer: &crate::queue::Producer) {
    super::install::iat(
        producer,
        "os_debug_string_a",
        IMPORT_A,
        detour_a as *const () as usize,
        &ORIG_A,
    );
    super::install::iat(
        producer,
        "os_debug_string_w",
        IMPORT_W,
        detour_w as *const () as usize,
        &ORIG_W,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_say_which_api_and_only_set_flags() {
        let f = string_fields("hi", false, true, 0);
        assert!(f.contains(&("api", json!("W"))));
        assert!(!f
            .iter()
            .any(|(k, _)| *k == "suppressed" || *k == "truncated"));
        let f = string_fields("hi", true, false, 6);
        assert!(f.contains(&("api", json!("A"))));
        assert!(f.contains(&("suppressed", json!(6))));
        assert!(f.contains(&("truncated", json!(true))));
    }

    /// IAT addresses from the QA `SGW.exe`'s import directory
    /// (`KERNEL32.dll`); the install checks each against the module's
    /// export before swapping.
    #[test]
    fn the_iat_slots_are_the_import_directorys() {
        assert_eq!(IAT_OUTPUT_DEBUG_STRING_W, 0x017e_f230);
        assert_eq!(IAT_OUTPUT_DEBUG_STRING_A, 0x017e_f32c);
    }
}
