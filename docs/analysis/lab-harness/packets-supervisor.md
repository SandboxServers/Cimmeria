# LH expander D: issue #1660 items 12-16 (packets LH-13..LH-17)

Worktree: `WT = <wt>`. All paths below are relative to `WT` unless absolute.

| Packet | Item | Title | Size | Depends on |
|---|---|---|---|---|
| LH-13 | 12 | `server_log_tail`: forwarded client telemetry gets its own ring; `include_client` arg | M | none |
| LH-14 | 13 | Watchdog: IO-progress load grace and a separate budget for load-phase stalls | M | none |
| LH-15 | 13 | Wait for the build lane to go idle before a client launch | S | LH-14 (both edit `supervisor/mod.rs`, different lines; run LH-14 first) |
| LH-16 | 14 + 15 | Keep-awake counts windowless `SGW.exe`; `lab_login` uses a server list's only row | S | none |
| LH-17 | 16 | `lab-driver` agent gets the missing tools | S | all other packets that add a lab tool (run last) |

Packets LH-13, LH-14, LH-16 touch disjoint files and can run in parallel. LH-15 after LH-14.

Common footer for every packet that edits `docs/**/*.md` or `.claude/agents/*.md` (CRLF files): after editing, normalise line endings with

```powershell
$p = '<absolute path of the edited file>'
$t = [IO.File]::ReadAllText($p); $t = $t -replace "(?<!\r)\n", "`r`n"
[IO.File]::WriteAllText($p, $t, (New-Object Text.UTF8Encoding $false))
```

and check `git diff --stat` shows only the lines you meant (a whole-file diff means line endings broke).

---

## Item 12: `server_log_tail` is useless during a lab run

### Root cause (confidence: high for the mechanism, medium-high for which events flood)

`server_log_tail` already has `level`, `target`, `contains` and `since_ms` filters (`crates/lab-mcp/src/tools/logs.rs:14-39`, wired in `crates/lab-mcp/src/tools/mod.rs:219-241`). They run on a snapshot, after the fact. The loss is earlier: `LogBuffer::push` (`crates/admin-api/src/ws/broadcast_layer/mod.rs:128-141`) evicts the oldest entry once the single ring holds `BUFFER_CAPACITY = 500` entries. The telemetry ingest replays every uploaded client event as a server tracing event with targets `client.native` (`crates/admin-api/src/routes/telemetry/replay_native.rs:302,342`) and `launcher.client_log`, `launcher.debug_log`, `launcher.key_dump`, `launcher.session_meta` (`.../replay.rs:187,204,224,236`; `bundle_unzip.rs:232` also uses `launcher.client_log`). A lab client uploads hundreds of those per second, so the 500-slot ring turns over in seconds and a real server line is evicted before anyone filters for it. A read-time filter cannot bring back an evicted line, so the fix must be at ingest: keep forwarded client telemetry out of the ring that holds server lines.

Scope: this is the shared `LogBuffer` of `cimmeria-admin-api`, which also feeds the admin WebSocket history. The change is additive (two rings; `snapshot()` still returns everything, merged by time), so the WS behaviour changes only in that the connect-time history can hold up to 500 + 500 entries instead of 500. The lab control endpoint (`lab-mcp`) is the only reader of the new `snapshot_server()`. No product wire or gameplay change. If the owner objects to touching `admin-api`, the only alternative is a lab-only second subscriber layer in `crates/server`, which is larger; not recommended.

No live probe needed.

### LH-13 packet spec

**ID / title / size:** LH-13, `server_log_tail` keeps server lines when client telemetry floods, M. Depends on: none.

**Files**
1. `crates/admin-api/src/ws/broadcast_layer/mod.rs`: `LogBuffer` struct (line 106), `LogBuffer::new` (117), `push` (128), `snapshot` (144); new const and fn above `LogBuffer`.
2. `crates/admin-api/src/ws/broadcast_layer/tests.rs`: one new test.
3. `crates/lab-mcp/src/state.rs`: `log_snapshot` (line 58-61).
4. `crates/lab-mcp/src/tools/logs.rs`: `LogFilter`, `log_tail`.
5. `crates/lab-mcp/src/tools/mod.rs`: `LogTailArgs` (line 69-85) and `server_log_tail` (216-241).
6. `docs/guides/live-research-lab.md` (CRLF): the `server_log_tail` table row (line 333) and the paragraph at line 373.

**Steps**

1. In `crates/admin-api/src/ws/broadcast_layer/mod.rs`, replace the module doc bullet
   `//! - The ring holds at most [`BUFFER_CAPACITY`] entries and at most` / `//!   [`RING_BYTE_BUDGET`] bytes, evicting the oldest entries first.`
   with:
   ```rust
   //! - Each ring holds at most [`BUFFER_CAPACITY`] entries and at most
   //!   [`RING_BYTE_BUDGET`] bytes, evicting the oldest entries first. Events
   //!   the telemetry ingest replays from an uploading client
   //!   ([`FORWARDED_CLIENT_TARGETS`]) go to a second ring, so a flood of them
   //!   never pushes server lines out of the first.
   ```
2. Insert directly above `/// Ring contents with the running byte total` (line 94):
   ```rust
   /// Tracing targets of the events the telemetry ingest replays from an
   /// uploading client or launcher (`routes/telemetry/replay.rs`,
   /// `replay_native.rs`, `bundle_unzip.rs`). A lab client produces hundreds a
   /// second; they are kept in their own ring (see [`LogBuffer`]).
   pub const FORWARDED_CLIENT_TARGETS: [&str; 5] = [
       "client.native",
       "launcher.client_log",
       "launcher.debug_log",
       "launcher.key_dump",
       "launcher.session_meta",
   ];

   /// Whether `target` is one of [`FORWARDED_CLIENT_TARGETS`].
   pub fn is_forwarded_client_target(target: &str) -> bool {
       FORWARDED_CLIENT_TARGETS.contains(&target)
   }

   ```
3. Replace the `LogBuffer` struct, its `Default` impl stays, and replace `new`, `push` and `snapshot` so the whole block from `/// Thread-safe ring buffer of recent log entries.` through the end of `impl LogBuffer` reads:
   ```rust
   /// Thread-safe ring buffers of recent log entries: one for server events and
   /// one for client events forwarded by the telemetry ingest
   /// ([`FORWARDED_CLIENT_TARGETS`]), each with its own entry and byte caps.
   ///
   /// Shared between the tracing layer (writer) and WebSocket handlers (reader).
   /// Uses a `std::sync::Mutex` (not tokio) because the tracing layer runs in
   /// a synchronous context.
   #[derive(Clone)]
   pub struct LogBuffer {
       inner: Arc<Mutex<Ring>>,
       client: Arc<Mutex<Ring>>,
   }

   impl Default for LogBuffer {
       fn default() -> Self {
           Self::new()
       }
   }

   impl LogBuffer {
       pub fn new() -> Self {
           let empty = || {
               Arc::new(Mutex::new(Ring {
                   entries: VecDeque::new(),
                   bytes: 0,
               }))
           };
           Self {
               inner: empty(),
               client: empty(),
           }
       }

       /// Push an entry into the ring its target belongs to, evicting that
       /// ring's oldest until both the entry count and the byte budget hold.
       fn push(&self, entry: LogEntry) {
           let size = entry.approx_bytes();
           let ring = if is_forwarded_client_target(&entry.target) {
               &self.client
           } else {
               &self.inner
           };
           if let Ok(mut ring) = ring.lock() {
               while !ring.entries.is_empty()
                   && (ring.entries.len() >= BUFFER_CAPACITY
                       || ring.bytes + size > RING_BYTE_BUDGET)
               {
                   if let Some((_, evicted)) = ring.entries.pop_front() {
                       ring.bytes -= evicted;
                   }
               }
               ring.bytes += size;
               ring.entries.push_back((entry, size));
           }
       }

       fn read(ring: &Mutex<Ring>) -> Vec<LogEntry> {
           ring.lock()
               .map(|ring| ring.entries.iter().map(|(e, _)| e.clone()).collect())
               .unwrap_or_default()
       }

       /// Snapshot the server ring only (oldest first): every event except the
       /// forwarded client telemetry. This is what `server_log_tail` reads.
       pub fn snapshot_server(&self) -> Vec<LogEntry> {
           Self::read(&self.inner)
       }

       /// Snapshot all buffered entries, server and forwarded client, merged
       /// by timestamp (oldest first; equal timestamps keep server entries
       /// first).
       pub fn snapshot(&self) -> Vec<LogEntry> {
           let mut all = self.snapshot_server();
           all.extend(Self::read(&self.client));
           all.sort_by_key(|e| e.timestamp_ms);
           all
       }

       /// The server ring's running byte total.
       #[cfg(test)]
       fn total_bytes(&self) -> usize {
           self.inner.lock().map(|ring| ring.bytes).unwrap_or_default()
       }
   }
   ```
   (Keep the `Ring` struct and everything below `impl LogBuffer` unchanged.)
4. In `crates/lab-mcp/src/state.rs` replace
   ```rust
       /// Snapshot of the recent-log ring buffer (oldest first).
       pub fn log_snapshot(&self) -> Vec<cimmeria_admin_api::ws::broadcast_layer::LogEntry> {
           self.log_buffer.snapshot()
   ```
   with
   ```rust
       /// Snapshot of the recent-log ring buffer (oldest first): the server
       /// ring, or with `include_client` also the forwarded client telemetry.
       pub fn log_snapshot(
           &self,
           include_client: bool,
       ) -> Vec<cimmeria_admin_api::ws::broadcast_layer::LogEntry> {
           if include_client {
               self.log_buffer.snapshot()
           } else {
               self.log_buffer.snapshot_server()
           }
   ```
   (the existing closing `}` of the fn stays).
