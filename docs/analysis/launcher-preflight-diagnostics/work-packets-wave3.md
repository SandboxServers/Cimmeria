# Launcher preflight diagnostics: work packets (waves 3-4, close-out)

Ledger: [README.md](README.md). The rules R1-R10, the checks table and the contracts C-PD1 to C-PD5 are in [work-packets.md](work-packets.md); **every packet here follows them, so read § Rules, § Checks and § Contract there first.** Display packets PD-22 to PD-30: [work-packets-display.md](work-packets-display.md).

Paths: `E` = `crates/launcher/desktop/engine/src/`, `S` = `crates/launcher/desktop/shell/src/`, `FE` = `crates/launcher/desktop/frontend/`, `LD` = `crates/launcher/desktop/docs/`. Anchors were taken at `main` `2019922fe`; waves 1-2 move some of them. Re-grep and return `blocked` when one no longer holds.

## Dispatch

| Wave | Packets and order | Ships as |
|---|---|---|
| 3 | Starts after P2 (wave 2) merges. PD-13, PD-14, PD-15 and PD-17 in parallel, each from the integration tip. PD-16 after PD-13 and PD-15 merged in. PD-18 last, after the other five merged in. | Integration branch `launcher-preflight-diagnostics/causes` (worktree `pd-w3`) from `origin/main`; `merge-noff.sh` per packet; one PR (P3). |
| 4 | PD-41 any time after wave 1 (independent). When wave D and wave M have merged: PD-29, then PD-30 from the PD-29 tip; PD-37; PD-27 after UAT-PD8. PD-36 after PD-24, PD-29 and PD-30. PD-28 moved to wave M. | One PR per packet. |
| 5 (gated) | PD-19 (after D-PD4), PD-20 (after D-PD6), PD-39 (after PD-38 and UAT-PD12; D-PD22 if it needs a hook), PD-40 (after UAT-PD10 and D-PD23). | One PR per packet. |
| Close-out | Campaign review loop on `origin/main`, then PD-21. | PD-21 is one PR. |

## Wave 3: every failure logs its cause (integration branch, PR P3)

### PD-13: log every `JobError` and every silent launch refusal (F13; requirement 1)

