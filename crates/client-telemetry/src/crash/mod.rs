//! Phase 6: crash and exit capture.
//!
//! Tells SigNoz whether a session ended in a crash, a quit or nothing
//! at all, and leaves a small minidump in `Binaries/sessions/` for the
//! launcher's end-of-session bundle to ship.
//!
//! # How SGW.exe crashes (QA build, static evidence)
//!
//! - UE3 wraps `GuardedMain` (`0x00416010`) in `__try/__except` in
//!   `WinMain`, with a filter that calls `CreateMiniDump`
//!   (`0x0041ddb0`); four more engine thread bodies call it the same
//!   way. A fault on those threads never reaches the top-level filter.
//!   `CreateMiniDump` ends in CME's dump writer (`0x00a55a70`), the only
//!   caller of `MiniDumpWriteDump` (through the thunk at `0x012f5906`,
//!   `jmp [0x017f0058]`). It passes the live exception pointers and a
//!   dump type of 0 (normal) or 2 (full memory) and writes under
//!   `Binaries/CrashDumps/` ([crash-dumps.md]).
//! - SGW.exe calls `SetUnhandledExceptionFilter` twice, both CRT code:
//!   `__CxxSetUnhandledExceptionFilter` at start-up (`0x01237fab`,
//!   installs the filter that turns an uncaught C++ exception into
//!   `terminate()`), and `__report_gsfailure` (`0x012382e6`), which
//!   clears it before calling `UnhandledExceptionFilter` itself.
//! - A normal quit returns from `WinMain` into the CRT, which calls
//!   `exit` through SGW.exe's IAT (`0x01237832`). UE3's forced
//!   `appRequestExit` calls `ExitProcess(1)` through the IAT
//!   (`0x004910b9`), and so does a second quit request (`0x004cc125`).
//!
//! # What is hooked
//!
//! Four IAT slots, each swapped only when it holds exactly the address
//! its import resolves to (the same per-slot check the other IAT hooks
//! make), plus our own top-level filter:
//!
//! | Slot | Import | Why |
//! |---|---|---|
//! | `0x017F0058` | `dbghelp!MiniDumpWriteDump` | the game's crash handler: capture, then let it write its own dump |
//! | `0x017EF108` | `KERNEL32!SetUnhandledExceptionFilter` | keep our filter on top, chain to what the game sets |
//! | `0x017EF9A8` | `MSVCR80!exit` | normal quit: `client.exit` |
//! | `0x017EF238` | `KERNEL32!ExitProcess` | forced quit: `client.exit` |
//!
//! Nothing here swallows a crash. The `MiniDumpWriteDump` detour always
//! calls the original with the game's arguments. Our filter returns
//! whatever the filter it displaced returns ([`chain`]). A vectored
//! handler is deliberately not used: it sees every first-chance
//! exception, and UE3 and `lua51.dll` throw C++ exceptions as control
//! flow, so it cannot tell a crash from a caught error.
//!
//! In a Live Research Lab session the lab bridge's own crash tier
//! (`bridge::crash`) replaces the top-level filter after this runs and
//! terminates on a fault; the `MiniDumpWriteDump` detour still sees the
//! game's own crash path.
//!
//! # On a crash
//!
//! Once per process ([`CrashState::on_crash`]): a non-blocking log line,
//! the JSON sidecar, `client.crash` into the queue with an urgent flush
//! request, the minidump (`MiniDumpNormal` with thread info and unloaded
//! modules; a few MB, not the game's full-memory dump), then
//! `client.crash.dump`, then a wait of at most [`CRASH_FLUSH_TIMEOUT`]
//! for the uploader thread to ship both. The event goes before the dump
//! so a dump write that hangs (dbghelp under a held loader lock) cannot
//! also cost the event.
//!
//! [crash-dumps.md]: ../../../../docs/client/crash-dumps.md

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::flush::{wait_delivered, FlushSignal, FlushWait};
use crate::queue::Producer;

pub mod chain;
pub mod report;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod win;

pub use report::{CrashRecord, CrashSource, DumpOutcome, ExitPath, FaultModule};

/// The process's flush handshake, shared by the uploader thread (boot
/// passes it to `run_uploader_with`) and the crash and exit paths.
pub static FLUSH: FlushSignal = FlushSignal::new();

/// How long a crash waits for its events to ship before handing the
/// exception on. The game is already dead; this only delays its own
/// crash dialog.
pub const CRASH_FLUSH_TIMEOUT: Duration = Duration::from_secs(3);
/// How long a quit waits for `client.exit` to ship.
pub const EXIT_FLUSH_TIMEOUT: Duration = Duration::from_secs(2);

/// Where crash evidence goes and how events leave.
#[derive(Clone)]
pub struct CrashContext {
    /// `Binaries/sessions/`, next to `current-session.json`.
    pub sessions_dir: PathBuf,
    pub session_id: String,
    pub started: Instant,
    pub producer: Producer,
    pub flush: &'static FlushSignal,
    pub crash_flush_timeout: Duration,
    pub exit_flush_timeout: Duration,
}

/// What the native side read out of the exception pointers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fault {
    pub code: u32,
    pub flags: u32,
    pub address: u32,
    pub params: [Option<u32>; 2],
    pub module: Option<FaultModule>,
    /// The faulting thread, which is also the thread running the filter.
    pub thread_id: u32,
}

