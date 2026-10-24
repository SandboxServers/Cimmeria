# Launcher telemetry: install packets (amendment 6)

Wave 3 packets LT-14a and LT-14b, split out of [work-packets-coverage.md](work-packets-coverage.md) by amendment 6 ([LT-14-amendment.md](LT-14-amendment.md)). Campaign rules R1-R10 and contracts C1-C18 are in [work-packets.md](work-packets.md); each packet below inlines the rules it needs.

### LT-14a: install error codes and core retarget (amendment 6)

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-14` (exists, unedited at `3d10ddf4d`), branch `launcher-telemetry/lt-14-install`. LT-14a commits first. LT-14b then runs in the same worktree on the same branch, and the branch merges once after both have been reviewed. **Target:** `launcher.install`. **Contract:** C5, C6 (amendment 6).
- **Rules (inline, because packet-coder skips CLAUDE.md):**
  1. Start every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-14;`. Use PowerShell only and run everything in the foreground. Run cargo only through `pwsh tools/build-lane/lane.ps1`. Every clippy and test check passes `--locked`. Allow at most 5 fix rounds per check; after that, return `blocked` with the last `status=` line.
  2. Edit only the files below. Make one commit with the subject below, and end the message with the Co-Authored-By and Claude-Session lines from your brief.
  3. Files have a soft cap of 500 lines and a hard cap of 700. `install.rs` is 575 lines, so change only the four statements named here.
  4. Fields and messages may not contain a path, URL, host, IP, port, account, token or session id. Never format an error value with `%e`, `?e` or `{error}`. Use closed snake_case codes only.
  5. Logging never changes control flow or a return value.
  6. Out of scope, so leave these alone:
     - `install.rs` :269 (`?report`: its target is not forwarded), :319 (a path, per patch, kept local) and :565;
     - `install_seed.rs` :52 (a path);
     - `unpack/mod.rs`. Its `println!` at :356 sits inside an `#[ignore]` manual test that needs stdout, so that item is dropped from this packet.
  7. If code here does not match a line or type named below, and the fix is not mechanical (an import path, a `&`), return `blocked` and name the line. Do not redesign (R9).
- **Files:**
  - new `crates/launcher-core/src/install_codes.rs` (≤120 lines);
  - `crates/launcher-core/src/lib.rs` (32 lines): add `mod install_codes;` on a new line after :19 (`pub mod install_report;`);
  - `crates/launcher-core/src/install.rs` :223, :236, :257 and :271 (statement-level edits only);
  - `crates/launcher-core/src/install_seed.rs` :184.