5. In `crates/lab-mcp/src/tools/logs.rs`: add to `LogFilter` after `since_ms`:
   ```rust
       /// Also return the client telemetry the server replays into its log
       /// (targets `client.native`, `launcher.client_log`, ...). Off by
       /// default: it floods the ring during a lab run.
       pub include_client: bool,
   ```
   and change `log_tail` body to `tail_reply(state.log_snapshot(f.include_client), f, limit)`. Update the doc comment of `log_tail`: replace `(see `broadcast_layer::BUFFER_CAPACITY` and` / `` `RING_BYTE_BUDGET`; long messages are truncated), so this is a tail,`` unchanged, and append the sentence `Forwarded client telemetry is in a separate ring, read only with `include_client`.` to the first paragraph.
6. In `crates/lab-mcp/src/tools/mod.rs`: add to `LogTailArgs` after `since_ms`:
   ```rust
       /// Optional: also return the client telemetry the server replays into
       /// its log (targets client.native, launcher.client_log, ...). Default
       /// false.
       #[serde(default)]
       include_client: Option<bool>,
   ```
   In `server_log_tail`: add `"include_client": args.include_client,` to the `audit_args` `json!`, and `include_client: args.include_client.unwrap_or(false),` to the `logs::LogFilter { ... }` literal. Change the `#[tool(description = ...)]` text to:
   `"Return the most recent server log lines, optionally filtered by level, target, message text and since_ms; newest_ms is the buffer's newest timestamp. Forwarded client telemetry (client.native, launcher.client_log, ...) is left out unless include_client is true."`
7. Grep for other `LogFilter {` literals and `log_snapshot(` callers: `Grep pattern "LogFilter \{|log_snapshot\(" path crates/`. The tests in `crates/lab-mcp/src/tools/logs_tests.rs` use `..LogFilter::default()` and need no change; fix any literal that lists the four old fields by adding `..LogFilter::default()`.

**Test** (`crates/admin-api/src/ws/broadcast_layer/tests.rs`, append at the end of the file; `with_layer`, `BUFFER_CAPACITY` and `LogBuffer` are already in scope):
```rust
#[test]
fn forwarded_client_events_do_not_evict_server_lines() {
    let ring = with_layer(|| {
        tracing::info!("server line");
        for i in 0..BUFFER_CAPACITY * 2 {
            tracing::info!(target: "client.native", "native {i}");
        }
        tracing::info!(target: "launcher.client_log", "client log");
    });

    let server = ring.snapshot_server();
    assert_eq!(server.len(), 1, "only the server line is in the server ring");
    assert_eq!(server[0].message, "server line");

    // The client ring is bounded on its own, and snapshot() merges both.
    let all = ring.snapshot();
    assert_eq!(all.len(), BUFFER_CAPACITY + 1);
    assert!(all.iter().any(|e| e.message == "server line"));
    assert!(all.iter().all(|e| e.target != "client.native" || e.message.starts_with("native ")));
}
```
Why it fails when reverted: with one ring, the 1001 client pushes evict "server line" (capacity 500), so `snapshot_server()` is empty and `all.len()` is 500, not 501 (the test also stops compiling without `snapshot_server`, which is the point: the API is the fix).

**Lane checks** (run in this order from `WT`; `pkg` names verified from the Cargo.toml files):
```
pwsh -NoProfile -File WT\tools\build-lane\lane.ps1 cargo fmt -p cimmeria-admin-api -p cimmeria-lab-mcp
pwsh -NoProfile -File WT\tools\build-lane\lane.ps1 cargo clippy -p cimmeria-admin-api -p cimmeria-lab-mcp --all-targets -- -D warnings
pwsh -NoProfile -File WT\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-admin-api broadcast_layer
pwsh -NoProfile -File WT\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab-mcp logs
```
(If `cargo fmt` rejects two `-p`, run it once per package.)

**Docs** (`docs/guides/live-research-lab.md`, CRLF)
- Line 333, the `server_log_tail` table row: in the arguments cell, replace `` `since_ms` (strictly after), `limit` (default 100) `` with `` `since_ms` (strictly after), `include_client` (default false; also return the client telemetry the server replays into its log), `limit` (default 100) ``.
- Line 373 `` `server_log_tail` reads a ring of the last 500 lines (or 2 MiB). Its ``: replace with `` `server_log_tail` reads a ring of the last 500 server lines (or 2 MiB). Client telemetry that the server replays into its log (targets `client.native`, `launcher.client_log`, `launcher.debug_log`, `launcher.key_dump`, `launcher.session_meta`) goes to a second ring of the same size, so a running lab client cannot push server lines out; pass `include_client: true` to read it. Its ``.
- After the line ending `(each poll adds a line to the 500-line ring).` nothing changes.

**Live smoke** (coordinator, lab-server MCP, not the lab-driver): with a lab client in the world and uploading telemetry for at least 30 s, `server_log_tail {limit: 20}` returns server lines (for example `lab.tool_call` or `aoi.*` targets) and none with target `client.native`; `server_log_tail {target: "client.native", include_client: true, limit: 3}` returns 3 client lines. Before the fix the first call returns only `client.native`/`launcher.*` lines.

**Commit subject:** `fix(lab): server_log_tail keeps server lines when client telemetry floods the ring (LH-13)`

---

## Item 13: client start hangs before the window when C: is saturated

### Root cause (confidence: high for why the watchdog kills; medium that IO counters will move; the disk-saturation cause itself is correlated, not proven)

Evidence (`docs/analysis/haiku-lab-uat/ledger.md:77`): main thread parked in `Wait/PageIn` in image/PAK load at 0 ms CPU, C: queue 11-47, watchdog kill at a 115 s stall. Reading the code:

- The load grace only protects a stalled main thread that shows **main-thread CPU** (`stall_grace.rs:101-105` `main_thread_busy`, sampled by `main_thread.rs`). A thread blocked on page-in burns no CPU, so it is "idle", `busy_seen` stays false, and `decide` (`stall_grace.rs:133-149`) kills at five failed polls.
- Before the bridge has answered once, failed polls are ignored for `BOOT_GRACE = 90 s` (`stall_grace.rs:50`, `watchdog.rs:64`). Then five failures at about 6 s each (1 s poll + 5 s dispatch timeout) kill at about 90 + 30 = 120 s. That matches "kills at 115 s".
- `handle_death` (`watchdog.rs:138-149`) always calls `record_crash` (`watchdog.rs:174`), so three such kills hit the cap (`recovery.rs:132-134`, three in ten minutes) and relaunches stop.

So both asked-for fixes are needed: the watchdog must tell "waiting on the disk but progressing" from "hung", and a stall that was only a slow disk must not use up the crash budget. Gating launches on an idle lane (LH-15) lowers the chance of the stall; the Dev Drive move is a user-environment choice (the build lane already seeds one) and is documented, not coded.

Static reading cannot prove that `GetProcessIoCounters.ReadTransferCount` or the process CPU time moves during the page-in wait: memory-mapped page-ins are paging IO and may not be counted, though `ReadFile`-based PAK reads are, and the async loader threads usually spend some CPU. The packet therefore logs the counters on every grace and kill line so the first recurrence settles it, and the kill path degrades to today's behaviour (no worse) when they show nothing.

