# Repair design contract

**Status: native admission, reconstruction and retained replacement implemented;
checkpointed native restart recovery and current-operation backup cleanup are
implemented. Wine repair and UI remain unfinished.** This contract defines the replacement workflow and its gates.
Repair is not yet an available user command. See
[maintenance](maintenance.md) for implemented uninstall.

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
Admission, native staging and retained replacement below are implemented. Wine
repair adaptation and UI validation gates remain open. Pre-checkpoint
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

The public preparation entry point supports native Windows only. Local tests
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
to replace the game. The Wine repair adapter and UI remain unfinished. No frontend/visual UAT is claimed.

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
repair. Wine repair and UI integration remain unfinished. Native Windows rename/locking
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
