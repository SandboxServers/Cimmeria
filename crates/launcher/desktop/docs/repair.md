# Repair design contract

**Status: native admission, reconstruction and retained replacement implemented;
backup cleanup, restart recovery, Wine repair and UI remain unfinished.** This contract defines the replacement workflow and its gates.
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
repair adaptation, restart recovery, cleanup and UI validation gates remain open.

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
to replace the game. The Wine repair adapter, restart recovery, cleanup and UI
remain unfinished. No frontend/visual UAT is claimed.

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
The old backup remains even after success. Backup cleanup and explicit restart
recovery are not implemented. Public entry is native-Windows-only; private Mac
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
