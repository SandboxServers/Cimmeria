# Implement the Tauri launcher with Effect and focused operation telemetry

> **Type:** How-to (implementation plan, not a completion report)
> **Audience:** Agents and maintainers delivering the next launcher packets
> **Last updated:** 2026-10-04
> **Status:** Planning authorized; this packet changes documentation only
> **Companions:** [Platform research](launcher-platform-options.md), [packaging proof](../../../../crates/launcher/prototype-packaging/README.md), [playtest handoff](README.md), [test policy](../../../../TESTING.md), [telemetry operations](../../../operations/telemetry.md)

## Deliver the real workflow before the final packaging gate

Build the approved single-game experience in Tauri, using **Effect for real
application orchestration**, with retention of the current Rust installer and Windows game-launch mechanisms
as the recommended implementation strategy. Rust retention is not a user-mandated
requirement: its case is the verified behavior and test reuse documented below;
changing it would require a reviewed replacement/parity plan. Connect Play, Patch Notes and gear settings to
verified state. Do not promote the packaging proof's mock model into production.

“Traversal” is interpreted here as the getting-started → install → launch
journey. In-game travel instrumentation is outside this plan unless the user
explicitly expands that scope. Prior visual approval and packaging experiments
do not establish real workflow or telemetry readiness.

The authorized ordering puts self-contained startup testing **last**. It remains
a release gate: neither a successful build nor a working development webview
permits claiming a distributable launcher. The Mac experiment is scoped; retain
the Windows-native production build rules and route every compiling Cargo call,
including Tauri's nested build, through `tools/build-lane/lane.sh`.

Do not enable or connect WireGuard, query live observability, deploy telemetry,
or contact production ingestion while executing this plan. Use fixtures, local
servers and controlled failure injection. Any later live rollout is a separately
scoped action. This plan does not claim login readiness or game compatibility.

## Reuse audit and seams to extract

The following contracts were inspected in the current repository. File paths are
from the project root; these modules remain authoritative until extracted with
parity tests.

| Existing implementation | Reuse and required adaptation |
|---|---|
| `crates/launcher/src/manifest.rs`, `install.rs`, `install_report.rs` | Preserve signed-manifest validation, resumable downloads, hashes, declared patch order, dependency outcomes and honest partial-failure reporting. Expose typed results; do not duplicate these algorithms in TypeScript. |
| `unpack/`, `install_layout.rs`, `patch_dest.rs` | Preserve extraction and layout safety. Windows FDI cabinet extraction needs a tested Windows/Wine adapter for Mac; renderer changes do not solve this. |
| `state.rs`, `config.rs`, `identity.rs`, `instance_lock.rs` | Reuse state meanings and consent defaults. Specify app-data location and migration, atomic writes and operation ownership. Do not silently mint a new identity or replace an existing install ledger. |
| `client_setup/`, `client_patches/`, `client_telemetry_dll.rs` | Keep launch preparation, stock-case corrections, login-server setup and DLL plans in Rust. Move preparation out of the UI thread. |
| `worker/messages.rs`, `worker/mod.rs` | Current messages are in-process, not a versioned IPC contract. Add operation IDs, typed failures, snapshots and cancellation acknowledgements; bound event delivery. |
| `worker/launch_sgw.rs`, `start32_helper.rs`, `crates/client-launch` | Retain suspended-process/injection/resume behavior. A Windows PID is not a macOS process handle; track the helper and guest game lifecycle explicitly. |
| `telemetry/endpoint.rs`, `auth.rs`, `queue.rs`, `chunk.rs` | Reuse reviewed endpoint/auth/budget ideas where appropriate; do not start the game-tail/DLL pipeline just to report an install failure. |
| `self_update/` | Preserve current updater until an explicit migration packet. A Tauri updater needs SemVer/asset mapping and one updater owner; game-manifest trust remains separate. |
| `prototype-packaging/model/`, `tauri/app.ts` | Reference visual and test scenarios only. The mock install/repair/uninstall status and Promise chain are not real orchestration or persistent state. |

