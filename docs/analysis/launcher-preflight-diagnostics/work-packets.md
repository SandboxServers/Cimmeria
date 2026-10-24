# Launcher preflight diagnostics: work packets (rules, contract, waves 1-2)

Ledger: [README.md](README.md). Display wave (D) and its follow-ups: [work-packets-display.md](work-packets-display.md). Waves 3-5 and the close-out: [work-packets-wave3.md](work-packets-wave3.md). The owner's D4 extras (wave M, PD-31 to PD-40): [work-packets-mac-extras.md](work-packets-mac-extras.md). Every packet re-greps its anchors (`file:line` was taken at `main` `2019922fe`) and returns `blocked` when an anchor no longer holds.

Paths: `E` = `crates/launcher/desktop/engine/src/`, `S` = `crates/launcher/desktop/shell/src/`, `FE` = `crates/launcher/desktop/frontend/`, `RP` = `crates/launcher/desktop/runtime-probe/src/`, `LD` = `crates/launcher/desktop/docs/`.

## Dispatch

| Wave | Packets, dispatch and merge order | Branching | Ships as |
|---|---|---|---|
| 1 | Dispatch PD-22, PD-01, PD-02 in parallel. When PD-01 has `ship`: PD-03 and PD-04 in parallel, both from the PD-01 branch tip. When PD-04 has `ship` and PD-02 has merged into `main`: PD-05 from the PD-04 tip, which must contain PD-02 (if PD-04 branched before PD-02 merged, rebase it onto `origin/main` first with `rebase-onto.sh`). When PD-05 has `ship`: PD-06 from the PD-05 tip. Merge order: PD-22, PD-01, PD-02, PD-04, PD-03, PD-05, PD-06. | Each packet on its own branch from `origin/main` or from the branch named. | One PR per packet (`ship.sh pr`, stacked with `--base <branch>` where a packet branches from another). |
| D | PD-23 first. When PD-23 has `ship`: PD-24 and PD-26 in parallel from the PD-23 tip. When PD-24 has `ship`: PD-25 from the PD-24 tip. Merge order: PD-23, PD-24, PD-25, PD-26. Runs at the same time as wave 2 (shared files: `FE/ui/index.html`, `FE/src/composition.ts`, `FE/package.json`, separate hunks). | Each packet on its own branch. | One PR per packet. |
| 2 | Starts after wave 1 has merged. PD-07 first, into the integration branch. Then PD-08 and PD-10 in parallel. Then PD-09 (after PD-08) and PD-11 (after PD-10). Then PD-12. Each packet branches from the integration tip after the packets before it merged in (`merge-noff.sh`). | Integration branch `launcher-preflight-diagnostics/preflight` (worktree `pd-w2`) from `origin/main`. | One PR (P2). |
| M | Mac extras (owner D4), beside waves 2 and 3: [work-packets-mac-extras.md § Dispatch](work-packets-mac-extras.md). PD-38 can start now. | Integration branch `launcher-preflight-diagnostics/mac-extras` (worktree `pd-wm`). | One PR (PM). |
| 3, 4, 5 | [work-packets-wave3.md](work-packets-wave3.md). | | |

- Each packet's worktree is `pd-NN` on branch `launcher-preflight-diagnostics/pd-NN-<slug>`. Retire it and its lane target folder as soon as it merges (into `main`, or into an integration branch, then `rm-worktree.sh --merged-into <PR>` once P2 or PM merges). No packet needs a live database.
- Review loops per the `campaign-packet` skill: packet loop (at most 3 passes), then the wave loop. Wave 1 ships one PR per packet, so its wave reviewer gets every packet's branch and range.
- PD-01 is the incident hotfix: ship it as soon as its packet loop says `ship`; don't wait for the wave loop. PD-22 likewise (the black-screen hotfix, D2).
- PD-05 gets one extra read-only pass from `testing-validation-engineer` (spawn classification is hard to test; the pass checks the revert proof). PD-09 gets a `network-security-auth` pass (it runs a privileged command).

### Which host can run what (D-PD19)

The owner chose to build the campaign from this Mac. `packet-coder`, `packet-reviewer`, `ops` and `architect` refuse darwin, so:

- **Build:** each packet runs in its own `claude --agent hands-on` session that plays the packet-coder role: it reads R1-R10 and its packet, works only in its worktree, and returns the R9 line. "Agent: packet-coder role" in a packet means exactly that. The orchestrator dispatches `hands-on` with the packet as the brief.
- **Git and shipping:** the bash twins in `tools/build-lane/` (`mk-worktree.sh`, `branch-at.sh`, `merge-noff.sh`, `commit-paths.sh`, `push-branch.sh`, `rebase-onto.sh`, `ship.sh`, `rm-worktree.sh`), run from a hands-on session in place of `ops`. `mk-worktree.sh` links `external/` on macOS.
- **Review:** a fresh hands-on session given the packet-reviewer brief, read-only by instruction (D-PD25). CI is the gate either way.
- **Windows-only code:** anything under `cfg(windows)`, the i686 helper and native Windows behaviour can't be compiled or run here. Those packets carry the CI gate that verifies them (WG, RG, WR below) and are not `done` until it is green; the reviewer checks the job. Behaviour a CI job can't see is a Windows-PC UAT (UAT-PD5, UAT-PD9, UAT-PD13).
- The README status table's **Verify** column names, per packet, the gate beyond the local Mac checks.

## Rules (every packet; packet-coder reads these)

- **R1 Host and shell.** `Host: mac (hands-on)`: a `claude --agent hands-on` session on the Mac, in the packet-coder role. Use bash. Start every command with `cd <worktree> &&`, use absolute paths, run it in the foreground and wait for it. Never call `cargo` directly; only through `bash tools/build-lane/lane.sh` (its state lives under `~/.local/share/cimmeria-build`; one lane host per checkout). Run the checks in the listed order, with at most 5 fix rounds per check. If `logs/<worktree>/` shows no new lane run, you ran from the wrong folder. A gate you can't run here (WG, RG, WR) is reported as `pending CI`, never as passed.
- **R2 Scope and git.** Edit only the files your packet names, plus the new files it names. No `git stash`, no `git worktree prune`, no push, no rebase. Make one local commit with the given subject, ending with:

  ```text
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  ```

- **R3 File caps.** Soft cap 500 lines, hard cap 700. A named file already over 450 lines gets only call lines, `mod` lines and the one-line field additions the packet lists; the logic goes in the new file the packet names. Tests go in a sibling `*_tests.rs` file (`#[cfg(test)] #[path = "..."] mod tests;` as the neighbours do). Report every touched file's line count.
- **R4 Line endings.** Files under `docs/` and `LD/` are CRLF; keep them CRLF. Code files keep their current line endings. `git diff --stat` must show only your own lines. Never hand-edit a generated block (`<!-- gen:NAME -->`); name it in the report instead.
- **R5 Privacy.** In any event field or message: never an absolute path, URL, host, IP, port, account or user name, token, session id, or an error's `Display`/`Debug`. Use closed snake_case codes, counts, sizes and integers. `os_error` (a raw OS error number) is allowed only where the packet says D-PD5 allows it.
- **R6 Log discipline.** `info` for one start and one outcome, `warn` for a recoverable failure the player sees, `error` for an unrecoverable one. At most 12 fields per event, snake_case keys, numbers as numbers. Never the key `count` (the server denies it). Use only the existing targets (the local names of `crates/launcher-core/src/telemetry/launcher_targets.txt`; this campaign uses `launcher.launch`, `launcher.install`, `launcher.repair`, `launcher.game_update`, `launcher.settings` and `launcher.startup`): a new target is a wire change (`crates/launcher-core/src/telemetry/launcher_targets.txt`, pinned by engine and admin-api goldens) and is out of scope (D-PD12). Follow the neighbours' message style: `"launcher repair: <what happened>; <what was expected>; <what happens next>"`, and `operation = crate::operation_numbers::number(id)`.
- **R7 Tests.** Unit tests ([TESTING.md](../../../TESTING.md)). Every new test must fail when your change is reverted; say in the report which assertion fails and why. Pin the level, target and `event`/`reason`/`failure_code` fields, not message text. Capture tracing only with `crate::test_capture` (the existing helper; never add a second one).
- **R8 Frontend.** The decoders use `onExcessProperty:'error'`: a new or changed Rust DTO field and its TS schema land in the same commit. Every new `*.test.ts` file is added to the `npm test` script in `FE/package.json` (line 7 lists each file). A visible button gives visible feedback on the first press. User copy comes from `FE/src/failure-copy.ts` (C-PD1); don't write failure text inline.
- **R9 Report and return.** Write `<worktree>\.claude\reports\launcher-preflight-diagnostics\<packet>-coder.md`: the files and line counts, each check and its result, the revert proof, and anything you rejected and why. Return exactly `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. When the packet does not cover something, return `blocked`; don't invent a design.
- **R10 Unchanged behaviour.** Everything the packet doesn't name behaves byte for byte as before: downloaded and adopted installs, the summary wire (`SummaryErrorCode`), existing event fields, the Windows game process and helper command line when no display argument applies.

### Checks (copy the exact command)

| ID | Command |
|---|---|
| D1 | `bash tools/build-lane/lane.sh cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all` |
| D2e | `bash tools/build-lane/lane.sh cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings` |
| D2s | the same with `-p cimmeria-launcher-desktop` |
| D2p | the same with `-p cimmeria-runtime-probe` |
| D3e `<f>` | `bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine <f>` (engine tests use `cargo test`; nextest cannot list that crate) |
| D3s `<f>` | the same with `-p cimmeria-launcher-desktop <f>` |
| D3p `<f>` | the same with `-p cimmeria-runtime-probe <f>` |
| F1 | `npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend`, then `npm run check --prefix crates/launcher/desktop/frontend`, then `npm test --prefix crates/launcher/desktop/frontend` |
| MG | Mac gate: the packet's own D2e/D3e runs on this Mac, and CI `native-engine (macos-latest)` green on the PR |
| WG | Windows gate: CI `native-engine (windows-latest)` in `.github/workflows/launcher-desktop.yml` (self-hosted `cimmeria-windows` pool) green on the PR; it compiles and tests every `cfg(windows)` path |
| RG | Helper gate: CI `runtime-probe` in `.github/workflows/launcher-runtime-probe.yml` (i686-pc-windows-msvc clippy, test, build, `--self-test`) green |
| WR | CI `desktop release build dry run (Windows)` green (builds and bundles the i686 helper) |

## Contract

### C-PD1: the failure taxonomy (PD-04 builds it; every packet uses it)

**Rust** `E/failure.rs` (new, cross-platform), `pub mod failure;` in `E/lib.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode { /* one variant per code below, in table order */ }
impl FailureCode {
    pub const ALL: &'static [FailureCode];          // every variant, table order
    pub fn code(self) -> &'static str;              // the serde name
    pub fn log_message(self) -> &'static str;       // the "Log message" column; no path, URL or placeholder
}
```

**Fixture** `FE/src/failure-codes.json` (new), read by the Rust tests with `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../frontend/src/failure-codes.json"))` and by the TS tests with `readFileSync`:

```json
{"schema_version":1,
 "codes":["rosetta_missing", "...every code, table order..."],
 "job_errors":{"unsupported_schema":"launcher_too_old", "...the JobError map below..."},
 "install_outcomes":{"destination_unavailable":"game_folder_not_writable", "...the Outcome map below..."}}
```

**TS** `FE/src/failure-codes.ts` (new): `export const FailureCode = Schema.Literals([...])` with the codes in table order, and `export type FailureCode`. `FE/src/failure-copy.ts` (new): `export const failureCopy: Record<FailureCode, {message: string; fix: string}>` and `export const failureText = (code: FailureCode, facts?: {needed?: number; free?: number}) => {message, fix}`, which fills `{needed}` and `{free}` (bytes shown as `x.x GB`) and nothing else.

| Code | Log message (Rust) | User message (TS) | Suggested fix (TS) |
|---|---|---|---|
| `rosetta_missing` | Rosetta 2 is not installed | Rosetta 2 is not installed. The game's compatibility layer needs it on Apple silicon Macs. | The launcher can install it from Apple. macOS will ask for your administrator password. |
| `rosetta_check_timed_out` | the Rosetta check did not answer in time | The launcher couldn't confirm whether Rosetta 2 is installed. | Press Check again. If it keeps happening, restart your Mac. |
| `rosetta_install_failed` | the Rosetta install did not complete | Rosetta 2 could not be installed. | Open Terminal and run `softwareupdate --install-rosetta --agree-to-license`, then press Check again. |
| `rosetta_install_declined` | the administrator prompt was cancelled | Rosetta 2 was not installed because the administrator prompt was cancelled. | Press Install again and enter your Mac password when macOS asks. |
| `wine_runtime_missing` | the compatibility runtime is not downloaded | The game's compatibility runtime (Wine) is not on this Mac yet. | The launcher will download it (64 MB) from the WoWSilicon release page. |
| `wine_runtime_download_failed` | the compatibility runtime download failed | The compatibility runtime could not be downloaded. | Check your internet connection and try again. |
| `wine_runtime_invalid` | the compatibility runtime failed verification | The downloaded compatibility runtime was damaged or incomplete. | Try again; the launcher downloads a fresh copy. |
| `wine_runtime_busy` | the compatibility runtime is in use | Another launcher task is using the compatibility runtime. | Wait for it to finish, then try again. |
| `wine_runtime_unavailable` | the compatibility runtime could not be prepared | The compatibility runtime could not be prepared. | Try again. If it keeps failing, send logs from Settings. |
| `helper_unavailable` | the bundled launch helper is missing | Part of the launcher is missing (the game launch helper). | Download the launcher again and replace this copy. |
| `app_translocated` | the launcher runs from a translocated location | macOS is running the launcher from a temporary read-only location. | Move the launcher to your Applications folder and open it from there. |
| `physx_missing` | the PhysX runtime is not installed | The PhysX runtime the game needs is not installed. | The launcher can install it for you. Windows may ask for permission. |
| `physx_install_failed` | the PhysX install did not complete | The PhysX runtime could not be installed. | Run the PhysX installer again from Settings, or restart Windows and try again. |
| `disk_space_low` | not enough free disk space | There isn't enough free disk space: about {needed} is needed and {free} is free. | Free up space on that drive, or choose a game folder on another drive. |
| `game_folder_not_chosen` | no game folder is chosen | No game folder is chosen yet. | Choose a game folder in Settings. |
| `game_folder_missing` | the game folder is missing | The game folder can't be found. It may have been moved, renamed or deleted. | Choose the folder again in Settings, or reinstall the game. |
| `game_folder_invalid` | the chosen folder cannot be used | The chosen folder can't be used for the game. | Choose an empty folder you own, not a system folder. |
| `game_folder_not_writable` | the game folder is not writable | The launcher can't write to the game folder. | Choose a folder you own, or give your user permission to change it. |
| `game_files_invalid` | the game files failed verification | Some game files are missing or changed. | Run Repair from Settings. |
| `state_corrupt` | launcher state failed validation | The launcher's saved state is damaged. | Restart the launcher. If it keeps happening, send logs from Settings. |
| `local_io` | a local file operation failed | The launcher couldn't read or write a file on this computer. | Check that the drive is connected and has free space, then try again. |
| `manifest_unreachable` | the release service did not answer | The launcher couldn't reach the game's release service. | Check your internet connection and try again. Play still works offline. |
| `manifest_invalid` | the release manifest failed validation | The release information the launcher received was not valid. | Try again later. |
| `signing_key_unavailable` | the release signing key is unavailable | This launcher can't verify game releases. | Download the launcher again from the official page. |
| `download_failed` | a game download failed | The game download failed. | Check your internet connection and try again; finished parts are kept. |
| `operation_busy` | another operation owns the game | Another launcher task is still running. | Wait for it to finish, or cancel it. |
| `status_stale` | the status changed underneath the request | The launcher's status changed while you were acting on it. | Press the button again. |
| `identity_conflict` | the request does not match the current operation | That request doesn't match what the launcher is doing now. | Press the button again. |
| `recovery_required` | an interrupted operation needs recovery | An earlier task was interrupted and needs to be finished first. | Use Recover or Abandon in Settings. |
| `persistence_uncertain` | the launcher could not confirm a save | The launcher couldn't confirm it saved its state. | Restart the launcher to check the state again. |
| `repair_not_started` | the repair stopped before it started | The repair stopped before it changed anything. Nothing on disk was touched. | Press Repair again. If it fails again, the message names the cause. |
| `install_interrupted` | the install was interrupted | The install was interrupted. | Press Install to resume it, or Abandon it in Settings. |
| `install_failed` | the install failed | The install failed. | Press Install to try again; finished parts are kept. |
| `launcher_too_old` | this launcher is too old | This launcher is too old for the current game release or its saved files. | Update the launcher from Settings. |
| `platform_unsupported` | the action is not available on this platform | This isn't available on this computer's setup. | Check the system requirements in the player guide. |
| `launch_resources_missing` | the bundled Play resources are missing | Part of the launcher needed to start the game is missing. | Download the launcher again and replace this copy. |
| `login_servers_invalid` | the login server list is not valid | A login server in Settings is not valid. | Fix or reset the Login servers setting. |
| `launch_prepare_failed` | client setup before Play failed | The launcher couldn't prepare the game files before starting. | Run Repair from Settings. |
| `spawn_failed` | the game process could not be created | The game could not be started. | Try again. If it keeps failing, send logs from Settings. |
| `spawn_permission_denied` | the OS refused to start the game process | Your computer refused to start the game. | Allow the launcher in your security software, or on a Mac in System Settings, then try again. |
| `spawn_not_found` | the game program was not found | The game program can't be found. | Run Repair from Settings. |
| `game_exited_nonzero` | the game exited with an error code | The game closed with an error. | Try again. If it keeps happening, send logs from Settings. |
| `game_status_unknown` | the game's state could not be determined | The launcher lost track of the game. | If the game is still open, close it, then press Play. |
| `game_window_hidden` | the game window is not visible | The game is running but its window isn't visible. | Switch the display mode to Windowed in Settings and start the game again. |
| `display_settings_not_applied` | the display mode could not be applied | The launcher couldn't apply your display mode for this start. | The game uses its own last setting this time. Check the game folder permissions. |
| `unknown` | an unclassified failure | Something went wrong that the launcher doesn't recognise. | Try again. If it keeps happening, send logs from Settings. |

