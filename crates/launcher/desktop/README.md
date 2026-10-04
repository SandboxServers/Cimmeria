# Desktop launcher foundation

> **Type:** Reference
> **Audience:** Launcher contributors
> **Last updated:** 2026-10-04
> **Companions:** [Implementation plan](../../../docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-plan.md), [build rules](../../../CLAUDE.md), [test policy](../../../TESTING.md)

This standalone workspace contains native Rust state, Effect workflows and a
Tauri settings shell. The interface connects through Tauri invoke to native
preference persistence. Play/Patch Notes tabs and the settings panel are
implemented; patch notes load from a signed release manifest and game actions
remain disabled. The installer algorithms are shared and fixture-tested, but installation is not
connected to the UI. Repair, removal, launch and telemetry export remain pending. The existing Windows egui launcher retains its UI and shares the downloader cancellation fixes.

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
choose the state-directory root. The shell selects app data through Tauri's native resolver. Native first-install intent validation and worker dispatch are implemented below;
frontend operation commands remain pending.

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

## Shared installer core

The engine imports the existing launcher installation, archive, layout, state,
patch destination, report and client-preparation modules through Rust path
modules. It depends on the existing `cimmeria-patchset` crate for delta patches
and UPK normalization. Algorithms and their regression tests have one source;
the standalone lockfile resolves their dependencies independently. This includes
the existing workspace-hack dependency transitively; the desktop workspace is
still excluded from root aggregate checks.

`install_progress::ProgressSink::latest()` retains one progress value through a
Tokio watch channel. A stalled or disconnected observer cannot build a backlog
or fail installation. The existing egui worker uses the legacy adapter and
preserves its event stream. Progress is observational, not an operation journal
or proof that the game is ready.

The loopback installer test downloads a synthetic PE-in-ZIP seed, verifies its
hash, extracts it, overlays a patch, writes the login file, disables ASLR and
persists installed state. Its second pass makes no further blob requests, while
a stalled watch receiver retains the newest progress. This exercises the real
pipeline, not simulated progress. It does not download or launch SGW.

The original archive's spanning cabinet set still requires Windows FDI. Native
Mac ZIP/RAR tests do not prove Wine cabinet extraction. The next integration
needs to invoke the Windows helper, establish managed runtime/prefix ownership, validated
operation intents, reconciliation, cancellation and terminal readiness checks.
The legacy pipeline's success can occur without SGW.exe; a new readiness adapter
must not equate it with Play-ready. Existing install state is not an ownership
marker permitting uninstall, and its permissive reads are not recovery proof.
See [runtime provisioning evidence](../../../docs/analysis/playtests/2026-10-03-macos-wine/runtime-provisioning.md).

## Archive worker boundary

`engine/src/archive_worker/` defines a one-request extraction contract and
portable stdio mechanics. `cimmeria-archive-worker` is operational only when
built natively on Windows; non-Windows entry points exit with an error. The
desktop coordinator does not yet invoke it and Wine execution is unverified.

Input is NDJSON, bounded to 8,192 bytes per frame including the newline. An
extraction request carries `schema_version: 1`, `operation_id` (UUID), absolute
`archive` and `destination` paths, and `sha256`. The native parent supplies the
hash from authenticated content and owns the archive's directory. The helper
verifies the hash before creating a destination that must not already exist.
It never overlays an existing installation. On Windows, the verification handle
denies write/delete sharing and stays open while the extractor reopens the path.
This guards Windows file mutation/replacement; enforcement against native-host
writes under Wine remains unverified. It is not an adversarial filesystem sandbox.

Stdin remains open as the ownership channel. A control frame with the same
schema/operation ID and `cancel: true` requests cooperative cancellation. EOF
or malformed controls also cancel; valid frames for a different ID are ignored.
Cancellation checkpoints can be coarse (between ZIP entries, patchset operations
or archive stages). Completion can win a late cancellation; no immediate abort
or rollback is promised. Partial output stays for parent reconciliation.

Progress is latest-value only, emitted at most every 100 ms into a one-slot
stdout queue. Replies carry schema/version, operation ID and either progress
counts or a `finished` error code; no filenames or raw errors are emitted.
After the blocking extractor returns, terminal enqueue/flush has a two-second
budget. Lost/blocked output yields a nonzero exit; it does not prove no files
were written. The executable exits after the result, including when a detached
stdio thread remains blocked. The parent must drain stdout, keep stdin open,
impose startup/operation deadlines, own staging and reconcile uncertain exits.

The helper uses the existing Windows FDI chain implementation. A Windows process
test checks real stdio, Unicode/space paths, operation identity, exactly one
terminal result and hash failure before output. A Windows sharing test guards
write/rename denial. Portable tests exercise the actual control reader and
injected blocked/broken writers. They do not establish Wine or real-cabinet
compatibility. Native Windows CI retains a debug helper artifact for seven days
for supervised validation; this is not a published release.

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
native Mac/Windows checks and the frontend/native logic UAT. Shared-source
changes also run an existing-launcher Cargo check on native Windows.

