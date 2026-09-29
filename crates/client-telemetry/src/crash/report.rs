//! What a crash or exit looks like on the wire and on disk. Pure: the
//! native side (`win.rs`) fills a [`CrashRecord`] from the exception
//! pointers and hands it here, so every field is host-testable.

use crate::events::{ClientNativeEvent, EventBuilder};

/// Where a crash was seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashSource {
    /// The game's own crash handler was about to write its minidump
    /// (UE3 `__except(CreateMiniDump(...))` on the main and engine
    /// threads, via the `MiniDumpWriteDump` IAT slot).
    GameMinidump,
    /// Our top-level filter: a fault no `__except` frame handled (a
    /// thread UE3 did not start, or an uncaught C++ exception).
    UnhandledFilter,
}

impl CrashSource {
    pub fn as_str(self) -> &'static str {
        match self {
            CrashSource::GameMinidump => "game_minidump",
            CrashSource::UnhandledFilter => "unhandled_filter",
        }
    }
}

/// The module a faulting address falls in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultModule {
    /// File name only (`SGW.exe`, `lua51.dll`), never the full path:
    /// the path carries the player's user name.
    pub name: String,
    pub base: u32,
}

/// Everything captured about one crash, before it is formatted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashRecord {
    pub source: CrashSource,
    pub code: u32,
    pub flags: u32,
    pub address: u32,
    /// `ExceptionInformation[0..2]` when the record has them: for an
    /// access violation, the operation and the address touched.
    pub params: [Option<u32>; 2],
    pub module: Option<FaultModule>,
    pub thread_id: u32,
    /// Dump type the game asked for, when it was the game's handler.
    pub game_dump_type: Option<u32>,
    /// The dump file this crash is written to (planned name; the
    /// write's outcome is a separate event).
    pub dump_file: String,
    pub uptime_ms: u64,
    /// The producer's next `seq` when the crash was seen: every event
    /// of this session below it happened before the crash.
    pub last_seq: u64,
}

pub const EXCEPTION_ACCESS_VIOLATION: u32 = 0xC000_0005;
pub const EXCEPTION_IN_PAGE_ERROR: u32 = 0xC000_0006;
pub const EXCEPTION_CPP: u32 = 0xE06D_7363;

/// Name of a Win32 exception code, for the event. `None` for codes we
/// have no name for (the hex code is always in the event).
pub fn exception_name(code: u32) -> Option<&'static str> {
    Some(match code {
        EXCEPTION_ACCESS_VIOLATION => "access_violation",
        EXCEPTION_IN_PAGE_ERROR => "in_page_error",
        0xC000_001D => "illegal_instruction",
        0xC000_0025 => "noncontinuable_exception",
        0xC000_008C => "array_bounds_exceeded",
        0xC000_008E => "float_divide_by_zero",
        0xC000_0094 => "integer_divide_by_zero",
        0xC000_0095 => "integer_overflow",
        0xC000_0096 => "privileged_instruction",
        0xC000_00FD => "stack_overflow",
        0xC000_0374 => "heap_corruption",
        0xC000_0409 => "stack_buffer_overrun",
        0xC000_0420 => "assertion_failure",
        0x8000_0003 => "breakpoint",
        EXCEPTION_CPP => "cpp_exception",
        _ => return None,
    })
}

/// For an access violation or in-page error, what the faulting
/// instruction tried to do (`ExceptionInformation[0]`).
pub fn access_operation(code: u32, op: Option<u32>) -> Option<&'static str> {
    if code != EXCEPTION_ACCESS_VIOLATION && code != EXCEPTION_IN_PAGE_ERROR {
        return None;
    }
    Some(match op? {
        0 => "read",
        1 => "write",
        8 => "execute",
        _ => "unknown",
    })
}

