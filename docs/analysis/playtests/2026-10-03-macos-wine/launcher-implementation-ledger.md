# Desktop launcher implementation ledger

> **Type:** Reference
> **Audience:** Launcher contributors and reviewers
> **Last updated:** 2026-10-04
> **Companions:** [Requirements and delivery plan](launcher-implementation-plan.md), [campaign handoff](README.md), [runtime evidence](runtime-provisioning.md), [desktop contracts](../../../../crates/launcher/desktop/README.md)

These dated entries preserve what each packet established at its exact revision.
Older pending statements are historical; later entries record subsequent results.
Requirements and release gates stay in the implementation plan.

## Implementation packets

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

### 2026-10-04: native first-install content worker

Connected durable admission to a native-owned task that commits `running`,
claims/locks a fresh destination, installs into operation-specific staging,
checks content evidence, then promotes `<selected-directory>/game` and writes
a receipt before publishing success. Fixtures exercise observer disposal,
duplicate dispatch, invalid content, changed destinations and receipt uncertainty.
Review confirmed the corrected extraction cancellation mapping; a deterministic
checkpoint fixture retains the first extracted file, skips the next entry,
persists `Cancelled` and never promotes content. A stalled-download regression
also exposed network cancellation waiting for bytes; both HTTP-header and body
waits now observe cancellation in the shared Windows/desktop downloader.

Failed/cancelled output remains for future reconciliation; retry and recovery
are not implemented. No frontend changed, so JS REPL/visual UAT does not apply.
UI installation, the Wine cabinet adapter, runtime prerequisites and gameplay
validation remain pending. Content checks are not a full extracted-file audit
or power-loss proof. No GUI, real game archive, Wine process or live telemetry
was used.

Worker packet validation: 160 engine tests plus twelve process scenarios passed
on macOS; strict all-target clippy and root/desktop formatting passed. Native
Windows worker validation remains pending.

### 2026-10-04: interrupted-content reconciliation

Added native inspection for operations awaiting reconciliation. Missing/empty
output commits failure without creating/removing content. Matching intent,
verified release, ownership marker, acquired lock, receipt and current content
checks permit content-prepared success. Partial/conflicting evidence never
permits replay. Resolved executable and game paths must stay in the content root.
Marker decoding uses the locked handle rather than reopening it, avoiding
Windows lock conflicts. Fixtures cover persisted recovery, active ownership,
partial/missing content, foreign markers and Unix redirection.

168 engine tests and twelve process scenarios passed on macOS; strict all-target
clippy and formatting passed. Native Windows recovery checks remain pending.
No downloads, resume, cleanup or frontend changes were added. Exact authenticated
release input is still required; offline signed-release caching remains open.
Wine guest lifecycle and runtime/gameplay readiness remain separate gates.

### 2026-10-04: persisted signed release evidence

Admission now saves bounded per-operation original manifest/signature bytes
before intent and operation commits. Recovery can reverify those bytes under
the current signing policy and bind them to the saved intent digest without
relying on the mutable release URL. Fixtures cover exact-byte preservation
across restart, verification/identity failures, failed writes preventing
admission and malformed/oversized/missing files. Invalid or absent evidence
never triggers a replacement fetch. Key rotation and evidence retention policy
remain explicit concerns; no trust-policy bypass was added.

172 engine tests and twelve process scenarios passed locally; strict all-target
clippy and formatting passed. Native Windows validation remains pending.
Automatic resume and frontend installation remain open. No frontend behavior
changed, so JS REPL/visual UAT does not apply to this packet.

### 2026-10-04: explicit interrupted-install resume

Added native resume requiring current operation ID/revision and reconciliation
state. It reverifies cached release evidence, locks/validates owned staging,
rejects promoted output, then commits Running before continuing the pipeline.
Fixtures exercise Range continuation through promotion, missing staging,
failed journal commits, stale duplicates and refusal of unsafe/corrupt content.
No restart replay or frontend dispatch was added. Terminal cancelled/failed
retries and Wine guest reconciliation remain open; not every extraction/patch
interruption is covered by these fixtures.