On 2026-10-04, **172 engine tests**, **two shell-host tests**, **14 frontend tests**, strict clippy, TypeScript
checking and formatting passed locally on macOS. Three engine tests are ignored by default: the subprocess fixture invoked by
its parent, plus manual real-SGW-executable and real-client-RAR checks that
remain unrun. Coverage includes command/schema
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
Effect. Game actions remain disconnected from the frontend. Platform
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

Catalog CI run `37183173769` passed both native platforms at `f8ea7b844`.
The shared-installer/helper packets need their own native Windows results.
Helper stdio mechanics are compiled and tested on Mac without running the
Windows executable. Shell-host tests were last run in the preceding packet.


## Native helper supervision

`engine/src/helper_supervisor/` starts a native-selected executable with explicit
arguments, working directory and environment; inherited environment variables
are cleared. No deserialized or webview command accepts this configuration.
The caller must persist operation admission first. After spawning, `record_host`
must durably record the host PID before request dispatch, or refuse dispatch.
Production journal integration for this callback remains pending.

Protocol frames are limited to 8 KiB and event/progress channels retain one
observation. Default deadlines are five seconds for request writes, thirty
minutes for the operation, thirty seconds for cooperative cancellation and three
seconds for terminal/exit completion. Cancellation writes share the remaining
operation/cancellation deadline; cleanup cannot grant another cancellation
budget. Synchronous spawning and the persistence callback are not time-bounded.

Success requires a matching terminal event, successful process exit and stdout
EOF. Silence, malformed/duplicate events, identity mismatch, contradictory exit
and deadlines require reconciliation. These outcomes describe extraction, not
installation or game readiness. Cleanup targets only the owned child; stopping
a Wine host does not establish that its guest processes have exited. The native
operation scope must outlive the webview and retain recovery state on uncertainty.

A custom real-stdio harness exercises twelve scenarios, including failed
ownership before dispatch, cancellation, an unresponsive child, progress flooding
and supervisor abort. The abort case verifies direct-child OS-lock release.
All twelve scenarios passed locally, alongside 142 engine tests and strict
all-target clippy. A separate blocked-pipe regression checks cancellation-write deadlines. A
Windows-only integration test connects the supervisor to the actual archive
worker; its native CI result remains pending. No GUI or Wine execution was
performed. This packet changes no frontend behavior, so frontend JS REPL/visual
UAT does not apply. Installation remains disabled until coordinator integration.


## Durable install admission

`catalog::VerifiedRelease` retains the validated manifest and SHA-256 of its
original signed bytes. Native callers obtain it through signature/schema
verification; a frontend request cannot deserialize one.

`DesktopState::admit_install` records immutable inputs in
`install-intent-<operation-id>.json`: operation ID, original preference revision,
canonical destination, verified manifest digest and native-supplied login servers.
It writes this record before admitting the operation with the intent's digest.
`dispatch: true` is returned only after both writes succeed. Admission itself
dispatches no worker and creates no game files. Separate filenames preserve the
previous operation's evidence if a subsequent admission fails between writes.
Orphan/history cleanup remains part of coordinator recovery work.

First-install admission requires a saved absolute destination whose parent
already exists. It canonicalizes that parent and accepts a missing final
directory or an existing empty directory. Files, final-component symlinks,
nonempty directories and paths overlapping launcher state are rejected. Adoption
and repair require separate ownership evidence. These are admission-time checks,
not a filesystem reservation; the worker must acquire ownership and recheck.

A retry retains its saved intent and returns `dispatch: false`. It must retain
the original preference revision, verified release identity and login servers.
Changing diagnostics consent afterward does not invalidate the original retry.
Conflicting inputs fail without rewriting current recovery evidence. On restart,
interrupted operations require reconciliation; an identical retry never replays
installation. Reading recovery intent validates schema, operation ID and digest.
Missing, corrupt or mismatched evidence is not permission to run. An orphan
intent is not independently dispatchable.

Fixtures cover admission/restart without destination mutation, consent-change
retries, release conflicts, busy/stale requests, missing/corrupt/digest-mismatched
intent, failed writes, failed replacement admission, empty-directory acceptance
and unsafe destination rejection (symlinks on Unix). This establishes neither
installation readiness nor UI install behavior. No frontend behavior changed,
so JS REPL/visual UAT does not apply to this packet. The subsequent worker
section records dispatch implementation; runtime readiness, reconciliation and
UI installation remain pending.

Admission validation: 152 engine tests passed on macOS through the build lane,
plus the unchanged twelve-scenario process harness; strict all-target clippy
and formatting passed. Windows admission validation awaits native CI.


