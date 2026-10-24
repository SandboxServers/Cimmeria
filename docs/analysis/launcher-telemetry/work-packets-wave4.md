# Launcher telemetry: wave 4 packets (amendment 7)

Wave 4 (subsystem coverage B), pre-checked against the `lt-app-integration` tip (LT-05..LT-17 merged; LT-16b still open) and re-sliced by amendment 7 ([LT-18-23-amendment.md](LT-18-23-amendment.md)). LT-18..LT-23 become eight packets: LT-18, LT-19, LT-20, LT-21a, LT-21b, LT-22, LT-23a, LT-23b. Campaign rules R1-R10 and contracts C1-C18 are in [work-packets.md](work-packets.md); each packet below inlines the rules it needs, so a coder reads only its own section and § Capture standard.

## Start, parallel lanes and merge order

- **Start:** wave 4 starts after every wave-3 packet (the last is LT-16b) and the wave-3 review have merged into `lt-app-integration`, and `ops` has merged `origin/main` into it (work-packets § Branches). `ops` then creates all five worktrees below from that tip at once.
- **Five parallel lanes** (the lanes share no file):

  | Lane | Worktree (under `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\`) | Branch | Packets, in order | Files the lane owns |
  |---|---|---|---|---|
  | 1 | `lt-18` | `launcher-telemetry/lt-18-worker-codes` | LT-18 | runtime-probe `game_launch/{mod.rs,windows.rs}`; engine `storage/launch/{supervisor.rs,events.rs}` |
  | 2 | `lt-19` | `launcher-telemetry/lt-19-settings` | LT-19 | engine `commands.rs` (`dispatch` only), new `storage/settings_events.rs`, `storage/mod.rs` (one `mod` line), `storage/game_telemetry/mod.rs` (one call line) |
  | 3 | `lt-20` | `launcher-telemetry/lt-20-ui-errors` | LT-20 | frontend `ui-event.ts`, `client-patches-view.ts`, `server-list-view.ts`, `log-upload-view.ts`, `physx-view.ts`, `launch-workflow.ts` and their tests |
  | 4 | `lt-21` | `launcher-telemetry/lt-21-log-upload` | LT-21a, then LT-21b (fresh coder, same worktree) | engine `storage/log_upload/*` |
  | 5 | `lt-22` | `launcher-telemetry/lt-22-summary-endpoint` | LT-22, then LT-23a, then LT-23b (fresh coder each, same worktree) | engine `launcher_summary/{endpoint.rs,mod.rs}`, `launcher_summary/tests/batch_seam.rs`, `commands.rs` (`NativeSnapshot` and `inspect` only); shell `host/summary.rs`; frontend `contract.ts`, `view.ts`, `ui/index.html`, `view.test.ts` |

- Lanes 2 and 5 both edit `engine/src/commands.rs`, in different hunks: LT-19 edits `dispatch` (:71-106) and its tests, LT-23b edits `NativeSnapshot` (:29-40) and `inspect` (:60-68). Git merges them, but merge lane 2 before lane 5.
- **Ships as:** no per-packet PR. Each lane merges into `lt-app-integration` (branch `launcher-telemetry/launcher`, which ships as PR L) with `merge-noff.ps1` after its packets pass review.
- **Merge order:** LT-18 → LT-19 → LT-20 → LT-21 (a+b) → LT-22 lane (LT-22, LT-23a, LT-23b) → wave-4 review. A lane that is ready early may merge early, except that lane 5 waits for lane 2.
- **Wave-4 review:** eight packets, all small. One `packet-reviewer` holds the wave; if it runs short, split it as lanes 1-3 and lanes 4-5.

## Capture standard (every Rust capture test in this wave)

LT-16a's final commit (`44a052bda`, `uninstall/tests.rs`) made its test deterministic with two pieces. LT-17a (`launch/events.rs` tests, :199-205) added a third. A callsite that another test first hits with no subscriber can stay disabled in tracing's interest cache, so a capture test sees nothing. Use all three in every new capture test:

```rust
use std::sync::{Mutex, MutexGuard};
static SERIAL: Mutex<()> = Mutex::new(());
fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

#[test]
fn example() {
    let _serial = serial(); // first line: one capture test of this module at a time
    // A second live Dispatch keeps callsite interest per dispatcher, so a callsite
    // that another module's test hit first is not left disabled.
    let _second = tracing::Dispatch::new(tracing_subscriber::registry());
    let (_, events) = crate::test_capture::capture(|| {
        tracing::callsite::rebuild_interest_cache(); // first line in the closure
        /* the call under test */
    });
}
```

- One `SERIAL`/`serial()` pair per test module. Reuse the module's existing pair if it has one.
- `capture` is thread-local: only synchronous calls run inside it. Integers and booleans read back as strings (`"5"`, `"true"`). A field logged as `None` is absent (`field("win32") == None`). Since LT-40, capture panics if a captured event holds an IP literal.

## LT-18: launch-worker stage and Win32 code

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-18`, branch `launcher-telemetry/lt-18-worker-codes` (lane 1). **Target:** `launcher.launch`. **Contract:** C5, C6, C9. **Needs:** LT-17a (merged: `storage/launch/events.rs`).
- **Rules (inline):**
  1. Start every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-18;`. Use PowerShell only, in the foreground. Run cargo only through `pwsh tools/build-lane/lane.ps1`, with `--locked` on every clippy and test. At most 5 fix rounds per check, then return `blocked` with the last `status=` line.
  2. Edit only the files below. Make one commit with the subject below, ending with the Co-Authored-By and Claude-Session lines from your brief. No stash, push or rebase.
  3. Soft cap 500 lines, hard cap 700. Report every touched file's line count.
  4. No path, URL, host, IP, port, account, token, session id or operation UUID in a field or message. Never `%e`/`?e`. `operation` is always `crate::operation_numbers::number(id)`.
  5. Every new `tracing::` call lives in `engine/src/storage/launch/events.rs`, with `target: "launcher.launch"` and an `event =` field. Logging never changes control flow or a return value.
  6. Capture tests follow § Capture standard. Each new test must fail when the change is reverted.
  7. If the code does not match a line or type named here and the fix is not mechanical, return `blocked` naming the line. Do not redesign.
- **What changed from the original LT-18:** the decoder is `storage/launch/supervisor.rs`, not `worker.rs`. The panic path is already logged (`worker.rs:79-89`, `launch_failed(id, "panicked")`, LT-17a), so `worker.rs` is out of scope. `stage` is a closed enum, not a `String`, so `Event` stays `Copy`. `NotStarted` and `Unknown` are unit variants today and become struct variants with defaulted fields.
- **Files** (paths under `crates/launcher/desktop/`):
  - `runtime-probe/src/game_launch/mod.rs` (90 lines): the `Event` enum and its test module (:66-88);
  - `runtime-probe/src/game_launch/windows.rs` (116 lines): :52, :55-57, :60-63, :66-68, :73-77;
  - `engine/src/storage/launch/supervisor.rs` (171 lines): :98, :110, :127;
  - `engine/src/storage/launch/events.rs` (305 lines): two helpers and one test;
  - any other Rust constructor of `Event::NotStarted` or `Event::Unknown`. Before editing, run `Select-String -Path (Get-ChildItem -Recurse -Include *.rs crates/launcher/desktop) -Pattern 'Event::NotStarted|Event::Unknown'`. Expected: only `windows.rs` and `supervisor.rs`, plus possibly `supervisor_tests.rs` (`cfg(unix)`). Update each one mechanically to `{ stage: None, win32: None }`. JSON strings like `{"event":"not_started"}` need no change.
- **Change:**
  1. `game_launch/mod.rs`: add before `Event`:
     ```rust
     /// Where the helper stopped. A closed set; never a path or a message.
     #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
     #[serde(rename_all = "snake_case")]
     pub enum Stage { Request, CreateProcess, Inject, Resume, Wait }
     ```
     Change the two variants of `Event` (keep its derives and `#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]`):
     ```rust
     NotStarted {
         #[serde(default, skip_serializing_if = "Option::is_none")]
         stage: Option<Stage>,
         #[serde(default, skip_serializing_if = "Option::is_none")]
         win32: Option<u32>,
     },
     Unknown { /* the same two fields */ },
     ```
  2. `windows.rs`: add `fn win32_of(error: &InjectError) -> Option<u32>`, returning `Some(*code)` for `InjectError::Win32 { code, .. }` and `None` otherwise. Import `InjectError` the way `windows.rs` already imports from `client-launch` (`client-launch/src/inject.rs:36-63`). Then:

     | Line | Today | Becomes |
     |---|---|---|
     | :52 (request validation) | `Event::NotStarted` | `Event::NotStarted { stage: Some(Stage::Request), win32: None }` |
     | :55-57 (`create_process_suspended` `Err(_)`) | `Event::NotStarted` | bind the error `Err(e)`; `NotStarted { stage: Some(Stage::CreateProcess), win32: win32_of(&e) }` |
     | :60-63 (`inject_dll` `Err`, after terminate) | `Event::Unknown` | `Unknown { stage: Some(Stage::Inject), win32: win32_of(&e) }` |
     | :66-68 (`resume_running` `Err(_)`) | `Event::Unknown` | `Unknown { stage: Some(Stage::Resume), win32: win32_of(&e) }` |
     | :73-77 (`running.wait()` `Err(_)`, an `io::Error`) | `Event::Unknown` | `Unknown { stage: Some(Stage::Wait), win32: e.raw_os_error().and_then(\|c\| u32::try_from(c).ok()) }` |

     Never put the DLL path, the exe path or a command line into an event.
  3. `events.rs`: add `use cimmeria_runtime_probe::game_launch::Stage;` and a private `fn stage_reason(stage: Stage) -> &'static str`: `Request` → `request_refused`, `CreateProcess` → `create_process_failed`, `Inject` → `inject_failed`, `Resume` → `resume_failed`, `Wait` → `wait_failed`. Then add two helpers. They log the existing event names, so dashboards keep one event per outcome:

     | Helper | Level | Event | Fields |
     |---|---|---|---|
     | `launch_not_started_at(id: Uuid, stage: Option<Stage>, win32: Option<u32>)` | warn | `launch_not_started` | `operation`, `reason` = `stage_reason(s)`, or `helper_not_started` when `stage` is `None`; `win32` |
     | `launch_failed_at(id: Uuid, stage: Option<Stage>, win32: Option<u32>)` | warn | `launch_failed` | `operation`, `reason` = `stage_reason(s)`, or `worker_unknown` when `stage` is `None`; `win32` |

     `win32` is logged as the `Option<u32>` itself (tracing records only `Some`).
  4. `supervisor.rs`:
     - :98 `Event::NotStarted => {` becomes `Event::NotStarted { stage, win32 } => {`. In its success branch, :103 `events::launch_not_started(id, "helper_not_started")` becomes `events::launch_not_started_at(id, stage, win32)`. The unclean-exit branch (:106) stays.
     - Before `_ => {` at :110, add `Event::Unknown { stage, win32 } => { events::launch_failed_at(id, stage, win32); return Observation::Unknown; }`.
     - Before the `let Ok(Event::ProcessExited {` at :127, add `if let Ok(Event::Unknown { stage, win32 }) = event { events::launch_failed_at(id, stage, win32); return Observation::Unknown; }`.
