# Launcher telemetry: repair and launch packets (amendment 6)

Wave 3 packets LT-16a, LT-16b, LT-17a and LT-17b, re-sliced out of [work-packets-coverage.md](work-packets-coverage.md) by amendment 6 ([LT-14-amendment.md](LT-14-amendment.md)). They use `crate::operation_numbers` and the `code()` functions that LT-14b adds. Campaign rules R1-R10 and contracts C1-C18 are in [work-packets.md](work-packets.md); each packet below inlines the rules it needs.

### LT-16a: repair and uninstall events (amendment 6)

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-16`, branch `launcher-telemetry/lt-16-repair`. `ops` creates it from the `lt-app-integration` tip after the LT-14 branch (LT-14a and LT-14b) has merged there. LT-16b follows in the same worktree, and the branch merges once after both are reviewed. **Target:** `launcher.repair`. **Contract:** C5, C6 (amendment 6), C9. **Needs:** LT-14b (`crate::operation_numbers`, `code()` on `ContractError`, `StorageError` and `IntentError`) and LT-14a (`InstallError::code`, `http_status`).
- **Rules (inline):**
  1. Start every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-16;`. Use PowerShell only and run everything in the foreground. Run cargo only through `pwsh tools/build-lane/lane.ps1`. Every clippy and test check passes `--locked`. Allow at most 5 fix rounds per check, then return `blocked` with the last `status=` line.
  2. Edit only the files below. Make one commit with the subject below, and end it with the Co-Authored-By and Claude-Session lines from your brief.
  3. Files have a soft cap of 500 lines and a hard cap of 700. `uninstall/mod.rs` is 491 lines and takes call lines only; it may grow by 15 lines at most.
  4. Fields and messages may not contain a path, URL, host, IP, port, account, token, session id or operation UUID. Never format an error value with `%e` or `?e`. `operation` is always `crate::operation_numbers::number(id)`.
  5. Every `tracing::` call lives in `storage/repair/events.rs`, with the literal `target: "launcher.repair"` and an `event =` field. Use info for transitions and warn for failures. Logging never changes control flow or a return value.
  6. Tests use `crate::test_capture::capture(|| ...)`. It is thread-local, so run only synchronous calls inside it, or `block_on` a current-thread runtime. Integers and booleans read back as strings (`"false"`). Each new test must fail when the change is reverted.
  7. If code here does not match a line or type named below, and the fix is not mechanical, return `blocked` and name the line. Do not redesign.
- **Files** (paths under `crates/launcher/desktop/`):
  - new `engine/src/storage/repair/events.rs` (≤220 lines with its tests), declared `pub mod events;` on a new line after `pub mod commit;` in `storage/repair/mod.rs` (247 lines);
  - `engine/src/storage/repair/preparation/mod.rs` (365 lines);
  - `engine/src/storage/repair/commit/mod.rs` (233 lines) :80;
  - `engine/src/storage/uninstall/mod.rs` (call lines only);
  - `engine/src/storage/repair/preparation/tests.rs` (166 lines; one new test);
  - `engine/src/storage/uninstall/tests.rs` (263 lines; one new test);
  - `shell/src/host/repair/mod.rs` :290.
