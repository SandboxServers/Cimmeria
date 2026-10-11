# Launcher Consolidation Work Packets

> Type: how-to. Audience: the coordinator and packet workers.
> Updated: 2026-10-10. Companions: [ledger and decisions](README.md), [desktop launcher README](../../../crates/launcher/desktop/README.md), [testing playbook](../../../TESTING.md), [doc-update map](../../agents/doc-update-map.md).

## Dispatch rules

- One worktree per worker, made from the main checkout with `pwsh tools/build-lane/mk-worktree.ps1 launcher-consolidation/<packet>-<slug> lx-<packet>`. Packet ids are lower-case in names (`lx-03`).
- **PowerShell only.** No bash, no WSL, no direct `cargo`, no `git worktree prune`, no `git stash`. `build-desktop.sh` and other `.sh` files run in CI only.
- Every compiling cargo call goes through `pwsh tools/build-lane/lane.ps1`. `npm` calls do not compile Rust and run directly.
- The desktop launcher is its own workspace. Its checks always pass `--manifest-path crates/launcher/desktop/Cargo.toml --target-dir target/desktop --locked`.
- Workers commit locally and report once, with results. The coordinator runs a `packet-reviewer` on the diff, fixes what is real, then ships with `python tools/build-lane/ship.py pr -C <worktree>` and merges with `ship.py merge <PR> --retire <worktree>`.
- Frontend changes owe a REPL-style logic UAT ([AGENTS.md](../../../AGENTS.md)): an `npm run uat:<feature>` script beside the unit tests.
- Every button gets visible feedback on the first press: a busy state while native work runs, then a result line.
- Commit messages end with the attribution lines the coordinator puts in the brief.
- Initial state: documentation only, against `main` @ `2c1def5bc`.

### Standard checks

Each packet lists which of these it runs, in this order.

```powershell
# D1 format (no compile)
cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all -- --check
# D2 clippy, the crates the packet touched
pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --target-dir target/desktop --all-targets -- -D warnings
pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --target-dir target/desktop --all-targets -- -D warnings
# D3 tests, same crates
pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --target-dir target/desktop
pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --target-dir target/desktop
# F1 frontend
npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend
npm run check --prefix crates/launcher/desktop/frontend
npm test --prefix crates/launcher/desktop/frontend
# R1 root workspace, the crates the packet touched
cargo fmt --all -- --check
pwsh tools/build-lane/lane.ps1 cargo clippy -p <crate> --all-targets -- -D warnings
pwsh tools/build-lane/lane.ps1 cargo nextest run -p <crate>
```

## Contract fixed by this ledger

Wave 1 packets build against these names. LX-01 and LX-02 create them; nobody renames them locally. A worker who needs a change raises it with the coordinator.

### Shared crate (LX-01)

- New root-workspace crate `crates/launcher-core/`, package `cimmeria-launcher-core`, `publish = false`.
- Modules, moved verbatim with their `#[cfg(test)]` tests: `manifest`, `client_setup` (with `login_servers`, `aslr`, `stock_case`), `install`, `install_layout`, `install_report`, `patch_dest`, `state`, `unpack` (with `zip`, `rar`, `cab_set`, `fdi`, `dos_time`, `prerequisites`), `install_progress`, `telemetry_endpoint`, `telemetry_session`. LX-01 also moves `client_paths` (for LX-06), `client_changes` and `overlay_meta` (for LX-07), `logs` (for LX-05) and `telemetry/{auth,queue,runner,tail,process_watch,install_result,patch_log,patch_counts}` (for LX-08), so wave 1 ports from one place.
- `crates/launcher/src/lib.rs` (egui) replaces each moved module with `pub use cimmeria_launcher_core::<module>;` so the egui crate keeps building until LX-18.
- `crates/launcher/desktop/engine/src/lib.rs` replaces every `#[path = "../../../src/..."]` with `pub use cimmeria_launcher_core::<module>;` under the same public name, so `cimmeria_launcher_engine::manifest` and the rest keep resolving.

### New preferences (LX-02)

`Preferences` in `engine/src/storage/mod.rs` goes to `schema_version: 2`. A v1 file reads as v2 with these defaults, and is written back as v2 on the next save.

