# Launcher telemetry on Linux: work packets

Ledger: [README.md](README.md). Resume card: [RESUME.md](RESUME.md). The anchors (`file:line`) were taken at `origin/main` `a26764cb6`. Every packet re-greps its anchors and returns `blocked` when one no longer holds.

Paths:

- `E` = `crates/launcher/desktop/engine/src/storage/game_telemetry/`
- `S` = `crates/launcher/desktop/shell/src/`
- `D` = `crates/admin-api/src/routes/dev_session/`
- `W` = `<repo>\.claude\worktrees\`

## Dispatch

**One PR for the whole campaign (D-LTL7, owner, 2026-10-26).** Every packet, including LTL-B2b and LTL-B2, merges into the integration branch `launcher-telemetry-linux/forwarding` (worktree `ltl-int`) with `merge-noff.ps1`. One PR ships that branch at the end. There is no PR M, F, S or C any more; PR #2038 (LTL-B2b on its own) was closed for this.

| Wave | Packets, in merge order into `ltl-int` | Branches from | State |
|---|---|---|---|
| 1 | LTL-02, LTL-01, LTL-03 | the `ltl-int` tip | merged (tip `f3cedb7c3`); wave review pass 1 = `decision`, ruled in `worknotes/wave1.md` |
| 1 | the wave 1 fix (N1-N3 of `wave1-review-1.md`): one fresh packet-coder in `ltl-int`, then the ruling check | - | next |
| 1 | LTL-B2b (branch `launcher-telemetry-linux/LTL-B2b`, head `e7bfea11b`, packet loop `ship`) | `origin/main` `a26764cb6` | merge into `ltl-int` after the wave 1 fix, before LTL-04 branches (both edit `E/mod.rs`, in different hunks; merge-tree is clean) |
| 1 | LTL-B1 (research, read-only) | `origin/main` | done (H1 confirmed); copy its report to `worknotes/LTL-B1.md` |
| 2 | LTL-04, then LTL-05, then LTL-06, one at a time | the `ltl-int` tip after the packet before it has merged in | after LTL-B2b |
| 2 | LTL-B2, in parallel with LTL-04 to 06 (no shared file) | the `ltl-int` tip after LTL-B2b | gated on D-LTL4 |
| 2 | LTL-07, last in the wave | the `ltl-int` tip after LTL-06 **and** LTL-B2 (both edit `docs/operations/telemetry.md`, in different sections) | gated on D-LTL3 |
| 3 | the campaign review on the `ltl-int` tip (diff against `origin/main`), its fixes in `ltl-int`, then LTL-08, then the PR | the `ltl-int` tip | after wave 2 |

- Each packet `LTL-NN` has worktree `ltl-NN` on its own branch; `ops` merges it into `ltl-int` with `merge-noff.ps1`. Once the campaign PR merges, `ops` retires them all with `rm-worktree.ps1 --merged-into <PR> <name>...`.
- **Opening the PR.** After LTL-08 has merged in: `ops` merges `origin/main` into `ltl-int` (`merge-noff.ps1`). If that conflicts (PR #2031: the `S/main.rs` tail, the privacy-inventory row, `index.html:91`), a fresh packet-coder resolves it in `ltl-int`, keeping both sides. Then run E2E, S2 and S3 `dev_session` once on the tip, then `ship.ps1 pr -C <W>\ltl-int`. Merge only on green CI.
- **No partial ship.** Never ship any packet alone. LTL-01 on its own makes a 5xx hold for 60 s while today's loop still drops the batch (wave 1 review D7); the single PR is what makes that safe.
- If the owner refuses D-LTL4, LTL-B2 is dropped, F5 stays open and the architect re-specifies. Whether the PR may open while D-LTL4 is still undecided is an owner call (README, D-LTL7).
- PR #2031 overlap: only LTL-06 (`S/main.rs` tail) and LTL-07 (privacy-inventory row, `index.html:91`). See the README's conflict table.
- Review loops are in the README's Review plan.

## Rules (every packet; the coder reads these)

- **R1 Host and shell.** `Host: windows`. Use PowerShell only, never Bash. Start every command with `Set-Location <worktree>;`, use absolute paths, run it in the foreground and wait for it. Never call `cargo` directly; only through `pwsh tools/build-lane/lane.ps1`. Run the checks in the listed order, with at most 5 fix rounds per check. If `logs/<worktree>/` shows no new lane run, you ran from the wrong folder.
- **R2 Scope and git.** Edit only the files your packet names, plus the new files it names. No `git stash`, no `git worktree prune`, no push, no rebase. Make one local commit with the given subject, ending with:
  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01WRynanEMGjFbBzPn7s7aWe
  ```
