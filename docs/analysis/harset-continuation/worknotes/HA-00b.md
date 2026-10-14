# HA-00b: open the Harset continuation ledger

> Type: worknote. Audience: four-zone coordinator and independent reviewer. Date: 2026-10-14.
> Branch: `harset/ha-00b-ledger`; base: `d3efb7d41aad0b6008397f112d224ce7430d7acf`.

## Result

Opened [the continuation ledger](../README.md), [current packet gates](../work-packets.md), [placement evidence](../placements.md) and [ID candidates](../id-allocation.md). Added one forward pointer to the historical rebuild README. No old decisions or shared four-zone ID table changed. HA-00b awaits coordinator review/integration; it has produced the evidence contract, with unresolved items marked BlockedEvidence/BlockedDependency/BlockedDecision. No runtime packet is advertised as Ready.

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

The coordinator integrates exact ID proposals into the shared table, reconciles sibling uncommitted claims, and signs the remaining per-packet manifests. 1664 stays provisional until Castle's trigger and Harset's recipient graph both close. 1360/567 need CS-08 before end-to-end completion; 567 also needs item2698 evidence. 618/621 need exact recipients or approved reconstruction. HA-01a and encounter graphs need doorway/placement evidence; no model may guess them from an available range.

Reusable verified corrections (once semantics, current navmesh lineage and candidate/recipient caveats) are recorded in the ledger; the coordinator can graduate them to shared memory/index pages during integration, outside HA-00b's allowlist. Full audit report is gitignored at `.claude/reports/four-zone/ha-00b.md` in this registered worktree.

Measurement: this is a specialist evidence gate, not a Haiku implementation pilot. A scoped cost estimate was not recorded in the dispatch brief; actual harness token totals and full elapsed/check time are unavailable to this worker and must be filled by coordinator token profiling. No estimate is represented as measured usage. No implementation pilot or reviewer/fixer round is counted.

## Final validation

- ZIP/census: 73 unique IDs, titles and relation fields match; four carry-ins disjoint.
- Global seed primary-key/candidate checks: pass; all 495 chain candidates and other proposed IDs collision-free in current seed and inspected registered committed branch snapshots.
- Introduced relative links: 25/25 resolve. Five new files use CRLF. The historical README differs only by the permitted forward pointer and blank separator.
- `npx --yes markdownlint-cli2 --no-globs` on five new files plus historical README: six files, zero issues, exit 0. The PowerShell wrapper's first invocation unintentionally used its whole-tree config (4,805 existing issues); the direct scoped invocation supplies the relevant result. The initial pointer blockquote lint issue was corrected.
- `git diff --check`: clean. No generated marker content changed.
- Historical README has a pre-existing broken link to the pre-crate-split services executor; this packet leaves it untouched under its one-line-only allowlist. All introduced links resolve.
- No runtime/build/database/client validation was performed.