`job_errors` map (every `JobError` variant, `S/host/install/contract.rs:106-122`, by its serde name): `unsupported_schema`→`launcher_too_old`, `platform_unavailable`→`platform_unsupported`, `launcher_too_old`→`launcher_too_old`, `io`→`local_io`, `corrupt_state`→`state_corrupt`, `invalid_directory`→`game_folder_invalid`, `stale_revision`→`status_stale`, `busy`→`operation_busy`, `unknown_operation`→`status_stale`, `identity_conflict`→`identity_conflict`, `recovery_required`→`recovery_required`, `persistence_uncertain`→`persistence_uncertain`, `manifest_unavailable`→`manifest_unreachable`, `invalid_manifest`→`manifest_invalid`, `signing_key_unavailable`→`signing_key_unavailable`.

`install_outcomes` map (`FE/src/install-workflow.ts:11`, failures only): `destination_unavailable`→`game_folder_not_writable`, `install_failed`→`install_failed`, `content_invalid`→`game_files_invalid`, `reconciliation_required`→`install_interrupted`, `rosetta_required`→`rosetta_missing`, `runtime_unavailable`→`wine_runtime_unavailable`.

Launch not-started reasons (PD-05): `verify_failed`→`game_files_invalid`, `login_servers_refused`→`login_servers_invalid`, `prepare_failed`→`launch_prepare_failed`, spawn errors per `classify_spawn` (C-PD3), a dispatch that failed before anything was recorded (the PD-02 path) → `JobError::from(error).failure()` (the `job_errors` map, e.g. `io`→`local_io`).

### C-PD2: pre-start repair and launch failures (PD-01, PD-02, PD-03)

- A repair or launch whose dispatch fails while the operation is still `Starting` (and, for launch, with no launch record yet) goes to `Failed`, logs why, and queues the result. `mark_uncertain` stays only for: `PersistenceUncertain`, an operation that is no longer `Starting`, and a launch that already has a launch record (D-PD2).
- Events (all `operation = crate::operation_numbers::number(id)`):

| Event | Target, level | Fields | Packet |
|---|---|---|---|
| `repair_not_started` | `launcher.repair`, WARN | `operation`, `reason` (the `JobError` code, e.g. `corrupt_state`), `failure_code` (C-PD1, from `JobError::failure()` once PD-04 lands; PD-01 sends `repair_not_started`) | PD-01 |
| `repair_reconciled_unstarted` | `launcher.repair`, INFO | `operation`, `outcome = "failed"` | PD-03 |
| `launch_not_started` (existing) | `launcher.launch`, WARN | existing `operation`, `reason`; PD-02 sends `reason` = the `IntentError` code; PD-05 adds `failure_code` | PD-02, PD-05 |
| `launch_refused` | `launcher.launch`, WARN | `reason` = `resources_unavailable` (no bundled Play resources) or `resources_unresolved` (`resolve_play_resources` returned none) | PD-02 |

### C-PD3: the launch failure field (PD-05)

- `E/storage/launch/mod.rs` `Record` (`:60-64`, the `launch-result-{id}.json` record) gains `#[serde(default, skip_serializing_if = "Option::is_none")] failure: Option<FailureCode>`. Old records still read. `Observation::NotStarted` stays a unit variant (about 14 files match on it).
- `impl DesktopState` (in the new `E/storage/launch/failure.rs`): `pub(crate) fn note_launch_failure(&mut self, plan: &Plan, code: FailureCode) -> Result<(), IntentError>` sets `failure` on the current record without changing its observation; `pub fn launch_failure(&self) -> Result<Option<FailureCode>, IntentError>` reads it. `record_launch` keeps a `failure` that is already in the record. `pub(crate) fn record_not_started(&mut self, plan: &Plan, code: FailureCode) -> Result<(), IntentError>` writes one record, `observation: NotStarted` and `failure: Some(code)`, in a single `write_launch` after the same `launch_plan()? == Some(plan)` check `record_launch` makes.
- The PD-02 dispatch-failure path (`E/storage/launch/not_started.rs` `dispatch_failed`) writes **no** `launch-result` record as shipped, so `note_launch_failure` has nothing to tag and `launcher_summary` reports the failure as `unspecified` (`LD/launcher-summaries.md` error-code table). PD-05 makes `dispatch_failed` call `record_not_started` before `observe(id, Failed)`, the same record-then-observe order the worker's terminal commit uses (`E/storage/launch/worker.rs`, the `commit` closure).
- `pub(crate) fn classify_spawn(kind: std::io::ErrorKind, raw: Option<i32>, macos: bool) -> FailureCode`: `macos && raw == Some(86)` (EBADARCH, "Bad CPU type in executable") → `rosetta_missing`; `NotFound` → `spawn_not_found`; `PermissionDenied` → `spawn_permission_denied`; anything else → `spawn_failed`. Pure, cross-platform, so its tests run on Windows.
- `S/host/launch/mod.rs` `LaunchStatus` (`:36-50`) gains `failure: Option<FailureCode>` (serialized as `null` when none). `FE/src/launch-workflow.ts` adds `failure: Schema.NullOr(FailureCode)` to the status schema in the same commit.
- `launch_not_started` gains `failure_code`. A spawn failure also carries `os_error` (the raw OS error, an integer) **only if** the brief says D-PD5 = yes; otherwise omit the field.

### C-PD4: preflight (PD-07 to PD-12)

`E/preflight/mod.rs` (new, cross-platform), `pub mod preflight;` in `E/lib.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum PreflightOperation { Play, Install, Repair }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum CheckId { Rosetta, WineRuntime, CompatibilityHelper, DiskSpace, GameFolder, ReleaseManifest, AppLocation, Physx }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum CheckStatus { Ok, Missing, Warning, Unknown }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum CheckAction { None, AutoInstall, Download, AskUser, Manual }
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Check {
    pub id: CheckId, pub status: CheckStatus, pub action: CheckAction,
    pub failure: Option<FailureCode>, pub blocking: bool,
    pub bytes_needed: Option<u64>, pub bytes_free: Option<u64>, pub download_bytes: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report { pub schema_version: u32 /* 1 */, pub operation: PreflightOperation, pub checks: Vec<Check>, pub ready: bool }

#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum RosettaProbe { Present, Missing, TimedOut, Error }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum ManifestProbe { Ok, Unreachable, Invalid, SigningKeyUnavailable }
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub struct FolderFacts { pub chosen: bool, pub exists: bool, pub writable: bool }
#[derive(Debug, Clone, Default)]
pub struct Facts {               // None = the check does not apply and is left out of the report
    pub rosetta: Option<RosettaProbe>,      // macOS + Wine backend only
    pub wine_runtime_cached: Option<bool>,  // macOS + Wine backend only
    pub helper_available: Option<bool>,
    pub free_bytes: Option<u64>,            // the game folder, or its nearest existing ancestor
    pub folder: Option<FolderFacts>,
    pub manifest: Option<ManifestProbe>,    // Install and Repair only
    pub translocated: Option<bool>,         // macOS only
    pub physx_installed: Option<bool>,      // Windows only
}
pub const INSTALL_BYTES: u64 = 14_000_000_000;   // launcher-guide.md:134, macos.md:44
pub const PLAY_WARN_BYTES: u64 = 1_000_000_000;
pub const WINE_RUNTIME_BYTES: u64 = 63_744_856;  // E/mac_runtime/mod.rs SIZE
pub fn evaluate(operation: PreflightOperation, facts: &Facts) -> Report;  // pure
```

`evaluate` rules (D-PD7, D-PD8). Checks appear in `CheckId` order; `ready` is true when no check has `blocking`.