- **Why the original LT-16 changed:** repair has no verify step. It rebuilds through `install::install_all` into a stage, then checks `install_worker::content_valid` (preparation :255). No file, byte or bad-file count exists, so `verify_*`, `files` and `bytes` are dropped.
- **Change:**
  1. Helpers in `events.rs`, each taking `id: Uuid` and logging `operation = crate::operation_numbers::number(id)`:

     | Helper | Level | Event | Fields |
     |---|---|---|---|
     | `repair_started(id)` | info | `repair_started` | `operation` |
     | `repair_claim_failed(id, error: crate::StorageError)` | warn | `repair_claim_failed` | `operation`, `reason = error.code()` |
     | `repair_stage_failed(id, error: &crate::install::InstallError)` | warn | `repair_stage_failed` | `operation`, `reason = error.code()`, `http_status = error.http_status()` |
     | `repair_content_invalid(id)` | warn | `repair_content_invalid` | `operation` |
     | `repair_finished(id)` | info | `repair_finished` | `operation`, `outcome = "ok"`, `duration_ms = crate::operation_numbers::elapsed_ms(id)` |
     | `repair_ended(id, failure: &super::preparation::Failure)` | `Cancelled`: info; otherwise warn | `Cancelled`: `repair_finished`; `Failed` and `ReconciliationRequired`: `repair_failed` | `Cancelled`: `outcome = "cancelled"`; otherwise `reason = "failed"` or `"reconciliation_required"`; always `duration_ms` |
     | `pub fn mark_uncertain_failed(error: crate::ContractError)` | warn | `mark_uncertain_failed` | `reason = error.code()` (no operation; `pub` for the shell) |
     | `uninstall_started(id)` | info | `uninstall_started` | `operation` |
     | `uninstall_finished(id, recovering: bool)` | info | `uninstall_finished` | `operation`, `outcome = "ok"`, `recovering`, `duration_ms` |
     | `uninstall_failed(id, reason: &'static str)` | warn | `uninstall_failed` | `operation`, `reason` |
     | `uninstall_kept_adopted_folder()` | info | `uninstall_kept_adopted_folder` | none |

     All are `pub(crate)` except `mark_uncertain_failed`.
  2. `repair/preparation/mod.rs`, applied from the bottom up:
     - :356-357: as the first line inside `if open && ended {`, add `events::repair_ended(id, &finished);`. One terminal event per run: only the call that ended the run gets here.
     - :349 `let ended = operations.mark_uncertain(id).is_ok();` becomes `let ended = match operations.mark_uncertain(id) { Ok(_) => true, Err(error) => { events::mark_uncertain_failed(error); false } };`.
     - :255-256: before `return Err(Failure::Failed);` inside `if !install_worker::content_valid(...)`, add `events::repair_content_invalid(plan.id);`.
     - :252 `Err(_) => return Err(Failure::Failed),` becomes `Err(error) => { events::repair_stage_failed(plan.id, &error); return Err(Failure::Failed); }`.
     - :200 `.map_err(|_| Failure::Failed)?;` becomes `.map_err(|error| { events::repair_claim_failed(plan.id, error); Failure::Failed })?;`. `claim` returns `StorageError`; `plan` is still owned, because :196 cloned it.
     - :171 `let _ = operations.mark_uncertain(id);` becomes `if let Err(error) = operations.mark_uncertain(id) { events::mark_uncertain_failed(error); }`.
     - Before :152 (`runtime.spawn(async move {`) add `events::repair_started(id);`.
     - Add `use super::events;` to the imports if `events::` does not resolve. The module is `repair::events`, and preparation is `repair::preparation`.
  3. `repair/commit/mod.rs` :80: as the first line inside `if result.is_ok() {`, add `super::events::repair_finished(id);`.
  4. `uninstall/mod.rs` (line numbers at base `3d10ddf4d`, applied from the bottom up). Import the helpers as `crate::repair::events`, or via `super::repair::events`, whichever resolves:
     - :353: replace the whole `tracing::info!(target: "uninstall", directory = ..., ...)` statement with `events::uninstall_kept_adopted_folder();`. Its path field goes (R5).
     - Before :207 `Ok(())`, add `events::uninstall_finished(plan.id, recovering);`.
     - :201-205: as the first line inside `if let Err(error) = result {`, add `events::uninstall_failed(plan.id, error.code());`. Then :203 `let _ = operations.mark_uncertain(plan.id);` becomes `if let Err(e) = operations.mark_uncertain(plan.id) { events::mark_uncertain_failed(e); }`.
     - :181-185: the same two edits. The first line inside `if let Err(error) = result {` is `events::uninstall_failed(plan.id, error.code());` (`error` is an `IntentError`, which is `Copy`), and :183 changes the same way.
     - Before :170 (`self.remove_installation(&plan)`), add `events::uninstall_started(plan.id);`.
  5. Shell `host/repair/mod.rs` :290 `let _ = operations.mark_uncertain(id);` becomes `if let Err(error) = operations.mark_uncertain(id) { cimmeria_launcher_engine::repair::events::mark_uncertain_failed(error); }`.

  Left alone, and named here so the review knows: `repair/recovery/mod.rs:41` `let _ = send.send(outcome)`, whose gone receiver is transient, and LT-16b's client-maintenance and client-changes sites.