- **Tests:**
  - `game_launch/mod.rs` tests: `an_old_helper_event_without_stage_still_decodes`. Build the message bytes as the existing test at :66-88 does, with an event of `{"event":"not_started"}` and then `{"event":"unknown"}`. `Message::decode` gives `NotStarted { stage: None, win32: None }` and `Unknown { stage: None, win32: None }`. Also add `stage_and_win32_round_trip`: serialize `Unknown { stage: Some(Stage::Inject), win32: Some(5) }`, check that the JSON holds `"stage":"inject"` and `"win32":5`, and decode it back equal. If an old event no longer decodes under `deny_unknown_fields`, return `blocked` with the error.
  - `events.rs` tests: `worker_failures_carry_stage_reason_and_win32`, per § Capture standard (this module already uses `_second`; add `SERIAL`). Each call asserts `level == WARN` and `target == "launcher.launch"`:

    | Call | `event` | `reason` | `win32` |
    |---|---|---|---|
    | `launch_failed_at(id, Some(Stage::Inject), Some(5))` | `launch_failed` | `inject_failed` | `"5"` |
    | `launch_failed_at(id, None, None)` | `launch_failed` | `worker_unknown` | absent |
    | `launch_not_started_at(id, Some(Stage::CreateProcess), Some(2))` | `launch_not_started` | `create_process_failed` | `"2"` |
  - The supervisor path is pinned only by `supervisor_tests.rs`, which is `cfg(unix)` and does not run on this host. Do not add tests there. Say so in your report.