| Check | Fact | Result |
|---|---|---|
| Rosetta | `Present` | Ok, None |
| | `Missing` | Missing, AutoInstall, `rosetta_missing`, blocking |
| | `TimedOut` or `Error` | Unknown, AskUser, `rosetta_check_timed_out`, not blocking |
| WineRuntime | `true` | Ok |
| | `false` | Missing, Download, `wine_runtime_missing`, `download_bytes = WINE_RUNTIME_BYTES`; blocking for Play and Repair, not blocking for Install (the install downloads it) |
| CompatibilityHelper | `false` | Missing, Manual, `helper_unavailable`, blocking |
| DiskSpace | Install, Repair: `free < INSTALL_BYTES` | Missing, Manual, `disk_space_low`, blocking, `bytes_needed`, `bytes_free` |
| | Play: `free < PLAY_WARN_BYTES` | Warning, Manual, `disk_space_low`, not blocking |
| GameFolder | `!chosen` | Missing, AskUser, `game_folder_not_chosen`, blocking |
| | Play or Repair: `!exists` | Missing, Manual, `game_folder_missing`, blocking |
| | `!writable` | Missing, Manual, `game_folder_not_writable`, blocking |
| ReleaseManifest | `Unreachable` / `Invalid` / `SigningKeyUnavailable` | Missing, Manual, `manifest_unreachable` / `manifest_invalid` / `signing_key_unavailable`, blocking |
| AppLocation | `true` | Warning, Manual, `app_translocated`, not blocking |
| Physx | `false` | Warning, AutoInstall, `physx_missing`, not blocking (the existing PhysX panel owns the install) |

Any fact that is fine gives `Ok`, `None`, no failure, not blocking.

**Dependency handling per check (owner D1).** "Auto" means the launcher installs it after one confirmation; "Download" means it fetches it with progress and no prompt beyond the Play press.

| Check | Platform | Action | What the player sees | Manual fallback (shown with the fix) |
|---|---|---|---|---|
| Rosetta | macOS | AutoInstall (PD-09: `softwareupdate` through an admin prompt) | `Install Rosetta` button, then progress | Terminal: `softwareupdate --install-rosetta --agree-to-license` |
| WineRuntime | macOS | Download (64 MB, automatic) | progress in the Play line | Retry; `Send logs` if it keeps failing (no manual install exists) |
| CompatibilityHelper | both | Manual | `Re-download the launcher` | the download page |
| DiskSpace | both | Manual | the bytes needed and free | free space, or choose another drive in Settings |
| GameFolder | both | AskUser | `Choose game folder` opens Settings | none needed |
| ReleaseManifest | both | Manual | `Try again`; Play still works offline | retry later |
| AppLocation | macOS | Manual (warning) | `Move the launcher to Applications` | drag it there, reopen |
| Physx | Windows | AutoInstall (the existing PhysX flow) | the existing PhysX panel | NVIDIA's legacy PhysX installer (the URL the PhysX panel already uses) |
| WebView2 | Windows, at startup | AutoInstall (PD-41) | a native prompt before the window opens | Microsoft's WebView2 download page |
| VC++ and DirectX runtimes | Windows | not checked (D-PD6, deferred) | none | the game's own installer |

IPC (PD-10), `S/host/preflight.rs` (new); one `generate_handler!` line in `S/main.rs`:

```rust
#[derive(Debug, Deserialize)] #[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreflightRequest {
    Inspect { schema_version: u32, operation: PreflightOperation },
    Install { schema_version: u32, operation: PreflightOperation, check: CheckId },
}
#[tauri::command]
pub async fn preflight_command(/* the same State parameters host::telemetry_status uses */ request: PreflightRequest)
    -> Result<Report, JobError>;   // Install re-runs Inspect after installing and returns the new Report
```

TS (PD-11) `FE/src/preflight-workflow.ts`: `Report` schema mirroring the Rust types (`failure: Schema.NullOr(FailureCode)`, the three byte fields `Schema.NullOr(Schema.Number)`), decoded with `onExcessProperty:'error'`.

### C-PD5: preflight and dependency events (PD-09, PD-10)

No new target (D-PD12). Preflight events go on the target of the operation they guard: Play → `launcher.launch`, Install → `launcher.install`, Repair → `launcher.repair`. Dependency installs go on `launcher.install`.

