//! File-open failures: every `CreateFileW` and `CreateFileA` that returns
//! `INVALID_HANDLE_VALUE`, with the path and the Win32 error.
//!
//! `SGW.exe` imports both from `kernel32.dll` (IAT `0x017ef2a8` and
//! `0x017ef2a4`). The client's own log line for a missing cooked file is
//! `Error opening static cache archive <path>` (log4cxx, and the real
//! `SGWDebugLog.log` has ten of them for `covernodes_local.pak` and
//! `TextStrings.pak`); what it does not say is *why* the open failed. A
//! missing file, an access-denied and a sharing violation (an anti-virus or a
//! second client holding the file) all look the same one layer up. This hook
//! reports the error code at the call.
//!
//! The detour reads the last error immediately after the original returns and
//! writes it back before returning, so the game sees exactly the error the
//! kernel set, whatever this hook did in between (formatting and queueing
//! events can themselves set it).
//!
//! # Volume
//!
//! Successful opens (the overwhelming majority) cost one comparison and
//! report nothing. Failures are rate-limited per path; "file not found" and
//! "path not found" are routine probes (the engine tests for optional
//! packages and localised files) and are `debug`, additionally limited per
//! file extension so a run of missing packages cannot flood; every other
//! error is a `warn`.
//!
//! Static evidence only (2026-09-28: PE import directory); not yet seen from
//! a live client.

use serde_json::json;

use crate::hooks::sinks::emit::Fields;

/// IAT slot of `CreateFileA`.
pub const IAT_CREATE_FILE_A: usize = 0x017e_f2a4;
/// IAT slot of `CreateFileW`.
pub const IAT_CREATE_FILE_W: usize = 0x017e_f2a8;

/// `INVALID_HANDLE_VALUE`.
pub const INVALID_HANDLE: usize = usize::MAX;

/// Longest path kept.
pub const MAX_PATH_CHARS: usize = 260;

/// Telemetry target.
pub const TARGET: &str = "client.io.open_failed";

/// `ERROR_FILE_NOT_FOUND`.
pub const ERROR_FILE_NOT_FOUND: u32 = 2;
/// `ERROR_PATH_NOT_FOUND`.
pub const ERROR_PATH_NOT_FOUND: u32 = 3;

/// A readable name for the errors an open is likely to fail with.
pub fn error_name(code: u32) -> &'static str {
    match code {
        2 => "file_not_found",
        3 => "path_not_found",
        4 => "too_many_open_files",
        5 => "access_denied",
        19 => "write_protected",
        21 => "not_ready",
        32 => "sharing_violation",
        33 => "lock_violation",
        80 => "file_exists",
        87 => "invalid_parameter",
        123 => "invalid_name",
        183 => "already_exists",
        206 => "filename_too_long",
        1117 => "io_device_error",
        1392 => "file_corrupt",
        _ => "other",
    }
}

/// Whether the error is "the file is not there": a routine probe.
pub fn is_not_found(code: u32) -> bool {
    matches!(code, ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND)
}

/// Telemetry level: not-found is `debug`, anything else `warn`.
pub fn level(code: u32) -> &'static str {
    if is_not_found(code) {
        "debug"
    } else {
        "warn"
    }
}

/// Whether the requested access includes writing (`GENERIC_WRITE`,
/// `FILE_WRITE_DATA`, `FILE_APPEND_DATA`).
pub fn wants_write(access: u32) -> bool {
    access & (0x4000_0000 | 0x0002 | 0x0004) != 0
}

/// The extension of `path`, lower case, for the not-found bucket.
pub fn extension(path: &str) -> String {
    let name = path.rsplit(['\\', '/']).next().unwrap_or(path);
    match name.rsplit_once('.') {
        Some((_, ext)) if !ext.is_empty() && ext.len() <= 8 => ext.to_ascii_lowercase(),
        _ => "<none>".to_string(),
    }
}

/// Rate-limit key of one path (case-insensitive: Windows paths are).
pub fn path_key(code: u32, path: &str) -> String {
    format!("{code}:{}", path.to_ascii_lowercase())
}

