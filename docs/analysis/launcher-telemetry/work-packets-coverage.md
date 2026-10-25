# Launcher telemetry: work packets (waves 3, 4 and 7)

Waves 5 and 6 (adoption reasons, file index) are in [work-packets-adoption.md](work-packets-adoption.md). The old wave 5 is now wave 7 (amendment 2026-10-21).

Ledger: [README.md](README.md). **The rules (R1-R10), checks (D*/C*/S*/F1/L0) and contract (C1-C13, C18) are in [work-packets.md](work-packets.md); read § Rules and your contract entries there first.** Every packet here is `Host: windows`. Each one branches from the `lt-app-integration` tip at dispatch (work-packets § Branches), unless it says otherwise.

The pattern for every wave-3 and wave-4 instrumentation packet:

- Put the tracing calls in one `events.rs` helper file per area (C6). The packet names the file.
- Edit only call lines in any file over 450 lines.
- Replace `let _ = x` with `if let Err(e) = x { events::<name>_failed(events::kind(&e)) }`, or the closest closed code when the error is not `io::Error`. Keep the original control flow; logging must never change behaviour.
- Replace `map_err(|_| X)` with `map_err(|e| { events::<name>_failed(<code from e>); X })`, taking the HTTP status from a `reqwest::Error` with `e.status().map(|s| s.as_u16())`.
- One `<op>_started` and one `<op>_finished`/`<op>_failed` per command action, at the shell command boundary. Do not log progress ticks.
- Tests (R7): each helper under capture, plus the real failing path the packet names.

**Amendment 2 (2026-10-21, directives 5 and 6).** This file adds LT-40 (the launcher IP guard, first in wave 3) and LT-41 (the built-in https default, PR H, BlockedDecision D-LT17). It re-specifies LT-21, LT-22, LT-23, LT-25, LT-26 and LT-27. Every summary and default upload URL is https, and no launcher row, field or uploaded log holds an IP literal.

**Amendment 4 (2026-10-21, directive 7).** D-LT17 is resolved: the colo serves `https://play.cimmeria.app` through the board's Caddy proxy. LT-41 is unblocked (BlockedDependency on PR S, PR L and its status-code probe). LT-27 gains U10 (LT-42's per-client quotas behind the proxy).

**Amendment 6 (2026-10-21, LT-14 packet gaps).** LT-14 is split into LT-14a (core error codes and core `launcher.install` events) and LT-14b (engine install lifecycle). The split adds two shared engine modules that later packets reuse: `operation_numbers` (a process-local `operation: u64` per operation UUID, plus `duration_ms` from first sight) and `error_codes` (`code()` on `ContractError`, `StorageError` and `IntentError`). LT-16 and LT-17 were pre-checked against base `3d10ddf4d` and re-sliced into LT-16a/LT-16b and LT-17a/LT-17b with named call lines and value sources. The amended packets live in [work-packets-install.md](work-packets-install.md) and [work-packets-repair-launch.md](work-packets-repair-launch.md), which keeps this file under the line cap. C6 gains `phase`, `patch` and the named booleans (work-packets.md § C6). Report: [LT-14-amendment.md](LT-14-amendment.md).

**Amendment 7 (2026-10-21, wave-4 pre-check).** LT-18..LT-23 were checked against the `lt-app-integration` tip and re-sliced into LT-18, LT-19, LT-20, LT-21a, LT-21b, LT-22, LT-23a and LT-23b, in five parallel lanes, with named call lines, value sources and the capture standard. They live in [work-packets-wave4.md](work-packets-wave4.md). Report: [LT-18-23-amendment.md](LT-18-23-amendment.md).

**Amendment 8 (2026-10-21, LT-25 packet gaps).** The LT-25 coder returned blocked: steps 4 and 5 called helpers private to other modules, and the `.mjs` path and branch name were wrong. LT-25 is re-specified against `b9fc68cc3` with two seams (`pub(crate) record_outcome`; a `test-support` `launch_not_started_for_test`), the real store open for step 1, and exact file:line anchors. Report: [LT-25-amendment.md](LT-25-amendment.md).

## Wave 3: subsystem coverage A

### LT-40: launcher IP guard: scrub forwarded rows, and capture fails on an IP (amendment 2)

