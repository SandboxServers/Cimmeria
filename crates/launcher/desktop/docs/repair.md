# Repair design contract

**Status: native admission implemented; reconstruction, commit/recovery and UI
remain planned.** This contract defines the replacement workflow and its gates.
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
Only the admission layer below is implemented; the reconstruction, commit,
fault-recovery and UI validation gates above are still open.

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
A missing game is repairable identity, not lost ownership. Preferences are not
used to redirect repair. Matching duplicate IDs return `dispatch: false`,
including after reopen; identity conflicts and orphan plans are refused.

Three admission tests passed (`20261004-084102-41373`): modified content remains
untouched, missing content is recorded, confirmation/revision/owner mismatches
and held locks refuse admission, conflicting stages are preserved, plan tampering
fails validation, and reopen retains reconciliation gating and consent. These
are persistence/ownership fixtures, not reconstruction or platform rename tests.
The admission lock ends when the call returns; the future retained worker must
reacquire and revalidate all resources before touching game files. No frontend
behavior or JS/visual UAT is claimed for this internal API packet.