178 engine tests and twelve process scenarios passed locally; strict clippy and
formatting passed. Native Windows resume validation remains pending. Repeated
milestone pushes cancelled preceding native runs, so desktop CI now retains
active work and queues the latest pending revision. This changes scheduling,
not what checks run or which commit their results validate.

### 2026-10-04: restricted native installation IPC

Added versioned inspect/install/cancel/resume/reconcile commands around shared
native state and the retained worker. Native code owns release selection and
login-server inputs. Identical current-operation retries use reverified cached
evidence before considering network access; corrupt/mismatched evidence fails
without a replacement fetch. Dispatch failure after admission attempts immediate
reconciliation gating. Successful reconciliation clears old worker observations.
Progress exposes static phases/counts and errors use flat safe codes.

Thirteen shell tests passed locally. No frontend behavior changed: game controls
remain disabled and unconnected, so frontend JS REPL/visual UAT does not apply.
Mac install/resume is rejected before downloads until the Wine adapter exists.
Actual Tauri interaction, runtime/game readiness and final startup remain gates.
Prior native CI `37187754913` passed both platforms at `117344e76`. Resume CI
`37188326146` at `bf8029e28` subsequently passed both platforms. Shell CI
`37189445603` at `17b949f4c` passed macOS and Windows. These runs do not validate the
newer frontend installation controls.

### 2026-10-04: Effect installation controls

Connected Windows installation, cancellation and interrupted-operation
inspection/resume to the restricted native IPC. Effect refreshes state before
mutations, never automatically replays writes, bounds transport-only read retries
and permits cancellation between observation polls. Scope disposal stops
observation without cancelling native work; uncertain persistence stops polling
and requires restart. Mac install/resume stays disabled. Success is labelled
content prepared, while terminal failed/cancelled retry, cleanup and Play remain
unavailable.

Twenty-three frontend tests, TypeScript checking and the frontend build passed.
`npm run uat:install` passed sequential actual-DOM/Effect fixture checks for
install/progress, cancel-requested state, disposal/reconnection without replay,
completion winning cancellation, and no Play-readiness or consent inference.
Native installation IPC is mocked; filesystem installation, Wine, visual layout
and gameplay are not covered. The separate Rust `state_bridge` settings UAT
also passed its disk/restart checks. CI now runs the installation logic UAT. The approved settings preview predates these changes; native
visual/keyboard inspection and actual Tauri installation IPC are still unverified.
No game/runtime readiness or final startup validation is claimed.

### 2026-10-04: seed extraction adapter boundary

Added `install_all_with_seed_extractor` with a separate native-owned download
cache and fresh content destination. Shared code authenticates the archive before
adapter dispatch; patch overlays, preparation and ledger transitions retain their
existing path. Uncertain extraction preserves input/partial output and requires
reconciliation. Existing `install_all` callers are unchanged and no production
caller uses the adapter yet. No Wine runtime was selected or invoked.

The engine suite passed 181 tests, including three new seed tests. The enhanced
seed subset also passed native-overlay/reuse checks. Strict all-target engine
clippy and root formatting passed. Native backend/cache ownership, durable process identity, helper/Wine
invocation and backend-aware recovery remain integration gates. No frontend
behavior changed; frontend JS REPL/visual UAT does not apply to this packet.

### 2026-10-04: pinned managed Mac runtime cache

Added macOS cache preparation with native path/OS-lock ownership, bounded HTTPS
download, pinned archive verification, blocking staged tar extraction and
canonical full-tree validation before publication and reuse. Existing damaged
caches are preserved. Cancellation interrupts downloads; during extraction the
blocking task retains staging/lock ownership and checks cancellation before
publication. The explicitly run external-archive smoke passed extraction/tree
verification without executing Wine. Eight ordinary runtime tests are included in the 189 passing engine tests;
four entries are ignored in the ordinary run and the runtime-archive smoke
passed separately. Strict all-target engine clippy passed after the test-only read-count fix. No production
caller, prefix, Wine execution or game prerequisites are connected. Licensing,
compatibility and final startup gates remain open; no frontend behavior changed.

