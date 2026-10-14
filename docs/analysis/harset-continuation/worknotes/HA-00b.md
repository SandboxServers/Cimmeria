# HA-00b: open the Harset continuation ledger

> Type: worknote. Audience: four-zone coordinator and independent reviewer. Date: 2026-10-14.
> Branch: `harset/ha-00b-ledger`; base: `d3efb7d41aad0b6008397f112d224ce7430d7acf`.

## Result

Opened [the continuation ledger](../README.md), [current packet gates](../work-packets.md), [placement evidence](../placements.md) and [ID candidates](../id-allocation.md). Added one forward pointer to the historical rebuild README. The original packet changed no old decisions or shared ID table. The fixer records Harset-only inactive restoration proposals in that table. HA-00b awaits coordinator review/integration; it has produced the evidence contract, with unresolved items marked BlockedEvidence/BlockedDependency/BlockedDecision. No runtime packet is advertised as Ready.

## Reconciliation evidence

- Reviewed all 73 unique census records: 29 core, 39 older-revision holds, two Tollana conditional handoffs, three offworld holds. Preserved four unique non-ZIP records (567/1360/1377/1410) for 77 distinct reviewed IDs. Mission 1664 is a separate cross-campaign interface, not a hidden addition to the 33.
- Inspected all four Harset chain seeds and current replay/side-effect guard bodies. Existing early graphs are 742, 1200, 1324, 1326 and 1361. No new graph was inferred from an enum, old packet status or mission catalog row. 1326 remains dependent on absent 1325 completion.
- Reconciled every actual old H01–H55 section and H99, recording absent packet numbers. None is newly marked Done. Existing implementation remains UATPending; incomplete graphs keep explicit evidence gates. H05 still uses constant competency/ability mask despite implemented difficulty. `remove_effect` still has no executor arm.
- Confirmed 6501/6502/6505 and disabled 6503/6504, exact Castle-owned 1360/567 step interfaces and item IDs. The 1360 receiving guard rejects step 4037. Current 6007 and 1361 acceptance/abandon pair are enabled, pinned by the biconditional test; stale historical M0 blockers are not inherited.
- Signed off existing exact Harset actor tags/world/spawn/template for Tollana's named recipients. 618's “leadership” and 621's blank recipient remain unassigned. Distinct Tollana Copplemann template/tag is mandatory because dialog bindings key on templates.
- 1664's 5224–5226/6063/6059–6062 and Texts 27308–27315 support Harset geography. Dialog 6169/DSM7016/speaker3219 is a candidate, not a proved Kaerlia join. No template references3219; Castle's exact accept trigger, Kaerlia placement and faction/objective graph are still unsealed.
- Reconciled NA26/NA29/NA36/NA40: all interiors now have meshes; all four worlds are advisory; temporary gate offset removed; five world-57 spawns now mobile. Current tests replace the old 1,939-component/no-interior-mesh premise. Raised telemetry positions/spawn308 still need evidence. PL-C-E12 is 0.3 m outside set2122's Z minimum; old blanket inclusion claim is not adopted.
- Corrected the historical engine briefing: `once` is per cell-entity life since #802; persistent guards still required. All matching chains execute from resolved conditions, with nested mission events able to see new state during action execution.

## Static checks

| Check | Result |
|---|---|
| ZIP entry identity | 73 unique IDs/titles/evidence relations match HA-00a; entry SHA-256 `237bd359bb06efe9246656087410c8058aa8183da5b4c5a2cb2ac631b86064a8`; no build column or SQLite entry |
| Census uniqueness/count/dispositions | 73 rows / 73 unique; 29/39/2/3; four carry-ins unique and disjoint |
| Seed PK uniqueness | Chain 349, spawn 632, set 101, point 202, template 449, DSM 4673, dialog 5418, item 6074; mission1041, step3480, objective4037 and task4358; each equals its distinct-key count |
| Candidate collisions | Chains 6008–6032; spawns360–379; templates260–269; sets2130–2159; points2560–2679 absent from current seed; historical Bar/pens sets2108/2109 retained |
| Registered committed branch scan | 85 refs; 72 had differing relevant seed files; zero proposed candidate collisions. Uncommitted sibling claims remain coordinator's integration gate. |
| Actual baseline artifacts | First-wave commits 00d4f49a/84ce4eda/34e297f0/90593af7/03ad0fc1 are ancestors. Later recorded H55/H53/H54 SHAs are not ancestors, so current artifacts/guards, not historical commit labels, establish presence. |
| Relative links, CRLF, markdownlint, diff | 25 introduced relative links resolve; five new files CRLF; scoped markdownlint six files clean; git diff --check clean |

The parser reads named columns and multi-row VALUES with quoted commas. The preliminary first-column point scan was discarded: point_set_points primary key is point_id (second column). The final primary-key scan found no duplicate points. No parser/tool was added to the repository.

## Test and UAT limits

No cargo/build/lane, database reload/live-DB test, client UAT, lab lease, client launch or server change occurred. Existing guards were inspected for exact action/side-effect and negative controls; historical worknotes record their revert proofs. HA-00b did not rerun those proofs and makes no fresh Rust pass or integrated-playability claim. Docs-only checks establish the ledger's consistency, not mission execution. A fresh independent review and any fix rounds belong to the coordinator.

## Handoff

The coordinator signs the recorded inactive restoration proposals, reconciles sibling uncommitted claims and seals the remaining manifests. HA-00c owns authored/repeatable catalog allocation; HA-17/18 depend on HA-00c AND CB-N05a. 1664 stays provisional until Castle's trigger and Harset's recipient graph both close. 1360/567 need CS-08 before end-to-end completion; 567 also needs item2698 evidence. 618/621 need exact recipients or approved reconstruction. HA-01a and encounter graphs need doorway/placement evidence; no model may guess them from an available range.

