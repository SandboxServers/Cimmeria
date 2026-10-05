# Native installed-game Update journey

> **Type:** Reference (implementation handoff)
> **Audience:** Launcher engine and shell maintainers
> **Last updated:** 2026-10-04
> **Companions:** [Migration contract](../../../../../crates/launcher/desktop/docs/migration.md), [owner/current release](owner-current-release.md), [adoption audit](adoption-contract-audit.md)

Base: `60a8489eaeff0f9f551c4b909ab4a92b847c18ef`.
Branch: `launcher/game-update`.
Worktree: `.claude/worktrees/launcher-game-update`.

## Implemented behavior

Update has its own old/new signed identities, work directory, tree roles and
commit/discard/cleanup records. Retained preparation reconstructs the target with
shared installation/extraction primitives, progress and precommit cancellation.
A bounded derived stage fingerprint detects file changes after preparation. The
handoff retains owner/work locks and, for Wine, its private adapter. Neither lost
preparation nor lost commit observation causes redispatch.

Commit retains the original game as an owned backup, promotes the reconstructed
stage, publishes the root content-ready receipt, then the installed-content index,
and only then succeeds. Partial receipt/index publication requires explicit
checkpoint recovery. The permanent owner, original Install intent, adoption
provenance, imported legacy JSON and setup inputs remain unchanged. Update creates
no substitute Install operation. Shared tree-role primitives also retain Repair's
same-current-release semantics.

Rollback is a separately confirmed new Update reconstruction using the previous
signed release and a fresh launcher-minimum check. It preserves both backups and
does not activate potentially modified backup files. Explicit abandonment retains
precommit output; confirmed discard removes only its owned cancelled/failed work.
Backup cleanup uses resumable deletion checkpoints. Uninstall validates completed
Update/Repair auxiliary directories against saved plans, checkpoint, owner and
role evidence before removing them with the installed root.

## Public integration surface

- `DesktopState::admit_update(Request)` binds the confirmed owner, expected current
  release, authenticated target and operation revision. Honor `dispatch == false`;
  do not replay retained work after a lost response.
- `update::preparation::{prepare_native, prepare_wine}` return `Preparation` with
  `progress`, `result` and `request_cancel()`. Success returns `Prepared`.
- `update::commit::{commit_native, commit_wine}` consume `Prepared` and return a
  retained result receiver. Commit serializes cancellation before either rename.
- `update::recovery::recover_*`, `update::abandon::abandon_*`,
  `update::discard::discard_*` and `update::cleanup::cleanup_*` accept the observed
  operation ID/revision. Abandon/discard additionally require explicit confirmation.
  The caller must obtain separate destructive confirmation for backup cleanup.
- `update_plan()`, the operation snapshot, and `update::cleanup::status` inspect
  retained identity, progress state and backup availability without redispatch.
- `admit_update_rollback(completed_update, id, revision, expected_current, confirmed)`
  derives the previous target from the completed durable Update plan and evidence;
  its returned admission then uses the same preparation/commit APIs.
- The `test-support` feature exposes `update::test_support::{prepare, commit,
  recover, cleanup}` for local HTTP fixture/IPC UAT, without relaxing production
  platform or catalog transport gates.

All functions are exported through the already registered `update` module. No
shared `lib.rs`, storage registry, operations schema or dependency edit is needed.
The shared integration commit carries extraction identity, index publication,
Wine quiescence, common tree roles and Uninstall consumers. Apply it with the core
Update commit; the isolated core commit alone is not a compilable handoff.

## Verification

Native Mac full engine run: **382 passed, 18 ignored**, plus helper and prerequisite
subprocess protocol harnesses passed. Lane log identifier:
`20261004-131756-63298`. Strict all-target engine clippy with `test-support` passed;
format and diff checks passed.

The filesystem/HTTP fixtures exercise:

- Two distinct signed seeds through Update → reopen → Repair of the new current
  release → Uninstall, preserving the original owner and both signed evidences.
- Both sides of both renames and both root/index publications, followed by reopen
  and explicit recovery with unchanged HTTP request count.
- Precommit cancellation, lost prepared handoff, lost commit observer, no replay,
  explicit abandonment/discard and successful backup cleanup.
- Foreign work ownership, occupied backup and altered nonempty staged executable
  refusal; Uninstall also refuses foreign auxiliary ownership.
- New confirmed signed rollback reconstruction and a separate negative admission
  guard for the previous release's launcher minimum.
- Actual adoption followed by signed Update preserves provenance, exact imported
  JSON and original legacy source files. This is a Mac filesystem fixture.
- Wine helper identity refusal and pre-dispatch cancellation verify target
  extraction binding without starting a helper, runtime download or guest process.

Rerun from this worktree through the build lane:

```bash
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine
bash tools/build-lane/lane.sh cargo clippy --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets --features test-support -- -D warnings
cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all -- --check
```

## Remaining gates and presentation contract

This packet changes no shell, frontend or launcher executable updater. Coordinator
integration owns the IPC/Effect journey, native persistence JS UAT, window/focus
validation and shared guide/index/ledger updates. Native confirmation must display
and bind both reviewed signed releases, permanent destination and revision.

There is no old-tree per-file modification report. The UI must explicitly explain
full reconstruction: modifications and game-local user files remain only in the
old backup and are not merged into the new active game. The stage fingerprint is
derived integrity evidence, not publisher-signed per-file provenance or proof that
another application cannot modify files after verification.

No real game/user installation, legacy account data, external service, observability
or VPN state was used. Real-client Wine helper execution was not performed in this
packet; the adapter's dispatch/quiescence wiring is not that external validation.
Native Windows execution, locking/rename behavior and hardware power-loss durability
remain CI/platform gates. Successful reconstruction alone does not prove Play,
server login, effective imported settings or end-user migration parity.

## Coordinator shell and Effect integration

The combined-validation branch mounts game Update in Settings. Review binds the
native-held signed offer, directory and operation revision. Apply retains the
preparation-to-commit handoff; cancellation and confirmed recovery/abandonment,
partial-file cleanup, backup cleanup and signed rollback route to native workers.
The UI discloses that modifications and game-local saves stay only in the backup.

Six scoped shell tests pass (two opt-in UAT bridges ignored). The mounted
`uat:game-update-apply` pass exercises a signed inert archive over loopback, real
filesystem replacement, confirmation dismissal, lost Apply reply inspection,
backup cleanup and store reopen. Frontend tests pass 60, including stale-review
invalidation and minimum-version blocking. These checks do not execute a real
client, Windows/Wine helpers, rollback through the mounted view or visual confirmation UAT. Packaged Settings Check was separately observed
to return an installed-release match; no real game replacement was attempted.
The original engine packet's platform limits still apply.