```rust
pub struct Preferences {
    pub schema_version: u32,          // 2
    pub revision: u64,
    pub install_directory: Option<PathBuf>,
    pub launcher_summary_consent: bool,
    /// Empty means `client_setup::login_servers::default_servers()`.
    pub login_servers: Vec<LoginServer>,
    pub client_patches_enabled: bool, // default true
}
```

`LoginServer` is the struct in `cimmeria_launcher_core::client_setup::login_servers` (LX-01 makes it `pub` and `Serialize + Deserialize` if it is not).

New `NativeCommand` variant; `SavePreferences` is unchanged:

```rust
SaveClientOptions {
    schema_version: u32,      // 1
    expected_revision: u64,
    login_servers: Vec<LoginServer>,
    client_patches_enabled: bool,
},
```

### New native commands (LX-02)

Each is a `#[tauri::command]` in `shell/src/main.rs`, registered in `generate_handler!`, with its request/status types in its own `shell/src/host/<name>.rs` and its engine half in `engine/src/storage/<name>/mod.rs`. LX-02 creates all of them with bodies that return `Err(<Error>::Unavailable)`. Each request enum is `#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]` and carries `schema_version: 1`.

| Tauri command | Host file | Engine module | Requests | Filled by |
|---|---|---|---|---|
| `log_upload_command` | `host/log_upload.rs` | `storage/log_upload/` | `Inspect`, `Upload` | LX-05 |
| `client_maintenance_command` | `host/client_maintenance.rs` | `storage/client_maintenance/` | `Inspect`, `ResetCache`, `ResetAllClientState { confirmed: bool }` | LX-06 |
| `client_changes_command` | `host/client_changes.rs` | `storage/client_changes/` | `Inspect` | LX-07 |

Every new operation that writes to disk calls `ensure_updater_idle()` before its first write and refuses while an install, Play, Repair or update is nonterminal (`docs/updater.md`, "New mutation paths must preserve this rule").

### New frontend files (LX-02)

Under `crates/launcher/desktop/frontend/src/`, each a stub that renders a heading and an "unavailable" line, mounted in Settings by `view.ts`, with an empty passing `*.test.ts` added to the `test` script in `package.json` and a `uat:<name>` script that runs `<name>-uat.mjs`:

| View | Test | UAT script | Filled by |
|---|---|---|---|
| `server-list-view.ts` | `server-list-view.test.ts` | `uat:server-list` | LX-03 |
| `client-patches-view.ts` | `client-patches-view.test.ts` | `uat:client-patches` | LX-04 |
| `log-upload-view.ts` | `log-upload-view.test.ts` | `uat:log-upload` | LX-05 |
| `client-maintenance-view.ts` | `client-maintenance-view.test.ts` | `uat:client-maintenance` | LX-06 |
| `client-changes-view.ts` | `client-changes-view.test.ts` | `uat:client-changes` | LX-07 |

`contract.ts` gains the TypeScript mirrors of every request and status type above.

### Release names (LX-15)

| Item | Name |
|---|---|
| Dated release tag | `launcher-YYYYMMDD-<sha7>` (unchanged) |
| Rolling release tag | `launcher-current` |
| Zip | `StargateWorlds-Launcher-windows-x64.zip`, plus `.sha256` and `.zip.sig` (Minisign) |
| Updater feed | `latest.json` on `launcher-current`, Tauri v2 static format, platform `windows-x86_64` |
| Compiled version | `YYYY.M.D` (D-LX8) from `CIMMERIA_LAUNCHER_VERSION` |
| Updater public key | `LAUNCHER_UPDATER_PUBKEY` at compile time; feed URL compiled in as the `launcher-current` `latest.json` |

## Wave 0

### LX-01: shared `cimmeria-launcher-core` crate

