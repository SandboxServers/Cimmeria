# Desktop launcher foundation

> **Type:** Reference
> **Audience:** Launcher contributors
> **Last updated:** 2026-10-04
> **Companions:** [Implementation plan](../../../docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-plan.md), [build rules](../../../CLAUDE.md), [test policy](../../../TESTING.md)

This standalone workspace contains the native Rust operation owner and the
Effect workflow foundation for the approved Tauri launcher. Native settings are
persisted and exercised by a headless bridge. The Tauri window is not connected
here yet. No game installation, repair, removal, launch or telemetry export is
implemented in this workspace. The existing Windows egui launcher is unchanged.

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
adapters must also surface `requires_reopen`. Windows and power-loss durability
remain unvalidated; OS sync calls alone do not prove hardware crash behavior.

Preferences persist an optional absolute install path and separate default-off
`launcher_summary_consent`. Saves require the current preference revision.
An active operation blocks path changes but permits consent changes. The path
check does not establish installation ownership or safe deletion. This flag is
for future launcher summaries only; there is no exporter or game/DLL consent
integration yet, and no immediate-export revocation claim is made by this packet.

`engine/src/commands.rs` exposes versioned `inspect` and `save_preferences`
commands. No command lets frontend callers set native operation outcomes or
choose the state-directory root. Canonical game intent validation, digest
calculation, app-data selection and worker integration remain pending.

## Effect workflows

`frontend/` pins Effect **4.0.0** and uses its actual services, scopes, semaphore,
Ref, Schema, scheduling, test clock and bounded PubSub. The version was checked
against the npm stable tag on 2026-10-04. API references:
[scopes](https://effect.website/docs/v4/resource-management/scope) and
[scheduling](https://effect.website/docs/v4/scheduling/using-schedules).

`bridgeLayer` accepts a transport; production will supply Tauri invoke and the
logic UAT supplies the native harness. Replies are schema-validated, native
failures are allowlisted codes, and raw transport errors are discarded.
`makeLauncher` lives in the application scope. It serializes inspection/saves,
rejects older snapshots, and publishes a one-entry sliding state stream so slow
views cannot build an unbounded progress backlog.

Read-only inspection has at most two retries, with 100 ms exponential backoff.
Each IPC call has a five-second observation timeout. Saves are sent **once**;
timeout, interruption or failure leaves inspection required before another
mutation. A cancelled Effect fiber does not cancel a native save. Scope cleanup
releases subscriptions; it does not claim native rollback. Uncertain native
storage keeps the mutation gate closed until the native store is reopened.

## Validation

Run from the repository root. Windows checks run natively on Windows; the
Mac launcher work runs natively on macOS. All compilation uses the pinned
Rust toolchain and repository build lane:

```bash
bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml
bash tools/build-lane/lane.sh cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml -- --check
npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend
npm run check --prefix crates/launcher/desktop/frontend
npm test --prefix crates/launcher/desktop/frontend
bash tools/build-lane/lane.sh cargo build --locked --manifest-path crates/launcher/desktop/Cargo.toml --example state_bridge --target-dir target/desktop
npm run uat --prefix crates/launcher/desktop/frontend -- "$PWD/target/desktop/debug/examples/state_bridge"
```

On Windows, append `.exe` to the harness path. Root workspace tests do not run
this nested workspace. `.github/workflows/launcher-desktop.yml` adds explicit
native Mac/Windows checks and the frontend/native logic UAT.

On 2026-10-04, **23 Rust tests**, **eight Effect tests**, strict clippy, TypeScript
checking and formatting passed locally on macOS. The one ignored Rust test is a
subprocess fixture invoked by its parent test. Coverage includes command/schema
validation, ownership/retries, cancellation races, file failures before/after
replacement, preference persistence, stale revisions, corrupt/future state,
symlink rejection and bounded reads. A child process holds the lock while
writing; its parent verifies exclusion, kills it and reopens interrupted state.
That interrupts an idle child after completed writes, not a write in progress
or a power failure. Effect tests use virtual time for retry/timeout behavior.

The JS logic UAT exercises the actual Effect service, Rust command handler and
filesystem: first-open consent off; saved consent/path; process restart with
preserved values; opt-out; a second restart proving opt-out persisted. Tests also
cover a lost save reply without replay, save failure, old snapshots and scoped
subscription interruption. No browser, native webview, keyboard/visual layout,
real game or live telemetry was exercised. CI outcomes must be checked before
claiming the new Windows-native coverage passed.

## Next integration gates

Connect the Tauri shell with native app-data ownership and settings UI, then
extract existing installation/preparation behind validated native intents.
Prove actual worker dispatch follows persistence, implement authoritative
reconciliation, and deliver explicit operation cancellation and progress through
Effect. The original plan's install/launch, platform provisioning, telemetry,
migration/updater and final self-contained startup/release gates remain open.