- **Tests:**
  - Inline in `events.rs`, under capture, all on `target == "launcher.repair"`:
    - `repair_start_and_finish_share_one_operation`;
    - `a_cancelled_repair_is_an_info_finish` (`repair_ended` with `Failure::Cancelled`);
    - `a_failed_repair_is_a_warn_with_its_reason` (`Failure::Failed` gives `repair_failed` with reason `failed`);
    - `a_stage_failure_carries_code_and_status` (`InstallError::UnexpectedStatus { status: 503, url: "https://example.invalid/x".into() }` gives `unexpected_status` and `"503"`, and no field contains `example.invalid`);
    - `uninstall_failed_carries_its_reason`.
  - Real failing path: add to `repair/preparation/tests.rs` `#[tokio::test] async fn failed_terminal_persistence_logs_mark_uncertain_failed()`.
    1. Run the setup from `failed_terminal_persistence_is_reported_as_uncertain` (:98-102: fixture, `admit`, and the journal replaced by a directory).
    2. Run `let (finished, events) = crate::test_capture::capture(|| finish_failure(&state, plan.id, Failure::Failed));`. This is a synchronous call, so it runs inside the async test.
    3. Assert `finished == Failure::ReconciliationRequired`.
    4. Assert that an event with `target == "launcher.repair"` and `event == "mark_uncertain_failed"` exists at WARN, with a `reason`.

    If there is no such event, `mark_uncertain` succeeded on this host. Return `blocked` with the captured event names.
  - Real success path: add to `uninstall/tests.rs` `#[test] fn confirmed_removal_logs_uninstall_started_and_finished()`.
    1. `let (_root, mut state, intent) = fixture();`
    2. `let (result, events) = crate::test_capture::capture(|| remove(&mut state, &intent));` then `result.unwrap();`.
    3. Assert, on `launcher.repair`: `uninstall_started` at INFO, then `uninstall_finished` at INFO with outcome `ok` and recovering `"false"`, both with the same `operation`.
- **Checks (in order):**
  - D1 `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all`;
  - D2e `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings`;
  - D2s, the same with `-p cimmeria-launcher-desktop`;
  - D3e `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine repair`;
  - D3e … `uninstall`;
  - D3s `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop repair`.
- **Docs:** none (LT-26).
- **Commit:** `LT-16a: repair and uninstall lifecycle events`.

### LT-16b: client maintenance and client-changes events (amendment 6)

