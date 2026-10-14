# Harset Wave-0 resume

> Type: reference. Audience: campaign coordinator and reviewers. Updated: 2026-10-14.
> Companions: [ledger](README.md), [packet gates](work-packets.md), [HA-00b worknote](worknotes/HA-00b.md), [execution contract](../four-zone-planning/execution-contract.md).

- Campaign branch: `content/harset-campaign`; registered integration worktree: `zone-harset`.
- Base: `d3efb7d41aad0b6008397f112d224ce7430d7acf`.
- Reviewed tip: `22982c17ac571024c3bf0e95564e2b976dc3ad7c`; the closeout commit is discoverable in this branch's log.
- Integrated packet: HA-00b `37ffd858d73a47e9f68088af0b6cca2c3852c4b0` plus repair `22982c17a`, documentation evidence only.
- Reviews: round 1 fresh Sol packet-reviewer has three findings; separate `/root/harset_fix` fixes all in `22982c17a`; round 2 fresh `/root/harset_rereview` has no findings.
- Static evidence: census 73 unique, 29/39/2/3 dispositions, four carry-ins disjoint; twelve seed PK inventories match; 495 chain candidates collision-free; scratch missing disposition fails then restoration passes.
- Recorded committed-branch scan: 84 refs, 72 differing relevant seed snapshots, no collisions; newer/uncommitted claims still need seal-time reconciliation.
- Closeout checks: changed ledger links/anchors, CRLF/file caps, four-file markdownlint, working/staged diff whitespace checks pass; exact commands/results in HA-00b worknote and ignored report.
- No lane: fmt/clippy/tests not applicable to this docs gate. No cargo, database reload/live-DB, historical revert rerun or client/lab operation.
- Next: reconcile and sign inactive restoration ID proposals; seal and budget one small supported restoration packet with its exact evidence, then measure implementation/check/review cost before larger waves.
- HA-00c: BlockedDependency; stock catalog/CB-N05a evidence and owner/advisor approvals still needed. HA-17/18 require HA-00c AND CB-N05a merged, reviewed and client-proven.
- Runtime blockers: doors/arrivals, encounter/navmesh/containment evidence, 1664 grant/Kaerlia graph, unnamed 618/621 recipients, item4512 source and other owner decisions.
- Cross-zone gates amended 2026-10-14: CS-08a Castle-origin `stargate_crossed` Castle→Harset reconstruction (chain 1366, single ID approved) advances 1360 4037→4038 while retaining 3730. 567 still needs CS-08's item2698 acquisition evidence. Tollana owns its seven Harset recipient graphs.
- Marsh must require the letter before completing 1360 (owner decision, 2026-10-14). [HA-1360a](work-packets.md#ha-1360a--authoritative-letter-turn-in-prerequisite) is BlockedEvidence on DB/authority/mission/testing signoffs for atomicity, stale-writer CAS or proved serialization, recovery/session fences and exact bounded manifest. [HA-1360b](work-packets.md#ha-1360b--marsh-receiving-seed-and-cross-zone-proof) is BlockedDependency on the integrated/reviewed engine prerequisite and CS-08a. Neither is Ready; the compound action/outbox design is a proposal.
- Preserve receipt IDs 6501/6502/6505 and disabled 567 IDs 6503/6504. No new Harset content IDs are allocated. Cross-zone regression must prove committed inventory/mission state through the real bridge, with independent seed/engine/CS-08a revert failures and authorized client UAT pending.
- Historical artifacts stay UATPending or blocked; five seeded mission graphs do not establish playability, and 1326 still depends on missing 1325.
- Client UAT pending; obtain owner approval before a lab lease or any client/server change. No remote PR/push or campaign completion is recorded.
- Measurement debt: actual working/context tokens and elapsed implementation/check/review time remain unavailable; coordinator token profiling must fill them, not estimates.