- **Change:**
  1. `install_codes.rs`. The `{ .. }` patterns match unit, tuple and struct variants alike, so do not change them:
     ```rust
     //! Closed snake_case codes for core install errors in launcher events (C6).
     use crate::install::InstallError;
     use crate::seed_source::LocalSeedError;

     impl InstallError {
         pub fn code(&self) -> &'static str {
             match self {
                 Self::Http { .. } => "http",
                 Self::Io { .. } => "io",
                 Self::Unpack { .. } => "unpack",
                 Self::State { .. } => "state",
                 Self::HashMismatch { .. } => "hash_mismatch",
                 Self::UnexpectedStatus { .. } => "unexpected_status",
                 Self::InvalidSha256 { .. } => "invalid_sha256",
                 Self::Cancelled => "cancelled",
                 Self::SeedSwitched => "seed_switched",
                 Self::LocalSeed { .. } => "local_seed",
                 Self::SeedExtractionUncertain => "seed_extraction_uncertain",
                 Self::PatchDest { .. } => "patch_dest",
                 Self::PatchesFailed { .. } => "patches_failed",
             }
         }
         /// The HTTP status behind the error, when there is one. Never the URL.
         pub fn http_status(&self) -> Option<u16> {
             match self {
                 Self::UnexpectedStatus { status, .. } => Some(*status),
                 Self::Http(error) => error.status().map(|s| s.as_u16()),
                 _ => None,
             }
         }
     }

     impl LocalSeedError {
         pub fn code(&self) -> &'static str {
             match self {
                 Self::NotAFile { .. } => "not_a_file",
                 Self::SizeMismatch { .. } => "size_mismatch",
                 Self::HashMismatch { .. } => "hash_mismatch",
                 Self::Changed { .. } => "changed",
                 Self::Cancelled { .. } => "cancelled",
                 Self::Io { .. } => "io",
             }
         }
     }
     ```
     If `crate::seed_source` is not the module path of `LocalSeedError`, use the path `install_seed.rs` imports it from.
  2. Retarget in `install.rs`. Each replacement replaces the whole statement:
     - :223 becomes `info!(target: "launcher.install", event = "bundled_paks_moved", "moved the bundled cooked-data PAKs into SourceCache");`
     - :236 becomes `warn!(target: "launcher.install", event = "patch_skipped", patch = %patch.id, reason = "dependency_failed", "patch skipped: a patch it builds on did not apply");`. The `after` field goes. `patch` is a signed-manifest patch id, which C6 now allows.
     - :257 becomes `warn!(target: "launcher.install", event = "patch_failed", patch = %patch.id, reason = e.code(), http_status = e.http_status(), "patch not applied");`. `error = %e` goes, because it can print a URL. The `report.push_failure` and `failures.push` lines below it stay as they are.
     - :271 becomes `warn!(target: "launcher.install", event = "game_exe_missing", "SGW.exe missing after install");`
  3. `install_seed.rs` :184 becomes `warn!(target: "launcher.install", event = "seed_copy_failed", reason = error.code(), "the chosen seed archive could not be copied; the next attempt downloads it");`
- **Tests:** inline `#[cfg(test)] mod tests` in `install_codes.rs`:
  - `unexpected_status_reports_its_code_and_status`: `InstallError::UnexpectedStatus { status: 503, url: "https://example.invalid/blob".into() }` gives `"unexpected_status"` and `Some(503)`;
  - `cancelled_has_no_status`: `InstallError::Cancelled` gives `"cancelled"` and `None`;
  - `io_is_io`: `InstallError::Io(std::io::Error::other("x")).code() == "io"`;
  - `local_seed_codes`: `LocalSeedError::SizeMismatch { .. }` gives `"size_mismatch"`. Build it with the fields the variant has; read `seed_source.rs` for them.

  Reverting `code()` breaks the build of these tests. No capture test in core: `crates/launcher-core/Cargo.toml` has no `tracing-subscriber` (coder finding 4, accepted). LT-14b's real-path test reads the codes back through the engine.
- **Checks (in order):**
  - C1 `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-launcher-core`;
  - C2 `pwsh tools/build-lane/lane.ps1 cargo clippy --locked -p cimmeria-launcher-core --all-targets -- -D warnings`;
  - C3 `pwsh tools/build-lane/lane.ps1 cargo test --locked -p cimmeria-launcher-core install_codes`;
  - C3 `pwsh tools/build-lane/lane.ps1 cargo test --locked -p cimmeria-launcher-core install`.
- **Docs:** none (LT-26).
- **Commit:** `LT-14a: core install error codes and launcher.install core events`.

### LT-14b: install lifecycle events, operation numbers and engine error codes (amendment 6)