- **Host:** windows. **Worktree:** the same `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-16`, branch `launcher-telemetry/lt-16-repair`, after LT-16a's commit. **Target:** `launcher.repair`. **Contract:** C5, C6 (amendment 6), C9. **Needs:** LT-14b (`StorageError::code`).
- **Rules (inline):** the same as LT-16a's rules 1, 2, 4, 5, 6 and 7, with these differences:
  - the shell calls start with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-16;`;
  - the helper file is `storage/client_maintenance/events.rs`;
  - `client_maintenance/mod.rs` (276 lines) and `client_changes/mod.rs` (199 lines) are under 450 lines, so they may be edited freely within the named functions.
- **Files** (under `crates/launcher/desktop/engine/src/storage/`):
  - new `client_maintenance/events.rs` (≤200 lines with its tests), declared `pub(crate) mod events;` after :12 of `client_maintenance/mod.rs`;
  - `client_maintenance/mod.rs` :97-101, :122-126, :209, :218-221, :230;
  - `client_changes/mod.rs` :178.
- **Change:**
  1. In `events.rs`, each macro uses `target: "launcher.repair"`. There is no `operation`, because maintenance is not a journal operation.
     - `pub(crate) fn action(request: &super::ClientMaintenanceCommand) -> &'static str` returns `"inspect"`, `"reset_cache"` or `"reset_all_client_state"`.
     - `impl super::ClientMaintenanceError { pub fn code(self) -> &'static str }` returns `"confirmation_required"`, `"profile_unavailable"` or `"unsafe_path"`, and `Storage(e)` gives `e.code()`.
     - `client_maintenance_finished(action: &'static str, removed: bool, took: std::time::Duration)` logs info `client_maintenance_finished` with `action`, `outcome = "ok"`, `removed` and `duration_ms`.
     - `client_maintenance_failed(action: &'static str, reason: &'static str)` logs warn `client_maintenance_failed`.
     - `client_maintenance_removal_failed(error_kind: &'static str)` logs warn `client_maintenance_removal_failed` with `error_kind`.
     - `client_changes_ledger_unreadable(reason: &'static str)` logs warn `client_changes_ledger_unreadable`.
  2. `client_maintenance/mod.rs`:
     - :97-101: replace the `tracing::warn!(?request, ...)` statement with `events::client_maintenance_failed(events::action(&request), "profile_unavailable");`.
     - :122-126 (`client_maintenance_at`): replace the body with:
       ```rust
       let started = std::time::Instant::now();
       let result = self.client_maintenance_in(request, firesky);
       match &result {
           Ok(status) => {
               if let Some(removed) = status.removed {
                   events::client_maintenance_finished(events::action(&request), removed, started.elapsed());
               }
           }
           Err(error) => events::client_maintenance_failed(events::action(&request), error.code()),
       }
       result
       ```
       `Inspect` (`removed: None`) logs nothing, because the panel runs it on every open. `ClientMaintenanceCommand` and `ClientMaintenanceError` are `Copy`.
     - :209: delete the `tracing::warn!(folder = ...)` statement. The `client_maintenance_at` failure event carries `unsafe_path`, and the folder is a path (R5).
     - :220: replace the warn with `events::client_maintenance_removal_failed(crate::game_telemetry::launcher_event::kind(&error));`. Keep the `StorageError::Io.into()` return. In the doc comment at :218, change "the log keeps the path and the OS error" to "the event keeps the I/O error kind".
     - :230: delete the `tracing::info!(path = ...)` statement. `client_maintenance_finished` replaces it.
  3. `client_changes/mod.rs` :178 `Err(_) => (crate::state::InstalledState::default(), true),` becomes `Err(error) => { super::client_maintenance::events::client_changes_ledger_unreadable(error.code()); (crate::state::InstalledState::default(), true) }`. `read` returns `StorageError`. If the compiler shows a type with no `code()`, use `reason = "unreadable"` instead. That fallback is mechanical, so do not block on it.
- **Tests:** inline in `events.rs`, under capture, on `target == "launcher.repair"`:
  - `an_unconfirmed_reset_logs_client_maintenance_failed`. This is the real path.
    - Setup: `let root = tempfile::tempdir().unwrap(); let firesky = root.path().join("Firesky"); let mut state = crate::DesktopState::open(&root.path().join("state")).unwrap();`.
    - Run `capture(|| state.client_maintenance_fixture(ClientMaintenanceCommand::ResetAllClientState { schema_version: 1, confirmed: false }, &firesky))`.
    - The result is `Err(ClientMaintenanceError::ConfirmationRequired)`, and there is one WARN `client_maintenance_failed` with action `reset_all_client_state` and reason `confirmation_required`.
  - `an_empty_cache_reset_logs_removed_false`: the same setup, with `ClientMaintenanceCommand::ResetCache { schema_version: 1 }` on the absent `Firesky`. The result is `Ok` with `removed == Some(false)`, and there is one INFO `client_maintenance_finished` with action `reset_cache`, removed `"false"` and a `duration_ms`.
  - `an_inspect_logs_nothing`: `Inspect { schema_version: 1 }` gives no `launcher.repair` event.
  - `storage_errors_use_the_storage_code`: `ClientMaintenanceError::Storage(StorageError::Busy).code() == "busy"`.
  - `ledger_unreadable_carries_its_reason`.
- **Checks (in order):**
  - D1 `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all`;
  - D2e `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings`;
  - D2s, the same with `-p cimmeria-launcher-desktop`;
  - D3e `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine client_maintenance`;
  - D3e … `client_changes`;
  - D3s `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop client_maintenance`.
- **Docs:** none (LT-26).
- **Commit:** `LT-16b: client maintenance and client-changes events`.

