//! The x86-only half of the [log4cxx sink](super::log4cxx): the IAT detours
//! for `Logger::forcedLog` (narrow and wide) and the four `is*Enabled`
//! checks. Split out of `log4cxx.rs` to keep both under the size cap; the
//! pure logic, the layouts and the evidence live there.

use std::ffi::c_void;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::install::SlotImport;
use super::log4cxx::*;
use super::mem;
use super::throttle::SinkThrottle;
use crate::hooks::name_throttle::Decision;
use crate::msvc_string::{self, Width};

pub(super) static ORIG_FORCED_A: AtomicUsize = AtomicUsize::new(0);
pub(super) static ORIG_FORCED_W: AtomicUsize = AtomicUsize::new(0);
pub(super) static ORIG_IS_ERROR: AtomicUsize = AtomicUsize::new(0);
pub(super) static ORIG_IS_WARN: AtomicUsize = AtomicUsize::new(0);
pub(super) static ORIG_IS_DEBUG: AtomicUsize = AtomicUsize::new(0);
pub(super) static ORIG_IS_INFO: AtomicUsize = AtomicUsize::new(0);

static THROTTLE: SinkThrottle = SinkThrottle::new();

const fn import(slot: usize, symbol: &'static std::ffi::CStr) -> SlotImport {
    SlotImport {
        slot,
        module: "log4cxx.dll",
        symbol,
    }
}

pub(super) const IMPORT_FORCED_A: SlotImport = import(
    IAT_FORCED_LOG_A,
    c"?forcedLog@Logger@log4cxx@@QBEXABV?$ObjectPtrT@VLevel@log4cxx@@@helpers@2@ABV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@ABVLocationInfo@spi@2@@Z",
);
pub(super) const IMPORT_FORCED_W: SlotImport = import(
    IAT_FORCED_LOG_W,
    c"?forcedLog@Logger@log4cxx@@QBEXABV?$ObjectPtrT@VLevel@log4cxx@@@helpers@2@ABV?$basic_string@_WU?$char_traits@_W@std@@V?$allocator@_W@2@@std@@ABVLocationInfo@spi@2@@Z",
);
pub(super) const IMPORT_IS_ERROR: SlotImport = import(
    IAT_IS_ERROR_ENABLED,
    c"?isErrorEnabled@Logger@log4cxx@@QBE_NXZ",
);
pub(super) const IMPORT_IS_WARN: SlotImport = import(
    IAT_IS_WARN_ENABLED,
    c"?isWarnEnabled@Logger@log4cxx@@QBE_NXZ",
);
pub(super) const IMPORT_IS_DEBUG: SlotImport = import(
    IAT_IS_DEBUG_ENABLED,
    c"?isDebugEnabled@Logger@log4cxx@@QBE_NXZ",
);
pub(super) const IMPORT_IS_INFO: SlotImport = import(
    IAT_IS_INFO_ENABLED,
    c"?isInfoEnabled@Logger@log4cxx@@QBE_NXZ",
);

/// Read a `std::string`/`std::wstring` argument of a hooked call.
///
/// # Safety
///
/// `obj` is a live string object the caller passed by reference.
unsafe fn read_message(obj: *const c_void, width: Width) -> (String, bool) {
    match unsafe { msvc_string::read(obj as *const u8, width, MAX_MESSAGE) } {
        Some(d) => (d.text, d.truncated),
        None => ("<unreadable>".to_string(), false),
    }
}

const MAX_MESSAGE: usize = super::text::MAX_MESSAGE_CHARS + 16;

