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
Task 1 is [PR #1992](https://github.com/SandboxServers/Cimmeria/pull/1992), draft:
setup preservation fix `1cbfc54f5` and helper-unavailable UI fix `ed6940107` are complete; fresh review 4 is in progress. Integration reruns after those fixes remain pending.
Task 2 [PR #1991](https://github.com/SandboxServers/Cimmeria/pull/1991) merged as
`7be8413b0`; its worktree was retired at 2026-10-23T03:38:43Z. Required checks,
native Windows/macOS/package CI, and merge-queue full tests passed for Task 2.
No actual release was performed.

## Contract under validation

New adoption uses matching schema-2 plan and intent records, without adding a
field to the digested `Plan`. Existing ownership/provenance/journal checks precede
content-root resolution. Schema-2 content remains at the selected root;
successful schema-1 managed/legacy installations keep `root/game`.

Matched originals retain their paths and file IDs unless an approved individual
content transformation requires replacement. Direct-root maintenance commits per
file and preserves extras. Selective Uninstall retains the root and unrelated
files. All interrupted schema-1 adoption, including legacy copy journals, refuses
recovery and offers metadata-only abandon,
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

## Integrated safety follow-up

The historical blocked status in `.claude/reports/launcher-adopt-in-place/fix-fd-2.md`
is resolved by subsequent integration. Shared retained-directory guards now cover
adoption, Repair, Update, rollback, and selective Uninstall. The authenticated
owner's file identity binds the selected root before descendants are created or
mutated. Tree-identity and content-ready publication also use retained parents
bound to that owner, closing the post-promotion pathname seam identified in
`fix-consumer-safety-2.md`; `fix-metadata-3.md` records the resolution.

Unix operations use descriptor-relative `openat`/`mkdirat`/`unlinkat` and native
exclusive rename primitives with retained parent synchronization. The former
Darwin volfs translation is superseded; no volfs compatibility claim remains.
Windows retains ordinary directory/file handles without delete sharing and
executes rename-refusal and junction safety assertions. Unix descriptor
substitution tests still need native macOS CI execution; Windows results do not
stand in for those branches.

Repair/Update backups now use a journal-owned partial copy, verification/sync,
and exclusive publication before changing live output. Empty or prefix partials
can recover only against the complete expected live original; malformed final
backups and changed player content refuse. Legacy schema-1 copy recovery is also
explicitly refused by `23b7374a4`; metadata-only abandon preserves all content.
Successful legacy adoption keeps its recorded layout.

Latest integrated checks: engine **962 passed, 3 ignored**, log
`20261022-230703-59764`; all-target engine clippy clean,
`20261022-230654-408180`. These supersede earlier engine totals without erasing
the earlier regression evidence. Those integration checks passed before the later setup fix, as recorded below;
their post-setup repeats remain pending at this checkpoint.

Review-2 partial-backup mutations failed with the old final-copy behavior:
`20261022-223819` and `20261022-225603-482080`. The latter failure and restored
scoped passes are documented in `fix-consumer-safety-2.md`; an older-timestamp
restoration that reused a mutant executable is explicitly superseded there.
Replacing receipt owner binding with a root-only constructor failed the actual
wrong-owner guard at `20261022-230640-317548`; restoration precedes the full
962-pass suite. A preliminary zero-test invocation is not regression evidence.

Initial macOS PR CI failed because a Windows-only fixture was imported in
`provenance_tests`; conditional gating was fixed in `094f8e904`. Latest Task 1
macOS CI is pending, not passed. Fresh review 3 found the setup-preservation gap
below; its fix is complete and fresh review 4 remains pending.

The profiler's post-merge attempt rejected unsupported transcript shapes and did
not post cost statistics. That reporting limitation does not change test results.

## Setup-preservation fix and latest checkpoint

Fresh review 3 identified a setup-stage player-edit preservation gap, resolved in
`1cbfc54f5` (`.claude/reports/launcher-adopt-in-place/fix-setup-4.md`). A separate
versioned expectation record is bound to the pinned plan digest, recording exact
original and approved states: presence, size, SHA-256, and filename casing. ASLR
expectations are derived on an isolated temporary file, not the live executable.
Recover checks every setup target before any setup write; Abandon checks every
target before any rollback write. Later edits to a still-parseable `SGW.exe` or
`LoginInternal.lua`, missing expected targets, or newly appeared targets refuse
without overwriting the player's content or saved originals. If partial or full
original snapshots exist without the expectation record, recovery refuses rather
than re-pinning expectations to later edits. Exact original and approved states
remain retryable. Filename normalization is covered explicitly.

Six production Light interruption journeys passed (`20261022-232350-370448`).
Disabling the original/approved comparison caused four runtime guard failures
(`20261022-232444-389596`); restoring it preceded final engine **968 passed,
3 ignored** (`20261022-232513-157888`), fmt `20261022-232502-457064`, and clean
all-target engine clippy `20261022-232506-312140`. These are the latest engine
results. No mutation remains. Fresh review 4 is pending, not a ship verdict.

Before that setup fix, the identity/layout mutation failed two guards with zero
success events (`20261022-231123`), then restored checks passed 4/4
(`20261022-231137`). Shell tests passed 147 with 7 ignored (`20261022-231348`)
after correcting an old legacy-copy expectation in `64985f11a`. Actual native-backed
`npm run uat:adoption` passed again; workspace clippy passed
(`20261022-231456`) and production debug build passed in 49.276 seconds
(`20261022-231503`). These passes predate the setup fix and must not be presented
as its final integration rerun.

After the helper-unavailable UI fix `ed6940107`, the root frontend pass reports
250 tests, footer REPL including helper unavailability, and build passing;
`cimmeria-adoption-frontend-final.log` is its temporary log basename. The earlier
Light REPL also passed. Final shell/workspace/build and native-backed UAT repeats
after the setup fix are still running at this checkpoint.

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

Check PR #1992 on GitHub before resuming: if still open, finish fresh review and
CI, then complete the ship-pr process. If merged, verify its worktree retirement
and treat implementation as complete; remaining owner UAT is recorded separately.
Task 2 is merged and retired. A real release requires explicit owner GO.
Never `git worktree prune`.
