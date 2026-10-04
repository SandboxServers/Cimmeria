# Repair design contract

> **Type:** Reference
> **Audience:** Launcher contributors and testers
> **Last updated:** 2026-10-04
> **Companions:** [Desktop](../README.md), [validation handoff](../../../../docs/analysis/playtests/2026-10-03-macos-wine/worknotes/repair-ui.md)

**Status: Settings confirmation, retained native preparation/replacement, progress,
precommit cancellation, explicit recovery/abandonment and current-backup cleanup
are connected. Platform release gates remain open.** See
[maintenance](maintenance.md) for uninstall.

## Settings journey

Choose **Repair game** in Settings. The confirmation displays the saved installed
directory, even if Settings currently selects another folder or game content is
missing. Repair reconstructs the same authenticated release and replaces game
modifications. Dismissal sends no mutation. Confirmation creates one fresh work ID;
repeated clicks are suppressed and native duplicate IDs never dispatch again.

The application-owned Effect workflow reinspects native state before each action,
then observes the retained native worker. Download/extraction progress comes from
that worker. Closing the view does not cancel it. **Cancel** requests native
precommit cancellation; once replacement begins, it must finish or require recovery.
Successful repair preserves settings and diagnostics consent and never enables Play.

After a lost reply, choose **Recheck status**. Never automatically replay repair.
After a process restart, Settings exposes two explicitly confirmed recovery actions:

- **Recover repair** finishes only a valid checkpointed replacement. Native ownership,
  tree and Wine helper evidence checks can refuse it without changing files.
- **Abandon preparation** accepts only pre-checkpoint work. It preserves the original
  game and all retained stages, and records cancellation. It does not repair content.

Unknown helper outcomes remain gated. Preserve the game, backup, stages and journal
for manual inspection; restarting and rechecking does not prove a helper stopped.
Neither button promises recovery of every interruption.

After durable success, **Remove old repair backup** separately confirms permanent
removal of the current repair's backup, including modifications there. Cleanup is
recoverable and idempotent through its native checkpoint. It does not delete stages.
Perform it before starting another operation: historical backup/stage cleanup is
outside this command's scope. Settings and diagnostics consent remain available
according to the existing native persistence gates.

## IPC and validation boundary

`install_command` accepts `repair` with work ID, operation revision, permanent
installation ID and confirmation. `recover_repair`, `abandon_repair` and
`cleanup_repair` accept the inspected work ID/revision and confirmation. Paths,
release choices and claimed results are not accepted from the renderer.
The status includes a native repair target and explicit recovery/cleanup flags;
these are presentation eligibility, while mutation independently validates evidence.
Recovery buttons mean “request native validation,” not a guarantee it will succeed.

The host retains preparation's cancellation/progress handle and transfers only its
result receiver to the coordinator. That coordinator consumes `Prepared` directly
into native/Wine commit without releasing the root, work or prefix ownership.
Dispatch failure marks reconciliation immediately. Active observation avoids
reopening the worker-owned root marker, which matters for Windows file locking.

The `frontend/repair-uat.mjs` exercise runs actual Effect/view logic against the
ignored native `repair_uat_bridge` test. It checks durable admission, cancellation,
reopen, refused recovery, explicit abandonment, saved-directory identity and consent.
Its controlled worker seam holds work before reconstruction: it does **not** exercise
real downloads, Wine, replacement or cleanup. Rust engine fixtures cover those
filesystem checkpoints separately. Native Windows locking/power-loss and packaged
visual/focus UAT remain required; the coordinator owns the visible UI pass.
## Historical engine implementation evidence

The sections below retain earlier engine milestones. Statements that UI work was
unconnected describe those milestones, not the current Settings contract above.

## Identity and reconstruction

Repair reconstructs the installation's same cached, signed release from scratch
in a private operation stage. Reverify the cached manifest/signature and original
body digest; do not silently select the latest release or adopt the selected
preferences folder. Downloads, if needed, must follow that verified release.
The existing game is not an input to patch reconstruction.

The permanent `InstallIntent`, root owner marker and installed-content identity
continue to identify the original installation. A separate durable `RepairPlan`
binds a new work UUID to that owner ID, release digest, backend and exact owned
stage/backup paths. The Repair journal tracks work using the work UUID; helper
attempts and progress must not substitute it for the permanent owner ID.
Admission requires the inspected revision, verified ownership and exclusive
operation/root access. Existing operation and Wine ownership gates still apply.