- **Checks**, in this order:
  - D1 `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all`
  - D2p `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-runtime-probe --all-targets -- -D warnings`
  - D3p `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-runtime-probe game_launch`
  - D2e: the D2p command with `-p cimmeria-launcher-engine`
  - D3e `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine launch::`
- **Commit:** `LT-18: report launch-worker stage and Win32 code`.

## LT-19: settings and consent change events

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-19`, branch `launcher-telemetry/lt-19-settings` (lane 2). **Target:** `launcher.settings` (already in the forward table). **Contract:** C6, C9.
- **Rules (inline):** rules 1-7 of LT-18, with the worktree `lt-19`, and two more:
  - Every new `tracing::` call lives in `engine/src/storage/settings_events.rs`, with `target: "launcher.settings"`.
  - **The file `settings_events.rs` must never contain the text `launcher_summary_consent`.** A source-scan test, `only_the_allowed_files_mention_the_summary_consent_flag` (`docs/launcher-summaries.md:91`), fails if it does. Call the parameter `setup_diagnostics`.
- **What changed from the original LT-19:**
  - `preferences_unreadable` is dropped. `storage/mod.rs:131` is a migration default and `:263-266` is a missing-file default; read errors already propagate with `?`, so there is no swallowed error to log.
  - `login_servers` and `custom_login_servers` are not C6 fields. They become `count` and the boolean `custom_servers`.
  - The game-diagnostics event moves from the shell into the engine setter, so the shell is untouched.
  - The fresh-host default save (`shell/src/host.rs:211-212`) is automatic, not a player change, and stays silent.
- **Files** (under `crates/launcher/desktop/engine/src/`):
  - new `storage/settings_events.rs` (≤160 lines with tests), declared `pub(crate) mod settings_events;` next to the other `mod` lines at the top of `storage/mod.rs` (541 lines: that one line only);
  - `commands.rs` (232 lines): `dispatch`, the `SavePreferences` arm :78-92 and the `SaveClientOptions` arm :93-103, plus one test;
  - `storage/game_telemetry/mod.rs`: `set_game_telemetry` (:115-130), one call line after the write at :127.
- **Change:**
  1. Helpers in `settings_events.rs`. All are info; none takes a path or a URL.

     | Helper | Event | Fields |
     |---|---|---|
     | `preferences_saved(install_dir_changed: bool, setup_diagnostics: bool)` | `preferences_saved` | both booleans |
     | `setup_diagnostics_changed(opted_in: bool)` | `setup_diagnostics_changed` | `opted_in` |
     | `client_options_saved(servers: &[LoginServer], client_patches_enabled: bool)` | `client_options_saved` | `count: u64` = `servers.len()` (0 means the defaults); `custom_servers: bool` = any `servers` entry whose `url` is not the `url` of an entry of `crate::client_setup::login_servers::default_servers()`; `client_patches_enabled` |
     | `game_diagnostics_changed(opted_in: bool)` | `game_diagnostics_changed` | `opted_in` |

     Import `LoginServer` from the path `commands.rs` uses. Doc comment on the module: "Logged after the save succeeds. An opt-out is logged after the gate has closed, so it stays in the local log; an opt-in is forwarded."
  2. `commands.rs` `SavePreferences` arm: before the `self.save_preferences(...)?` call (:87-91), take `let before = self.preferences().clone();`. After it, with `let after = self.preferences();`:
     - call `settings_events::preferences_saved(before.install_directory != after.install_directory, after.launcher_summary_consent)`;
     - then, only if `before.launcher_summary_consent != after.launcher_summary_consent`, call `settings_events::setup_diagnostics_changed(after.launcher_summary_consent)`.

     `commands.rs` may name the flag; only the new file may not. If the borrow checker refuses, clone `after` too.
  3. `commands.rs` `SaveClientOptions` arm, after the `save_client_options(...)?` call at :102: `let after = self.preferences(); settings_events::client_options_saved(&after.login_servers, after.client_patches_enabled);`.
  4. `game_telemetry/mod.rs`: right after the write at :127 succeeds, call `crate::storage::settings_events::game_diagnostics_changed(<the new opted_in value>)`. The early return at :120-122 already skips an unchanged value.
- **Tests:**
  - `commands.rs`: `saving_client_options_reports_a_count_not_the_servers`. Copy `stale_client_options_are_refused_and_write_nothing` (:129) for the `DesktopState::open(dir.path())` setup and the command shape (`schema_version: 1`, `expected_revision: 0`). Save two servers, `https://a.example` and `https://b.example`. Run `state.dispatch(..)` inside capture per § Capture standard. Assert one `client_options_saved` on `launcher.settings` at INFO with `count == "2"`, `custom_servers == "true"` and `client_patches_enabled` matching the command, and assert that no captured field contains `a.example`.
  - `settings_events.rs` inline tests:
    - each helper under capture (event, level, target, fields);
    - `client_options_saved(&default_servers(), true)` gives `custom_servers == "false"`;
    - one real path: open a `DesktopState` in a tempdir, call `set_game_telemetry` with `true` twice (read its signature at `game_telemetry/mod.rs:115`), and assert exactly one `game_diagnostics_changed` with `opted_in == "true"`.