### LT-17a: launch and Play lifecycle events (amendment 6)

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-17`, branch `launcher-telemetry/lt-17-launch`. `ops` creates it from the `lt-app-integration` tip after the LT-14 branch has merged; it runs in parallel with LT-16. LT-17b follows in the same worktree, and the branch merges once after both are reviewed. **Target:** `launcher.launch`. **Contract:** C5, C6 (amendment 6), C9. **Needs:** LT-14b (`operation_numbers`, `ContractError::code`, `IntentError::code`).
- **Rules (inline):**
  1. Start every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-17;`. Use PowerShell only and run everything in the foreground. Run cargo only through `pwsh tools/build-lane/lane.ps1`. Every clippy and test check passes `--locked`. Allow at most 5 fix rounds per check, then return `blocked` with the last `status=` line.
  2. Edit only the files below. Make one commit with the subject below, and end it with the Co-Authored-By and Claude-Session lines from your brief.
  3. Files have a soft cap of 500 lines and a hard cap of 700. `launch/worker.rs` is 606 lines and takes call lines only; it may grow by 25 lines at most. Do not touch `shell/src/host/launch/resources.rs` `dll_bundled` (LT-06).
  4. Fields and messages may not contain a path, URL, host, IP, port, account, token, session id or operation UUID. Never format an error value with `%e` or `?e`. `operation` is always `crate::operation_numbers::number(id)`, where `id` is the launch operation's `Uuid` (`plan.id`, or `request.operation_id` in the supervisor: the same value). The shared map is what lets the supervisor, which only has the UUID, log the same number.
  5. Every `tracing::` call lives in `storage/launch/events.rs`, with the literal `target: "launcher.launch"` and an `event =` field. Use info for transitions and warn for failures. Logging never changes control flow or a return value.
  6. Tests use `crate::test_capture::capture(|| ...)`. It is thread-local, so async code runs inside it on `tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()` through `block_on`. Values read back as strings. Each new test must fail when the change is reverted. `supervisor_tests.rs` is `cfg(unix)`, so it does not run on this host. Keep it compiling: do not change `supervisor::run`'s signature.
  7. If code here does not match a line or type named below, and the fix is not mechanical, return `blocked` and name the line. Do not redesign.
- **Files** (paths under `crates/launcher/desktop/`):
  - new `engine/src/storage/launch/events.rs` (≤220 lines with its tests), declared `pub mod events;` after :13 (`mod telemetry;`) of `storage/launch/mod.rs` (269 lines);
  - new `engine/src/storage/launch/events_tests.rs` (≤100 lines), declared `#[cfg(test)] mod events_tests;` in `launch/mod.rs` next to its `mod tests;`;
  - `engine/src/storage/launch/mod.rs` :88, :94, :116, :128, :148, :188;
  - `engine/src/storage/launch/worker.rs` (call lines only);
  - `engine/src/storage/launch/supervisor.rs` (144 lines);
  - `shell/src/host/launch/mod.rs` :72.