- **Host:** windows. **Worktree:** `lt-40`, branch `launcher-telemetry/lt-40-ip-guard`, from `lt-app-integration` after LT-05. It merges first in wave 3. **Rules:** R1-R9. **Contract:** C2, C9.
- **Why:** directive 6. R5 already forbids an IP in a forwarded field, but nothing enforces it. A stray `?e` of a `reqwest::Error` prints the URL, and a URL can hold an IP host. This packet enforces the rule twice: at the one wire chokepoint, and in every test that uses the C9 capture helpers.
- **Files:**
  - new `engine/src/storage/game_telemetry/ip_scrub.rs` (≤150 lines, with an inline `#[cfg(test)] mod tests`), declared in `game_telemetry/mod.rs`;
  - `engine/src/storage/game_telemetry/launcher_event.rs`: the function that turns `LauncherRecord`s into wire rows (`launcher_events`, C2; grep it). Call lines only;
  - `engine/src/test_capture.rs` and `shell/src/test_capture.rs` (C9).
- **Change:**
  1. `ip_scrub.rs`:
     ```rust
     /// True when `s` holds an IPv4 or IPv6 literal: a token split on chars outside
     /// `[0-9A-Fa-f:.\[\]%]` that parses as `std::net::IpAddr` after stripping brackets
     /// and a `%zone`, either whole or with one trailing `:port` removed (try whole first,
     /// so `fe80::1` is not cut to `fe80:`). A `[addr]` or `[addr]:port` token parses
     /// only the text between the brackets.
     pub fn contains_ip(s: &str) -> bool;
     /// Replace every such token with "[ip]". Borrowed when nothing changes.
     pub fn scrub_ips(s: &str) -> std::borrow::Cow<'_, str>;
     ```
     No new dependency. Never hash; the replacement is the fixed text `[ip]`.
  2. In `launcher_events`, pass the message and every string field value through `scrub_ips` after AU's `Redaction`, so nothing on the wire can hold an IP.
  3. In both C9 helpers, the accessor that returns the captured events panics with the event name and field key when `contains_ip` is true for a message or a string field value. From this packet on, every instrumentation test fails if its event carries an IP.