Prepare and validate a fresh stage while preserving the original game tree.
Current `content_valid` checks ledger/layout and executable containment; it is not
a complete per-file integrity scan. Neither those checks nor an existing receipt
justify claiming detection or repair of arbitrary corruption. The replacement
must be described as reconstruction from the authenticated release. Replacing
`game` also replaces user modifications there; confirmation must explain this.

## Commit and cancellation boundary

Before commit, cancellation leaves the original tree untouched and retains or
cleans only proven operation-owned scratch output under an explicit policy.
Before the first rename, persist a commit plan/checkpoint binding the prepared
stage, original-tree presence and backup to this operation. If `game` exists,
rename it to the operation-owned backup, then rename the prepared stage to `game`.
When it was already missing, record that absence instead of inventing a backup.

These two renames are not a single atomic swap. Persist the transition boundaries
and sync affected directories using the platform persistence contract. Retain the
owned backup until replacement validation, required identity/receipt publication
and terminal success are durable. Preserve the permanent owner identity throughout.
Backup disposal after success needs a separately recoverable cleanup checkpoint;
an uncertain terminal write must never authorize deletion of the backup.

Once committing begins, cancellation cannot abandon or roll back the sequence.
The retained worker must finish, or expose explicit reconciliation. Lost replies
trigger inspection, never automatic mutation replay. Preferences, diagnostics
consent, logs and signed evidence remain unchanged. A successful Repair would
mean prepared content, not runtime readiness or permission to launch.

## Required recovery fault matrix

Recovery must inspect durable checkpoints and exact owned paths before acting;
absence alone is never proof of completion. No blind replay or foreign-tree
replacement is permitted.

| Interruption/evidence | Required behavior |
| --- | --- |
| Preparation only; old tree untouched | Preserve old tree; allow explicit cancellation or a validated continuation of this work. |
| Commit planned; neither rename happened | Verify prepared stage and original-tree state, then explicitly finish commit. |
| Old tree moved; checkpoint write interrupted | Recognize the exact owned backup and absent game; reconcile the first rename before proceeding. |
| Backup present; stage ready; game absent | Retain backup and finish promotion under ownership locks. |
| Stage promoted; receipt/checkpoint incomplete | Validate the replacement against the plan before publishing; do not repeat renames. |
| Replacement published; terminal result uncertain | Reopen/reconcile durability; retain backup until success is confirmed. |
| Success durable; backup cleanup interrupted | Resume only recorded backup cleanup; never reinterpret the backup as the active install. |
| Missing original game at admission | Reconstruct without backup using the recorded absence; unexpected new content blocks commit. |
| Missing stage/backup unexpectedly, conflicting records, foreign entries or links/reparse points | Remain gated; do not delete, infer success or overwrite a replacement tree. |

## Validation gates before exposure

Fault-injection tests must cover both sides of every rename and checkpoint,
receipt/terminal publication failures, reopen, duplicate work IDs, stale revisions,
owner/work-ID confusion, cancellation before commit and observation loss during
commit. Assert original-tree preservation, backup retention, unchanged consent and
preferences, and refusal of foreign, linked, special or Windows-reparse content.

Native Windows tests must exercise actual locking/rename behavior. Wine validation
must cover the owned helper/prefix lifecycle, interrupted preparation and a real
signed-release reconstruction, without claiming guest-process quiescence from
host exit alone. Neither a portable fixture nor module-load probes replace these
gates.

Before UI release, run real Effect service logic UAT through native persistence:
confirmation/dismissal, preinspection, one mutation under duplicate clicks,
precommit cancellation, no replay after timeout/reconnect, explicit recovery and
preserved consent. Add visual/manual UAT separately and state its coverage.
Admission, staging, retained replacement and constrained Wine recovery below are
implemented. UI and broader process-crash validation gates remain open. Pre-checkpoint
work can be explicitly abandoned as described below.

## Implemented admission boundary

`DesktopState::admit_repair` requires explicit confirmation, the inspected
operation revision and the original installation ID. It reverifies installed
identity and cached signed-release evidence, locks/rechecks the permanent root
owner during admission, and writes `repair-plan-<work-id>.json` before beginning
a Repair operation. The original `InstallIntent` is unchanged; `install_intent()`
does not reinterpret repair work as a first installation.

The plan records original-game presence and derives sibling
`.cimmeria-repair-<work-id>` / `.cimmeria-backup-<work-id>` paths under that
installation. Those paths must be absent and are not created by admission.
The actual replacement stage is the work directory’s `game` child.
A missing game is repairable identity, not lost ownership. Preferences are not
used to redirect repair. Matching duplicate IDs return `dispatch: false`,
including after reopen; identity conflicts and orphan plans are refused.