- **R3 File caps.** The soft cap is 500 lines and the hard cap 700. A named file already over 450 lines gets only call lines and `mod` lines; put the logic in the new file the packet names. Tests go in a sibling `*_tests.rs` file wired the way its neighbours are. Report every touched file's line count.
- **R4 Line endings.** Files under `docs/` are CRLF; keep them CRLF. Code files keep their current line endings. `git diff --stat` must show only your own lines.
- **R5 Privacy.** Never put an absolute path, URL, host, IP, port, account or user name, token, session id, or an error's `Display`/`Debug` in any event field or message. Use closed snake_case codes, counts and sizes. Nothing in this campaign writes telemetry to disk.
- **R6 Consent (D-LTL2).** An event enters the queue only if it was built by `launcher_batch` while the player was opted in, as today. The queue is cleared, unsent, when the target is `None` or the consent epoch changes. Never add a disk file, a retry file or a second queue.
- **R7 Logs.** At most 12 fields per event, snake_case keys, numbers as numbers. Never use the key `count`. Every new log line goes on target `launcher.pipeline` (file only) or `launcher.telemetry`.
- **R8 Tests.** Unit tests ([TESTING.md](../../../TESTING.md)). Every new test must fail when your change is reverted; say in the report which assertion fails and why. Pin targets, levels and fields, not message text. Capture tracing only with `crate::test_capture`. For time logic, pass `Instant`s in; no `sleep` longer than 100 ms, except where a packet allows it.
- **R9 Report and return.** Write `<worktree>\.claude\reports\launcher-telemetry-linux\<packet>-coder.md` with:
  - the files and their line counts;
  - each check and its result;
  - the revert proof;
  - anything you rejected, and why.

  Return exactly `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. When the packet does not cover something, return `blocked`; don't invent a design.
- **R10 Unchanged surfaces.** These stay byte for byte the same unless the packet names them:
  - the `TelemetryStatus` DTO (`S/host/telemetry_status.rs`) and its TS schema;
  - `record_outcome`'s signature (used by `S/host/launcher_telemetry_uat.rs`);
  - `spawn_forwarder`'s signature (called at `S/main.rs:640`);
  - `LauncherSender::post_batch`.

### Checks (copy the exact command)

| ID | Command |
|---|---|
| D1 | `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all` |
| D2e | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings` |
| D2s | the same with `-p cimmeria-launcher-desktop` |
| D3e `<f>` | `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine <f>` (engine tests use `cargo test`; nextest cannot list that crate) |
| D3s `<f>` | the same with `-p cimmeria-launcher-desktop <f>` |
| S1 | `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-admin-api -p cimmeria-auth -p cimmeria-base-world-entry` |
| S2 | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked -p cimmeria-admin-api --all-targets -- -D warnings` |
| S3 `<f>` | `pwsh tools/build-lane/lane.ps1 cargo test --locked -p cimmeria-admin-api --lib <f>` |
| S4 | `pwsh tools/build-lane/lane.ps1 cargo check --locked -p cimmeria-auth -p cimmeria-base-world-entry` |
| E2E | the whole engine suite (D3e with no filter), then the whole shell suite (D3s with no filter); run once, last |

## Contract

### C1: `SendReport` (LTL-01, `E/launcher_send.rs`)

```rust
/// What one send did, per event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SendReport {
    pub outcome: SendOutcome,
    /// Events accepted by the server (the sum of the posted chunk ranges).
    pub posted: usize,
    /// `Some(i)`: `events[i..]` were not posted and may be sent again.
    /// `None`: whatever was not posted is dropped.
    pub retry_from: Option<usize>,
}
impl LauncherSender {
    pub async fn send_report(&self, target: &LauncherTarget, events: &[TelemetryEvent]) -> SendReport;
    pub async fn send(&self, target: &LauncherTarget, events: &[TelemetryEvent]) -> SendOutcome; // = send_report(..).await.outcome
    pub fn hold_remaining(&self) -> Option<Duration>; // was #[cfg(test)]; now always compiled
}
```

| Case | `outcome` | `posted` | `retry_from` |
|---|---|---|---|
| `events` empty | `Empty` | 0 | `None` |
| a hold is running | `Held` | 0 | `Some(0)` |
| the mint failed | `MintFailed { reason }` | 0 | `Some(0)` |
| `split_batches` failed | `Dropped` | 0 | `None` |
| every chunk 2xx | `Sent` | `events.len()` | `None` |
| a chunk got 429 or 503 (the hold is taken as today) | `Dropped` | the events before that chunk | `Some(range.start)` |
| a chunk got 401 and the second mint failed | `Dropped` | the events before that chunk | `Some(range.start)` |
| a chunk got `ChunkError::Http` or `Status` (D-LTL5: also `self.hold(MIN_RETRY_HOLD)`) | `Dropped` | the events before that chunk | `Some(range.start)` |
| a chunk got `Rejected`, `Json` or `Io` | `Dropped` | the events before that chunk | `None` |

The debug lines on `launcher.pipeline` stay as they are.

### C2: `LauncherQueue` (LTL-02, `E/launcher_queue.rs`)

```rust
pub const QUEUE_CAP: usize = 500;
pub const COALESCE: Duration = Duration::from_secs(2);
pub const MIN_SPACING: Duration = Duration::from_secs(7);
pub const FILL: usize = 64;                    // a full group sends without waiting for COALESCE
pub const SEND_MAX: usize = LAUNCHER_MAX_ROWS; // 200, at most per take
pub const FLUSH_TARGET: &str = "launcher.flush"; // the exit sentinel's LauncherRecord.target

/// True for the sentinel (target == FLUSH_TARGET) and for a record with target
/// "client.launcher.launch" whose fields["event"] == "launch_finished".
pub fn is_flush_trigger(record: &LauncherRecord) -> bool;

#[derive(Debug, Default)]
pub struct LauncherQueue { /* VecDeque<TelemetryEvent>, oldest: Option<Instant>, last_send: Option<Instant>, urgent: bool, lost: u64, epoch: Option<u64> */ }
impl LauncherQueue {
    pub fn new() -> Self;
    /// If not empty and `epoch` differs from the stored epoch, clears first (not counted as lost).
    /// Stores `epoch`, appends, sets `oldest` when it was None, then drops from the front
    /// past QUEUE_CAP, adding each to `lost`.
    pub fn push(&mut self, events: Vec<TelemetryEvent>, epoch: u64, now: Instant);
    pub fn mark_urgent(&mut self);
    /// !empty && hold_left.is_none() && (urgent ||
    ///   (last_send.map_or(true, |t| now >= t + MIN_SPACING) && (len >= FILL || now >= oldest + COALESCE)))
    pub fn due(&self, now: Instant, hold_left: Option<Duration>) -> bool;
    /// None when empty; Some(h) when hold_left == Some(h); Some(ZERO) when urgent;
    /// else max(spacing_at, if len >= FILL { now } else { oldest + COALESCE }) - now, saturating.
    pub fn next_wake(&self, now: Instant, hold_left: Option<Duration>) -> Option<Duration>;
    /// Drains up to SEND_MAX from the front and sets last_send = now. When the queue is
    /// left empty: urgent = false, oldest = None.
    pub fn take(&mut self, now: Instant) -> Vec<TelemetryEvent>;
    /// Puts `unsent` back at the front, in order, then trims the front past QUEUE_CAP into `lost`.
    /// Sets oldest = Some(now) if it was None.
    pub fn give_back(&mut self, unsent: Vec<TelemetryEvent>, now: Instant);
    /// Clears everything, including urgent, oldest and epoch; returns how many events it cleared (not lost).
    pub fn withdraw(&mut self) -> usize;
    pub fn take_lost(&mut self) -> u64; // returns and zeroes `lost`
    pub fn epoch(&self) -> Option<u64>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}