/// Final path component of a module path, whichever separator it uses.
pub fn module_basename(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// `SGW.exe+0x00123456`, or the bare address when no module owns it
/// (a jump into freed or JIT memory).
pub fn fault_location(address: u32, module: Option<&FaultModule>) -> String {
    match module {
        Some(m) if address >= m.base => format!("{}+0x{:08x}", m.name, address - m.base),
        _ => format!("0x{address:08x}"),
    }
}

/// Keep a session id safe to put in a file name.
pub fn file_safe(id: &str) -> String {
    let s: String = id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(64)
        .collect();
    if s.is_empty() {
        "nosession".into()
    } else {
        s
    }
}

/// `crash-<session>-<ts_ms>.dmp`, written into `Binaries/sessions/`,
/// which the launcher ships in its end-of-session bundle.
pub fn dump_file_name(session_id: &str, ts_ms: i64) -> String {
    format!("crash-{}-{ts_ms}.dmp", file_safe(session_id))
}

/// The JSON-lines sidecar next to the dump: the same fields as
/// `client.crash`, so a crash whose live event never left the machine
/// still reaches SigNoz through the bundle. `.jsonl`, not `.json`: the
/// launcher never uploads `sessions/*.json` (those hold the session
/// token), and this file holds nothing secret.
pub fn sidecar_file_name(dump_file: &str) -> String {
    match dump_file.strip_suffix(".dmp") {
        Some(stem) => format!("{stem}.jsonl"),
        None => format!("{dump_file}.jsonl"),
    }
}

fn hex(v: u32) -> serde_json::Value {
    serde_json::Value::String(format!("0x{v:08x}"))
}

impl CrashRecord {
    /// The fields shared by the `client.crash` event and the sidecar.
    pub fn fields(&self) -> Vec<(&'static str, serde_json::Value)> {
        use serde_json::json;
        let mut f = vec![
            ("source", json!(self.source.as_str())),
            ("exception_code", hex(self.code)),
            ("exception_flags", hex(self.flags)),
            ("exception_address", hex(self.address)),
            (
                "fault",
                json!(fault_location(self.address, self.module.as_ref())),
            ),
            ("thread_id", json!(self.thread_id)),
            // No `session_id`: the server stamps it from the upload token,
            // and the dump file name carries it.
            ("dump_file", json!(self.dump_file)),
            ("uptime_ms", json!(self.uptime_ms)),
            ("last_seq", json!(self.last_seq)),
        ];
        if let Some(name) = exception_name(self.code) {
            f.push(("exception_name", json!(name)));
        }
        if let Some(m) = &self.module {
            f.push(("module", json!(m.name)));
            f.push(("module_base", hex(m.base)));
            f.push(("module_offset", hex(self.address.wrapping_sub(m.base))));
        }
        if let Some(op) = access_operation(self.code, self.params[0]) {
            f.push(("access", json!(op)));
            if let Some(target) = self.params[1] {
                f.push(("access_address", hex(target)));
            }
        }
        if let Some(t) = self.game_dump_type {
            f.push(("game_dump_type", json!(t)));
        }
        f
    }

    /// `client.crash`, level `error`.
    pub fn event(&self) -> EventBuilder {
        self.fields().into_iter().fold(
            ClientNativeEvent::builder("client.crash", "error"),
            |b, (k, v)| b.field(k, v),
        )
    }

    /// The sidecar file's bytes: one JSON object, one line.
    pub fn sidecar_json(&self) -> Vec<u8> {
        let mut map = serde_json::Map::new();
        map.insert("event".into(), "client.crash".into());
        for (k, v) in self.fields() {
            map.insert(k.into(), v);
        }
        let mut out = serde_json::to_vec(&serde_json::Value::Object(map)).unwrap_or_default();
        out.extend_from_slice(b"\r\n");
        out
    }

    /// One line for the local log.
    pub fn log_line(&self) -> String {
        format!(
            "CRASH {} code 0x{:08x} at {} thread {} dump {}",
            self.source.as_str(),
            self.code,
            fault_location(self.address, self.module.as_ref()),
            self.thread_id,
            self.dump_file
        )
    }
}

/// Outcome of writing our minidump.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DumpOutcome {
    Written {
        bytes: u64,
        dump_type: u32,
    },
    /// `MiniDumpWriteDump` failed with this `GetLastError`, or the file
    /// could not be created (`os_error` from the create).
    Failed {
        stage: &'static str,
        os_error: u32,
    },
}

