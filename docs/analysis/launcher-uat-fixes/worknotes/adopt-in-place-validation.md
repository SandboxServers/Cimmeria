# Direct-root adoption validation

> **Type:** Reference
> **Audience:** Launcher engineers, reviewers, and the owner
> **Last updated:** 2026-10-23
> **Companions:** [Approved design](../design-adopt-in-place.md), [ledger](../README.md), [resume](../RESUME.md), [migration contract](../../../../crates/launcher/desktop/docs/migration.md), [UAT guide](../../../guides/unified-uat.md#launcher-uat-fixes)

## Scope and approvals

The owner approved direct-root adoption on macOS as well as Windows, explicit
per-file setup/patch changes, cooked normalization, and replacement backups at
the selected root. Bulk copying, relocation, and re-homing remain prohibited.
D-LX15's stable release version scheme is approved separately. This task has no
authorization to cut a release; historical release instructions do not override
that boundary.

The original U1-U5 PRs (#1984 through #1988) and close-out #1989 are merged.
Task 1 is [PR #1992](https://github.com/SandboxServers/Cimmeria/pull/1992).
Review-1 fixes N1-N6 are complete; fresh review 2 is pending. Task 2 is
[PR #1991](https://github.com/SandboxServers/Cimmeria/pull/1991): fresh review 3
returned ship, required CI is green, and merge waits for native CI. Neither PR
has merged. These are independent task worktrees, not unfinished original waves.

## Contract under validation

New adoption uses matching schema-2 plan and intent records, without adding a
field to the digested `Plan`. Existing ownership/provenance/journal checks precede
content-root resolution. Schema-2 content remains at the selected root;
successful schema-1 managed/legacy installations keep `root/game`.

Matched originals retain their paths and file IDs unless an approved individual
content transformation requires replacement. Direct-root maintenance commits per
file and preserves extras. Selective Uninstall retains the root and unrelated
files. Interrupted legacy movers refuse recovery and offer metadata-only abandon,
retaining game bytes, staged originals, and backups for manual inspection.
Schema-2 abandon rolls back approved per-file changes.

The main-page button uses content validity, independently of Play/runtime gates,
and enters the existing adoption workflow with immediate first-press feedback.

## Initial reported validation baseline

The implementing session supplied these results and build-lane log identifiers;
this documentation update does not claim to have rerun them.

| Check | Result | Evidence |
|---|---|---|
| Engine suite | 931 passed, 3 ignored | Lane log `20261022-205655-395740` |
| Desktop workspace clippy, all targets | Clean | Lane log `20261022-205615-207808` |
| Shell adoption tests | 24 passed, 1 ignored | Implementing session's shell test result |
| Frontend tests | 250 passed | Frontend agent report |
| Frontend build | Passed | Frontend agent report |
| JS REPL logic UAT | `npm run uat:adoption-footer` passed | Real frontend composition with deferred bridge, per frontend agent report |

All compiling Cargo checks use `pwsh tools/build-lane/lane.ps1 cargo ...`.
The three engine ignores and one shell ignore are not passing executions.
Native macOS CI remains pending.

## Review-1 fixes and updated validation

N1-N3 guard nested paths through retained directories, refuse unexpected
reappeared targets, and make Abandon restoration restartable. N4 journals removal
of previously authenticated outputs absent from a rollback target, while
preserving live extras. N5 refuses readiness without the retained authenticated
adoption plan. N6 qualifies the migration documentation's full-reconstruction
backup behavior as schema-1 only. All six fixes are complete; fresh review 2 is
pending, so this is not final review acceptance.

| Post-fix check | Result | Evidence |
|---|---|---|
| Safety adoption suite | 257 passed | `20261022-213247-208240`; `.claude/reports/launcher-adopt-in-place/fix-safety-1.md` |
| Engine fmt / all-target clippy | Passed / clean | `20261022-221153-70516`, `20261022-221157-142624` |
| Full engine after rollback fix | 939 passed, 3 ignored | `20261022-221204-254736`; `.claude/reports/launcher-adopt-in-place/fix-rollback-1.md` |
| Restored retained-plan provenance tests | 2 passed | `20261022-221644-453132` |
| Native-backed adoption UAT | `npm run uat:adoption` passed | Implementing session's production Effect/native-host fixture run |
| Footer and Light REPL / frontend build | Passed again | Implementing session's post-fix results |

The native-backed UAT uses the production Effect workflow and native host with
isolated disk fixtures and a signed loopback release. It exercises modified-file
replacement consent, hard-link refusal, a single confirmation, the executable at
its original path, and selective Uninstall preserving an extra file. The fixture
provides native paths for both platforms; this run does not establish macOS
runtime behavior. It excludes Wine, Tauri windows, the physical picker, and a real
game. The three ignored engine tests remain unexecuted.

## Regression proof

Restoring bulk relocation into `root/game` made the adoption location invariant
fail (lane log `20261022-205443-44468`). Restoring the fix made the targeted check
pass (`20261022-205550-94624`). The guard covers unchanged matched-file location
and identity, so a copy/delete substitute does not satisfy the invariant merely
by preserving bytes.

The frontend agent separately mutated visibility and first-feedback behavior:
5 of 6 targeted checks failed. Restoring the fix made all 6 pass. This is evidence
for those guards; it does not claim mutation testing of every added assertion.

Additional review-fix mutations were restored before passing checks:

- N1: removing the Windows parent-open no-follow flag fails the nested-link
  journey (`20261022-212815-159012`, 1 failure).
- N2: disabling unexpected-target refusal makes both recovery preservation tests
  fail (`20261022-212941-237992`, 2 failures).
- N3: repeating setup restoration on every Abandon retry corrupts the executable
  restoration assertion (`20261022-212952-263940`, 1 failure).
- N4: omitting previous managed paths absent from the rollback stage fails the
  A/B/rollback/Uninstall journey (`20261022-221126-112228`) and removal-recovery
  guard (`20261022-221138-423724`). The restored full suite passes as above.
- N5: allowing readiness without the retained plan fails the missing-plan guard
  (`20261022-221612-136968`); restored provenance checks pass 2/2
  (`20261022-221644-453132`).

The safety report distinguishes unsuccessful preliminary mutations from the
actual failing guards; those preliminary attempts are not counted as proof.

## Browser mock and coverage limits

At a 1280 × 720 viewport, a browser mock showed the main-page footer entry.
One press opened Settings, immediately displayed pending status, and disabled a
duplicate submission. Unrelated mock data was incomplete, so this is a bounded
visual observation and not validation of the native launcher.

Not exercised: actual folder-picker windows, packaged native visual behavior,
real macOS runtime/filesystem adoption, a game launch, physical power loss, or
SigNoz end-to-end ingestion. No laptop, lab, or live-client acceptance is claimed.
No release was cut. AIP-U1 through AIP-U5 remain owner UAT rows in the ledger and
unified guide; obtain permission before laptop or lab use.

## Documentation and next actions

The doc-update map rows covered are desktop launcher workflows/layout, campaign
follow-up state, UAT steps, and committed project memory. Companion contracts are
`crates/launcher/desktop/docs/migration.md`, desktop README, the player/macOS
guides, the approved design, and AIP-U1..U5. Existing generated and global status
blocks are outside this follow-up.

Complete fresh Task 1 review 2 and pending platform CI on PR #1992, then follow
the normal ship-pr review/merge process. Task 2 PR #1991 has a ship verdict and
green required CI but still waits for native CI before merge. No merge or release
is claimed. Retire each worktree after its PR merges; never `git worktree prune`.
