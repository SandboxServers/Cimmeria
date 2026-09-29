//! PhysX errors: every message the PhysX SDK reports to the client's error
//! stream, which the client throws away.
//!
//! PhysX 2.x reports through an `NxUserOutputStream` the application passes
//! to `NxCreatePhysicsSDK` (imported from `PhysXLoader.dll`, IAT
//! `0x017efd34`, called from `FUN_005590c0`). UE3's is `FNxOutputStream`
//! (RTTI `.?AVFNxOutputStream@@`, vtable `0x01839d94`):
//!
//! | Slot | Address | Method | Body |
//! |---|---|---|---|
//! | 0 | `0x0055c5e0` | `reportError(NxErrorCode code, const char* message, const char* file, int line)` | `__thiscall`, `ret 0x10`; copies `message` into a string and compares it against two known texts (`"Mesh has a negative volume!"`, `"Creating static compound shape"`); reports nothing |
//! | 1 | `0x0055c5d0` | `reportAssertViolation(const char* message, const char* file, int line)` | `mov eax, 2; ret 0xc` (answers "ignore"); reports nothing |
//! | 2 | `0x00af6810` | `print(const char*)` | `RET 4`: does nothing |
//! | 3 | `0x0055c700` | the scalar deleting destructor | sets the vtable, frees when the delete bit is set |
//!
//! So every PhysX warning (a convex mesh with a bad hull, a joint with a
//! degenerate axis, an out-of-memory in the cooker) is dropped on the floor,
//! and an assertion inside the SDK is answered "ignore". Hooking slots 0 and 1
//! turns both into events:
//!
//! - **`client.physx.error`**: the code, message, file and line.
//! - **`client.physx.assert`**: message, file and line; the original's answer
//!   (`2`) is passed through untouched.
//!
//! The error code is reported as the integer. Codes 1-5 are the SDK's
//! `NXE_INVALID_PARAMETER`, `NXE_INVALID_OPERATION`, `NXE_OUT_OF_MEMORY`,
//! `NXE_INTERNAL_ERROR` and `NXE_ASSERTION`; the debug codes are `NXE_DB_INFO`,
//! `NXE_DB_WARNING` and `NXE_DB_PRINT`. The numeric values of the debug codes
//! come from memory of the 2.8 headers and are not confirmed against this
//! build (its PhysX headers are not available), so only 1-5 get names here;
//! any other code is reported as `code_name = "other"` with the integer.
//!
//! Static evidence only (2026-09-28: Ghidra decompile and disassembly of
//! slots 0 and 1, the RTTI walk to the vtable); not yet seen from a live
//! client.

use serde_json::json;

use crate::hooks::sinks::emit::Fields;
use crate::hooks::sinks::text;

/// `FNxOutputStream` vtable slot 0 (`reportError`).
pub const SLOT_REPORT_ERROR: usize = 0x0183_9d94;
/// `FNxOutputStream` vtable slot 1 (`reportAssertViolation`).
pub const SLOT_REPORT_ASSERT: usize = 0x0183_9d98;

/// Longest message, file name read.
pub const MAX_CHARS: usize = 256;

/// Telemetry target of an error.
pub const ERROR_TARGET: &str = "client.physx.error";
/// Telemetry target of an assertion.
pub const ASSERT_TARGET: &str = "client.physx.assert";

/// The name of an `NxErrorCode` 1-5; anything else is `other`.
pub fn code_name(code: i32) -> &'static str {
    match code {
        1 => "invalid_parameter",
        2 => "invalid_operation",
        3 => "out_of_memory",
        4 => "internal_error",
        5 => "assertion",
        _ => "other",
    }
}

/// Telemetry level: the five real error codes are warnings (an out-of-memory
/// or internal error is an error), everything else is `debug`.
pub fn level(code: i32) -> &'static str {
    match code {
        3..=5 => "error",
        1 | 2 => "warn",
        _ => "debug",
    }
}

/// Rate-limit key: the code and the message's shape.
pub fn throttle_key(code: i32, message: &str) -> String {
    format!("{code}:{}", text::message_shape(message))
}

