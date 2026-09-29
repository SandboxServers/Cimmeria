//! The UE3 `check()` reporter: every failed engine assertion, with the
//! expression and the source file and line.
//!
//! # Where it hooks
//!
//! `0x00486000` is `__cdecl(const char* expr, const char* file, int line)`.
//! Every UE3 `check()`/`checkSlow()` in this build compiles to
//! `if (!cond) FUN_00486000("cond", ".\\Src\\File.cpp", line)`: call sites
//! throughout `UnLevAct.cpp`, `UnWorld.cpp` and `UnObj.cpp` (`"CurrentLevel"`
//! at `UnLevAct.cpp:0x84`, `"GWorld == this"` at 0x86, `"ThisActor->IsValid()"`
//! at 0x1ad, `"StreamingLevel"` at `UnWorld.cpp:0x440`, ...). The
//! function copies the two strings into `std::string`s, hands them to the
//! assertion reporter (`0x00a5ab70` singleton, `0x00a5ad80`) and *returns*:
//! a failed `check` in this client is reported, not fatal. That makes this
//! hook the way to see engine assertions that fire and are survived, which
//! nothing else in the client records.
//!
//! The expression and file are narrow C strings in `.rdata`; the line is an
//! integer.
//!
//! Static evidence only (2026-09-28, Ghidra: decompile of `0x00486000` and
//! of its callers `0x00876970`, `0x00875290`); not yet seen from a live
//! client.

use serde_json::json;

use super::emit::Fields;

/// Entry of the `check()` reporter.
pub const ADDR_CHECK_FAILED: usize = 0x0048_6000;

/// Longest expression or path kept.
pub const MAX_CHARS: usize = 256;

/// Telemetry target.
pub const TARGET: &str = "client.ue3.assert";

/// Rate-limit key: one bucket per source line.
pub fn throttle_key(file: &str, line: u32) -> String {
    format!("{file}:{line}")
}

/// The fields of one failed check.
pub fn assert_fields(expr: &str, file: &str, line: u32, suppressed: u64) -> Fields {
    let mut f: Fields = vec![
        ("expr", json!(expr)),
        ("file", json!(file)),
        ("line", json!(line)),
    ];
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

/// A path as the engine wrote it (`.\Src\UnLevAct.cpp`) reduced to the file
/// name, so the SigNoz field is the same however the build was rooted.
pub fn base_name(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod x86 {
    use std::ffi::c_char;
    use std::panic::AssertUnwindSafe;
    use std::sync::OnceLock;

    use super::super::mem;
    use super::super::text;
    use super::super::throttle::SinkThrottle;
    use super::*;
    use crate::hooks::name_throttle::Decision;

    pub(in super::super) static TRAMPOLINE: OnceLock<usize> = OnceLock::new();

    static THROTTLE: SinkThrottle = SinkThrottle::new();

    fn read_c(ptr: *const c_char) -> String {
        mem::ansi_at(ptr as usize, MAX_CHARS).map_or_else(
            || "<unreadable>".to_string(),
            |s| text::decode_ansi(&s.units),
        )
    }

    fn observe(expr: *const c_char, file: *const c_char, line: u32) {
        let file = read_c(file);
        let Decision::Emit { suppressed } = THROTTLE.check(&throttle_key(&file, line)) else {
            return;
        };
        let expr = read_c(expr);
        super::super::emit::emit(
            TARGET,
            "error",
            "ue3.assert",
            assert_fields(&expr, base_name(&file), line, suppressed),
        );
    }

    /// Detour for the `check()` reporter.
    ///
    /// **Threads:** any (asserts fire on the game, render and load threads).
    #[allow(improper_ctypes_definitions)]
    pub(in super::super) unsafe extern "C-unwind" fn detour(
        expr: *const c_char,
        file: *const c_char,
        line: u32,
    ) {
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| observe(expr, file, line)));

        let Some(t) = TRAMPOLINE.get() else {
            return;
        };
        let original: unsafe extern "C-unwind" fn(*const c_char, *const c_char, u32) =
            unsafe { std::mem::transmute(*t) };
        original(expr, file, line);
    }

    #[cfg(test)]
    mod tests {
        use super::super::super::emit::take_captured;
        use super::*;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static SEEN: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];

        unsafe extern "C-unwind" fn fake_original(
            expr: *const c_char,
            file: *const c_char,
            line: u32,
        ) {
            SEEN[0].store(expr as usize, Ordering::SeqCst);
            SEEN[1].store(file as usize, Ordering::SeqCst);
            SEEN[2].store(line as usize, Ordering::SeqCst);
        }

        #[test]
        fn the_detour_reports_the_check_and_forwards_all_three_arguments() {
            TRAMPOLINE
                .set(fake_original as *const () as usize)
                .expect("only this test sets the trampoline");
            let expr = c"GWorld == this";
            let file = c".\\Src\\UnLevAct.cpp";
            unsafe { detour(expr.as_ptr(), file.as_ptr(), 0x86) };

            assert_eq!(SEEN[0].load(Ordering::SeqCst), expr.as_ptr() as usize);
            assert_eq!(SEEN[1].load(Ordering::SeqCst), file.as_ptr() as usize);
            assert_eq!(SEEN[2].load(Ordering::SeqCst), 0x86);

            let events = take_captured();
            assert_eq!(events.len(), 1);
            let e = &events[0];
            assert_eq!(e.target, TARGET);
            assert_eq!(e.level, "error");
            assert_eq!(e.get("expr"), Some(&json!("GWorld == this")));
            assert_eq!(e.get("file"), Some(&json!("UnLevAct.cpp")));
            assert_eq!(e.get("line"), Some(&json!(0x86)));
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
        "ue3_assert",
        ADDR_CHECK_FAILED,
        detour as *mut std::ffi::c_void,
        &TRAMPOLINE,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_reduces_to_its_file_name() {
        assert_eq!(base_name(".\\Src\\UnLevAct.cpp"), "UnLevAct.cpp");
        assert_eq!(base_name("c:/x/y/Foo.h"), "Foo.h");
        assert_eq!(base_name("bare.cpp"), "bare.cpp");
        assert_eq!(base_name(""), "");
    }

    #[test]
    fn one_bucket_per_source_line() {
        assert_ne!(throttle_key("A.cpp", 1), throttle_key("A.cpp", 2));
        assert_ne!(throttle_key("A.cpp", 1), throttle_key("B.cpp", 1));
        assert_eq!(throttle_key("A.cpp", 1), "A.cpp:1");
    }

    #[test]
    fn fields_carry_expr_file_line() {
        let f = assert_fields("Template!=NULL", "UnLevAct.cpp", 0xbc, 0);
        assert!(f.contains(&("expr", json!("Template!=NULL"))));
        assert!(f.contains(&("file", json!("UnLevAct.cpp"))));
        assert!(f.contains(&("line", json!(0xbc))));
        assert!(!f.iter().any(|(k, _)| *k == "suppressed"));
        assert!(assert_fields("x", "y", 1, 3).contains(&("suppressed", json!(3))));
    }

    #[test]
    fn the_hooked_address_is_the_ghidra_one() {
        assert_eq!(ADDR_CHECK_FAILED, 0x0048_6000);
    }
}