Repair and uninstall need separately specified file ownership, confirmation and
recovery contracts. Keep them unavailable or explicitly marked unsupported
until implemented; never report a successful repair from an unchanged mock.

## Architecture and ownership

Under the recommended reuse design, the Rust engine owns install truth, durable
operation state, filesystem mutation,
consent persistence and game process observation. The Tauri bridge exposes a
small versioned command/snapshot/event contract. Effect owns application workflow
coordination through that contract; views own tab/modal/focus state only.

| Layer | Responsibilities | Must not do |
|---|---|---|
| Rust engine | Inspect actual state; validate intents; serialize conflicting operations; run install/launch; persist results; emit bounded observations | Trust a frontend's claimed installation or readiness state |
| Tauri adapter | Validate input and schema/version; constrain paths/operations; map engine errors; return authoritative snapshots | Expose an unrestricted shell/filesystem command or accept arbitrary telemetry fields |
| Effect services/workflows | Admit user intents, manage subscriptions, sequence dependent steps, classify typed failures, reconcile after reconnect, bound permitted retries and finalize resources | Reimplement patch application, convert timeout into rollback success, or mark an operation cancelled merely because a fiber stopped |
| View | Render snapshots, immediate acknowledgement and actionable errors; preserve keyboard focus | Increment synthetic install progress or turn process creation into login success |

Define service interfaces for the engine bridge, snapshot stream, preferences,
clock and consent-aware summary exporter. Supply real and test implementations.
An Effect install workflow must issue a real engine operation, subscribe to that
operation's state and finish from its terminal result. Effect must be used beyond
wrapping `invoke` in a renamed Promise.

Keep resource scopes tied to application/runtime lifetime, not transient DOM
mounts. Disposing a view unsubscribes that view; it must not silently abort an
installation. Explicit cancellation calls the engine with an operation ID and
waits for authoritative acknowledgement/termination. If the connection drops,
query the current snapshot before allowing a conflicting operation or retry.

Retry read-only snapshot/manifest queries with bounded backoff where safe.
Retry a mutation only with a defined idempotency key and engine guarantee; never
blindly replay Install, Uninstall or Launch because IPC timed out. A new launch
request after an uncertain result must first reconcile the running process.