- **Host:** windows. **Worktree:** the same `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-14`, on branch `launcher-telemetry/lt-14-install`, after LT-14a's commit. **Target:** `launcher.install`. **Contract:** C5, C6 (amendment 6), C9. **Needs:** LT-14a (`InstallError::code`, `http_status`).
- **Rules (inline):**
  1. Start every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-14;`. Use PowerShell only and run everything in the foreground. Run cargo only through `pwsh tools/build-lane/lane.ps1`. Every clippy and test check passes `--locked`. Allow at most 5 fix rounds per check; after that, return `blocked` with the last `status=` line.
  2. Edit only the files below. Make one commit with the subject below, and end it with the Co-Authored-By and Claude-Session lines from your brief.
  3. Files have a soft cap of 500 lines and a hard cap of 700. `install_worker/mod.rs` is 504 lines and takes call lines only; it may grow by 25 lines at most.
  4. Fields and messages may not contain a path, URL, host, IP, port, account, token, session id or operation UUID. Never format an error value with `%e` or `?e`. Use closed codes only.
  5. Every `tracing::` call lives in `install_worker/events.rs` and uses the literal `target: "launcher.install"` and an `event =` field. Use info for transitions and warn for failures. Logging never changes control flow or a return value.
  6. Tests use `crate::test_capture::capture(|| ...) -> (R, Vec<Captured>)`, with `Captured { target, level, fields }` and `.field(key)`.
     - Capture is thread-local, so async code runs inside the closure on `tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()` through `block_on`.
     - Integers read back as strings (`"503"`).
     - Capture panics if a `launcher*` event holds an IP. Fixture URLs use `example.invalid`.
     - Each new test must fail when the change is reverted.
  7. If code here does not match a line or type named below, and the fix is not mechanical, return `blocked` and name the line. Do not redesign.
- **Files** (paths under `crates/launcher/desktop/`):
  - new `engine/src/operation_numbers.rs` (≤90 lines);
  - new `engine/src/error_codes.rs` (≤140 lines);
  - `engine/src/lib.rs` (75 lines): after :3 (`pub use operations::*;`) add `mod error_codes;` and `pub mod operation_numbers;`;
  - new `engine/src/storage/install_worker/events.rs` (≤230 lines, with its inline tests);
  - `engine/src/storage/install_worker/mod.rs` (call lines only, listed below);
  - `engine/src/storage/install_worker/telemetry_tests.rs` (195 lines; adds one test);
  - `shell/src/host/install/mod.rs` :301.
- **Change:**
  1. `operation_numbers.rs` is the C6 `operation` source. It is shared: LT-16 and LT-17 reuse it.
     ```rust
     //! Process-local operation numbers for launcher events (C6). The operation
     //! UUID is a durable identifier and never leaves the process (R5); every
     //! event of one operation carries the same small number instead.
     use std::sync::atomic::{AtomicU64, Ordering};
     use std::sync::Mutex;
     use std::time::Instant;
     use uuid::Uuid;

     const KEEP: usize = 64;
     static NEXT: AtomicU64 = AtomicU64::new(1);
     static SEEN: Mutex<Vec<(Uuid, u64, Instant)>> = Mutex::new(Vec::new());

     fn entry(id: Uuid) -> (u64, Instant) {
         let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
         if let Some(&(_, number, first)) = seen.iter().find(|(known, _, _)| *known == id) {
             return (number, first);
         }
         if seen.len() == KEEP {
             seen.remove(0);
         }
         let made = (NEXT.fetch_add(1, Ordering::Relaxed), Instant::now());
         seen.push((id, made.0, made.1));
         made
     }

     /// The number for `id`, assigned the first time this process logs it.
     pub fn number(id: Uuid) -> u64 {
         entry(id).0
     }

     /// Milliseconds since `number` (or this call) first saw `id`.
     pub fn elapsed_ms(id: Uuid) -> u64 {
         u64::try_from(entry(id).1.elapsed().as_millis()).unwrap_or(u64::MAX)
     }
     ```
     Inline tests (parallel-safe, so they never assert a specific number):
     - `the_same_id_keeps_its_number`: two calls with one `Uuid::new_v4()` are equal.
     - `different_ids_get_different_numbers`.
     - `an_old_id_is_forgotten_after_keep_others`: take `a`, then 64 fresh ids; `number(a)` now differs from the first value.
  2. `error_codes.rs`: `impl ContractError`, `impl StorageError` and `impl IntentError`, each with `pub fn code(self) -> &'static str`. Use `use crate::{ContractError, IntentError, StorageError};`. The codes are the snake_case variant names:
     - `ContractError`: `unsupported_schema`, `invalid_revision`, `stale_revision`, `busy`, `identity_conflict`, `unknown_operation`, `invalid_transition`, `persistence_failed`, `persistence_uncertain`.
     - `StorageError`: `in_use`, `io`, `corrupt`, `unsupported_schema`, `too_large`, `unsafe_file`, `invalid_directory`, `stale_revision`, `busy`, `persistence_uncertain`, `invalid_server_list`.
     - `IntentError`: `LauncherTooOld` gives `"launcher_too_old"`, `Storage(e)` gives `e.code()` and `Operation(e)` gives `e.code()`.

     Inline tests:
     - `codes_match_the_serde_names`: for every `ContractError` and `StorageError` variant, `serde_json::to_value(e).unwrap() == serde_json::Value::from(e.code())`;
     - `intent_errors_use_the_inner_code`: `LauncherTooOld` gives `launcher_too_old`, `Storage(TooLarge)` gives `too_large` and `Operation(Busy)` gives `busy`.
  3. `install_worker/events.rs`, declared `pub mod events;` on a new line after mod.rs :496. Every macro uses `target: "launcher.install"`, and each helper derives `operation` as `crate::operation_numbers::number(id)`:
     - `pub(crate) fn install_started(id: Uuid, resumed: bool)`: info, `event = "install_started"`, `operation`, `action` (`"resume"` if `resumed`, else `"install"`).
     - `pub(crate) fn install_finished(id: Uuid, outcome: super::Outcome)`, with `duration_ms = crate::operation_numbers::elapsed_ms(id)`:
       - `ContentPrepared` logs info `install_finished` with `outcome = "ok"`;
       - `Cancelled` logs info `install_finished` with `outcome = "cancelled"`;
       - every other outcome logs warn `install_failed` with `reason` set to its code: `destination_unavailable`, `install_failed`, `content_invalid`, `rosetta_required`, `runtime_unavailable` or `reconciliation_required`. Put that mapping in a private `fn outcome_code(Outcome) -> &'static str`.
     - `pub(crate) struct PhaseLog { id: Uuid, seen: [bool; 2] }`, with `pub(crate) fn new(id: Uuid) -> Self` and `pub(crate) fn enter(&mut self, phase: TimedPhase)`:
       - `TimedPhase::Download` maps to slot 0 and `"download"`; `TimedPhase::Extraction` maps to slot 1 and `"extraction"`;
       - if the slot was already set, return;
       - otherwise set it and log info `install_phase` with `operation` and `phase`.

       That is at most 2 per run: a patch reopens `download`, and that is not logged again. Import `use crate::launcher_summary::TimedPhase;`, the same path mod.rs uses.
     - `pub(crate) fn install_reconcile_unknown(id: Uuid)`: warn, with `operation`. The worker task panicked, so the outcome is unknown.
     - `pub(crate) fn install_seed_forget_failed(id: Uuid)`: warn, with `operation`.
     - `pub(crate) fn install_stage_failed(id: Uuid, error: &crate::install::InstallError)`: warn, with `operation`, `reason = error.code()` and `http_status = error.http_status()`.
     - `pub fn mark_uncertain_failed(error: crate::ContractError)`: warn, with `reason = error.code()` and no operation. It is `pub` because the shell calls it.
  4. `install_worker/mod.rs` call lines. Line numbers are at base `3d10ddf4d`, so apply from the bottom up:
     - :387 `Err(_) => return Outcome::InstallFailed,` becomes `Err(error) => { events::install_stage_failed(intent.operation_id, &error); return Outcome::InstallFailed; }`.
     - :311 and :296 `let _ = operations.mark_uncertain(id);` each become `if let Err(error) = operations.mark_uncertain(id) { events::mark_uncertain_failed(error); }`.
     - Inside `watch_phases`: after :245 (`let mut entered = None;`) add `let mut phases = events::PhaseLog::new(id);`, and after :255 (the `}` closing the `continue` block) add `phases.enter(phase);`.
     - Between :226 (`let published = publish(...)`) and :227 (`result.send_replace(...)`), add `events::install_finished(id, published);`. `Outcome` is `Copy`.
     - :223 `tracing::warn!("forgetting the unusable seed archive failed");` becomes `events::install_seed_forget_failed(id);`.
     - :214 `.unwrap_or((Outcome::ReconciliationRequired, None));` becomes `.unwrap_or_else(|_| { events::install_reconcile_unknown(id); (Outcome::ReconciliationRequired, None) });`.
     - Before :150 (`runtime.spawn(watch_phases(...))`) add `events::install_started(id, input.ownership.is_some());`. `input` is moved only at :152. `ownership` is `Some` only for a resume (`resume/mod.rs:84`).
  5. Shell `host/install/mod.rs` :301 `let _ = operations.mark_uncertain(id);` becomes `if let Err(error) = operations.mark_uncertain(id) { install_worker::events::mark_uncertain_failed(error); }`. `install_worker` is already imported at :6.

  `files` and `bytes` are dropped from the install events: the install path has no counts. The install-result row (`Run::Install`) already carries the report. `error_kind` is dropped too, because no install-path error here is an `io::Error`.