## Native first-install worker

`engine/src/storage/install_worker/` dispatches an admitted intent only after
matching its operation ID and verified release digest. It commits `running`
before claiming the destination; a second dispatch is rejected. The task retains
native state independently of observers. Dropping a view or progress receiver
does not cancel installation. Explicit cancellation commits `cancel_requested`
before signalling the worker.

The worker rechecks the canonical first-install destination, creates an exclusive
`.cimmeria-install.json` marker and holds its OS lock. It runs the shared pipeline
inside `.cimmeria-stage-<operation-id>`. Before promotion it reads the bounded
installation ledger, checks expected seed and patch entries, and requires a
nonempty regular SGW executable and an `SGWGame` directory. These are content
checks, not a complete extracted-file inventory, executable compatibility or
gameplay validation. Filesystem ownership is cooperative, not a hostile-user sandbox.

Content moves to `<selected-directory>/game`. The worker persists
`content-ready.json` before committing operation success and publishing
`ContentPrepared`. Receipt or terminal-state persistence failures require
reconciliation; visible files alone are not reported as confirmed success.
Power-loss durability of the complete extracted tree remains unvalidated.

Failure and cancellation retain the marker and partial output. Interrupted native content can be inspected conservatively as described below;
automatic retry, cleanup and adoption remain unimplemented. The Wine cabinet-helper
adapter, runtime provisioning and frontend dispatch remain pending. Installation
stays disabled in the UI; content preparation does not establish launch readiness.

Fixtures cover actual HTTP/ZIP staging and promotion, observer disposal,
duplicate dispatch, missing-executable rejection, cancellation while waiting for
headers and at an extraction checkpoint, changed destinations/redirected parents,
and receipt failure after promotion. The shared downloader now interrupts waits
for HTTP headers and body chunks; extraction cancellation remains `Cancelled`
through the install pipeline instead of becoming a generic patch failure.
No frontend behavior changed, so JS REPL/visual UAT is not applicable here.

Worker validation: 160 engine tests and the twelve-scenario process harness
passed locally on macOS. Strict all-target engine clippy and root/desktop
formatting passed. Native Windows worker checks await CI; no GUI UAT occurred.


## Interrupted content reconciliation

`engine/src/storage/install_recovery/` reconciles interrupted in-process content
work without downloading, extracting, deleting or resuming it. It requires an
operation awaiting reconciliation, matching saved intent and verified release
digest, and an unchanged resolved destination parent. Missing or empty output
commits `failed` and returns `NoOutput`, leaving the filesystem unchanged.

For nonempty output, recovery acquires the ownership-marker lock and reads its
bounded contents through that same handle (required by Windows lock semantics).
The marker must match the intent. Only a matching completion receipt and current
content checks permit `succeeded` / `ContentPrepared`; the lock remains held
through the journal commit. Partial content stays untouched and gated.

Content checks require the expected ledger, a nonempty regular executable and
an `SGWGame` directory, with resolved executable/game paths inside the content
root. They do not validate every installed file, runtime prerequisites or game
compatibility. This path covers only the native in-process worker and does not
establish that an unobserved Wine guest exited. Resume, cleanup and UI routing remain pending. Callers can use the saved signed
evidence below to supply the exact verified release used by the attempt.

Recovery fixtures cover missing/empty output, preserved partial content,
receipt-before-terminal recovery across reopen, missing executable, active owner
locks, foreign markers and Unix content-path redirection. Local checks passed:
168 engine tests, twelve process scenarios, strict all-target clippy and
formatting. Native Windows recovery checks await CI. No frontend changed, so
JS REPL/visual UAT does not apply to this packet.


## Signed release evidence for offline recovery

`engine/src/storage/release_evidence/` persists the exact signed manifest and
signature in `release-evidence-<operation-id>.bin` before install intent and
operation admission. Its format is a four-byte little-endian body length,
at most 1 MiB of manifest bytes and at most 256 signature bytes. It preserves
original bytes, including whitespace, instead of reserializing parsed JSON.
The existing 64 KiB limits for ordinary state files remain unchanged.

`cached_install_release` rechecks the current embedded signing-key policy and
matches the original-byte digest to the current durable intent. Missing,
malformed, oversized, tampered or mismatched evidence fails without fetching a
replacement from the mutable release URL. A signing-key change can prevent old
evidence from being accepted; this cache never bypasses current verification.
Per-operation evidence/orphan retention cleanup remains pending.

Fixtures cover exact-byte recovery after restart, tampering, a different valid
release, failed cache writes preventing admission and malformed/oversized/missing
files. Local validation: 172 engine tests, twelve process scenarios, strict
all-target clippy and formatting passed. Native Windows validation awaits CI.
This adds offline recovery inputs, not automatic resume or UI installation.
No frontend behavior changed, so JS REPL/visual UAT does not apply.
