# Desktop launcher foundation

> **Type:** Reference
> **Audience:** Launcher contributors
> **Last updated:** 2026-10-04
> **Companions:** [Implementation plan](../../../docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-plan.md), [build rules](../../../CLAUDE.md), [test policy](../../../TESTING.md)

This standalone workspace contains native Rust state, Effect workflows and a
Tauri settings shell. The interface connects through Tauri invoke to native
preference persistence. Play/Patch Notes tabs and the settings panel are
implemented; patch notes load from a signed release manifest. Effect
installation controls call native IPC on Windows and on Mac builds containing
a helper verified against its compiled artifact identity. Mac builds without
that resource retain settings and patch notes but cannot install. Content preparation does not establish runtime or
Play readiness. Repair, removal, launch and telemetry export remain pending. The
existing Windows egui launcher shares the downloader cancellation fixes.

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
adapters must also surface `requires_reopen`. Windows-native engine checks have
passed; power-loss durability remains unvalidated; OS sync calls alone do not
prove hardware crash behavior.

On the first untouched preferences state (revision zero, no saved path and no
operation), the shell saves `<app-local-data>/Stargate Worlds` as the default folder
with diagnostics consent off. This saves a preference only: it creates no game
directory, downloads nothing and starts no installation. Only its native parent
is created if needed; the existing app-data/state settings root is unchanged.
Windows uses LocalAppData rather than roaming storage for the game path. Previously saved or
explicitly cleared paths and consent choices are preserved.

Preferences persist an optional absolute install path and separate default-off
`launcher_summary_consent`. Saves require the current preference revision. An
active operation blocks path changes but permits consent changes. The path check
does not establish installation ownership or safe deletion. This flag is for
future launcher summaries only; there is no exporter or game/DLL consent
integration yet, and no immediate-export revocation claim is made by this
packet.

`engine/src/commands.rs` exposes versioned `inspect` and `save_preferences`
commands. No command lets frontend callers set native operation outcomes or
choose the state-directory root. The shell selects app data through Tauri's
native resolver. Native first-install intent validation and worker dispatch are
implemented below; frontend operation commands remain pending.

## Effect workflows