**Optional live probe LP-13** (coordinator-run, needs a loaded disk, ask the user first because it uses the lab): in one shell run a disk load, `pwsh -NoProfile -Command "1..40 | % { [IO.File]::WriteAllBytes((Join-Path $env:TEMP 'lh13.bin'), (New-Object byte[] 200MB)) }"` while another session runs `lane.ps1 cargo check -p cimmeria-lab`; then `lab_client_start` (via the coordinator's own call, not a Haiku driver). Read `labd.log`: with LH-14 in place a stalled start shows `load grace in effect ... io_read_kb=<n> process_cpu_ms=<n> io_seen=true`. Outcome A (`io_read_kb` large or `process_cpu_ms` > 0 while the main thread is idle): the fix works as designed. Outcome B (both about 0 during the stall): the page-in is invisible to these counters, and the fallback is the follow-up "add `GetProcessMemoryInfo` page-fault count" (needs the `Win32_System_ProcessStatus` windows-sys feature and `cargo hakari generate`); do not add it speculatively.

### LH-14 packet spec

**ID / title / size:** LH-14, Watchdog IO-progress load grace and a separate budget for load-phase stalls, M. Depends on: none.

**Files**
1. `crates/lab/src/supervisor/process_activity.rs` (new).
2. `crates/lab/src/supervisor/mod.rs`: add `pub mod process_activity;` after `pub mod process;` (line 58).
3. `crates/lab/src/supervisor/stall_grace.rs`: constants, `io_grace_from`, `Step`, `StallTracker`.
4. `crates/lab/src/supervisor/stall_grace_tests.rs`: new tests.
5. `crates/lab/src/supervisor/watchdog.rs`: `watchdog_loop`, `handle_death`, `after_death`, `DeathKind`, existing test.
6. `crates/lab/src/supervisor/recovery.rs`: `RecoveryTracker`, new tests.
7. `docs/guides/live-research-lab.md` (CRLF): watchdog section.

**Steps**

1. Create `crates/lab/src/supervisor/process_activity.rs`:
   ```rust
   //! Whole-process activity of the client, read from outside it: bytes read
   //! and CPU time of every thread. The load grace ([`super::stall_grace`])
   //! uses it for a stall the main thread spends waiting on the disk: a client
   //! starting while the drive is saturated by builds sits in page-in waits at
   //! 0 ms of main-thread CPU for minutes (2026-10-12, `docs/analysis/haiku-lab-uat`),
   //! which the main-thread CPU test alone reads as a hang.

   use super::stall_grace::main_thread_busy;

   /// Bytes read per sample interval at or above which the process counts as
   /// making IO progress (256 KiB: far above a hung client's idle reads).
   pub const IO_PROGRESS_BYTES: i64 = 256 * 1024;

   /// What the process did between two samples.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub struct Activity {
       /// Bytes read (all threads) since the previous sample.
       pub read_bytes: i64,
       /// Kernel + user CPU time of all threads since the previous sample, in ms.
       pub cpu_ms: i64,
       /// Wall clock since the previous sample, in ms.
       pub wall_ms: i64,
   }

   /// Whether the interval shows the process working or reading: at least
   /// [`IO_PROGRESS_BYTES`] read, or the busy share of CPU
   /// ([`main_thread_busy`]'s threshold, applied to all threads).
   pub fn io_progress(a: Activity) -> bool {
       a.wall_ms > 0 && (a.read_bytes >= IO_PROGRESS_BYTES || main_thread_busy(a.cpu_ms, a.wall_ms))
   }

   /// The sampler for one launch: remembers the previous counters.
   #[derive(Debug, Default)]
   pub struct ProcessActivity {
       /// `(read_bytes, cpu_ms, wall_ms)` of the previous sample.
       prev: Option<(i64, i64, i64)>,
   }

   impl ProcessActivity {
       /// Sample `pid` at `now_ms`. `None` on the first sample or when the
       /// process cannot be read (the caller then treats the stall as no
       /// progress, the behaviour before this existed).
       pub fn sample(&mut self, pid: u32, now_ms: i64) -> Option<Activity> {
           let Some((read, cpu)) = read_counters(pid) else {
               self.prev = None;
               return None;
           };
           let out = self.prev.map(|(prev_read, prev_cpu, prev_wall)| Activity {
               read_bytes: read - prev_read,
               cpu_ms: cpu - prev_cpu,
               wall_ms: now_ms - prev_wall,
           });
           self.prev = Some((read, cpu, now_ms));
           out
       }
   }

   #[cfg(not(windows))]
   fn read_counters(_pid: u32) -> Option<(i64, i64)> {
       None
   }

   /// `(bytes read, cpu ms)` of the whole process.
   #[cfg(windows)]
   fn read_counters(pid: u32) -> Option<(i64, i64)> {
       use windows_sys::Win32::Foundation::{CloseHandle, FALSE, FILETIME};
       use windows_sys::Win32::System::Threading::{
           GetProcessIoCounters, GetProcessTimes, OpenProcess, IO_COUNTERS,
           PROCESS_QUERY_INFORMATION,
       };

       let ticks = |ft: &FILETIME| (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime);
       // SAFETY: OpenProcess/GetProcessIoCounters/GetProcessTimes/CloseHandle
       // with valid out-params; the handle is closed on every path.
       unsafe {
           let h = OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, pid);
           if h.is_null() {
               return None;
           }
           let mut io: IO_COUNTERS = std::mem::zeroed();
           let zero = FILETIME {
               dwLowDateTime: 0,
               dwHighDateTime: 0,
           };
           let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
           let io_ok = GetProcessIoCounters(h, &mut io) != 0;
           let times_ok = GetProcessTimes(h, &mut created, &mut exited, &mut kernel, &mut user) != 0;
           CloseHandle(h);
           (io_ok && times_ok).then(|| {
               (
                   io.ReadTransferCount as i64,
                   ((ticks(&kernel) + ticks(&user)) / 10_000) as i64,
               )
           })
       }
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       /// 256 KiB read in the interval is progress with an idle CPU; 2 % CPU
       /// is progress with no reads; a hung client's trickle is neither.
       #[test]
       fn progress_is_reads_or_cpu_not_a_trickle() {
           let a = |read_bytes, cpu_ms| Activity {
               read_bytes,
               cpu_ms,
               wall_ms: 6_000,
           };
           assert!(io_progress(a(IO_PROGRESS_BYTES, 0)));
           assert!(io_progress(a(0, 120)));
           assert!(!io_progress(a(IO_PROGRESS_BYTES - 1, 119)));
           assert!(!io_progress(a(0, 0)));
           assert!(!io_progress(Activity {
               read_bytes: 10 << 20,
               cpu_ms: 1_000,
               wall_ms: 0
           }));
       }

       /// The sampler reads this very process: the first sample has no
       /// interval, the second does. Windows only (a no-op elsewhere).
       #[cfg(windows)]
       #[test]
       fn the_sampler_reads_a_live_process() {
           let pid = std::process::id();
           let mut s = ProcessActivity::default();
           assert_eq!(s.sample(pid, 0), None);
           let a = s.sample(pid, 1_000).expect("own process is readable");
           assert_eq!(a.wall_ms, 1_000);
           assert!(a.read_bytes >= 0 && a.cpu_ms >= 0);
       }

       #[test]
       fn a_missing_process_has_no_sample() {
           let mut s = ProcessActivity::default();
           assert_eq!(s.sample(0xFFFF_FFF1, 0), None);
           assert_eq!(s.sample(0xFFFF_FFF1, 1_000), None);
       }
   }
   ```
2. `supervisor/mod.rs`: after the line `pub mod process;` add `pub mod process_activity;`.
3. `stall_grace.rs`:
   a. After `pub const MAX_LOAD_GRACE ...;` (line 45) insert:
   ```rust
   /// Longest stall tolerated while the process is making IO progress
   /// ([`super::process_activity`]): a client starting on a drive saturated by
   /// builds. Replaces [`LOAD_GRACE`] for a stall that showed IO or CPU
   /// progress; never shorter than it.
   pub const IO_LOAD_GRACE: Duration = Duration::from_secs(300);

   /// Overrides [`IO_LOAD_GRACE`], in whole seconds (`0` or less than the load
   /// grace means the load grace applies; at most [`MAX_LOAD_GRACE`]).
   pub const IO_GRACE_ENV: &str = "CIMMERIA_LAB_IO_GRACE_SECS";
   ```
   b. After `load_grace_from` (ends line 99) insert:
   ```rust
   /// The IO-progress grace from the raw [`IO_GRACE_ENV`] value. Unset, empty
   /// or unparseable falls back to [`IO_LOAD_GRACE`]; clamped like
   /// [`load_grace_from`].
   pub fn io_grace_from(raw: Option<&str>) -> Duration {
       raw.map(str::trim)
           .and_then(|s| s.parse::<u64>().ok())
           .map(|secs| Duration::from_secs(secs.min(MAX_LOAD_GRACE.as_secs())))
           .unwrap_or(IO_LOAD_GRACE)
   }
   ```
   c. In `struct Step` (line 163-170) add after `busy_seen`:
   ```rust
       /// Some sample inside the current stall showed IO or CPU progress of the
       /// whole process (the longer [`IO_LOAD_GRACE`] applies).
       pub io_seen: bool,
   ```
   d. In `struct StallTracker` add fields after `busy_seen: bool,`:
   ```rust
       /// Grace once the stall has shown process-wide progress.
       io_grace: Duration,
       io_seen: bool,
   ```
   e. In `StallTracker::new` add `io_grace: grace,` and `io_seen: false,` to the literal. Then add after `new`:
   ```rust
       /// Use `io_grace` instead of the load grace for a stall that has shown
       /// IO or CPU progress (never below the load grace).
       pub fn with_io_grace(mut self, io_grace: Duration) -> Self {
           self.io_grace = io_grace.max(self.grace);
           self
       }
   ```
   f. Replace the `step` method (line 201-245) with two methods. The existing doc comment and signature stay on `step`, whose body becomes `self.step_with_io(hb, sample, false, now_ms, stalled_ms)`. Add:
   ```rust
       /// [`Self::step`] plus `io_progress`: this interval showed the whole
       /// process reading or working ([`super::process_activity::io_progress`]).
       /// Like main-thread busy it is sticky for the stall and ignores the
       /// first two samples; it also lengthens the grace to `io_grace`.
       pub fn step_with_io(
           &mut self,
           hb: Heartbeat,
           sample: Option<(i64, i64)>,
           io_progress: bool,
           now_ms: i64,
           stalled_ms: i64,
       ) -> Step {
           let poll = match hb {
               Heartbeat::Answered { stale } => {
                   self.fails = 0;
                   Poll::Answered { stale }
               }
               Heartbeat::Failed { ms_since_last_ok } => {
                   self.fails = next_fail_count(self.fails, ms_since_last_ok, BUSY_GRACE_MS);
                   Poll::Failed { fails: self.fails }
               }
           };

           // A Tick advance starts a new stall clock, and busy is per stall.
           let start = now_ms - stalled_ms;
           if self.stall_start_ms != Some(start) {
               self.stall_start_ms = Some(start);
               self.polls_in_stall = 0;
               self.busy_seen = false;
               self.io_seen = false;
           }
           let busy_now = sample.is_some_and(|(cpu, wall)| main_thread_busy(cpu, wall));
           // The first two samples of a stall cover frames the thread still
           // ticked: the advancing poll's own, and the next one, which spans
           // the stall's onset. Only samples wholly inside the stall count.
           if self.polls_in_stall >= 2 && busy_now {
               self.busy_seen = true;
           }
           if self.polls_in_stall >= 2 && io_progress {
               self.busy_seen = true;
               self.io_seen = true;
           }
           self.polls_in_stall = self.polls_in_stall.saturating_add(1);

           let grace = if self.io_seen {
               self.io_grace
           } else {
               self.grace
           };
           Step {
               verdict: decide(poll, self.max_fails, self.busy_seen, stalled_ms, grace),
               poll,
               busy_now,
               busy_seen: self.busy_seen,
               io_seen: self.io_seen,
           }
       }
   ```
   (The body is the old `step` body with the three marked additions: `io_seen` reset, the `io_progress` block, and the grace choice.)
4. `stall_grace_tests.rs`: append (the helpers `tracker`, `ticked`, `POLL`, `IDLE`, `BUSY`, `FAILS`, `LOAD_GRACE` already exist in this file):
   ```rust
   // --- IO-progress grace (LH-14) ---------------------------------------

   fn io_tracker() -> StallTracker {
       StallTracker::new(FAILS, LOAD_GRACE).with_io_grace(IO_LOAD_GRACE)
   }

   /// The `n`th failed poll of a stall that began at `start`, with an idle
   /// main thread and `io` progress of the whole process.
   fn failed_io(t: &mut StallTracker, start: i64, n: i64, io: bool) -> Step {
       let now = start + n * POLL;
       let hb = Heartbeat::Failed {
           ms_since_last_ok: Some(now - start),
       };
       t.step_with_io(hb, IDLE, io, now, now - start)
   }

   /// Live regression (2026-10-12): a client starting on a drive saturated by
   /// builds sat in page-in waits at 0 ms of main-thread CPU and was killed
   /// at 115 s. A stall whose process keeps reading waits past the 120 s load
   /// grace, up to the IO grace.
   #[test]
   fn an_io_bound_stall_waits_past_the_cpu_grace() {
       let mut t = io_tracker();
       ticked(&mut t, 0);
       // 30 polls = 180 s: past LOAD_GRACE (120 s), inside IO_LOAD_GRACE.
       let mut last = None;
       for n in 1..=30 {
           last = Some(failed_io(&mut t, 0, n, true));
       }
       let s = last.unwrap();
       assert_eq!(s.verdict, Verdict::Grace { stalled_ms: 180_000 });
       assert!(s.io_seen && s.busy_seen);
   }

   /// IO progress is a cap too: at the IO grace the stall is killed, and the
   /// kill says it was loading.
   #[test]
   fn an_io_bound_stall_dies_at_the_io_grace() {
       let mut t = io_tracker();
       ticked(&mut t, 0);
       let mut last = None;
       for n in 1..=51 {
           last = Some(failed_io(&mut t, 0, n, true));
       }
       assert_eq!(
           last.unwrap().verdict,
           Verdict::Kill {
               loading: true,
               stalled_ms: 306_000
           }
       );
   }

   /// No reads and no CPU: a hung client dies at five failures as before,
   /// whatever the IO grace is.
   #[test]
   fn a_stall_with_no_progress_still_dies_at_five_failures() {
       let mut t = io_tracker();
       ticked(&mut t, 0);
       let mut last = None;
       for n in 1..=5 {
           last = Some(failed_io(&mut t, 0, n, false));
       }
       assert_eq!(
           last.unwrap().verdict,
           Verdict::Kill {
               loading: false,
               stalled_ms: 30_000
           }
       );
   }

   /// Main-thread CPU alone keeps the 120 s cap: the IO grace is only for a
   /// stall that showed process-wide progress.
   #[test]
   fn a_cpu_busy_stall_without_io_keeps_the_load_grace() {
       let mut t = io_tracker();
       ticked(&mut t, 0);
       let v = stall(&mut t, 0, 21, |_| BUSY); // 126 s
       assert_eq!(
           v[20],
           Verdict::Kill {
               loading: true,
               stalled_ms: 126_000
           }
       );
   }

   #[test]
   fn the_io_env_override_parses_seconds() {
       assert_eq!(io_grace_from(None), IO_LOAD_GRACE);
       assert_eq!(io_grace_from(Some("abc")), IO_LOAD_GRACE);
       assert_eq!(io_grace_from(Some(" 600 ")), Duration::from_secs(600));
       assert_eq!(io_grace_from(Some("99999")), MAX_LOAD_GRACE);
       // Never below the load grace once applied.
       let t = StallTracker::new(FAILS, LOAD_GRACE).with_io_grace(Duration::from_secs(10));
       assert_eq!(t.io_grace, LOAD_GRACE);
   }
   ```
   Why they fail when reverted: without `step_with_io` the IO flag cannot reach the tracker (compile failure); if the tracker ignores it, `an_io_bound_stall_waits_past_the_cpu_grace` sees `Kill{loading:false}` at 30 s, the old behaviour.
5. `recovery.rs`: in `RecoveryTracker` (line 125-129) add a field `stalls: VecDeque<i64>,`; in `new` add `stalls: VecDeque::new(),`. Add above the struct:
   ```rust
   /// Load-phase stalls (a start the disk starved, killed after the IO grace)
   /// have their own, looser cap: they say nothing about a crashing probe, but
   /// a relaunch loop into a saturated disk must still end.
   pub const MAX_LOAD_STALLS: usize = 6;
   /// The window for [`MAX_LOAD_STALLS`].
   pub const LOAD_STALL_WINDOW_MS: i64 = 60 * 60 * 1000;
   ```
   Add the method after `record_crash`:
   ```rust
       /// Record a load-phase stall at `now_ms`. It does not count toward the
       /// crash cap.
       pub fn record_load_stall(&mut self, now_ms: i64) {
           self.stalls.push_back(now_ms);
           self.prune(now_ms);
       }
   ```
   Change `should_relaunch` body to `self.prune(now_ms); self.crashes.len() < self.max && self.stalls.len() < MAX_LOAD_STALLS`, and `prune` to also drop stalls:
   ```rust
       fn prune(&mut self, now_ms: i64) {
           let cutoff = now_ms - self.window_ms;
           while self.crashes.front().is_some_and(|&t| t < cutoff) {
               self.crashes.pop_front();
           }
           let stall_cutoff = now_ms - LOAD_STALL_WINDOW_MS;
           while self.stalls.front().is_some_and(|&t| t < stall_cutoff) {
               self.stalls.pop_front();
           }
       }
   ```
   Append a test module only if none exists; otherwise add to it (check `Grep "#\[cfg(test)\]" recovery.rs`; the tests for `RecoveryTracker` live in this file or `recovery_tests.rs`, add beside them):
   ```rust
   #[test]
   fn load_stalls_do_not_use_the_crash_budget() {
       let mut r = RecoveryTracker::new_default();
       for i in 0..5 {
           r.record_load_stall(i * 1_000);
       }
       assert!(r.should_relaunch(5_000), "five stalls: still allowed");
       assert_eq!(r.recent_crash_count(5_000), 0);
       r.record_load_stall(6_000);
       assert!(!r.should_relaunch(6_000), "six stalls in an hour end the loop");
       // An hour later they have aged out.
       assert!(r.should_relaunch(6_000 + LOAD_STALL_WINDOW_MS + 1));
       let mut c = RecoveryTracker::new_default();
       for i in 0..3 {
           c.record_crash(i * 1_000);
       }
       assert!(!c.should_relaunch(3_000), "three crashes still stop it");
   }
   ```
6. `watchdog.rs`:
   a. imports: add `use super::process_activity::{self, ProcessActivity};` after `use super::main_thread::MainThreadCpu;`.
   b. In `watchdog_loop`, after the `let grace = ...;` statement add:
   ```rust
        let io_grace = stall_grace::io_grace_from(
            std::env::var(stall_grace::IO_GRACE_ENV).ok().as_deref(),
        );
        let mut activity = ProcessActivity::default();
   ```
   and change `let mut tracker = StallTracker::new(MAX_HEARTBEAT_FAILS, grace);` to `let mut tracker = StallTracker::new(MAX_HEARTBEAT_FAILS, grace).with_io_grace(io_grace);`.
   c. Replace the lines
   ```rust
            let (cpu_ms, wall_ms) = sample.unwrap_or((0, 0));
            let stalled_ms = self.state.lock().await.watchdog.stalled_ms(ts, started_ms);
            let step = tracker.step(hb, sample, ts, stalled_ms);
   ```
   with
   ```rust
            let (cpu_ms, wall_ms) = sample.unwrap_or((0, 0));
            let act = activity.sample(my_pid, ts);
            let io_progress = act.is_some_and(process_activity::io_progress);
            let io_read_kb = act.map_or(0, |a| a.read_bytes / 1024);
            let process_cpu_ms = act.map_or(0, |a| a.cpu_ms);
            let stalled_ms = self.state.lock().await.watchdog.stalled_ms(ts, started_ms);
            let step = tracker.step_with_io(hb, sample, io_progress, ts, stalled_ms);
   ```
   d. In both `tracing::warn!` calls (`Verdict::Grace` and `Verdict::Kill`) add the fields after `interval_ms = wall_ms,`:
   ```rust
                        io_read_kb,
                        process_cpu_ms,
                        io_seen = step.io_seen,
   ```
   and in the `Grace` message keep the text. In the `Kill` arm replace `self.handle_death(my_pid).await;` with
   ```rust
                    let kind = if loading && step.io_seen {
                        DeathKind::LoadStall
                    } else {
                        DeathKind::Crash
                    };
                    self.handle_death(my_pid, kind).await;
   ```
   and the `client process gone` call with `self.handle_death(my_pid, DeathKind::Crash).await;`.
   e. `handle_death(&self, dead_pid: u32, kind: DeathKind)` calls `self.after_death(dead_pid, kind)`. `after_death(&self, dead_pid: u32, kind: DeathKind)`: replace `st.recovery.record_crash(now_ms());` with
   ```rust
            match kind {
                DeathKind::Crash => st.recovery.record_crash(now_ms()),
                DeathKind::LoadStall => st.recovery.record_load_stall(now_ms()),
            }
   ```
   and change the cap log text to `"recovery cap reached (3 crashes / 10 min, or 6 load stalls / 1 h); not relaunching"`.
   f. Add above `enum AfterDeath`:
   ```rust
   /// Why the client was terminated, for the recovery budget.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   enum DeathKind {
       /// Gone, or hung with no progress: counts toward three per ten minutes.
       Crash,
       /// Killed after the IO grace while still reading: a starved disk, not a
       /// crash, so it has its own budget (`recovery::MAX_LOAD_STALLS`).
       LoadStall,
   }
   ```
   g. Existing test: change `sup.after_death(NO_SUCH_PID).await` to `sup.after_death(NO_SUCH_PID, DeathKind::Crash).await`. Add the test:
   ```rust
    /// Regression guard (2026-10-12): three load-phase stalls used up the
    /// crash budget and stopped relaunches. A stall is its own kind; three of
    /// them leave all three crash slots free.
    #[tokio::test]
    async fn load_stalls_do_not_spend_the_crash_budget() {
        let book = Arc::new(LeaseBook::default());
        let sup = fake_bridge::supervisor(Arc::new(|_, _| Ok(serde_json::json!({}))))
            .await
            .with_leases(book.clone());
        book.acquire(AcquireRequest {
            owner: "test".into(),
            purpose: "watchdog".into(),
            ..Default::default()
        })
        .unwrap();
        for _ in 0..3 {
            assert_eq!(
                sup.after_death(NO_SUCH_PID, DeathKind::LoadStall).await,
                AfterDeath::Relaunch
            );
        }
        for _ in 0..2 {
            assert_eq!(
                sup.after_death(NO_SUCH_PID, DeathKind::Crash).await,
                AfterDeath::Relaunch
            );
        }
        assert_eq!(
            sup.after_death(NO_SUCH_PID, DeathKind::Crash).await,
            AfterDeath::CapReached,
            "the third real crash still stops it"
        );
    }
   ```
   Fails when reverted: if a stall is recorded as a crash, the third stall already returns `CapReached`.
7. Check file sizes stay under 700 lines (`stall_grace_tests.rs` ends near 430, `watchdog.rs` near 370).

**Lane checks**
```
pwsh -NoProfile -File WT\tools\build-lane\lane.ps1 cargo fmt -p cimmeria-lab
pwsh -NoProfile -File WT\tools\build-lane\lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
pwsh -NoProfile -File WT\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab stall_grace
pwsh -NoProfile -File WT\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab process_activity
pwsh -NoProfile -File WT\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab recovery
pwsh -NoProfile -File WT\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab watchdog
```
No dependency or feature change (`GetProcessIoCounters`, `GetProcessTimes`, `OpenProcess`, `IO_COUNTERS` are in `Win32_System_Threading`, `FILETIME` in `Win32_Foundation`, both already enabled), so no `cargo hakari` step.

**Docs** (`docs/guides/live-research-lab.md`, CRLF)
- After the line starting `A spinning main thread also reads as busy, and so does a deadlock` insert a paragraph:
  `A client starting while the drive is saturated (concurrent builds, sccache writes) can sit in page-in waits at 0 ms of main-thread CPU for minutes. So the watchdog also samples the whole process (`supervisor::process_activity`): at least 256 KiB read, or 2 % CPU across all threads, in a sample interval counts as progress, sticky for the stall like busy. A stall that showed progress waits up to the **IO grace** (300 s from the last Tick advance, or from the launch for a client whose window never came) instead of 120 s. `CIMMERIA_LAB_IO_GRACE_SECS` changes it (`labd.env`; at most 3600; never below the load grace). A stall with no reads and no CPU still dies at the old time. The grace and kill lines carry `io_read_kb`, `process_cpu_ms` and `io_seen`.`
- Replace the sentence in the "After a kill" paragraph `and at most three crashes in ten minutes (`recovery cap reached (3 crashes / 10 min); not relaunching`).` with `and at most three crashes in ten minutes (`recovery cap reached (3 crashes / 10 min, or 6 load stalls / 1 h); not relaunching`). A kill that came after the IO grace while the process was still reading (a starved disk, `load_in_progress=true` with `io_seen=true`) counts against a separate budget of six per hour, so slow starts under build load do not use up the crash budget. To avoid them, start lab clients when the build lane is idle (see below) or keep the client install on the Dev Drive.`
- In the `labd.log` table, row `client main thread busy past the load grace; terminating`: append `; with `io_seen=true` the cap was the IO grace`.

**Live smoke:** `lab_ensure_in_world {stop_at: "running"}` on an idle machine returns the window and bridge up as before (no regression); `labd.log` has no `load grace in effect` line. (The saturated-disk case is LP-13 above; do not stage it as a Haiku step.)

**Commit subject:** `fix(lab): watchdog waits while the client is reading; load-phase stalls keep their own budget (LH-14)`

---

### LH-15 packet spec

**ID / title / size:** LH-15, Wait for the build lane to go idle before a client launch, S. Depends on: LH-14 (both edit `supervisor/mod.rs`; no overlapping lines).

**Design:** `tools/build-lane/lane-slots.ps1` documents the protocol: a held slot is the directory `%LOCALAPPDATA%\cimmeria-build\lane\slot.N` (or `$LANE_ROOT\lane`), holding `what`, and `winpid` (lane.ps1) or `pid` (an MSYS pid from lane.sh). No `slot.*` directory means the lane is idle. Before `launch_client` (so `lab_client_start`, `lab_client_restart` and the watchdog relaunch) the supervisor waits up to 90 s (`CIMMERIA_LAB_LANE_WAIT_SECS`, `0` off, at most 600) for the lane to idle, then launches anyway with a warning. It never blocks forever and never fails a launch. A slot whose `winpid` is dead is ignored; a slot with only an MSYS `pid` is counted busy (as `lane.sh` does), so a stale one costs one bounded wait. Default of 90 s is a recommendation: a longer wait risks the MCP client's own timeout on `lab_client_start`; builds that outlast it just get a launch with the warning, as today.

**Files**
1. `crates/lab/src/supervisor/lane_idle.rs` (new).
2. `crates/lab/src/supervisor/mod.rs`: add `pub mod lane_idle;` after `pub mod keys;` (line 50); in `launch_client` (line 403-405) call it.
3. `docs/guides/live-research-lab.md` (CRLF).

**Steps**

1. Create `crates/lab/src/supervisor/lane_idle.rs`:
   ```rust
   //! Wait for the build lane to go idle before a client launch.
   //!
   //! The client's startup reads many GB of images and PAKs. With concurrent
   //! cargo jobs and sccache saturating the drive it sat in page-in waits
   //! until the watchdog killed it (2026-10-12, three times out of three
   //! between 01:57Z and 02:28Z), and started in about 2 s once the builds
   //! drained. The lane's slot protocol is `tools/build-lane/lane-slots.ps1`:
   //! a held slot is the directory `slot.N` under `<LANE_ROOT>/lane`.

   use std::path::{Path, PathBuf};
   use std::time::Duration;

   use tokio::time::Instant;

   use super::process;

   /// How long a launch waits for the lane to go idle.
   pub const LANE_WAIT: Duration = Duration::from_secs(90);

   /// Overrides [`LANE_WAIT`], in whole seconds; `0` turns the wait off.
   pub const LANE_WAIT_ENV: &str = "CIMMERIA_LAB_LANE_WAIT_SECS";

   /// The longest wait [`LANE_WAIT_ENV`] can set.
   pub const MAX_LANE_WAIT: Duration = Duration::from_secs(600);

   /// How often the wait looks at the lane.
   const POLL: Duration = Duration::from_secs(5);

   /// The wait from the raw [`LANE_WAIT_ENV`] value: unset, empty or
   /// unparseable is [`LANE_WAIT`]; clamped to [`MAX_LANE_WAIT`].
   pub fn lane_wait_from(raw: Option<&str>) -> Duration {
       raw.map(str::trim)
           .and_then(|s| s.parse::<u64>().ok())
           .map(|secs| Duration::from_secs(secs.min(MAX_LANE_WAIT.as_secs())))
           .unwrap_or(LANE_WAIT)
   }

   /// `<lane root>/lane`, the way `lane.ps1` derives it: `LANE_ROOT`, else
   /// `%LOCALAPPDATA%\cimmeria-build`.
   pub fn lane_dir(lane_root: Option<&str>, local_app_data: Option<&str>) -> Option<PathBuf> {
       let root = lane_root
           .filter(|s| !s.is_empty())
           .map(PathBuf::from)
           .or_else(|| {
               local_app_data
                   .filter(|s| !s.is_empty())
                   .map(|d| Path::new(d).join("cimmeria-build"))
           })?;
       Some(root.join("lane"))
   }

   /// The held slots of the lane at `dir`, as `slot.N [what]`, sorted. A slot
   /// whose `winpid` is dead (`is_alive` false) is a leftover and is skipped;
   /// one with only an MSYS `pid` counts as held.
   pub fn busy_slots(dir: &Path, is_alive: impl Fn(u32) -> bool) -> Vec<String> {
       let Ok(entries) = std::fs::read_dir(dir) else {
           return Vec::new();
       };
       let mut busy = Vec::new();
       for e in entries.flatten() {
           let name = e.file_name().to_string_lossy().into_owned();
           if !name.starts_with("slot.") || !e.path().is_dir() {
               continue;
           }
           let read = |f: &str| {
               std::fs::read_to_string(e.path().join(f))
                   .unwrap_or_default()
                   .trim()
                   .to_string()
           };
           if read("winpid").parse::<u32>().is_ok_and(|p| !is_alive(p)) {
               continue;
           }
           busy.push(format!("{name} [{}]", read("what")));
       }
       busy.sort();
       busy
   }

   /// What a wait found.
   #[derive(Debug, Clone, PartialEq, Eq)]
   pub struct IdleWait {
       pub waited: Duration,
       /// The slots still held when the wait ended; empty means idle.
       pub busy: Vec<String>,
   }

   /// Poll the lane at `dir` every `poll` until no slot is held or `max`
   /// has passed.
   pub async fn wait_for_idle(
       dir: &Path,
       max: Duration,
       poll: Duration,
       is_alive: impl Fn(u32) -> bool,
   ) -> IdleWait {
       let start = Instant::now();
       loop {
           let busy = busy_slots(dir, &is_alive);
           let waited = start.elapsed();
           if busy.is_empty() || waited >= max {
               return IdleWait { waited, busy };
           }
           tokio::time::sleep(poll).await;
       }
   }

   /// Called before every client launch: wait (bounded) for the build lane to
   /// go idle, then launch regardless. Never fails.
   pub async fn wait_before_launch(cause: &'static str) {
       let max = lane_wait_from(std::env::var(LANE_WAIT_ENV).ok().as_deref());
       if max.is_zero() {
           return;
       }
       let Some(dir) = lane_dir(
           std::env::var("LANE_ROOT").ok().as_deref(),
           std::env::var("LOCALAPPDATA").ok().as_deref(),
       ) else {
           return;
       };
       let w = wait_for_idle(&dir, max, POLL, process::is_alive).await;
       if w.busy.is_empty() {
           if !w.waited.is_zero() {
               tracing::info!(
                   target: "lab.client",
                   event = "lane_idle",
                   cause,
                   waited_ms = w.waited.as_millis() as u64,
                   "build lane idle; launching"
               );
           }
       } else {
           tracing::warn!(
               target: "lab.client",
               event = "lane_busy_launch",
               cause,
               waited_ms = w.waited.as_millis() as u64,
               busy = %w.busy.join(" "),
               "build lane still busy after the wait; launching anyway (a slow start may be killed by the watchdog)"
           );
       }
   }
   ```
2. `mod.rs`: add `pub mod lane_idle;` after `pub mod keys;`. In `launch_client`, directly after the `.map_err(|e| format!("display check: {e}"))??;` line (405) add:
   ```rust
        // Starts on a saturated drive stall in page-in; give builds a bounded
        // chance to finish first (LH-15).
        lane_idle::wait_before_launch(cause).await;
   ```
3. Tests in the same file (append):
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       fn slot(dir: &Path, name: &str, files: &[(&str, &str)]) {
           let d = dir.join(name);
           std::fs::create_dir_all(&d).unwrap();
           for (f, c) in files {
               std::fs::write(d.join(f), c).unwrap();
           }
       }

       #[test]
       fn only_live_slots_count_as_busy() {
           let t = tempfile::tempdir().unwrap();
           slot(t.path(), "slot.1", &[("winpid", "1111"), ("what", "10:00 wt :: cargo check")]);
           slot(t.path(), "slot.2", &[("winpid", "2222"), ("what", "dead job")]);
           slot(t.path(), "slot.3", &[("pid", "77"), ("what", "bash lane")]);
           slot(t.path(), ".stage-9-abc", &[("what", "staging")]);
           std::fs::write(t.path().join("SLOTS"), "4").unwrap();
           let busy = busy_slots(t.path(), |p| p == 1111);
           assert_eq!(busy, vec!["slot.1 [10:00 wt :: cargo check]", "slot.3 [bash lane]"]);
           assert!(busy_slots(&t.path().join("missing"), |_| true).is_empty());
       }

       #[test]
       fn the_lane_dir_follows_lane_ps1() {
           assert_eq!(
               lane_dir(Some("D:/x"), Some("C:/L")),
               Some(PathBuf::from("D:/x").join("lane"))
           );
           assert_eq!(
               lane_dir(None, Some("C:/L")),
               Some(PathBuf::from("C:/L").join("cimmeria-build").join("lane"))
           );
           assert_eq!(lane_dir(Some(""), None), None);
       }

       #[test]
       fn the_env_override_parses_seconds() {
           assert_eq!(lane_wait_from(None), LANE_WAIT);
           assert_eq!(lane_wait_from(Some("x")), LANE_WAIT);
           assert_eq!(lane_wait_from(Some("0")), Duration::ZERO);
           assert_eq!(lane_wait_from(Some("9999")), MAX_LANE_WAIT);
       }

       /// A held lane is waited out to the cap and reported busy; one that
       /// frees mid-wait returns idle at the poll that sees it.
       #[tokio::test(start_paused = true)]
       async fn the_wait_ends_when_the_lane_frees_or_the_cap_passes() {
           let t = tempfile::tempdir().unwrap();
           slot(t.path(), "slot.1", &[("winpid", "1"), ("what", "job")]);
           let held = wait_for_idle(t.path(), Duration::from_secs(20), Duration::from_secs(5), |_| true).await;
           assert_eq!(held.busy, vec!["slot.1 [job]"]);
           assert!(held.waited >= Duration::from_secs(20));

           let dir = t.path().to_path_buf();
           tokio::spawn(async move {
               tokio::time::sleep(Duration::from_secs(12)).await;
               std::fs::remove_dir_all(dir.join("slot.1")).unwrap();
           });
           let freed = wait_for_idle(t.path(), Duration::from_secs(60), Duration::from_secs(5), |_| true).await;
           assert!(freed.busy.is_empty());
           assert!(freed.waited >= Duration::from_secs(12) && freed.waited < Duration::from_secs(20));
       }
   }
   ```
   Why it fails when reverted: `busy_slots` ignoring `winpid` liveness fails the first test (slot.2 would be listed); a wait that never polls fails the last. The one-line hook in `launch_client` is covered only by the live smoke (a fake `Supervisor::launch_client` needs a real client).

**Lane checks**
```
pwsh -NoProfile -File WT\tools\build-lane\lane.ps1 cargo fmt -p cimmeria-lab
pwsh -NoProfile -File WT\tools\build-lane\lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
pwsh -NoProfile -File WT\tools\build-lane\lane.ps1 cargo nextest run -p cimmeria-lab lane_idle
```

**Docs** (`docs/guides/live-research-lab.md`, CRLF): in the watchdog section, before the paragraph "After a kill the watchdog relaunches the client", insert:
`Before every launch (`lab_client_start`, `lab_client_restart`, and a watchdog relaunch) the supervisor waits up to 90 s for the build lane to go idle: no `slot.N` directory under `%LOCALAPPDATA%\cimmeria-build\lane` (or `$LANE_ROOT\lane`), ignoring slots whose `lane.ps1` holder is dead. It then launches anyway; `labd.log` shows `lane_idle` (INFO, `waited_ms`) or `lane_busy_launch` (WARN, the held slots). A client that starts while four cargo jobs and sccache saturate the drive can sit in page-in waits until the watchdog gives up, so on a shared workstation still prefer to start labs when `python tools/build-lane/lane_stats.py --recent 5` shows nothing running. `CIMMERIA_LAB_LANE_WAIT_SECS` changes the wait (`labd.env`; `0` turns it off; at most 600). Putting the client install on the Dev Drive avoids the contention altogether.`
Add a row to the daemon environment-variable table if one lists `CIMMERIA_LAB_LOAD_GRACE_SECS` (check with `Grep "CIMMERIA_LAB_LOAD_GRACE_SECS" docs tools`), naming `CIMMERIA_LAB_IO_GRACE_SECS` (LH-14) and `CIMMERIA_LAB_LANE_WAIT_SECS`.

**Live smoke:** hold a lane slot (`pwsh -NoProfile -File WT\tools\build-lane\lane.ps1 pwsh -NoProfile -Command "Start-Sleep 40"` in one shell), then `lab_client_start`: the call returns after about 40 s and `labd.log` shows `lane_idle ... waited_ms~40000`. With nothing held, `lab_client_start` returns as fast as before and logs nothing.

**Commit subject:** `feat(lab): launches wait up to 90 s for the build lane to go idle (LH-15)`

---

## Item 14: display keep-awake never logged `held=true`

### Root cause (confidence: medium-high; one live check available)

`display::win::spawn_keep_awake` (`crates/lab/src/supervisor/display.rs:113-135`) decides `want = !process::running_sgw_pids().is_empty()`. `running_sgw_pids` (`process.rs:254-273`) is built on `EnumWindows`: it returns only an `SGW.exe` that **owns a top-level window**. The 01:57Z-02:28Z launches are exactly the ones that hung before the window appeared (item 13: "0 windows, empty Launch.log"), so for the whole launch the thread saw no `SGW.exe`, held nothing and logged nothing. The same rule explains the earlier observation that the keep-awake "engaged about 2 minutes after launch" (`docs/analysis/lab-spec-vocab/worknotes/SV-15.md`): it engaged when the window finally showed, up to 10 s later (poll interval). The screensaver risk is highest in that launch phase.

The missing `ensure_display_for_launch` probe line is expected, not a bug: on success it returns silently (`display.rs:151-153`). Add an INFO line so the next investigation can see it ran.

The orchestrator's ledger line `ledger.md:76` says "keep-awake held" at 02:24Z, which contradicts "never logged": that was a power-request check on an earlier thread state (the thread had last logged `held=true` at 00:15:36Z when a windowed client ran, and held it until `held=false`); it does not say the thread held it during these launches. Treat as consistent.

**Live check (optional, coordinator):** `lab_client_start`, then read `labd.log` within 15 s: before the fix `lab display keep-awake held=true` appears only once the window shows; after the fix it appears within 2-3 s of `launched`. A hung start (no window) after the fix still logs `held=true`.

### LH-16 (part 1) packet spec: keep-awake

**Files:** `crates/lab/src/supervisor/display.rs` only (part 2 below touches `flows/login.rs`; one commit each is fine, or one packet with two commits).

**Steps**
1. In `display.rs`, above `#[cfg(windows)] mod win` add the pure filter:
   ```rust
   /// Pids of `SGW.exe` among `(pid, exe file name)` process entries. The
   /// keep-awake counts every such process, window or not: a client that hangs
   /// before its window shows (a starved drive) is exactly when the screensaver
   /// must not start.
   pub fn sgw_pids_in(entries: impl IntoIterator<Item = (u32, String)>) -> Vec<u32> {
       entries
           .into_iter()
           .filter(|(_, exe)| crate::supervisor::process::is_sgw_image(exe))
           .map(|(pid, _)| pid)
           .collect()
   }
   ```