fn observe(
    logger: *const c_void,
    level: *const c_void,
    message: *const c_void,
    loc: *const c_void,
    wide: bool,
) {
    let read = &mem::process_reader;
    // SAFETY: `message` is the `const std::[w]string&` argument of the
    // hooked call, alive until it returns.
    let (raw, raw_truncated) =
        unsafe { read_message(message, if wide { Width::Wide } else { Width::Narrow }) };
    let (message, cut) = super::text::message_field(&raw);
    let level_int = resolve_level(read, level as usize);
    // SAFETY: `logger` is the `Logger` the method was called on.
    let logger_name = unsafe {
        msvc_string::read(
            (logger as usize + LOGGER_NAME_OFFSET) as *const u8,
            Width::Wide,
            MAX_NAME_CHARS,
        )
    }
    .map_or_else(|| "?".to_string(), |d| d.text);

    let key = throttle_key(&logger_name, level_int.unwrap_or(0), &message);
    let Decision::Emit { suppressed } = THROTTLE.check(&key) else {
        return;
    };
    let (file, line, method) = read_location(read, loc as usize);
    emit_event(
        &EventInfo {
            logger: logger_name,
            level: level_int,
            message,
            truncated: cut || raw_truncated,
            file,
            line,
            method,
        },
        wide,
        suppressed,
    );
}

type ForcedLogFn =
    unsafe extern "thiscall-unwind" fn(*const c_void, *const c_void, *const c_void, *const c_void);

/// `Logger::forcedLog(const LevelPtr&, const std::string&, const LocationInfo&)`.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn forced_log_a(
    this: *const c_void,
    level: *const c_void,
    message: *const c_void,
    loc: *const c_void,
) {
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        observe(this, level, message, loc, false)
    }));
    let orig = ORIG_FORCED_A.load(Ordering::Acquire);
    if orig != 0 {
        let original: ForcedLogFn = unsafe { std::mem::transmute(orig) };
        original(this, level, message, loc);
    }
}

/// `Logger::forcedLog(const LevelPtr&, const std::wstring&, const LocationInfo&)`.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn forced_log_w(
    this: *const c_void,
    level: *const c_void,
    message: *const c_void,
    loc: *const c_void,
) {
    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
        observe(this, level, message, loc, true)
    }));
    let orig = ORIG_FORCED_W.load(Ordering::Acquire);
    if orig != 0 {
        let original: ForcedLogFn = unsafe { std::mem::transmute(orig) };
        original(this, level, message, loc);
    }
}

/// `bool Logger::isXEnabled() const`: `true` under `unfilter`, else the
/// client's own answer.
fn is_enabled(orig: &AtomicUsize, this: *const c_void) -> bool {
    if crate::capture::unfilter() {
        return true;
    }
    let addr = orig.load(Ordering::Acquire);
    if addr == 0 {
        return false;
    }
    let original: unsafe extern "thiscall-unwind" fn(*const c_void) -> bool =
        unsafe { std::mem::transmute(addr) };
    unsafe { original(this) }
}

/// `Logger::isErrorEnabled`.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn is_error_enabled(this: *const c_void) -> bool {
    is_enabled(&ORIG_IS_ERROR, this)
}
/// `Logger::isWarnEnabled`.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn is_warn_enabled(this: *const c_void) -> bool {
    is_enabled(&ORIG_IS_WARN, this)
}
/// `Logger::isDebugEnabled`.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn is_debug_enabled(this: *const c_void) -> bool {
    is_enabled(&ORIG_IS_DEBUG, this)
}
/// `Logger::isInfoEnabled`.
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "thiscall-unwind" fn is_info_enabled(this: *const c_void) -> bool {
    is_enabled(&ORIG_IS_INFO, this)
}

#[cfg(test)]
mod tests {
    use super::super::emit::take_captured;
    use super::*;
    use serde_json::json;

