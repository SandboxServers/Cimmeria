//! The x86-only half of the [BigWorld message sink](super::bw_message): the
//! detour, the `_vsnprintf` formatter and the threshold override. Split out
//! of `bw_message.rs` to keep both under the size cap; the pure logic and
//! the evidence (what is hooked, why, and the layouts) live there.

use std::ffi::{c_char, c_void};
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use super::bw_message::*;
use super::mem;
use super::nesting::SinkGuard;
use super::text;
use super::throttle::SinkThrottle;
use crate::hooks::name_throttle::Decision;

/// Trampoline to the original.
pub(super) static TRAMPOLINE: OnceLock<usize> = OnceLock::new();

static THROTTLE: SinkThrottle = SinkThrottle::new();

/// The implementation object last seen, so the forced threshold is
/// written once per object rather than on every message.
static FORCED_FOR: AtomicUsize = AtomicUsize::new(0);

/// `_vsnprintf(char* buf, size_t count, const char* fmt, va_list)`.
type VsnprintfFn = unsafe extern "C" fn(*mut u8, usize, *const c_char, *const c_void) -> i32;

/// `_vsnprintf` from the system C runtime, resolved once. `msvcrt.dll` is
/// in every Windows process and its format language is the one the
/// client's `vsprintf` uses (`%s`, `%d`, `%I64d`, `%ls`).
fn vsnprintf() -> Option<VsnprintfFn> {
    static F: OnceLock<usize> = OnceLock::new();
    let addr = *F.get_or_init(|| {
        use windows_sys::Win32::System::LibraryLoader::{
            GetModuleHandleW, GetProcAddress, LoadLibraryW,
        };
        let name: Vec<u16> = "msvcrt.dll".encode_utf16().chain(Some(0)).collect();
        // SAFETY: NUL-terminated wide name; the module is never unloaded.
        unsafe {
            let mut m = GetModuleHandleW(name.as_ptr());
            if m.is_null() {
                m = LoadLibraryW(name.as_ptr());
            }
            if m.is_null() {
                return 0;
            }
            GetProcAddress(m, c"_vsnprintf".as_ptr().cast()).map_or(0, |f| f as usize)
        }
    });
    // SAFETY: the address is `_vsnprintf`, whose signature is `VsnprintfFn`.
    (addr != 0).then(|| unsafe { std::mem::transmute::<usize, VsnprintfFn>(addr) })
}

/// Format `fmt` with the caller's `va_list`, as the client would.
///
/// # Safety
///
/// `fmt` and `va` are the live arguments of the hooked call.
pub(super) unsafe fn format_c(fmt: *const c_char, va: *const c_void) -> Option<String> {
    // The format must be readable; `_vsnprintf` would fault on a bad one.
    mem::read_u8(&mem::process_reader, fmt as usize)?;
    let f = vsnprintf()?;
    let mut buf = [0u8; 1024];
    // SAFETY: `buf` is 1024 bytes and at most 1023 are written; the
    // arguments are the caller's own.
    let n = unsafe { f(buf.as_mut_ptr(), buf.len() - 1, fmt, va) };
    buf[buf.len() - 1] = 0;
    let len = if n < 0 {
        // Truncated: `_vsnprintf` filled the buffer without a NUL.
        buf.len() - 1
    } else {
        (n as usize).min(buf.len() - 1)
    };
    Some(text::decode_ansi(&buf[..len]))
}

/// Write [`FORCED_THRESHOLD`] into the implementation's filter
/// threshold, once per object.
fn force_threshold(imp: usize) {
    if imp == 0 || FORCED_FOR.load(Ordering::Acquire) == imp {
        return;
    }
    let at = imp + THRESHOLD_OFFSET;
    if mem::read_i32(&mem::process_reader, at).is_none() {
        return;
    }
    // SAFETY: `at` was just read, so it is mapped; the threshold is a
    // plain `int` the filter reads under its own lock, and a torn read
    // of an aligned word cannot happen on x86.
    unsafe { std::ptr::write_volatile(at as *mut i32, FORCED_THRESHOLD) };
    FORCED_FOR.store(imp, Ordering::Release);
}