2. Inside `mod win`, extend the `use` lines and add the process walk:
   ```rust
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
   ```
   and
   ```rust
    /// Pids of every running `SGW.exe`, whether or not it has a window.
    pub fn sgw_process_pids() -> Vec<u32> {
        let mut entries = Vec::new();
        // SAFETY: a process snapshot walked with a correctly sized entry; the
        // snapshot handle is closed below.
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE {
                return Vec::new();
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut more = Process32FirstW(snap, &mut entry) != 0;
            while more {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                entries.push((
                    entry.th32ProcessID,
                    String::from_utf16_lossy(&entry.szExeFile[..len]),
                ));
                more = Process32NextW(snap, &mut entry) != 0;
            }
            CloseHandle(snap);
        }
        super::super::display::sgw_pids_in(entries)
    }
   ```
   (`super::super::display` is `crate::supervisor::display`; write it as `crate::supervisor::display::sgw_pids_in(entries)`.) The `Win32_System_Diagnostics_ToolHelp` feature is already enabled (`Cargo.toml:89`).
3. In `spawn_keep_awake` change `let want = !process::running_sgw_pids().is_empty();` to `let pids = sgw_process_pids(); let want = !pids.is_empty();`, the log to `tracing::info!(held = want, sgw_pids = pids.len(), "lab display keep-awake");`, and `std::thread::sleep(Duration::from_secs(10));` to `Duration::from_secs(2)`. Remove the now-unused `process` import only if clippy flags it (`dismiss_screensaver` still uses `process::`).
4. In `ensure_display_for_launch`, on the success path `if win::probe_d3d().is_ok() { return Ok(()); }` log first: `if win::probe_d3d().is_ok() { tracing::info!(target: "lab.client", event = "display_probe", ok = true, "lab launch: Direct3D probe ok"); return Ok(()); }`.