    static SEEN: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];

    unsafe extern "thiscall-unwind" fn fake_forced(
        this: *const c_void,
        level: *const c_void,
        message: *const c_void,
        loc: *const c_void,
    ) {
        SEEN[0].store(this as usize, Ordering::SeqCst);
        SEEN[1].store(level as usize, Ordering::SeqCst);
        SEEN[2].store(message as usize, Ordering::SeqCst);
        SEEN[3].store(loc as usize, Ordering::SeqCst);
    }

    unsafe extern "thiscall-unwind" fn fake_enabled(_this: *const c_void) -> bool {
        false
    }

    /// A narrow `std::string` with inline storage.
    fn narrow(text: &str) -> [u8; msvc_string::OBJECT_SIZE] {
        let mut obj = [0u8; msvc_string::OBJECT_SIZE];
        obj[4..4 + text.len()].copy_from_slice(text.as_bytes());
        obj[0x14..0x18].copy_from_slice(&(text.len() as u32).to_le_bytes());
        obj[0x18..0x1c].copy_from_slice(&15u32.to_le_bytes());
        obj
    }

    /// A `Logger` whose name (a wide `std::string`, inline storage) is
    /// at `+0xc`: 12 bytes of unrelated fields, then the string.
    fn logger(name: &str) -> Vec<u8> {
        let mut obj = vec![0u8; LOGGER_NAME_OFFSET + msvc_string::OBJECT_SIZE];
        let base = LOGGER_NAME_OFFSET;
        for (i, u) in name.encode_utf16().enumerate() {
            obj[base + 4 + i * 2..base + 6 + i * 2].copy_from_slice(&u.to_le_bytes());
        }
        obj[base + 0x14..base + 0x18].copy_from_slice(&(name.len() as u32).to_le_bytes());
        obj[base + 0x18..base + 0x1c].copy_from_slice(&7u32.to_le_bytes());
        obj
    }

    /// The forced-log detour, end to end over real memory: a logger
    /// named `common`, a `LevelPtr` (vtable pointer, then `Level*`)
    /// whose level int is at `+0xc`, a message, a `LocationInfo`. It
    /// reports them, forwards all four arguments, and answers the
    /// `is*Enabled` question with the original's unless `unfilter` is
    /// on. One test owns the originals and the capture switch.
    #[test]
    fn forced_log_is_reported_and_forwarded_and_unfilter_forces_enabled() {
        ORIG_FORCED_A.store(fake_forced as *const () as usize, Ordering::SeqCst);
        ORIG_IS_DEBUG.store(fake_enabled as *const () as usize, Ordering::SeqCst);

        let logger_obj = logger("common");
        let level_obj: [u32; 4] = [0, 0, 0, 10_000]; // DEBUG at +0xc
        let level_ptr: [u32; 2] = [0x1000_0000, level_obj.as_ptr() as u32];
        let message = narrow("inside lock");
        let file = c"..\\common\\lock.cpp";
        // line, file, method
        let loc: [u32; 3] = [321, file.as_ptr() as u32, 0];

        unsafe {
            forced_log_a(
                logger_obj.as_ptr().cast(),
                level_ptr.as_ptr().cast(),
                message.as_ptr().cast(),
                loc.as_ptr().cast(),
            );
        }
        assert_eq!(SEEN[0].load(Ordering::SeqCst), logger_obj.as_ptr() as usize);
        assert_eq!(SEEN[1].load(Ordering::SeqCst), level_ptr.as_ptr() as usize);
        assert_eq!(SEEN[2].load(Ordering::SeqCst), message.as_ptr() as usize);
        assert_eq!(SEEN[3].load(Ordering::SeqCst), loc.as_ptr() as usize);

        let events = take_captured();
        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!(e.target, TARGET);
        assert_eq!(e.level, "debug");
        assert_eq!(e.bridge_kind, "log4cxx.event");
        assert_eq!(e.get("logger"), Some(&json!("common")));
        assert_eq!(e.get("level"), Some(&json!("DEBUG")));
        assert_eq!(e.get("message"), Some(&json!("inside lock")));
        assert_eq!(e.get("file"), Some(&json!("..\\common\\lock.cpp")));
        assert_eq!(e.get("line"), Some(&json!(321)));
        assert_eq!(e.get("method"), None);

        // `unfilter` off: the client's answer (false) comes through;
        // on: true, without calling it.
        let l = logger_obj.as_ptr().cast();
        assert!(!unsafe { is_debug_enabled(l) });
        crate::capture::init(crate::capture::CaptureConfig {
            unfilter: true,
            firehose: false,
        });
        assert!(unsafe { is_debug_enabled(l) });
        crate::capture::init(crate::capture::CaptureConfig::default());
        assert!(!unsafe { is_debug_enabled(l) });
    }
}