- **Checks:** D1; D2e; D3e `commands`; D3e `settings_events`; D3e `game_telemetry` (D3e is `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine <filter>`). Also run the source-scan test by name: D3e `only_the_allowed_files_mention_the_summary_consent_flag`.
- **Commit:** `LT-19: settings and consent change events`.

## LT-20: frontend swallowed errors through `ui_event`

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-20`, branch `launcher-telemetry/lt-20-ui-errors` (lane 3). Frontend only. **Uses:** `reportUiEvent` (LT-10, merged).
- **Rules (inline):**
  1. Start every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-20;`. Use PowerShell only, in the foreground. No cargo.
  2. Edit only the files below. One commit with the subject below and the trailer lines from your brief.
  3. No new UI text, and no visible behaviour changes. Each report is an addition beside the existing handling.
  4. `event` and `code` must match `^[a-z][a-z0-9_]{0,39}$`; the native side drops anything else. `area` must be one of the `UiArea` literals at `ui-event.ts:5`. If an area named below is missing there, return `blocked`; do not add one.
  5. Tests use `node:test` and linkedom, not vitest. To observe `reportUiEvent`, a test sets `globalThis.window={__TAURI__:{core:{invoke}}}` and records the calls, as `ui-event.test.ts:6-11` (`fakeTauri`) and `telemetry-status-view.test.ts:61-71` do. Assert the call `['ui_event',{area,event,code,level:'warn'}]`, and delete the fake window in `finally`. Each new test must fail when its source change is reverted.
  6. Every test file you touch must be listed in the `test` script of `frontend/package.json`. If one is missing, add it and name `package.json` in your report.
- **What changed from the original LT-20:** the site list is cut to the swallowed errors a test can drive. Dropped, with the reason:
  - `client-maintenance-view.ts:93`, `patch-notes.ts:62` and `seed-archive-view.ts:90` are dispose, abort or teardown only;
  - `view.ts:100-103`, `install-view.ts:137` and `:145` are defect-only catches after `Effect.result`;
  - `install-view.ts:136` is a typed observe failure that is hard to drive (follow-up F-W4-1 in the report);
  - `game-update-workflow.ts:14` already passes string codes through.

  Neither workflow file has an "interruption only" comment, so that rule is void.