- **Tests:**
  - Inline in `events.rs`, each under `capture`, all on `target == "launcher.install"`:
    - `started_and_finished_share_one_operation`: `install_started(id, false)`, then `install_finished(id, Outcome::ContentPrepared)`. Expect:
      - `install_started` with action `install`;
      - `install_finished` at INFO, with outcome `ok` and a `duration_ms` field;
      - equal `operation` values.
    - `a_failed_outcome_is_a_warn_with_its_code`: `ContentInvalid` gives WARN `install_failed` with reason `content_invalid`.
    - `a_cancel_is_an_info_finish`: INFO `install_finished`, outcome `cancelled`.
    - `a_resume_sets_the_action`: action `resume`.
    - `each_phase_is_logged_once`: enter Download, Download, Extraction, Download gives exactly 2 `install_phase` events, `download` then `extraction`.
    - `a_stage_failure_carries_its_code_and_status`: `install_stage_failed(id, &InstallError::UnexpectedStatus { status: 503, url: "https://example.invalid/blob".into() })` gives reason `unexpected_status` and http_status `"503"`, and no field value contains `example.invalid`.
    - `mark_uncertain_failed_carries_the_contract_code`: `ContractError::PersistenceFailed` gives reason `persistence_failed`.
  - Real failing path: add `#[test] fn a_failed_download_logs_the_install_lifecycle()` to `telemetry_tests.rs`.
    1. Build a current-thread runtime `rt`.
    2. Run `let (worker_outcome, events) = crate::test_capture::capture(|| rt.block_on(async { ... }));`. The async block runs the body of `a_failed_download_queues_an_error_result` (:81-94) and returns `outcome(worker.result.clone()).await`.
    3. Assert `worker_outcome == Outcome::InstallFailed`.
    4. Keep only the events with `target == "launcher.install"`, and assert:
       - an `install_started` with action `install`;
       - an `install_stage_failed` with http_status `"404"`. The wiremock server answers 404 for the unmounted seed path, and core :410 turns that into `UnexpectedStatus`;
       - a WARN `install_failed` with reason `install_failed`;
       - all three carry the same `operation`.

    Reverting `events::install_finished` or `install_started` fails this test.

    Stop conditions:
    - If `install_stage_failed` has a different `operation` from `install_started`, `intent.operation_id` is not the worker id. Drop the `id` argument from `install_stage_failed` (log it without `operation`), drop that one assertion, and say so in the report.
    - If capture panics on an IP from an event this packet did not add, return `blocked` and name that event.
- **Checks (in order):**
  - D1 `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all`;
  - D2e `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings`;
  - D2s, the same with `-p cimmeria-launcher-desktop`;
  - D3e `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine operation_numbers`;
  - D3e … `error_codes`;
  - D3e … `install_worker`;
  - D3s `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop install`.
- **Docs:** none (LT-26).
- **Commit:** `LT-14b: install lifecycle events, operation numbers and engine error codes`.