/// The fields of one error.
pub fn error_fields(code: i32, message: &str, file: &str, line: i32, suppressed: u64) -> Fields {
    let mut f: Fields = vec![
        ("code", json!(code)),
        ("code_name", json!(code_name(code))),
        ("message", json!(message)),
        ("file", json!(file)),
        ("line", json!(line)),
    ];
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

/// The fields of one assertion.
pub fn assert_fields(message: &str, file: &str, line: i32, suppressed: u64) -> Fields {
    let mut f: Fields = vec![
        ("message", json!(message)),
        ("file", json!(file)),
        ("line", json!(line)),
    ];
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod x86 {
    use std::ffi::{c_char, c_void};
    use std::panic::AssertUnwindSafe;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::hooks::name_throttle::Decision;
    use crate::hooks::sinks::emit::emit;
    use crate::hooks::sinks::mem;
    use crate::hooks::sinks::throttle::SinkThrottle;

    pub(in crate::hooks) static ORIG_ERROR: AtomicUsize = AtomicUsize::new(0);
    pub(in crate::hooks) static ORIG_ASSERT: AtomicUsize = AtomicUsize::new(0);

    static ERROR_THROTTLE: SinkThrottle = SinkThrottle::new();
    static ASSERT_THROTTLE: SinkThrottle = SinkThrottle::new();

    fn read_c(ptr: *const c_char) -> String {
        mem::ansi_at(ptr as usize, MAX_CHARS).map_or_else(
            || "<unreadable>".to_string(),
            |s| text::decode_ansi(&s.units),
        )
    }

    /// `void reportError(NxErrorCode, const char*, const char*, int)`, `ret 0x10`.
    #[allow(improper_ctypes_definitions)]
    pub(in crate::hooks) unsafe extern "thiscall-unwind" fn error_detour(
        this: *mut c_void,
        code: i32,
        message: *const c_char,
        file: *const c_char,
        line: i32,
    ) {
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let msg = read_c(message);
            let Decision::Emit { suppressed } = ERROR_THROTTLE.check(&throttle_key(code, &msg))
            else {
                return;
            };
            emit(
                ERROR_TARGET,
                level(code),
                "physx.error",
                error_fields(code, text::trim_line(&msg), &read_c(file), line, suppressed),
            );
        }));
        let orig = ORIG_ERROR.load(Ordering::Acquire);
        if orig != 0 {
            let original: unsafe extern "thiscall-unwind" fn(
                *mut c_void,
                i32,
                *const c_char,
                *const c_char,
                i32,
            ) = unsafe { std::mem::transmute(orig) };
            original(this, code, message, file, line);
        }
    }

    /// `NxAssertResponse reportAssertViolation(const char*, const char*, int)`, `ret 0xc`.
    #[allow(improper_ctypes_definitions)]
    pub(in crate::hooks) unsafe extern "thiscall-unwind" fn assert_detour(
        this: *mut c_void,
        message: *const c_char,
        file: *const c_char,
        line: i32,
    ) -> u32 {
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let msg = read_c(message);
            let Decision::Emit { suppressed } = ASSERT_THROTTLE.check(&text::message_shape(&msg))
            else {
                return;
            };
            emit(
                ASSERT_TARGET,
                "error",
                "physx.assert",
                assert_fields(text::trim_line(&msg), &read_c(file), line, suppressed),
            );
        }));
        let orig = ORIG_ASSERT.load(Ordering::Acquire);
        if orig == 0 {
            // The stream's own answer is "ignore" (2).
            return 2;
        }
        let original: unsafe extern "thiscall-unwind" fn(
            *mut c_void,
            *const c_char,
            *const c_char,
            i32,
        ) -> u32 = unsafe { std::mem::transmute(orig) };
        original(this, message, file, line)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::hooks::sinks::emit::take_captured;

        static SEEN: [AtomicUsize; 5] = [const { AtomicUsize::new(0) }; 5];

        unsafe extern "thiscall-unwind" fn fake_error(
            this: *mut c_void,
            code: i32,
            m: *const c_char,
            f: *const c_char,
            l: i32,
        ) {
            SEEN[0].store(this as usize, Ordering::SeqCst);
            SEEN[1].store(code as usize, Ordering::SeqCst);
            SEEN[2].store(m as usize, Ordering::SeqCst);
            SEEN[3].store(f as usize, Ordering::SeqCst);
            SEEN[4].store(l as usize, Ordering::SeqCst);
        }
        unsafe extern "thiscall-unwind" fn fake_assert(
            _this: *mut c_void,
            _m: *const c_char,
            _f: *const c_char,
            _l: i32,
        ) -> u32 {
            2
        }

        #[test]
        fn errors_and_assertions_are_reported_and_forwarded() {
            ORIG_ERROR.store(fake_error as *const () as usize, Ordering::SeqCst);
            ORIG_ASSERT.store(fake_assert as *const () as usize, Ordering::SeqCst);
            let _ = take_captured();

            let msg = c"Mesh has a negative volume!\n";
            let file = c"NxConvexMesh.cpp";
            unsafe { error_detour(0x10 as *mut c_void, 1, msg.as_ptr(), file.as_ptr(), 412) };
            assert_eq!(SEEN[0].load(Ordering::SeqCst), 0x10);
            assert_eq!(SEEN[1].load(Ordering::SeqCst), 1);
            assert_eq!(SEEN[2].load(Ordering::SeqCst), msg.as_ptr() as usize);
            assert_eq!(SEEN[4].load(Ordering::SeqCst), 412);
            let events = take_captured();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].target, ERROR_TARGET);
            assert_eq!(events[0].level, "warn");
            assert_eq!(
                events[0].get("code_name"),
                Some(&json!("invalid_parameter"))
            );
            assert_eq!(
                events[0].get("message"),
                Some(&json!("Mesh has a negative volume!"))
            );
            assert_eq!(events[0].get("file"), Some(&json!("NxConvexMesh.cpp")));
            assert_eq!(events[0].get("line"), Some(&json!(412)));

            let r = unsafe { assert_detour(std::ptr::null_mut(), msg.as_ptr(), file.as_ptr(), 9) };
            assert_eq!(r, 2, "the original's answer is passed through");
            let events = take_captured();
            assert_eq!(events[0].target, ASSERT_TARGET);
            assert_eq!(events[0].level, "error");
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(in crate::hooks) use x86::{assert_detour, error_detour, ORIG_ASSERT, ORIG_ERROR};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_five_sdk_codes_have_names_and_the_rest_do_not() {
        assert_eq!(code_name(1), "invalid_parameter");
        assert_eq!(code_name(2), "invalid_operation");
        assert_eq!(code_name(3), "out_of_memory");
        assert_eq!(code_name(4), "internal_error");
        assert_eq!(code_name(5), "assertion");
        assert_eq!(code_name(0), "other");
        assert_eq!(code_name(205), "other");
    }

    #[test]
    fn severe_codes_are_errors_and_debug_codes_are_debug() {
        assert_eq!(level(1), "warn");
        assert_eq!(level(2), "warn");
        assert_eq!(level(3), "error");
        assert_eq!(level(4), "error");
        assert_eq!(level(5), "error");
        assert_eq!(level(206), "debug");
    }

    #[test]
    fn the_same_message_with_different_numbers_shares_a_bucket() {
        assert_eq!(
            throttle_key(1, "Joint 12 axis is degenerate"),
            throttle_key(1, "Joint 99 axis is degenerate")
        );
        assert_ne!(throttle_key(1, "a"), throttle_key(2, "a"));
    }

    #[test]
    fn the_slots_are_the_output_stream_vtables_first_two() {
        // Vtable 0x01839d94; slot n at base + 4n.
        assert_eq!(SLOT_REPORT_ERROR, 0x0183_9d94);
        assert_eq!(SLOT_REPORT_ASSERT, SLOT_REPORT_ERROR + 4);
    }
}