Three admission tests passed (`20261004-084102-41373`): modified content remains
untouched, missing content is recorded, confirmation/revision/owner mismatches
and held locks refuse admission, conflicting stages are preserved, plan tampering
fails validation, and reopen retains reconciliation gating and consent. These
are persistence/ownership fixtures, not reconstruction or platform rename tests.
The admission lock ends when the call returns; the retained worker reacquires
and revalidates ownership before touching game files. No frontend
behavior or JS/visual UAT is claimed for this internal API packet.

## Implemented preparation boundary

Repair reconstructs the saved, reverified signed release into a fresh private
`game` stage beneath the work directory, using the shared seed/patch/setup
pipeline. It does not reuse the damaged game or its ledger as reconstruction
input. The permanent installation identity remains unchanged.

Preparation rechecks original-tree presence, refuses an existing backup/work
path, and holds the installed-root and work-owner locks. Successful handoff
returns `Prepared` with both locks retained for the commit coordinator.
`repair-prepared-<work-id>.json` records completed staging. This layer never
renames, replaces or deletes the existing game. Pre-cancellation avoids staging
and downloads. A receiver lost before handoff preserves output and marks the
operation for reconciliation; failed terminal persistence reports uncertainty.

`prepare_native` supports native Windows; `prepare_wine` is described below. Local tests
use the private entry point with signed ZIP fixtures on macOS: six focused
repair tests passed (`20261004-084930-43826`), including fresh extraction despite
a damaged old tree and current ledger, retained locks, pre-cancel, lost receiver
and failed terminal persistence. Strict clippy passed (`20261004-085024-44236`).
These results do not establish original-client or native Windows extraction.

Dropping an already-delivered `Prepared` now notifies a retained observer that
marks the operation for reconciliation without waiting for restart. The drop
notification does not acquire the state mutex, so callers may release a handoff
while holding that mutex. Both trees remain unchanged, including when cancellation
arrives after staging. Seven focused repair tests pass
(`20261004-085543-45495`), including abandonment after delivery and cancellation.
The commit coordinator retains the handoff and checks recorded cancellation
before the first commit mutation. Receiving `Prepared` alone is not permission
to replace the game. The Wine repair adapter is described below; UI remains unfinished. No frontend/visual UAT is claimed.

## Implemented replacement boundary

`commit_native` consumes `Prepared`, retaining its root/work locks throughout
replacement. It rewinds and rereads ownership through those locked handles and
validates the current plan, prepared record, signed release and stage. Recursive
checks reject links, reparse points and special files in both work and original
trees. Reopening a locked owner file is avoided for Windows compatibility.

Cancellation is serialized with commit entry: an already-recorded cancellation
preserves the original game. Once commit holds the state lock, queued cancellation
cannot interrupt replacement. Dropping the result observer does not abort it.
Checkpoints advance Planned → OriginalMoved → Promoted → Published. Existing
`game` moves to the owned backup, then the prepared stage becomes `game`; an
originally missing game needs no backup. The replacement is checked and its
receipt published with the unchanged installation identity before terminal
success. These are separate renames, not an atomic swap.

Errors retain reconciliation gating; an existing commit record refuses replay.
The old backup remains after replacement success until explicit backup cleanup
below. Checkpointed restart recovery is also described below. Public entry is native-Windows-only; private Mac
fixtures do not establish Windows rename or power-loss behavior. Unix directory
syncs are issued; Windows directory power-loss durability remains a validation
gate. Content checks remain ledger/layout evidence, not an exhaustive corruption
scan or runtime/game readiness.

Thirteen focused repair tests passed (`20261004-090059-47640`). The replacement
fixtures cover damaged and absent originals, unchanged identity/preferences,
observer loss, precommit cancellation, all eight injected rename/checkpoint
boundaries, refused replay, failed terminal persistence, conflicting backup or
checkpoint paths, and nested Unix links in either tree. Each injected boundary
asserts it was actually reached. This is native ZIP-fixture evidence on macOS,
not full-client repair, native Windows rename validation or frontend/visual UAT.

The full engine suite passed 269 tests with 12 ignored environment-dependent
cases (`20261004-090148-48013`); strict clippy passed
(`20261004-090110-47779`).


## Explicit native restart recovery