```

### C3: health counts events (LTL-03, `E/health.rs`, `S/telemetry.rs`)

- `TelemetryHealth` gains `pub events_sent: u64`, `pub events_lost: u64` and `pub events_lost_since_success: u64`.
- New `pub fn record_events(&mut self, sent: u64, lost: u64)`, which adds `sent` to `events_sent` and `lost` to both lost counters.
- `Recovery` gains `pub events_lost: u64`.
- `succeeded` takes `events_lost_since_success` into the `Recovery`. It returns `Some` when the failures are not all zero **or** `events_lost_since_success > 0`.
- Shell: `pub(crate) fn record_pumped(health: &SharedHealth, outcome: &SendOutcome, sent: u64, lost: u64, now_ms: u64) -> Option<Recovery>`. It calls `record_events`, then does what `record_outcome` does.
- `pipeline_recovered` gains the field `events_lost = recovery.events_lost`.
- `record_outcome` keeps its signature (R10).

### C4: `Forwarder` (LTL-04, `E/launcher_forwarder.rs`)

```rust
/// What the forwarder reads from the host. Each call takes the state lock only to read.
pub trait ForwardSource {
    /// `launcher_batch`'s events for `records`, or None (no module, or not opted in).
    fn events(&self, records: &[LauncherRecord]) -> Option<Vec<TelemetryEvent>>;
    /// `launcher_target()` now, or None.
    fn target(&self) -> Option<LauncherTarget>;
    /// `consent_epoch()` now.
    fn epoch(&self) -> u64;
}
/// Bumped by `set_game_telemetry` on every change of choice (D-LTL2).
pub fn consent_epoch() -> u64;
pub(crate) fn consent_changed();
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pumped { pub outcome: SendOutcome, pub sent_events: u64, pub lost_events: u64 }
pub struct Forwarder { /* queue: LauncherQueue, sender: LauncherSender, flush_pending: bool, flush_done: bool */ }
impl Forwarder {
    pub fn new(sender: LauncherSender) -> Self;
    /// Reads `let epoch = source.epoch();` FIRST, before anything else (D1 ruling).
    /// A sentinel record (target FLUSH_TARGET) sets flush_pending and is not built.
    /// The other records go to `source.events`; Some(events) is pushed with that
    /// `epoch`, None is discarded. Never read the epoch after `events`.
    /// Last: if any record was a flush trigger AND the queue is not empty, mark it urgent
    /// (D4 ruling: never mark an empty queue urgent).
    pub fn on_records<S: ForwardSource>(&mut self, source: &S, records: Vec<LauncherRecord>, now: Instant);
    /// queue.next_wake(now, sender.hold_remaining()); Some(ZERO) while flush_pending.
    pub fn next_wake(&self, now: Instant) -> Option<Duration>;
    /// At most one send. See LTL-04 for the steps.
    pub async fn pump<S: ForwardSource>(&mut self, source: &S, now: Instant) -> Option<Pumped>;
    /// True once since the last call when a flush finished (sent, failed, held or nothing to send).
    pub fn take_flush_done(&mut self) -> bool;
    #[cfg(test)] pub(crate) fn sender(&self) -> &LauncherSender;
    #[cfg(test)] pub(crate) fn queued(&self) -> usize; // queue.len()
}
```

### C5: shell forward loop and exit flush (LTL-05, LTL-06, `S/telemetry_forwarder.rs`)

```rust
pub(crate) const EXIT_FLUSH_WAIT: Duration = Duration::from_secs(3);
/// Receives, coalesces and pumps until `records` disconnects, then pumps once more as a flush.
pub(crate) fn forward_loop<S: ForwardSource>(
    records: &Receiver<LauncherRecord>,
    forwarder: &mut Forwarder,
    source: &S,
    on_pumped: &mut dyn FnMut(&Pumped),
    on_flushed: &mut dyn FnMut(),
);
pub(crate) struct FlushSignal { /* Mutex<u64>, Condvar */ }
impl FlushSignal { pub(crate) fn new() -> Self; pub(crate) fn notify(&self); }
/// Sends the sentinel on `tx` and waits up to `wait` for `done` to move on. False on a full channel or a timeout.
pub(crate) fn request_flush(tx: &SyncSender<LauncherRecord>, done: &FlushSignal, wait: Duration) -> bool;
pub(crate) fn remember_sender(tx: SyncSender<LauncherRecord>); // stores FLUSH_TX (OnceLock)
pub(crate) fn flush_on_exit() -> bool; // request_flush(FLUSH_TX, FLUSH_DONE, EXIT_FLUSH_WAIT); false if never initialised
```

## Wave 1

### LTL-01: `SendReport` (F1)

- **Host:** windows. **Agent:** packet-coder. **Worktree:** `W\ltl-01`, branch `launcher-telemetry-linux/ltl-01-send-report`, from the `ltl-int` tip. **Rules:** R1-R10. **Contract:** C1; D-LTL5.
- **Files:**
  - `E/launcher_session.rs` (492 lines: over 450, so call lines and `mod` lines only, and it must shrink);
  - new `E/launcher_send.rs`;
  - `E/launcher_session_tests.rs`.
- **Do:**
  1. In `launcher_session.rs`, add `#[path = "launcher_send.rs"] mod send;` and `pub use send::SendReport;`. A child module can call the parent's private `held`, `hold`, `usable_session`, `mint`, `post` and `locked`.
  2. Move the body of `send` (`:162-234`) into `launcher_send.rs` as `impl super::LauncherSender { pub async fn send_report(..) -> SendReport }`. Keep every debug line, and follow C1's table exactly. Count `posted` by adding each successful `range.len()`.
  3. Turn `send` into a one-line wrapper over `send_report(..).await.outcome`, keeping its doc comment.
  4. Remove `#[cfg(test)]` from `hold_remaining` (`:364`), but keep it on `clear_hold`.
  5. Change the module doc (`:1-3`) from "a send that cannot post drops its events" to "a send reports which events it posted; the forwarder decides what to retry (in memory only)".
