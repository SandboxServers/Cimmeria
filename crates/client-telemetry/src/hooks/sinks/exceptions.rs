//! The first-chance exception sink: every hardware or system exception the
//! client raises that is not one of the ordinary C++/debugger ones, seen
//! before any handler in the client runs.
//!
//! A vectored exception handler is registered first in the chain and always
//! answers "continue search", so it observes and changes nothing: the
//! client's own SEH frames, UE3's `appHandleException` filter and the
//! crash-capture code in the lab bridge all still run afterwards.
//!
//! What counts as reportable (see [`is_reportable`]): the NT error class
//! (`0xC0000000`, so access violations, illegal instructions, divides by
//! zero, heap corruption, stack-buffer overruns) and custom `0xE0...` codes,
//! except the MSVC C++ throw (`0xE06D7363`: UE3 and the Lua glue throw C++
//! exceptions as ordinary control flow, thousands of times) and the CLR
//! codes. Debugger and OS plumbing (`0x406D1388` thread naming, the
//! `OutputDebugString` exceptions, breakpoints, single steps, guard-page
//! hits from stack growth) is dropped. Stack overflow is not touched at all:
//! there is no stack left to format an event on.
//!
//! The one report worth knowing in advance: when a debugger is attached,
//! `FOutputDeviceWindowsError::Serialize` (`0x004ce3a0`) deliberately writes
//! to address 3 to break into it (`MOV [0x00000003], 0xd`). That shows up
//! here as an access violation at address 3, immediately before the
//! `client.ue3.fatal_error` event.
//!
//! Rate-limited per exception code and faulting site, so a fault in a loop
//! reports its first few occurrences and then a count.

use serde_json::json;

use super::emit::Fields;

/// Telemetry target.
pub const TARGET: &str = "client.os.exception";

/// `STATUS_ACCESS_VIOLATION`.
pub const ACCESS_VIOLATION: u32 = 0xC000_0005;
/// `STATUS_STACK_OVERFLOW`: never reported (no stack to report on).
pub const STACK_OVERFLOW: u32 = 0xC000_00FD;
/// The MSVC C++ `throw`.
pub const CPP_EXCEPTION: u32 = 0xE06D_7363;

/// Whether an exception with `code` is worth an event.
pub fn is_reportable(code: u32) -> bool {
    if code == STACK_OVERFLOW || code == CPP_EXCEPTION {
        return false;
    }
    match code >> 28 {
        // NT error severity.
        0xC => true,
        // Custom codes (bit 29 set, severity 3): all but the CLR's.
        0xE => !matches!(code, 0xE043_4352 | 0xE043_4F4D),
        _ => false,
    }
}

/// A readable name for the codes the client is likely to raise.
pub fn code_name(code: u32) -> &'static str {
    match code {
        0xC000_0005 => "ACCESS_VIOLATION",
        0xC000_0006 => "IN_PAGE_ERROR",
        0xC000_0008 => "INVALID_HANDLE",
        0xC000_001D => "ILLEGAL_INSTRUCTION",
        0xC000_0025 => "NONCONTINUABLE_EXCEPTION",
        0xC000_0094 => "INTEGER_DIVIDE_BY_ZERO",
        0xC000_0095 => "INTEGER_OVERFLOW",
        0xC000_0096 => "PRIVILEGED_INSTRUCTION",
        0xC000_008C => "ARRAY_BOUNDS_EXCEEDED",
        0xC000_008D => "FLOAT_DENORMAL_OPERAND",
        0xC000_008E => "FLOAT_DIVIDE_BY_ZERO",
        0xC000_008F => "FLOAT_INEXACT_RESULT",
        0xC000_0090 => "FLOAT_INVALID_OPERATION",
        0xC000_0091 => "FLOAT_OVERFLOW",
        0xC000_0092 => "FLOAT_STACK_CHECK",
        0xC000_0093 => "FLOAT_UNDERFLOW",
        0xC000_00FD => "STACK_OVERFLOW",
        0xC000_0374 => "HEAP_CORRUPTION",
        0xC000_0409 => "STACK_BUFFER_OVERRUN",
        0xC000_0417 => "INVALID_CRUNTIME_PARAMETER",
        _ => "OTHER",
    }
}

/// The kind of access an access violation attempted, from
/// `ExceptionInformation[0]`: 0 read, 1 write, 8 execute (DEP).
pub fn access_kind(info0: usize) -> &'static str {
    match info0 {
        0 => "read",
        1 => "write",
        8 => "execute",
        _ => "unknown",
    }
}

/// Rate-limit key: the code and the faulting site.
pub fn throttle_key(code: u32, module: &str, rva: usize) -> String {
    format!("{code:08x}:{module}:{rva:x}")
}