- **Files** (under `crates/launcher/desktop/frontend/src/`): `ui-event.ts`, `client-patches-view.ts`, `server-list-view.ts`, `log-upload-view.ts`, `physx-view.ts`, `launch-workflow.ts`, and the tests `ui-event.test.ts`, `client-patches-view.test.ts`, `server-list-view.test.ts`, `log-upload-view.test.ts`, `physx-view.test.ts`, `launch-view.test.ts`.
- **Change:**
  1. `ui-event.ts`: add `export function uiCode(error:unknown,fallback='transport'):string`. It returns `error` when it is a string matching the rule-4 pattern, else `fallback`.
  2. Sites:

     | Site | Today | Add |
     |---|---|---|
     | `client-patches-view.ts:62` | `await context.refresh().catch(()=>{})` | `.catch(e=>reportUiEvent('client_patches','refresh_failed',uiCode(e)))` |
     | `server-list-view.ts:97` | the same | `.catch(e=>reportUiEvent('server_list','refresh_failed',uiCode(e)))` |
     | `log-upload-view.ts:46` | `catch{return {error:'schema'};}` | `reportUiEvent('log_upload','status_decode_failed','schema');` before the `return` |
     | `physx-view.ts:60` | `.catch(()=>{throw 'schema';})` | `reportUiEvent('physx','status_decode_failed','schema');` before the `throw` |
     | `launch-workflow.ts:17` | `catch:error=>new LaunchFailure({code:error==='launcher_too_old'?'launcher_too_old':'transport'})` | a block body: `if(error!=='launcher_too_old')reportUiEvent('launch','invoke_failed',uiCode(error));`, then return the same `LaunchFailure` |
- **Tests** (one new test per file; copy the named neighbour):
  - `ui-event.test.ts`: `uiCode` returns `'busy'` for `'busy'`, and `'transport'` for `'Bad Code'`, `{}`, `undefined` and a 41-character string.
  - `client-patches-view.test.ts`: copy the stale-revision test (:56-58, `mounted(save, refresh)`), with `refresh` rejecting `'transport'`. Assert one `ui_event` with `area:'client_patches'`, `event:'refresh_failed'`, `code:'transport'`.
  - `server-list-view.test.ts`: the same, from :83-85, with `area:'server_list'`.
  - `log-upload-view.test.ts`: with the `answers[request.command]` helper (:11-14), answer the status command with `{}`. Assert `('log_upload','status_decode_failed','schema')` and that the view still shows its schema error.
  - `physx-view.test.ts`: with the mount helper (:15), resolve the inspect to `{}`. Assert `('physx','status_decode_failed','schema')`.
  - `launch-view.test.ts`: copy the rejected-launch test at :41 with the invoke rejecting `'busy'`, and assert `('launch','invoke_failed','busy')`. A second case rejecting `'launcher_too_old'` asserts no `ui_event` call.
- **Checks:** F1: `npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend`, then `npm run check --prefix crates/launcher/desktop/frontend`, then `npm test --prefix crates/launcher/desktop/frontend`.
- **Commit:** `LT-20: report swallowed frontend errors to launcher telemetry`.

## LT-21a: log-upload events on `launcher.telemetry`

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-21`, branch `launcher-telemetry/lt-21-log-upload` (lane 4). LT-21b follows in the same worktree and the branch merges once. **Target:** `launcher.telemetry`. **Contract:** C6, C9.
- **Rules (inline):** rules 1-7 of LT-18, with the worktree `lt-21`. Every new `tracing::` call lives in the new `storage/log_upload/events.rs`, with `target: "launcher.telemetry"`.
- **What changed from the original LT-21:** split in two. The core `launcher-core/src/logs.rs:301` is dropped: `upload_blob` (:282) is called only at :592, inside `logs.rs`, never by the desktop engine, which PUTs through `log_upload/store.rs`.
- **Files** (under `crates/launcher/desktop/engine/src/storage/log_upload/`): new `events.rs` (≤180 lines with tests), declared `mod events;` after `mod store;` (`mod.rs:14`); `mod.rs` (280 lines); `store.rs` (131 lines).
- **Change:** replace the nine untargeted calls with helpers. Keep each call's level and control flow.

  | Site | Today | Helper, `event` and fields |
  |---|---|---|
  | `mod.rs:209` | `info!(?error, "…ledger not saved: an operation started")` | `ledger_skipped(reason)`: info `log_upload_ledger_skipped`, `reason = error.code()`. `ensure_no_active_operation` returns an error type that has `code()` (LT-14b); if not, use `"operation_active"` |
  | `mod.rs:215` | `warn!(%error, "…ledger was not saved")` | `ledger_save_failed()`: warn `log_upload_ledger_save_failed`, `reason = "ledger_save_failed"` |
  | `mod.rs:234` | `warn!(left_out, max_bytes, …)` | `logs_too_large(left_out: usize, max_bytes: u64)`: warn `log_upload_too_large`, `count`, `bytes` |
  | `mod.rs:244` | `info!(left_out, max_bytes, …)` | `logs_trimmed(left_out, max_bytes)`: info `log_upload_trimmed`, `count`, `bytes` |
  | `mod.rs:254` | `debug!("…same content as a past upload")` | `nothing_new()`: debug `log_upload_nothing_new` |
  | `mod.rs:278` | `warn!(%error, …)` in `collect_failed` | `collect_failed(reason)`: warn `log_upload_collect_failed`, `reason` = the snake_case name of the `logs::LogError` variant (grep `pub enum LogError` in `crates/launcher-core/src/logs.rs`; one `match` arm per variant, in `events.rs`) |
  | `store.rs:75` | `warn!(status = …, "…refused")` | `rejected(http_status: u16)`: warn `log_upload_rejected`, `http_status` |
  | `store.rs:78` | `info!(blob = %blob_name, bytes, …)` | `stored(bytes: usize)`: info `log_upload_stored`, `bytes: u64`. **The blob name is dropped** (R5: it is a store path) |
  | `store.rs:115` | `warn!(reason = transport_reason(error), …)` | `failed(reason)`: warn `log_upload_failed`, `reason` |
- **Tests:** in `events.rs`, one capture test per helper, per § Capture standard, pinning event, level, target and fields. One real path: in `tests.rs`, run the existing over-the-cap flow (`over_the_cap_the_oldest_logs_are_left_out`, :265) under capture, and assert one `log_upload_trimmed` with `count == "1"` (or the count that test leaves out) and no `blob` field anywhere. If driving it under capture needs async, use `tokio::runtime::Builder::new_current_thread().enable_all().build()` and `block_on` inside the closure.
- **Checks:** D1; D2e; D3e `log_upload`.
- **Commit:** `LT-21a: log-upload events on launcher.telemetry`.

## LT-21b: the player log upload includes redacted launcher logs

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-21`, on top of LT-21a's commit (fresh coder). **Decision:** D-LT11, D-LT18. **Contract:** C9.
- **Rules (inline):** rules 1-7 of LT-18, with the worktree `lt-21`. The uploaded copy is scrubbed; the local files are never changed.
- **Files** (under `crates/launcher/desktop/engine/src/storage/log_upload/`):
  - new `launcher_logs.rs` (≤150 lines), declared `mod launcher_logs;` in `mod.rs`;
  - new `launcher_logs_tests.rs` (≤220 lines), declared at the bottom of `launcher_logs.rs` as `#[cfg(test)] #[path = "launcher_logs_tests.rs"] mod tests;`;
  - `mod.rs`: the `LogCollection` struct (:119-123), its construction (:189-193) and `collect` (:257).

  The engine already depends on `zip` 9 (`engine/Cargo.toml`); if it does not, return `blocked`.
