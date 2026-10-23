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
Task 1's follow-up has no PR yet. Task 2 is [PR #1991](https://github.com/SandboxServers/Cimmeria/pull/1991),
in review. Task 1 review fixes N1-N5 are in progress; the results below are the pre-fix baseline and affected checks must be rerun. These are independent task worktrees, not unfinished original waves.

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

## Reported validation evidence

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

## Regression proof

Restoring bulk relocation into `root/game` made the adoption location invariant
fail (lane log `20261022-205443-44468`). Restoring the fix made the targeted check
pass (`20261022-205550-94624`). The guard covers unchanged matched-file location
and identity, so a copy/delete substitute does not satisfy the invariant merely
by preserving bytes.

The frontend agent separately mutated visibility and first-feedback behavior:
5 of 6 targeted checks failed. Restoring the fix made all 6 pass. This is evidence
for those guards; it does not claim mutation testing of every added assertion.

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

Complete review fixes N1-N5, rerun affected checks, finish review and pending platform CI, then open Task 1's PR with these results and
limits, and complete the normal ship-pr review/merge process. Task 2 continues in
PR #1991. Retire each worktree after its PR merges; never `git worktree prune`.