**Test** (`display.rs` tests module, append):
```rust
    /// Regression guard (2026-10-12): the keep-awake counted only an SGW.exe
    /// with a top-level window, so a client hung before its window never
    /// held the display. Every SGW.exe process counts, whatever else runs.
    #[test]
    fn a_windowless_client_process_keeps_the_display_awake() {
        let procs = vec![
            (100, "explorer.exe".to_string()),
            (4242, "SGW.exe".to_string()),
            (4300, "sgw.EXE".to_string()),
            (4400, "NotSGW.exe".to_string()),
            (4500, "SGW.exe.bak".to_string()),
        ];
        assert_eq!(sgw_pids_in(procs), vec![4242, 4300]);
        assert!(sgw_pids_in(Vec::new()).is_empty(), "no client, nothing held");
    }
```
Why it fails when reverted: the walk's selection is `sgw_pids_in`; the old rule (`EnumWindows`) cannot be expressed through it. Honest limit: the Windows walk itself and the thread are covered by the live check only (a unit test would need a spawned `SGW.exe`).

**Lane checks:** as for LH-14 with filter `display`.

**Docs** (`docs/guides/live-research-lab.md`, CRLF): replace line 594 `- While any `SGW.exe` runs, a supervisor thread holds `ES_DISPLAY_REQUIRED` (released when none runs), so the screensaver doesn't start.` with `- While any `SGW.exe` process exists, window or not (a client hung before its window still counts), a supervisor thread holds `ES_DISPLAY_REQUIRED` (checked every 2 s, released when none runs), so the screensaver doesn't start. `labd.log` shows `lab display keep-awake held=true` within a few seconds of `launched`.`; and in line 595 append `A passing probe logs `display_probe ok=true`.`

