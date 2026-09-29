//! FMOD Designer events: which sound events the client starts and stops, and
//! the ones FMOD refuses.
//!
//! The client plays its sound through FMOD Event (`fmod_event.dll`,
//! `fmodex.dll`). `SGW.exe` imports the public `FMOD::Event` methods it uses
//! by decorated name; two of them are the seams:
//!
//! | IAT slot | Import |
//! |---|---|
//! | `0x017f00a4` | `?start@Event@FMOD@@QAG?AW4FMOD_RESULT@@XZ` |
//! | `0x017f0080` | `?stop@Event@FMOD@@QAG?AW4FMOD_RESULT@@_N@Z` |
//!
//! Both are `__stdcall` members (`QAG`), and MSVC passes `this` of a
//! `__stdcall` member on the stack: the call site in `FUN_009035f0`
//! (`0x00903698`: `PUSH EDX(event); CALL start`; `0x0090365d`: `PUSH 1;
//! PUSH EAX(event); CALL stop`, no stack adjustment after either) confirms
//! it. `start(Event*)` and `stop(Event*, bool immediate)`, `ret 4` and
//! `ret 8`, returning an `FMOD_RESULT` (`0` = `FMOD_OK`).
//!
//! The event's name comes from `Event::getInfo(int* index, char** name,
//! FMOD_EVENT_INFO* info)` (`?getInfo@Event@FMOD@@QAG?AW4FMOD_RESULT@@PAHPAPADPAUFMOD_EVENT_INFO@@@Z`,
//! IAT `0x017f0084`), called through the client's own import slot with the
//! same handle. It returns a pointer into FMOD's data, which is read through
//! the fault-free reader.
//!
//! One event, `client.audio.event`, with `action` = `start` or `stop`, the
//! event `name`, and FMOD's `result`. A non-zero result is a warning, so
//! "the sound never played" shows up as a queryable event. Rate-limited per
//! (action, name): a footstep fires many times a second, and one bucket per
//! event name keeps the rare ones (a music cue, a dialogue line) visible.
//!
//! FMOD's result codes are reported as the integer `result`; the meaning is
//! FMOD 4.x's `FMOD_RESULT` enum, which this build's headers are not
//! available to confirm, so no name table is attached.
//!
//! Static evidence only (2026-09-28: PE import directory, the DLL's export
//! table, the call sites above); not yet seen from a live client.

use serde_json::json;

use crate::hooks::sinks::emit::Fields;

/// IAT slot of `Event::start`.
pub const IAT_EVENT_START: usize = 0x017f_00a4;
/// IAT slot of `Event::stop`.
pub const IAT_EVENT_STOP: usize = 0x017f_0080;
/// IAT slot of `Event::getInfo`.
pub const IAT_EVENT_GET_INFO: usize = 0x017f_0084;

/// Longest event name read.
pub const MAX_NAME_CHARS: usize = 96;

/// Telemetry target.
pub const TARGET: &str = "client.audio.event";

/// Rate-limit key.
pub fn throttle_key(action: &str, name: &str) -> String {
    format!("{action}:{name}")
}

/// Telemetry level: FMOD refusing is a warning, everything else is debug
/// (a footstep is not news).
pub fn level(result: i32) -> &'static str {
    if result == 0 {
        "debug"
    } else {
        "warn"
    }
}

