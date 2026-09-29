//! The log4cxx sink: every event the client's log4cxx loggers emit, with the
//! logger name, level, source file and line.
//!
//! # What the client does with log4cxx
//!
//! `SGW.exe` imports 60-odd names from `log4cxx.dll` (the DLL that ships in
//! the client's `binaries` folder) and configures it from `SGWLogConfig.xml`
//! next to the executable: root level `all`, a console appender and a file
//! appender writing `SGWDebugLog.log`. In practice the file is 65 000 lines
//! of `DEBUG common - inside writeLock` lock traces with a few real `ERROR`
//! lines between them (`Error opening static cache archive ...`). The
//! client also defines its own appender, `log4cxx::UnrealAppender`
//! (`0x00c64680` returns its name), whose `append` maps a level to UE3's
//! `debugf`/`warnf`/`errorf`; those are compiled out of this build, so
//! nothing reaches `GLog` from it.
//!
//! # Where it hooks
//!
//! Two of `SGW.exe`'s `log4cxx.dll` imports are the logger's `forcedLog`,
//! the function every log macro ends in once its level check has passed:
//!
//! | Slot | Import |
//! |---|---|
//! | `0x017f0160` | `Logger::forcedLog(const LevelPtr&, const std::string&, const LocationInfo&) const` |
//! | `0x017f0188` | `Logger::forcedLog(const LevelPtr&, const std::wstring&, const LocationInfo&) const` |
//!
//! Both are `__thiscall` (`QBEX`), three stack arguments (`ret 0xc`). The
//! level checks `Logger::is{Error,Warn,Debug,Info}Enabled` are imports too
//! (`0x017f017c`, `0x017f01ac`, `0x017f01c4`, `0x017f01d8`); the client
//! never imports `isTraceEnabled` or `isFatalEnabled`.
//!
//! # Layouts (read from `log4cxx.dll`, 2026-09-28)
//!
//! - `Level::toInt()` is `mov eax, [ecx+0xc]`: the level int is at `+0xc`.
//!   Levels are `ALL` = `INT_MIN`, `TRACE` 5000, `DEBUG` 10000, `INFO`
//!   20000, `WARN` 30000, `ERROR` 40000, `FATAL` 50000, `OFF` = `INT_MAX`.
//! - `Logger::getName(std::wstring&)` copies from `Logger + 0xc`: the
//!   logger name is a wide `std::string` at `+0xc`.
//! - `LocationInfo`'s constructor stores its arguments as line at `+0`, file
//!   name pointer at `+4`, method name pointer at `+8`.
//! - `LevelPtr` is an `ObjectPtrT<Level>`. The client's own
//!   `UnrealAppender::append` reads `*(getFatal() + 4)` to compare level
//!   objects, which puts the `Level*` at `+4` behind a vtable pointer, but
//!   the sink does not rely on that: it tries `+4`, then `+0`, and accepts
//!   the candidate whose `+0xc` holds a standard level int.
//!
//! # `unfilter`
//!
//! The four `is*Enabled` slots are swapped for a function that answers
//! `true` while `unfilter` is on, so a logger whose level was raised (or
//! whose configuration did not load) still logs. Off, they forward.
//!
//! Static evidence only (2026-09-28: Ghidra strings and imports, byte dump of
//! `log4cxx.dll`); not yet seen from a live client.

use serde_json::json;

use super::emit::{self, Fields};
use super::mem::{self, Reader};

/// IAT slot of the narrow `Logger::forcedLog`.
pub const IAT_FORCED_LOG_A: usize = 0x017f_0160;
/// IAT slot of the wide `Logger::forcedLog`.
pub const IAT_FORCED_LOG_W: usize = 0x017f_0188;
/// IAT slot of `Logger::isErrorEnabled`.
pub const IAT_IS_ERROR_ENABLED: usize = 0x017f_017c;
/// IAT slot of `Logger::isWarnEnabled`.
pub const IAT_IS_WARN_ENABLED: usize = 0x017f_01ac;
/// IAT slot of `Logger::isDebugEnabled`.
pub const IAT_IS_DEBUG_ENABLED: usize = 0x017f_01c4;
/// IAT slot of `Logger::isInfoEnabled`.
pub const IAT_IS_INFO_ENABLED: usize = 0x017f_01d8;