`frontend/` pins Effect **4.0.0** and uses its actual services, scopes,
semaphore, Ref, Schema, scheduling, test clock and bounded PubSub. The version
was checked against the npm stable tag on 2026-10-04. API references:
[scopes](https://effect.website/docs/v4/resource-management/scope) and
[scheduling](https://effect.website/docs/v4/scheduling/using-schedules).

`bridgeLayer` accepts a transport: the shell supplies Tauri invoke and logic UAT
supplies the native process harness. Replies are schema-validated, native
failures are allowlisted codes, and raw transport errors are discarded.
`makeLauncher` lives in the application scope. It serializes inspection/saves,
rejects older snapshots, and publishes a one-entry sliding state stream so slow
views cannot build an unbounded progress backlog.

Read-only inspection has at most two retries, with 100 ms exponential backoff.
Each settings IPC call has a five-second observation timeout. Saves are sent
**once**; timeout, interruption or failure leaves inspection required before
another mutation. A cancelled Effect fiber does not cancel a native save. Scope
cleanup releases subscriptions; it does not claim native rollback. Uncertain
native storage keeps the mutation gate closed until the native store is
reopened.

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

The development UI explicitly labels unavailable game operations. Verified patch
notes describe available release patches, not installed-game status. Native
window appearance, dialogs and actual Tauri IPC still require interactive UAT.
Compilation and headless DOM tests do not establish those behaviors.

## Shared installer core

The engine imports the existing launcher installation, archive, layout, state,
patch destination, report and client-preparation modules through Rust path
modules. It depends on the existing `cimmeria-patchset` crate for delta patches
and UPK normalization. Algorithms and their regression tests have one source;
the standalone lockfile resolves their dependencies independently. This includes
the existing workspace-hack dependency transitively; the desktop workspace is
still excluded from root aggregate checks.

`install_all_with_seed_extractor` adds a seed-only native adapter boundary;
existing `install_all` callers keep their current extraction path. `SeedBackend`
separates the download cache from fresh content staging. The shared pipeline
verifies the seed hash before invoking the adapter, while patch overlays,
installation-state updates and client preparation remain shared. An uncertain
external extraction retains authenticated input and partial output without
recording the seed as applied; the desktop worker maps this distinct outcome to
reconciliation. Successful extraction permits archive cleanup and normal
patching.

No production caller selects this backend yet. No Wine runtime was selected or
invoked. Fixtures cover fresh output, native patch overlay and subsequent reuse,
hash failure before adapter dispatch, and uncertainty retaining evidence without
a completion ledger. The engine suite passed 181 tests; the three enhanced seed
fixtures also passed separately. Strict all-target engine clippy and root
formatting also passed.

`install_progress::ProgressSink::latest()` retains one progress value through a
Tokio watch channel. A stalled or disconnected observer cannot build a backlog
or fail installation. The existing egui worker uses the legacy adapter and
preserves its event stream. Progress is observational, not an operation journal
or proof that the game is ready.

Before HTTP, shared seed extraction checks an existing cache's complete declared
size and SHA-256. Authenticated complete input is reused even if its host is
unavailable; extraction verifies it again. A complete cache with a confirmed hash mismatch is deleted before a fresh
non-Range download. Incomplete cache is retained for ordinary Range handling;
downloaded content is still verified. A fixture proves zero requests for valid
cache and one request for invalid cache. This does not enable automatic resume.

The loopback installer test downloads a synthetic PE-in-ZIP seed, verifies its
hash, extracts it, overlays a patch, writes the login file, disables ASLR and
persists installed state. Its second pass makes no further blob requests, while
a stalled watch receiver retains the newest progress. This exercises the real
pipeline, not simulated progress. It does not download or launch SGW.

The original archive's spanning cabinet set still requires Windows FDI. Native
Mac ZIP/RAR tests do not prove Wine cabinet extraction. The next integration
needs to invoke the Windows helper, establish managed runtime/prefix ownership,
validated operation intents, reconciliation, cancellation and terminal readiness
checks. The legacy pipeline's success can occur without SGW.exe; a new readiness
adapter must not equate it with Play-ready. Existing install state is not an
ownership marker permitting uninstall, and its permissive reads are not recovery
proof. See [runtime provisioning
evidence](../../../docs/analysis/playtests/2026-10-03-macos-wine/runtime-provisioning.md).

## Wine runtime and helper validation

The experimental headless Wine adapter prepares a pinned runtime and uses a
Windows helper with durable ownership checkpoints. Fixture ZIP and original client
RAR/chained-cabinet extraction smokes passed; patching and runtime/game readiness
remain separate gates. Mac installation is conditional on the packaged helper binding below. See [Wine/runtime/helper validation](docs/wine-validation.md) for
protocol limits, backend identity, cancellation/recovery boundaries, managed cache
checks and supervised smoke commands. Game prerequisites and readiness remain
separate gates.

## Verified release patch notes

`engine/src/catalog/mod.rs` fetches the native-owned `content-current` manifest
and detached signature over HTTPS. Limits: 1 MiB manifest, 256-byte signature,
five-second connection timeout, fifteen seconds per request including body
consumption, and five redirects. Size limits cover declared and streamed bodies.

A Rust path module shares `crates/launcher/src/manifest.rs` with the Windows
launcher; the catalog uses its schema, signature verification and validation,
but replaces its unbounded fetcher. Signature verification precedes JSON
parsing. Notes-only IPC exposes IDs, titles and descriptions in manifest order,
with the ID as fallback for a missing or blank title. Errors contain safe codes
only. Changes to the shared source also trigger the desktop CI workflow.

`LAUNCHER_MANIFEST_PUBKEY_HEX` supplies the existing build-time trust key.
Release builds without a usable key fail closed. Test/debug builds can use the
existing development fallback; it cannot authenticate the live release. Supply
the maintainer-approved public key when building a launcher for live content. Do
not discover or trust a key from the manifest being verified.

The Patch Notes tab starts an independent read-only Effect fetch on first open.
Refresh explicitly fetches again; no automatic retries or overlapping UI loads.
Frontend observation times out after 35 seconds. Disposal interrupts
observation, not native network work, which remains bounded by its own
deadlines. Successful notes stay in session memory; a failed refresh retains
them with an explicit previously-verified label. All content renders as literal
DOM text, never HTML.

A read-only live check uses the same native fetcher:

```bash
# Set LAUNCHER_MANIFEST_PUBKEY_HEX to the approved release public key first.
bash tools/build-lane/lane.sh cargo run --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --example catalog_probe --target-dir target/desktop
```

## Validation

Run from the repository root. Windows checks run natively on Windows; the Mac
launcher work runs natively on macOS. All compilation uses the pinned Rust
toolchain and repository build lane:

```bash
bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml
bash tools/build-lane/lane.sh cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml --all-targets -- -D warnings
cargo fmt --all --manifest-path crates/launcher/desktop/Cargo.toml -- --check
npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend
npm run check --prefix crates/launcher/desktop/frontend
npm test --prefix crates/launcher/desktop/frontend
npm run uat:install --prefix crates/launcher/desktop/frontend
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
Windows). For a development Mac `.app`, after building the UI and installing the
Tauri CLI, use the lane for its nested Cargo build:

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

Validation covers native ownership, persistence/revision guards, process locking,
helper protocols, interrupted content and explicit cleanup. The lock/death test
kills an idle child after committed writes; it is not a power-loss or
kill-during-write simulation. Effect tests use virtual time for retry/timeout
behavior. The settings JS UAT uses real Rust persistence across restart;
installation-control UAT uses mocked IPC. Neither establishes native visual,
keyboard or real-dialog behavior.

Current external-asset evidence and smoke commands live in
[Wine validation](docs/wine-validation.md). Dated counts and exact CI revisions
live in the [implementation ledger](../../../docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-ledger.md).
An older green run never validates a later revision. Ignored external-asset tests
must be reported separately when explicitly run.

## Next integration gates

Verify the native window, keyboard behavior, actual Tauri IPC, folder chooser
and saved-folder reveal interactively. Validate the newly connected Effect
installation controls against actual native IPC, including cancellation and
recovery. Platform provisioning, telemetry, migration/updater and final
self-contained startup and release gates remain open.

The shell includes PNG and Windows ICO resources. Tauri compiles the ICO into
the Windows executable even for shell tests; native Windows CI is the check for
that resource step. The ICO is copied from the existing Windows launcher.

Catalog tests cover signed fixture decoding, order/title fallback, tampered and
malformed signatures, invalid JSON/schema, size bounds, and loopback HTTP status
and chunked-body handling. The live `catalog_probe` succeeded on 2026-10-04 with
seven patches using the release public key recorded in the bring-up handoff.
This verifies a current response, not future availability or packaged UI
routing. The local fixture suite uses the development key; run it without a
release-key override. Real Tauri catalog IPC and visual UAT remain unverified.

Catalog CI run `37183173769` passed both native platforms at `f8ea7b844`. The
shared-installer/helper packets need their own native Windows results. Helper
stdio mechanics are compiled and tested on Mac without running the Windows
executable. Shell-host tests were last run in the preceding packet.

## Durable install admission

`catalog::VerifiedRelease` retains the validated manifest and SHA-256 of its
original signed bytes. Native callers obtain it through signature/schema
verification; a frontend request cannot deserialize one.

`DesktopState::admit_install` records immutable inputs in
`install-intent-<operation-id>.json`: operation ID, original preference
revision, canonical destination, verified manifest digest and native-supplied
login servers. It writes this record before admitting the operation with the
intent's digest. `dispatch: true` is returned only after both writes succeed.
Admission itself dispatches no worker and creates no game files. Separate
filenames preserve the previous operation's evidence if a subsequent admission
fails between writes. Orphan/history cleanup remains part of coordinator
recovery work.

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
retries, release conflicts, busy/stale requests,
missing/corrupt/digest-mismatched intent, failed writes, failed replacement
admission, empty-directory acceptance and unsafe destination rejection (symlinks
on Unix). This establishes neither installation readiness nor UI install
behavior. No frontend behavior changed, so JS REPL/visual UAT does not apply to
this packet. The subsequent worker section records dispatch implementation;
runtime readiness, reconciliation and UI installation remain pending.

Admission validation: 152 engine tests passed on macOS through the build lane,
plus the unchanged twelve-scenario process harness; strict all-target clippy and
formatting passed. Windows admission validation awaits native CI.

## Native first-install worker

`engine/src/storage/install_worker/` dispatches an admitted intent only after
matching its operation ID and verified release digest. It commits `running`
before claiming the destination; a second dispatch is rejected. The task retains
native state independently of observers. Dropping a view or progress receiver
does not cancel installation. Explicit cancellation commits `cancel_requested`
before signalling the worker.

The worker rechecks the canonical first-install destination, creates an
exclusive `.cimmeria-install.json` marker and holds its OS lock. It runs the
shared pipeline inside `.cimmeria-stage-<operation-id>`. Before promotion it
reads the bounded installation ledger, checks expected seed and patch entries,
and requires a nonempty regular SGW executable and an `SGWGame` directory. These
are content checks, not a complete extracted-file inventory, executable
compatibility or gameplay validation. Filesystem ownership is cooperative, not a
hostile-user sandbox.

Content moves to `<selected-directory>/game`. The worker persists
`content-ready.json` before committing operation success and publishing
`ContentPrepared`. Receipt or terminal-state persistence failures require
reconciliation; visible files alone are not reported as confirmed success.
Power-loss durability of the complete extracted tree remains unvalidated.

Failure and cancellation retain the marker and partial output. Interrupted
native content can be inspected conservatively as described below; automatic
adoption remains unimplemented; explicit failed-attempt cleanup and retry are
described below. The Wine cabinet-helper
adapter, runtime provisioning and frontend dispatch remain pending. Installation
is now connected through the Windows UI described below; content preparation
does not establish launch readiness.

Fixtures cover actual HTTP/ZIP staging and promotion, observer disposal,
duplicate dispatch, missing-executable rejection, cancellation while waiting for
headers and at an extraction checkpoint, changed destinations/redirected
parents, and receipt failure after promotion. The shared downloader now
interrupts waits for HTTP headers and body chunks; extraction cancellation
remains `Cancelled` through the install pipeline instead of becoming a generic
patch failure. No frontend behavior changed, so JS REPL/visual UAT is not
applicable here.

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

Content checks require the expected ledger, a nonempty regular executable and an
`SGWGame` directory, with resolved executable/game paths inside the content
root. They do not validate every installed file, runtime prerequisites or game
compatibility. This path covers only the native in-process worker and does not
establish that an unobserved Wine guest exited. Explicit native resume is
described below; cleanup and frontend routing remain pending. Callers can use
saved signed evidence to supply the exact verified release used by the attempt.

Recovery fixtures cover missing/empty output, preserved partial content,
receipt-before-terminal recovery across reopen, missing executable, active owner
locks, foreign markers and Unix content-path redirection. Local checks passed:
168 engine tests, twelve process scenarios, strict all-target clippy and
formatting. Native Windows recovery checks await CI. No frontend changed, so JS
REPL/visual UAT does not apply to this packet.

## Signed release evidence for offline recovery

`engine/src/storage/release_evidence/` persists the exact signed manifest and
signature in `release-evidence-<operation-id>.bin` before install intent and
operation admission. Its format is a four-byte little-endian body length, at
most 1 MiB of manifest bytes and at most 256 signature bytes. It preserves
original bytes, including whitespace, instead of reserializing parsed JSON. The
existing 64 KiB limits for ordinary state files remain unchanged.

`cached_install_release` rechecks the current embedded signing-key policy and
matches the original-byte digest to the current durable intent. Missing,
malformed, oversized, tampered or mismatched evidence fails without fetching a
replacement from the mutable release URL. A signing-key change can prevent old
evidence from being accepted; this cache never bypasses current verification.
Per-operation evidence/orphan retention cleanup remains pending.

Fixtures cover exact-byte recovery after restart, tampering, a different valid
release, failed cache writes preventing admission and
malformed/oversized/missing files. Local validation: 172 engine tests, twelve
process scenarios, strict all-target clippy and formatting passed. Native
Windows validation awaits CI. This adds offline recovery inputs, not automatic
resume or UI installation. No frontend behavior changed, so JS REPL/visual UAT
does not apply.

## Explicit interrupted-install resume

`install_worker::resume` requires the current operation ID and inspected
revision. It accepts only `reconciliation_required`, reverifies cached signed
evidence, acquires the matching ownership-marker lock and validates staging and
its ledger. It rejects symlinks/special files, Windows reparse points,
conflicting ledger entries, and existing promoted content or receipts. It
commits `running` before continuing the shared pipeline, including Range
downloads from partial archives.

Resume is never automatic on restart. Supported Windows builds expose an
explicit resume control through the Effect workflow described below. It does not
retry terminal cancelled/failed attempts or establish Wine guest ownership.
Promoted-content uncertainty belongs to recovery inspection. Fixtures cover
Range continuation through promotion, interruption before staging creation,
stale duplicates, corrupt-ledger preservation, active ownership,
promoted-content refusal, failed journal commit and Unix staging-symlink
rejection. They do not prove recovery from every mid-extraction or mid-patch
checkpoint.

Validation: 178 engine tests and twelve process scenarios passed locally, with
strict all-target clippy and formatting. Native Windows resume checks await CI.
No frontend behavior changed; JS REPL/visual UAT does not apply to this packet.
The desktop CI workflow now lets active native checks finish instead of
cancelling on each milestone push. GitHub keeps the latest pending run for this
branch; results for an older commit are never evidence that the latest commit
passed.

## Restricted installation IPC

The shell's versioned `install_command` exposes inspect, install, cancel, resume
and reconcile. Install carries an operation ID and operation/preference
revisions; resume and reconcile require the currently inspected ID/revision.
Cancel identifies the retained worker. Requests cannot supply paths, executable
configuration, URLs, manifests or native outcomes.

New installation requests fetch the fixed signed release natively. An identical
current-operation retry first reverifies its cached release and checks the
original preference revision and native login-server inputs. It does not fetch
the mutable release URL again; invalid cached evidence fails without fallback.
Resume and reconciliation also use saved signed evidence.

The host shares one `DesktopState` with its retained worker. If dispatch fails
after admission, it attempts to mark the operation for reconciliation.
Successful reconciliation clears retained worker observations so status cannot
reuse an old outcome. Status includes the native snapshot and platform support;
progress projects only fixed download/extraction phases and JavaScript-safe
counts, with no path-bearing labels. The native snapshot still includes the
saved directory. Failures cross IPC as flat allowlisted codes.

Install supports Windows and verified-helper Mac builds. Unsupported builds
reject it before downloads or destination mutation. Resume supports interrupted
native Windows attempts; Wine resume/recovery remains rejected. The Effect
controls follow the native capability flags. Native IPC
availability does not establish interactive Tauri routing or runtime/gameplay
readiness.

Local validation for this packet: thirteen shell tests passed. No frontend
behavior changed, so JS REPL/visual UAT does not apply to this native-only
change. Prior [CI run
37187754913](https://github.com/SandboxServers/Cimmeria/actions/runs/37187754913)
passed macOS and Windows at `117344e76`. The resume packet's [run
37188326146](https://github.com/SandboxServers/Cimmeria/actions/runs/37188326146)
at `bf8029e28` passed both native platforms. The newer shell [run
37189445603](https://github.com/SandboxServers/Cimmeria/actions/runs/37189445603)
at `17b949f4c` passed macOS and Windows. These revisions do not validate the
newer frontend installation controls.

## Effect installation controls

`frontend/src/install-workflow.ts` and `install-view.ts` connect installation
controls to native IPC on Windows and verified-helper Mac builds. Install
requires a selected folder and no current operation. Mac builds without the
verified packaged helper cannot install. Settings saves refresh installation status. Repair,
uninstall, runtime setup and Play remain unavailable.

The application-scoped Effect service inspects native state before each
mutation, uses current revisions and never automatically replays a mutation
after a lost reply. Read-only inspection retries transport errors at most twice
with 100 ms exponential backoff. The install admission IPC reply times out after
35 seconds, as do reconcile/cleanup replies; reads and other IPC replies time
out after five seconds. These deadlines do
not cancel native work or cap overall operation polling, which continues until a
terminal/recovery state or observation failure.

Active operations are polled every 250 ms without holding the command semaphore
between polls, allowing explicit cancellation. Tabs preserve the workflow;
disposal stops frontend observation without cancelling native installation.
`requires_reopen` stops observation and directs the user to restart. Interrupted
operations expose explicit inspection/reconciliation and resume controls.

Success displays “Content prepared”, never Play-ready. Failed/cancelled attempts
retain partial files and cannot be retried or cleaned up through this UI yet.
The current frontend suite has 23 passing tests; TypeScript checking and the
frontend build passed. `npm run uat:install` passed a sequential actual-DOM and
Effect fixture flow covering installation/progress, cancellation requested,
disposal/reconnection without replay, completion winning cancellation, and no
Play-readiness or consent inference. Its native IPC is mocked: it does not
exercise filesystem installation, Wine, visual layout or the real game. CI now
runs this installation logic UAT. The separate `npm run uat` against the Rust
`state_bridge` also passed, preserving real settings-disk/restart coverage. The
earlier approved settings preview predates these controls; their native visual,
keyboard and actual Tauri IPC UAT remain unverified. No real game or runtime
readiness is established by frontend tests.


## Retained Wine content worker

`install_worker::dispatch_wine` connects an admitted Wine intent to a retained
native task. It validates backend/runtime/helper identity before committing
`running`, then claims the destination, prepares the Wine adapter and invokes
the shared seed pipeline with separate cache and fresh staging. Patch overlays
and client setup remain native; ordinary content checks, promotion and receipt
publication follow extraction. Observer disposal does not abort the task.

`RosettaRequired` and `RuntimeUnavailable` outcomes now have typed frontend
messages, distinct from cancellation and uncertainty. Packaged resource binding now enables Mac content installation when verified;
Wine resume remains unsupported; conservative Mac reconciliation is described
below. A caller hashing
a local helper file is not an artifact trust policy. Native Windows replay of Wine intents remains
rejected; this API alone does not establish packaged startup or game readiness.

An explicitly run retained-worker fixture passed in 22.596 seconds, combining
Wine seed extraction, a native ZIP patch and content receipt after its observer
was dropped. Both strengthened ignored checks passed in a 19.090-second run: duplicate
dispatch was rejected, and immediate cancellation preceded runtime cache, prefix,
helper and network work. Strict engine clippy and thirteen shell tests passed; the final engine suite
passed 204 tests with eight ignored entries. Twenty-five frontend tests,
checking/build and sequential JS logic UAT passed,
including new failure decoding and preserved consent. Native visual UAT was not
performed. These fixtures do not establish real game prerequisites or readiness.


## Packaged Mac helper binding

The shell resolves only `resource_dir/windows/cimmeria-archive-worker.exe` and
checks it against compile-time `CIMMERIA_WINDOWS_HELPER_SHA256`. It verifies again
before release fetching/admission and persists the immutable Wine backend.
Missing, mismatched or unconfigured helpers leave Mac installation unavailable.
Status now declares `can_resume`/`can_reconcile`; Wine resume stays hidden; eligible Mac reconciliation is now exposed as
described below.
The engine continues to reject native replay of Wine intents.

See the [repeatable helper staging and Mac build recipe](docs/wine-validation.md#packaged-helper-staging-and-mac-build).
A development bundle with the verified helper was built without opening it.
The staged-resolver → Wine-admission → cancellation integration passed in 1.961
seconds. Python staging guards, 26 frontend tests, check/build and sequential
logic UAT passed. Final checks passed 205 engine tests (eight ignored), 15 shell tests (one
ignored), and strict engine/shell clippy. The ignored resource smoke passed
separately. The final development bundle built with its embedded helper hash
verified; packaged permission verification remains pending. No visual UAT or packaged-app startup was performed; the final
self-contained startup gate remains deferred. Game prerequisites/Play and Wine
recovery remain unfinished.


## Durable installation outcomes

`install-result.json` is a bounded schema-1 record binding an outcome to its
operation ID, intent digest and exact terminal journal revision. The worker
writes it before committing the terminal operation state. The record alone never
proves completion: active, reconciliation-required and requires-reopen states
hide it. An older operation/reconciliation revision cannot reuse it; a matching
record with inconsistent digest or terminal state is rejected as corrupt.
Legacy journals may legitimately have no result record.

Shell status reads this durable outcome, so a confirmed failure reason survives
host disposal and reopening without a retained worker. This adds result reporting,
not itself recovery or retry; those explicit flows are described separately. The shell suite passed 15 tests with
one ignored entry, including actual host drop/reopen preserving `InstallFailed`.
Final engine checks passed 209 tests with eight ignored entries; combined strict
clippy and `npm run uat:install` passed. That JS pass uses mocked installation
IPC, not native filesystem/Wine or visual UAT. The existing Windows launcher now
exports the shared install module publicly, matching its progress API and fixing
unused/dead-code lint failures; native Windows clippy passed on commit `b2032cc63` (run 37194766086);
final-revision Windows build/tests remain required.

## Conservative Mac Wine reconciliation

Mac Wine operations awaiting reconciliation now expose inspection through
`can_reconcile`; `can_resume` remains false. With no helper journal, the durable
protocol establishes that no helper was spawned, so inspection needs no runtime
or download. Otherwise, only observed finished helper results are eligible.
Recorded hosts must be absent: signal zero checks liveness but never kills a PID,
and live/reused PIDs keep recovery gated. LaunchIntent, HostStarted and Uncertain
records remain blocked pending startup/descendant crash validation.

Recovery locks the canonical, intent-matching private prefix, verifies the full
cached runtime under its cache lock, then runs prefix-scoped `wineserver -k/-w`
with ten-second limits per command. Both ownership guards remain held through
content/receipt inspection and the journal decision. It never downloads, deletes,
resumes or targets shared profiles. Stop/wait failure cannot authorize success.

The reopened-state managed-Wine ZIP smoke passed in 22.685 seconds, stopping the
verified prefix while preserving output and unresolved recovery state. This is
not proof of arbitrary crash recovery. Final checks passed 212 engine tests/eight ignored, 15 shell tests/one ignored
and combined strict clippy. JS logic UAT passed enabled reconciliation without
resume or inferred success, using mocked native IPC. No native visual or gameplay UAT is claimed.

## Confirmed partial-install cleanup and retry

`CleanFailed` requires explicit confirmation plus the inspected operation ID and
revision. Only terminal failed/cancelled installations qualify. Cleanup checks
the canonical destination, locks its exact intent-matching owner marker, and
preflights every entry before deletion. Only that attempt's marker, staging and
cache names are allowed; promoted content, receipts and foreign files veto the
operation. Recursive checks reject symlinks, special files and Windows reparse
points. Eligible Wine attempts retain the conservative prefix-stop requirements.

Cleanup deletes staging/cache, then the marker last, leaving the destination
empty. Repeating explicitly after inspecting a lost reply is idempotent; it does
not admit a replacement operation or change the old terminal state. `can_retry`
enables a separate fresh-UUID install. Selecting another empty folder also enables
retry while preserving previous output. Runtime cache and extraction prefixes
remain; this is not uninstall or full application cleanup.

The UI asks for confirmation and supports dismissal without dispatch. Install,
reconcile and cleanup replies have 35-second observation deadlines; reads retain
five-second deadlines. Final ordinary checks passed 217 engine tests (nine opt-in tests now ignored)
and 16 shell tests/one ignored. Twenty-six frontend tests, checking/build and JS
logic UAT passed confirmation/dismissal, one cleanup call, new-ID retry and
unchanged consent. Combined strict clippy passed. Windows junction regression is added
but not locally run. Real failed-Wine cleanup passed in 24.096 seconds: the helper
finished extraction, invalid content failed validation, confirmed cleanup emptied
the game destination and enabled retry without changing consent. Native visual UAT remains open.

Current default-folder/cache checks passed 218 engine tests/ten ignored and
17 shell tests/one ignored. Mocked-native JS installation UAT passed the default
folder/enabled-install/unchanged-consent case. Strict clippy passed. The real signed-release content test passed in 314.73
seconds; its initial lane invocation later failed in a development-key harness
run under the production key. The corrected `--lib` rerun passed in 312.55
seconds (315.308 seconds lane, exit zero); see [validation details](docs/wine-validation.md#original-signed-release-content-smoke).

## Installed-content identity

`installed-content.json` preserves the installation identity independently of the
latest operation and selected folder. It is written after promotion/receipt and
before terminal success, including recovery; publication failure retains the
recovery gate. Reading returns `InstalledContent` with a reverified signed release,
matching saved per-ID intent, canonical owned root, owner marker and receipt.
Missing game files or the entire `game` directory preserve identity for future
Repair; an existing game directory must be ordinary and not a link/reparse point.
Missing ownership records or signed evidence remain errors.

An older current successful Install can migrate on read. Selecting a folder never
adopts its contents. This record does not grant launch/delete permission or prove
readiness: current-operation, content and runtime checks remain required. There
is no new UI or Repair execution in this packet.

Settings uninstall, explicit recovery and data-retention boundaries are described
in [installed-content maintenance](docs/maintenance.md).