/// What the handler read out of an `EXCEPTION_RECORD`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExceptionInfo {
    /// `ExceptionCode`.
    pub code: u32,
    /// `ExceptionAddress`.
    pub address: usize,
    /// `ExceptionFlags` (bit 0 = non-continuable).
    pub flags: u32,
    /// `ExceptionInformation[0..2]` for an access violation.
    pub access: Option<(usize, usize)>,
    /// Module containing the faulting address, if any.
    pub module: Option<String>,
    /// Offset of the address inside that module.
    pub rva: Option<usize>,
    /// Thread id.
    pub thread_id: u32,
}

/// The fields of one `client.os.exception`.
pub fn exception_fields(e: &ExceptionInfo, suppressed: u64) -> Fields {
    let mut f: Fields = vec![
        ("code", json!(format!("0x{:08x}", e.code))),
        ("code_name", json!(code_name(e.code))),
        ("address", json!(super::text::hex32(e.address))),
        ("first_chance", json!(true)),
        ("noncontinuable", json!(e.flags & 1 != 0)),
        ("thread_id", json!(e.thread_id)),
    ];
    if let Some(module) = &e.module {
        f.push(("module", json!(module)));
    }
    if let Some(rva) = e.rva {
        f.push(("rva", json!(super::text::hex32(rva))));
    }
    if e.code == ACCESS_VIOLATION {
        if let Some((kind, target)) = e.access {
            f.push(("access", json!(access_kind(kind))));
            f.push(("target", json!(super::text::hex32(target))));
        }
    }
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod x86 {
    use std::cell::Cell;
    use std::sync::atomic::{AtomicBool, Ordering};

    use windows_sys::Win32::Foundation::MAX_PATH;
    use windows_sys::Win32::System::Diagnostics::Debug::{
        AddVectoredExceptionHandler, EXCEPTION_POINTERS,
    };
    use windows_sys::Win32::System::LibraryLoader::{
        GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
        GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
    };
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;

    use super::super::throttle::SinkThrottle;
    use super::*;
    use crate::hooks::name_throttle::Decision;

    static THROTTLE: SinkThrottle = SinkThrottle::new();
    static REGISTERED: AtomicBool = AtomicBool::new(false);

    thread_local! {
        static IN_HANDLER: Cell<bool> = const { Cell::new(false) };
    }

    /// The module holding `address` and the address's offset in it.
    fn module_of(address: usize) -> Option<(String, usize)> {
        let mut hmod = std::ptr::null_mut();
        // SAFETY: FROM_ADDRESS treats the argument as an address, not a name;
        // UNCHANGED_REFCOUNT takes no reference on the module.
        let ok = unsafe {
            GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                    | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                address as *const u16,
                &mut hmod,
            )
        };
        if ok == 0 || hmod.is_null() {
            return None;
        }
        let mut buf = [0u16; MAX_PATH as usize];
        // SAFETY: `buf` is MAX_PATH wide units.
        let len = unsafe { GetModuleFileNameW(hmod, buf.as_mut_ptr(), buf.len() as u32) } as usize;
        if len == 0 || len >= buf.len() {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len]);
        let name = path.rsplit(['\\', '/']).next().unwrap_or(&path).to_string();
        Some((name, address.wrapping_sub(hmod as usize)))
    }

    /// The vectored handler: observe, never handle.
    unsafe extern "system" fn handler(info: *mut EXCEPTION_POINTERS) -> i32 {
        const CONTINUE_SEARCH: i32 = 0;
        if info.is_null() || IN_HANDLER.with(|h| h.replace(true)) {
            return CONTINUE_SEARCH;
        }
        let _ = std::panic::catch_unwind(|| {
            // SAFETY: the OS passes a valid EXCEPTION_POINTERS for the
            // duration of the call.
            let record = unsafe { (*info).ExceptionRecord };
            if record.is_null() {
                return;
            }
            let rec = unsafe { &*record };
            let code = rec.ExceptionCode as u32;
            if !is_reportable(code) {
                return;
            }
            let address = rec.ExceptionAddress as usize;
            let (module, rva) = match module_of(address) {
                Some((m, r)) => (Some(m), Some(r)),
                None => (None, None),
            };
            let key = throttle_key(
                code,
                module.as_deref().unwrap_or("?"),
                rva.unwrap_or(address),
            );
            let Decision::Emit { suppressed } = THROTTLE.check(&key) else {
                return;
            };
            let access = (code == ACCESS_VIOLATION && rec.NumberParameters >= 2)
                .then(|| (rec.ExceptionInformation[0], rec.ExceptionInformation[1]));
            let info = ExceptionInfo {
                code,
                address,
                flags: rec.ExceptionFlags,
                access,
                module,
                rva,
                thread_id: unsafe { GetCurrentThreadId() },
            };
            super::super::emit::emit(
                TARGET,
                "error",
                "os.exception",
                exception_fields(&info, suppressed),
            );
        });
        IN_HANDLER.with(|h| h.set(false));
        CONTINUE_SEARCH
    }

    /// Register the handler first in the chain. Once only.
    pub(in super::super) fn register() -> bool {
        if REGISTERED.swap(true, Ordering::SeqCst) {
            return true;
        }
        // SAFETY: `handler` matches PVECTORED_EXCEPTION_HANDLER and lives for
        // the process (the DLL never unloads).
        let h = unsafe { AddVectoredExceptionHandler(1, Some(handler)) };
        !h.is_null()
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_module_address_resolves_to_its_file_and_offset() {
            // This function's own address is inside the test executable.
            let here = a_module_address_resolves_to_its_file_and_offset as *const () as usize;
            let (name, rva) = module_of(here).expect("the test binary is a module");
            assert!(name.to_ascii_lowercase().ends_with(".exe"), "{name}");
            assert!(rva > 0 && rva < here);
            // An unmapped low address is in no module.
            assert_eq!(module_of(0x10), None);
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) use x86::register;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_faults_are_reportable() {
        assert!(is_reportable(0xC000_0005), "access violation");
        assert!(is_reportable(0xC000_001D), "illegal instruction");
        assert!(is_reportable(0xC000_0094), "integer divide by zero");
        assert!(is_reportable(0xC000_0374), "heap corruption");
        assert!(is_reportable(0xC000_0409), "stack buffer overrun");
    }

    /// The exceptions the client raises as ordinary control flow, and the
    /// debugger plumbing, are not reported: one C++ `throw` per Lua error
    /// would swamp everything.
    #[test]
    fn ordinary_exceptions_are_not_reportable() {
        assert!(!is_reportable(CPP_EXCEPTION), "MSVC C++ throw");
        assert!(!is_reportable(0x406D_1388), "thread naming");
        assert!(!is_reportable(0x4001_0006), "DBG_PRINTEXCEPTION_C");
        assert!(!is_reportable(0x4001_000A), "DBG_PRINTEXCEPTION_WIDE_C");
        assert!(!is_reportable(0x8000_0003), "breakpoint");
        assert!(!is_reportable(0x8000_0004), "single step");
        assert!(!is_reportable(0x8000_0001), "guard page");
        assert!(!is_reportable(0xE043_4352), "CLR");
        assert!(!is_reportable(0xE043_4F4D), "CLR");
    }

    /// There is no stack to format an event on after an overflow.
    #[test]
    fn a_stack_overflow_is_never_touched() {
        assert!(!is_reportable(STACK_OVERFLOW));
    }

    #[test]
    fn other_custom_codes_are_reportable() {
        assert!(is_reportable(0xE000_0001));
        assert!(is_reportable(0xE1AB_CDEF));
    }

    #[test]
    fn access_kinds_follow_exception_information() {
        assert_eq!(access_kind(0), "read");
        assert_eq!(access_kind(1), "write");
        assert_eq!(access_kind(8), "execute");
        assert_eq!(access_kind(5), "unknown");
    }

    #[test]
    fn an_access_violation_event_carries_access_and_target() {
        let e = ExceptionInfo {
            code: ACCESS_VIOLATION,
            address: 0x004c_e3c5,
            flags: 0,
            access: Some((1, 3)),
            module: Some("SGW.exe".into()),
            rva: Some(0xce3c5),
            thread_id: 77,
        };
        let f = exception_fields(&e, 0);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("code"), Some(json!("0xc0000005")));
        assert_eq!(get("code_name"), Some(json!("ACCESS_VIOLATION")));
        assert_eq!(get("address"), Some(json!("0x004ce3c5")));
        assert_eq!(get("module"), Some(json!("SGW.exe")));
        assert_eq!(get("rva"), Some(json!("0x000ce3c5")));
        assert_eq!(get("access"), Some(json!("write")));
        assert_eq!(get("target"), Some(json!("0x00000003")));
        assert_eq!(get("noncontinuable"), Some(json!(false)));
        assert_eq!(get("first_chance"), Some(json!(true)));
        assert_eq!(get("suppressed"), None);
    }

    #[test]
    fn a_non_access_exception_has_no_access_fields_and_keeps_flags() {
        let e = ExceptionInfo {
            code: 0xC000_0094,
            address: 0x1234,
            flags: 1,
            access: Some((0, 0)),
            module: None,
            rva: None,
            thread_id: 1,
        };
        let f = exception_fields(&e, 5);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("access"), None);
        assert_eq!(get("module"), None);
        assert_eq!(get("noncontinuable"), Some(json!(true)));
        assert_eq!(get("suppressed"), Some(json!(5)));
    }

    #[test]
    fn one_bucket_per_code_and_site() {
        assert_ne!(
            throttle_key(ACCESS_VIOLATION, "SGW.exe", 0x100),
            throttle_key(ACCESS_VIOLATION, "SGW.exe", 0x104)
        );
        assert_ne!(
            throttle_key(ACCESS_VIOLATION, "SGW.exe", 0x100),
            throttle_key(0xC000_0094, "SGW.exe", 0x100)
        );
    }
}