- **Agent:** rust-gameserver-dev. **Branch:** `launcher-consolidation/lx-01-core`. **Worktree:** `lx-01`.
- **Do:** create `crates/launcher-core/` as in the contract. `git mv` each module from `crates/launcher/src/` so history follows. Move the dependencies those modules need from `crates/launcher/Cargo.toml` into the new crate. Fix `crate::` paths inside the moved files. Where a moved file needed something the desktop engine supplied through its own `crate::` (the `#[path]` trick allowed that), make it an explicit parameter or a small trait in the core crate, not a back-reference.
- Add the crate to the root `Cargo.toml` members; add it to the desktop engine as `cimmeria-launcher-core = { path = "../../../launcher-core" }`; regenerate the desktop `Cargo.lock` with `--locked` dropped once, then commit the lock.
- Run `cargo hakari generate` and `cargo hakari manage-deps --yes` through the lane, then `python tools/crate-graph/crate_graph.py --check`.
- **Tests:** none new. Every moved test must still run: compare `cargo nextest list` counts for `sgw-launcher` before plus `cimmeria-launcher-core` after against `sgw-launcher` before. Put the before and after counts in the commit body.
- **Checks:** R1 for `cimmeria-launcher-core` and `sgw-launcher`; D1, D2, D3 for the engine.
- **Docs:** `crates/README.md` crate table row; `docs/readme.md` is unaffected.
- **Commit:** `refactor(launcher): shared cimmeria-launcher-core crate replaces the desktop engine's #[path] includes (LX-01)`.

### LX-02: scaffold the contract

- **Agent:** rust-gameserver-dev. **Depends on:** LX-01. **Worktree:** `lx-02`.
- **Do:** everything under "New preferences", "New native commands" and "New frontend files" above, with stub bodies. Mirror the structure of `game_telemetry_command` (`shell/src/main.rs`, `shell/src/host/game_telemetry.rs`, `engine/src/storage/game_telemetry/`, `frontend/src/game-telemetry-view.ts`).
- **Tests:**
  - engine unit: a v1 `preferences.json` (copy the shape from `engine/src/storage/tests.rs`) loads as v2 with `login_servers` empty and `client_patches_enabled` true. Fails if the migration is removed.
  - engine unit: `SaveClientOptions` with a stale `expected_revision` is refused and writes nothing, like `command_version_is_checked_before_saving`.
  - the `bridge_has_no_native_observation_or_arbitrary_field_command` test gains a `save_client_options` line with an unknown field.
- **Checks:** D1, D2, D3, F1.
- **Docs:** `crates/launcher/desktop/README.md` command list.
- **Commit:** `feat(launcher-desktop): scaffold client options, log upload, maintenance and changes commands (LX-02)`.

### LX-25: adopt the desktop launcher's open acceptance gates

- **Agent:** documentation-writer. **Worktree:** `lx-25`.
- **Do:** read `docs/analysis/playtests/2026-10-03-macos-wine/launcher-acceptance.md` (open items, lines 109-125) and the ledger README there. Copy every open Windows gate into a new `docs/analysis/launcher-consolidation/acceptance.md`, one row each, with a column naming the LX packet or LX-26 UAT step that closes it. Leave macOS-only gates in their ledger and add one line to that ledger's README pointing launcher Windows work here.
- **Checks:** `pwsh tools/lint-md.ps1` on the two files. Keep CRLF line endings.
- **Commit:** `docs(launcher): consolidation ledger owns the desktop launcher's open Windows gates (LX-25)`.

## Wave 1 (parallel once LX-02 merges)

### LX-03: server list editor

- **Agent:** packet-coder. **Files:** `engine/src/storage/mod.rs` (`save_client_options`), the launch preparation that calls `client_setup::prepare` (find it with `Grep "default_servers" crates/launcher/desktop`), `frontend/src/server-list-view.ts`, its test, `frontend/server-list-uat.mjs`.
- **Do:** Play writes `Preferences.login_servers` into `LoginInternal.lua`, or `default_servers()` when the list is empty. Adopted installs keep their reviewed list, as today. The view is a textarea, one `Name = http://host:port` per line, parsed with the core crate's existing parser; a parse error is shown under the box and nothing is saved; Save shows "Saved" on success.
- **Tests:** engine unit: with two saved servers, the prepared `LoginInternal.lua` contains both and not the default (fails if Play ignores the preference). Frontend unit: bad line shows the error and sends no command. UAT script: enter, save, reload, list persists.
- **Checks:** D1, D2, D3, F1, `npm run uat:server-list`.
- **Commit:** `feat(launcher-desktop): edit the login server list in Settings (LX-03)`.