/// Offset of the level int inside a `Level`.
pub const LEVEL_INT_OFFSET: usize = 0x0c;
/// Offset of the name (a wide `std::string`) inside a `Logger`.
pub const LOGGER_NAME_OFFSET: usize = 0x0c;
/// Offsets inside a `LocationInfo`.
pub const LOCATION_LINE_OFFSET: usize = 0x00;
/// Offset of the file name pointer inside a `LocationInfo`.
pub const LOCATION_FILE_OFFSET: usize = 0x04;
/// Offset of the method name pointer inside a `LocationInfo`.
pub const LOCATION_METHOD_OFFSET: usize = 0x08;

/// Longest logger name, file name or method name kept.
pub const MAX_NAME_CHARS: usize = 64;

/// Telemetry target.
pub const TARGET: &str = "client.log4cxx.event";

/// The standard log4cxx levels, by int.
const LEVELS: [(i32, &str); 8] = [
    (i32::MIN, "ALL"),
    (5_000, "TRACE"),
    (10_000, "DEBUG"),
    (20_000, "INFO"),
    (30_000, "WARN"),
    (40_000, "ERROR"),
    (50_000, "FATAL"),
    (i32::MAX, "OFF"),
];

/// A standard level's name, `None` for any other int.
pub fn level_name(level: i32) -> Option<&'static str> {
    LEVELS.iter().find(|(v, _)| *v == level).map(|(_, n)| *n)
}

/// Telemetry level for a log4cxx level int. Custom levels map by range.
pub fn telemetry_level(level: i32) -> &'static str {
    match level {
        i32::MIN..=19_999 => "debug",
        20_000..=29_999 => "info",
        30_000..=39_999 => "warn",
        _ => "error",
    }
}

/// Read the level int out of a `LevelPtr`, without knowing whether the
/// `ObjectPtrT` puts the `Level*` at `+4` (behind a vtable pointer) or `+0`.
/// A candidate is accepted only when the int at `+0xc` of what it points to
/// is a standard level.
pub fn resolve_level(read: Reader, level_ptr: usize) -> Option<i32> {
    [4usize, 0].into_iter().find_map(|off| {
        let level_obj = mem::read_u32(read, level_ptr + off)? as usize;
        let value = mem::read_i32(read, level_obj + LEVEL_INT_OFFSET)?;
        level_name(value).map(|_| value)
    })
}

/// Rate-limit key: the logger, the level and the message's shape.
pub fn throttle_key(logger: &str, level: i32, message: &str) -> String {
    format!("{logger}:{level}:{}", super::text::message_shape(message))
}

/// What one forced-log call carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventInfo {
    /// Logger name (`common`).
    pub logger: String,
    /// Level int, if the `LevelPtr` resolved.
    pub level: Option<i32>,
    /// Message text.
    pub message: String,
    /// Message was cut.
    pub truncated: bool,
    /// Source file, when the macro passed one.
    pub file: Option<String>,
    /// Source line.
    pub line: Option<i32>,
    /// Function name, when the macro passed one.
    pub method: Option<String>,
}

/// The fields of one `client.log4cxx.event`.
pub fn event_fields(e: &EventInfo, wide: bool, suppressed: u64) -> Fields {
    let mut f: Fields = vec![
        ("logger", json!(e.logger)),
        (
            "level",
            json!(e
                .level
                .and_then(level_name)
                .map_or_else(|| "UNKNOWN".to_string(), |n| n.to_string())),
        ),
        ("message", json!(e.message)),
        ("wide", json!(wide)),
    ];
    if let Some(level) = e.level {
        f.push(("level_int", json!(level)));
    }
    if let Some(file) = &e.file {
        f.push(("file", json!(file)));
    }
    if let Some(line) = e.line {
        f.push(("line", json!(line)));
    }
    if let Some(method) = &e.method {
        f.push(("method", json!(method)));
    }
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    if e.truncated {
        f.push(("truncated", json!(true)));
    }
    f
}