Native CI update: shell run `37189445603` passed both platforms at `17b949f4c`.
The pending frontend run for `9947a1003` was superseded/cancelled; seed run
`37190303728` at `cd7366c61`, which also contains that frontend work, is running.
These earlier revisions do not validate the managed runtime-cache packet.

### 2026-10-04: extraction identity and durable helper checkpoints

Bound native/Wine backend selection into immutable install intent, preserving
legacy digests by omitting the default native value. Wine identity includes
runtime/helper hashes; native dispatch/resume/recovery rejects those intents
rather than falling back after restart. Added operation/attempt-bound helper
launch, host-started and finished records, retaining PID across uncertain finish.
The owned supervisor wrapper commits checkpoints around spawn, request delivery
and outcome return; journal failure requires caller reconciliation.

The engine suite passed 195 tests before the wrapper addition; five journal tests
passed after PID-retention correction. All thirteen real-stdio scenarios passed,
including the owned-checkpoint fixture; strict clippy and formatting passed. No production Wine adapter, prefix or frontend change was added;
PID records do not prove guest death or permit automatic redispatch.

### 2026-10-04: experimental headless Wine seed adapter

Added an experimental adapter with pinned runtime/helper identity checks,
Rosetta probing, private prefix ownership, guest-path mapping and durable helper
supervision. It is not wired into shell installation; Mac remains disabled.
Five ordinary tests passed. The explicitly run Windows-helper ZIP smoke failed
after approximately 307 seconds and persisted `HelperResult::Uncertain`; no
successful extraction was established. No Wine processes remained after adapter
cleanup. Cause is undiagnosed; concurrent duplicate-ownership coverage is being
added. No frontend changed or frontend UAT was rerun. Helper/cabinet compatibility,
production coordination, prerequisites and game readiness remain open.

Wine smoke follow-up: a minimal diagnostic isolated the missing C-drive/system32
path. Adding `drive_c` and `dosdevices/c:` → `../drive_c` changed the diagnostic
to successful terminal/extraction; the original full managed-runtime Windows CI
helper ZIP smoke then passed in 19.574 seconds. The C-drive unit regression was
observed red before the fix. Six ordinary adapter tests include concurrent
ownership. After the final mapping correction, 201 library tests passed with
five ignored entries and strict all-target engine clippy passed. Final process
inspection found no test Wineboot/wineserver processes. The earlier uncertain failure is historical, not
the current smoke outcome. Actual client RAR/FDI chains and gameplay remain
unverified. Seed CI `37190303728` passed macOS and Windows at `cd7366c61`; that
revision does not validate the newer Wine adapter.

The desktop reference now separates [Wine/runtime/helper contracts and smoke
commands](../../../../crates/launcher/desktop/docs/wine-validation.md) from its
README. Latest ordinary checks passed 201 library tests with six ignored entries
and strict clippy; the real-archive smoke result remains pending.

### 2026-10-04: original client RAR through managed Wine passed