| Event | Level | Fields |
|---|---|---|
| `preflight_checked` | INFO when `ready`, WARN otherwise | `operation` (`play`/`install`/`repair`), `ready`, `blocking_checks`, `warning_checks` (u32), `missing` (comma-joined `CheckId` codes that are not Ok, at most 8, omitted when empty) |
| `dependency_install_started` | INFO | `dependency` (`rosetta` / `wine_runtime` / `webview2`), `source` (`apple_software_update` / `wowsilicon_release` / `microsoft_evergreen_bootstrapper`), `download_bytes` (when known). `webview2` goes on `launcher.startup`, and only when tracing is up (PD-41) |
| `dependency_install_finished` | INFO when `installed`, WARN otherwise | `dependency`, `outcome` (`installed` / `failed` / `declined` / `timed_out`), `failure_code` (omitted when installed), `elapsed_ms`, `exit_code` (the installer's integer exit status, when there is one) |

## Wave 1: incident fixes and the taxonomy (one PR per packet)

### PD-01: repair pre-start failure goes to Failed with a reason (F1, F2, D-PD2)

- **Host:** mac (hands-on). **Mac gate:** yes (the flipped mac test). **Agent:** packet-coder role. **Worktree:** `pd-01`, branch `launcher-preflight-diagnostics/pd-01-repair-not-started`, from `origin/main`.
- **Files:** `E/storage/repair/preparation/not_started.rs` (new), `E/storage/repair/preparation/not_started_tests.rs` (new), `E/storage/repair/preparation/mod.rs` (`mod` line and re-export only), `E/storage/repair/events.rs`, `S/host/repair/mod.rs`, `S/host/install/contract.rs`, `S/host/repair/tests.rs`, `LD/repair.md`.
- **Do:**
  1. `S/host/install/contract.rs`: add `impl JobError { pub fn code(self) -> &'static str }` returning each variant's serde name (`:106-122`), with a test that `serde_json::to_string(&v)` equals `"\"{code}\""` for every variant.
  2. `events.rs`: add `pub(crate) fn repair_not_started(id: Uuid, reason: &'static str)` per C-PD2 (WARN, `failure_code = "repair_not_started"` for now). Message: `"launcher repair: run stopped before it started; expected the installed copy and plan to verify; nothing on disk changed and the operation is failed"`.
  3. `not_started.rs`: `pub fn not_started(state: &std::sync::Mutex<DesktopState>, id: Uuid, reason: &'static str) -> bool`. Under the lock: when the current operation has this `id`, kind `Repair` and state `Starting`, call `operations_mut()?.observe(id, OperationState::Failed)`, then `events::repair_not_started(id, reason)`, then `queue_install_result(id, Run::Repair, Ending::Failed(reason))`, and return `true`. Any other case, or any error on the way, returns `false` without changing state. Expose it to the shell the same way `start_with_backend` is exposed.
  4. `S/host/repair/mod.rs:185-187`: replace the unconditional `mark_uncertain(&state, id)` with: when `error != JobError::PersistenceUncertain` and `not_started(&state, id, error.code())` returns `true`, don't mark; otherwise `mark_uncertain(&state, id)` as today. Still `return Err(error)`.
  5. `LD/repair.md`: in the state description, a failure before the repair starts now ends `failed` with a logged reason; only an unconfirmed save leaves `reconciliation_required`.
- **Tests:**
  - Engine unit, `not_started_tests.rs` (cross-platform; reuse the fixture the existing repair preparation tests use to put a Repair operation in `Starting`): `not_started` returns `true`, the operation is `Failed`, and the capture has WARN `launcher.repair` `event=repair_not_started reason=corrupt_state`. A Repair operation already `Running` → `false`, state unchanged, no event. Revert proof: with `observe` removed, the state assertion fails.
  - Host, `S/host/repair/tests.rs:250-291` (mac test `admitted_dispatch_failure_is_gated_and_duplicate_does_not_restart_work`): now expects `Failed` and `!after.repair.recovery`; the duplicate-request half stays. Revert proof: restoring the unconditional `mark_uncertain` gives `ReconciliationRequired`, so the state assertion fails. **This test runs only on macOS: MG.**
  - If a non-mac host repair test can reach a pre-start error (installed content removed before dispatch, so `start_with_backend` returns `Corrupt`), add the same assertion there. If none can, say so in the report; the engine test is then the Windows guard.
- **Checks:** D1, D2e, D2s, D3e `repair`, D3s `repair`, D3s `contract`, MG.
- **Docs rows:** 57 (`LD/repair.md`), 34 (the new event is on an existing forwarded target; list it in `LD/launcher-telemetry.md` if that file has an event table outside a generated block).
- **Commit:** `fix(launcher): a repair that fails before it starts ends failed with a reason (PD-01)`

### PD-02: Play dispatch failure goes to Failed; shell refusals log (F3, F13, D-PD2)

- **Host:** mac (hands-on). **Agent:** packet-coder role. **Worktree:** `pd-02`, branch `launcher-preflight-diagnostics/pd-02-launch-not-started`, from `origin/main`.
- **Files:** `E/storage/launch/not_started.rs` (new), `E/storage/launch/not_started_tests.rs` (new), `E/storage/launch/mod.rs` (`mod` line), `E/storage/launch/events.rs` (one fn), `S/host/launch/mod.rs`, the shell launch test file next to it, `LD/launch.md`.
- **Do:**
  1. `not_started.rs`: `pub fn dispatch_failed(state: &std::sync::Mutex<DesktopState>, id: Uuid, error: &IntentError) -> bool`. Under the lock: when `error.code() != "persistence_uncertain"`, the current operation has this `id`, kind `Launch`, state `Starting`, and `launch_observation()` is `Ok(None)`: `observe(id, Failed)`, then `events::launch_not_started(id, error.code())`, return `true`. Otherwise `false`, state unchanged.
  2. `events.rs`: add `pub fn launch_refused(reason: &'static str)` (C-PD2). Message: `"launcher launch: Play refused before an operation started; expected the bundled Play resources; nothing was started"`.
  3. `S/host/launch/mod.rs`: at `:100` call `launch_refused("resources_unavailable")` and at `:107` `launch_refused("resources_unresolved")` before returning `PlatformUnavailable`. At `:116-130`, when `launch::dispatch` errors, call `dispatch_failed(&store, operation_id, &error)` first; only when it returns `false` keep today's `mark_uncertain` path (and its `mark_uncertain_failed` log). The returned error is unchanged.
  4. `LD/launch.md`: a Play that fails before the game starts ends `failed` with a logged reason.
- **Tests** (corrected after the PD-02 reviews to match what is testable and what shipped at `a4d0a0cf5`):
  - Engine unit (`not_started_tests.rs`, cross-platform; calls `dispatch_failed` directly on the launch tests' `fixture()`): `a_starting_launch_with_no_record_fails_and_logs_the_error_code` (an `UnknownOperation` error used only as input: `true`, `Failed`, WARN `launcher.launch` `event=launch_not_started reason=unknown_operation`); `a_launch_that_already_recorded_preparing_is_left_alone` (`false`, still `Starting`, no event); `an_unconfirmed_save_or_another_operation_is_left_alone` (a `PersistenceUncertain` error and an unknown id both → `false`). Revert proofs: removing the observation check makes the second test return `true`; removing the `persistence_uncertain` check fails the third. The unknown-id half is not a revert proof for the id check (`observe` rejects an unknown id anyway); none is required.
  - Host, `S/host/launch/tests.rs` mac test `play_releases_the_store_before_dispatch_and_its_failure_path`: Play runs on a thread with no async runtime, so `dispatch` fails with `io` while the operation is `Starting` with no launch record; it now expects `Failed`, not `ReconciliationRequired`. Revert proof: restoring the unconditional `mark_uncertain` fails the state assertion. **This test runs only on macOS: MG.** No Windows test reaches the shell arm; the engine tests guard the decision on every platform. Deleting the launch plan record is not a usable vehicle: a missing plan makes `launch_plan()` return `Err(Corrupt)`, so `dispatch` fails with `corrupt_state`, `launch_observation()` is `Err`, and `dispatch_failed` correctly leaves the operation for reconciliation. `unknown_operation` cannot reach `Failed` in production (`dispatch` returns it only when the current operation is not a Launch).
  - `launch_refused` (shipped as `launch_refused_before_operation`, because `events.rs` already had `launch_refused(id, reason)`; the event name, target, level and fields are C-PD2's): `missing_artifacts_never_admit_or_change_consent` asserts one WARN `launch_refused` with `reason=resources_unavailable` and no `operation` field. Revert proof: removing the call fails `refused.len() == 1`. `resources_unresolved` has no test (not required).
- **Checks:** D1, D2e, D2s, D3e `launch`, D3s `launch`.
- **Docs rows:** 57 (`LD/launch.md`), 34.
- **Commit:** `fix(launcher): a Play that fails before the game starts ends failed and says why (PD-02)`

### PD-03: auto-reconcile an unstarted stuck repair on open (F4, D-PD3)

- **Host:** mac (hands-on). **Mac gate:** yes (the abandon module has macOS branches). **Agent:** packet-coder role. **Worktree:** `pd-03`, branch `launcher-preflight-diagnostics/pd-03-reconcile-unstarted`, from the PD-01 branch tip.
- **Files:** `E/storage/repair/unstarted.rs` (new), `E/storage/repair/unstarted_tests.rs` (new), `E/storage/repair/mod.rs` (`mod` line), `E/storage/repair/abandon/mod.rs` (visibility of the helpers `absent`, `directory_or_absent`, `read` to `pub(super)` only), `E/storage/repair/events.rs` (one fn), `S/host.rs` (one call next to `reconcile_launcher_update`, `:195-204`), `LD/repair.md`.
- **Do:**
  1. First verify, in `preparation/mod.rs` and the Wine and native preparation it calls, that `claim()` (which creates the work directory) runs before any process is spawned. If any spawn can happen before `claim()`, stop and return `blocked`.
  2. `unstarted.rs`: `impl DesktopState { pub fn reconcile_unstarted_repair(&mut self) -> Result<Option<Uuid>, IntentError> }`. Copy the precondition order of `abandon()` (`abandon/mod.rs:46-134`) without the confirmation and without the macOS `repair_recovery::stop` call: not `requires_reopen()`; current operation is `Repair` in `ReconciliationRequired`; `repair_plan()` is some; `lock_owner(&plan.installation)` (an `InUse` error returns `Ok(None)`); `repair-commit-{id}.json` absent; `plan.backup()` absent; `root.canonicalize() == root`; content-root presence equals `plan.original_present`; and, in addition, `repair-prepared-{id}.json` absent **and** `plan.work_directory()` absent. Any precondition that fails returns `Ok(None)` and changes nothing. When all hold: `reconcile(id, OperationState::Failed)`, emit `repair_reconciled_unstarted` (INFO, C-PD2; message `"launcher repair: an interrupted repair had not started; expected no repair files on disk and found none; marked failed"`), `queue_install_result(id, Run::Repair, Ending::Failed("repair_not_started"))`, return `Ok(Some(id))`.
  3. `S/host.rs` `store()`: after the state opens, call it; on `Err` log through the same `startup_event` pattern as `update_reconcile_failed` (add a sibling `repair_reconcile_failed` only if that module has a fn per case; otherwise reuse its generic helper).
  4. `LD/repair.md`: on start-up, an interrupted repair that never started is marked failed automatically; one with files on disk still needs Recover or Abandon.
- **Tests:** engine unit (`unstarted_tests.rs`; build the stuck state with the PD-01 fixture plus `mark_uncertain`): all absent → `Ok(Some(id))`, state `Failed`, INFO event pinned, one queued install result. The work directory present → `Ok(None)`, state still `ReconciliationRequired`. The commit record present → `Ok(None)`. An Install operation in `ReconciliationRequired` → `Ok(None)`. Revert proof: dropping the work-directory check makes the second case reconcile. Host: if `S/host.rs` has a `store()` test fixture, open a store holding the stuck state and assert the operation is `Failed`; otherwise say so in the report.
- **Checks:** D1, D2e, D2s, D3e `repair`, D3s `host`, MG.
- **Docs rows:** 57 (`LD/repair.md`), 34.
- **Commit:** `fix(launcher): reconcile an interrupted repair that never started on open (PD-03)`

### PD-04: the failure taxonomy, fixture and TS code list (F5, D-PD9)

- **Host:** mac (hands-on; Rust and frontend). **Agent:** packet-coder role. **Worktree:** `pd-04`, branch `launcher-preflight-diagnostics/pd-04-failure-taxonomy`, from the PD-01 branch tip (it needs `JobError::code()`).
- **Files:** `E/failure.rs` (new), `E/failure_tests.rs` (new), `E/lib.rs` (`mod` line), `S/host/install/contract.rs` (one fn), the contract test file it already uses, `E/storage/repair/events.rs` (`failure_code` from the code, see 4), `FE/src/failure-codes.json` (new), `FE/src/failure-codes.ts` (new), `FE/src/failure-copy.ts` (new), `FE/src/failure-copy.test.ts` (new), `FE/package.json`.
- **Do:**
  1. Build C-PD1 exactly: the enum, `ALL`, `code()`, `log_message()`, the fixture, the TS literal list and the copy table, word for word.
  2. `contract.rs`: `impl JobError { pub fn failure(self) -> cimmeria_launcher_engine::failure::FailureCode }` per the `job_errors` map.
  3. Nothing renders the copy yet; PD-06 and PD-16 do.
  4. `repair_not_started` (PD-01) keeps `failure_code = "repair_not_started"`; no change unless PD-01 left a TODO there.
- **Tests:**
  - Rust (`failure_tests.rs`): `ALL` serialized in order equals the fixture `codes`; every `log_message()` is non-empty, at most 80 bytes, and contains no `/`, `:` or `{`. Shell (contract tests): for every `JobError` variant, `failure().code()` equals `job_errors[code()]`, and the fixture has no extra keys. Revert proof: removing one enum variant or changing one map arm fails the comparison.
  - TS (`failure-copy.test.ts`, `node:test`): the `FailureCode` literals equal the fixture `codes`; every code has a non-empty `message` (at most 160 characters) and `fix` (at most 200); only `disk_space_low` contains placeholders, and `failureText('disk_space_low', {needed: 14e9, free: 2.5e9})` reads `about 14.0 GB is needed and 2.5 GB is free`; every `job_errors` and `install_outcomes` value is a code. Revert proof: deleting a copy entry fails the type check (`npm run check`) and the coverage test.
- **Checks:** D1, D2e, D2s, D3e `failure`, D3s `contract`, F1.
- **Docs rows:** 57 (none of `LD/` yet; PD-18 documents the taxonomy).
- **Commit:** `feat(launcher): one failure taxonomy with user copy and a shared fixture (PD-04)`

### PD-05: NotStarted carries a reason; spawn errors classified (F6, F7, C-PD3)

- **Host:** mac (hands-on). **Mac gate:** yes. **Agent:** packet-coder role. **Worktree:** `pd-05`, branch `launcher-preflight-diagnostics/pd-05-launch-failure`, from the PD-04 branch tip, which must contain PD-02. First check that `E/storage/launch/not_started.rs` exists and defines `pub fn dispatch_failed`; if not, return `blocked` (PD-02 is not in the base).
- **Brief input:** the orchestrator states D-PD5 (`os_error` yes or no).
- **Files:** `E/storage/launch/failure.rs` (new), `E/storage/launch/failure_tests.rs` (new), `E/storage/launch/mod.rs` (`Record` field, `mod` line, `record_launch` keeps `failure`), `E/storage/launch/supervisor.rs`, `E/storage/launch/supervisor_tests.rs` (call sites only), `E/storage/launch/worker.rs`, `E/storage/launch/events.rs`, `E/storage/launch/events_tests.rs`, `E/storage/launch/not_started.rs`, `E/storage/launch/not_started_tests.rs`, `E/storage/launcher_summary/tests/launch_rows.rs`, `S/host/launch/mod.rs`, `S/host/launch/tests.rs`, `FE/src/launch-workflow.ts`, `FE/src/launch-workflow.test.ts`, `LD/launch.md`, `LD/launcher-summaries.md`. Both `LD/` files are LF on `main`; keep LF (R4's CRLF rule does not hold for them).
- **Do:**
  1. `failure.rs`: `classify_spawn`, `note_launch_failure` and `launch_failure` per C-PD3.
  2. `events.rs`: `launch_not_started(id, reason, failure: FailureCode)` adds `failure_code`. Add `launch_spawn_failed(id, failure: FailureCode, os_error: Option<i32>)` (WARN, `event=launch_not_started`, `reason="spawn_failed"`, `failure_code`, plus `os_error` only when D-PD5 = yes). Update the other callers and `launch_not_started_for_test`.
  3. `supervisor.rs:63-69`: bind `Err(error)`, compute `classify_spawn(error.kind(), error.raw_os_error(), cfg!(target_os = "macos"))`, call `launch_spawn_failed`, hand the code to the new last parameter of `supervisor::run`, `on_failure: impl FnOnce(FailureCode)`, and return `Observation::NotStarted`. Every existing `supervisor::run(` call in `supervisor_tests.rs` passes `|_| {}`.
  4. `worker.rs`: at `:165` (`verify_failed` → `game_files_invalid`), `:194` (`prepare_failed` → `launch_prepare_failed`) and `:304` (`login_servers_refused` → `login_servers_invalid`) pass the code to `launch_not_started` and call `note_launch_failure` under the state lock. `:178` already returns after `:304` logged; leave it. The `supervisor::run` call (`:224`) passes an `on_failure` that calls `note_launch_failure` under the lock and ignores a lock error. Keep `worker.rs` under 450 lines; if it would pass, move the three not-started blocks into a helper in `failure.rs`.
  5. `S/host/launch/mod.rs`: `LaunchStatus.failure` from `launch_failure()`; `FE/src/launch-workflow.ts`: the schema field (same commit, R8).
  6. The PD-02 path (C-PD3; it writes no launch record today). `failure.rs`: add `record_not_started` per C-PD3. `not_started.rs`: `dispatch_failed` gains a last parameter `failure: FailureCode`. After its existing checks pass (`persistence_uncertain`, id, kind, `Starting`, `launch_observation()` is `Ok(None)`), and still under the same lock: `let Ok(Some(plan)) = owner.launch_plan() else { return false };`, then `owner.record_not_started(&plan, failure)` (any `Err` → return `false`), then `observe(id, OperationState::Failed)` (an `Err` → return `false`), then `events::launch_not_started(id, reason, failure)`, then `true`. Update the module and fn doc comments to say the record is written. `S/host/launch/mod.rs`: the match guard becomes `launch::dispatch_failed(&store, operation_id, &error, JobError::from(error).failure())` (`IntentError` is `Copy`); the returned error and the `mark_uncertain` arm are unchanged. Result: this path reads back as `Observation::NotStarted` with a `failure`, so `LaunchStatus.failure` is set and `launcher_summary` reports `launch_not_started`. Don't change `summary_error_code`.
  7. `LD/launch.md`: in the paragraph PD-02 added ("A Play that fails before the game starts ends `failed` ..."), say the operation first records a not-started launch result carrying `failure` (the failure code, e.g. `local_io`), and that `launch_not_started` carries `failure_code`. `LD/launcher-summaries.md`: one sentence after the error-code table: a Play whose dispatch failed before the game started records `NotStarted` too, so its row is `launch_not_started`. Don't change the table.
- **Tests:**
  - `not_started_tests.rs` (cross-platform): every existing `dispatch_failed` call passes a code. `a_starting_launch_with_no_record_fails_and_logs_the_error_code` passes `FailureCode::StatusStale` and also asserts `launch_observation() == Ok(Some(Observation::NotStarted))`, `launch_failure() == Ok(Some(FailureCode::StatusStale))` and `failure_code=status_stale` on the event. `a_launch_that_already_recorded_preparing_is_left_alone` also asserts the observation is still `Preparing` and `launch_failure()` is `Ok(None)`. `an_unconfirmed_save_or_another_operation_is_left_alone` also asserts `launch_observation() == Ok(None)` (no record written). Revert proof: removing the `record_not_started` call fails the observation assertion in the first test.
  - `launch_rows.rs` (cross-platform, plain `#[test]`, the existing `admitted()` fixture): new `a_dispatch_that_fails_before_it_starts_is_a_launch_not_started_row`: wrap the state in `std::sync::Mutex`, `assert!(launch::dispatch_failed(&state, plan.id, &StorageError::Io.into(), FailureCode::LocalIo))`, then `finalize_summaries()`; one queued row, `SummaryOperation::Launch`, `SummaryOutcome::Failed`, `error_code == Some(SummaryErrorCode::LaunchNotStarted)`. Revert proof: without the record the row's code is `Unspecified`.
  - `S/host/launch/tests.rs`, mac test `play_releases_the_store_before_dispatch_and_its_failure_path`: also send `after.observation` and `after.failure` from the worker thread and assert `Some(launch::Observation::NotStarted)` and `Some(FailureCode::LocalIo)` (dispatch fails with `io` there; the state assertion stays `Failed`). Revert proof: removing the `record_not_started` call makes both `None`. **macOS only: MG.**
  - `failure_tests.rs` (cross-platform): `classify_spawn` table: (`Other`, `Some(86)`, `true`) → `rosetta_missing`; (`Other`, `Some(86)`, `false`) → `spawn_failed`; `NotFound` → `spawn_not_found`; `PermissionDenied` → `spawn_permission_denied`. `note_launch_failure` then `record_launch(NotStarted)` → `launch_failure()` is the code; an old record JSON without `failure` reads as `None`. Revert proof: dropping the `macos` condition fails row 2; not preserving the field in `record_launch` fails the second test.
  - `supervisor_tests.rs`: a spawn of a missing executable reaches `on_failure(spawn_not_found)` and logs `failure_code=spawn_not_found`. Revert proof: restore `let Ok(..) else`, and no code arrives.
  - `launch-workflow.test.ts`: a status with `failure: "rosetta_missing"` decodes; one without the key is rejected.
  - On macOS, the existing mac launch tests must still pass (MG).
- **Checks:** D1, D2e, D2s, D3e `launch`, D3e `launcher_summary`, D3s `launch`, F1, MG.
- **Docs rows:** 57 (`LD/launch.md`: the `failure` field, the not-started codes and the PD-02 path's record; `LD/launcher-summaries.md`: the PD-02 path's row code), 34 (`failure_code`, `os_error`).
- **Commit:** `feat(launcher): Play says why the game did not start, including a missing Rosetta (PD-05)`

### PD-06: Play names its blocker, with an inline Abandon (F5, trigger incident step 3)

- **Host:** mac (hands-on; frontend only). **Agent:** packet-coder role. **Worktree:** `pd-06`, branch `launcher-preflight-diagnostics/pd-06-play-blocker`, from the PD-05 branch tip.
- **Files:** `FE/src/launch-view.ts`, `FE/src/launch-view.test.ts`, `FE/src/launch-workflow.ts`, `FE/src/repair-view.ts`, `FE/src/composition.ts`, `FE/ui/index.html` (one button in the footer at `:45`).
- **Do:**
  1. `launch-view.ts` `launchText` (`:12-28`): replace "Another operation owns the game." (`:24`) with a line per operation kind and state, keeping `adoptLine` (`:7-11`) for `adopt`:
     - `repair` in `reconciliation_required`: `An earlier repair was interrupted. Abandon it to play, or recover it in Settings.`
     - `repair` running: `A repair is running. Play is available when it finishes.`
     - `install` in `reconciliation_required`: `An earlier install was interrupted. Resume or abandon it in the install panel.`
     - `install`, `update`, `uninstall`, `prepare_runtime` running: `The launcher is busy (<install|game update|uninstall|compatibility setup>). Play is available when it finishes.`
     - `update` in `reconciliation_required`: `An earlier game update was interrupted. Recover or abandon it in Settings.`
     - snapshot `requires_reopen`: `The launcher needs to restart to check its saved state. Close and reopen it.`
     - any other kind: today's line, unchanged.
  2. Not-started and failed Play: when `status.failure` is set, the status line is `failureText(code).message` and a second line `failureText(code).fix`. A `ProcessExited` with a non-zero code uses `game_exited_nonzero`. The generic line (`:15`, `:22`) stays only when there is no code.
  3. `launch-workflow.ts:21`: keep every `JobError` code the C-PD1 `job_errors` map knows (map it to its `FailureCode`), not only `launcher_too_old`; anything else stays `transport`.
  4. Inline Abandon: `index.html` gets `<button id="launch-abandon-repair" hidden>Abandon interrupted repair…</button>` after `#launch-status`. Shown only for `repair` in `reconciliation_required`. `repair-view.ts` `repairControls` gains a returned `request(action: 'abandon')` that opens the gear (`#gear`, as `#settings` does) and shows the existing `#repair-confirmation` for abandon, exactly as `#abandon-repair` does. `composition.ts` wires the new button to it. The first press shows the confirmation (R8).
- **Tests (`launch-view.test.ts`, linkedom):** each kind and state line above; `#launch-abandon-repair` visible only for repair + `reconciliation_required`, and a click unhides `#gear` and `#repair-confirmation`; a status with `failure: "rosetta_missing"` shows the Rosetta message and fix; an `adopt` operation keeps `adoptLine`; an unknown kind keeps "Another operation owns the game." Revert proof: with the old `launchText`, the repair line test fails.
- **Checks:** F1.
- **Docs rows:** 57 (`docs/client/launcher-guide.md` troubleshooting is PD-18's; none here).
- **Commit:** `fix(launcher): Play names what blocks it and offers Abandon inline (PD-06)`

PD-22 (the macOS display-mode hotfix) ships in wave 1 too; its section is in [work-packets-display.md](work-packets-display.md#pd-22-macos-never-changes-the-display-mode).

## Wave 2: preflight (integration branch `launcher-preflight-diagnostics/preflight`, PR P2)

Each packet branches from the `pd-w2` tip after the packets before it merged in.

### PD-07: preflight model, pure evaluation and disk probe (F10, C-PD4)

- **Host:** mac (hands-on). **Mac gate:** yes (the unix disk branch). **Agent:** packet-coder role. **Worktree:** `pd-07`, branch `launcher-preflight-diagnostics/pd-07-preflight-model`.
- **Files:** `E/preflight/mod.rs` (new), `E/preflight/disk.rs` (new), `E/preflight/tests.rs` (new), `E/lib.rs` (`mod` line).
- **Do:**
  1. `mod.rs`: every type, constant and `evaluate` in C-PD4, exactly as written.
  2. `disk.rs`: `pub fn free_bytes(path: &Path) -> Option<u64>`. Walk up to the nearest existing ancestor. Unix: `libc::statvfs`, `f_bavail * f_frsize`. Windows: `windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW` (free bytes available to the caller). Any error → `None`. No new dependency (both are in `engine/Cargo.toml:41-45`).
  3. `pub fn folder_facts(path: Option<&Path>, must_exist: bool) -> FolderFacts`: `chosen` = path is some; `exists` = it is a directory; `writable` = creating and removing `.cimmeria-write-probe-<uuid>` in it (or, when `!must_exist` and it is absent, in its nearest existing ancestor) succeeds.
- **Tests (`tests.rs`):** one test per row of the `evaluate` table; `ready` false with one blocking check and true with only warnings; check order follows `CheckId`; `free_bytes` on a temp dir is `Some(n > 0)` and on a missing child of the temp dir equals the temp dir's value; `folder_facts` on a temp dir is writable, on a missing path with `must_exist` is `exists = false`. Revert proof: making DiskSpace a warning for Install fails its row.
- **Checks:** D1, D2e, D3e `preflight`, MG, WG (the Windows disk probe).
- **Docs rows:** none (PD-18).
- **Commit:** `feat(launcher): a preflight model with disk and folder checks (PD-07)`

### PD-08: macOS probes: Rosetta, Wine runtime, translocation (F8, F11, F12, D-PD8)

- **Host:** mac (hands-on). **Mac gate:** yes. **Agent:** packet-coder role. **Worktree:** `pd-08`, branch `launcher-preflight-diagnostics/pd-08-mac-probes`.
- **Files:** `E/preflight/mac.rs` (new), `E/preflight/mac_tests.rs` (new), `E/preflight/mod.rs` (`mod` line).
- **Do:**
  1. `pub(crate) fn classify_rosetta(outcome: Option<std::io::Result<bool>>) -> RosettaProbe`: `None` (timed out) → `TimedOut`; `Some(Err(_))` (spawning `/usr/bin/arch` failed) → `Error`; `Some(Ok(true))` (exit 0) → `Present`; `Some(Ok(false))` → `Missing`. Pure, cross-platform.
  2. `pub async fn rosetta() -> RosettaProbe`: on macOS run `tokio::process::Command::new("/usr/bin/arch").args(["-x86_64", "/usr/bin/true"])` with `env_clear()`, null stdio, `kill_on_drop(true)`, under a 5-second `tokio::time::timeout`, then `classify_rosetta`. On other platforms return `Present` without spawning (the check is left out by the caller anyway).
  3. `pub fn translocated(exe: &Path) -> bool`: true when the path contains a component sequence `AppTranslocation`. Pure.
  4. `#[cfg(target_os = "macos")] pub fn wine_runtime_cached(state_root: &Path) -> bool`: `crate::mac_runtime::verified_cached(&state_root.join("runtimes")).is_ok()`; drop the returned lock at once. Note: this verifies the tree and can take a second; PD-10 calls it from `spawn_blocking`.
  5. Do not change `E/mac_wine/mod.rs:100-113` (R10); PD-14 aligns it.
- **Tests (`mac_tests.rs`):** the four `classify_rosetta` cases; `translocated` true for `/private/var/folders/x/y/T/AppTranslocation/ABC/d/Cimmeria Launcher.app/Contents/MacOS/x` and false for `/Applications/Cimmeria Launcher.app/Contents/MacOS/x`. A `#[cfg(target_os = "macos")]` test that `rosetta()` returns `Present` or `Missing` (never `Error`) on the CI Mac. Revert proof: mapping `TimedOut` to `Missing` fails case 1.
- **Checks:** D1, D2e, D3e `preflight`, MG.
- **Docs rows:** none (PD-18).
- **Commit:** `feat(launcher): macOS preflight probes for Rosetta, the Wine runtime and translocation (PD-08)`

### PD-09: Rosetta auto-install with a re-probe (F9, D-PD1, D-PD10)

- **Host:** mac (hands-on). **Mac gate:** yes. **Agent:** packet-coder role. **Extra pass:** `network-security-auth` (privileged command). **Worktree:** `pd-09`, branch `launcher-preflight-diagnostics/pd-09-rosetta-install`.
- **Files:** `E/preflight/rosetta_install.rs` (new), `E/preflight/rosetta_install_tests.rs` (new), `E/preflight/mod.rs` (`mod` line), `E/preflight/events.rs` (new; the C-PD5 dependency events).
- **Do:**
  1. `pub enum InstallOutcome { Installed, Failed { exit_code: Option<i32> }, Declined, TimedOut }`.
  2. `pub(crate) fn classify_install(exit_code: Option<i32>, stderr_tail: &str, probe_after: RosettaProbe) -> InstallOutcome`: `probe_after == Present` → `Installed` whatever the exit code (exit codes are unreliable, F9); otherwise a `stderr_tail` containing `(-128)` (AppleScript "User canceled") → `Declined`; otherwise `Failed { exit_code }`. Pure.
  3. `pub async fn install_rosetta() -> InstallOutcome` (macOS; elsewhere `Failed { exit_code: None }`): emit `dependency_install_started` (`rosetta`, `apple_software_update`); run `/usr/bin/osascript -e 'do shell script "/usr/sbin/softwareupdate --install-rosetta --agree-to-license" with administrator privileges'` with `env_clear()`, null stdin, piped stderr (keep at most the last 512 bytes, never logged), `kill_on_drop(true)`, under a 15-minute timeout (timeout → `TimedOut`, and the child is killed); then `rosetta().await` (PD-08) and `classify_install`; emit `dependency_install_finished` with `outcome`, `failure_code` (`rosetta_install_failed`, `rosetta_install_declined`, or `rosetta_install_failed` for `TimedOut`), `elapsed_ms`, `exit_code`. The command string is a constant; nothing from the user, the state or the environment is interpolated into it.
- **Tests:** `classify_install` table (exit 0 + `Missing` → `Failed`; exit 1 + `(-128)` + `Missing` → `Declined`; exit 1 + `Present` → `Installed`); the command constant equals the string above (a guard against interpolation); the event helpers pinned with `test_capture` (fields only). Revert proof: deciding by exit code makes row 1 `Installed`.
- **Checks:** D1, D2e, D3e `preflight`, MG.
- **Docs rows:** 34 (the dependency events).
- **Commit:** `feat(launcher): install Rosetta 2 for the player and confirm it with a re-check (PD-09)`

### PD-10: `preflight_command` IPC and fact gathering (C-PD4, C-PD5, D-PD13)

- **Host:** mac (hands-on). **Mac gate:** yes (the macOS fact branches). **Agent:** packet-coder role. **Worktree:** `pd-10`, branch `launcher-preflight-diagnostics/pd-10-preflight-ipc`.
- **Files:** `S/host/preflight.rs` (new), `S/host/preflight_tests.rs` (new), `S/host.rs` (`mod` line), `S/main.rs` (one `generate_handler!` line, `:609-631`), `E/preflight/events.rs` (add `preflight_checked`).
- **Do:**
  1. Define `PreflightRequest` and `preflight_command` per C-PD4, with the same `State` parameters and registration style as `host::telemetry_status::telemetry_status`. `schema_version != 1` → `JobError::UnsupportedSchema`.
  2. Gather `Facts` (all blocking work in `spawn_blocking`):
     - `rosetta`, `wine_runtime_cached`: only on macOS when the Wine backend applies (the host's `self.helper` is some).
     - `helper_available`: macOS `self.helper.is_some()`; Windows: the bundled Play resources resolve (the check at `S/host/launch/mod.rs:100-107`).
     - `free_bytes`, `folder`: from the saved install directory (`preferences.install_directory`); `must_exist` for Play and Repair.
     - `manifest`: Install and Repair only, `cimmeria_launcher_engine::catalog::fetch_release(..).await` as `run_install_command` calls it; `CatalogError::Network` → `Unreachable`; `SigningKeyUnavailable`, `DevelopmentSigningKey` → `SigningKeyUnavailable`; anything else → `Invalid`.
     - `translocated`: macOS, `preflight::mac::translocated(&std::env::current_exe()?)`.
     - `physx_installed`: Windows, from the existing PhysX detection `physx_command` uses.
  3. `evaluate`, emit `preflight_checked` (C-PD5) on the operation's target, return the `Report`.
  4. `Install`: refuse with `JobError::Busy` while any operation is non-terminal (D-PD13). `Rosetta` → `install_rosetta().await`. `WineRuntime` (macOS) → `cimmeria_launcher_engine::mac_runtime::prepare(state_root.join("runtimes"), CancellationToken::new(), <a no-op ProgressSink>)` wrapped in `dependency_install_started`/`finished` (`wine_runtime`, `wowsilicon_release`, `download_bytes = WINE_RUNTIME_BYTES`; `RuntimeError::Network` → `wine_runtime_download_failed`, `Verification`/`Extraction` → `wine_runtime_invalid`, `Busy` → `wine_runtime_busy`, others → `wine_runtime_unavailable`). Any other check → `JobError::PlatformUnavailable`. Then run Inspect again and return its report.
- **Tests (`preflight_tests.rs`):** request decoding (unknown field and `schema_version: 2` rejected); a fixture host with no install directory and operation Install → the report has GameFolder `game_folder_not_chosen`, `ready = false`, and the WARN `preflight_checked` event on `launcher.install` with `missing` containing `game_folder`; `Install` while an operation is running → `Busy`. Manifest fetch is not exercised (no network in tests); keep the fact gatherer's catalog call behind a parameter so the test passes `None`. Revert proof: dropping the busy guard fails the last test.
- **Checks:** D1, D2e, D2s, D3s `preflight`, MG, WG (the Windows PhysX fact).
- **Docs rows:** 57 (IPC list in `LD/` if one exists; else PD-18), 34.
- **Commit:** `feat(launcher): a preflight command that checks and installs what an operation needs (PD-10)`

### PD-11: the preflight panel: what is missing, what will happen, the fix (requirement 2)

- **Host:** mac (hands-on; frontend). **Agent:** packet-coder role. **Worktree:** `pd-11`, branch `launcher-preflight-diagnostics/pd-11-preflight-panel`.
- **Files:** `FE/src/preflight-workflow.ts` (new), `FE/src/preflight-workflow.test.ts` (new), `FE/src/preflight-view.ts` (new), `FE/src/preflight-view.test.ts` (new), `FE/ui/index.html` (one `<section id="preflight-panel" hidden aria-labelledby="preflight-title">` above the footer), `FE/package.json`.
- **Do:**
  1. `preflight-workflow.ts`: the `Report` schema (C-PD4), `inspectPreflight(operation)` and `installDependency(operation, check)` calling `preflight_command`, the same Effect style as `launch-workflow.ts`.
  2. `preflight-view.ts`: `mountPreflight(document, workflow)` returns `show(report)`, `hide()`. For each check that is not Ok, one row: the `failureText(failure, {needed, free})` message, then what happens:
     - AutoInstall + Rosetta: `The launcher will install Rosetta 2 from Apple. macOS will ask for your administrator password.` and a button `#preflight-install-rosetta` (`Install Rosetta 2`).
     - Download + WineRuntime: `The launcher will download the compatibility runtime (64 MB) from the WoWSilicon release page.` and `#preflight-install-wine_runtime` (`Download now`) for Play and Repair; for Install the text says it downloads during the install, with no button.
     - AskUser + GameFolder: the fix text and a button `#preflight-choose-folder` that opens Settings (`#gear`).
     - AskUser + Rosetta (`Unknown`): the fix text and `#preflight-recheck`.
     - Manual: the fix text only.
     - Warning rows say `You can continue.`
  3. Buttons: `#preflight-recheck` (always), `#preflight-continue` (only when `ready` and there are warnings). An install button disables itself and shows `Installing…` on the first press, then renders the returned report; on a JobError it shows `failureText` of the mapped code.
- **Tests:** workflow: a report decodes; one with an extra key is rejected. View (linkedom): a report with Rosetta Missing shows the install line and button; pressing it calls `installDependency('play','rosetta')` once and shows `Installing…`; a ready report with a translocation warning shows `#preflight-continue`; a disk row shows both sizes. Revert proof: without the AutoInstall branch, the Rosetta button test fails.
- **Checks:** F1.
- **Docs rows:** none (PD-18).
- **Commit:** `feat(launcher): a preflight panel that says what is missing and what happens next (PD-11)`

### PD-12: Play, Install and Repair run preflight first (requirement 2)

- **Host:** mac (hands-on; frontend). **Agent:** packet-coder role. **Worktree:** `pd-12`, branch `launcher-preflight-diagnostics/pd-12-preflight-gate`.
- **Files:** `FE/src/launch-view.ts`, `FE/src/launch-view.test.ts`, `FE/src/install-view.ts` (call lines only), `FE/src/install-view.test.ts`, `FE/src/repair-view.ts`, `FE/src/composition.ts`.
- **Do:**
  1. `#launch` click: `inspectPreflight('play')` first. `ready` and no warnings → today's launch call. `ready` with warnings → show the panel; `#preflight-continue` launches. Not ready → show the panel and don't launch; the status line reads `Play needs attention: see below.`
  2. The install primary (Install) and the repair confirmation (`#confirm-repair` for a repair run) do the same with `install` and `repair`.
  3. After a successful dependency install, the panel shows the new report; when it is `ready`, the panel says `Ready. Press Play again.` (or Install/Repair). It doesn't start the operation by itself.
  4. A preflight command error (transport or JobError) never blocks the operation silently: show the mapped failure text and a `#preflight-continue` labelled `Continue anyway`.
- **Tests:** Play click with a blocking report sends no launch command and shows the panel; with a ready report it sends one launch command; a warning report waits for `#preflight-continue`; a preflight transport error shows `Continue anyway`; Install and Repair the same with one test each. Revert proof: removing the gate makes the first test send a launch command.
- **Checks:** F1.
- **Docs rows:** none (PD-18).
- **Commit:** `feat(launcher): check dependencies before Play, Install and Repair (PD-12)`