- **Facts:**
  - The shell writes launcher logs to `<app_data>/logs` as `launcher.<yyyy-mm-dd>.log` (`shell/src/main.rs:579`, `shell/src/telemetry.rs:28`).
  - The state directory is `<app_data>/state` (`main.rs:581`), and the engine sees it as `self.directory.root`, so the launcher log folder is `self.directory.root.parent()` joined with `logs`.
  - `Redaction` is `crate::storage::game_telemetry::launcher_event::Redaction` (`from_env()` :82, `scrub()` :109, which runs `replace_home` first). `scrub_ips` is `crate::storage::game_telemetry::ip_scrub::scrub_ips` (:23) and writes `[ip]`.
- **Change:**
  1. `launcher_logs.rs`:
     - `pub(super) const LAUNCHER_LOG_CAP: usize = 4 * 1024 * 1024;`
     - `pub(super) fn entries(dir: &Path, redaction: &Redaction, cap: usize) -> Vec<(String, Vec<u8>)>`. It takes the files in `dir` whose names start `launcher.` and end `.log`, keeps the three greatest by name (the date sorts), and orders them oldest first. It reads each one (a read error skips the file), decodes it with `String::from_utf8_lossy`, and maps each line to `scrub_ips(&redaction.scrub(line))`. While the total bytes are over `cap`, it drops the first line of the oldest remaining file, and drops a file once it is empty. The entry name is `launcher/<file name>`. A missing `dir` gives an empty list.
     - `pub(super) fn append(body: Vec<u8>, entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>, LogUploadError>`. It opens `zip::ZipWriter::new_append(std::io::Cursor::new(body))`, writes each entry with the same file options `launcher-core/src/logs.rs:230-252` uses, then `finish()`es and returns the inner `Vec`. A zip error maps to `LogUploadError::CollectFailed`.
  2. `mod.rs`: add `launcher_logs: Option<PathBuf>` and `redaction: Redaction` to `LogCollection`. Set them at :189-193 to `self.directory.root.parent().map(|p| p.join("logs"))` and `Redaction::from_env()`. At :257, after `let Some(body) = selection.zip(..)`, run `let body = launcher_logs::append(body, &entries)?;`, where `entries` is `launcher_logs::entries(dir, &self.redaction, LAUNCHER_LOG_CAP)` when `launcher_logs` is `Some`, else empty.
  3. Known limits, written into the module doc: the digest and the "nothing new" check still cover only the game logs, and with no game logs nothing is uploaded.
- **Tests** (`launcher_logs_tests.rs`; a `tempfile::tempdir()` holds the logs; `Redaction { home: Some(<tempdir path>), user: None }`):
  - `entries_scrub_home_and_ips_and_keep_three_newest`: write four files, `launcher.2026-10-18.log` through `launcher.2026-10-21.log`. The newest holds `<home>\x\launcher.toml read` and `connect to 198.51.100.7:8081 failed`. Assert three entries, `launcher/launcher.2026-10-19.log` first; the newest holds `[ip]:8081` and no `198.51.100.7` or home path; the local file is unchanged. Reverting the `scrub_ips` call puts the address back and fails the test.
  - `entries_drop_the_oldest_lines_over_the_cap`: two files at 10 lines each with `cap` = 15 lines' bytes. The oldest file loses its first lines; the newest is whole.
  - `append_adds_launcher_entries_to_the_game_zip`: append to a zip holding `Binaries/a.log`. `zip::ZipArchive` lists both names.
  - `entries_without_a_folder_are_empty`.