### LX-04: client-patches toggle

- **Agent:** packet-coder. **Files:** the launch plan in `engine/src/storage/launch/` that picks the DLLs, `frontend/src/client-patches-view.ts`, its test, UAT script.
- **Do:** when `client_patches_enabled` is false, Play injects no client-patches DLL (the telemetry DLL still follows its own opt-in). Checkbox "Load client patches" saves through `SaveClientOptions`.
- **Tests:** engine unit on the launch plan: false gives a plan without the patches DLL; true keeps it. Frontend unit and UAT.
- **Checks:** D1-D3, F1, `npm run uat:client-patches`.
- **Commit:** `feat(launcher-desktop): Load client patches setting (LX-04)`.

### LX-05: debug log upload

- **Agent:** packet-coder. **Files:** `engine/src/storage/log_upload/`, `shell/src/host/log_upload.rs`, `frontend/src/log-upload-view.ts`, test, UAT.
- **Do:** port `cimmeria_launcher_core::logs`: zip `Binaries/sgwdebuglog*` and `sessions/**` under the install's `game` root, PUT to the SAS URL compiled in from `LAUNCHER_LOG_SAS_URL` (`option_env!`), skip a repeat of the same digest using a ledger in the launcher state directory (keep 100). No SAS compiled in means `Inspect` reports `Disabled` and the button is hidden. The button shows "Uploading…" then "Uploaded" or the error.
- **Tests:** engine with `wiremock`: one PUT with the expected blob name pattern; a second upload of the same files sends nothing (fails if the ledger is dropped). Frontend unit and UAT.
- **Checks:** D1-D3, F1, `npm run uat:log-upload`.
- **Docs:** add the `LAUNCHER_LOG_SAS_URL` row to `crates/launcher/desktop/docs/release.md`.
- **Commit:** `feat(launcher-desktop): upload debug logs (LX-05)`.

### LX-06: reset client cache and client state

- **Agent:** packet-coder. **Files:** `engine/src/storage/client_maintenance/`, `shell/src/host/client_maintenance.rs`, `frontend/src/client-maintenance-view.ts`, test, UAT.
- **Do:** port `cimmeria_launcher_core::client_paths`. `ResetCache` deletes `Firesky\SGWGame\Cache.en-US` under the user's Documents; `ResetAllClientState { confirmed: true }` deletes the whole `Firesky` tree; `confirmed: false` is refused. Both refuse while Play is running. The second button opens a confirm step in the view.
- **Tests:** engine unit with a temp profile: each reset removes exactly its tree; unconfirmed reset deletes nothing. Frontend unit: the confirm step is required. UAT.
- **Checks:** D1-D3, F1, `npm run uat:client-maintenance`.
- **Commit:** `feat(launcher-desktop): reset client cache and client state (LX-06)`.

### LX-07: "Changes to your client" panel