/// Read the location a log macro passed: `(file, line, method)`. A
/// `LocationInfo` a macro built with no location has null pointers.
pub fn read_location(read: Reader, loc: usize) -> (Option<String>, Option<i32>, Option<String>) {
    if loc == 0 {
        return (None, None, None);
    }
    let text = |ptr: Option<u32>| {
        let addr = ptr? as usize;
        let s = mem::read_ansi_z(read, addr, MAX_NAME_CHARS)?;
        (!s.units.is_empty()).then(|| super::text::decode_ansi(&s.units))
    };
    let file = text(mem::read_u32(read, loc + LOCATION_FILE_OFFSET));
    let line =
        mem::read_i32(read, loc + LOCATION_LINE_OFFSET).filter(|l| (0..10_000_000).contains(l));
    let method = text(mem::read_u32(read, loc + LOCATION_METHOD_OFFSET));
    (file, line, method)
}

/// Emit one event that has passed the rate limit.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
pub(super) fn emit_event(e: &EventInfo, wide: bool, suppressed: u64) {
    let level = e.level.map_or("info", telemetry_level);
    emit::emit(
        TARGET,
        level,
        "log4cxx.event",
        event_fields(e, wide, suppressed),
    );
}

/// Install every log4cxx hook: the two forced-log slots and the four level
/// checks.
///
/// # Safety
///
/// The IAT addresses are the QA build's (a per-slot check guards each).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install(producer: &crate::queue::Producer) {
    use super::install::iat;
    use super::log4cxx_detours::*;
    iat(
        producer,
        "log4cxx_forced_log_a",
        IMPORT_FORCED_A,
        forced_log_a as *const () as usize,
        &ORIG_FORCED_A,
    );
    iat(
        producer,
        "log4cxx_forced_log_w",
        IMPORT_FORCED_W,
        forced_log_w as *const () as usize,
        &ORIG_FORCED_W,
    );
    iat(
        producer,
        "log4cxx_is_error_enabled",
        IMPORT_IS_ERROR,
        is_error_enabled as *const () as usize,
        &ORIG_IS_ERROR,
    );
    iat(
        producer,
        "log4cxx_is_warn_enabled",
        IMPORT_IS_WARN,
        is_warn_enabled as *const () as usize,
        &ORIG_IS_WARN,
    );
    iat(
        producer,
        "log4cxx_is_debug_enabled",
        IMPORT_IS_DEBUG,
        is_debug_enabled as *const () as usize,
        &ORIG_IS_DEBUG,
    );
    iat(
        producer,
        "log4cxx_is_info_enabled",
        IMPORT_IS_INFO,
        is_info_enabled as *const () as usize,
        &ORIG_IS_INFO,
    );
}

#[cfg(test)]
mod tests {
    use super::super::mem::fake::FakeMemory;
    use super::*;

    #[test]
    fn standard_levels_have_names_and_others_do_not() {
        assert_eq!(level_name(10_000), Some("DEBUG"));
        assert_eq!(level_name(40_000), Some("ERROR"));
        assert_eq!(level_name(i32::MIN), Some("ALL"));
        assert_eq!(level_name(12_345), None);
    }

    #[test]
    fn levels_map_to_telemetry_levels_by_range() {
        assert_eq!(telemetry_level(5_000), "debug");
        assert_eq!(telemetry_level(10_000), "debug");
        assert_eq!(telemetry_level(20_000), "info");
        assert_eq!(telemetry_level(30_000), "warn");
        assert_eq!(telemetry_level(40_000), "error");
        assert_eq!(telemetry_level(50_000), "error");
        assert_eq!(telemetry_level(i32::MIN), "debug");
    }

    /// The `ObjectPtrT<Level>` layout is not certain (vtable pointer then
    /// `Level*`, per `UnrealAppender::append`), so both are accepted; a
    /// candidate is taken only when its `+0xc` is a standard level.
    #[test]
    fn a_level_ptr_resolves_with_or_without_a_vtable_pointer() {
        let mut m = FakeMemory::new();
        // Level object at 0x2000 with WARN at +0xc.
        m.put(0x2000 + LEVEL_INT_OFFSET, &30_000i32.to_le_bytes());
        // With a vtable pointer: [vptr][Level*].
        m.put(0x1000, &0x1234_5678u32.to_le_bytes());
        m.put(0x1004, &0x2000u32.to_le_bytes());
        // Without: [Level*].
        m.put(0x3000, &0x2000u32.to_le_bytes());
        m.put(0x3004, &0x0u32.to_le_bytes());
        let r = m.reader();
        assert_eq!(resolve_level(&r, 0x1000), Some(30_000));
        assert_eq!(resolve_level(&r, 0x3000), Some(30_000));
        drop(r);
        // Garbage: neither candidate points at a standard level.
        m.put(0x4000, &0x9999u32.to_le_bytes());
        m.put(0x4004, &0x9999u32.to_le_bytes());
        assert_eq!(resolve_level(&m.reader(), 0x4000), None);
        assert_eq!(resolve_level(&m.reader(), 0), None);
    }