**Live smoke:** `lab_client_start`; `labd.log` shows `lab display keep-awake held=true sgw_pids=1` within 3 s of the `launched` row, before the window exists; `lab_client_stop` then logs `held=false` within 3 s.

**Commit subject:** `fix(lab): display keep-awake counts SGW.exe processes without a window (LH-16)`

---

## Item 15: `lab_ensure_in_world` with `server: "colo"` fails at `pick_server`

### Root cause (confidence: high)

Two different namespaces share the word "server":

- `LoginInternal.lua` rows (`login_servers.rs`: `Desktop`, `Cimmeria`, ...) are the client's **login servers**. The lab uses them to mint the telemetry token (`login_servers::pick`, which has a single-row fallback and an error listing the rows).
- The row `pick_server` selects (`flows/login.rs:307-348`, `select_server_chunk` at `:123-135`) is a **shard** in `ServerSelect_List`, named by the server's shard table. The colo has one shard, `Test` (`wireclient` defaults and the shard fixtures use `"Test"`; `crates/services/src/orchestrator_shards.rs:115`).

`lab-account.json` `server: "colo"` (and a `server: "colo"` argument) is a label for the deployment, so `findColumnItemWithText("colo", 0, nil)` is nil, the chunk returns `missing, Test`, and because the name came from the request `creds.server_explicit` is true, so `flows/login.rs:321-329` fails with `no server named "colo"; the list has: Test`. When the name came from the account file it only warns and uses the preselected row, which is why `lab_ensure_in_world` without an explicit server works.