/// Report one message, before the client's own filter runs.
fn observe(this: *mut c_void, header: *const i32, fmt: *const c_char, va: *const c_void) {
    let read = &mem::process_reader;
    let Some(component) = mem::read_i32(read, header as usize) else {
        return;
    };
    let Some(priority) = mem::read_i32(read, header as usize + 4) else {
        return;
    };
    let imp = mem::read_u32(read, this as usize).unwrap_or(0) as usize;
    if crate::capture::unfilter() {
        force_threshold(imp);
    }

    let Decision::Emit { suppressed } = THROTTLE.check(&throttle_key(fmt as usize, priority))
    else {
        return;
    };
    let threshold = if imp != 0 {
        mem::read_i32(read, imp + THRESHOLD_OFFSET)
    } else {
        None
    };
    // With the threshold forced down nothing is filtered; report what
    // the client's own setting would have done either way.
    let filtered = threshold.is_some_and(|t| would_filter(component, priority, t))
        && !crate::capture::unfilter();

    // SAFETY: `fmt`/`va` are the live arguments of the hooked call.
    let raw = unsafe { format_c(fmt, va) }.unwrap_or_else(|| "<unformattable>".to_string());
    let (msg, truncated) = text::message_field(&raw);
    super::emit::emit(
        TARGET,
        telemetry_level(priority),
        "bw.message",
        message_fields(
            &msg,
            truncated,
            component,
            priority,
            filtered,
            fmt as usize,
            suppressed,
        ),
    );
}

/// Detour for `DebugMsgHelper::message`.
///
/// **Threads:** the network thread and the main thread (BigWorld logs
/// from both). Reads its arguments through a fault-free reader, takes
/// only the throttle's short lock, and forwards everything untouched.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn detour(
    this: *mut c_void,
    header: *const i32,
    fmt: *const c_char,
    va: *const c_void,
) {
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| observe(this, header, fmt, va)));

    let Some(t) = TRAMPOLINE.get() else {
        return;
    };
    // The original's own default output calls `OutputDebugStringA`; the
    // debug-string hook must not report that a second time.
    let _sink = SinkGuard::enter();
    let original: unsafe extern "thiscall-unwind" fn(
        *mut c_void,
        *const i32,
        *const c_char,
        *const c_void,
    ) = unsafe { std::mem::transmute(*t) };
    original(this, header, fmt, va);
}

#[cfg(test)]
mod tests {
    use super::super::emit::take_captured;
    use super::*;
    use serde_json::json;
    use std::sync::atomic::AtomicU32;