- **Tests:**
  - `contains_ip` / `scrub_ips` cases:
    - true: `198.51.100.7`, `http://198.51.100.7:8081/x`, `[2001:db8::1]:8081`, `fe80::1%3`;
    - false: `0.9.3`, `10.0.26300.0`, `12:34:56`, `launcher.install`, a SHA-256 hex string, `C:\Users\x` (the path is Redaction's job, not this one);
    - scrub output: `error at [ip]:8081`.
  - A `launcher_events` test: a record whose message and one field hold `198.51.100.7` comes out with `[ip]` in both. Reverted, the IP reaches the wire row and the test fails.
  - A C9 test: capturing an event with field `x = "198.51.100.7"` and reading it panics (`#[should_panic]`).
- **Checks:** D1, D2e, D2s, D3e `game_telemetry`, D3e `test_capture`, D3s `test_capture`. Then run D3e and D3s over the whole crate once (D3e and D3s with no filter), so that any existing test that captured an IP is listed in the report; fix only its fixture value.
- **Docs:** none (LT-26).
- **Commit:** `LT-40: scrub IP literals from forwarded launcher rows and fail captures that hold one`.

### LT-12: updater check, download and verify

- **Worktree:** `lt-12`, branch `launcher-telemetry/lt-12-updater`. **Rules:** R1-R9. **Target:** `launcher.update`.
- **Files:**
  - new `engine/src/storage/updater/events.rs` (≤200 lines);
  - the engine `storage/updater/{release_check.rs, transport.rs, policy.rs, bundle.rs, mod.rs}` (mod.rs is 436 lines: call lines only);
  - `shell/src/host/updater/mod.rs` (~332 lines).
  - **Do not touch** `storage/updater/apply/*` (already structured) or `shell/src/host.rs` (LT-06).
- **Events:**
  - `update_check_finished` (info: `outcome` = `up_to_date`|`available`|`disabled`, `duration_ms`);
  - `update_check_failed` (warn: `reason`, `http_status`);
  - `update_download_failed` (warn: `reason`, `http_status`), at transport.rs:53;
  - `update_downloaded` (info: `bytes`, `duration_ms`);
  - `update_verify_failed` (warn: `reason` = `signature`|`signed_version`|`feed`), at policy.rs :51, :98, :103, :113, :140;
  - `updater_disabled` (info: `reason`), when the shell's inspect finds the updater off;
  - in the shell, `update_action_finished` (info: `action` = `inspect`|`check`|`prepare`|`apply`, `outcome`, `duration_ms`).
  - Replace the `eprintln!` at shell `host/updater/mod.rs:~60` with a warn that has a `reason`.
- **Test:** extend one existing policy test in `storage/updater/tests.rs` that feeds a bad signature, and assert `update_verify_failed reason=signature`.
- **Checks:** D1, D2e, D2s, D3e `updater`, D3s `updater`.
- **Commit:** `LT-12: launcher updater check, download and verify events`.

### LT-13: game update and release catalog

- **Worktree:** `lt-13`, branch `launcher-telemetry/lt-13-game-update`. **Rules:** R1-R9. **Target:** `launcher.game_update`.
- **Files:**
  - new `engine/src/catalog/events.rs` (≤150 lines);
  - `engine/src/catalog/mod.rs` (:70, :86, :94, :118, :126, :129, :132);
  - new `shell/src/host/game_update/events.rs`;
  - `shell/src/host/game_update/{dispatch.rs, mod.rs}`;
  - engine `storage/update/{preparation.rs, commit.rs, recovery.rs, discard.rs, abandon.rs}` (call lines only).
  - **Do not edit `shell/src/main.rs`**: its `ReleaseUnavailable` mappings stay, because the cause is now logged at the source.
- **Events:**
  - `release_fetch_failed` (warn: `reason` = `network`|`http_status`|`body`|`signature`|`invalid_manifest`, `http_status`);
  - `release_fetched` (info: `duration_ms`);
  - `game_update_action_finished` (info: `action`, `outcome`, `duration_ms`), in dispatch;
  - `mark_uncertain_failed` (warn: `error_kind`), at dispatch.rs:175 and preparation.rs:172;
  - `update_staged` (info: `files`, `bytes`, `duration_ms`);
  - `update_committed`, `update_recovered` (`outcome`), `update_discarded`, `update_abandoned` (`reason`).
  - `let _ = send.send(result)` stays as it is: a gone receiver is transient, so at most a `debug`.
- **Test:** a catalog fetch against a closed local port (or the existing catalog test fake, if there is one; grep `fetch_release` in the tests) asserts `release_fetch_failed reason=network`.
- **Checks:** D1, D2e, D2s, D3e `catalog`, D3e `update`, D3s `game_update`.
- **Commit:** `LT-13: game update and release catalog events`.

### LT-14a and LT-14b: install (amendment 6)

Moved to [work-packets-install.md](work-packets-install.md): LT-14a (core error codes and core `launcher.install` events) and LT-14b (engine install lifecycle, `operation_numbers`, `error_codes`). LT-14a commits first; both share the `lt-14` worktree and branch.

### LT-15: seed archive, PhysX, migration, prerequisites

- **Worktree:** `lt-15`, branch `launcher-telemetry/lt-15-setup-extras`. **Rules:** R1-R9. **Target:** `launcher.install`.
- **Files:**
  - new `shell/src/host/setup_events.rs` (≤200 lines; the helpers for all four areas);
  - `shell/src/host/seed_archive.rs` (461 lines: call lines only, at :101, :113);
  - engine `storage/seed_archive/downloads.rs` (:49, :54);
  - `shell/src/host/physx.rs`;
  - engine `runtime_setup/windows_physx/mod.rs` (12 free-string warns: retarget, `%error` becomes `error_kind`/`reason`);
  - the shell migration host file (grep `migration_command`);
  - engine `prerequisite` supervisor `mod.rs` (grep `prerequisite`).
  - **Do not edit `shell/src/main.rs`.**
- **Events:**
  - `<area>_started` and `<area>_finished` (`action`, `outcome`, `duration_ms`) and `<area>_failed` (`reason`, `error_kind`, `http_status`), for `seed_archive`, `physx`, `migration` and `prerequisite`;
  - `seed_archive_release_failed` (warn: `reason`, `http_status`).
- **Test:** the helpers under capture, plus one seed-archive inspect failure through an existing seed-archive test (grep `seed_archive` tests).
- **Checks:** D1, D2e, D2s, D3e `windows_physx`, D3s `seed_archive`, D3s `setup_events`.
- **Commit:** `LT-15: seed archive, PhysX, migration and prerequisite events`.

### LT-16a, LT-16b, LT-17a and LT-17b (amendment 6)

Moved to [work-packets-repair-launch.md](work-packets-repair-launch.md): LT-16a (repair and uninstall), LT-16b (client maintenance and client changes), LT-17a (launch and Play lifecycle), LT-17b (game-telemetry session, helper stop, DLL artifact). Both pairs start after the LT-14 branch merges into `lt-app-integration`.

## Wave 4: subsystem coverage B

Moved to [work-packets-wave4.md](work-packets-wave4.md) by amendment 7: LT-18 (launch-worker stage and Win32 code), LT-19 (settings and consent events), LT-20 (frontend swallowed errors), LT-21a/b (log-upload events; launcher logs in the upload), LT-22 (https-only summary endpoint), LT-23a/b (shell endpoint resolution; consent box copy and feedback). They run in five parallel lanes after wave 3 and its review have merged.

## Wave 7: verification, docs, close-out (was wave 5)

### LT-24: SigNoz saved view, dashboard, offline guard (PR F)

- **Worktree:** `lt-24`, branch `launcher-telemetry/lt-24-signoz`, from `origin/main` after PR S. **Rules:** R1-R7, R9, R10.
- **Files:**
  - new `docs/operations/signoz/launcher-telemetry.view.json`: logs where `service.name = cimmeria-client` and `cimmeria.session_kind = launcher`;
  - new `docs/operations/signoz/launcher-telemetry.dashboard.json`, with these panels:
    1. events by `client_target`;
    2. warn and error by `client_target` and `reason`;
    3. the sum of `pipeline_recovered` and its `outage_ms`;
    4. `panic` and `previous_run_panicked` counts;
    5. `game_session_mint_failed` by `reason`;
    6. `cimmeria-server` `launcher.ingest` `event=upload_refused` by `route` and `reason`;
    7. `cimmeria-server` boot WARN and unreachable WARN by `reason`;
  - `crates/admin-api/src/routes/telemetry/launcher_summary/fixture_tests/` (the `VIEWS` table and `PANEL_CAVEATS`).
- **Change:**
  - Copy the key names and JSON shape exactly from `launcher-summary.view.json` and `launcher-journey.dashboard.json`.
  - Add both files to the offline guard, so that a misspelt field or index fails.
  - JSON files use LF, like the existing fixtures. Check them with `git diff`.
- **Test:** the guard passes; mutating one field name in a test copy fails, using the existing `breakage.rs` pattern.
- **Checks:** S1, S2, S3 `routes::telemetry::launcher_summary::fixture_tests`.
- **Docs:** `docs/operations/telemetry.md` gets one paragraph on importing these files after the first rows exist (CRLF).
- **Commit:** `LT-24: SigNoz view and dashboard for launcher telemetry`.

### LT-25: REPL logic UAT `uat:launcher-telemetry` (amendment 8)

- **Host:** windows. **Worktree:** `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-25`, branch `launcher-telemetry/lt-25`, at the launcher integration tip `b9fc68cc3` (waves 2-4 in). Fresh coder; the worktree is unedited. **Rules:** R1-R9 (inlined below). **Contract:** C1, C2, C7, C9. **Amendment 8** ([LT-25-amendment.md](LT-25-amendment.md)) replaces the first spec: steps 4 and 5 now go through two named seams, step 1 uses the real store open, and the paths are fixed.
- **Rules for this packet (from work-packets § Rules):**
  1. PowerShell only. Start every command with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-25;`, run it in the foreground and wait. Never call `cargo` directly; only `pwsh tools/build-lane/lane.ps1 cargo ...`. At most 5 fix rounds per check.
  2. Edit only the six files below. No `git stash`, no `git worktree prune`, no push, no rebase. One local commit with the subject below, ending with the two trailer lines from work-packets § R2.
  3. File caps: soft 500, hard 700 lines. Report the line count of every file you touched.
  4. Code files keep their line endings. `git diff --stat` shows only your lines.
  5. No value in any printed row may hold a path, URL, host, IP, account or user name (R5, D-LT18). The test itself prints only what `launcher_batch` returns.
  6. Do not edit the call sites of `launch_not_started` (`engine/src/storage/launch/worker.rs:158,172,273`, `supervisor.rs:24,28,44`) or `record_outcome` (`shell/src/telemetry.rs:244`). They stay as they are.
  7. If something below does not compile as written and the fix is not a one-line import or type path, return `blocked` with the error; don't invent a design (R9). Report at `<worktree>\.claude\reports\launcher-telemetry\LT-25-coder.md` (overwrite the old one) and return `LT-25 <done|blocked|failed> <sha|-> <report-path>`.
- **Files** (all under `crates/launcher/desktop/`):
  1. `engine/src/storage/launch/events.rs` (387 lines): add one test-support seam after line 45 (the end of `launch_not_started`). Nothing else in the file changes.
  2. `shell/src/telemetry.rs` (313 lines): line 200, `fn record_outcome(` becomes `pub(crate) fn record_outcome(`. One word; nothing else.
  3. `shell/src/host.rs` (380 lines): after line 20 (`mod launcher_telemetry;`) add the two lines `#[cfg(test)]` and `mod launcher_telemetry_uat;`.
  4. new `shell/src/host/launcher_telemetry_uat.rs` (≤150 lines).
  5. new `frontend/launcher-telemetry-native-uat.mjs` (≤120 lines), next to `frontend/game-telemetry-native-uat.mjs`. There is no `frontend/scripts/` folder; do not make one.
  6. `frontend/package.json`: one script line after line 26 (`"uat:seed-archive": ...`; add the comma to line 26): `"uat:launcher-telemetry": "node launcher-telemetry-native-uat.mjs"`. No esbuild step: this UAT mounts no view.
- **Change 1, the engine seam** (`events.rs`, after line 45). The repo already gates test seams on the engine's `test-support` feature (`engine/Cargo.toml:8`; for example `updater/mod.rs:26`), and the shell's dev-dependency turns it on (`shell/Cargo.toml:37`). `launch` and `launch::events` are public (`engine/src/lib.rs:62`, `launch/mod.rs:12`); only the helper is `pub(crate)`. Add exactly:
  ```rust
  /// Test seam (launcher-telemetry LT-25): the real `launch_not_started` event, for
  /// the shell's launcher-telemetry UAT bridge. Test-support builds only.
  #[cfg(feature = "test-support")]
  pub fn launch_not_started_for_test(id: Uuid, reason: &'static str) {
      launch_not_started(id, reason);
  }
  ```
- **Change 2, the bridge test** (`launcher_telemetry_uat.rs`, a child of `host`, so it can set the private `launch_resources` field):
  - Module doc: `//! LT-25 bridge for launcher-telemetry-native-uat.mjs: real emitters, the real ForwardLayer and launcher_batch; one JSON row per line. Temporary state only; nothing is sent.`
  - Imports (exact paths): `super::NativeHost`; `crate::host::GameTelemetryCommand`; `crate::host::ui_event::ui_event`; `crate::telemetry::{record_outcome, ForwardLayer, LauncherRecord, Redaction, FORWARD_CAPACITY}`; `cimmeria_launcher_engine::NativeCommand`; `cimmeria_launcher_engine::client_setup::login_servers::default_servers`; `cimmeria_launcher_engine::game_telemetry::health::{SharedHealth, TelemetryHealth}`; `cimmeria_launcher_engine::game_telemetry::launcher_session::SendOutcome`; `cimmeria_launcher_engine::game_telemetry::ip_scrub::contains_ip`; `cimmeria_launcher_engine::launch::events::launch_not_started_for_test`; `cimmeria_launcher_engine::launch::{Artifact, Resources}`; `sha2::{Digest, Sha256}`; `std::sync::{Arc, Mutex}`; `tracing_subscriber::{layer::SubscriberExt, Registry}`; `uuid::Uuid`.
  - Helpers `artifact` and `host`: copy `shell/src/host/launcher_telemetry.rs:40-58` verbatim, then drop the `with_dll` and `tag` parameters: the DLL artifact is always present (`client_telemetry: Some(artifact(root, "telemetry.dll"))`) and the state folder is `root.join("state")`.
  - One test:
    ```rust
    #[test]
    #[ignore = "JSON-lines bridge for launcher-telemetry-native-uat.mjs"]
    fn uat_bridge_launcher_telemetry() { ... }
    ```
    Body, in this order:
    1. `tempfile::tempdir()`, `canonicalize()` (as `launcher_telemetry.rs:62-63`); `let host = host(&root);`; `let (tx, rx) = std::sync::mpsc::sync_channel(FORWARD_CAPACITY);`; `let health: SharedHealth = Arc::new(Mutex::new(TelemetryHealth::default()));`.
    2. `tracing::subscriber::with_default(Registry::default().with(ForwardLayer::new(tx)), || { ... });`. The closure's first line is `tracing::callsite::rebuild_interest_cache();` (the capture standard, amendment 7). Inside, in this order:
       - **Step 1, startup.** `let snapshot = host.dispatch(NativeCommand::Inspect { schema_version: 1 }).unwrap();`. The first store open calls `startup_event::emit_startup` itself (`host.rs:228`), so this emits `launcher_started` on the production path. Do not call `emit_startup` directly.
       - Opt in, so that `launcher_batch` returns rows: `host.game_telemetry_command(GameTelemetryCommand::Set { schema_version: 1, opted_in: true }).unwrap();` (as `launcher_telemetry.rs:83-87`).
       - **Step 2, UI.** `ui_event("install".into(), "install_failed".into(), "transport".into(), "warn".into()).unwrap();` (argument order area, event, code, level; `ui_event.rs:84`).
       - **Step 3, settings.** Re-read the revision: `let revision = host.dispatch(NativeCommand::Inspect { schema_version: 1 }).unwrap().preferences.revision;`. Then `host.dispatch(NativeCommand::SaveClientOptions { schema_version: 1, expected_revision: revision, login_servers: default_servers(), client_patches_enabled: snapshot.preferences.client_patches_enabled }).unwrap();`. This emits `client_options_saved` (`commands.rs:117`).
       - **Step 4, pipeline recovery.** For `(at, outcome)` in `[SendOutcome::MintFailed { reason: "transport" }, SendOutcome::Dropped, SendOutcome::Sent]` (enumerated): `record_outcome(&health, &outcome, 100 * at as u64);`. This is the sequence of the existing test `telemetry.rs:288-312`; it emits one `pipeline_recovered`.
       - **Step 5, launch.** `launch_not_started_for_test(Uuid::new_v4(), "verify_failed");` (`verify_failed` is the real code at `worker.rs:158`).
    3. After the closure: `let records: Vec<LauncherRecord> = rx.try_iter().collect();`. Assert, with a message naming what is missing, that for each pair below some record has that `target` and `fields["event"]` equal to that name:
       `("client.launcher.startup", "launcher_started")`, `("client.launcher.ui", "install_failed")`, `("client.launcher.settings", "client_options_saved")`, `("client.launcher.telemetry", "pipeline_recovered")`, `("client.launcher.launch", "launch_not_started")`. Other rows (for example `game_diagnostics_changed` from the opt-in) may be present; do not assert a total count.
    4. Assert no string value in any record's `fields` satisfies `contains_ip` (the exact LT-40 rule).
    5. `let (_, events) = host.launcher_batch(&records, &Redaction::from_env()).expect("opted in with the module");` and assert `events.len() == records.len()`.
    6. For each event: `println!("LAUNCHER_TELEMETRY_UAT {}", serde_json::to_string(&event).unwrap());`, then flush stdout once. Each line is a serialized `TelemetryEvent`: `{"type":"client_native","ts_ms":..,"seq":..,"target":..,"level":..,"fields":{..}}`.
- **Change 3, the `.mjs`** (plain Node ESM, `node:` built-ins only; style of `game-telemetry-native-uat.mjs`):
  - `const binary = process.env.LAUNCHER_TELEMETRY_UAT_BINARY`; if it is unset, print `FAIL LAUNCHER_TELEMETRY_UAT_BINARY is not set` and exit 1.
  - Run it with `spawn(binary, ['--ignored', '--exact', 'host::launcher_telemetry_uat::uat_bridge_launcher_telemetry', '--nocapture'], {stdio: ['ignore', 'pipe', 'inherit']})`. Read stdout lines with `node:readline`; keep the lines that start with `'LAUNCHER_TELEMETRY_UAT '` and `JSON.parse` the rest of each line. Wait for exit; a non-zero exit code is a FAIL.
  - Load the allowed targets from `new URL('../../../launcher-core/src/telemetry/launcher_targets.txt', import.meta.url)`: trimmed lines, skipping empty lines and lines starting with `#`.
  - Checks, each printed as `PASS <name>` or `FAIL <name>: <detail>`:
    1. at least one row was read;
    2. every row has `type === 'client_native'` and a `target` in the allowed list;
    3. every row has at most 16 keys in `fields`;
    4. no string in `fields` (walk arrays and objects too) matches `/[A-Za-z]:\\Users\\/i`, contains `://`, or holds an IP by `hasIp` below;
    5. the five `(target, fields.event)` pairs from change 2, step 3, are all present.
  - `hasIp(s)`, a port of the LT-40 token rule (`engine/src/storage/game_telemetry/ip_scrub.rs:12`; the Rust test already applies the exact rule): split `s` on `/[^0-9A-Fa-f:.\[\]%]+/`; for each non-empty token `t`, try `t` and, when `t` matches `/^(.*):\d+$/`, also its first group; for each try, remove a leading `[` and a trailing `]`, cut at the first `%`, and return true when `net.isIP(<that>) !== 0` (`import net from 'node:net'`).
  - End with `Launcher telemetry logic UAT passed: <n> rows, every target allowed, at most 16 fields, no home path, URL or IP, the five events present.` when every check passed, otherwise `process.exitCode = 1`. Then print `Excluded: the forwarder thread and LauncherSender, session mint, any send, SigNoz, and the launch worker's own call sites (pinned by events_tests.rs and LT-18).`
- **Tests (R7).** The UAT is the test: removing any of the five emitters, or the seam, fails check 5 and the Rust asserts. No new unit test is needed: the emitters are already pinned (`startup_event.rs:123`, `ui_event.rs` tests, `commands.rs:139`, `telemetry.rs:288`, `launch/events_tests.rs:16`).
- **Checks**, in this order, each prefixed with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lt-25;`:
  1. D1: `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all`;
  2. D2e: `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings`;
  3. D2s: the same with `-p cimmeria-launcher-desktop`;
  4. `$env:LANE_VERBOSE='1'; pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --no-run`, and take the path from its `Executable unittests src/main.rs (<path>)` line;
  5. `$env:LAUNCHER_TELEMETRY_UAT_BINARY='<path>'; npm run uat:launcher-telemetry --prefix crates/launcher/desktop/frontend`: every line PASS and exit 0. Paste its output into the report;
  6. F1: `npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend`, then `npm run check --prefix crates/launcher/desktop/frontend`, then `npm test --prefix crates/launcher/desktop/frontend`.
- **Commit:** `LT-25: REPL logic UAT for launcher telemetry`.

### LT-26: launcher and operations docs

- **Agent:** documentation-writer. **Worktree:** `lt-26`, from `lt-app-integration` after LT-25 **and** after ops has merged `origin/main` into it (PR S changed `telemetry.md`). **Rules:** R2, R4, R9.
- **Files:**
  - `crates/launcher/desktop/docs/`: the AU launcher-log or telemetry doc (grep `launcher.<date>.log`), `launcher-summaries.md` (§ "What a rollout packet must change", which is now done), `launch.md` (the events);
  - `docs/architecture/launcher-summary-telemetry.md` ("still open" changes to the D-LT4 outcome: https only, from an https login server or the built-in default);
  - `docs/operations/telemetry.md` (launcher summaries ~641-718, player opt-in ~720-757, the privacy section ~759-778, and the "No launcher rows in SigNoz" runbook from LT-04 extended to the launcher side: the status line, `pipeline_recovered`, `game_session_mint_failed`);
  - `docs/architecture/observability.md` (the event table per target).
- **Write:**
  - The C5 table, the C6 vocabulary, D-LT2's consent semantics as built, and the privacy rule R5 with D-LT18 (no IP anywhere, the `[ip]` scrub, no hashing).
  - The https rule (directive 5): summaries and the default upload URL are https only. Add the "not sent yet" state of setup diagnostics, and say that it ends with LT-41 (D-LT17).
  - Walk `docs/agents/doc-update-map.md` and list the rows.
  - Never hand-edit a `gen:` block; run `python tools/docs-gen/regen.py` if one is affected.
  - CRLF.
- **Commit:** `LT-26: document launcher telemetry`.

### LT-41: built-in https telemetry URL (PR H; D-LT17 resolved, amendment 4)

- **Status:** BlockedDependency (PR S and PR L merged, then the probe below). **D-LT17 is resolved by directive 7:** the origin is `https://play.cimmeria.app`, served by the board's Caddy proxy (`board-proxy`), which forwards only the five telemetry routes (dev-session mint and refresh, upload-chunk, upload-bundle, launcher-summary) to loopback :8081 and answers 404 for everything else. The earlier blocker (no https endpoint on record: `container.md:58`, the never-deployed Cloudflare Tunnel in `telemetry.md:145-202`) is gone; LT-04 rewrites those docs.
- **Unblock check (`ops`, right before dispatch).** A plain HTTP status-code probe from the workstation. It needs no lab, no ssh, no SigNoz and no credentials, and it mints nothing because it sends no body:
  - `curl.exe -sS -o NUL -w "%{http_code}" -X POST https://play.cimmeria.app/api/auth/dev-session` answers with an HTTP status (typically 400 or 415, or 503 before an endpoint is configured), not a TLS or connect error;
  - the same `-X POST` to `https://play.cimmeria.app/api/telemetry/launcher-summary`;
  - `curl.exe -sS -o NUL -w "%{http_code}" https://play.cimmeria.app/` answers 404, which shows that only the routes are forwarded.
  The `ops` report records the three status codes only. If any probe fails with a TLS or connect error, LT-41 goes back to BlockedDecision and the orchestrator tells the owner. `play.cimmeria.app` is a public DNS name, so it may be committed; an IP literal may not.
- **Host:** windows. **Worktree:** `lt-41`, branch `launcher-telemetry/lt-41-https-default`, from `origin/main` after PR S and PR L have merged. **Rules:** R1-R10. **Contract:** C10, C11, LT-22's `DEFAULT_SUMMARY_BASE`.
- **Files:**
  - `crates/admin-api/src/routes/dev_session/upload_endpoint.rs` and its tests;
  - engine `storage/game_telemetry/endpoint_sanity.rs` and engine `storage/launcher_summary/endpoint.rs`, with their tests.
- **Change:** three constants, each set to the verified origin. Nothing else changes:
  - `DEFAULT_UPLOAD_ENDPOINT = Some("https://play.cimmeria.app/api/telemetry")`, in both admin-api and the engine;
  - `DEFAULT_SUMMARY_BASE = Some("https://play.cimmeria.app/api/")`.
- **Tests:**
  - Flip the `None`-default cases: `resolve` with no env and no https Host now gives `source=default`, and `boot_warning(None)` is `None`.
  - (`http://play.example:8081`, loopback advertised) gives the default with `overridden=true`.
  - `SummaryEndpoint::resolve(Some("http://play.cimmeria.app:8081"))` gives source `default`.
  - The `None`-or-`https://` guards stay and pass.
  - Reverted, each flipped case fails.
- **Checks:** S1, S2, S3 `routes::dev_session`, S4, D1, D2e, D3e `game_telemetry`, D3e `launcher_summary`.
- **Docs:** `docs/operations/telemetry.md`: LT-04 already describes the Caddy route; LT-41 changes only the lines that say the built-in default is `None`, to name `https://play.cimmeria.app`, and says the env var is an optional override (CRLF).
- **Commit:** `LT-41: built-in https telemetry and summary default`.
- Amended by launcher-linux-install-telemetry LI-06: an http login server on a DNS name now resolves to `login_server_https_upgrade` before the default, so LT-41's tests must use an IP or single-label http server to reach `default`.
- Amended by launcher-linux-install-telemetry LI-09: `session::upload_for` upgrades an http login server on a DNS name before `effective_upload_endpoint`, so LT-41's tests reach `DEFAULT_UPLOAD_ENDPOINT` only from an IP or single-label http server.

### LT-27: close-out (PR X)

- **Agent:** documentation-writer. **Worktree:** `lt-close`, from `origin/main` after PR L, PR A, PR I, PR F and PR H have merged. If LT-41 has not merged (its probe failed), the close-out lists LT-41 and U1/U5 as owed.
- **Files:** `docs/project-status.md` and `docs/gap-analysis*` (once), `docs/guides/unified-uat.md`, and this ledger.
- **The owner UAT steps:**
  - U1 (was OQ-LT5; after LT-41 and the colo deploy): there is no `upload_endpoint_unset`, `_loopback` or `_plain_http` boot WARN; the boot INFO shows `upload_endpoint_source=default` (or `env` with an https URL); and a mint from a remote launcher shows `upload_endpoint_https=true`. Never record the address;
  - U9 (directive 6): in SigNoz over a day of colo traffic, no `cimmeria-server` row on `launcher.ingest`, `dev_session` or `admin.request` with a telemetry path, and no `cimmeria-client` launcher row, has a `peer` attribute or an IP literal. Query: `has(attributes, 'peer')` plus a regex on the body; record counts only;
  - U2: a release launcher with Send game diagnostics on shows `launcher_started` in the saved view within a minute;
  - U3: toggling each checkbox updates the status line on the first press;
  - U4: an install, an update check and a Play each show their start and outcome events;
  - U5 (after LT-41): with Share setup diagnostics ticked on a fresh install (no server list edits), a summary row appears under `launcher.summary`, sent over https. Before LT-41, the box shows the "not sent yet" line instead;
  - U6: an adopt of a folder with one modified managed file shows the check, the file and expected vs found on screen, and the same `adoption_rejected` row (or review difference) in the saved view;
  - U7 (after LT-37): an adopt on the other laptop's install reaches the review without downloading the seed (`adoption_index_published` info, no `adoption_index_fallback`);
  - U8: with the file index blob unreachable (block the release host for the launcher), the review names `index_download_failed` and the log and saved view show `adoption_index_fallback`;
  - U10 (amendment 4, after PR S and the colo deploy): with two launchers on different networks minting through `https://play.cimmeria.app`, SigNoz over a day shows no `dev_session_quota_exceeded` with `scope=mint/ip` and no summary 429 at normal load. If it does, the server sees the proxy as a non-loopback address (D-LT20). Record counts only.
- Then `ops` retires every `lt-*` worktree (`rm-worktree.ps1 --merged`, and `--merged-into <PR>` for the packet worktrees), empties `.claude/state/active-campaign`, and posts a board handoff.
- **Commit:** `LT-27: launcher-telemetry close-out`.