/// The fields of one failed open.
pub fn open_fields(
    path: &str,
    code: u32,
    access: u32,
    disposition: u32,
    suppressed: u64,
) -> Fields {
    let mut f: Fields = vec![
        ("path", json!(path)),
        ("error", json!(code)),
        ("error_name", json!(error_name(code))),
        ("write", json!(wants_write(access))),
        ("disposition", json!(disposition)),
    ];
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod x86 {
    use std::ffi::c_void;
    use std::panic::AssertUnwindSafe;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use windows_sys::Win32::Foundation::{GetLastError, SetLastError};

    use super::*;
    use crate::hooks::name_throttle::Decision;
    use crate::hooks::sinks::emit::emit;
    use crate::hooks::sinks::install::SlotImport;
    use crate::hooks::sinks::mem;
    use crate::hooks::sinks::text;
    use crate::hooks::sinks::throttle::SinkThrottle;

    pub(in crate::hooks) static ORIG_A: AtomicUsize = AtomicUsize::new(0);
    pub(in crate::hooks) static ORIG_W: AtomicUsize = AtomicUsize::new(0);

    static PATH_THROTTLE: SinkThrottle = SinkThrottle::new();
    static NOT_FOUND_THROTTLE: SinkThrottle = SinkThrottle::new();

    /// Test switch: make `report` clobber the thread's last error, as real
    /// event formatting and queueing could, so the restore is what the test
    /// observes.
    #[cfg(test)]
    pub(super) static CLOBBER_LAST_ERROR: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);

    pub(in crate::hooks) const IMPORT_A: SlotImport = SlotImport {
        slot: IAT_CREATE_FILE_A,
        module: "kernel32.dll",
        symbol: c"CreateFileA",
    };
    pub(in crate::hooks) const IMPORT_W: SlotImport = SlotImport {
        slot: IAT_CREATE_FILE_W,
        module: "kernel32.dll",
        symbol: c"CreateFileW",
    };

    fn report(path: String, code: u32, access: u32, disposition: u32) {
        #[cfg(test)]
        if CLOBBER_LAST_ERROR.load(Ordering::SeqCst) {
            unsafe { SetLastError(0) };
        }
        let mut carried = 0;
        if is_not_found(code) {
            match NOT_FOUND_THROTTLE.check(&extension(&path)) {
                Decision::Emit { suppressed } => carried += suppressed,
                Decision::Suppress => return,
            }
        }
        let Decision::Emit { suppressed } = PATH_THROTTLE.check(&path_key(code, &path)) else {
            return;
        };
        emit(
            TARGET,
            level(code),
            "io.open_failed",
            open_fields(&path, code, access, disposition, carried + suppressed),
        );
    }

    type CreateFileWFn = unsafe extern "stdcall-unwind" fn(
        *const u16,
        u32,
        u32,
        *mut c_void,
        u32,
        u32,
        *mut c_void,
    ) -> usize;
    type CreateFileAFn = unsafe extern "stdcall-unwind" fn(
        *const u8,
        u32,
        u32,
        *mut c_void,
        u32,
        u32,
        *mut c_void,
    ) -> usize;

    /// `HANDLE CreateFileW(LPCWSTR, DWORD, DWORD, LPSECURITY_ATTRIBUTES, DWORD, DWORD, HANDLE)`.
    #[allow(improper_ctypes_definitions)]
    pub(in crate::hooks) unsafe extern "stdcall-unwind" fn create_file_w_detour(
        name: *const u16,
        access: u32,
        share: u32,
        security: *mut c_void,
        disposition: u32,
        flags: u32,
        template: *mut c_void,
    ) -> usize {
        let orig = ORIG_W.load(Ordering::Acquire);
        if orig == 0 {
            return INVALID_HANDLE;
        }
        let original: CreateFileWFn = unsafe { std::mem::transmute(orig) };
        let handle = original(name, access, share, security, disposition, flags, template);
        if handle == INVALID_HANDLE {
            // The error the kernel just set; everything below may change it.
            let code = unsafe { GetLastError() };
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
                if let Some(s) = mem::wide_at(name as usize, MAX_PATH_CHARS) {
                    report(text::decode_wide(&s.units), code, access, disposition);
                }
            }));
            unsafe { SetLastError(code) };
        }
        handle
    }

    /// `HANDLE CreateFileA(LPCSTR, DWORD, DWORD, LPSECURITY_ATTRIBUTES, DWORD, DWORD, HANDLE)`.
    #[allow(improper_ctypes_definitions)]
    pub(in crate::hooks) unsafe extern "stdcall-unwind" fn create_file_a_detour(
        name: *const u8,
        access: u32,
        share: u32,
        security: *mut c_void,
        disposition: u32,
        flags: u32,
        template: *mut c_void,
    ) -> usize {
        let orig = ORIG_A.load(Ordering::Acquire);
        if orig == 0 {
            return INVALID_HANDLE;
        }
        let original: CreateFileAFn = unsafe { std::mem::transmute(orig) };
        let handle = original(name, access, share, security, disposition, flags, template);
        if handle == INVALID_HANDLE {
            let code = unsafe { GetLastError() };
            let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
                if let Some(s) = mem::ansi_at(name as usize, MAX_PATH_CHARS) {
                    report(text::decode_ansi(&s.units), code, access, disposition);
                }
            }));
            unsafe { SetLastError(code) };
        }
        handle
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::hooks::sinks::emit::take_captured;

        static SEEN_ACCESS: AtomicUsize = AtomicUsize::new(0);

        /// A `CreateFileW` that fails with `ERROR_ACCESS_DENIED` for one
        /// path and succeeds for the others.
        unsafe extern "stdcall-unwind" fn fake_w(
            name: *const u16,
            access: u32,
            _share: u32,
            _sec: *mut c_void,
            _disp: u32,
            _flags: u32,
            _tpl: *mut c_void,
        ) -> usize {
            SEEN_ACCESS.store(access as usize, Ordering::SeqCst);
            let first = unsafe { *name };
            if first == u16::from(b'X') {
                unsafe { SetLastError(5) };
                INVALID_HANDLE
            } else if first == u16::from(b'M') {
                unsafe { SetLastError(2) };
                INVALID_HANDLE
            } else {
                0x1234
            }
        }

        fn wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(Some(0)).collect()
        }

        /// Failures are reported and the game's `GetLastError` is exactly
        /// the kernel's; successes cost nothing and report nothing.
        #[test]
        fn a_failed_open_is_reported_and_last_error_is_preserved() {
            ORIG_W.store(fake_w as *const () as usize, Ordering::SeqCst);
            CLOBBER_LAST_ERROR.store(true, Ordering::SeqCst);
            let _ = take_captured();

            // Success: no event.
            let ok = wide("Ok.txt");
            let h = unsafe {
                create_file_w_detour(
                    ok.as_ptr(),
                    0x8000_0000,
                    1,
                    std::ptr::null_mut(),
                    3,
                    0,
                    std::ptr::null_mut(),
                )
            };
            assert_eq!(h, 0x1234);
            assert!(take_captured().is_empty());

            // Access denied on a write.
            let bad = wide("X:\\cache\\TextStrings.pak");
            let h = unsafe {
                create_file_w_detour(
                    bad.as_ptr(),
                    0x4000_0000,
                    0,
                    std::ptr::null_mut(),
                    2,
                    0,
                    std::ptr::null_mut(),
                )
            };
            assert_eq!(h, INVALID_HANDLE);
            assert_eq!(
                unsafe { GetLastError() },
                5,
                "the kernel's error survives the hook"
            );
            let events = take_captured();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].target, TARGET);
            assert_eq!(events[0].level, "warn");
            assert_eq!(events[0].get("error_name"), Some(&json!("access_denied")));
            assert_eq!(events[0].get("write"), Some(&json!(true)));
            assert_eq!(
                events[0].get("path"),
                Some(&json!("X:\\cache\\TextStrings.pak"))
            );

            // A missing file is a debug-level probe.
            let missing = wide("Missing.pak");
            let _ = unsafe {
                create_file_w_detour(
                    missing.as_ptr(),
                    0x8000_0000,
                    1,
                    std::ptr::null_mut(),
                    3,
                    0,
                    std::ptr::null_mut(),
                )
            };
            assert_eq!(unsafe { GetLastError() }, 2);
            let events = take_captured();
            assert_eq!(events[0].level, "debug");
            assert_eq!(events[0].get("error_name"), Some(&json!("file_not_found")));
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(in crate::hooks) use x86::{
    create_file_a_detour, create_file_w_detour, IMPORT_A, IMPORT_W, ORIG_A, ORIG_W,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routine_probes_are_debug_and_real_failures_are_warnings() {
        assert_eq!(level(2), "debug");
        assert_eq!(level(3), "debug");
        assert_eq!(level(5), "warn");
        assert_eq!(level(32), "warn");
        assert!(is_not_found(2) && is_not_found(3) && !is_not_found(5));
    }

    #[test]
    fn known_errors_have_names() {
        assert_eq!(error_name(2), "file_not_found");
        assert_eq!(error_name(5), "access_denied");
        assert_eq!(error_name(32), "sharing_violation");
        assert_eq!(error_name(99999), "other");
    }

    #[test]
    fn a_write_open_is_recognised() {
        assert!(wants_write(0x4000_0000), "GENERIC_WRITE");
        assert!(wants_write(0x0002), "FILE_WRITE_DATA");
        assert!(wants_write(0xC000_0000), "read+write");
        assert!(!wants_write(0x8000_0000), "GENERIC_READ");
        assert!(!wants_write(0x0001), "FILE_READ_DATA");
    }

    #[test]
    fn extensions_bucket_missing_files() {
        assert_eq!(extension("C:\\a\\b\\Castle.UPK"), "upk");
        assert_eq!(extension("x/y/covernodes_local.pak"), "pak");
        assert_eq!(extension("noext"), "<none>");
        assert_eq!(extension("dir.d\\file"), "<none>");
        assert_eq!(extension("trailing."), "<none>");
    }

    #[test]
    fn a_path_key_ignores_case_but_not_the_error() {
        assert_eq!(path_key(2, "C:\\A.pak"), path_key(2, "c:\\a.PAK"));
        assert_ne!(path_key(2, "a"), path_key(5, "a"));
    }

    #[test]
    fn fields_carry_the_path_error_and_direction() {
        let f = open_fields("C:\\x.pak", 32, 0x8000_0000, 3, 4);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("path"), Some(json!("C:\\x.pak")));
        assert_eq!(get("error"), Some(json!(32)));
        assert_eq!(get("error_name"), Some(json!("sharing_violation")));
        assert_eq!(get("write"), Some(json!(false)));
        assert_eq!(get("suppressed"), Some(json!(4)));
    }

    #[test]
    fn the_iat_slots_are_the_import_directorys() {
        assert_eq!(IAT_CREATE_FILE_A, 0x017e_f2a4);
        assert_eq!(IAT_CREATE_FILE_W, 0x017e_f2a8);
    }
}