### LH-16 (part 2) packet spec: a list with one row selects that row

Rule: a requested name that is missing from a server list of exactly one row selects that row and says so; two or more rows and no match stays an error (the existing guard, `server_explicit`). This mirrors `login_servers::pick`'s single-row rule (`login_servers.rs:63`), so no alias table is needed.

**Files:** `crates/lab/src/supervisor/flows/login.rs` (`select_server_chunk`, `pick_server`, tests), `docs/guides/live-research-lab.md`.

**Steps**
1. Replace `select_server_chunk` body (lines 123-135) with:
   ```rust
   pub fn select_server_chunk(name: &str) -> String {
       format!(
           "local item = ServerSelect_List:findColumnItemWithText({n}, 0, nil) \
            local only = '' \
            if item == nil then local names = {{}} \
              for i = 1, getShardCount() do names[#names + 1] = tostring(getShardInfo(i).name) end \
              if #names == 1 then \
                item = ServerSelect_List:findColumnItemWithText(names[1], 0, nil) only = 'only' end \
              if item == nil then return 'missing', table.concat(names, ', ') end end \
            ServerSelect_List:clearAllSelections() \
            ServerSelect_List:setItemSelectState(item, true) \
            local s = ServerSelect_List:getFirstSelectedItem() \
            return 'selected', tostring(s and s:getText()), only",
           n = widgets::lua_quote(name)
       )
   }
   ```
   and extend its doc comment: `Returns `selected, name, only`  (`only` is `only` when the requested name was missing and the list has just one row, which was selected) or `missing, <available names>`.`
2. In `pick_server`, replace the `Some("selected") => { ... }` arm (lines 316-320) with:
   ```rust
                Some("selected") => {
                    let got = r.get(1).cloned().unwrap_or_default();
                    if r.get(2).map(String::as_str) == Some("only") {
                        tracing::info!(
                            requested = %name,
                            selected = %got,
                            "lab_login: requested server not in the list; using its only row"
                        );
                        run.record(
                            "pick_server",
                            t0,
                            json!({ "server": got, "requested": name, "only_row": true }),
                        );
                    } else {
                        run.record("pick_server", t0, json!({ "server": got }));
                    }
                    return Ok(got);
                }
   ```
3. Tests (append inside the existing `mod tests` of `login.rs`; `fake_bridge`, `Arc`, `Value` are imported there; `FlowRun` is in scope via `use super::*` through `super::{..., FlowRun}`; `run.steps` is readable here because this module is a child of `flows`):
   ```rust
    fn creds(server: &str, explicit: bool) -> ResolvedLogin {
        ResolvedLogin {
            account: "a".into(),
            password: "p".into(),
            server: Some(server.into()),
            server_explicit: explicit,
        }
    }

    /// Live regression (2026-10-12): `server: "colo"` against a client whose
    /// list has the one row "Test" failed `pick_server`. The chunk now falls
    /// back to the only row and says so; the flow records it.
    #[tokio::test]
    async fn an_unknown_name_selects_the_only_row_and_says_so() {
        let sup = fake_bridge::supervisor(Arc::new(|method: &str, p: &Value| {
            assert_eq!(method, "lua_eval");
            let chunk = p["chunk"].as_str().unwrap_or_default();
            assert!(chunk.contains("findColumnItemWithText(\"colo\""), "{chunk}");
            Ok(fake_bridge::lua_ok(&["selected", "Test", "only"]))
        }))
        .await;
        let mut run = FlowRun::new(&sup, "lab_login");
        let got = sup.pick_server(&mut run, &creds("colo", true)).await.unwrap();
        assert_eq!(got, "Test");
        let step = run.steps.last().unwrap();
        assert_eq!(step["detail"]["only_row"], true, "{step}");
        assert_eq!(step["detail"]["requested"], "colo", "{step}");
    }

    /// With several rows and no match the explicit request still fails and
    /// lists the rows (the guard against picking the wrong shard).
    #[tokio::test]
    async fn an_unknown_name_with_several_rows_is_still_an_error() {
        let sup = fake_bridge::supervisor(Arc::new(|_, _| {
            Ok(fake_bridge::lua_ok(&["missing", "Test, Dev"]))
        }))
        .await;
        let mut run = FlowRun::new(&sup, "lab_login");
        let e = sup.pick_server(&mut run, &creds("colo", true)).await.unwrap_err();
        assert!(e.message.contains("no server named") && e.message.contains("Test, Dev"), "{}", e.message);
    }

    #[test]
    fn the_chunk_falls_back_to_a_single_row() {
        let c = select_server_chunk("colo");
        assert!(c.contains("if #names == 1 then"), "{c}");
        assert!(c.contains("only = 'only'"), "{c}");
        assert!(c.trim_end().ends_with("only"), "{c}");
    }
   ```
   `pick_server` is a private method of `Supervisor` defined in this file's `impl`; the tests module is in the same file, so it is callable. Why they fail when reverted: reverting the `Some("selected")` arm drops `only_row`/`requested`; reverting the chunk fails the text guard (the Lua branch cannot run offline, so the text test plus the live smoke guard it).