The supervised real-archive smoke passed in 299.64 seconds (301.146 seconds lane
elapsed). It authenticated the signed manifest, verified the 4,135,724,034-byte
source and its SHA-256, then used the Windows debug helper from `17b949f4c` under
the managed runtime/private prefix. Final cabinet progress was 5,983/5,983;
assertions found the expected MZ executable and SGWGame directory, no remaining
`.tmp-unpack`, and durable helper completion. The test tree auto-cleaned.
Hashes/key and command are recorded in the [validation reference](../../../../crates/launcher/desktop/docs/wine-validation.md#real-client-archive-evidence--2026-10-04).

This passes the original RAR/chained-cabinet extraction smoke, not patching,
prerequisites, launch, login or gameplay. The debug-helper duration is not a
release performance benchmark; Mac shell installation remains disabled.
Helper-journal CI `37191310279` passed both platforms at `ef10c31f9`; that earlier
revision does not validate subsequent Wine changes.

### 2026-10-04: retained native Wine worker integration

Added `dispatch_wine`: validate durable backend/runtime/helper identity before
Running, claim the destination, prepare Wine and run the shared seed adapter,
then apply native patches/client setup and publish checked content/receipt.
The retained task survives observer disposal. RosettaRequired/RuntimeUnavailable
outcomes now decode to frontend messages without altering diagnostics consent.

The explicit retained-worker fixture passed in 22.596 seconds with Wine seed,
native ZIP patch and receipt after observer drop. Both strengthened ignored checks passed in a 19.090-second run: duplicate
rejection and cancellation before runtime cache/prefix/helper/network. Strict
engine clippy and thirteen shell tests passed; final engine suite passed 204
tests with eight ignored entries. Frontend tests 25, check/build and sequential
logic UAT passed. No visual UAT was performed. Mac shell install remains
disabled pending
trusted resource binding; native Wine recovery remains rejected. No end-user
Mac-install or game-readiness claim follows from this native API.

### 2026-10-04: packaged Windows helper binding for Mac content installation

Added guarded artifact staging and fixed-resource resolution against a compiled
helper digest. Hash/PE checks precede staging; its receipt is provenance only.
The shell verifies before enabling Mac installation and again before fetch/admit,
then records immutable backend identity. Missing resources retain settings/notes
only. Capability flags hide unsupported Wine recovery and keep recheck read-only.

The development Mac bundle contains the expected Windows CI helper. It was not
opened. Staged resolver/admission/cancel passed in 1.961 seconds; staging guards,
26 frontend tests, check/build and sequential logic UAT passed. Final native checks passed 205 engine tests/eight ignored, 15 shell tests/one
ignored, and strict engine/shell clippy. The ignored resource smoke passed
separately. Final development bundling and embedded-helper hash verification
passed; packaged permission verification remains pending. No visual UAT or
final self-contained startup occurred. Wine recovery and gameplay remain open.

### 2026-10-04: durable installation outcome reporting

Added bounded schema-1 `install-result.json`, written before terminal commit and
bound to operation ID, intent digest and exact terminal revision. Active/recovery
or uncertain-reopen state hides the record; mismatched historical revisions stay
hidden and legacy missing results are allowed. Shell status now preserves the
confirmed reason across host reopen. Fifteen shell tests passed with one ignored,
including persisted InstallFailed after host drop/reopen. Final engine checks
passed 209 tests with eight ignored entries; combined strict clippy and
`npm run uat:install` passed. JS installation IPC remains mocked, so this adds no
native filesystem/Wine or visual UAT evidence. The existing Windows launcher
exports the shared install module publicly to resolve unused/dead-code lint
failures reported by CI `37194260466`; native Windows validation awaits the next
run. Wine recovery and terminal-attempt retry remain unsupported.

### 2026-10-04: conservative Mac Wine reconciliation

Added recovery inspection for absent helper records or observed finished results
with recorded host absence. Canonical intent-owned prefix and verified cached
runtime stay locked through bounded prefix stop/wait, content inspection and the
journal decision. Ambiguous launch/host/uncertain records remain gated; no PID
kill, download, deletion, shared-profile access or resume was added. Mac exposes
reconciliation capability while Wine resume remains false.

The real ZIP recovery smoke passed in 22.685 seconds after state reopen, retaining
output and unresolved recovery state. Final checks passed 212 engine tests/eight
ignored, 15 shell tests/one ignored and combined strict clippy. JS logic UAT
passed enabled inspection without resume or inferred success; native IPC remains
mocked. No native visual/gameplay proof is claimed.

### 2026-10-04: confirmed failed-attempt cleanup and retry

Added confirmed CleanFailed IPC bound to ID/revision and terminal failed/cancelled
install state. Canonical ownership lock and complete name/type/tree preflight
veto promoted content, receipts, foreign files, symlinks and reparse points before
deletion. Stage/cache are removed before the marker; runtime cache/prefixes stay.
The destination remains empty and old terminal state persists. Explicit repeat
after lost-reply inspection is safe; retry separately creates a fresh UUID.
Selecting another empty folder preserves old output. Wine stop restrictions stay.

Engine 217 ordinary tests and shell 16/one ignored passed; engine now has nine opt-in tests. Frontend 26 tests,
check/build and sequential logic UAT passed confirmation/dismissal, one cleanup,
new-ID retry and consent preservation. Combined strict clippy passed. Windows junction
regression awaits native validation. Real failed-Wine cleanup passed in 24.096 seconds,
including helper completion, content rejection, empty destination and retry readiness.
Native visual UAT was not exercised. Reconcile/cleanup observation timeout is now 35 seconds, reads remain five seconds.

Earlier desktop CI `37194260496` passed macOS and Windows at `ba765f95`. The
new cleanup revision awaits its own CI; the older result does not validate it.

### 2026-10-04: initial folder preference and authenticated seed-cache reuse

Untouched revision-zero preferences without an operation now receive the native
app-local-data/Stargate Worlds path once, with diagnostics consent off. No game folder,
download or installation is started; saved/cleared choices remain unchanged.
Shared seed extraction reuses complete-size SHA-authenticated input before HTTP;
confirmed complete hash mismatch removes the cache before fresh HTTP without
Range, while incomplete downloads retain ordinary Range handling. The fixture
passed zero-request valid reuse and one-request invalid-cache replacement.

A real signed-release full-content smoke is running: original client RAR, seven
patches, client setup and promotion. Its outcome remains pending; prior extraction
success does not prove this larger workflow. README validation history was
consolidated into links to this ledger and the existing Wine validation reference.

Default-path refinement: use native app_local_data_dir/Stargate Worlds (Windows
LocalAppData), creating only its parent on first preference save. The settings
root remains app_data_dir/state. Final ordinary checks passed 218 engine tests
with ten ignored and 17 shell tests with one ignored (235 total). JS installation
UAT passed the default-folder/enabled-install/unchanged-consent case using mocked
native IPC. Strict clippy and the real-release content smoke remain pending.

Real-release update: the full-content test itself passed in 314.73 seconds,
covering authenticated original seed, seven patches, production claim, shared
client setup, content checks, promotion and receipt, with consent false. The
lane subsequently failed when an omitted `--lib` allowed the development-key
process harness to run under the production key. Correct library-only rerun is
pending; do not report the first invocation as wholly green. Strict clippy passed.
The [smoke recipe](../../../../crates/launcher/desktop/docs/wine-validation.md#original-signed-release-content-smoke)
now requires `--lib`; ordinary fixture suites must not use the production key.

Final scoped full-content result: the exact `--lib` smoke passed in 312.55 seconds
(lane315.308s, exit0; `20261004-054057-59469`). It confirmed authenticated original
seed, all seven patches, content validation and receipt with consent false. This
supersedes the earlier pending clean-run status, without erasing the first
invocation's unrelated development-key harness failure.

Final ordinary evidence: engine218/ten ignored (`20261004-053903-58363`), shell17/
one ignored (`20261004-054308-60834`), combined strict clippy
(`20261004-053949-58844`) passed. Latest-source Mac development bundling passed
(`20261004-054415-61415`) with the known STATIC_VCRUNTIME deprecation warning.
No launch/login/gameplay result is implied; final self-contained packaging/startup
remains deferred.

### 2026-10-04: retain original prerequisites as inert data

Shared cabinet extraction now preserves optional `Data/Prerequisites` in
`.cimmeria-prerequisites`, separate from `Working`, before staging cleanup.
Same-volume publication accepts an identical tree on retry and refuses changed,
extra, linked, special or Windows-reparse content; cancellation is honored.
No runtime probe or prerequisite installer is run. Independent extraction of the
verified original seed inventoried 119 files and 160,028,982 bytes.

Combined ordinary engine/shell checks passed 239 tests with 11 ignored
(`20261004-055701-67012`). No frontend behavior changed, so JS logic UAT does not
apply. Final engine library rerun passed 222/ten ignored (`20261004-055927-68733`);
combined strict all-target clippy passed (`20261004-055946-69045`).
The staged Windows helper predates retention;
a native rebuild and real retention smoke remain gates. The enhanced full-release
smoke now requires the retained directory and exact independently measured SHA-256
hashes of four vendor executables. The earlier 312.55-second pass predates these
assertions. Existing successful content receipts do not imply retained or installed
prerequisites.

Earlier CI `37195142523` passed both platforms at `6ad941268`. Windows CI
`37195617733` failed while creating the junction fixture, before exercising
cleanup: mixed path separators made `mklink` reject its argument. The fixture now
normalizes separators and captures command output; native validation is pending.

### 2026-10-04: independent installed-content reference

`installed-content.json` records the prepared installation independently of the
latest operation. Workers and recovery publish it after promotion/receipt, before
terminal success; publication failure retains recovery. The reader returns the
saved intent and reverified signed release after checking the per-ID intent,
canonical owned root, marker and receipt. Missing game files or the entire game
directory preserve identity needed by future Repair; an existing game directory
must be ordinary and not a link/reparse point. Missing ownership records or signed
evidence remain errors. Recovery reuses the verified owner under its held lock.
A legacy current successful Install may migrate on read; selected folders are never adopted.
Current-operation gates still apply, and the reference alone is neither readiness
nor launch/delete permission. No frontend behavior changed; JS logic UAT is not
applicable to this backend-only packet. Local engine/shell tests passed 246 with
11 ignored (`20261004-060534-72136`); combined strict all-target clippy passed
(`20261004-060607-72674`). Tests cover identity across reopen/new operation,
legacy migration, uncommitted-reference gating, missing and redirected content,
mismatched owner/evidence/schema, recovery and publication failure. Read-only
review found no remaining blocker after the Windows held-lock correction.
Final-revision native Windows validation remains pending.

### 2026-10-04: confirmed native uninstall

The native uninstall API binds explicit confirmation to operation ID/revision and
saved installation ID. It validates owned content, rejects foreign top-level or
unsafe recursive entries, detaches the root by rename, checkpoints, deletes with
the owner marker last and forgets installed identity before terminal success.
Explicit same-ID recovery covers rename/checkpoint and partial-deletion boundaries
without touching a replacement folder. Preferences/consent, logs, signed evidence,
runtime caches and extraction prefixes remain. Mac Wine removal first stops the
original owned prefix. Lost replies require inspection; admitted removal cannot
be cancelled. See [maintenance contracts](../../../../crates/launcher/desktop/docs/maintenance.md).

This packet exposes the native API only; shell/frontend wiring and native Windows
validation remain pending. No frontend behavior changed, so JS UAT is not
applicable. Final combined engine/shell tests passed 256/11 ignored
(`20261004-061342-76259`); strict combined all-target clippy passed
(`20261004-061351-76556`). Ten uninstall tests include empty/missing detached
folders, reference removal before terminal commit, lost replies, owner conflicts,
foreign files, links and pre-admission persistence failure. The ignored real
signed-release Wine smoke now additionally performs confirmed uninstall after
retention assertions; that enhanced smoke awaits a newly built Windows helper.
No actual original-client removal result is claimed yet.

### 2026-10-04: Settings uninstall and restricted IPC

Restricted native Uninstall now supplies its saved installation ID/folder and
recovery capability to Settings. Inline confirmation identifies removed files and
modifications plus retained preferences/shared compatibility resources; dismissal
does not mutate. Effect pre-inspects, observes the reply for 35 seconds and never
replays a mutation after a lost reply. Explicit Finish uninstall confirms the same
operation ID. No cancellation is offered after admission; Install cleanup/cancel/
reconcile controls do not apply. Successful removal permits a new Install into an
eligible empty destination.

Engine/shell checks passed 257 tests with 11 ignored (`20261004-061858-78957`),
including native host removal from disk and consent preservation. All 28 frontend
tests, type checking/build and JS logic UAT passed. Fixture IPC exercised confirm/
dismiss, double-click protection, owned-path targeting, acknowledged removal and
fresh install, and unchanged consent; it does not validate native deletion. No app
was opened or visual UAT performed.

The retention-capable Windows helper from CI `37197224000` at `1fa1a13ce` is
staged; that CI passed both platforms. Enhanced smoke `20261004-062038-80112`
passed full-content preparation and all four prerequisite hashes, then correctly
refused uninstall with `Storage(InUse)` because the fixture retained a preclaim
guard beyond publication. Production releases that guard before publishing.
The fixture is corrected and its full rerun (`20261004-062633-83023`) remains pending; no completed real
uninstall is claimed. Final combined strict clippy passed (`20261004-062715-83742`);
updated Mac development bundling passed (`20261004-062355-81738`) with the new
helper resource SHA verified. The app was not opened.

### 2026-10-04: Windows x86 module-load probe contract

The standalone desktop workspace now contains `cimmeria-runtime-probe`: bounded
8 KiB stdin, SGW manifest resource-1 activation and fixed VC80 CRT/CPP, D3DX9_40,
XInput1_3 and absolute game PhysXLoader checks. Reports are path-free; loading a
module never asserts SDK initialization or game readiness. Three portable tests
passed (`20261004-063409-86560`); strict clippy and native Windows i686 CI remain
pending. No local Windows compilation or real-game probe is claimed. Managed
integration still requires helper hash/owned-root binding and supervisor deadlines.
See [probe contracts](../../../../crates/launcher/desktop/docs/prerequisites.md).

### 2026-10-04: Settings Repair journey

Settings connects confirmation against the saved installation identity, retained
preparation/replacement, progress, precommit cancellation, explicit recovery/
abandonment and current-backup cleanup. Lost replies require inspection rather
than mutation replay; success does not enable Play.

The [worker handoff](worknotes/repair-ui.md) records implementation `d00f6bc09`,
reserved dependency `622f7fff7` and commands. Worker results: 38 frontend tests;
shell 26 passed/3 ignored; repair engine 39 passed/3 ignored; strict shell clippy
and frontend build passed. Native-persistence Effect UAT exercised confirmation/
dismissal, duplicate suppression, saved-directory identity, durable cancellation,
lost-reply reopening, refused recovery, abandonment and preserved preferences/consent.

The UAT fixture holds preparation before reconstruction. It does not establish a
real reconstruction worker, helper lifecycle, replacement or cleanup. Original-
client Wine repair, native Windows locking/rename/power-loss behavior and packaged
visual/focus/layout UAT remain open. The chain is locally integrated as
`292f0678d`, `8a13518f6`, `e3709de6b`; review and integrated validation are pending.

### 2026-10-04: Retained adoption references and native extraction preflight

The [adoption preparation packet](worknotes/published-adoption-preparation.md)
is integrated as `211226a8` and `2857705a7`. It owns an Adopt operation before
Wine extraction, supports explicit preparation cleanup and preserves the source.
Two isolated real-helper fixtures passed, including RAR/CAB; the helper predates
the coordinator's RAR/FDI preflight (`198573ce7`). Current helper rebuild, full
published seed, effective settings/UI and owner/current-release parity remain
required. Latest integrated local engine tests pass 363, with 18 ignored;
this includes the owner-lock correction described in the acceptance checklist.

### 2026-10-04: Combined validation checkpoint

The separate combined branch includes [owner/current-release and Update admission](worknotes/owner-current-release.md),
[updater Apply](worknotes/updater-apply.md), and its [post-handoff persistence fix](worknotes/updater-handoff-fix.md).
It preserves the development rebuild checkpoint; PR #1164 integration is pending.
Fixture evidence does not establish game Update execution/UI, effective settings,
production updater configuration or signed packaged Mac/Windows upgrade readiness.

### 2026-10-04: Mounted Update and rollback verification

`8436ed4e8` connects signed game Update review to retained execution in Settings.
`b525072a9` integrates [actual signed rollback UAT](worknotes/game-update-rollback-uat.md):
eight host tests, 61 frontend tests, native-persistence mounted rollback/reopen
UAT and strict scoped Clippy pass. Original and current backups remain separate;
partial-download cancel/discard and interrupted preparation abandonment preserve
the old game. These portable fixtures do not establish real Wine replacement.

The current development build reached the actual SGW login screen through Play;
operator visual confirmation is recorded in [native-window UAT](worknotes/native-window-uat.md).
Authentication/world entry and windowed focus remain unverified. Windows updater
diagnosis is handed off through issue #1194; migration settings/adoption UI remain
independent implementation work. Full launcher acceptance is still open.

### 2026-10-04: Adoption UI, effective settings and Wine app identity combined

`launcher/uat-integration` squash-merges three branches from their final commits:
[verified-copy adoption UI](worknotes/adoption-ui.md) (`6c372b849`),
[effective imported settings](worknotes/effective-settings.md) (`4786bc621`) and
[opt-in Wine app identity](worknotes/wine-computer-use.md) (`4d17fe1ba`, with the
30 FPS cap). Git reported no conflicts; the semantic ones and their resolutions
are in the [integration note](worknotes/uat-integration.md).

A Wine-backed copy adopted through Settings is now offered prerequisite setup,
and Play once prerequisites succeed, with the patch setting and login server
order the user reviewed. New host journeys adopt through the production host and
carry the same store to one admitted Play, with patches reviewed off and on. The
shared `HeldDownload` test origin no longer panics on macOS.

Local macOS checks on the combined tree passed: the desktop workspace tests,
strict Clippy and formatting, seven ignored Wine fixtures in isolated prefixes,
frontend typecheck, tests and build, and the native-backed JS UATs for adoption
(portable and Wine), launch, migration, game Update with Apply and rollback, and
the updater. Exact counts are in the integration note.

This is fixture evidence. No game was started and no package was rebuilt. Still
open: the signed Mac rebuild, native window UAT of adoption, real prerequisite
preparation and Play of an adopted copy, the `CIMMERIA_WINE_APP_IDENTITY=1`
runtime and computer-use checks, the published RAR/CAB seed with a rebuilt
helper, and native Windows validation.


### 2026-10-04: MacBook testing stopped; repository checkpoint

The [MacBook testing checkpoint](worknotes/macbook-testing-checkpoint.md) records
integration through `d36dbebfa`, the normal signed `e79e99b1e` build and the
hidden-panel fix native-verified in a separate signed UAT app. Native Play
started SGW and the operator confirmed its login screen; launcher status and
rechecks stayed stable. Authentication/world entry and actual game computer use
remain unverified.

Isolated native legacy import preserved diagnostics consent false and created
no installed owner. Verified-copy preparation ended interrupted with explicit
cleanup offered; cleanup was not pressed, and no completed adoption,
prerequisites or adopted Play is claimed. The cause and inconsistent recovery
feedback remain open. Preserve test state and files; MacBook testing and builds
stop at the user's request. The existing Wine identity investigation may finish
its current assignment, but receives no new work and is not integrated here.

Full launcher/release acceptance, the current Windows helper, Windows parity,
production updater configuration, clean-machine/notarization and reserved
observability work remain open. This checkpoint does not close the campaign.


### 2026-10-04: repository preservation after MacBook testing stopped

Published the tested integration checkpoint and its acceptance evidence on
`launcher/uat-integration`. Existing draft PRs #1164 and #1190 link to the
checkpoint; their branch heads were not advanced or merged. Preserved the
completed Wine identity follow-up separately at `ea7b4ec42` on
`launcher/wine-identity-followup`. Its worker reports bounded unit/native Wine
fixture validation; actual SGW computer-use and Windows remain unverified.
See the [checkpoint](worknotes/macbook-testing-checkpoint.md) for the test
failure caveat, exact branch boundary and remaining gates. No new coordinator
build, UAT, release or additional worker assignment accompanies this record.