/// Once-per-process crash and exit bookkeeping.
#[derive(Default)]
pub struct CrashState {
    ctx: OnceLock<CrashContext>,
    crashed: AtomicBool,
    exited: AtomicBool,
}

impl CrashState {
    pub const fn new() -> Self {
        Self {
            ctx: OnceLock::new(),
            crashed: AtomicBool::new(false),
            exited: AtomicBool::new(false),
        }
    }

    /// Set once, from the bootstrap thread. Without it the hooks still
    /// chain but record nothing.
    pub fn configure(&self, ctx: CrashContext) {
        let _ = self.ctx.set(ctx);
    }

    pub fn has_crashed(&self) -> bool {
        self.crashed.load(Ordering::Acquire)
    }

    /// Record a crash. Returns `false` (and does nothing) for every crash
    /// after the first, including a fault inside this code: a nested
    /// exception comes back through the same filters and must pass
    /// straight through. `write_dump` writes the minidump to the path it
    /// is given.
    pub fn on_crash(
        &self,
        source: CrashSource,
        fault: Fault,
        game_dump_type: Option<u32>,
        now_ms: i64,
        write_dump: impl FnOnce(&std::path::Path) -> DumpOutcome,
    ) -> bool {
        if self.crashed.swap(true, Ordering::AcqRel) {
            return false;
        }
        let Some(ctx) = self.ctx.get() else {
            crate::log::line_nonblocking(format_args!(
                "CRASH code 0x{:08x} before crash capture was configured",
                fault.code
            ));
            return true;
        };
        let dump_file = report::dump_file_name(&ctx.session_id, now_ms);
        let record = CrashRecord {
            source,
            code: fault.code,
            flags: fault.flags,
            address: fault.address,
            params: fault.params,
            module: fault.module,
            thread_id: fault.thread_id,
            game_dump_type,
            dump_file: dump_file.clone(),
            uptime_ms: ctx.started.elapsed().as_millis() as u64,
            last_seq: ctx.producer.next_seq(),
        };
        crate::log::line_nonblocking(record.log_line());
        let _ = std::fs::write(
            ctx.sessions_dir.join(report::sidecar_file_name(&dump_file)),
            record.sidecar_json(),
        );
        let deadline = Instant::now() + ctx.crash_flush_timeout;
        let crash_seq = ctx.producer.try_emit_seq(record.event());
        if let Some(seq) = crash_seq {
            // The uploader starts shipping while we write the dump.
            ctx.flush.request(seq);
        }

        let outcome = write_dump(&ctx.sessions_dir.join(&dump_file));
        crate::log::line_nonblocking(format_args!("crash dump {dump_file}: {outcome:?}"));
        let last = ctx
            .producer
            .try_emit_seq(report::dump_event(&dump_file, &outcome))
            .or(crash_seq);

        let waited = match last {
            Some(seq) => wait_delivered(
                ctx.flush,
                seq,
                fault.thread_id,
                deadline.saturating_duration_since(Instant::now()),
                || std::thread::sleep(Duration::from_millis(10)),
            ),
            None => FlushWait::TimedOut,
        };
        crate::log::line_nonblocking(format_args!("crash events: {}", waited.as_str()));
        true
    }

    /// Record the process leaving. Returns `false` for every exit after
    /// the first (the CRT's `exit` can end in a hooked `ExitProcess`).
    pub fn on_exit(&self, path: ExitPath, code: u32, current_thread: u32) -> bool {
        if self.exited.swap(true, Ordering::AcqRel) {
            return false;
        }
        let Some(ctx) = self.ctx.get() else {
            return true;
        };
        let after_crash = self.has_crashed();
        let uptime_ms = ctx.started.elapsed().as_millis() as u64;
        crate::log::line_nonblocking(format_args!(
            "exit via {} code {code} after_crash {after_crash}",
            path.as_str()
        ));
        if let Some(seq) =
            ctx.producer
                .try_emit_seq(report::exit_event(path, code, after_crash, uptime_ms))
        {
            let waited = wait_delivered(
                ctx.flush,
                seq,
                current_thread,
                ctx.exit_flush_timeout,
                || std::thread::sleep(Duration::from_millis(10)),
            );
            crate::log::line_nonblocking(format_args!("exit event: {}", waited.as_str()));
        }
        true
    }
}

/// The process's crash state, read by the native detours.
pub static STATE: CrashState = CrashState::new();

/// Record where crash evidence goes. Called by the bootstrap thread once
/// the session is loaded, before the hooks go in.
pub fn configure(sessions_dir: PathBuf, session_id: &str, producer: Producer) {
    STATE.configure(CrashContext {
        sessions_dir,
        session_id: session_id.to_string(),
        started: Instant::now(),
        producer,
        flush: &FLUSH,
        crash_flush_timeout: CRASH_FLUSH_TIMEOUT,
        exit_flush_timeout: EXIT_FLUSH_TIMEOUT,
    });
}

/// Install the crash filter and the four IAT detours. Called from
/// `hooks::install_all`, so only after the fingerprint gate passed and
/// under the install lock. A no-op off the i686 target.
pub fn install(producer: &Producer) {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    win::install(producer);
    #[cfg(not(all(target_os = "windows", target_arch = "x86")))]
    let _ = producer;
}

#[cfg(test)]
mod tests;