- **Checks:** D1; D2e; D3e `log_upload`.
- **Commit:** `LT-21b: include redacted launcher logs in the player log upload`.

## LT-22: https-only summary endpoint from the login server or the built-in default

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-22`, branch `launcher-telemetry/lt-22-summary-endpoint` (lane 5). LT-23a and LT-23b follow in the same worktree. **Decision:** D-LT4 (owner directive 5).
- **Rules (inline):** rules 1-3 and 7 of LT-18, with the worktree `lt-22`. No logging in this packet.
- **What changed from the original LT-22:** `parse` is **not** https-only. It accepts loopback `http` for local fixtures, and the summary tests and mock servers (`http://127.0.0.1:…`) depend on that. Keep `parse` unchanged; the https rule lives in the new `for_login_server`. Tests are inline in `endpoint.rs` (:61-135), not under `tests/`.
- **Files:** `crates/launcher/desktop/engine/src/storage/launcher_summary/endpoint.rs` (135 lines); `launcher_summary/mod.rs` :54, one `pub use` line.
- **Change:**
  1. `pub const DEFAULT_SUMMARY_BASE: Option<&str> = None;`, with the doc comment "The built-in https base (`https://<host>/api/`); LT-41 sets it. Never a plain-http value." Re-export it next to `SummaryEndpoint` at `mod.rs:54`.
  2. `impl SummaryEndpoint`:
     - `pub fn for_login_server(login_server: &str) -> Option<Self>`: `Url::parse(login_server).ok()?`; return `None` unless `scheme() == "https"`, so even loopback http is refused here. Build `format!("https://{}{}/api/", host_str, port.map(|p| format!(":{p}")).unwrap_or_default())` and pass it through `Self::parse`. The login server's own path is ignored.
     - `pub fn resolve(first_login_server: Option<&str>) -> (Option<Self>, &'static str)`. Return `for_login_server` with `"login_server"` when it gives `Some`. Otherwise return `DEFAULT_SUMMARY_BASE` through `parse` with `"default"` when that gives `Some`, else `(None, "no_https_endpoint")`.
  3. Fix the struct doc at :6 so it says that loopback `http` is for fixtures only and that login servers must be https.
- **Tests** (inline, new):
  - `an_https_login_server_gives_its_api_base`: `https://play.example:8443` gives `https://play.example:8443/api/telemetry/launcher-summary`, and `https://play.example/login` gives `https://play.example/api/telemetry/launcher-summary`.
  - `plain_http_login_servers_are_refused`: `http://play.example:8081`, `http://127.0.0.1:8081` and `http://localhost` give `None`. Reverting the scheme check fails this.
  - `other_schemes_and_bad_values_are_refused`: `ftp://x`, `play.example`, `https://user@play.example` and `""` give `None`.
  - `resolve_falls_through_to_no_https_endpoint`: `resolve(Some("http://play.example:8081"))` and `resolve(None)` give `(None, "no_https_endpoint")` while the default is `None`. `resolve(Some("https://play.example"))` has source `"login_server"`.
  - `the_default_base_is_none_or_https`: `assert!(DEFAULT_SUMMARY_BASE.is_none_or(|b| b.starts_with("https://")))`.
  - The two existing tests stay unchanged.
- **Checks:** D1; D2e; D3e `launcher_summary`.
- **Commit:** `LT-22: https-only launcher-summary endpoint from the login server or the built-in default`.

