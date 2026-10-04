# Desktop launcher foundation

> **Type:** Reference
> **Audience:** Launcher contributors
> **Last updated:** 2026-10-04
> **Companions:** [Implementation plan](../../../docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-plan.md), [build rules](../../../CLAUDE.md), [test policy](../../../TESTING.md)

This standalone workspace contains native Rust state, Effect workflows and a
Tauri settings shell. The interface connects through Tauri invoke to native
preference persistence. Play/Patch Notes tabs and the settings panel are
implemented; patch notes load from a signed release manifest and game actions
remain disabled. Installation, repair, removal, launch and telemetry export
are not implemented here. The existing Windows egui launcher is unchanged.

## Native operation and storage contracts

`engine/src/operations.rs` owns a versioned snapshot with a revision and one
current operation. Each operation carries an ID, kind, intent digest and state.

- Matching retries of the current operation return its snapshot without
  admitting work again. Reusing its ID with another kind or digest fails.
  Other IDs require the current revision. Transport retries retain the original
  request revision; this is not a history of every previously used ID.
- Cancellation records `cancel_requested`; only native observation establishes
  a terminal outcome. Actual completion may win a cancellation race.
- Restoring interrupted work records `reconciliation_required`, without resuming
  it. An authoritative filesystem/process inspector must resolve ownership.
- Changes commit before replacing the published snapshot. Failures retain the
  last confirmed in-memory state. If replacement may already have occurred,
  `persistence_uncertain` blocks further mutations until reopening.

`engine/src/storage/` implements `FileJournal` and `DesktopState`. The latter
holds an exclusive OS lock on `launcher.lock` while accessing `operation.json`
and `preferences.json`. JSON reads are limited to 64 KiB; malformed, oversized
or unsupported state fails open attempts without resetting files. Checked
state-file paths reject symlinks and nonregular files. The app-data directory
must be native-selected and private to the launcher; these checks are not an
adversarial filesystem sandbox.

Writes use a same-directory temporary file, sync its contents, replace the
destination, and sync the resulting file. Unix also syncs the parent directory.
A failure after replacement reports uncertainty: reopen and inspect before
issuing more mutations. Reads expose the last confirmed in-memory snapshot, so
adapters must also surface `requires_reopen`. Windows-native engine checks have passed; power-loss durability
remains unvalidated; OS sync calls alone do not prove hardware crash behavior.

Preferences persist an optional absolute install path and separate default-off
`launcher_summary_consent`. Saves require the current preference revision.
An active operation blocks path changes but permits consent changes. The path
check does not establish installation ownership or safe deletion. This flag is
for future launcher summaries only; there is no exporter or game/DLL consent
integration yet, and no immediate-export revocation claim is made by this packet.

`engine/src/commands.rs` exposes versioned `inspect` and `save_preferences`
commands. No command lets frontend callers set native operation outcomes or
choose the state-directory root. The shell selects app data through Tauri's native resolver. Canonical game intent
validation, digest calculation and worker integration remain pending.

## Effect workflows