/// `client.crash.dump`: whether the dump named in `client.crash` made it
/// to disk, and how big it is.
pub fn dump_event(dump_file: &str, outcome: &DumpOutcome) -> EventBuilder {
    use serde_json::json;
    let b = ClientNativeEvent::builder(
        "client.crash.dump",
        match outcome {
            DumpOutcome::Written { .. } => "info",
            DumpOutcome::Failed { .. } => "warn",
        },
    )
    .field("dump_file", json!(dump_file));
    match outcome {
        DumpOutcome::Written { bytes, dump_type } => b
            .field("written", json!(true))
            .field("dump_bytes", json!(bytes))
            .field("dump_type", hex(*dump_type)),
        DumpOutcome::Failed { stage, os_error } => b
            .field("written", json!(false))
            .field("reason", json!(stage))
            .field("os_error", json!(os_error)),
    }
}

/// How the process is leaving, for `client.exit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitPath {
    /// The CRT's `exit()`: `WinMain` returned (a normal quit), or game
    /// code called `exit` on a fatal error.
    CrtExit,
    /// `ExitProcess` from game code: UE3's forced `appRequestExit`, or a
    /// second quit request while the first is still running.
    ExitProcess,
}

impl ExitPath {
    pub fn as_str(self) -> &'static str {
        match self {
            ExitPath::CrtExit => "crt_exit",
            ExitPath::ExitProcess => "exit_process",
        }
    }
}