- **Agent:** packet-coder. **Files:** `engine/src/storage/client_changes/`, `shell/src/host/client_changes.rs`, `frontend/src/client-changes-view.ts`, test, UAT.
- **Do:** port `cimmeria_launcher_core::client_changes`: setup steps, one row per applied patch (manifest title and description, else the built-in catalog, else the id), the DLLs actually injected (honouring LX-04's flag when it lands; read the preference, do not assume), and server-pushed cooked data. Read-only.
- **Tests:** engine unit with a manifest fixture: a patch with a title shows the title, one without shows the catalog entry. Frontend unit and UAT.
- **Checks:** D1-D3, F1, `npm run uat:client-changes`.
- **Commit:** `feat(launcher-desktop): Changes to your client panel (LX-07)`.

### LX-08a: telemetry transport

- **Agent:** rust-gameserver-dev. **Files:** new `engine/src/storage/game_telemetry/transport/`.
- **Do:** port token refresh on 401, the drop-oldest disk queue (`telemetry-queue.jsonl` in the launcher state directory, recovered at startup), and gzipped NDJSON chunk upload with 429/503 back-off, from `cimmeria_launcher_core::telemetry::{auth,queue,runner}`. Keep the desktop's 8 s mint bound and the core endpoint policy (https anywhere; http only to this machine or a login-server host). Nothing runs unless the player opted in to game telemetry.
- **Tests:** `wiremock`: 401 then refresh then success; 429 backs off and retries; queue survives a reopen. The endpoint-policy tests already in core keep passing.
- **Checks:** D1-D3.
- **Commit:** `feat(launcher-desktop): telemetry token refresh, queue and chunk upload (LX-08a)`.

### LX-09: single instance

- **Agent:** packet-coder. **Files:** `shell/Cargo.toml`, `shell/src/main.rs`.
- **Do:** add `tauri-plugin-single-instance` (version matching the pinned `tauri` 2 minor). A second launch focuses the existing window and exits 0. During an updater relaunch the old process has exited before the new one starts (check `with_updater_shutdown`), so no special case is needed; if it has not, say so in the report instead of adding one.
- **Tests:** none automatable in CI. Add a row to LX-26.
- **Checks:** D1-D3 for `cimmeria-launcher-desktop`.
- **Commit:** `feat(launcher-desktop): a second launch focuses the running launcher (LX-09)`.

### LX-10: Windows in-place adoption (BlockedDecision D-LX12)

- **Agent:** rust-gameserver-dev, reviewed by `testing-validation-engineer`.
- **Do once D-LX12 is approved:** a Windows adoption backend in `engine/src/storage/adoption/` that adopts the chosen folder in place. Verify stock files and applied patches against the signed release, using `launcher-installed.json` to know which patches the egui launcher applied. Write the owner marker. Uninstall of an adopted folder removes only release-listed files. Update the adoption view so Windows no longer says "no adoption backend".
- **Tests:** fixture install directory with a fake `launcher-installed.json`: adopt succeeds; one modified stock file makes it fail with the file named; uninstall leaves an unlisted user file in place.
- **Checks:** D1-D3, F1, `npm run uat:adoption`.
- **Commit:** `feat(launcher-desktop): adopt an existing Windows install in place (LX-10)`.

### LX-11: WebView2 runtime check

- **Agent:** packet-coder. **Files:** `shell/src/main.rs`, new `shell/src/webview2.rs`, `shell/Cargo.toml` (`windows-sys` features if needed).
- **Do:** on Windows, before `tauri::Builder`, read the WebView2 `pv` value under `SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}` (HKLM, then HKCU, and the non-WOW6432Node path). Missing or `0.0.0.0`: show a `MessageBoxW` saying the Microsoft Edge WebView2 Runtime is needed, offer Yes to open `https://go.microsoft.com/fwlink/p/?LinkId=2124703` with `ShellExecuteW`, then exit 1.
- **Tests:** unit test on the version-string check (`""`, `"0.0.0.0"` missing; `"128.0.2739.42"` present), with the registry read behind a function parameter.
- **Checks:** D1-D3 for `cimmeria-launcher-desktop`.
- **Commit:** `feat(launcher-desktop): explain a missing WebView2 runtime instead of failing silently (LX-11)`.

### LX-12: Windows PhysX setup

- **Agent:** rust-gameserver-dev. **Files:** `engine/src/storage/runtime_setup/`, `engine/src/prerequisites/`, `runtime-probe/src/windows_physx.rs`, the runtime-setup UI.
- **Do (D-LX13):** on Windows, runtime setup detects PhysX System Software and, when missing, offers to run the pinned vendor MSI, verified with the hash the macOS path uses, elevated with `ShellExecuteExW` verb `runas`. Play warns, without blocking, when PhysX is missing.
- **Tests:** detection unit tests with the registry and DLL probe injected; MSI hash mismatch refuses to run.
- **Checks:** D1-D3, F1.
- **Docs:** `crates/launcher/desktop/docs/prerequisites.md`; close [#1121](https://github.com/SandboxServers/Cimmeria/issues/1121) in the PR body if it covers it.
- **Commit:** `feat(launcher-desktop): install PhysX System Software on Windows (LX-12)`.

### LX-13: zip self-update apply on Windows

- **Agent:** rust-gameserver-dev, reviewed by `testing-validation-engineer`.
- **Files:** `engine/src/storage/updater/apply/` (new `zip_swap.rs`), `engine/src/storage/updater/` feed and key configuration.
- **Do (D-LX3):** on Windows the package is the release zip, not an installer. After the existing Minisign and version checks: extract to `<exe dir>\.update-<version>\`; rename every file the zip replaces to `<name>.old` (a running exe can be renamed on Windows); move the new files in; relaunch; the new process acknowledges startup as the updater already requires; on the next start delete `*.old` and the staging folder. If any move fails, rename the `.old` files back and report Failed. Compile the feed URL and public key from the release names in the contract; with neither set, the status stays Disabled as today.
- **Tests:** temp-dir fixtures: a full swap leaves only new files; a failure injected on the third move restores the originals byte for byte (fails if rollback is removed); a startup with `.old` files cleans them.
- **Checks:** D1-D3, F1, `npm run uat:updater-apply`.
- **Docs:** `crates/launcher/desktop/docs/updater.md` Windows section.
- **Commit:** `feat(launcher-desktop): Windows self-update by zip swap (LX-13)`.

### LX-14: move `pack-client-overlay`

- **Agent:** packet-coder. **Files:** `crates/launcher/src/bin/pack-client-overlay.rs`, `crates/launcher/src/overlay_pack.rs`, `crates/launcher/Cargo.toml`, `crates/patchset/Cargo.toml`, `crates/patchset/src/`, `tools/launcher-release/build.sh` (edit the `overlay` stage's `cargo run -p` line only).
- **Do:** `git mv` both into `crates/patchset/` as `src/bin/pack-client-overlay.rs` and `src/overlay_pack.rs`, with its tests. The binary name stays `pack-client-overlay`. Remove the `[[bin]]` from the egui `Cargo.toml`.
- **Tests:** the moved tests run under `-p cimmeria-patchset`.
- **Checks:** R1 for `cimmeria-patchset` and `sgw-launcher`.
- **Commit:** `refactor(patchset): pack-client-overlay moves out of the egui launcher (LX-14)`.

### LX-19: delete `tools/SGWLauncher/` and the prototypes

- **Agent:** packet-coder. **Files:** `tools/SGWLauncher/`, `crates/launcher/prototype-packaging/`, `crates/launcher/prototype-macos/`, root `Cargo.toml` (`exclude` line for `tools/SGWLauncher/src-tauri`), `docs/readme.md` rows for the packaging proof, any `Grep` hit for those three paths outside `docs/analysis/`.
- **Do:** `git rm -r` the three trees and remove every reference found. Leave historical mentions in `docs/analysis/` alone.
- **Do not touch the repo-root `src-tauri/`.** That is `cimmeria-app`, the Cimmeria Admin desktop app, not a launcher. Only `tools/SGWLauncher/src-tauri` goes, and only its line in the root `Cargo.toml` `exclude` list; the `"src-tauri"` member line stays.
- **Checks:** `cargo metadata --format-version 1 --no-deps` succeeds; `pwsh tools/lint-md.ps1` on the touched docs.
- **Commit:** `chore(launcher): remove the dead Tauri prototype and the packaging prototypes (LX-19)`.

## Wave 2

### LX-08b: telemetry session

- **Agent:** rust-gameserver-dev. **Depends on:** LX-08a.
- **Do:** from Play to game exit: tail `SGWDebugLog.log` and session files into the LX-08a queue, record the game's exit code and pid, and post the end-of-session bundle. Port from `telemetry::{tail,process_watch,runner}`. Closing the launcher while the game runs must not lose the queue.
- **Tests:** a fake log grown in steps is shipped once per line; a fake process exit records its code.
- **Commit:** `feat(launcher-desktop): game session log tailing and exit tracking (LX-08b)`.

### LX-08c: telemetry events

- **Agent:** packet-coder. **Depends on:** LX-08a.
- **Do:** emit `client.launcher.install_result` after install, Repair and game Update; the client-patches boot verdict; Black Market claimed/delivered/dropped counts. Use the field names in `telemetry::{install_result,patch_log,patch_counts}` unchanged so SigNoz queries keep working.
- **Tests:** byte-exact JSON fixture per event, copied from the egui tests.
- **Commit:** `feat(launcher-desktop): install result, patch verdict and BM count events (LX-08c)`.

### LX-15: release zip only, `launcher-current`, signed feed

- **Agent:** rust-gameserver-dev. **Depends on:** LX-13, LX-14. A live run needs D-LX9.
- **Files:** `.github/workflows/launcher-release.yml`, `launcher-release-on-comment.yml`, `tools/launcher-release/build-desktop.sh`, `crates/launcher/desktop/docs/release.md`.
- **Do:** remove the egui `release` job; the `desktop-windows` job creates the dated release itself. Name the zip as in the contract. Sign the zip with Minisign using `LAUNCHER_UPDATER_MINISIGN_KEY` with a trusted comment carrying `version:<YYYY.M.D>`; write `latest.json`; delete and recreate the assets on `launcher-current`. Stamp `CIMMERIA_LAUNCHER_VERSION`, `LAUNCHER_UPDATER_PUBKEY`, `LAUNCHER_LOG_SAS_URL` and `LAUNCHER_MANIFEST_PUBKEY_HEX`. Without the Minisign secret the job still builds and uploads the dated release, skips `launcher-current`, and says so in the job summary. Remove the "preview" wording and the "original launcher is still supported" note.
- **Tests:** `launcher-desktop.yml`'s dry run covers the build. Add a step that verifies the produced `latest.json` against a throwaway Minisign key generated in the job.
- **Commit:** `ci(launcher-release): publish the desktop launcher as a signed zip with a stable download link (LX-15)`.

### LX-17: window title, version stamp, About

- **Agent:** packet-coder.
- **Do:** the window title drops "development" in release builds; Settings shows the compiled version and tag; `tauri.conf.json` version reads `CIMMERIA_LAUNCHER_VERSION` when set.
- **Tests:** frontend unit on the About line.
- **Commit:** `feat(launcher-desktop): release title and version in Settings (LX-17)`.

## Wave 3 (after D-LX1 sign-off and D-LX14)

### LX-18: delete the egui launcher

- **Agent:** packet-coder.
- **Do:** `git rm -r crates/launcher/src crates/launcher/build.rs crates/launcher/Cargo.toml crates/launcher/gen crates/launcher/binaries crates/launcher/icons` after checking with `Grep` that the desktop shell does not read `crates/launcher/icons` (if it does, `git mv` them into `crates/launcher/desktop/shell/icons` first). Remove `crates/launcher` from root members. Remove `sgw-launcher` from `.config/hakari.toml`, `.cargo/config.toml` (`test-workspace`), `codecov.yml` (`launcher` flag and component), `tools/crate-graph/groups.toml`, `tools/build-metrics/measure-build.ps1`, `.gitignore`, `docker/Dockerfile.dockerignore`, and the `--exclude sgw-launcher` in `CLAUDE.md`, `.claude/skills/lane-build/SKILL.md` (then `python tools/agent-skills/sync.py`), `docs/agents/pre-pr-checks.md`, `CONTRIBUTING.md`, `TESTING.md`, `docs/building.md`, `docs/guides/getting-started.md`, `.github/workflows/test.yml`. Excludes go from seven to six everywhere.
- **Checks:** `cargo hakari generate --diff` and `manage-deps --dry-run` clean; `pwsh tools/build-lane/lane.ps1 --exclusive cargo check --workspace <six excludes>`.
- **Commit:** `chore(launcher): retire the egui launcher (LX-18)`.

### LX-16: CI

- **Agent:** packet-coder. **Depends on:** LX-18.
- **Do:** rename `launcher-build.yml` to `client-launch.yml` keeping only the `cimmeria-client-launch` and `cimmeria-start32` jobs and their coverage flag; delete the `cargo check -p sgw-launcher` step in `launcher-desktop.yml`.
- **Commit:** `ci: client-launch workflow replaces the egui launcher workflow (LX-16)`.

### LX-20: `setup.ps1` and `bootstrap/`

- **Agent:** packet-coder. **Depends on:** LX-18.
- **Do:** `-WithLauncher` downloads the current release zip from `launcher-current` and unpacks it beside the server, instead of building. Remove `Build-CimmeriaLauncher.ps1` and the 7za sidecar install, or point them at the zip; say which in the report.
- **Commit:** `chore(setup): -WithLauncher fetches the released launcher zip (LX-20)`.

### LX-21: references in other crates

- **Agent:** packet-coder. **Depends on:** LX-18.
- **Do:** fix comment and doc paths that pointed into `crates/launcher/src/`: `crates/lab/src/supervisor/process.rs`, `crates/lab/src/**/login_servers.rs`, `crates/client-telemetry/src/lib.rs`, `crates/client-launch/README.md`, `crates/client-patches/README.md`, `data/client-patches/README.md`. Point them at `crates/launcher-core/`.
- **Commit:** `docs: launcher paths point at cimmeria-launcher-core (LX-21)`.

## Wave 4: docs

### LX-22: player docs and announcement

- **Agent:** documentation-writer. **Depends on:** LX-15.
- **Do:** rewrite `docs/client/launcher-guide.md` around the zip: the stable download link, unzip anywhere and keep `windows/` beside the exe, first run (WebView2, PhysX, SmartScreen), adopting an existing install, settings import from the old launcher, troubleshooting. README download section. `docs/guides/macos.md` per D-LX14. Write `docs/analysis/launcher-consolidation/announcement.md`, the Discord and release-notes text for D-LX2.
- **Commit:** `docs(launcher): player guide for the zipped launcher (LX-22)`.

### LX-23: developer and operator docs

- **Agent:** documentation-writer. **Depends on:** LX-18.
- **Do:** archive `docs/client/sgw-launcher.md` (header note pointing at the desktop README); rewrite `docs/client/launcher-self-update.md` and `launcher-distribution-setup.md` for the zip updater and the three keys; update `docs/readme.md`, `docs/agents/doc-update-map.md`, `crates/README.md`, `README.md` diagram, `SECURITY.md`, `docs/troubleshooting.md`, `docs/operations/telemetry.md`, the `docs/architecture/` launcher mentions, `docs/testing/inventory/launcher.md`, `.github/instructions/launcher.instructions.md`, `.github/copilot-instructions.md`, and `crates/launcher/desktop/docs/maintenance.md` (it still says Repair is not implemented).
- **Commit:** `docs(launcher): one launcher in the developer docs (LX-23)`.

### LX-24: agent-memory sweep

- **Agent:** documentation-writer. **Depends on:** LX-18.
- **Do:** for each `.claude/agent-memory/**` note that names `sgw-launcher` or the egui launcher, add a dated line saying it is retired and what replaced it, or delete the note if nothing in it still applies. Update the indexes.
- **Commit:** `chore(memory): annotate egui-launcher notes after its retirement (LX-24)`.

## Wave 5

### LX-26: Windows UAT of the released zip

Owner-run, on a Windows machine with no launcher state, from the `launcher-current` download. Each row records pass or fail in `acceptance.md`. Play rows can also run through the lab (`lab-uat` skill, ask the user first).

1. Download and unzip from the stable link; first run with WebView2 present, and once on a machine or VM without it (LX-11).
2. Fresh install from the archive.org RAR; Play reaches character select.
3. PhysX missing: setup offers and installs it (LX-12).
4. Adopt an existing egui install in place; Play works without a reinstall (LX-10).
5. Import egui settings; the server list carries over (LX-03).
6. Server list edit, client-patches toggle off and on, both visible in "Changes to your client" (LX-03, LX-04, LX-07).
7. Reset cache; reset all with confirm (LX-06).
8. Upload logs twice; the second says nothing new (LX-05).
9. Telemetry opted in: a session shows in SigNoz with tailed log lines and exit code (LX-08).
10. Second launch focuses the first (LX-09).
11. Self-update from release N-1 to N; then a forced failed update rolls back (LX-13).
12. Repair, game Update and Uninstall on Windows (desktop ledger gates moved by LX-25).

### LX-27: close-out

Coordinator. Update `docs/project-status.md`, `docs/gap-analysis.md` and its area file once; add the remaining owner UAT rows to `docs/guides/unified-uat.md`; retire every `lx-*` worktree with `pwsh tools/build-lane/rm-worktree.ps1 --merged`; post the handoff on the board; mark this ledger Done.
