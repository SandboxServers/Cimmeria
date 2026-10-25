# Desktop launcher foundation

> **Type:** Reference
> **Audience:** Launcher contributors
> **Last updated:** 2026-10-25
> **Companions:** [Implementation plan](../../../docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-plan.md), [build rules](../../../docs/architecture/build-system.md#standalone-desktop-launcher-validation), [test policy](../../../TESTING.md)

This standalone workspace contains native Rust state, Effect workflows and a
Tauri shell. The interface connects through Tauri invoke to native
preference persistence. Play is the main page; Settings contains maintenance,
folders, servers and recovery. There is no Patch Notes tab. Effect
installation controls call native IPC on Windows and on Mac builds containing
a helper verified against its compiled artifact identity. Mac builds without
that resource retain settings and the main-page change summary but cannot install. Content preparation does not establish runtime or
Play readiness. Confirmed uninstall and interrupted-removal recovery are wired
through Settings. Settings also connects confirmed [Repair](docs/repair.md),
precommit cancellation, recovery/abandonment and current-backup cleanup. [Play](docs/launch.md)
now observes native game lifecycle and, for a player who opts in, injects the
[game-telemetry DLL](docs/launch.md#opt-in-game-telemetry) with a server session.
Bounded native [adoption and Play phase timing](docs/launcher-telemetry.md) uses the existing opt-in launcher event pipeline.
The [privacy inventory](docs/privacy-inventory.md) lists every collector family,
fixed-schema field category, raw-log boundary, consent effect and storage limit
behind the expanded main-page disclosure. Game diagnostics are not anonymous.
Consented [launcher summaries](docs/launcher-summaries.md) are connected and
go only over https: to the first login server when it is https, else to the https upgrade of an http DNS-name login server (the same name on 443, LI-06), else to the built-in default, which is not set yet; an IP, localhost or single-label http server sends nothing. Real-client/platform gates remain open.
A player's own [seed archive](docs/seed-archive.md) can replace the seed download for Install, Repair and Update; the seed is never rehosted.
Explicit archive selection or accepting a Downloads offer starts an eligible
fresh installation as soon as verification succeeds, from either the main page
or Settings. The existing Install action refreshes native status and admission;
cancellation, inspection/reopen, failures and an existing/owned installation do
not start a new install. See the [handoff contract](docs/seed-archive.md#explicit-selection-starts-eligible-installation).

Adoption on Windows and macOS uses the selected installation root directly.
The shell offers Light only; it never copies or bulk relocates an installation.
Approved per-file setup, patches, cooked normalization, and replacement backups
can change content there. Schema-2 intent/plan validation selects that layout;
schema-1 managed and successful legacy installs retain `root/game`. Direct-root
Repair/Update commit per file, and Uninstall removes only managed release files,
retaining the root and extras. See [migration](docs/migration.md#adopting-an-existing-folder)
for validity, recovery, and verification boundaries. The native picker asks
**Choose SGW.exe in your existing install** and filters for `.exe` files. Only
`SGW.exe` is accepted; native resolution maps its `Working/Binaries` parent to
the installation root. Selection only opens the existing confirmation review;
no installation files change until approval. The IPC name remains
`choose_existing_game_folder`; no new command or Plan field is introduced.

## Windows single-executable packaging

The published Windows ZIP keeps its existing name and contains one root
`cimmeria-launcher-desktop.exe`. It embeds the three independently pinned Play
pieces, then verifies and repairs them at startup in the native app-local
`payloads/<content-digest>/windows/` cache. Shared version leases preserve active
payloads during cleanup. Embedded builds ignore older loose sidecars and leave
them untouched; players do not need to copy companion folders. Build staging
requires the pins, and packaging verifies a clean EXE-only copy with
`--verify-embedded-payload`. See [launch resources](docs/launch.md#embedded-windows-play-resources)
and the [release stages](docs/release.md#what-the-windows-job-builds).

macOS already distributes one `Cimmeria Launcher.app` bundle inside its ZIP;
its internal resource layout is unchanged. WebView2, game content and the
Mac Wine runtime remain separate prerequisites or downloads.

## Linux

The shell builds and its tests run on Linux (`native-engine (ubuntu-latest)` in
`launcher-desktop.yml`). Building needs WebKitGTK 4.1 and GTK 3 development
packages, plus clang and mold for `.cargo/config.toml`'s Linux linker.

**Install runs in-process, as on Windows.** The seed is the original installer's
RAR, whose `Data/DATA*.CAB` set Windows expands with `cabinet.dll`. Linux
expands it with the portable decoder in `cimmeria-launcher-core`
(`unpack::cab_portable`): MSZIP and stored folders, folders and blocks that
continue from one cabinet into the next, the cabinets' DOS timestamps, and the
same name and file-count preflight as the FDI path. The 2009 client's four
cabinets are all MSZIP. Independent folders expand on up to four threads; a
folder never splits across them. A Quantum or LZX folder is refused by name (Play reads the DirectX cabinets
separately, one LZX member at a time).
Resuming an interrupted install works as on Windows. Linux file systems are
case-sensitive and the cabinets spell `Working\binaries` in lower case, so plain
patch zips (such as `004-log-config`'s `Working/Binaries/SGWLogConfig.xml`) and
the `sgw_game` patch root are resolved against the install's own spelling, as
recipe patches already were. Adoption publishes its copy with
`renameat2(RENAME_NOREPLACE)`, the Linux form of the exclusive rename macOS and
Windows use; a filesystem without it refuses the publish.

Releases carry an x86-64 AppImage, `StargateWorlds-Launcher-linux-x86_64.AppImage`
([release](docs/release.md#what-the-linux-job-builds)).

**Play runs through Proton.** Linux Play needs a Proton build the player already
has (Proton Experimental or a numbered Proton from Steam, or GE-Proton from
ProtonUp-Qt); the launcher downloads none. It keeps a private prefix, extracts
the game's own DirectX DLLs from its cabinets, installs PhysX once and starts
the game windowed. The details, the failure codes and the checklist are in
[Linux (Proton)](docs/launch.md#linux-proton).

**Not on Linux yet:** Repair, Update and the launcher self-update. Those paths
still answer `platform_unavailable`.

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
does not establish installation ownership or safe deletion. The flag gates
[launcher summaries](docs/launcher-summaries.md) only, never game/DLL telemetry. Opt-out aborts a
send in flight. With no https endpoint (an IP, localhost or single-label http login server, and the built-in default is not set yet), nothing is tracked, queued or sent.

Preferences are `schema_version: 2`. They also hold the client options: the
`login_servers` list (empty means the built-in default list) and
`client_patches_enabled` (default on). A version 1 file reads as version 2 with
those defaults and is written back as version 2 by the next save. A file whose
fields do not match its version is corrupt and is left untouched.

`engine/src/commands.rs` exposes versioned `inspect`, `save_preferences`,
`save_client_options` and `save_display_options` commands. `save_display_options`
stores the game's display mode and window size (default windowed 1024x768) with
the same admission as `save_client_options`; it fails with
`invalid_display_options` on an out-of-range size or a mode the platform does not
offer, and a default display is stored as no key. Its optional `monitor` is
remembered per display setup ([launch.md § The monitor per display
setup](docs/launch.md#the-monitor-per-display-setup)). Every snapshot carries
`preferences.display` and `display_modes`, the modes this platform offers, plus
`displays`, `display_sizes`, `display_monitor` and `frame_caps` (the frame caps this platform offers: three on macOS, `fps30` and `fps60` on Linux, none on Windows). `save_client_options` replaces both client
options at once, needs the current preference revision, and is refused while the
launcher updater or any operation other than Play is running (Play reads them
only at launch, so a save during Play applies to the next one); it leaves the
install path and consent alone. A non-empty login-server list must pass the core
parser's rules or the save fails with `invalid_server_list` and writes nothing;
an empty list means the default server. Play writes the saved list (or the
default) into `LoginInternal.lua` and checks it again first; a legacy-bound
adopted copy keeps the imported list its adoption reviewed, and every snapshot carries that list as
`adopted_login_servers` (`null` otherwise) so the interface can say so. Every
snapshot also carries `launcher`, the compiled version and release tag (`null`
for a development build), which Settings shows on its About line. With
`client_patches_enabled` off, Play on a copy this launcher installed injects no
client-patches DLL, and admission refuses a launch request that carries one; the
telemetry DLL still follows its own opt-in. A direct-folder adoption follows the
current saved patch preference for the next Play. Only a legacy-bound adopted
copy (`adopted_login_servers` is non-null) keeps its reviewed patch setting,
whatever the preference says; the main-page control still saves the preference
and explains that binding until Repair. The opt-in telemetry session is minted
from the same list Play writes. No command lets frontend callers set native operation outcomes or
choose the state-directory root. The shell selects app data through Tauri's
native resolver. Native first-install intent validation and worker dispatch are
implemented below; installation and maintenance use a separate native operation boundary.

Three further Tauri commands began as scaffolds (LX-02); the ones not yet filled
answer `unavailable` until their packets land. Each fails with one flat code string, `unavailable` or a storage
code such as `busy`, the same shape `safeFailure` reads. Each takes a versioned
request with no free-form fields, has its
engine half in `engine/src/storage/<name>/` and its host half in
`shell/src/host/<name>.rs`, and has a panel in `frontend/src/<name>-view.ts` (Settings except for the
main-page client-change summary):

| Command | Requests | Panel | Filled by |
|---|---|---|---|
| `log_upload_command` | `inspect`, `upload` | `log-upload-view.ts` | LX-05 (done) |
| `client_maintenance_command` | `inspect`, `reset_cache`, `reset_all_client_state` | `client-maintenance-view.ts` | LX-06 |
| `client_changes_command` | `inspect` | Main-page `client-changes-view.ts` | LX-07 |
| `seed_archive_command` | `inspect`, `cancel`, `forget` | available; see [seed archive](docs/seed-archive.md) | AI-06 (part B) |
| `choose_seed_archive` | none; the native `*.rar` picker | available; see [seed archive](docs/seed-archive.md) | AI-06 (part B) |
| `choose_existing_game_folder` | `mode: light`; native SGW.exe file picker; `copy` and `in_place` are refused with `UnsupportedMode` | Adopts at the selected root on Windows and on macOS with a verified packaged helper; no copied installation | [Migration contract](docs/migration.md#adopting-an-existing-folder) |

> [!NOTE]
> `upload` reads the new launcher and game log lines and posts them to the telemetry
> server through a `purpose = logs` session (`engine/src/storage/log_upload/`:
> `lines.rs`, `game_lines.rs`, `offsets.rs`, `ingest.rs`, `ingest_flow.rs`). The
> operator side is [telemetry.md § Launcher log
> ingest](../../../docs/operations/telemetry.md#launcher-log-ingest).

The Settings login-server panel and main-page client-patches panel
(`server-list-view.ts`, `client-patches-view.ts`, LX-03 and LX-04) save through
`save_client_options`.

`client_maintenance_command` is filled (LX-06). `reset_cache` removes
`Documents\My Games\Firesky\SGWGame\Cache.en-US`; `reset_all_client_state`
removes the whole `Firesky` tree and is refused with `confirmation_required`
unless `confirmed: true`, which the panel sends only from its confirm step.
Both refuse with `busy` while the updater or any operation, Play included, is
running. Removal never follows a link: a link inside the tree is removed as
itself, and either reset is refused with `unsafe_path` when `My Games` or
`Firesky` (or, for the cache, `SGWGame`) is a link. `Documents` is the folder
the client asks the shell for (`SHGetFolderPathW(CSIDL_PERSONAL)`), so a
redirected (OneDrive) Documents is followed. On macOS every request answers
`profile_unavailable`: the Wine client writes inside its prefix, not under
`$HOME/Documents`, and resetting there is not built yet. The other code is `io`.

`physx_command` (`inspect`, `install`, `install_legacy`, LX-12) detects the exact
2.6.3 core on native Windows. Main-page **Install legacy PhysX…** downloads the
pinned NVIDIA 9.13.0604 MSI when that core is missing, including adopted copies
without a retained installer. Settings retains the game-bundled installer action.
Both require an explicit press and use the interactive elevated vendor wizard;
post-install detection hides the offer only when the core is found. macOS keeps
its Wine prerequisite flow. See [prerequisites](docs/prerequisites.md#native-windows-physx-system-software-lx-12).

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

Visible Windows folder paths omit the internal `\\?\` prefix (including the
`\\?\UNC\` spelling for network shares). This is display formatting only:
native snapshots, persisted preferences and command arguments keep their original
path spelling. Settings, adoption status/backups and maintenance confirmations
use the same formatter.

The main page checks for a signed launcher update at startup. Play waits for
that check and remains blocked when an update is known. **Update and restart**
downloads, verifies and applies it with one press; a previously verified Ready
package is applied without downloading again. No download or replacement starts
automatically. A failed check with no known offer permits Play with visible retry
feedback; a failed recheck retains a known offer and its Play gate. See the
[updater contract](docs/updater.md#startup-and-main-page-integration).

The Cimmeria Launcher window opens on Play. Client patches, launcher diagnostics
and game diagnostics sit below Play, with their existing native persistence and
separate consent gates. Diagnostics remain off by default. Game diagnostics
shows a compact status with details collapsed at rest; saving or an error opens
the details automatically. The full privacy copy is under About diagnostics
and privacy. The native Changes to
your client summary appears below them; View all changes expands the full list.
The main page scrolls so controls remain reachable in small windows.

Native `LaunchStatus.setup_complete` controls setup visibility separately from
Play admission. It requires verified installed content, available launch resources,
a prepared Wine runtime when required, no required reopen and either idle state
or a known non-reconciling launch. It remains independent of `installation_id`,
which can be withheld during a launch or launcher update. Once setup is complete,
the main page hides Recheck Play status and Use a downloaded game archive,
including while Play runs or a launcher update is offered. Settings retains
archive access and its own **Check Play status** button with adjacent result
and immediate first-press feedback. A known launcher update still gates
Play independently. A completed saved adoption no longer produces an unfinished
compatibility-setup warning; real failures and recovery remain visible. Adoption
button visibility retains its separate content-validity predicate.

The development UI explicitly labels unavailable game operations. Native
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

The retained Mac worker selects the Wine backend with a verified packaged helper.
Fixtures cover fresh output, patch reuse, hash failure before dispatch and
uncertainty retaining evidence. Dated results live in the implementation ledger.

`install_progress::ProgressSink::latest()` retains one progress value through a
Tokio watch channel. A stalled or disconnected observer cannot build a backlog
or fail installation. The legacy adapter keeps the unbounded event stream the
retired egui launcher's worker read. Progress is observational, not an operation journal
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

Windows expands the original archive's spanning cabinet set with FDI. Linux and
the Steam Deck use the portable MSZIP expander (`unpack::cab_portable`), with no
Wine. macOS keeps the Wine helper. Native Mac ZIP/RAR tests do not prove Wine
cabinet extraction. The next integration needs to invoke the Windows helper,
establish managed runtime/prefix ownership,
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

## Retained release catalog API

`engine/src/catalog/mod.rs` fetches the native-owned `content-current` manifest
and detached signature over HTTPS. Limits: 1 MiB manifest, 256-byte signature,
five-second connection timeout, fifteen seconds per request including body
consumption, and five redirects. Size limits cover declared and streamed bodies.

The engine shares `manifest` with the Windows launcher through
`cimmeria-launcher-core` (`crates/launcher-core/src/manifest.rs`); the catalog uses its schema, signature verification and validation,
but replaces its unbounded fetcher. Signature verification precedes JSON
parsing. Notes-only IPC exposes IDs, titles and descriptions in manifest order,
with the ID as fallback for a missing or blank title. Errors contain safe codes
only. Changes to the shared source also trigger the desktop CI workflow.

`LAUNCHER_MANIFEST_PUBKEY_HEX` supplies the build-time public verification key.
Release builds without a usable key fail closed. Debug builds without an
explicit value use the development test key for local signed fixtures. The
fixed public catalog refuses that default key before networking with
`CatalogError::DevelopmentSigningKey`, logged as `development_key`; a missing
usable key logs `signing_key_unavailable`. Local `verify_release` fixtures keep
their existing development-key behavior. Signature verification is unchanged.

Install maps either key preflight failure to `SigningKeyUnavailable`. Archive
selection distinguishes `signing_key_unavailable`, verification rejection
`release_verification_failed`, and network `release_unavailable`. Key guidance
directs players to the published ZIP and explains that the archive has not
been checked or changed. It does not diagnose an unverified RAR as corrupt.
See the [diagnosis](../../../docs/analysis/launcher-uat-fixes/design-development-catalog-key.md)
and [archive contract](docs/seed-archive.md#public-catalog-preflight).

### Build against released game content

Set the maintainer-approved release **public verification key** before compiling
a development launcher that will use published content. The value is 32 bytes
encoded as 64 hexadecimal characters. It is not the private signing key; never
copy a signing secret into a build environment or source file. Do not discover
or trust a key from the manifest being verified.

From a PowerShell session at the repository root:

```powershell
$env:LAUNCHER_MANIFEST_PUBKEY_HEX = '<approved 64-character release public key hex>'
pwsh tools/build-lane/lane.ps1 cargo run --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --example catalog_probe
```

Replace the placeholder before running the read-only catalog probe. Keep the
variable set while following the native build steps below. Restarting an
already-built launcher with a new environment variable cannot change its
compiled trust key: rebuild it. This key is independent of the launcher's
self-update verification key. For local fixture tests that expect the debug
key, use a separate shell without this variable or remove it before rebuilding
the tests:

```powershell
Remove-Item Env:\LAUNCHER_MANIFEST_PUBKEY_HEX -ErrorAction SilentlyContinue
```

The main-page layout removes the Patch Notes tab and its UI fetch. The native
read-only catalog API and its verification tests remain available to other
consumers and the probe below. The installed-client summary uses
`client_changes_command`, rather than presenting release notes as installed state.

A read-only live check uses the same native fetcher:

```bash

# Set LAUNCHER_MANIFEST_PUBKEY_HEX to the approved release public key first.
bash tools/build-lane/lane.sh cargo run --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --example catalog_probe --target-dir target/desktop
```

## Validation

Run from the repository root. Windows checks run natively on Windows; the Mac
launcher work runs natively on macOS, and the Linux work on Linux. All
compilation uses the pinned Rust toolchain and repository build lane:

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

The native-backed UATs (`uat:adoption`, `uat:launch`, `uat:migration`,
`uat:game-telemetry`, `uat:game-update`, `uat:game-update-apply`, `uat:updater`,
`uat:updater-apply`)
each spawn an ignored bridge test from a Cargo test binary named by an
environment variable. The updater pair needs the engine test binary in
`UPDATER_UAT_BINARY`; the others need the shell test binary
(`GAME_TELEMETRY_UAT_BINARY` for the game-diagnostics toggle). The adoption
bridge and its optional Wine backend are described in the
[migration contract](docs/migration.md#adopting-an-existing-folder).

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

The bundle is under `target/desktop/debug/bundle/macos/`. The released,
ad-hoc signed app is built in CI instead ([release](docs/release.md#what-the-macos-job-builds)).
Creating this bundle is not final self-contained startup validation. Do not open it during unattended
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
recovery. Platform provisioning, summary-export activation, migration/updater and final
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
verified packaged helper cannot install. Settings saves refresh installation status.
Verified Mac builds sequence content completion into prerequisite checks. [Play](docs/launch.md)
requires pinned native resources; Settings Repair follows the [repair contract](docs/repair.md). Confirmed uninstall uses the saved
installation identity; see [maintenance](docs/maintenance.md).

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

Content completion advances to compatibility setup when the native host exposes
an eligible installed identity. Reopening requires “Continue installation”; a
lost reply is inspected without replay. Explicit cancellation prevents automatic
advancement even if content completion wins that race. Failed content can be
cleaned up with confirmation and retried. Prerequisite success remains distinct
from Play readiness; see [prerequisites](docs/prerequisites.md#effect-and-main-install-flow).
The current frontend suite has 37 passing tests; TypeScript checking and the
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
Missing game files or the entire `game` directory preserve identity for
Repair; an existing game directory must be ordinary and not a link/reparse point.
Missing ownership records or signed evidence remain errors.

An older current successful Install can migrate on read. Selecting a folder never
adopts its contents. This record does not grant launch/delete permission or prove
readiness: current-operation, content and runtime checks remain required.
Repair follows the [maintenance contract](docs/repair.md).

Settings uninstall, explicit recovery and data-retention boundaries are described
in [installed-content maintenance](docs/maintenance.md).
See [prerequisite evidence and probe semantics](docs/prerequisites.md) for the
experimental Windows x86 module-load helper and remaining integration gates.

See [legacy migration](docs/migration.md) for the native preview/confirmed import API,
identity and consent preservation, historical ledger limits and crash recovery.
How a release is built and published is in [release](docs/release.md). Current import controls and signed launcher Apply/restart are documented in [migration](docs/migration.md) and [updater](docs/updater.md); production updater configuration remains disabled.

## Launcher build-time policy

Native PR checks restore the standalone Rust graph and save caches from main
only. Superseded PR runs cancel; main runs finish to populate those caches.
CI disables dev/test debug information while retaining native tests, clippy,
executable builds and JS persistence UAT. Local build settings stay unchanged.
Release feed and overlay CLIs use a fast-compiling `release-tools` profile;
the launcher and shipped helpers retain normal release builds. The signing
runner remains uncached and sees its key only during signing. See the
[release contract](docs/release.md#build-time-and-signing-isolation),
[pre-PR checks](../../../docs/agents/pre-pr-checks.md#desktop-launcher-ci-policy-checks)
and [measured design](../../../docs/analysis/launcher-uat-fixes/design-ci-speed.md).

## Stable repeat releases

D-LX15 uses UTC stable versions `YYYY.M.(D*100+N)`, daily slots 1..99.
Every Windows updater artifact requires a Minisign signature; the workflow
refreshes `launcher-current` for repeat releases too. Existing installations
on D-LX8's date versions accept the new versions. See the
[release contract](docs/release.md#version-and-tag) for identity reservations,
reruns, Windows file-version limits and the signing environment.
Applied content observations read the retained installed manifest and live
ledger; [launch.md](docs/launch.md#applied-content-identity) defines their fields.