- **Change:**
  1. Helpers in `events.rs`. Each `id: Uuid` helper logs `operation`, and every one is `pub(crate)` unless noted:

     | Helper | Level | Event | Fields |
     |---|---|---|---|
     | `launch_refused(id, reason: &'static str)` | warn | `launch_refused` | `operation`, `reason` |
     | `launch_started(id, telemetry_dll: bool)` | info | `launch_started` | `operation`, `telemetry_dll` |
     | `launch_not_started(id, reason: &'static str)` | warn | `launch_not_started` | `operation`, `reason` |
     | `launch_failed(id, reason: &'static str)` | warn | `launch_failed` | `operation`, `reason` |
     | `game_started(id)` | info | `game_started` | `operation` |
     | `game_exited(id, code: i32, early: bool, ran: std::time::Duration)` | info | `game_exited` | `operation`, `exit_code = i64::from(code)`, `early`, `duration_ms` from `ran` |
     | `launch_finished(id, observation: super::Observation)` | info | `launch_finished` | `operation`, `outcome` (`ProcessExited { .. }` gives `"exited"`, `NotStarted` gives `"not_started"`, `Cancelled` gives `"cancelled"`, anything else gives `"unknown"`), `duration_ms = crate::operation_numbers::elapsed_ms(id)` |
     | `patch_events_queue_failed(id)` | warn | `patch_events_queue_failed` | `operation` |
     | `pub fn mark_uncertain_failed(error: crate::ContractError)` | warn | `mark_uncertain_failed` | `reason = error.code()` |
  2. `launch/mod.rs` (`admit_launch`): replace each whole `tracing::warn!(operation = %id, ...)` statement:

     | Line | Becomes |
     |---|---|
     | :88 | `events::launch_refused(id, "retry_without_plan");` |
     | :94 | `events::launch_refused(id, "retry_plan_mismatch");` |
     | :116 | `events::launch_refused(id, "not_the_installed_copy");` |
     | :128 | `events::launch_refused(id, "telemetry_not_opted_in");` |
     | :148 | `events::launch_refused(id, "client_patches_off");` |
     | :188 | `events::launch_refused(id, "plan_exists");` |
  3. `worker.rs`, at base `3d10ddf4d`, applied from the bottom up:
     - :248: replace the whole `tracing::warn!(operation = %plan.id, ?error, ...)` statement with `events::launch_not_started(plan.id, "login_servers_refused");`. If `error` becomes unused, bind it as `_error`. :141-143 then stays silent; it is the same refusal.
     - :226-229 `let _ = tokio::task::spawn_blocking(move || ...).await;` becomes `if tokio::task::spawn_blocking(move || ...).await.is_err() { events::patch_events_queue_failed(plan.id); }`. Keep the closure unchanged.
     - :151-153: inside the `else` block, before `return`, add `events::launch_not_started(plan.id, "prepare_failed");`.
     - :134-140: before `return Ok(Observation::NotStarted);` at :139, add `events::launch_not_started(plan.id, "verify_failed");`.
     - :114: before `updates.send_replace(...)`, add `events::launch_finished(plan.id, if commit.is_ok() { result } else { Observation::Unknown });`. `Observation` is `Copy`.
     - :107-113 becomes:
       ```rust
       if commit.is_err() {
           events::launch_failed(plan.id, "commit_failed");
           if let Ok(mut owner) = state.lock() {
               if let Ok(operations) = owner.operations_mut() {
                   if let Err(error) = operations.mark_uncertain(id) {
                       events::mark_uncertain_failed(error);
                   }
               }
           }
       }
       ```
     - :78-81 becomes `let result = match result { Ok(Ok(result)) => result, Ok(Err(error)) => { events::launch_failed(plan.id, error.code()); Observation::Unknown } Err(_) => { events::launch_failed(plan.id, "panicked"); Observation::Unknown } };`. `error` is an `IntentError`, and `code()` comes from LT-14b.
     - After :63 `owner.record_launch(&plan, Observation::Preparing)?;`, add `events::launch_started(plan.id, plan.resources.client_telemetry.is_some());`.
     - Import with `use super::events;` if `events::` does not resolve.
  4. `supervisor.rs` (`run`): first statement `let id = request.operation_id;`, then, keeping every return value unchanged:

     | Line | Edit |
     |---|---|
     | :22-24 | `launch_not_started(id, "request_invalid")` |
     | :25-27 | `launch_not_started(id, "request_too_large")` |
     | :40-42 | `launch_not_started(id, "spawn_failed")` |
     | :45-46 | split `if host_pid == 0 \|\| observe(..).is_err()` into two `if`s with the same expressions and returns: `launch_failed(id, "no_pid")` and `launch_failed(id, "observe_failed")` |
     | :51-59 | each `Observation::Unknown` return: `launch_failed(id, "stdin_write_failed")` |
     | :67-70 | `_ =>` arm: `launch_failed(id, "no_first_event")` |
     | :72-74 | the `observe(...)` failure: `launch_failed(id, "observe_failed")`; then before `(guest_pid, Instant::now())` add `events::game_started(id);` |
     | :83-92 | a clean helper exit (returns `NotStarted`): `launch_not_started(id, "helper_not_started")`; the other arm: `launch_failed(id, "helper_unclean_exit")` |
     | :93 | `launch_failed(id, "unexpected_event")` |
     | :107-113 | `launch_failed(id, "no_exit_event")` |
     | :114-116 | `launch_failed(id, "pid_mismatch")` |
     | :117-122 | `launch_failed(id, "helper_unclean_exit")` |
     | :123-128 | move the existing `early` expression, unchanged, into `let early = ...;` above the struct. Compute `started.elapsed()` once as `let ran = started.elapsed();` and use it in that expression. Add `events::game_exited(id, code, early, ran);` and build the struct with `early` |

     A `return x` that needs an event becomes `{ events::...; return x; }`.
  5. Shell `host/launch/mod.rs` :72 `let _ = operations.mark_uncertain(operation_id);` becomes `if let Err(error) = operations.mark_uncertain(operation_id) { cimmeria_launcher_engine::launch::events::mark_uncertain_failed(error); }`. The `:115` item of the original LT-17 is dropped: it is a `cfg!(target_os = "macos")` filter, not an event.