`frontend/` pins Effect **4.0.0** and uses its actual services, scopes, semaphore,
Ref, Schema, scheduling, test clock and bounded PubSub. The version was checked
against the npm stable tag on 2026-10-04. API references:
[scopes](https://effect.website/docs/v4/resource-management/scope) and
[scheduling](https://effect.website/docs/v4/scheduling/using-schedules).

`bridgeLayer` accepts a transport: the shell supplies Tauri invoke and logic
UAT supplies the native process harness. Replies are schema-validated, native
failures are allowlisted codes, and raw transport errors are discarded.
`makeLauncher` lives in the application scope. It serializes inspection/saves,
rejects older snapshots, and publishes a one-entry sliding state stream so slow
views cannot build an unbounded progress backlog.

Read-only inspection has at most two retries, with 100 ms exponential backoff.
Each settings IPC call has a five-second observation timeout. Saves are sent **once**;
timeout, interruption or failure leaves inspection required before another
mutation. A cancelled Effect fiber does not cancel a native save. Scope cleanup
releases subscriptions; it does not claim native rollback. Uncertain native
storage keeps the mutation gate closed until the native store is reopened.

## Tauri settings shell

`shell/` selects `<app-data>/state` with identifier
`app.cimmeria.launcher.desktop` and lazily opens one `DesktopState` behind a
mutex. Storage commands run on blocking workers. The native folder chooser is
parented to the requesting window; cancelling it makes no save. Show folder
reveals the saved directory in the file manager, rather than opening it through
file associations. It accepts no frontend path and requires an existing folder.

`frontend/src/view.ts` owns one application-scoped `ManagedRuntime`. Tabs and
settings preserve that runtime. Consent changes show pending feedback, wait for
native acknowledgement and restore confirmed state on failure. Folder selection
saves through the same Effect workflow and preserves consent. Disposal removes
handlers and interrupts frontend observation without claiming native rollback.

The development UI explicitly labels unavailable game operations. Verified
patch notes describe available release patches, not installed-game status.
Native window appearance, dialogs and actual Tauri IPC still require interactive
UAT. Compilation and headless DOM tests do not establish those behaviors.

## Verified release patch notes

`engine/src/catalog/mod.rs` fetches the native-owned `content-current` manifest
and detached signature over HTTPS. Limits: 1 MiB manifest, 256-byte signature,
five-second connection timeout, fifteen seconds per request including body
consumption, and five redirects. Size limits cover declared and streamed bodies.

A Rust path module shares `crates/launcher/src/manifest.rs` with the Windows
launcher; the catalog uses its schema, signature verification and validation,
but replaces its unbounded fetcher. Signature verification precedes JSON parsing.
Notes-only IPC exposes IDs, titles and descriptions in manifest order, with the
ID as fallback for a missing or blank title. Errors contain safe codes only.
Changes to the shared source also trigger the desktop CI workflow.

`LAUNCHER_MANIFEST_PUBKEY_HEX` supplies the existing build-time trust key.
Release builds without a usable key fail closed. Test/debug builds can use the
existing development fallback; it cannot authenticate the live release. Supply
the maintainer-approved public key when building a launcher for live content.
Do not discover or trust a key from the manifest being verified.

The Patch Notes tab starts an independent read-only Effect fetch on first open.
Refresh explicitly fetches again; no automatic retries or overlapping UI loads.
Frontend observation times out after 35 seconds. Disposal interrupts observation,
not native network work, which remains bounded by its own deadlines. Successful
notes stay in session memory; a failed refresh retains them with an explicit
previously-verified label. All content renders as literal DOM text, never HTML.

A read-only live check uses the same native fetcher:

```bash
# Set LAUNCHER_MANIFEST_PUBKEY_HEX to the approved release public key first.
bash tools/build-lane/lane.sh cargo run --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --example catalog_probe --target-dir target/desktop
```

## Validation

Run from the repository root. Windows checks run natively on Windows; the
Mac launcher work runs natively on macOS. All compilation uses the pinned
Rust toolchain and repository build lane:

```bash
bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml
bash tools/build-lane/lane.sh cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml --all-targets -- -D warnings
cargo fmt --all --manifest-path crates/launcher/desktop/Cargo.toml -- --check
npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend
npm run check --prefix crates/launcher/desktop/frontend
npm test --prefix crates/launcher/desktop/frontend
bash tools/build-lane/lane.sh cargo build --locked --manifest-path crates/launcher/desktop/Cargo.toml --example state_bridge --target-dir target/desktop
npm run uat --prefix crates/launcher/desktop/frontend -- "$PWD/target/desktop/debug/examples/state_bridge"
```

Build the development executable without opening it:

```bash
bash crates/launcher/desktop/build-native.sh
bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --target-dir target/desktop
bash tools/build-lane/lane.sh cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --target-dir target/desktop --all-targets -- -D warnings
```

The executable is `target/desktop/debug/cimmeria-launcher-desktop` (`.exe` on
Windows). For a development Mac `.app`, after building the UI and installing
the Tauri CLI, use the lane for its nested Cargo build:

```bash
bash tools/build-lane/lane.sh bash crates/launcher/desktop/bundle-macos-dev.sh
```

The bundle is under `target/desktop/debug/bundle/macos/`. Creating this bundle
is not final self-contained startup validation. Do not open it during unattended
work without the tester's explicit desktop permission.

On Windows, append `.exe` to the harness path. Root workspace tests do not run
this nested workspace. `.github/workflows/launcher-desktop.yml` adds explicit
native Mac/Windows checks and the frontend/native logic UAT.

On 2026-10-04, **52 engine tests**, **two shell-host tests**, **14 frontend tests**, strict clippy, TypeScript
checking and formatting passed locally on macOS. The one ignored Rust test is a
subprocess fixture invoked by its parent test. Coverage includes command/schema
validation, ownership/retries, cancellation races, file failures before/after
replacement, preference persistence, stale revisions, corrupt/future state,
symlink rejection and bounded reads. A child process holds the lock while
writing; its parent verifies exclusion, kills it and reopens interrupted state.
That interrupts an idle child after completed writes, not a write in progress
or a power failure. Effect tests use virtual time for retry/timeout behavior.

The frontend suite includes eight workflow tests and six DOM tests covering
navigation, disabled game actions, pending/failed consent saves, folder choice
and cancellation, and disposal during pending IPC. Two shell-host tests cover
lazy ownership, saved-folder resolution and retry after another owner releases
the lock. Native shell compilation, its two host tests and clippy passed locally
on macOS. No desktop window was opened for those checks.

JS logic UAT mounts the actual HTML and view code in a headless DOM. It drives a
checkbox through Effect, the Rust process harness and disk, then restarts the
process and confirms persistence. It also checks restored settings and a mocked
cancelled chooser without an extra save. Earlier workflow steps cover default-off
consent, path persistence and persisted opt-out. The patch tab renders a notes
response fixture as literal text while consent remains unchanged. Signature
verification is covered separately in Rust, not by this DOM fixture. This does not exercise native
webview rendering, Tauri command routing, real dialogs or file-manager reveal,
keyboard behavior, gameplay or telemetry export.

[CI run 37181383914](https://github.com/SandboxServers/Cimmeria/actions/runs/37181383914)
passed the Windows and macOS engine/frontend persistence checks at `69fc13d3f`,
before the shell was added. CI now includes shell tests, clippy and executable
builds. Run `37182711152` passed both native platforms at `aaf00cb9e`, after
the Windows icon correction and before the catalog addition.

## Next integration gates

Verify the native window, keyboard behavior, actual Tauri IPC, folder chooser
and saved-folder reveal interactively. Extract existing installation/preparation
behind validated native intents, prove worker dispatch follows persistence,
and implement authoritative reconciliation, cancellation and progress through
Effect. All game actions remain unimplemented. Platform
provisioning, telemetry, migration/updater and final self-contained startup and
release gates remain open.

The shell includes PNG and Windows ICO resources. Tauri compiles the ICO into
the Windows executable even for shell tests; native Windows CI is the check for
that resource step. The ICO is copied from the existing Windows launcher.

Catalog tests cover signed fixture decoding, order/title fallback, tampered and
malformed signatures, invalid JSON/schema, size bounds, and loopback HTTP status
and chunked-body handling. The live `catalog_probe` succeeded on 2026-10-04 with
seven patches using the release public key recorded in the bring-up handoff.
This verifies a current response, not future availability or packaged UI routing.
The local fixture suite uses the development key; run it without a release-key
override. Real Tauri catalog IPC and visual UAT remain unverified.