    #[test]
    fn a_location_reads_file_line_and_method() {
        let mut m = FakeMemory::new();
        m.put_ansi(0x5000, "servconn.cpp");
        m.put_ansi(0x5100, "logOn");
        m.put(0x6000 + LOCATION_LINE_OFFSET, &88i32.to_le_bytes());
        m.put(0x6000 + LOCATION_FILE_OFFSET, &0x5000u32.to_le_bytes());
        m.put(0x6000 + LOCATION_METHOD_OFFSET, &0x5100u32.to_le_bytes());
        let (file, line, method) = read_location(&m.reader(), 0x6000);
        assert_eq!(file.as_deref(), Some("servconn.cpp"));
        assert_eq!(line, Some(88));
        assert_eq!(method.as_deref(), Some("logOn"));
    }

    /// A macro with no location leaves null strings and a zero line: the
    /// file and method are absent, not empty.
    #[test]
    fn an_empty_location_yields_no_file_or_method() {
        let mut m = FakeMemory::new();
        m.put(0x6000, &[0u8; 12]);
        let (file, line, method) = read_location(&m.reader(), 0x6000);
        assert_eq!((file, method), (None, None));
        assert_eq!(line, Some(0));
        assert_eq!(read_location(&m.reader(), 0), (None, None, None));
    }

    /// The lock trace, per the real `SGWDebugLog.log`: one bucket for the
    /// whole family whatever the thread id.
    #[test]
    fn lock_trace_lines_share_a_throttle_bucket() {
        assert_eq!(
            throttle_key(
                "common",
                10_000,
                "Thread id 51428 holds write lock, num readers = 0"
            ),
            throttle_key(
                "common",
                10_000,
                "Thread id 22296 holds write lock, num readers = 0"
            )
        );
        assert_ne!(
            throttle_key("common", 10_000, "inside writeLock"),
            throttle_key("common", 40_000, "inside writeLock")
        );
    }

    #[test]
    fn fields_carry_logger_level_and_location() {
        let e = EventInfo {
            logger: "common".into(),
            level: Some(40_000),
            message: "Error opening static cache archive X.pak".into(),
            truncated: false,
            file: Some("cache.cpp".into()),
            line: Some(12),
            method: None,
        };
        let f = event_fields(&e, false, 0);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("logger"), Some(json!("common")));
        assert_eq!(get("level"), Some(json!("ERROR")));
        assert_eq!(get("level_int"), Some(json!(40_000)));
        assert_eq!(get("file"), Some(json!("cache.cpp")));
        assert_eq!(get("line"), Some(json!(12)));
        assert_eq!(get("method"), None);
        assert_eq!(get("wide"), Some(json!(false)));

        let unknown = EventInfo { level: None, ..e };
        let f = event_fields(&unknown, true, 3);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("level"), Some(json!("UNKNOWN")));
        assert_eq!(get("level_int"), None);
        assert_eq!(get("suppressed"), Some(json!(3)));
        assert_eq!(get("wide"), Some(json!(true)));
    }

    #[test]
    fn the_iat_slots_are_the_import_directorys() {
        // From the QA SGW.exe's import directory (log4cxx.dll).
        assert_eq!(IAT_FORCED_LOG_A, 0x017f_0160);
        assert_eq!(IAT_FORCED_LOG_W, 0x017f_0188);
        assert_eq!(IAT_IS_ERROR_ENABLED, 0x017f_017c);
        assert_eq!(IAT_IS_WARN_ENABLED, 0x017f_01ac);
        assert_eq!(IAT_IS_DEBUG_ENABLED, 0x017f_01c4);
        assert_eq!(IAT_IS_INFO_ENABLED, 0x017f_01d8);
    }
}