    static SEEN: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];
    static CALLS: AtomicU32 = AtomicU32::new(0);

    unsafe extern "thiscall-unwind" fn fake_original(
        this: *mut c_void,
        header: *const i32,
        fmt: *const c_char,
        va: *const c_void,
    ) {
        SEEN[0].store(this as usize, Ordering::SeqCst);
        SEEN[1].store(header as usize, Ordering::SeqCst);
        SEEN[2].store(fmt as usize, Ordering::SeqCst);
        SEEN[3].store(va as usize, Ordering::SeqCst);
        CALLS.fetch_add(1, Ordering::SeqCst);
        assert!(
            crate::hooks::sinks::nesting::in_known_sink(),
            "the original must run under the sink guard"
        );
        if CALLS.load(Ordering::SeqCst) == 99 {
            panic!("engine error");
        }
    }

    /// One test owns the trampoline: it is a process-wide `OnceLock`.
    #[test]
    fn the_detour_reports_then_forwards_all_four_arguments() {
        TRAMPOLINE
            .set(fake_original as *const () as usize)
            .expect("only this test sets the trampoline");

        // `%s` `%d` over an x86 va_list: consecutive words.
        let fmt = c"Entity %s has %d items".as_ptr();
        let name = c"Ada".as_ptr();
        let va: [usize; 2] = [name as usize, 7];
        let header: [i32; 2] = [1, 4]; // component 1, WARNING
        let this = [0usize; 2]; // impl pointer 0: no threshold known

        unsafe {
            detour(
                this.as_ptr() as *mut c_void,
                header.as_ptr(),
                fmt,
                va.as_ptr() as *const c_void,
            );
        }

        assert_eq!(SEEN[0].load(Ordering::SeqCst), this.as_ptr() as usize);
        assert_eq!(SEEN[1].load(Ordering::SeqCst), header.as_ptr() as usize);
        assert_eq!(SEEN[2].load(Ordering::SeqCst), fmt as usize);
        assert_eq!(SEEN[3].load(Ordering::SeqCst), va.as_ptr() as usize);

        let events = take_captured();
        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!(e.target, TARGET);
        assert_eq!(e.level, "warn");
        assert_eq!(e.bridge_kind, "bw.message");
        assert_eq!(e.get("message"), Some(&json!("Entity Ada has 7 items")));
        assert_eq!(e.get("priority_name"), Some(&json!("WARNING")));
        assert_eq!(e.get("component_priority"), Some(&json!(1)));
        assert_eq!(e.get("fmt_addr"), Some(&json!(text::hex32(fmt as usize))));
        assert!(!super::super::nesting::in_known_sink());

        // A C++ exception out of the original unwinds through the
        // detour (the other detours' contract, #915), and the sink
        // guard is released by the unwind.
        CALLS.store(98, Ordering::SeqCst);
        let caught = std::panic::catch_unwind(|| unsafe {
            detour(
                this.as_ptr() as *mut c_void,
                header.as_ptr(),
                fmt,
                va.as_ptr() as *const c_void,
            );
        });
        assert!(caught.is_err());
        assert!(!super::super::nesting::in_known_sink());
        let _ = take_captured();
    }

    /// The formatter reads a real x86 `va_list` the way the client's
    /// `vsprintf` does: string, int, and a percent sign.
    #[test]
    fn format_c_expands_an_x86_va_list() {
        let fmt = c"%s=%d (%d%%)".as_ptr();
        let key = c"hp".as_ptr();
        let va: [usize; 3] = [key as usize, 42, 50];
        let s = unsafe { format_c(fmt, va.as_ptr() as *const c_void) };
        assert_eq!(s.as_deref(), Some("hp=42 (50%)"));
    }

    /// The assertion path passes the text as the argument of `"%s"`.
    #[test]
    fn format_c_handles_the_assertion_wrappers_percent_s() {
        let text = c"ASSERTION FAILED: entitiesEnabled_\n..\\servconn.cpp(12)";
        let va: [usize; 1] = [text.as_ptr() as usize];
        let s = unsafe { format_c(c"%s".as_ptr(), va.as_ptr() as *const c_void) };
        assert_eq!(
            s.as_deref(),
            Some("ASSERTION FAILED: entitiesEnabled_\n..\\servconn.cpp(12)")
        );
    }

    /// A message longer than the buffer is cut, not overrun.
    #[test]
    fn format_c_bounds_a_long_message() {
        let long = std::ffi::CString::new("x".repeat(3000)).unwrap();
        let va: [usize; 1] = [long.as_ptr() as usize];
        let s = unsafe { format_c(c"%s".as_ptr(), va.as_ptr() as *const c_void) }.unwrap();
        assert_eq!(s.len(), 1023);
    }

    #[test]
    fn an_unreadable_format_string_is_not_formatted() {
        let va: [usize; 0] = [];
        let s = unsafe { format_c(0x10 as *const c_char, va.as_ptr() as *const c_void) };
        assert_eq!(s, None);
    }
}