Commit writes operation-specific Original/Replacement markers into the respective
trees before its schema-2 Planned checkpoint. Markers move with renamed trees and
bind their roles to the exact repair plan. They establish ownership correspondence,
not cryptographic integrity of all tree contents. Writing the Original marker
modifies the old tree during commit; preparation and precommit cancellation still
leave that tree untouched.

`recover_native` requires the current Repair operation ID, inspected revision and
reconciliation state. Under root/work ownership locks, it verifies the prepared
record, schema-2 commit record, cached signed release and permitted game/stage/
backup role combinations. It finishes interrupted renames or republishes validated
promoted content, then explicitly reconciles success. It does not download or
reconstruct again. The retained worker does not depend on its response observer;
recovery offers no cancellation after entry.

Missing/conflicting ownership, unexpected tree shapes, unsafe entries and legacy
schema-1 commit records remain gated. Backup content is preserved. An interruption
between writing tree markers and the Planned checkpoint also remains gated: this
entry point requires a complete commit record. It cannot yet resolve every
commit-entry interruption or interrupted preparation. Explicit abandonment below handles pre-checkpoint work without completing the
repair. Wine recovery is described below; UI integration remains unfinished. Native Windows rename/locking
and power-loss validation remain separate gates.

The recovery tests reopen state across the checkpoint/rename matrix for both
present and absent originals. They also interrupt recovery itself at seven
mutation/checkpoint boundaries, reopen again, and finish without losing the old
backup. Negative cases cover stale IDs/revisions, foreign game folders, missing
backups, mismatched roles, legacy records and interruptions during marker writes.
Each fault asserts it was reached. These are signed ZIP fixtures on macOS, not
full-client repair or frontend/visual UAT.

Nineteen focused repair tests passed (`20261004-090713-49685`); the full
engine suite passed 275 tests with 12 environment-dependent cases ignored
(`20261004-090800-50062`). Strict clippy passed (`20261004-090750-49925`).


## Explicit abandonment before commit

`abandon_native` requires confirmation and the current recovery operation ID and
revision. It accepts only native Repair plans with no commit-record entry of any
kind and no backup. Under the root ownership lock, it verifies the canonical root
and recorded original-game presence. If a work directory exists, its matching
work-owner lock is also held. An optional prepared record must match the plan.
Missing work ownership, contradictory prepared evidence or changed original-tree
presence refuses abandonment.

Abandonment reconciles the operation to Cancelled without deleting or changing
game/stage content, markers, caches or evidence. It does not declare incomplete
work repaired or recursively validate retained bytes. A separately confirmed
Repair with a fresh UUID may reconstruct again, preserving the abandoned attempt.
Existing, malformed, linked or legacy commit records forbid abandonment;
checkpointed replacement requires explicit commit recovery instead. Retained
stages can accumulate; their cleanup remains separate work. The backup cleanup
below does not remove abandoned stages.

Tests cover admission-only interruption, partial staging without content adoption,
both pre-checkpoint marker-write interruptions, confirmation/ID/revision refusal,
held ownership, ambiguous backup/original paths, failed terminal persistence,
reopen and new-operation admission. These are internal native-engine fixtures;
native Windows validation and frontend/visual UAT remain separate gates.

Twenty-four focused repair tests passed (`20261004-091210-51600`); strict clippy
passed (`20261004-091209-51614`).


## Explicit backup cleanup after success

Internal `cleanup_native` requires the current successful Repair ID and inspected
revision. Under the installation-owner lock, it verifies a schema-2 Published
commit, matching replacement-tree marker, signed release, content layout and
receipt before deleting the operation-owned original backup. Success must already
be durable: uncertain replacement never authorizes cleanup.

A separate cleanup checkpoint advances Deleting → Empty → Removed. Remaining
backup content is checked before deletion; its role marker is removed last.
Empty permits recovery after marker or directory removal, while unexpected entries
at that phase refuse deletion. Removed requires the backup to remain absent.
Deleting removes ordinary content under the marked backup, including files added
there later. Recursive checks are not a filesystem snapshot; ownership locks
serialize cooperating launcher work, not arbitrary external filesystem writers.
Cleanup preserves the successful Repair result, active game and preferences.

This removes only that backup. Work directories and abandoned staging remain.
Cleanup after another operation replaces the current journal entry is unsupported.
There is no UI wiring; native Windows locking/rename and power-loss validation
remain separate gates. Layout checks are not exhaustive content-integrity proof.

