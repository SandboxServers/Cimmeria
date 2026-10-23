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
Task 1 is [PR #1992](https://github.com/SandboxServers/Cimmeria/pull/1992), open/ready:
setup preservation fix `1cbfc54f5` and helper-unavailable UI fix `ed6940107` are complete; fresh review 4 returned ship with no findings. Post-setup integration reruns pass; latest native macOS CI reports nine failures; fresh review 6 returned ship and the restored full engine passes 969 tests (3 ignored); final shell/workspace/native-build and native-backed UAT passes are recorded below; next macOS CI remains pending.
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
backup behavior as schema-1 only. All six fixes were complete at this earlier checkpoint; subsequent reviews and
their final acceptance are recorded below.

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
their passing post-setup repeats are recorded in the final checkpoint below.

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
below; its fix is complete and fresh review 4 returned ship.

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
results. No mutation remains. Fresh review 4 subsequently returned ship.

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
Light REPL also passed. The final post-setup repeats pass as recorded below.

## Final local integration and review checkpoint

Fresh read-only review 4 returned **ship**, with no fix notes, for
`7be8413b0..0829e5f1a`. Its report
`.claude/reports/launcher-adopt-in-place/review-4.md` accepts the setup expectations,
prior safety fixes, legacy interruption escape, and helper-unavailable footer
behavior. The reviewer read the validation reports rather than rerunning tests.

The coordinator's post-setup integration checks passed:

| Check | Result | Evidence |
|---|---|---|
| Desktop shell tests | 147 passed, 7 ignored | `20261022-233034-250280` |
| Native-backed adoption UAT | Passed again | `npm run uat:adoption`, production host with isolated store and signed loopback |
| Workspace all-target clippy, `-D warnings` | Clean | `20261022-233200-159608` |
| Production desktop debug build | Passed, 24.286 seconds | `20261022-233219-365080` |

Final formatting import-order correction is `6e733e12e`. The earlier remote CI
at `64985f11a` failed formatting only in shell conditional-import ordering; that
failure is fixed by this commit, and engine-fix formatting was unaffected. Latest
native macOS CI is still pending after the final push; no green platform result
is inferred from the local Windows checks. PR #1992 remains draft and unmerged
at this checkpoint. Ship through CI/merge queue and verify worktree retirement
before recording implementation complete. No release was performed.

## macOS CI test-helper gate checkpoint

After `89d24ab36`, CI run `39900989143` failed macOS clippy solely for unused
`restore_setup_originals`: the test-only legacy helper was compiled on macOS,
while its only callers in `setup_originals_tests` were excluded there. The fix
aligns the helper's attribute to `#[cfg(all(test, not(target_os = "macos")))]`
in `storage/adoption/in_place_check.rs`; production code is unchanged.

The cfg-only fix is committed as `a36a5f5f5`. Engine all-target clippy passes
(`20261022-235944-125636`); targeted setup-original tests pass 7/7 with no
failures (`20261023-000009-174576`); fmt passes (`20261023-000042-223524`).
Independent review 5 returned ship: `review-5.md` checked the exact one-line
cfg gate and its callers, confirming production behavior is unchanged.
Corrected CI run `39903463069` is queued behind `39900989143`; native macOS
CI for the corrected head remains pending. Review 4's ship verdict
still records the previously reviewed implementation, not acceptance of an
uncompleted CI run. No native macOS runtime result is claimed.

The older queued workflow `39899489669` ended with all packages passing. Run
`39900989143` still has packaging and Windows checks in progress at this
checkpoint; its macOS failure above remains an actual gate failure until a
corrected run passes. Earlier local integration results and Task 2's merged and
retired status remain unchanged. No release was performed.

## macOS test execution and compatibility follow-up

PR #1992 is now open/ready, not draft. Actual macOS run `39903893543`, job
`120257546227`, reports **915 passed and 9 failed**. Earlier queued/pending notes
are historical checkpoints, not evidence that the Unix guards passed.

The coordinator's four-file fix under review addresses three causes:

- Six Unix retained-parent fixtures need canonical temporary roots: macOS `/var`
  and `/private/var` name the same storage, while production plans already hold
  canonical paths. Correct the fixture, without weakening the safety assertions.
- The updater-busy test selected now-unsupported public Copy. Use supported Light
  and run the contract on every host.
- Two legacy settings/minimum tests exposed a real compatibility regression:
  unconditional adoption-plan verification in `installed_for_launch` hid the
  schema-1 owner and launcher minimum when only imported settings were bad.
  Restrict the new verification there to schema 2, retaining schema-1's prior
  read-only identity/minimum view. `effective_settings::launch_binding` remains
  strict before Play; this does not admit refused settings. Schema-2 retained-plan
  provenance verification remains mandatory.

The cross-platform guard
`legacy_owner_minimum_survives_refused_settings_without_admitting_them` passes
(`20261023-003711-284512`). Mutating back to unconditional verification makes it
fail (`20261023-003829-293908`); the coordinator restored source in a finally
block. Restored full engine run `20261023-003909-271772` passes 969 tests with
3 ignored in 64.676 seconds. Independent `review-6.md` returned ship, confirming
legacy read-only owner/minimum identity checks remain intact, schema-2 provenance
stays strict, and settings are still validated before Play. Final integration passes: shell 147 tests with 7 ignored
(`20261023-004023-392512`), desktop workspace all-target clippy
(`20261023-004036-97380`), and native production debug build
(`20261023-004045-181756`, 8.468 seconds). Actual `npm run uat:adoption`
passes again after the compatibility fix, using the production host, isolated
store and signed loopback; it excludes Tauri, physical picker and Wine.
Latest fmt check `20261023-004122-422112` is still running at the documentation
checkpoint; no outcome is assumed. No Mac pass is inferred from Windows checks.
Native macOS CI must run again after the next push. No new owner decision is needed.

Earlier Windows native CI passed in 22 minutes 47 seconds. Earlier package jobs
passed on Windows/macOS; the old `89d24ab36` macOS native warning was fixed by
`a36a5f5f5`. Those results do not supersede this new nine-failure macOS test gate.
No real native macOS guard execution is claimed successful until corrected CI
passes. No release was performed.

## macOS engine pass and shell compatibility follow-up

Run `39907274967` establishes actual native macOS engine execution: **925 passed,
24 ignored**, including all six Unix retained-descriptor substitution guards,
the legacy owner/minimum guard, and the updater Light Busy check. Frontend
250 tests, REPL and UI build pass. The ignored tests are not executions. This
supersedes earlier uncertainty about those engine branches, not the real-client
or Wine limitations below.

Mac shell job `120266139979` reports **157 passed, 3 failed**. Two failures concern
legacy installation status after an imported-settings edit or missing historical
adoption checkpoint. Newly unconditional `verify_provenance` called strict
`published_plan` for every schema. Restore the original `published_record` check
for authenticated schema-1 records only; schema-2 still requires `published_plan`,
and effective-settings validation remains strict before Play. Installed-content
and Uninstall display must retain the authenticated legacy identity without
admitting rejected settings or authorizing unvalidated mutations.

The extended cross-platform legacy guard verifies installed-content/Uninstall
display under both bad settings and missing checkpoint and passes locally
(`20261023-010758-304172`). The third Mac-only test expected the now-prohibited
legacy-copy Recover. Its replacement in shared `legacy_binding_tests.rs`,
registered on both hosts, requires `RecoveryRequired`, metadata-only Abandon,
no settings binding, and preservation of all content except the exact owner and
ready metadata intentionally removed. That new guard passes locally
(`20261023-010925-421728`).

Independent `review-7.md` returned ship with no findings. Restoring strict
`published_plan` for schema-1 provenance made the legacy guard fail at
`installed_content` (`20261023-011012-378384`); source was restored in a finally
block. Final post-review-7 local integration passes:

| Check | Result | Evidence |
|---|---|---|
| Full engine | 969 passed, 3 ignored; 64.430 seconds | `20261023-011056-248396` |
| Desktop shell | 148 passed, 7 ignored, including the shared legacy-binding guard | `20261023-011202-315260` |
| Desktop workspace all-target clippy | Passed | `20261023-011216-407232` |
| Native production debug build | Passed, 11.821 seconds | `20261023-011224-155080` |
| Fmt | Passed | `20261023-011239-182448` |
| Native-backed adoption UAT | Passed again after all fixes | `npm run uat:adoption` |

The UAT still uses the production host, isolated store and signed loopback;
it does not exercise Tauri, the physical picker, Wine or a real game.
Corrected Mac CI must run on the next pushed head. Its current engine/frontend
passes and three shell failures remain the actual CI state, not an overall pass. No new
owner decision is required. The complete Mac job is not green; corrected CI must
run again. No release was performed.

## macOS engine and shell pass; production lint follow-up

Mac run `39910259596`, job `120276089926`, now passes the engine (**925 passed,
24 ignored**) and shell (**160 passed, 17 ignored**), including all three previous
shell failures. Frontend 250 tests, persistence REPL and UI build pass. Ignored
checks remain unexecuted; these passes do not establish real Wine/game behavior.

Production macOS clippy still reports unused `legacy_import`,
`legacy_choice_allowed`, and the shared IPC enum's `ImportRequired` variant.
The coordinator's narrow fix gates the two helpers with
`#[cfg(any(test, not(target_os = "macos")))]` to match their callers. The IPC
variant is retained, with `expect(dead_code)` only for non-test macOS builds;
no wire variant or runtime behavior is removed. Two stale comments about macOS
copying/in-place moving are corrected alongside the attributes.

Independent `review-8.md` returned ship: the helper gates match their callers,
the shared IPC variant remains with a narrow expectation, and runtime behavior
is unchanged. Final local checks pass: shell 148 with 7 ignored
(`20261023-013855-191772`), workspace all-target clippy
(`20261023-013906-320520`), native build (`20261023-013911-218704`), and fmt
(`20261023-013918-79376`). Engine 969/3 ignored remains the last local engine
result. Mac engine 925/24 ignored, shell 160/17 ignored, frontend 250/persistence
REPL/UI build are verified in the temporary `cimmeria-adoption-mac-clippy.log`.
Native macOS production clippy and final executable build remain pending on
the corrected next head. The corrected next-head Mac production
clippy run must still pass before overall CI can be recorded green. No new
owner decision or real release is involved.

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