- **Tests:**
  - Inline in `events.rs`, under capture, on `target == "launcher.launch"`:
    - `launch_finished_maps_each_observation` (`ProcessExited`, `NotStarted`, `Cancelled`, `Unknown`);
    - `game_exited_carries_code_and_duration` (code `-1` reads back as `"-1"`, early `"true"`);
    - `refusals_carry_their_reason`;
    - `mark_uncertain_failed_carries_the_contract_code`.
  - Real failing path: `events_tests.rs` gets `#[test] fn a_replaced_helper_logs_launch_not_started_verify_failed()`.
    1. Build a current-thread runtime `rt`.
    2. Run `let ((), events) = crate::test_capture::capture(|| rt.block_on(async { ... }));`. The async block runs the body of `tests.rs::dropping_observer_retains_native_task_and_persists_preparation_failure` (:247-271):
       - `super::tests::fixture()`;
       - overwrite `plan.resources.helper.path()`;
       - `dispatch(state.clone(), plan.id)`;
       - keep the worker as `_worker`;
       - wait, with the same loop, until the operation is terminal.
    3. Assert, on `launcher.launch`:
       - `launch_started` with `telemetry_dll` `"false"`;
       - WARN `launch_not_started` with reason `verify_failed`;
       - `launch_finished` with outcome `not_started`;
       - all three with one `operation`.

    The task commits and logs `launch_finished` without an `.await` in between, so it has logged by the time the loop sees the terminal state.
- **Checks (in order):**
  - D1 `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all`;
  - D2e `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings`;
  - D2s, the same with `-p cimmeria-launcher-desktop`;
  - D3e `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine launch`;
  - D3s `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop launch`.
- **Docs:** none (LT-26).
- **Commit:** `LT-17a: launch and Play lifecycle events`.

### LT-17b: game-telemetry session, helper stop and DLL artifact events (amendment 6)

- **Host:** windows. **Worktree:** the same `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-17`, branch `launcher-telemetry/lt-17-launch`, after LT-17a's commit. **Target:** `launcher.launch`. **Contract:** C5, C6 (amendment 6), C9. **Needs:** LT-17a (`launch/events.rs`, `events_tests.rs`).
- **Rules (inline):** the same as LT-17a's rules 1, 2, 4, 5, 6 and 7. All the files here are under 450 lines.
- **Files** (paths under `crates/launcher/desktop/`):
  - `engine/src/storage/launch/events.rs` (adds four helpers);
  - `engine/src/storage/launch/telemetry.rs` (114 lines) :21-22, :46-47, :64, :112;
  - `engine/src/helper_supervisor/mod.rs` (265 lines) :223-224;
  - `shell/src/host/launch/resources.rs` (116 lines) :41-45;
  - `engine/src/storage/launch/events_tests.rs` (adds one test).