- **Tests** (unit, `#[tokio::test]` in `launcher_session_tests.rs`, using the file's `target`, `event`, `mount_mint` and `mount_chunk`):
  1. **A 429 on the second chunk.** Send 300 events, which `LAUNCHER_MAX_ROWS = 200` splits into two chunks. Answer the first chunk 200 (`up_to_n_times(1)`), then 429 with `Retry-After: 120`. Expect `{ Dropped, posted: 200, retry_from: Some(200) }`, and `hold_remaining()` ≥ 110 s.
  2. **Held.** After test 1's hold, `send_report` returns `{ Held, 0, Some(0) }` and posts nothing; use `.expect(2)` on the chunk mocks.
  3. **A rejected chunk.** A 403 gives `{ Dropped, 0, None }`, and `hold_remaining()` is `None`.
  4. **A 500 (D-LTL5).** A 500 gives `{ Dropped, 0, Some(0) }`, and `hold_remaining()` is `Some` and ≥ 55 s.

  Revert proof: `SendReport` and `send_report` are new. Name the assertion each test pins, and say that test 4 fails if the 500 does not hold. The existing tests must still pass unchanged.
- **Checks:** D1, D2e, D3e `launcher_session`.
- **Docs:** none (LTL-07).
- **Commit:** `feat(launcher): telemetry sends report posted events and a retry point (LTL-01)`

### LTL-02: `LauncherQueue` (F1)

- **Host:** windows. **Agent:** packet-coder. **Worktree:** `W\ltl-02`, branch `launcher-telemetry-linux/ltl-02-queue`, from the `ltl-int` tip. **Rules:** R1-R10. **Contract:** C2; D-LTL1, D-LTL2.
- **Files:**
  - new `E/launcher_queue.rs`;
  - new `E/launcher_queue_tests.rs`;
  - `E/mod.rs`: add `pub mod launcher_queue;` and `#[cfg(test)] mod launcher_queue_tests;` in alphabetical order (after `launcher_event_tests`). Nothing else in that file.
- **Do:**
  - Implement C2 exactly as a pure struct: no I/O, no clock reads, no logging.
  - `TelemetryEvent` is `super::transport::TelemetryEvent`, `LauncherRecord` is `super::launcher_event::LauncherRecord` and `LAUNCHER_MAX_ROWS` is `super::launcher_session::LAUNCHER_MAX_ROWS`.
  - `is_flush_trigger` reads `record.fields.get("event").and_then(Value::as_str)`.
  - If clippy asks for `Default` alongside `new`, derive it.
- **Tests** (unit, `launcher_queue_tests.rs`; `let t0 = Instant::now();`, offsets with `t0 + Duration::from_millis(..)`; build events like `launcher_session_tests::event()`):
  1. One event at `t0`: not due at `t0 + 1.9 s`, due at `t0 + 2 s`; `next_wake(t0)` is 2 s.
  2. After `take(t0 + 2 s)`, a new event at `t0 + 3 s` is not due at `t0 + 5 s` (spacing) and is due at `t0 + 9 s`.
  3. 64 events at `t0` are due at `t0` (FILL); `take` returns at most 200 of 250.
  4. With `hold_left = Some(30 s)`, nothing is due even when urgent, and `next_wake` is 30 s.
  5. `mark_urgent` makes one event due at once; `next_wake` is ZERO.
  6. 510 pushes leave `len() == 500` and `take_lost() == 10`, then 0; the front is the 11th event, so check a field that varies by index.
  7. `give_back` keeps order ahead of newer events and trims the oldest past 500.
  8. A push with a different epoch clears the old events and does not count them as lost. `withdraw` returns the count and leaves `epoch() == None`.
  9. `is_flush_trigger`: true for `FLUSH_TARGET`; true for `client.launcher.launch` with `event = "launch_finished"`; false for `launch_started`; false for `client.launcher.install` with `launch_finished`.

  Revert proof: the module is new. Name the assertion each test pins.
- **Checks:** D1, D2e, D3e `launcher_queue`.
- **Docs:** none.
- **Commit:** `feat(launcher): in-memory coalescing hold queue for launcher telemetry (LTL-02)`

### LTL-03: health counts events (F3)

- **Host:** windows. **Agent:** packet-coder. **Worktree:** `W\ltl-03`, branch `launcher-telemetry-linux/ltl-03-event-health`, from the `ltl-int` tip. **Rules:** R1-R10. **Contract:** C3.
- **Files:** `E/health.rs` (154 lines; its tests are inline at `:85`), `S/telemetry.rs` (317 lines).
- **Do:**
  1. Make the C3 changes in `health.rs`, and update the module doc to say it counts sends and events.
  2. In `S/telemetry.rs`:
     - add `record_pumped` next to `record_outcome` (`:200`); factor out the shared lock-record-log body so it is written once;
     - add `events_lost = recovery.events_lost,` to `pipeline_recovered` (`:214`);
     - leave `spawn_forwarder` unchanged (LTL-05 owns it).
  3. If `#[allow(dead_code)]` is needed on `record_pumped` until LTL-05, add it with a `// LTL-05 calls this` comment.
- **Tests** (unit):
  1. In `health.rs`: `record_events(5, 2)`, then `record(Dropped)`, then `record_events(3, 0)`, then `record(Sent)` gives a `Recovery` with `events_lost == 2`, and `events_sent == 8`.
  2. In `health.rs`: `record_events(0, 4)`, then `record(Sent)`, with no failed send, still gives `Some(Recovery)` with `events_lost == 4` and all-zero failures.
  3. In `S/telemetry.rs`'s test module, under `crate::test_capture`: `record_pumped(Dropped, 0, 3)`, then `record_pumped(Sent, 2, 0)` emits exactly one `pipeline_recovered` with `events_lost = "3"`.

  Revert proof: tests 1 and 3 fail without the counters or the field, and test 2 fails on today's early `return None` in `succeeded`.
- **Checks:** D1, D2e, D2s, D3e `health`, D3s `telemetry`.
- **Docs:** none (LTL-07).
- **Commit:** `feat(launcher): telemetry health counts lost events (LTL-03)`

### LTL-B2b: a real Linux `machine_id` (F6)

- **Host:** windows. **Agent:** packet-coder. **Worktree:** `W\ltl-b2b`, branch `launcher-telemetry-linux/LTL-B2b`, from `origin/main`. **Rules:** R1-R10. **Ships:** merged into `ltl-int` (D-LTL7); done, packet loop `ship` at `e7bfea11b`, PR #2038 closed.
- **Files:** `E/mod.rs` (182 lines), `E/tests.rs` (the module's sibling tests, `#[cfg(test)] mod tests;` at `:27`). If `E/tests.rs` is over 450 lines, use a new `E/machine_id_tests.rs` wired with one `#[cfg(test)] mod machine_id_tests;` line instead.
- **Do:**
  1. In `host_name()` (`:169`), change the `#[cfg(target_os = "macos")]` block to `#[cfg(unix)]`. `libc` is already a `cfg(unix)` dependency; check that in `engine/Cargo.toml` and return `blocked` if it is not.
  2. Add `fn machine_id_for(name: &str) -> String`, the `sha256(name)[..8]` hex that `Identity::mint` computes today, and make `mint` call it.
  3. Add `pub(crate) const EMPTY_NAME_MACHINE_ID: &str = "e3b0c44298fc1c14";` with a comment that this is `sha256("")`, the id every Linux launcher minted before LTL-B2b.
  4. In `game_telemetry()` (`:99`), after validation: if `identity.machine_id == EMPTY_NAME_MACHINE_ID` and `host_name()` is not empty, replace it **in the returned value only** with `machine_id_for(&host_name())`. Don't write the file (that function reads). Put the logic in a small `fn repaired(identity: &mut Identity, name: &str)` so tests need no environment.
- **Tests** (unit):
  1. `machine_id_for("") == EMPTY_NAME_MACHINE_ID`. This pins the constant.
  2. `repaired` on an identity holding the empty-name id with the name `"deck"` changes it to `machine_id_for("deck")`; with the name `""` it leaves it unchanged; any other id is never changed.
  3. On Windows CI, `host_name()` is not empty (`COMPUTERNAME` is always set there).

  Revert proof: tests 1 and 2 fail because the fns are new. Say in the report that the `cfg(unix)` path is compiled only on Linux and macOS CI.
- **Checks:** D1, D2e, D3e `game_telemetry`.
- **Docs:** none. LTL-07 owes nothing for this packet: [dev-session-telemetry.md](../../architecture/dev-session-telemetry.md) already says "host name".
- **Commit:** `fix(launcher): Linux telemetry machine id from gethostname, not an empty name (LTL-B2b)`

### LTL-B1: why the Deck was `absent` (research, read-only)

- **Host:** windows. **Agent:** `network-security-auth` (read-only: no edits, no builds, no commits). **Worktree:** `W\ltl-b1` (an `ops` worktree from `origin/main`). **Report:** `W\ltl-b1\.claude\reports\launcher-telemetry-linux\LTL-B1-research.md`. The orchestrator copies it to `docs/analysis/launcher-telemetry-linux/worknotes/LTL-B1.md` before the worktree is retired.
- **Question:** name the exact cause of the Deck's world-entry `client_telemetry="absent"` (2026-10-26), or the tests that would tell the causes apart. Use SigNoz (the `telemetry-triage` skill; no IPs are logged, so use the session ids, `session_kind`, the route and timing facts) and the code.
- **Hypotheses, in order:**
  - **H1 (leading).** `D/handlers.rs:362` and `:481` record the stream under the socket peer, not `rate_ip`. Behind the board Caddy's loopback proxy that peer is `127.0.0.1`. Check:
    - Did the Deck's game-session mint or refresh go over the https route (proxied), while the Windows `present` sessions went over direct `http://…:8081`, or came before CT-51 / #2030?
    - What `login_servers` URL was the Deck using?
    - Does the DLL's refresh under Proton go through the same route?
  - **H2.** The mint went over IPv6 while the game connection used IPv4. `lookup` compares canonical addresses.
  - **H3.** F6's shared empty-name `machine_id` folds Linux machines together in `by_machine` (`insert_newest`). Is any other Linux or WSL launcher minting under `e3b0c44298fc1c14`?
  - **H4.** TTL or ordering: the stream expired, or was recorded after world entry.
  - **Also:** where does the DLL read its session file under Proton? See `crates/client-telemetry/src/session.rs` (`SESSION_FILE_ENV = "CIMMERIA_LAB_SESSION_FILE"`, `resolve_session_path`) and `boot.rs:246`, `:417`. Is the path the launcher writes reachable as a Wine path, and did the DLL upload at all?
  - **Also:** does a stream that names the account (`by_account`, D-TG19) make `present` reachable without any IP? If it does, say whether that would be a fix that needs no D-LTL4.
- **Return:**
  - the cause, with its evidence (SigNoz queries with the IPs left out, and `file:line`);
  - whether the LTL-B2 pre-spec below fixes it;
  - any change to that spec.

  Return the line `LTL-B1 done - <report-path>`.

## Wave 2

### LTL-04: engine `Forwarder` (F1)

- **Host:** windows. **Agent:** packet-coder. **Worktree:** `W\ltl-04`, branch `launcher-telemetry-linux/ltl-04-forwarder`, from the `ltl-int` tip, which holds LTL-01, 02, 03, the wave 1 fix and LTL-B2b. **Rules:** R1-R10. **Contract:** C1, C2, C4; D-LTL2, D-LTL5. The wave 1 ruling (`worknotes/wave1.md`, D1-D4) is already folded into the text below; follow this text.
- **Files:**
  - new `E/launcher_forwarder.rs`;
  - new `E/launcher_forwarder_tests.rs`;
  - `E/mod.rs`: two `mod` lines, plus one call line in `set_game_telemetry` (re-grep it; about `:120-136` after LTL-B2b): `launcher_forwarder::consent_changed();` right after `transport::discard_queue(..)`, so the epoch moves only after the new choice is stored.
- **Do:** implement C4.
  - `consent_epoch` and `consent_changed` use a `static CONSENT_EPOCH: AtomicU64`, read and bumped with `SeqCst`.
  - `on_records` (D1, D4), in this order:
    1. `let epoch = source.epoch();` This is the first line. Never read the epoch after `source.events(..)`: `set_game_telemetry` can run between the two calls, and events built while opted in would then carry the post-opt-out epoch and be sent. Reading it first costs at most a spurious drop (events built just after an off/on toggle carry the old epoch and are withdrawn at the next pump).
    2. Note whether any record is a flush trigger (`is_flush_trigger`). A sentinel sets `flush_pending` and is not passed to `source.events`.
    3. Pass the other records to `source.events(..)`. `Some(events)` is pushed with `epoch` from step 1; `None` is discarded.
    4. Only now: if a flush trigger was seen **and** `!queue.is_empty()`, call `queue.mark_urgent()`. `mark_urgent` itself stays unconditional; the caller rule keeps an opted-out `launch_finished` or a `Some(vec![])` from latching `urgent` on an empty queue.
  - `pump` runs these steps (D2: consent before `due`):
    1. If `flush_pending && !queue.is_empty()`, `queue.mark_urgent()`.
    2. **Consent.** Read `source.target()` first, then the epoch. If the target is `None`, or `!queue.is_empty() && queue.epoch() != Some(source.epoch())`: run `queue.withdraw()`, finish the flush (if one is pending, set `flush_pending = false` and `flush_done = true`) and return `None`. This runs on every pump, even under a hold or before the coalescing point, so events from before an opt-out never wait in memory.
    3. `let hold = sender.hold_remaining();`. If `!queue.due(now, hold)`, finish the flush and return `None`.
    4. `let batch = queue.take(now);` then `let report = sender.send_report(&target, &batch).await;` (the target read in step 2).
    5. `let retry = report.retry_from.map_or_else(Vec::new, |i| batch[i..].to_vec());` and `let retried = retry.len();` then `queue.give_back(retry, now);`.
    6. `lost = (batch.len() - report.posted - retried) as u64 + queue.take_lost();`
    7. If `queue.is_empty()`, or `report.outcome != SendOutcome::Sent`, finish the flush. Otherwise leave it pending, so the next pump stays urgent.
    8. Return `Some(Pumped { outcome: report.outcome, sent_events: report.posted as u64, lost_events: lost })`.
  - Steps 2 and 3 return `None` **without** calling `queue.take_lost()`. Events trimmed at the cap while held are counted at the next real send. This is intended; don't add a `take_lost` there.
  - No logging here; the shell logs through health.
- **Tests** (unit, `#[tokio::test]`, wiremock as in `launcher_session_tests.rs`). Use a `FakeSource { target: Option<LauncherTarget>, epoch: Cell<u64>, opted_in: bool, bump_on_events: bool }`. Its `events` returns `None` when `!opted_in`; otherwise, when `bump_on_events`, it adds 1 to `epoch` **before** returning, then maps each record to one `ClientLog` event. Read the queue length with `forwarder.queued()`.
  1. **A throttled event is kept, then sent.**
     - Chunk mock: 429 once (`Retry-After: 60`), then 200.
     - `on_records(1 record, t0)` and `pump(t0 + 2 s)` give `outcome = Dropped` and `lost_events = 0`; `queued() == 1`.
     - `forwarder.sender().clear_hold()`, then `pump(t0 + 10 s)` with **no new record** gives `Sent` with `sent_events = 1`.
     - The server received exactly 2 chunk posts. On today's design the second post never happens.
  2. **Withdrawn consent.** Queue 2 events at `t0`, then set the source's target to `None`: `pump(t0)` returns `None`, there is no chunk post (`.expect(0)`) and `queued() == 0`.
  3. **A new epoch.** Queue 2 events at `t0`, then bump the source's epoch: `pump(t0)` returns `None`, posts nothing and `queued() == 0`.
  4. **The sentinel.** A sentinel `LauncherRecord { target: FLUSH_TARGET, .. }` plus one ordinary record at `t0`: `next_wake(t0) == Some(ZERO)`, and `pump(t0)` posts at once. After that, `take_flush_done()` is true once.
  5. **A 403.** It gives `lost_events = 1` and `queued() == 0`.
  6. **`consent_changed()`** makes `consent_epoch()` larger: assert `after > before`, never `after == before + 1` (other engine tests call `set_game_telemetry` in parallel and bump the same global).
  7. **Epoch read first (D1).** A source with `bump_on_events = true`: `on_records(1 record, t0)`, then `pump(t0 + 2 s)` returns `None`, posts nothing (`.expect(0)`) and `queued() == 0`.
  8. **No latched urgency (D4).** A source with `opted_in = false`: `on_records` with one `client.launcher.launch` record whose `event = "launch_finished"`, at `t0`. Then set `opted_in = true` and `on_records` one `adoption` record at `t0`: `next_wake(t0) == Some(2 s)` (not ZERO), and `pump(t0)` returns `None` with no post.

  Revert proof: name the assertion each test pins. Test 1 fails if `give_back` is skipped. Tests 2 and 3 fail with the old step order (consent after `due`: at `t0` nothing is due, so the queue keeps 2). Test 7 fails if the epoch is read after `events` (the event carries the new epoch and is posted). Test 8 fails if a flush trigger marks an empty queue urgent (`next_wake` is ZERO).
- **Checks:** D1, D2e, D3e `launcher_forwarder`, D3e `game_telemetry`.
- **Docs:** none.
- **Commit:** `feat(launcher): telemetry forwarder keeps throttled events in memory and re-checks consent (LTL-04)`

### LTL-05: the shell forward loop (F1)

- **Host:** windows. **Agent:** rust-gameserver-dev (a thread and timing test). **Worktree:** `W\ltl-05`, branch `launcher-telemetry-linux/ltl-05-forward-loop`, from the `ltl-int` tip after LTL-04. **Rules:** R1-R10. **Contract:** C3 (`record_pumped`), C4, C5 (`forward_loop` only).
- **Files:**
  - `S/telemetry.rs`:
    - add `#[path = "telemetry_forwarder.rs"] pub(crate) mod forwarder;`;
    - re-implement `spawn_forwarder` on top of it, keeping the signature (R10);
    - delete `FORWARD_BATCH` only if nothing else uses it (grep first).
  - new `S/telemetry_forwarder.rs`;
  - new `S/telemetry_forwarder_tests.rs`, wired from the new file with `#[cfg(test)] #[path = "telemetry_forwarder_tests.rs"] mod tests;`;
  - `S/host/launcher_telemetry.rs` (96 lines).
- **Do:**
  1. In `launcher_telemetry.rs`, add `pub(crate) fn launcher_flush_target(&self) -> Option<LauncherTarget>`. It makes the same module check as `launcher_batch` (factor it into one private `fn module_available(&self) -> bool`) and returns `store.lock().launcher_target()`.
  2. In the same file, add `pub(crate) struct HostSource { pub host: Arc<NativeHost>, pub redaction: Redaction }`, implementing `ForwardSource`:
     - `events` is `launcher_batch(..).map(|(_, e)| e)`;
     - `target` is `launcher_flush_target()`;
     - `epoch` is `game_telemetry::launcher_forwarder::consent_epoch()`.
  3. `forward_loop` loops:
     1. Wait for records. With `forwarder.next_wake(Instant::now())` equal to `None`, use `records.recv()`. With `Some(d)`, use `records.recv_timeout(d)`; a `Timeout` is not an error.
     2. Drain up to 64 more with `try_recv`, then call `on_records`.
     3. Pump once with `tauri::async_runtime::block_on(forwarder.pump(source, Instant::now()))`. On `Some(p)`, call `on_pumped(&p)`. If `forwarder.take_flush_done()`, call `on_flushed()`.
     4. On `Disconnected`, mark one last flush (send nothing; call `on_records` with a sentinel record), pump once and return.
  4. `spawn_forwarder` builds `Forwarder::new(LauncherSender::new())` and a `HostSource`. Its `on_pumped` calls `record_pumped(&health(), &p.outcome, p.sent_events, p.lost_events, now_ms)`. Its `on_flushed` is a no-op until LTL-06.
  5. On `record_pumped` in `S/telemetry.rs` (about `:209`), delete the two lines LTL-03 left for you: `#[allow(dead_code)]` and the `// LTL-05 calls this` comment. D2s must then pass with no `dead_code` warning; if something else is still unused, report it rather than adding a new `allow`.
  6. `HostSource` adds no ordering of its own: `Forwarder::on_records` reads `epoch()` before `events()` (C4). Don't cache the epoch or the target in `HostSource`; each call reads them fresh.
- **Tests** (unit, `telemetry_forwarder_tests.rs`, plain `#[test]`). Start wiremock with `tauri::async_runtime::block_on(MockServer::start())`, and run `forward_loop` on a `std::thread` with a fake `ForwardSource`. Build its target the way `E/launcher_session_tests.rs::target` does; if a type is not reachable from the shell, return `blocked` and name it.
  1. **Coalesced.** Send three records within 500 ms (`adoption`, `adoption`, then `client.launcher.launch` with `event = "launch_finished"`) and drop the sender. Exactly **one** chunk post is received, and it holds 3 rows (read `received_requests()`; gunzip if the body is gzip, as the existing tests do). On today's loop this is 2 or 3 posts.
  2. **No flush trigger.** One `adoption` record, then wait 2.5 s (allowed: the COALESCE wait). One post arrives, and none before 1.9 s.
  3. In `launcher_telemetry.rs` tests: `launcher_flush_target` is `None` without the module, `None` before opt-in, and `Some` after it (copy `a_batch_needs_the_module_and_the_opt_in`'s setup).

  Revert proof: test 1 fails on the old `spawn_forwarder` loop shape (one send per burst), and test 3 fails because the fn is new. For step 5, report the `Grep "LTL-05 calls this"` result (no match) and that D2s is clean.
- **Checks:** D1, D2s, D3s `telemetry`, D3s `launcher_telemetry`.
- **Docs:** none.
- **Commit:** `feat(launcher): coalesce launcher telemetry sends and resend after a hold (LTL-05)`

### LTL-06: the exit flush (F2)

- **Host:** windows. **Agent:** rust-gameserver-dev. **Worktree:** `W\ltl-06`, branch `launcher-telemetry-linux/ltl-06-exit-flush`, from the `ltl-int` tip after LTL-05. **Rules:** R1-R10. **Contract:** C5; D-LTL6.
- **Files:**
  - `S/telemetry_forwarder.rs` and its tests;
  - `S/telemetry.rs`: one line in `init` (`:44`), `forwarder::remember_sender(tx.clone());`, and in `spawn_forwarder` the `on_flushed` closure becomes `FLUSH_DONE.notify()`;
  - `S/main.rs` (672 lines: call lines only, at most 8 changed lines, conflict surface with #2031).
- **Do:**
  1. Implement `FlushSignal`, `request_flush`, `remember_sender`, `flush_on_exit` and the statics `FLUSH_TX: OnceLock<SyncSender<LauncherRecord>>` and `FLUSH_DONE: FlushSignal` (a `LazyLock`, or a `OnceLock` behind a getter).
  2. `request_flush`:
     1. read the counter;
     2. `try_send` the sentinel `LauncherRecord { ts_ms: 0, level: "info".into(), target: FLUSH_TARGET, fields: Map::new() }` (return false on `Err`);
     3. wait on the condvar until the counter moves past its value or `wait` elapses (`wait_timeout_while`).
  3. In `main.rs`, change `.run(tauri::generate_context!()).expect(..)` (`:670-671`) to:
     ```rust
     .build(tauri::generate_context!())
     .expect("desktop launcher could not initialize")
     .run(|_, event| {
         if let tauri::RunEvent::Exit = event {
             telemetry::forwarder::flush_on_exit();
         }
     });
     ```
     Keep the expect text the same.
- **Tests** (unit, `telemetry_forwarder_tests.rs`):
  1. Run `forward_loop` on a thread, with wiremock and the fake source, and with `on_flushed` calling a test-local `FlushSignal::notify`. Send one `adoption` record, wait 100 ms, then call `request_flush(&tx, &signal, Duration::from_secs(3))`. It must return `true` in under 1.5 s, with exactly one chunk post received before it returned. On revert there is no sentinel handling, so the call returns `false` at 3 s.
  2. Under a running hold, `request_flush` returns `true` in under 500 ms with no post. Mount 429 `Retry-After: 600` first and pump one event to get the hold.
  3. `request_flush` on a channel whose receiver was dropped returns `false` at once.

  Revert proof: tests 1 and 2 fail without the sentinel acknowledgement.
- **Checks:** D1, D2s, D3s `telemetry`, E2E. If E2E's only failure is `host::repair::integration_tests::retained_host_handoff_commits_signed_seed_and_reports_current_backup_after_cleanup` (failing on the wave 1 tip without any change under `host/repair`; wave 1 review D6), report it as pre-existing and return `done`; don't touch `host/repair`.
- **Docs:** none.
- **Commit:** `feat(launcher): flush launcher telemetry on exit, bounded at 3 s (LTL-06)`

### LTL-07: docs and disclosure (F1-F4)

- **Host:** windows. **Agent:** packet-coder. **Worktree:** `W\ltl-07`, branch `launcher-telemetry-linux/ltl-07-docs`, from the `ltl-int` tip after LTL-06 and LTL-B2 (both this packet and B2 edit `docs/operations/telemetry.md`). **Gate:** D-LTL3 decided (use the owner's text exactly). **Rules:** R1-R10. All the docs here are CRLF (R4).
- **Files:**
  - `crates/launcher/desktop/docs/privacy-inventory.md`: row "Launcher forwarding" (`:97`), using the D-LTL3 text.
  - `crates/launcher/desktop/frontend/ui/index.html:91`: the same text, as one `<li>`. Only that line.
  - `crates/launcher/desktop/docs/launch-telemetry.md` (`:148`, the `pipeline_recovered` section):
    - the new field `events_lost`;
    - the coalescing (2 s or 64 events, at least 7 s apart);
    - the hold queue (500, oldest dropped first, in memory);
    - the flush on `launch_finished` and on exit (at most 3 s);
    - which failures retry (D-LTL5).
  - `docs/operations/telemetry-upload-endpoint.md:353` (item 7): add `events_lost` to the field list.
  - `docs/operations/telemetry.md` (the launcher limits table, about `:553`): one note that the launcher paces itself under the 10 per minute session quota (D-LTL1).
  - `crates/admin-api/src/routes/telemetry/upload_gate.rs`: the `DEFAULT_LAUNCHER_CHUNK_PER_SESSION` comment only. The launcher now sends at most one chunk about every 7 s, plus flushes; the old text said "about one every 2 s".
- **Do:** edit only those lines and sections. Then run `python tools/docs-gen/regen.py --check`; if it reports a stale block in a file you touched, run it without `--check` and say so.
- **Tests:** none (docs). Instead, `git diff --stat` shows CRLF-clean diffs, and F1 passes (the `index.html` change; `privacy-disclosure.test.ts` must still pass).
- **Checks:** F1 (`npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend`, then `npm run check --prefix crates/launcher/desktop/frontend`, then `npm test --prefix crates/launcher/desktop/frontend`), S1 (fmt only for `upload_gate.rs`), and `python tools/docs-gen/regen.py --check`.
- **Docs rows** (doc-update-map): "Desktop launcher workflows" (privacy inventory) and "Dev-session telemetry pipeline" (`launch-telemetry.md`, `telemetry.md`, `telemetry-upload-endpoint.md`). List them in the report.
- **Commit:** `docs(launcher): describe coalesced, held and flushed launcher telemetry (LTL-07)`

### LTL-B2: record the stream under the forwarded caller (F5, F7)

LTL-B1 confirmed H1 (`worknotes/LTL-B1.md`); its spec changes 1 and 2 are folded in below (step 4, tests 4 and 5) and change 3 into LTL-08.

- **Host:** windows. **Agent:** packet-coder, with a parallel `network-security-auth` review pass. **Worktree:** `W\ltl-b2`, branch `launcher-telemetry-linux/ltl-b2-stream-peer`, from the `ltl-int` tip after LTL-B2b. Merges into `ltl-int` (D-LTL7) before LTL-07 branches. **Gate:** D-LTL4 decided. **Rules:** R1-R10 (R5: the forwarded address is never logged or put in a field).
- **Files:**
  - `D/handlers.rs` (544 lines: call lines and log-field lines only);
  - `D/forwarded_for.rs` (doc comment only);
  - `D/account_name_tests.rs`, or a new `D/stream_peer_tests.rs` wired the way `account_name_tests` is (grep its `mod` line);
  - `crates/auth/src/auth/client_telemetry/mod.rs` (module doc `:4-8` only);
  - `crates/base-world-entry/src/base/world_entry/map_loaded.rs:244` (the trailing comment only);
  - `docs/architecture/dev-session-telemetry.md` (the `peer_ip` sentence);
  - `docs/operations/telemetry.md` (§ No address in any log: one sentence saying the in-memory registry keys on the forwarded caller and that this is never logged).
- **Do:**
  1. Pass `rate_ip` instead of `peer_ip` in two places: `launcher_kind::record_unless_launcher(launcher, &claims, rate_ip)` (`:362`), and `record_stream(&new_claims, rate_ip)` (`:481`). Rename the parameter of `record_stream` (`:505`) and of `record_unless_launcher` (`launcher_kind.rs:209`, its parameter name only) to `caller_ip`. `launcher_kind.rs` is then also a named file.
  2. In the `rate_limit_ip` doc in `forwarded_for.rs`, replace "or `StreamRecord` (D-LT18)" with: "It is also the `StreamRecord.peer_ip` of the in-memory client-telemetry registry (D-LTL4), which is never logged or written."
  3. The auth module doc and the `map_loaded.rs` comment: "a truncated SHA-256 of the Windows `MachineGuid`, or of the host name in the desktop launcher".
  4. Add two boolean fields to the info lines `"Minted dev-session telemetry token"` (`:376`) and `"Refreshed dev-session telemetry token"` (`:488`): `caller_forwarded = rate_ip != peer_ip` and `peer_loopback = peer_ip.is_loopback()`. Booleans only; no address in any field (R5). They give the Deck re-run SigNoz proof of the proxy hop.
- **Tests** (unit, admin-api `--lib`; follow `account_name_tests.rs`: `EnvGuard::install()`, `policy(10,10,10)`, `NOW_UNIX`, `192.0.2.0/24` addresses, a unique machine id per test; capture rows with `super::tests::capture`):
  1. Call `mint_with_headers` with peer `127.0.0.1` and the header `X-Forwarded-For: 192.0.2.41`, then `client_telemetry_for(None, Some(192.0.2.41), NOW_UNIX)`. It is `Present`, and the lookup at `127.0.0.1` is not that `session_id`.
  2. The same through `refresh_with_headers`. Mint from `192.0.2.42` directly, then refresh via loopback with `X-Forwarded-For: 192.0.2.43`. The stream is `Present` at `.43`.
  3. **Spoof guard.** A non-loopback peer `192.0.2.44` with `X-Forwarded-For: 192.0.2.45` is `Present` at `.44` only.
  4. **Fallbacks.** A loopback peer with no `X-Forwarded-For` records the stream under `127.0.0.1`; a loopback peer with a malformed header (`X-Forwarded-For: not-an-ip`) records it under `127.0.0.1` too.
  5. **Log fields.** Under `capture`, test 1's mint emits one `Minted dev-session telemetry token` row with `caller_forwarded = "true"` and `peer_loopback = "true"`, and no field of that row contains `192.0.2`. A direct mint from `192.0.2.42` gives `caller_forwarded = "false"`, `peer_loopback = "false"`.

  Revert proof: tests 1 and 2 fail on today's `peer_ip` argument (the stream sits at loopback); test 5 fails because the fields are new. Test 4 documents the fallback and passes before and after; say so.
- **Checks:** S1, S2, S3 `dev_session`, S4.
- **Docs rows:** "Dev-session telemetry pipeline" (`dev-session-telemetry.md`, `telemetry.md` § No address in any log).
- **Commit:** `fix(admin-api): record the client telemetry stream under the proxied caller (LTL-B2)`

## Wave 3

### LTL-08: close-out

- **Host:** windows. **Agent:** documentation-writer. **Worktree:** `W\ltl-08`, from the `ltl-int` tip after the campaign review's fixes have merged in; it merges into `ltl-int` and ships in the one campaign PR (D-LTL7). The UAT rows are owed after that PR is deployed. **Rules:** R1-R4, R9.
- **Do:**
  1. In `docs/guides/unified-uat.md`, add a `launcher-telemetry-linux` section with the steps the owner runs on the Deck. Each row has an action and an expected result:
     - **LTL-U1.** Adopt or install, then Play, then quit. Expected: SigNoz has every `client.launcher.*` row of the run, and `pipeline_recovered` has no `events_lost`. Also note any `held` count in `pipeline_recovered`: a send of up to 200 events can be two or three chunks, and at 7 s spacing that can briefly exceed the 10 per minute quota (a 429, a 120 s hold and a retry, not a loss; wave 1 review D8).
     - **LTL-U2.** World entry. Expected: `client_telemetry="present"`, the `client_machine_id` is not `e3b0c44298fc1c14`, and the Deck's mint row has `caller_forwarded=true peer_loopback=true`. Run it with the Windows launcher closed: two live streams behind the same NAT address give `ambiguous`, not `present` (LTL-B1).
     - **LTL-U3.** Turn diagnostics off during a hold. Expected: no later rows.
     - **LTL-U4.** The Deck walk test for movement (parked item; owner).
  2. Update this README's packet table.
  3. Add F8's observation: the 300-row budget, against the counts LTL-U1 sees.
  4. Don't touch the status docs unless the campaign review says a line is owed.
  5. If the shell test `host::repair::integration_tests::retained_host_handoff_commits_signed_seed_and_reports_current_backup_after_cleanup` was shown to fail on `origin/main` too (wave 1 review D6), list it in the README as a known flake, with its issue number if `ops` filed one.
- **Checks:** `python tools/docs-gen/regen.py --check`; CRLF.
- **Commit:** `docs(launcher): close out launcher-telemetry-linux (LTL-08)`