/// The fields of one event.
pub fn event_fields(
    action: &str,
    name: &str,
    result: i32,
    immediate: Option<bool>,
    suppressed: u64,
) -> Fields {
    let mut f: Fields = vec![
        ("action", json!(action)),
        ("name", json!(name)),
        ("result", json!(result)),
    ];
    if let Some(now) = immediate {
        f.push(("immediate", json!(now)));
    }
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
    use crate::hooks::sinks::install::SlotImport;
    use crate::hooks::sinks::mem;
    use crate::hooks::sinks::text;
    use crate::hooks::sinks::throttle::SinkThrottle;

    pub(in crate::hooks) static ORIG_START: AtomicUsize = AtomicUsize::new(0);
    pub(in crate::hooks) static ORIG_STOP: AtomicUsize = AtomicUsize::new(0);
    /// `Event::getInfo`, resolved at install from the client's own import.
    pub(in crate::hooks) static GET_INFO: AtomicUsize = AtomicUsize::new(0);

    static THROTTLE: SinkThrottle = SinkThrottle::new();

    pub(in crate::hooks) const IMPORT_START: SlotImport = SlotImport {
        slot: IAT_EVENT_START,
        module: "fmod_event.dll",
        symbol: c"?start@Event@FMOD@@QAG?AW4FMOD_RESULT@@XZ",
    };
    pub(in crate::hooks) const IMPORT_STOP: SlotImport = SlotImport {
        slot: IAT_EVENT_STOP,
        module: "fmod_event.dll",
        symbol: c"?stop@Event@FMOD@@QAG?AW4FMOD_RESULT@@_N@Z",
    };
    pub(in crate::hooks) const IMPORT_GET_INFO: SlotImport = SlotImport {
        slot: IAT_EVENT_GET_INFO,
        module: "fmod_event.dll",
        symbol: c"?getInfo@Event@FMOD@@QAG?AW4FMOD_RESULT@@PAHPAPADPAUFMOD_EVENT_INFO@@@Z",
    };

    /// `FMOD_RESULT __stdcall Event::getInfo(Event*, int*, char**, FMOD_EVENT_INFO*)`.
    type GetInfoFn = unsafe extern "stdcall-unwind" fn(
        *mut c_void,
        *mut i32,
        *mut *const c_char,
        *mut c_void,
    ) -> i32;

    /// The event's name, or `<unknown>`.
    fn event_name(event: *mut c_void) -> String {
        let f = GET_INFO.load(Ordering::Acquire);
        if f == 0 || event.is_null() {
            return "<unknown>".to_string();
        }
        let get_info: GetInfoFn = unsafe { std::mem::transmute(f) };
        let mut index = 0i32;
        let mut name: *const c_char = std::ptr::null();
        // SAFETY: `event` is the handle the client just passed to start or
        // stop; getInfo only reads FMOD's own tables.
        let r = unsafe { get_info(event, &mut index, &mut name, std::ptr::null_mut()) };
        if r != 0 || name.is_null() {
            return "<unknown>".to_string();
        }
        mem::ansi_at(name as usize, MAX_NAME_CHARS)
            .map_or_else(|| "<unknown>".to_string(), |s| text::decode_ansi(&s.units))
    }

    fn report(action: &str, event: *mut c_void, result: i32, immediate: Option<bool>) {
        let name = event_name(event);
        let Decision::Emit { suppressed } = THROTTLE.check(&throttle_key(action, &name)) else {
            return;
        };
        emit(
            TARGET,
            level(result),
            "audio.event",
            event_fields(action, &name, result, immediate, suppressed),
        );
    }

    /// `FMOD_RESULT __stdcall Event::start(Event*)`.
    #[allow(improper_ctypes_definitions)]
    pub(in crate::hooks) unsafe extern "stdcall-unwind" fn start_detour(event: *mut c_void) -> i32 {
        let orig = ORIG_START.load(Ordering::Acquire);
        if orig == 0 {
            return 0;
        }
        let original: unsafe extern "stdcall-unwind" fn(*mut c_void) -> i32 =
            unsafe { std::mem::transmute(orig) };
        let result = original(event);
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| report("start", event, result, None)));
        result
    }

    /// `FMOD_RESULT __stdcall Event::stop(Event*, bool immediate)`.
    #[allow(improper_ctypes_definitions)]
    pub(in crate::hooks) unsafe extern "stdcall-unwind" fn stop_detour(
        event: *mut c_void,
        immediate: u8,
    ) -> i32 {
        let orig = ORIG_STOP.load(Ordering::Acquire);
        if orig == 0 {
            return 0;
        }
        let original: unsafe extern "stdcall-unwind" fn(*mut c_void, u8) -> i32 =
            unsafe { std::mem::transmute(orig) };
        let result = original(event, immediate);
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            report("stop", event, result, Some(immediate != 0));
        }));
        result
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::hooks::sinks::emit::take_captured;

        static SEEN: [AtomicUsize; 2] = [const { AtomicUsize::new(0) }; 2];

        unsafe extern "stdcall-unwind" fn fake_start(event: *mut c_void) -> i32 {
            SEEN[0].store(event as usize, Ordering::SeqCst);
            // FMOD refuses events whose handle is odd.
            i32::from(event as usize & 1 == 1) * 46
        }
        unsafe extern "stdcall-unwind" fn fake_stop(event: *mut c_void, immediate: u8) -> i32 {
            SEEN[0].store(event as usize, Ordering::SeqCst);
            SEEN[1].store(immediate as usize, Ordering::SeqCst);
            0
        }
        static NAME: &std::ffi::CStr = c"footstep_grass";
        unsafe extern "stdcall-unwind" fn fake_get_info(
            _event: *mut c_void,
            index: *mut i32,
            name: *mut *const c_char,
            _info: *mut c_void,
        ) -> i32 {
            unsafe {
                *index = 7;
                *name = NAME.as_ptr();
            }
            0
        }

        #[test]
        fn start_and_stop_report_the_event_name_and_pass_results_through() {
            ORIG_START.store(fake_start as *const () as usize, Ordering::SeqCst);
            ORIG_STOP.store(fake_stop as *const () as usize, Ordering::SeqCst);
            GET_INFO.store(fake_get_info as *const () as usize, Ordering::SeqCst);
            let _ = take_captured();

            let r = unsafe { start_detour(0x1000 as *mut c_void) };
            assert_eq!(r, 0);
            assert_eq!(SEEN[0].load(Ordering::SeqCst), 0x1000);
            let events = take_captured();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].target, TARGET);
            assert_eq!(events[0].level, "debug");
            assert_eq!(events[0].get("action"), Some(&json!("start")));
            assert_eq!(events[0].get("name"), Some(&json!("footstep_grass")));

            // A refused start: the result is the original's, and it is a warning.
            let r = unsafe { start_detour(0x1001 as *mut c_void) };
            assert_eq!(r, 46);
            let events = take_captured();
            assert_eq!(events[0].level, "warn");
            assert_eq!(events[0].get("result"), Some(&json!(46)));

            let r = unsafe { stop_detour(0x2000 as *mut c_void, 1) };
            assert_eq!(r, 0);
            assert_eq!(SEEN[1].load(Ordering::SeqCst), 1);
            let events = take_captured();
            assert_eq!(events[0].get("action"), Some(&json!("stop")));
            assert_eq!(events[0].get("immediate"), Some(&json!(true)));

            // Without getInfo the event still reports, as unknown.
            GET_INFO.store(0, Ordering::SeqCst);
            let _ = unsafe { start_detour(0x3000 as *mut c_void) };
            let events = take_captured();
            assert_eq!(events[0].get("name"), Some(&json!("<unknown>")));
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(in crate::hooks) use x86::{
    start_detour, stop_detour, GET_INFO, IMPORT_GET_INFO, IMPORT_START, IMPORT_STOP, ORIG_START,
    ORIG_STOP,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_non_zero_result_is_a_warning() {
        assert_eq!(level(0), "debug");
        assert_eq!(level(46), "warn");
        assert_eq!(level(-1), "warn");
    }

    #[test]
    fn each_action_and_name_has_its_own_bucket() {
        assert_ne!(throttle_key("start", "a"), throttle_key("stop", "a"));
        assert_ne!(throttle_key("start", "a"), throttle_key("start", "b"));
    }

    #[test]
    fn fields_carry_the_action_name_result_and_immediacy() {
        let f = event_fields("stop", "music_castle", 0, Some(false), 5);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("name"), Some(json!("music_castle")));
        assert_eq!(get("immediate"), Some(json!(false)));
        assert_eq!(get("suppressed"), Some(json!(5)));
        let f = event_fields("start", "x", 0, None, 0);
        assert!(!f.iter().any(|(k, _)| *k == "immediate"));
    }

    #[test]
    fn the_iat_slots_are_the_import_directorys() {
        assert_eq!(IAT_EVENT_START, 0x017f_00a4);
        assert_eq!(IAT_EVENT_STOP, 0x017f_0080);
        assert_eq!(IAT_EVENT_GET_INFO, 0x017f_0084);
    }
}