- **Change:**
  1. New helpers in `events.rs`, each with `target: "launcher.launch"`:

     | Helper | Level | Event | Fields |
     |---|---|---|---|
     | `game_telemetry_skipped(id, reason: &'static str)` | warn | `game_telemetry_skipped` | `operation`, `reason` |
     | `game_telemetry_record_failed(id)` | warn | `game_telemetry_record_failed` | `operation`, `reason = "status_save_failed"` |
     | `pub(crate) fn helper_stop_failed(error_kind: &'static str)` | warn | `helper_stop_failed` | `error_kind` |
     | `pub fn dll_artifact_unavailable(reason: &'static str)` | warn | `dll_artifact_unavailable` | `reason` |
  2. `telemetry.rs`. A player who is opted out, or has no session, stays silent: those are normal outcomes.
     - :21 `let owner = state.lock().ok()?;` becomes `let Ok(owner) = state.lock() else { events::game_telemetry_skipped(plan.id, "state_lock_poisoned"); return None; };`.
     - :22 `let choice = owner.game_telemetry().ok()?;` becomes a `match` whose `Err(_)` arm logs `game_telemetry_skipped(plan.id, "choice_unreadable")` and returns `None`.
     - :46 `preparation::contained(&game, &marker).ok()?;` becomes `if preparation::contained(&game, &marker).is_err() { events::game_telemetry_skipped(plan.id, "marker_outside_game_dir"); return None; }`.
     - :47 becomes `let session = session?;` (silent) followed by `if crate::telemetry_session::write_current_session(&game, session.marker()).is_err() { events::game_telemetry_skipped(plan.id, "marker_write_failed"); return None; }`.
     - :64 is split into three steps:
       1. A lock failure logs `"state_lock_poisoned"` and returns `None`.
       2. `Consent::read(&*owner)` returning `Err(_)` logs `"consent_unreadable"` and returns `None`.
       3. `Ok(None)` returns `None` silently.

       Drop the guard before the `match Transport::open` exactly as the original expression does: bind it in an inner block.
     - :69 stays as it is: the warn on the module target, which is not forwarded.
     - :112 `let _ = owner.record_game_telemetry(plan.id, outcome);` becomes `if owner.record_game_telemetry(plan.id, outcome).is_err() { events::game_telemetry_record_failed(plan.id); }`.
  3. `helper_supervisor/mod.rs` `stop` :223-224:
     - keep `let _ = child.start_kill();`, with the comment `// An exited child refuses the kill; the wait below reports a real failure.`;
     - replace `let _ = timeout(Duration::from_secs(3), child.wait()).await;` with a `match`:
       - `Ok(Ok(_)) => {}`;
       - `Ok(Err(error)) => crate::launch::events::helper_stop_failed(crate::game_telemetry::launcher_event::kind(&error))`;
       - `Err(_) => crate::launch::events::helper_stop_failed("timed_out")`.

     No operation is in scope here. The only caller is mac_wine `run_owned`, which is not the Play path.
  4. Shell `resources.rs` :41-45. `bundled` runs once, from `with_launch_resources`, so this logs at most once per process. Replace the `client_telemetry` binding with:
     ```rust
     let client_telemetry = match digests.client_telemetry {
         // This build pins no telemetry DLL: nothing to report.
         None => None,
         Some(digest) => match Artifact::open(root.join("windows/cimmeria_client_telemetry.dll"), digest) {
             Err(_) => {
                 cimmeria_launcher_engine::launch::events::dll_artifact_unavailable("unverified");
                 None
             }
             Ok(dll) if dll.refuse_lab_build().is_err() => {
                 cimmeria_launcher_engine::launch::events::dll_artifact_unavailable("lab_build");
                 None
             }
             Ok(dll) => Some(dll),
         },
     };
     ```
     Keep the comment above it. `dll_bundled` stays untouched.
- **Tests:**
  - Inline in `events.rs`: each of the four helpers under capture, checking its event, level and reason.
  - Real failing path: `events_tests.rs` gets `#[test] fn an_unreadable_choice_logs_game_telemetry_skipped()`. On a current-thread runtime inside capture:
    1. `let (root, state, mut plan) = super::tests::fixture();`
    2. `plan.resources.client_telemetry = Some(super::tests::artifact(&root.path().canonicalize().unwrap(), "telemetry.dll"));`
    3. Replace the game-telemetry choice file in `state.state_root()` with a directory. Its file name is the private `NAME` constant at `storage/game_telemetry/mod.rs:28`, so use the literal `"game-telemetry.json"`. Check how `game_telemetry()` (:97) builds the path; if it is not `state_root().join(NAME)`, use the path it builds.
    4. Run `super::telemetry::session(&Arc::new(Mutex::new(state)), &plan, &[]).await`, and assert it returns `None`.
    5. Assert one WARN `game_telemetry_skipped` with reason `choice_unreadable`.

    If the read of a directory does not fail on this host, return `blocked` with the events seen.
  - `resources.rs` has no Windows test: its tests are `cfg(target_os = "macos")`, because of verbatim paths. The helper test covers the event, and the review reads the match.
- **Checks:**
  - D1 `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all`;
  - D2e `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings`;
  - D2s, the same with `-p cimmeria-launcher-desktop`;
  - D3e `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine launch`;
  - D3e … `helper_supervisor`;
  - D3s `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop launch`.
- **Docs:** none (LT-26).
- **Commit:** `LT-17b: game-telemetry session, helper stop and DLL artifact events`.