- **Host:** mac (hands-on). **Agent:** packet-coder role. **Worktree:** `pd-13`, branch `launcher-preflight-diagnostics/pd-13-command-failures`.
- **Files:** `S/host/command_failure.rs` (new), `S/host/command_failure_tests.rs` (new), `S/host.rs` (`mod` line), `S/main.rs` (634 lines: call lines only; it must stay at or under 660, else return `blocked`), `E/storage/launch/mod.rs` (call lines), `E/storage/launch/events.rs` (one fn, if `launch_refused` from PD-02 can't take the extra field), `LD/launcher-telemetry.md`.
- **Do:**
  1. `command_failure.rs`: `#[derive(Clone, Copy)] pub(crate) enum CommandKind { Install, Repair, Launch, GameUpdate, Adoption, Settings, LauncherUpdate, Other }` and `pub(crate) fn log(kind: CommandKind, command: &'static str, error: JobError)`. One `tracing::warn!` per kind with a literal target: Install → `launcher.install`, Repair → `launcher.repair`, Launch → `launcher.launch`, GameUpdate → `launcher.game_update`, Adoption → `launcher.adoption`, Settings → `launcher.settings`, LauncherUpdate → `launcher.update`, Other → `launcher.ui`. Fields: `event = "command_failed"`, `command` (the tauri command's name, a literal), `error = error.code()`, `failure_code = error.failure().code()`. Message: `"launcher command: <command> refused; the launcher returned <error> to the window; nothing started"` built from literals only (no `format!` of user data). `busy` and `stale_revision` log at `info` (they are routine double-clicks), everything else at `warn`.
  2. `S/main.rs`: in every `#[tauri::command]` that returns `Result<_, JobError>`, one call line on the error path. Where an `if let Some(error) = result.as_ref().err().copied()` block exists (for example `install_command` `:160-169`, `launch_command` `:256-264`), put the call inside it. Elsewhere add `if let Err(error) = &result { command_failure::log(.., *error) }` as one line before the return. List every command and its kind in the report.
  3. Engine launch admission refusals, `E/storage/launch/mod.rs:93-225` (13 `return Err(..)` sites that log nothing): before each, call `events::launch_refused` with `reason = <the error's code()>` and a new field `check` naming the guard in snake_case (for example `operation_open`, `plan_missing`, `revision_mismatch`; derive each name from the condition and list all 13 in the report). Don't change any condition or error.
  4. `LD/launcher-telemetry.md`: the `command_failed` event and the `check` field, outside any generated block.
- **Tests:** `command_failure_tests.rs`: `log(Install, "install_command", JobError::InvalidDirectory)` gives WARN `launcher.install` `event=command_failed error=invalid_directory failure_code=game_folder_invalid`; `Busy` gives INFO; one row per `CommandKind` checks the target. Engine: one refusal case in the existing launch tests (for example a second Play while one is open) asserts `event=launch_refused check=operation_open` (use the name you chose). Revert proof: removing the call line fails the engine test; mapping `Busy` to warn fails the level row.
- **Checks:** D1, D2e, D2s, D3e `launch`, D3s `command_failure`.
- **Docs rows:** 34, 57 (`LD/launcher-telemetry.md`).
- **Commit:** `feat(launcher): every refused command and Play admission logs why (PD-13)`

### PD-14: macOS runtime and Wine failures say why (F12; requirement 1)

- **Host:** mac (hands-on). **Mac gate:** yes. **Agent:** packet-coder role. **Worktree:** `pd-14`, branch `launcher-preflight-diagnostics/pd-14-mac-runtime-causes`.
- **Files:** `E/mac_runtime/download.rs`, `E/mac_runtime/mod.rs`, `E/mac_runtime/events.rs` (new), `E/mac_runtime/tests.rs`, `E/storage/install_worker/wine/mod.rs` (`preparation_outcome`, `:139-146`), `E/storage/install_worker/wine/tests.rs`, `E/mac_wine/mod.rs` (the Rosetta check, `:100-113`), `E/mac_wine/tests.rs`.
- **Do:**
  1. `download.rs:17-22`: split the check. A non-success HTTP status returns `RuntimeError::Network` and remembers the status; a declared length that differs from `SIZE` stays `Verification`. Return the status to the caller (add `http_status: Option<u16>` to the error path the way `catalog/events.rs:9` takes it; if `RuntimeError` must stay a unit enum, return it beside the error from `fetch` and log it there).
  2. `E/mac_runtime/events.rs`: `runtime_prepare_failed(error: &RuntimeError, http_status: Option<u16>)` WARN on `launcher.install`: `event`, `error` (closed snake name of the variant), `failure_code` (`Network` → `wine_runtime_download_failed`; `Verification`, `Extraction` → `wine_runtime_invalid`; `Busy` → `wine_runtime_busy`; `Io` → `local_io`), `http_status` when known. Not for `Cancelled`. Call it once in `prepare` (`mod.rs:96-134`) on every error return. Add `impl RuntimeError { pub fn failure(&self) -> Option<FailureCode> }` (None for `Cancelled`) and use it in both places.
  3. `install_worker/wine/mod.rs` `preparation_outcome`: unchanged outcomes (R10), plus a WARN `wine_preparation_failed` on `launcher.install` with `error` (`rosetta_required`, `invalid`, or the runtime error name) and `failure_code` (`rosetta_missing`, `wine_runtime_unavailable`, or `RuntimeError::failure()`), skipped for `Cancelled`.
  4. `mac_wine/mod.rs:100-113`: keep the result (a timeout or spawn error still returns `RosettaRequired`, R10), but log `rosetta_probe` with `result` = `present` / `missing` / `timed_out` / `spawn_error` (WARN unless present). Use `preflight::mac::classify_rosetta` (PD-08) to name the result.
- **Tests:** `download.rs` with `wiremock`: a 404 → `Network` with status 404; a 200 with a wrong `Content-Length` → `Verification`. `runtime_prepare_failed` fields pinned. `preparation_outcome(WineError::Runtime(Network))` still returns `RuntimeUnavailable` and logs `failure_code=wine_runtime_download_failed`. Revert proof: restoring the joined check turns the 404 into `Verification`.
- **Checks:** D1, D2e, D3e `mac_runtime`, D3e `install_worker`, MG (most of this is macOS-only code).
- **Docs rows:** 34, 57 (`LD/prerequisites.md` or `LD/wine-validation.md`, whichever describes the runtime download).
- **Commit:** `feat(launcher): Wine runtime and Rosetta failures on macOS log their cause (PD-14)`

### PD-15: install worker and catalog failures say why (F15; requirement 1)

- **Host:** mac (hands-on). **Agent:** packet-coder role. **Worktree:** `pd-15`, branch `launcher-preflight-diagnostics/pd-15-install-causes`.
- **Files:** `E/storage/install_worker/causes.rs` (new), `E/storage/install_worker/causes_tests.rs` (new), `E/storage/install_worker/mod.rs` (520 lines: call lines and the `mod` line only), `E/storage/install_worker/events.rs`, `E/catalog/mod.rs`, `E/catalog/events.rs`, `E/catalog/tests.rs`, and each caller of `catalog::fetch_release` that the purpose change touches (list them in the report).
- **Do:**
  1. `causes.rs`: `pub(crate) fn io_cause(error: &std::io::Error) -> &'static str` → `permission_denied`, `not_found`, `already_exists`, `directory_not_empty`, `read_only`, `storage_full`, `other` (from `ErrorKind`; `StorageFull`, `ReadOnlyFilesystem`, `DirectoryNotEmpty` are stable on the pinned toolchain; check, and fall back to `other` for any that is not). Pure.
  2. `install_worker/mod.rs:339`, `:373`, `:415`: where `destination_unavailable` is chosen, call `events::destination_unavailable(id, stage, cause)` (new fn in `events.rs`: WARN `launcher.install`, `operation`, `stage` = `create` / `check` / `promote`, `cause` from `io_cause`, `failure_code = "game_folder_not_writable"`). `:417`: keep the promote rename error and pass it to the same call instead of dropping it. Outcomes stay as they are (R10).
  3. Catalog: `release_fetch_failed` (`catalog/events.rs:9`) gains `purpose: Purpose` (`install`, `update`, `adopt`, `seed`, `preflight`), logged as a field and used in the message instead of the fixed "game update". Add `pub async fn fetch_release_for(purpose: Purpose)`; `fetch_release()` stays and means `Update`. Switch the install, adoption, seed and preflight (PD-10) callers to `fetch_release_for`.
- **Tests:** `io_cause` table; an install-worker test where the destination is a file, not a folder, logs `stage=create` (or `check`) with a closed `cause` and still returns `destination_unavailable`; `release_fetch_failed` with `Purpose::Install` logs `purpose=install`. Revert proof: removing the `:417` change makes the promote case log no cause (use the existing promote fixture; if there is none, say so).
- **Checks:** D1, D2e, D3e `install_worker`, D3e `catalog`, D2s (callers in the shell).
- **Docs rows:** 34, 57 (`LD/launcher-telemetry.md`).
- **Commit:** `feat(launcher): install and release-fetch failures log their cause (PD-15)`

### PD-16: install, repair and game-update copy through the taxonomy (F5; requirement 3)

- **Host:** mac (hands-on; frontend only). **Agent:** packet-coder role. **Worktree:** `pd-16`, branch `launcher-preflight-diagnostics/pd-16-copy`.
- **Files:** `FE/src/install-view.ts` (247 lines), `FE/src/install-view.test.ts`, `FE/src/game-update-view.ts`, `FE/src/game-update-view.test.ts`, `FE/src/repair-view.ts`, `FE/src/repair-view.test.ts` (create if missing, and add it to `package.json`), `FE/package.json` (only if a test file is new).
- **Do:**
  1. `install-view.ts:12-23`: replace the outcome-to-text map with `failureText(installOutcomes[outcome])` for the six failure outcomes (the `install_outcomes` map of C-PD1, imported from `failure-codes.json`); keep `content_prepared` and `cancelled` lines as they are. Show the fix as a second line.
  2. `install-view.ts:94`, `:98`, `:109-112` and `game-update-view.ts:16-25`: every place that turns a `JobError` code into text uses `failureText(jobErrors[code])`; an unknown code uses `unknown`.
  3. Replace "Unknown helper outcomes stay blocked" (trigger incident step 3) with the `recovery_required` message and fix.
  4. `repair-view.ts`: a repair install-result with ending `failed` and reason `repair_not_started` (PD-01, PD-03) shows the `repair_not_started` message and fix.
- **Tests:** each outcome renders its C-PD1 message and fix; a `job_error` of `invalid_directory` renders `game_folder_invalid`; an unknown code renders `unknown`; the "Unknown helper outcomes" string appears nowhere in the rendered DOM for any state. Revert proof: restoring the old map fails the outcome rows.
- **Checks:** F1.
- **Docs rows:** none (PD-18).
- **Commit:** `fix(launcher): install, repair and game-update failures say what happened and what to do (PD-16)`

### PD-17: game update preparation failures log a reason (F14; requirement 1)

- **Host:** mac (hands-on). **Agent:** packet-coder role. **Worktree:** `pd-17`, branch `launcher-preflight-diagnostics/pd-17-update-causes`.
- **Files:** `E/storage/update/preparation/mod.rs` (463 lines: call lines only), `E/storage/update/events.rs`, `E/storage/update/commit/mod.rs`, the update test file that covers preparation and commit.
- **Do:**
  1. `events.rs`: `prepare_failed(id, stage: &'static str, error: &IntentError)` WARN on `launcher.game_update`: `operation`, `stage`, `error` (`intent_code`, already in the file), and `failure_code` when the error maps to one (`StorageError::Io` → `local_io`, `InvalidDirectory` → `game_folder_invalid`, `Corrupt` → `state_corrupt`; otherwise omit).
  2. `preparation/mod.rs` sites `:249`, `:297-302`, `:326`, `:330`, `:339`: one call line each before the error returns, with a `stage` name per site (list them in the report).
  3. `commit/mod.rs:91` `events::committed(&result)`: log at WARN when `result` is an error (today it is INFO on both paths). Keep the fields.
- **Tests:** one preparation failure (the easiest site to reach with the existing fixtures) logs WARN `event=update_prepare_failed stage=<name>`; a failed commit logs WARN `committed`. Revert proof: removing the call fails the first test; INFO on error fails the second.
- **Checks:** D1, D2e, D3e `update`.
- **Docs rows:** 34.
- **Commit:** `feat(launcher): game update preparation and commit failures log a reason (PD-17)`

### PD-18: docs: contracts, player troubleshooting, privacy rows

- **Host:** mac (docs). **Agent:** `documentation-writer`. **Worktree:** `pd-18`, branch `launcher-preflight-diagnostics/pd-18-docs`, after PD-13 to PD-17 merged into the integration branch (and wave D merged into `main`).
- **Brief input:** D-PD5 (`os_error` yes or no).
- **Files:** `LD/launcher-telemetry.md`, `LD/privacy-inventory.md`, `LD/prerequisites.md`, `LD/launch.md`, `LD/repair.md`, `docs/client/launcher-guide.md`, `docs/guides/macos.md`, `crates/launcher/desktop/README.md` (only if it lists IPC commands), and `FE/ui/index.html` (the privacy disclosure paragraph, only if D-PD5 = yes and it doesn't already cover "error numbers").
- **Do:**
  1. `LD/launcher-telemetry.md`: every event this campaign added or changed (C-PD2, C-PD3, C-PD5, C-PD6, C-PD7, PD-13 to PD-17, PD-29, and C-PD8 if wave M has merged), with target, level and fields. Never edit a `<!-- gen:NAME -->` block; if the events table is generated, name the generator in the PR body instead.
  2. `LD/privacy-inventory.md`: `failure_code` (closed codes), `os_error` (if D-PD5 = yes: an integer OS error number), preflight and dependency-install events (closed codes, byte counts, elapsed time), display events (mode and window size), and the local-only Wine and DXVK logs from PD-28 (contain local paths; never uploaded).
  3. `LD/prerequisites.md`: the preflight model: what is checked per platform and operation, what blocks and what only warns (D-PD7), what the launcher installs and how (D-PD1, D-PD10), and the 15-minute Rosetta limit.
  4. `docs/client/launcher-guide.md` § troubleshooting (CRLF): a table of the 15 most common failures (rosetta_missing, rosetta_install_declined, wine_runtime_download_failed, app_translocated, disk_space_low, game_folder_missing, game_folder_not_writable, manifest_unreachable, game_files_invalid, repair_not_started, install_interrupted, spawn_permission_denied, game_exited_nonzero, game_window_hidden, operation_busy) with the C-PD1 message and fix, word for word from `FE/src/failure-copy.ts`.
  5. `docs/guides/macos.md` (CRLF): Rosetta is installed by the launcher with an administrator prompt; the game always runs in a window, so Cmd+Tab works on one or two displays; the display size setting; where the Wine and DXVK logs are.
- **Tests:** none (docs only). Run `python tools/docs-gen/regen.py --check` (exit 1 lists stale generated blocks), and check every new link resolves.
- **Checks:** `python tools/docs-gen/regen.py --check`; `git diff --stat` shows CRLF kept (R4).
- **Docs rows:** 34, 57 (this packet is them). Row 35 does not apply unless PD-19 ships.
- **Commit:** `docs(launcher): preflight, failure codes and troubleshooting (PD-18)`

## Wave 4: display follow-ups and WebView2 (one PR per packet)

PD-27, PD-29, PD-30: [work-packets-display.md](work-packets-display.md). PD-36, PD-37: [work-packets-mac-extras.md](work-packets-mac-extras.md).

### PD-41: install the WebView2 runtime instead of only linking to it (owner D1; Windows)

- **Host:** mac (hands-on) for the edit; every line is `cfg(windows)`, so **WG** verifies it, plus a Windows-PC UAT (a VM or PC without WebView2: the launcher installs it and opens). **Agent:** packet-coder role. **Worktree:** `pd-41`, branch `launcher-preflight-diagnostics/pd-41-webview2-install`, from `origin/main`.
- **Files:** `S/webview2.rs`, `S/webview2_install.rs` (new), `S/webview2_install_tests.rs` (new), `S/main.rs` (only if the call site changes), `LD/` page that documents the WebView2 check (grep `WebView2`).
- **Do:**
  1. `ensure_runtime_or_exit` (`S/webview2.rs:33`): when the runtime is missing, the existing `MessageBoxW` asks `Install the Microsoft Edge WebView2 Runtime now? It is about 2 MB and installs for your user account.` Yes runs the install; No keeps today's path (open the download page, then exit).
  2. `webview2_install.rs`: download the Evergreen Bootstrapper from `DOWNLOAD_URL` (`:13`) into a temp file with the HTTP client the shell already depends on (no new crate; if there is none usable before Tauri starts, return `blocked`). Refuse the file unless it starts with `MZ` (the link can serve a page); then run it with `/silent /install`, unelevated, which installs per user (Microsoft Learn, WebView2 distribution). Wait at most 5 minutes. Then re-read the `pv` values with `runtime_present_in`.
  3. Installed: continue startup. Not installed, timed out, or a download error: fall back to today's download-page path with the line `The automatic install didn't finish.` Never loop.
  4. If the tracing pipeline is already up at that point, emit C-PD5 `dependency_install_started`/`_finished` with `dependency=webview2` on `launcher.startup`; if it is not (likely: this runs before the window), skip the events and say so in the report.
- **Tests:** pure parts only: `is_executable(bytes)` (`MZ` yes, `<!DOCTYPE` no, empty no); the decision `after_install(present, outcome) -> Next` (`Continue` / `FallBack`). Revert proof: dropping the `MZ` check accepts the HTML sample.
- **Checks:** D1, D2s, D3s `webview2` (on the Mac only the pure tests compile; say so), WG.
- **Docs rows:** 57.
- **Commit:** `feat(launcher): install the WebView2 runtime when it is missing (PD-41)`

## Wave 5: gated follow-ups (one PR per packet)

PD-39 and PD-40: [work-packets-mac-extras.md](work-packets-mac-extras.md).

### PD-19: the summary wire carries the new codes (D-PD4)

- **Status:** **BlockedDecision** (D-PD4). If the owner keeps the recommended default (map onto the existing 18 `SummaryErrorCode` values, no wire change), this packet becomes: map every `FailureCode` to an existing `SummaryErrorCode` in `E/storage/launcher_summary/` with a table test, no schema change. If the owner wants new values, the architect re-specs it as a cross-workspace wire change (engine plus `cimmeria-admin-api` goldens, doc row 35, checks S1-S3).
- **Host:** mac (hands-on). **Agent:** packet-coder role, after the ruling. Verify: MG (the summary code is cross-platform).

### PD-20: Windows VC++ and DirectX checks (D-PD6)

- **Status:** **BlockedDecision** (D-PD6). If the owner says yes, a hands-on design session specs two `CheckId` values (`VcRuntime`, `DirectX`) with detection by registry key and system DLL presence, and an install action that runs the redistributable with the Windows elevation prompt (D-PD1). Recommended default: defer.
- **Host:** mac (hands-on); all code is `cfg(windows)`, so WG plus a Windows-PC UAT.

## Close-out

### PD-21: close-out

- **Host:** mac (docs). **Agent:** `documentation-writer`. **Worktree:** `launcher-preflight-diagnostics-closeout`, branch `launcher-preflight-diagnostics/pd-21-closeout`, after the campaign review loop has shipped.
- **Do:** update `docs/project-status.md` and the launcher lines of `docs/gap-analysis*` once (the only time this campaign touches them); add UAT-PD1 to UAT-PD13 that the owner still has to run to `docs/guides/unified-uat.md`; set every finished packet row in this README to Done; record open items (PD-19, PD-20, PD-27, PD-39, PD-40 if still blocked; the parking lot) in the README.
- **Checks:** `python tools/docs-gen/regen.py --check`; CRLF kept (R4).
- **Commit:** `docs: close out the launcher preflight diagnostics campaign (PD-21)`
- The orchestrator then retires the worktrees through the bash twins (`rm-worktree.sh`) and empties `.claude/state/active-campaign`.