Twenty-nine focused repair tests passed (`20261004-091547-52717`). Cleanup fixtures
cover present/absent original backups, repeated cleanup after reopen, all six
injected cleanup boundaries, uncertain success, stale IDs/revisions, unexpectedly
missing backups, failed checkpoint creation, foreign entries after Empty, and
nested Unix links. Every injected boundary asserts it was reached. These are
native signed-ZIP fixtures on macOS; no original-client deletion or JS/visual UAT
is claimed.

Strict clippy passed (`20261004-091608-52964`).


## Extraction work identity for Wine integration

Native `ExtractionWork` derives the current Install or Repair work ID, journal
digest, permanent installation identity and stage/cache paths from the validated
durable intent or plan. Repair uses its own work ID/digest without replacing the
original installation owner. Helper journaling uses this descriptor to bind Wine
attempts; extraction rechecks it before accepting paths. This describes identity,
not admission, ownership locks or process quiescence.

The Wine extractor now retains the descriptor through prefix ownership, helper
requests and journal calls, as described below. The permanent installation ID
cannot substitute for the repair work ID. Retained Wine preparation/commit and
observed-result recovery are described below.

Three new identity tests passed (`20261004-091918-53906`), covering distinct repair
and installation identities, helper checkpoints/reopen, changed-plan refusal and
native-backend refusal. The full engine suite passed 288 tests with 12 ignored
(`20261004-092004-54235`). This includes existing helper journal and Mac adapter
fixtures; no new real Wine repair extraction or frontend UAT is claimed.

Strict clippy passed (`20261004-092031-54581`).


## Mac Wine repair extraction adapter

The adapter retains the complete `ExtractionWork` descriptor for a current
Running or CancelRequested Install/Repair operation. Extraction revalidates that
descriptor and uses the work ID for helper requests and journaling, preserving the
permanent installation identity. Install prefixes retain their existing path and
marker format. Repair uses a fresh `wine-repair-prefixes/<work-id>/bottle` with a
schema-1 owner record containing its descriptor. Existing prefixes are never
adopted or replayed. The supervisor reverifies the original cached signed release
and seed hash before helper admission.

Two new adapter fixtures verify isolated prefix ownership, refused readoption,
correct repair request/journal identity, rejection of original-install paths and
rejection of an unsigned seed hash before spawning. Eighteen Mac adapter tests
passed (`20261004-092636-56544`, five environment-dependent tests ignored).

The real Windows-native archive helper also extracted a signed ZIP fixture with
a Unicode path in a fresh headless repair prefix. Its work-ID journal completed,
the adapter stopped/waited for that prefix, and the existing inert game fixture
remained unchanged (`20261004-092639-56502`, 18.194 seconds). The helper binary was
built natively on Windows from `1fa1a13ce38fda449bc4e34037050612173271c6` and its
SHA-256 was rechecked as
`d0c89fad444cb4dc6478f1db8a5e62bc54d5696ee2a84a63d740bf3a5b92c6a3`.
Reproduce with `CIMMERIA_WINE_HELPER` pointing to that verified artifact and run
through the build lane:

```bash
bash tools/build-lane/lane.sh cargo test --locked \
  --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-engine --lib \
  mac_wine::prefix::tests::native_helper_extracts_zip_with_repair_work_identity \
  -- --exact --ignored --nocapture
```

This validates the extraction adapter, not original-client repair, game launch
or graphics. The retained worker below adds installation/work-tree ownership
across Wine extraction; constrained observed-result recovery is described below.
Broader process-crash recovery and Repair UI remain unfinished. No desktop window was opened and no frontend/visual UAT is claimed.

After the final seed-refusal test, the full engine suite passed 290 tests with
13 environment-dependent cases ignored (`20261004-092727-57278`); strict clippy
passed (`20261004-092751-57236`).


## Retained Mac reconstruction and commit

Mac Repair has internal `prepare_wine` and `commit_wine` entry points. Preparation
validates the native-selected helper resource and immutable Wine backend before
entering Running, then acquires installation/work ownership and reconstructs
through the shared installer using the repair operation's cache. Successful
`Prepared` retains both file locks and the Wine adapter's exclusive extraction
prefix ownership. The adapter completes prefix stop/wait before returning
successful extraction. `commit_wine` requires that retained adapter and consumes
the handoff through the checkpointed replacement sequence.

Missing/replaced resources refuse dispatch. Cancellation recorded before worker
execution avoids staging, runtime provisioning and downloads. Extraction or
stop/wait uncertainty remains reconciliation-gated; successful staging alone
never authorizes replacement after cancellation. The retained prefix lock is not
a runtime-cache lock.