## LT-23a: the shell resolves the summary endpoint

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-22`, on top of LT-22's commit (fresh coder). **Needs:** LT-22 (same branch).
- **Rules (inline):** rules 1-4 and 7 of LT-18, with the worktree `lt-22`.
- **Files:** `crates/launcher/desktop/shell/src/host/summary.rs` (415 lines: `start` :14-16, `config` :18-26, the module doc :1-3, and the tests). Not `host.rs`: `summary::start(&state)` at `host.rs:229` already passes the state.
- **Change:**
  1. `fn first_login_server(state: &DesktopState) -> Option<String>`. It uses the list Play uses: `let snapshot = state.inspect();` then `snapshot.adopted_login_servers.unwrap_or(snapshot.preferences.login_servers)`, or `cimmeria_launcher_engine::client_setup::login_servers::default_servers()` when that list is empty. It returns the first entry's `url`. This mirrors `engine/src/storage/launch/preparation.rs:17-23`.
  2. `fn config(first_login_server: Option<&str>) -> (SummaryConfig, &'static str)`: `endpoint` comes from `launcher_summary::SummaryEndpoint::resolve(first_login_server)`, and the source is returned beside it.
  3. `start`: `let first = state.lock().ok().and_then(|s| first_login_server(&s));`. Drop the guard before calling `launcher_summary::start`. Then `let (config, source) = config(first.as_deref());`, then `tracing::info!(target: "launcher.pipeline", event = "summary_endpoint_resolved", outcome = source, "launcher summary: endpoint source resolved");` (no URL), then `launcher_summary::start(state, config);`.
  4. Every other `config()` call in the file (the tests at :330-370) becomes `config(None).0`. Update the doc comments at :1-3 and :22-23: the endpoint comes from the first login server when it is https, else from the built-in default, else none.
  5. The endpoint is resolved once at start. A server-list change applies from the next launcher start; say so in the doc comment on `start`.
- **Tests:**
  - Rename `this_build_configures_no_endpoint` (:330) to `this_build_derives_an_https_endpoint_from_the_first_login_server`, asserting:
    - `config(Some("https://play.example"))` gives `https://play.example/api/telemetry/launcher-summary` with source `"login_server"`;
    - `config(Some("http://play.example:8081"))` gives `None` and `"no_https_endpoint"`;
    - `config(None)` gives `None` and `"no_https_endpoint"`.
  - New `the_first_login_server_is_the_default_list_when_none_is_saved`: open a fresh `DesktopState` the way the tests at :356-370 do, and assert `first_login_server(&state)` equals the first `default_servers()` url.
- **Checks:** D1; D2s (`pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --all-targets -- -D warnings`); D3s `summary` (`pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop summary`).
- **Commit:** `LT-23a: resolve the launcher-summary endpoint from the first login server`.

## LT-23b: the setup-diagnostics box says whether it sends

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-22`, on top of LT-23a (fresh coder). **Decision:** D-LT13 (first-press feedback). **Rules R8:** the Rust DTO field and its TS schema land in this one commit.
- **Rules (inline):** rules 1-4 and 7 of LT-18, with the worktree `lt-22`. Also:
  - frontend tests use `node:test` and linkedom;
  - the TS decoder runs with `onExcessProperty:'error'`;
  - no file may contain `launcher_summary_consent` unless it already does.
- **What changed from the original LT-23:**
  - Keep the HTML `disabled` attribute: `view.ts:75` sets `checkbox.disabled = !enabled` once a status loads, so the box is already clickable.
  - The DTO is the engine `NativeSnapshot` (`commands.rs:29-40`), and the frontend needs only "sends or not", so the field is a boolean, not the three-way source.
- **Files:**
  - `engine/src/storage/launcher_summary/mod.rs` (228 lines);
  - `engine/src/commands.rs` (:29-40, :60-68 only);
  - `engine/src/storage/launcher_summary/tests/batch_seam.rs` (211 lines);
  - `frontend/src/contract.ts` (:14-28);
  - `frontend/src/view.ts` (:133);
  - `frontend/ui/index.html` (:13);
  - `frontend/src/view.test.ts`.

  All paths are under `crates/launcher/desktop/`.
- **Change:**
  1. `launcher_summary/mod.rs`, in `impl DesktopState`: `pub fn summary_endpoint_configured(&self) -> bool { !matches!(lock(&self.summaries).gate(), Gate::NoEndpoint) }`. The gate is `NoEndpoint` exactly when no endpoint was configured (`tracker.rs:147-149`).
  2. `commands.rs`: add `pub setup_diagnostics_endpoint: bool,` to `NativeSnapshot` after `launcher`, with the doc "Whether setup diagnostics have an https endpoint to go to". In `inspect` add `setup_diagnostics_endpoint: self.summary_endpoint_configured(),`.
  3. `contract.ts`: in `NativeSnapshot`, after `launcher`, add `setup_diagnostics_endpoint: Schema.optional(Schema.Boolean),` with a comment. Use the optional style `contract.ts` already uses if it has one (grep `Schema.optional`). Optional keeps every existing test fixture valid.
  4. `view.ts:133`: the done text depends on the choice:
     - `consent && current.native?.setup_diagnostics_endpoint`: `'Diagnostics choice saved. Setup diagnostics are sent over https to your login server.'`
     - `consent` with no endpoint: `'Diagnostics choice saved. Setup diagnostics are saved but not sent yet: this server has no secure (https) address for them.'`
     - not `consent`: `'Diagnostics choice saved. Nothing is sent.'`
  5. `index.html:13`: change the `<small>` text to `Anonymous install and update summaries (no names, paths or addresses), sent over https to your login server. Off by default; game logs are separate.` Keep the label "Share setup diagnostics". Grep `sends nothing` under `frontend/`: any test asserting the old text is updated in this commit.
- **Tests:**
  - `batch_seam.rs`: `the_snapshot_says_whether_summaries_have_an_endpoint`. After `configure_summaries(config(None))`, `state.inspect().setup_diagnostics_endpoint` is false; after `config(Some(ENDPOINT))`, it is true.
  - `view.test.ts`: copy `'consent waits for native acknowledgement…'` (:87):
    - With `setup_diagnostics_endpoint:false` in the snapshot, one click on `#telemetry` and the save resolving leaves `#status` matching `/saved but not sent yet/`.
    - With `true`, `#status` matches `/sent over https/`.
    - A copy test reads the page and asserts that the `#telemetry` label contains `sent over https` and that no text says `This build sends nothing`.
    - Reverting the `view.ts` change fails the first case.
- **Checks:** D1; D2e; D3e `launcher_summary`; D2s; D3s `summary`; F1 (`npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend`, `npm run check --prefix …`, `npm test --prefix …`).
- **Commit:** `LT-23b: setup-diagnostics box says whether summaries are sent`.