Reusable verified corrections (once semantics, current navmesh lineage and candidate/recipient caveats) are recorded in the ledger; the fixer adds the continuation link and a dated sourced shared-memory/index pointer under the amended documentation scope. Full audit report is gitignored at `.claude/reports/four-zone/ha-00b.md` in this registered worktree.

Measurement: this is a specialist evidence gate, not a Haiku implementation pilot. A scoped cost estimate was not recorded in the dispatch brief; actual harness token totals and full elapsed/check time are unavailable to this worker and must be filled by coordinator token profiling. No estimate is represented as measured usage. No implementation pilot or reviewer/fixer round is counted.

## Final validation

- ZIP/census: 73 unique IDs, titles and relation fields match; four carry-ins disjoint.
- Global seed primary-key/candidate checks: pass; all 495 chain candidates and other proposed IDs collision-free in current seed and inspected registered committed branch snapshots.
- Introduced relative links: 25/25 resolve. Five new files use CRLF. The historical README differs only by the permitted forward pointer and blank separator.
- `npx --yes markdownlint-cli2 --no-globs` on five new files plus historical README: six files, zero issues, exit 0. The PowerShell wrapper's first invocation unintentionally used its whole-tree config (4,805 existing issues); the direct scoped invocation supplies the relevant result. The initial pointer blockquote lint issue was corrected.
- `git diff --check`: clean. No generated marker content changed.
- Historical README has a pre-existing broken link to the pre-crate-split services executor; this packet leaves it untouched under its one-line-only allowlist. All introduced links resolve.
- No runtime/build/database/client validation was performed.

## HA-00b fixer verification, 2026-10-14

The amended [HA-00b contract](../../four-zone-planning/harset-work-packets.md#ha-00b--open-the-continuation-ledger-and-lock-restoration-id-proposals) covers ledger reconciliation and restoration ID proposals. [HA-00c](../../four-zone-planning/harset-work-packets.md#ha-00c--seal-authored-and-repeatable-id-allocations) separately seals all six authored stories and three repeatables after catalog, graph and owner gates. HA-17/18 remain BlockedDependency on HA-00c AND CB-N05a merged, reviewed and client-proven. No guessed stock catalog ID closes either gate. HA-00b remains Review until the coordinator's fresh review/signoff; local integration alone does not certify the wave.

Catalog search: `tools/README.md` and `crates/*/src/bin/` expose no Harset census/claim validator; existing census tools inspect map archetypes, ability seed tooling validates its workbook, and token-profile validation measures attribution. A bounded PowerShell check was saved only in gitignored `.claude/reports/four-zone/validate-harset.ps1`; no repository tool was added. It reads INSERT column names, scans VALUES while respecting quoted/escaped strings and tuple depth, and uses point_id (second column). Its command runs from the integration worktree: `pwsh -NoProfile -File <worktree>\.claude\reports\four-zone\validate-harset.ps1 <worktree>`.

Scratch corruption proof uses the **same** `CheckCensus` function as the passing census check, on an in-memory copy of the actual 73 CSV records. Exact mutation: mission 571 `decision` changes from `HOLD_OLD_REVISION` to empty. Exact failure: `status=expected_failure scratch mission_id=571 decision emptied: census disposition HOLD_OLD_REVISION`. Restore 571's original disposition and rerun that checker: `status=passed scratch restored same checker`. The committed CSV was never changed. This proves the checklist detects a missing disposition rather than merely asserting an expected count.

Fresh primary-key validation matched the original counts: chain349, spawn632, set101, point202, template449, DSM4673, dialog5418, item6074, mission1041, step3480, objective4037, task4358; each equals its distinct-key count. All 495 exclusive chain candidates and proposed spawn/template/set/point IDs are absent from seed keys. Census: 73 unique; 29/39/2/3; four carry-ins disjoint. Static checks do not certify client visibility or mission execution.

Historical scan wording corrected: Castle `d5b6790616dad0df2fe1281e25e4a5554e721c5e` and Cellblock `b59c4a1245f354d8de58e9c83b5c8d603c85446a` were docs-only descendants of base `d3efb7d41aad0b6008397f112d224ce7430d7acf`, not identical integration heads. Their relevant seed snapshots match that base. Fresh refs/counts, link/CRLF/lint/diff results and the fixer commit are in the gitignored `.claude/reports/four-zone/harset-fix.md`; later sibling changes still require a seal-time reconciliation.

Ordered lane checks: `status=not_applicable fmt`, `status=not_applicable clippy`, `status=not_applicable tests`: the amended HA-00b acceptance explicitly says no cargo for this documentation gate. No Rust/seed change, lane result, live database pass or client proof is claimed.

Fresh committed-state recheck froze all 84 registered branch SHAs before reading their snapshots; 72 differed in relevant seed files, with zero candidate collisions. Four integration heads at that snapshot: Castle `1ebdff94fa4df3cb28fd9e5b4c83cfd63e7044bf`, Cellblock `457b4f75a57b74c6350fc29d93b5fa3f9df3ec5d`, Harset `37ffd858d73a47e9f68088af0b6cca2c3852c4b0`, Tollana `b105967cc7743601589fc5f304b581f3ca0ff9dd`. That count differs from the historical 85-ref scan because registered worktrees change. Relevant seed changes were inspected through Git objects only. Uncommitted/newer sibling claims remain outside this snapshot.

Final documentation recheck: introduced relative paths and heading anchors resolve; all changed docs are CRLF and under 700 lines; scoped Markdown lint reports nine files and zero issues; `git diff --check` passes. Four pre-existing memory-index blank-line warnings were repaired while adding its new pointer. Existing historical doc warnings are not runtime test results.