/// `client.exit`. `after_crash` tells a quit apart from the exit the
/// game's crash handler makes after writing its dump; a session with no
/// `client.exit` and no `client.crash` vanished (killed, or a fault
/// nothing caught).
pub fn exit_event(path: ExitPath, code: u32, after_crash: bool, uptime_ms: u64) -> EventBuilder {
    use serde_json::json;
    ClientNativeEvent::builder("client.exit", if after_crash { "warn" } else { "info" })
        .field("path", json!(path.as_str()))
        .field("exit_code", json!(code))
        .field("after_crash", json!(after_crash))
        .field("uptime_ms", json!(uptime_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record() -> CrashRecord {
        CrashRecord {
            source: CrashSource::GameMinidump,
            code: EXCEPTION_ACCESS_VIOLATION,
            flags: 0,
            address: 0x0041_6ec5,
            params: [Some(1), Some(0x0000_007b)],
            module: Some(FaultModule {
                name: "SGW.exe".into(),
                base: 0x0040_0000,
            }),
            thread_id: 4242,
            game_dump_type: Some(2),
            dump_file: "crash-abc-1700000000000.dmp".into(),
            uptime_ms: 1234,
            last_seq: 99,
        }
    }

    #[test]
    fn module_offset_is_address_minus_base() {
        let r = record();
        let ev = r.event();
        assert_eq!(ev.target, "client.crash");
        assert_eq!(ev.level, "error");
        assert_eq!(ev.fields["module"], "SGW.exe");
        assert_eq!(ev.fields["module_base"], "0x00400000");
        assert_eq!(ev.fields["module_offset"], "0x00016ec5");
        assert_eq!(ev.fields["fault"], "SGW.exe+0x00016ec5");
        assert_eq!(ev.fields["exception_code"], "0xc0000005");
        assert_eq!(ev.fields["exception_name"], "access_violation");
        assert_eq!(ev.fields["access"], "write");
        assert_eq!(ev.fields["access_address"], "0x0000007b");
        assert_eq!(ev.fields["source"], "game_minidump");
        assert_eq!(ev.fields["game_dump_type"], 2);
        assert_eq!(ev.fields["last_seq"], 99);
        assert_eq!(ev.fields["thread_id"], 4242);
    }

    #[test]
    fn address_outside_any_module_has_no_offset() {
        let mut r = record();
        r.module = None;
        let ev = r.event();
        assert_eq!(ev.fields["fault"], "0x00416ec5");
        assert!(!ev.fields.contains_key("module"));
        assert!(!ev.fields.contains_key("module_offset"));
    }

    /// A module lookup that returned a base above the address (a stale
    /// handle) must not print a wrapped-around offset as the location.
    #[test]
    fn base_above_address_falls_back_to_bare_address() {
        let m = FaultModule {
            name: "x.dll".into(),
            base: 0x1000_0000,
        };
        assert_eq!(fault_location(0x0040_0000, Some(&m)), "0x00400000");
    }

    #[test]
    fn access_fields_only_for_access_violations() {
        let mut r = record();
        r.code = EXCEPTION_CPP;
        let ev = r.event();
        assert_eq!(ev.fields["exception_name"], "cpp_exception");
        assert!(!ev.fields.contains_key("access"));
        assert!(!ev.fields.contains_key("access_address"));
    }

    #[test]
    fn access_operation_names() {
        let av = EXCEPTION_ACCESS_VIOLATION;
        assert_eq!(access_operation(av, Some(0)), Some("read"));
        assert_eq!(access_operation(av, Some(1)), Some("write"));
        assert_eq!(access_operation(av, Some(8)), Some("execute"));
        assert_eq!(access_operation(av, Some(3)), Some("unknown"));
        assert_eq!(access_operation(av, None), None);
        assert_eq!(access_operation(0xC000_0094, Some(0)), None);
    }

    #[test]
    fn unknown_code_has_no_name_but_keeps_hex() {
        let mut r = record();
        r.code = 0x1234_5678;
        let ev = r.event();
        assert!(!ev.fields.contains_key("exception_name"));
        assert_eq!(ev.fields["exception_code"], "0x12345678");
    }

    #[test]
    fn basename_drops_the_directory_and_user_name() {
        assert_eq!(
            module_basename(r"C:\Users\someone\SGW\Binaries\SGW.exe"),
            "SGW.exe"
        );
        assert_eq!(module_basename("a/b/lua51.dll"), "lua51.dll");
        assert_eq!(module_basename("plain.dll"), "plain.dll");
    }

    #[test]
    fn file_names_are_sanitised_and_paired() {
        let dump = dump_file_name("9f1c-..\\x/y:z", 1_700_000_000_000);
        assert_eq!(dump, "crash-9f1c-xyz-1700000000000.dmp");
        assert_eq!(
            sidecar_file_name(&dump),
            "crash-9f1c-xyz-1700000000000.jsonl"
        );
        assert_eq!(dump_file_name("", 5), "crash-nosession-5.dmp");
    }

    #[test]
    fn sidecar_is_one_json_line_with_the_event_fields() {
        let r = record();
        let bytes = r.sidecar_json();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.ends_with("\r\n"));
        assert_eq!(text.lines().count(), 1);
        let v: serde_json::Value = serde_json::from_str(text.trim_end()).unwrap();
        assert_eq!(v["event"], "client.crash");
        assert_eq!(v["fault"], "SGW.exe+0x00016ec5");
        assert_eq!(v["dump_file"], "crash-abc-1700000000000.dmp");
    }

    #[test]
    fn dump_event_reports_size_or_failure() {
        let ok = dump_event(
            "d.dmp",
            &DumpOutcome::Written {
                bytes: 2048,
                dump_type: 0x1021,
            },
        );
        assert_eq!(ok.target, "client.crash.dump");
        assert_eq!(ok.level, "info");
        assert_eq!(ok.fields["dump_bytes"], 2048);
        assert_eq!(ok.fields["dump_type"], "0x00001021");
        let bad = dump_event(
            "d.dmp",
            &DumpOutcome::Failed {
                stage: "minidump_write",
                os_error: 87,
            },
        );
        assert_eq!(bad.level, "warn");
        assert_eq!(bad.fields["written"], false);
        assert_eq!(bad.fields["reason"], "minidump_write");
        assert_eq!(bad.fields["os_error"], 87);
    }

    #[test]
    fn exit_event_marks_exits_after_a_crash() {
        let quit = exit_event(ExitPath::CrtExit, 0, false, 10);
        assert_eq!(quit.target, "client.exit");
        assert_eq!(quit.level, "info");
        assert_eq!(quit.fields["path"], "crt_exit");
        assert_eq!(quit.fields["after_crash"], false);
        let after = exit_event(ExitPath::ExitProcess, 1, true, 10);
        assert_eq!(after.level, "warn");
        assert_eq!(after.fields["path"], "exit_process");
        assert_eq!(after.fields["exit_code"], 1);
    }

    #[test]
    fn log_line_names_the_fault() {
        assert_eq!(
            record().log_line(),
            "CRASH game_minidump code 0xc0000005 at SGW.exe+0x00016ec5 thread 4242 dump \
             crash-abc-1700000000000.dmp"
        );
    }
}