Choose and pin the Effect release with schema/runtime/test dependencies together.
Use its typed errors, services, scopes, interruption and test clock against real
workflows; verify APIs against the pinned documentation before implementation.
The plan initially installed no dependency. The implementation ledger below
records the later pinned Effect foundation; full game workflows remain pending.
Effect's [scope guidance](https://effect.website/docs/v3/resource-management/scope/)
supports scoped acquisition/finalization; keep acquisition short rather than
putting a download in an uninterruptible registration step. Its
[tracing guidance](https://effect.website/docs/v3/observability/tracing/) provides
`withSpan`, but local spans do not become server-side phase spans by exporting
summary logs. Both documentation major-version routes exist; choose a compatible
stable version explicitly rather than assuming the latest route is the selected
release. Interrupting an `invoke` Promise does not cancel Rust work: send the
explicit cancel command, reconcile acknowledgement and retain the native journal.

## Honest operation states

Use one operation ID per user attempt, stable across transport retries, with a
revision/sequence to reject stale snapshots. Starting, running, cancel-requested,
succeeded, failed and cancelled have distinct meanings. A restart may require
reconciliation; do not invent a terminal result when observation was lost.

Install phases describe actual engine checkpoints: prerequisite inspection,
manifest verification, download, content verification, extraction, patch
application and final state validation. Progress estimates are presentation
only. Write “installed” only after the actual required files/ledger checks pass.

Launch phases describe preparation, helper invocation, injection and process
observation. `process_started` means exactly that. It does **not** mean login
screen visible, authenticated, world entered or server ready. Until a separately
validated game signal exists, display “Game started” and leave login/world
confirmation to human UAT. Missing observation is unknown, not failure or success.

## Focused telemetry: one bounded summary flow

### Existing gap

`crates/launcher/src/telemetry/install_result.rs` queues an install-result event
only when opted in. Its own contract says the queued event uploads with the next
game telemetry session. A tester whose installation prevents any game launch
therefore has a **failure-before-game visibility gap**.

The event currently uses `ClientNative`. On the server,
`crates/admin-api/src/routes/telemetry/replay.rs` replays that envelope at the
static `client.native` target into `cimmeria-client` logs, carrying the source
event name in fields. Those are structured log records, **not native tracing
spans**. Do not draw a trace waterfall or claim span parentage from that route.

### Proposed contract

Add one launcher-specific operation-summary flow independent of game sessions.
Use a typed envelope for a bounded operation result and bounded phase summaries;
the same source of truth feeds local status and the opted-in export projection.
Do not add parallel JS, Rust and DLL exports of the same operation.

Define an explicit server ingestion/auth scope for launcher-only summaries, or
prove an existing auth route can safely serve that purpose without starting the
game pipeline. This is a design/implementation task, not an existing capability.
The current dev-session mint is credential-free with self-asserted install
identity, a scoped HMAC token and quotas; it does not authenticate a person's
identity or make client-reported metrics trustworthy. Its required `machine_id`
field needs either a tested protocol-compatible random consent-scoped value or
versioned optional metadata. Do not add a hardware identifier for this scope.
Validate client-supplied IDs and do not trust them as arbitrary trace parents.
A missing token, unavailable endpoint or disabled exporter must never block
install or launch. No direct collector credentials belong in the webview.

Proposed starting budgets: one terminal summary plus at most 32 phase-boundary
summaries per attempt, no more than 2 KiB per event; a queue bounded to 256 events
and 512 KiB with a 24-hour TTL; export timeout of two seconds and at most two
retries with jitter and bounded `Retry-After` handling. These are tuning starting
points, not measured requirements. Fix phase/outcome/error enums in the schema.
Coalesce retries into attempt counts rather than emitting a row for every byte
or poll; record aggregate drop counts without recursive per-drop logging.

Allowed fields: schema version, random operation/event ID, launcher version,
OS/architecture enum, operation/phase enum, terminal outcome, monotonic duration,
coarse byte/count totals, retry count and a reviewed error code. Include a
manifest release identifier only through a bounded validator. A random opaque
installation correlation ID, if needed, requires an explicit retention rationale;
it must not become a username or a metric label.

Exclude raw logs, free-form errors, tokens, account/character names, IPs, server
URLs, command lines, absolute paths, directory listings and machine identifiers.
Map errors to allowlisted codes locally. A string truncated to 512 characters is
bounded, not necessarily safe: do not copy the existing report's error/path bag
straight into the new summary schema. Keep detailed local diagnostics separate
from automatic export and from the existing explicit Upload Debug Logs action.

Use event IDs for ingestion deduplication; never claim exactly-once transport.
Use an acknowledged queue with the bounded policy above and explicit
permanent-error drops. The existing telemetry queue has a much larger budget
and removes drained data before upload; do not copy that delivery behavior into
the new summary flow without acknowledgement/recovery semantics. Consent is checked at creation, enqueue and immediately before send.
The launcher must remain usable with the exporter absent or broken.

### Consent changes

Default off. Persist the user's choice before reporting it saved, and show a
local error if persistence fails. On opt-out, stop new launcher summary creation,
close the export gate, cancel pending sends where possible and delete unsent
launcher-summary entries. Recheck consent after token acquisition/backoff so an
old task cannot send after a later opt-out. Never retroactively export activity
collected while off when consent is enabled later.

An already-sent request cannot be recalled; do not promise retroactive removal.
The existing injected-DLL/game telemetry choice is configured at launch, and
current UI copy says changes apply on the next launch. Immediate cancellation
of the **new launcher exporter** must not be described as unloading an already
injected DLL or stopping every existing game session. Explain both boundaries
in consent copy and test them separately. Current code persists a Boolean while
workers hold launch-time configuration snapshots; immediate revocation of that
older pipeline is not established. Narrow launcher-summary consent must never
silently enable broad game logs or injected telemetry. Any shared-toggle migration must
preserve the saved opt-in choice and make the expanded data description clear.

### Minimal operator view

Use structured logs first through the existing server pipeline. Its current
`LiftedFields` does not expose operation/phase/outcome/duration fields: add a
typed validated variant and allowlisted lifting with replay tests before writing
queries against those columns. Aggregate only accepted, deduplicated summaries;
transport retries must not inflate attempt counts. Server receipt/OTLP spans are not the original
launcher phase spans.

Deliver a version-controlled dashboard definition/query fixture, not a live
probe or dashboard deployment. Keep the initial view to:

- Observed attempt counts and terminal outcomes by launcher version and OS.
- Install/launch failure counts by phase and reviewed error code.
- Phase duration distributions with sample counts.
- Ingest/export health where independently observable, with dedup/drop counts.

These describe opted-in, successfully received attempts. They are not an
all-player funnel, an installation success rate for non-reporting users or a
login success metric. No per-file series or per-operation metric labels. If a
failure prevents export, the dashboard cannot count it without another signal;
show that limitation rather than inferring missing users are healthy. Keep
unknown/abandoned attempts distinct from failures. No real SLO claim follows
from this initial, self-reported cohort.

## PR-sized delivery packets

Each runtime packet adds meaningful tests under TESTING.md and updates its
corresponding docs in the same PR. Keep existing egui available until parity.
Every meaningful frontend packet includes JS REPL-style UAT of actual workflow
logic plus a separate visual pass and explicit coverage exclusions.

| Packet | Deliverable and boundary | Required evidence before merge |
|---|---|---|
| 1. Engine contract extraction | Move configuration/preparation ownership behind Rust intents/snapshots; preserve existing egui behavior; version DTOs | Existing install fixtures retain outcomes; stale snapshot and conflicting-operation tests; no direct preparation work on UI thread; launcher design/reference docs |
| 2. Tauri + Effect foundation | Pin dependencies; real/test services; scoped runtime/subscription; typed bridge; read-only inspection and settings persistence | Schema mismatch, IPC rejection, reconnect, scope cleanup and save-failure tests; JS UAT of real services with controlled adapter; keyboard/visual inspection; frontend architecture docs |
| 3. Mac runtime/prerequisite adapter | Define managed-prefix ownership, pinned runtime source and verified license/redistribution/provisioning; Rosetta system/human requirements; legacy VC++/PhysX/DirectX prerequisites; FDI-compatible archive worker; Windows helper inside Wine with guest/host lifecycle tracking | Fixture-based adapter contracts, corrupted/missing runtime detection, prerequisite failures and helper lifetime tests; no automatic WoWSilicon download promise until redistribution/provisioning is verified; Mac setup/recovery docs |
| 4. Real install workflow | Effect coordinates authoritative install; Rust keeps download/hash/extract/patch logic; explicit cancellation/recovery | Corrupt content, interrupted transfer, partial patch failure, disk error, cancellation and restart fixtures; double-click/timeout does not create duplicate mutation; state/persistence JS UAT; user install guide |
| 5. Real launch workflow | Preparation and Windows helper adapter; observe lifecycle without invented readiness | Helper failure, injection failure, early exit, duplicate launch and lost-observation tests; guest/host PID contract checks; JS UAT for observed states; launch guide; human Mac game UAT remains separately reported |
| 6. Summary schema and consent | Typed safe projection, local queue and immediate launcher-export opt-out gate; no new live backend dependency yet | Seed secrets/paths in errors and prove exclusion; budgets, TTL, dedup keys, off-by-default, opt-out/send race, restart and exporter-failure isolation tests; consent docs |
| 7. Ingest and operator fixtures | Launcher-only auth/ingestion contract, bounded validation, structured summary replay and minimal dashboard fixture | Local endpoint tests for auth scope, unknown schema, size/rate limits, duplicate events and failure-before-game delivery; negative-log coverage; telemetry architecture/operator docs; no live probes |
| 8. Migration and parity | Adopt existing config/installed state; updater ownership and release-version mapping; replace mock-only affordances | Existing-user fixtures, no unwanted identity/consent reset, single updater owner, rollback/failure cases and installed/update/launch parity; migration/release docs |
| 9. Self-contained startup LAST | Final per-OS packages, offline first open, runtime/dependency inspection, clean-machine and signing checks | Mac packaged-webview visual/accessibility UAT; Windows-native build with bundled fixed WebView2 and verified CRT strategy; no dev server/runtime download on first open; explicit game-versus-launcher evidence; release checklist |

Packets can be split further if their review surface grows. Do not merge a
packet that advertises later unimplemented operations as functional. The final
self-contained startup gate is deferred, not waived; an unresolved Windows host
or signing prerequisite blocks release rather than silently reducing the claim.

## Validation and documentation discipline

Use deterministic clocks and controlled adapter failures for Effect retries and
cancellation. Test the production workflow graph, not a JS reimplementation of
Rust installation. Rust tests cover mutation/recovery truth; contract tests pin
serialized snapshots/errors; JS UAT verifies the actual Effect program consumes
them and preserves operation/consent state. Packaged UI tests cover the webview
and IPC; screenshots alone cover none of the durability claims.

Keep native platform/process checks distinct from offline fixture results. The
Mac login screen, world entry and Black Market behavior remain human UAT until
observed. No self-contained or production-readiness claim is justified before
the final gate passes on the intended operating systems.

Use [the doc-update map](../../../agents/doc-update-map.md): launcher design and
player guide for workflow changes; telemetry architecture and operations docs
for schema/auth/consent/retention; platform guides for packaging. Update indices
and project memory with each packet. Add an architecture decision only when the
contracts are reviewed; this plan records the requested direction and remaining
work, not an implemented system.

## Implementation ledger

### 2026-10-04: operation contract (packet 1, partial)

Added the standalone [desktop engine scaffold](../../../../crates/launcher/desktop/README.md)
with versioned snapshots, current-operation retry handling, revision checks,
cancellation acknowledgement states and an injected journal contract. Nine
Rust tests, strict clippy and formatting passed on macOS through the build lane
(compilation only). No existing egui configuration/preparation ownership has
been extracted yet. No concrete file journal, real mutation worker, Tauri IPC
or Effect service is connected. Next: durable journal/process ownership and
settings, then the Effect bridge and worker integration.

The tests use an in-memory journal. They do not prove crash durability, native
worker dispatch, cross-process ownership or Windows behavior. No frontend
changed, so no JS/visual UAT was run for this packet. Production release gates
and final self-contained startup validation remain open.

### 2026-10-04: native storage and Effect foundation (packets 1/2 partial)

Added bounded disk journals, exclusive state-directory ownership, synced
replacement and uncertain-commit handling. Native preferences persist an install
path and separate default-off launcher-summary consent. Effect 4.0.0 now owns
inspection/save sequencing, typed IPC validation, bounded read retries, stale
snapshot rejection, uncertain-save reconciliation and scoped bounded updates.
No save is automatically replayed after timeout or interruption.

Validation on macOS: 23 Rust tests, eight Effect tests, strict all-target clippy,
TypeScript and formatting passed. The single ignored Rust case is a subprocess
fixture invoked by its parent. Sequential JS logic UAT exercised the actual
Effect services and Rust handler across process restarts, verifying saved path,
opt-in and opt-out persistence. A native Mac/Windows CI workflow now covers the
standalone workspace explicitly; inspect its run before claiming Windows passed.

Not covered: native webview/visual UAT (window not connected), power loss or
kill-during-write, real game install/launch, Wine helper ownership, live summary
export or self-contained first open. The child-kill test kills an idle process
after its journal writes have finished. Next: connect the Tauri application
with native app-data selection and settings UI, then actual installer workers.
The settings store is new and does not yet migrate the egui configuration.

### 2026-10-04: Tauri settings shell (packet 2, partial)

Connected the application-scoped Effect runtime to Tauri commands backed by
native app-data selection and a lazily locked store. Added approved dark-only
Play/Patch Notes tabs, gear settings, persisted diagnostics choice, a parented
folder chooser and saved-folder-only reveal. Game actions remain disabled and
patch notes explicitly identify missing manifest integration. Mac native shell
compilation, two host tests and clippy passed; frontend tests total 12.

Headless JS UAT now drives a DOM checkbox through Effect, the Rust harness and
disk, then confirms persistence after restart. Chooser cancellation is mocked.
Visual, keyboard, real-dialog and actual Tauri IPC UAT remain open; no desktop
window was opened during automated checks. The earlier engine CI run
37181383914 passed on Windows and macOS at `69fc13d3f`, before this shell.
CI now also builds/tests the native shell on both platforms. The final
self-contained startup gate remains deferred, not waived.

The uncertainty regression guard was mutation-checked after the storage commit:
disabling its reopen gate made the test fail; restoring it made the same test
pass. This proves that guard detects loss of the gate, not complete recovery
correctness. Next: signed release-manifest integration and actual game workers.

### 2026-10-04: Windows shell resource correction

CI run `37182338053` passed the Mac shell build, tests and clippy. Windows
passed engine tests and JS persistence UAT, then failed the shell build script:
`icons/icon.ico` was missing. Added the existing launcher ICO to the standalone
shell and listed both icon formats explicitly. Native Windows validation of
this correction is pending the next run; no cross-compilation was attempted.

### 2026-10-04: signed release patch notes

The Windows icon correction passed native Windows and Mac shell CI in run
`37182711152` at `aaf00cb9e`. Added a bounded catalog loader sharing the existing
manifest schema/signature policy, notes-only IPC and lazy Effect tab loading.
Refresh is explicit; failures preserve and label previously verified session
notes. Titles/descriptions render as text. No installed-state claim is made.

Local validation: 52 engine tests, two shell-host tests, 14 frontend tests,
clippy, TypeScript and JS logic UAT passed. Catalog UAT renders a response fixture
while persisted consent stays unchanged; it does not test cryptography or actual
Tauri routing. Rust tests separately cover signed/tampered fixtures, malformed
signatures, schema errors, and bounded declared/chunked HTTP bodies. A read-only
live probe using the release public key recorded in the handoff authenticated
seven patches from `content-current`. Production key injection, native visual
UAT and final packaging gates remain open. No desktop application was opened.
Next: actual installer/runtime adapters and operation reconciliation.

A second headless JS pass rendered the live probe’s verified response through
the actual Effect view: seven entries, expected first title and consent still
off. It bypassed Tauri routing and did not open a window. The Mac executable
compiled with the recorded release public key; no new packaged UI UAT occurred.
Read-only advisor review found no actionable pre-commit issues. Manifest
freshness/rollback protection remains inherited and must be addressed before
using catalog availability as an update-readiness decision.

### 2026-10-04: shared installer algorithms and runtime evidence

Imported the existing installation/unpack/preparation modules into the desktop
engine without copying algorithms; delta application uses `cimmeria-patchset`.
Added a one-value progress sink for desktop workers while retaining the existing
egui event stream. A shared fixture now drives HTTP seed download, SHA-256,
ZIP extraction, overlay patching, PE/client preparation, persisted installed state
and an idempotent second pass with no new blob requests. No UI install action or
managed worker is connected yet. Legacy success without SGW.exe and permissive
installed-state reads must not be used as readiness or ownership evidence.

Mac engine validation: 132 tests pass; ignored cases are the parent-invoked child
fixture and two manual real-client checks, still unrun. Strict workspace clippy
passes. Catalog CI `37183173769` passed both native platforms at `f8ea7b844`;
shared-installer validation awaits its own run. Native Windows CI now checks the
existing egui launcher too because the progress interface is shared.

Primary-source research is recorded in [runtime-provisioning.md](runtime-provisioning.md).
It identifies the pinned Wine revision and separates D3D9/x87 resources from
that archive. A narrow Windows FDI helper under an exclusive managed prefix is
the recommended first cabinet path; redistribution inventory, legacy prerequisite
packages and fresh-prefix/game UAT remain gates. No runtime was installed and
no desktop application was opened. Next: helper protocol, validated native
worker admission and filesystem/process reconciliation.

### 2026-10-04: archive helper transport

Added a Windows-only operational archive-helper executable and shared bounded
NDJSON transport. It authenticates the local archive hash before creating a new
staging destination, refuses overlays, and retains partial output for recovery.
The Windows verification handle denies file write/delete until extraction ends.
Cancellation is cooperative; stdin EOF/malformed controls cancel, and late
cancellation may lose to completion. A one-slot writer isolates extraction from
stdout stalls; terminal delivery has a two-second budget after extraction exits.

Mac engine tests: 140 passed, three default ignores as previously described;
strict all-target clippy passed. Portable tests exercise real control-reader
logic and injected stalled/broken writers. Windows sharing and real-process
stdio tests await native CI; no Windows cross-compile, Wine invocation, real
archive extraction or new shell-host/visual UAT occurred. CI retains the helper
as a seven-day debug artifact for later supervised validation. Installation
remains disabled until parent ownership, deadlines, staging promotion and
reconciliation are connected. The helper is not a game launch worker.


### 2026-10-04: native helper supervision

Added native-owned process configuration and a host-PID recording callback before
request dispatch. Protocol/progress observations and write, operation,
cancellation and exit deadlines are bounded. Completion requires matching
terminal identity, process exit and EOF; uncertain observations retain the
reconciliation requirement. Cancellation writes obey the active deadline and
cleanup uses only its remaining grace.

The real-stdio harness exercises twelve scenarios, including ownership refusal,
cooperative/unresponsive cancellation, malformed lifecycle results, progress
flooding and direct-child cleanup after supervisor abort. A blocked-pipe unit
regression covers the cancellation deadline. Native Windows real-worker
supervisor integration awaits CI. Production journal callback and coordinator
wiring remain pending. No GUI, Wine or frontend behavior changed; no Wine guest
death or installation-readiness claim is made. Installation stays disabled and
the final startup gate remains open.

Validation: 142 engine tests and all twelve process-harness scenarios passed on
macOS through the build lane; strict all-target clippy passed. Three default
ignores retain the previously documented parent-fixture/real-client distinction.


### 2026-10-04: verified release identity and durable install admission

Added `VerifiedRelease`, binding validated manifest data to its original signed
byte digest, and native admission that persists immutable install inputs before
operation ownership. Per-operation intent filenames preserve previous recovery
evidence when the next admission fails between writes. First-install destinations
must be missing or empty with an existing canonical parent, and cannot overlap
launcher state. These checks do not reserve the filesystem.

Fixtures exercise evidence preservation, rejection paths, restart without replay,
identical retries after consent changes, and interrupted replacement admission.
Admission does not dispatch filesystem work or create game files. Ownership,
worker integration, readiness, reconciliation and UI installation remain pending.
No frontend behavior changed, so frontend JS REPL/visual UAT does not apply.
Archive-helper CI run `37184586437` passed both native platforms at `e96dfeaf3`;
the later supervisor commit `04b03a158` has its own CI run `37185555998`.

Local admission packet checks: 152 engine tests and the twelve-scenario process
harness passed; strict all-target clippy and formatting passed. Native Windows
validation for these new admission cases remains pending.