**Lane checks:** as LH-14 with filters `login` (`cargo nextest run -p cimmeria-lab login`).

**Docs** (`docs/guides/live-research-lab.md`): in the `lab_ensure_in_world` paragraph (line 625) after `` (server row `shard`, else `server`) `` insert: `. `server` / `shard` name a row of the client's server list, which is the shard list (the colo's one row is `Test`), not the launcher's login-server label: a name that is not in a list with exactly one row selects that row (the result's `pick_server` step carries `only_row: true` and `requested`); with two or more rows a name that is missing is still an error naming them`. (Keep the sentence flow so the following `, `lab_logout`` text still reads.)

**Live smoke** (needs the colo; lab-driver is fine): `lab_ensure_in_world {server: "colo", stop_at: "character_select"}` returns `at: "character_select"` and the `pick_server` step detail shows `server: "Test"`, `requested: "colo"`, `only_row: true`. Before the fix: `pick_server` fails with `no server named "colo"; the list has: Test`.

**Note (not in scope, flag to coordinator):** the telemetry-token side, `login_servers::pick(&rows, Some("colo"))`, errors when `LoginInternal.lua` has several login rows and none is named `colo` (`login_servers.rs:64-73`); that message already says to set `server` in `lab-account.json` or `CIMMERIA_LAB_SERVER_URL`. Untested here; if the lab box has both `Desktop` and `Cimmeria` rows and the account says `colo`, the launch (not `pick_server`) is what fails, and the fix is the account file, not code.

**Commit subject:** `fix(lab): lab_login uses a server list's only row when the requested name is missing (LH-16)`

(Parts 1 and 2 of LH-16 are independent files; commit them separately, or ship as two commits in one PR.)

---

## Item 16: `lab-driver` agent lacks tools

### Findings

- Repo copy `.claude/agents/lab-driver.md` (CRLF) already lists `client_cursor_move` and `client_input_mouse` (in its `tools:` line and in the Windows section). Still missing: `client_item_action`, `client_die_and_respawn`, `client_type_text`, `client_lua_eval`. All four exist as tools (`crates/lab/src/server/ui/mod.rs:203`, `server/combat/mod.rs:218`, `server/mod.rs:293`, `:520`) and are documented in `docs/guides/live-research-lab.md` (lines 282, 290, 292, 300).
- `client_chat_log` is already listed. Not asked, but `client_hotbar`, `client_use_ability`, `client_target`, `client_events_read` are also absent; leave them unless a brief needs them.
- `.codex/agents/lab-driver.toml` is generated from the md (`tools/agent-skills/sync.py`): rerun it after the edit.
- **User-level copy `C:\Users\Steve\.claude\agents\lab-driver.md` differs substantially** from the repo copy (diff run): it has `maxTurns: 40` (repo 50), lacks `lab_lease_status`, `lab_uat_run`, `lab_uat_report`, `client_wait_for`, `client_window_click`, `client_entity_find`, `client_entity_table`, `client_move_to`, `client_input_focus`, `client_cursor_move`, `client_input_mouse` in `tools:`, lacks the lent-lease, spec-row, calibration, NPC-interaction, chat/GM, Windows and new-character sections, and says `Take a lease first` with an `instance` argument sentence the repo copy dropped. In this repo the project agent normally shadows the user-level one of the same name, so this only matters from other directories. **Recommendation (user action, outside the repo, not part of the PR):** after LH-17 merges, overwrite the user-level file with the repo copy: `Copy-Item WT\.claude\agents\lab-driver.md C:\Users\Steve\.claude\agents\lab-driver.md -Force`, then re-add the one user-copy sentence worth keeping, `When the brief names a lab account or instance (lab2, p2), pass instance to lab_lease_acquire; after that the lease_id alone routes every call to that client.`, to the repo copy's lease rule if multi-instance driving is wanted (decision for the owner; it is true per `docs/guides/live-research-lab.md` line 458 onward).

### LH-17 packet spec

**ID / title / size:** LH-17, `lab-driver` gets item, respawn, typing and Lua tools, S. Depends on: run after every packet (LH-01..LH-16, other expanders) that adds a lab MCP tool; this packet lists the four known gaps and a slot for the rest.

**Files:** `.claude/agents/lab-driver.md` (CRLF), `.codex/agents/lab-driver.toml` (generated).

**Steps**
1. In `.claude/agents/lab-driver.md`, line 10 (`tools:`): replace the tail `mcp__cimmeria-lab__client_input_mouse, mcp__cimmeria-lab__lab_screenshot` with `mcp__cimmeria-lab__client_input_mouse, mcp__cimmeria-lab__client_item_action, mcp__cimmeria-lab__client_die_and_respawn, mcp__cimmeria-lab__client_type_text, mcp__cimmeria-lab__client_lua_eval, mcp__cimmeria-lab__lab_screenshot`. **Coordinator: before launching this packet, append here the `mcp__cimmeria-lab__<tool>` names of any tool other packets added (for example a new helper from LH-01..LH-12), and the matching guidance line below.** (The frontmatter `tools:` stays on one line.)
2. Insert before the line `Windows:` (line 46) this block (blank line after it):
   ```
   Items, death and raw input:
   - `client_item_action {action, ...}` uses an item like a player: `use`, `equip`, `unequip`, `rightclick` (a real right-click on the slot), `doubleclick`, `loot_all`, `loot_slot` (with `loot_index`, 1-based). Pick the item by container + slot, `item_id` or `name`. It reports the inventory diff seen within `wait_ms` and `native_level`; `slash_command` means it fell back to `/useitem`, `/equip` or `/lootitem` because the slot could not be put on screen. Report that field.
   - `client_die_and_respawn` does the whole death sequence: optional `setup_health` (it types `/gmsethealth n 0`, which does not kill by itself), waits for the defeat window, reads its respawners and countdown, clicks Release (or lets the countdown run with `respawn: "auto"`), and verifies alive, position and world. Give it the arguments the brief names; do not click `PlayerDefeat_Release` by hand.
   - `client_type_text` types into the focused edit box, one key press per character (letters, digits, space, `- _ / .`). It needs the edit box clicked first. For chat prefer `client_ui_sequence` `{do: "type", into: "Inst1Chat_Input"}`, which clicks it for you.
   - `client_lua_eval` runs one Lua chunk on the client's main thread. Use it only when the brief gives the chunk; several reads belong in one `client_batch`.
   ```
3. Run `python tools/agent-skills/sync.py` from `WT` (via the lane's Python is not needed; it does not compile) to regenerate `.codex/agents/lab-driver.toml`; check `git diff --stat` shows only the two files. Then run `python tools/agent-skills/test_sync.py` if it exists (`Glob tools/agent-skills/test_sync.py`).
4. Apply the CRLF footer to `.claude/agents/lab-driver.md`.

**Test:** none (prompt text). Verification: `Grep` that each of the four new tool names appears in the `tools:` line and that every `mcp__cimmeria-lab__*` name in it is a `#[tool]` fn in `crates/lab/src/server/` (`Grep "async fn client_item_action|async fn client_die_and_respawn|async fn client_type_text|async fn client_lua_eval" crates/lab/src/server`). No theatre test is worth adding; the sync test already pins the generator.

**Lane checks:** none compile. Run `python tools/agent-skills/sync.py` then `git status --short` (expect `.claude/agents/lab-driver.md` and `.codex/agents/lab-driver.toml`).

**Docs:** `docs/guides/automated-uat.md` / `live-research-lab.md` mention the lab-driver's tool list? Check `Grep "lab-driver" docs/guides docs/agents`; update any list of its tools found. If none, nothing else.

**Live smoke** (coordinator, Haiku `lab-driver` subagent, lab free and user OK): brief `lab_client_status` then `client_lua_eval {chunk: "return 1+1"}`; expected result `2` and no "tool missing from your tool list" report. That proves the new tool is exposed to the agent (the agent reloads on session start).

**Commit subject:** `docs(lab): lab-driver agent gets client_item_action, client_die_and_respawn, client_type_text and client_lua_eval (LH-17)`

---

## Owner decisions to flag

1. **LH-13 touches `crates/admin-api` (product crate).** Additive: a second ring inside `LogBuffer`; the admin WebSocket connect-time history can now hold up to 1000 entries (server 500 + forwarded client 500) merged by time. Recommended: accept. The alternative (a lab-only subscriber layer in `crates/server`) is larger.
2. **LH-14 defaults:** IO grace 300 s, load-stall budget 6 per hour, IO-progress threshold 256 KiB or 2 % CPU per interval. Recommended as written; the first real recurrence shows `io_read_kb` / `process_cpu_ms` on the log line (see LP-13 for the fallback if page-in is invisible to these counters).
3. **LH-15 default wait 90 s** (env `CIMMERIA_LAB_LANE_WAIT_SECS`). Recommended; a launch is never refused. Moving the client install to the Dev Drive is the real cure and is a user-environment choice (not coded).
4. **User-level `lab-driver.md`** is stale and should be overwritten from the repo copy after LH-17 (user action, outside the PR).

## Live probes needed

- Item 12, 14, 15, 16: none before coding; only the per-packet smoke steps.
- Item 13: optional LP-13 (disk-load stall, read `labd.log` for `io_read_kb`/`process_cpu_ms`), coordinator-run, ask the user first; not a blocker for LH-14, which degrades to today's behaviour if the counters show nothing.