Repair remains unavailable in the UI. Explicit Wine recovery, abandonment and
backup cleanup now use the observed-result gates below; ambiguous helper loss
remains gated. Gameplay/graphics readiness is separate from reconstructed content.


The retained signed-ZIP Wine repair smoke passed (`20261004-093516-60234`,
37.583 seconds including lane execution). It performed one HTTP seed download,
Windows-helper extraction, client setup, staged validation and commit. It verified
held installation/prefix locks before commit, their release afterward, unchanged
consent and permanent identity, new content in `game` and the original inert
fixture in the backup. It uses the same pinned Windows helper documented above.
Run with `CIMMERIA_WINE_HELPER` set to that artifact:

```bash
bash tools/build-lane/lane.sh cargo test --locked \
  --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-engine --lib \
  storage::repair::preparation::wine_tests::retained_wine_repair_reconstructs_and_commits_under_all_ownership_locks \
  -- --exact --ignored --nocapture
```

The fixture initially exposed its invalid empty login-server list; the shared
client-setup validation correctly refused it. The successful run uses valid saved
login settings. No production validation was bypassed. The full engine suite
passed 291 tests with 14 ignored (`20261004-093530-60393`). This is not an
original-client RAR repair, native Windows validation or in-game UAT.

Strict clippy passed (`20261004-093618-60749`).

## Observed-result Wine recovery and cleanup

`recover_wine`, `abandon_wine` and `cleanup_wine` reuse the filesystem workflows
above. After installation ownership is locked, they validate the current repair
descriptor, pinned runtime and exact repair-prefix owner. Promotion and backup
cleanup require an observed Completed helper result and recorded-host absence.
Abandonment also accepts observed Cancelled/Failed/NotStarted results; NotStarted
may legitimately have no PID. Signal zero checks absence without terminating a
PID; live/reused hosts and LaunchIntent/HostStarted/Uncertain outcomes stay gated.

Without a helper journal, only abandonment can proceed without starting Wine,
retaining any existing valid prefix-owner lock. Otherwise the gate reopens the
verified cached runtime and performs bounded stop/wait for the exact repair prefix
on a native blocking thread. Prefix/cache guards remain held through filesystem
reconciliation or cleanup. No runtime download or fallback bypasses failed checks.
Host absence alone never proves Wine guests have stopped.

The real `retained_wine_repair_recovers_promotion_and_cleans_backup` smoke passed
in 27.245 seconds (`20261004-094105-62166`). It exercised a simulated interruption
after promotion, explicit recovery and backup cleanup through the retained Wine
fixture. This is not proof of an actual process crash, arbitrary descendant loss,
original-client RAR repair or gameplay. Full engine validation passed 294 tests
with 15 ignored (`20261004-094147-62543`); strict clippy passed (`20261004-094534-63498`).
Earlier counts above remain dated evidence for their respective packets. Repair
UI remains unconnected; no frontend or visual UAT is claimed.

## Current backup observation and host integration guards

Settings exposes Repair for the saved installation identity. The host retains
preparation progress/cancellation and consumes its successful handoff in a native
coordinator that dispatches commit independently of the view. Closing or losing
an IPC observer cannot replace that coordinator. Earlier "UI unconnected" notes
above describe the earlier engine-only packets.

A successful Repair result does not imply a backup still exists. `repair.backup`
reports `retained`, `cleanup_pending`, `removed`, `not_retained`, or `unavailable`
from the engine's current plan, published commit checkpoint, backup role and
cleanup record. The cleanup control is available only for a retained backup or
unfinished cleanup. Missing-content repairs report no retained backup; completed
cleanup reports removal immediately and after reopening. Partial cleanup remains
resumable, including interruption between directory removal and its final record.
These observations never change the successful Repair outcome or Play readiness.

The shell integration guards dispatch actual Repair/Cancel/Recover/Cleanup
commands through the production host coordinator. The explicitly enabled engine
`test-support` feature supplies loopback transport, the shared signed inert ZIP,
portable native-algorithm entry points and an after-promotion fault. Production
OS eligibility and Wine ownership checks are unchanged. The tests verify progress,
pre-commit cancellation, retained commit, successful recovery and cleanup, missing
content and reopened backup status without Wine or original-client downloads.
`frontend/repair-native-uat.mjs` drives the real Effect/view journey against the
same host fixture; the older `repair-uat.mjs` remains an admission/persistence
fixture and does not prove preparation or commit. Neither replaces packaged
visual/focus UAT or native Windows/Wine validation.
