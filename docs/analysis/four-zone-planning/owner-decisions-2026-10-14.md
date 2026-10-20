# Four-zone campaign owner decisions — 2026-10-14

> **Type:** Reference
> **Audience:** Cellblock, Castle, Harset and Tollana campaign coordinators, packet authors and reviewers
> **Companions:** [planning index](README.md), [execution contract](execution-contract.md), [shared ID table](id-allocation.md)

This register records the owner's decisions in the four-zone campaign session on
2026-10-14. It is a standing authorization for the named campaign coordinators;
they do not need to ask the owner again for an action that stays within its stated
bounds. The [execution contract](execution-contract.md), zone ledgers and normal
repository review, CI and merge-queue rules still govern execution. A decision
authorizes work; it does not assert that the implementation or client UAT passed.
Earlier 2026-10-13 planning decisions remain in the [planning index](README.md#owner-direction).

## Campaign operation

| ID | Owner decision | Boundary and record required |
|---|---|---|
| O14-01 | Start the initial four-zone campaign work and unblock parallel restoration. Use the lean campaign principles while that orchestration work is still in progress. Choose the cheapest **capable** model separately for packet implementation, read-only review and review fixing; do not trade away code or output quality. | Seal a bounded packet before giving it to a small coder. Give architecture, ambiguous evidence, adversarial review and complex fixes to capable specialists. Keep one worker worktree/test database per packet, independent reviewer/fixer rounds, measured pilot and the one-PR-per-zone topology. |
| O14-02 | Create a new main session for each of Cellblock, Castle, Harset and Tollana so each can autonomously run its packets and wrap its campaign. | Each session uses its existing `content/<zone>-campaign` integration branch and continuation ledger. It coordinates cross-zone contracts and the shared lab before dependent work. |
| O14-03 | A zone coordinator may approve a **bounded Cimmeria reconstruction** for missing 2009 wiring after source review and domain signoff, without returning for each trigger, placement or dialog. | Mark every inferred beat `CIMMERIA_RECONSTRUCTION` in the seed/ledger and cite the supported historical goal plus the inferred link. Do not claim an incompatible or unproved revision as restored source. |
| O14-04 | A zone coordinator may allocate individual mission, chain, spawn, dialog and item IDs inside approved zone ranges. | Before allocation, document a collision audit of the stock client catalog, shared ID table, seeds and all active zone branches. Record the allocation in the zone register; do not treat a proposed range as a blanket reservation. |
| O14-05 | Coordinators may exchange task messages about cross-zone contracts, integrated SHAs, blockers and lab scheduling with this coordinating session and one another. | A sending session identifies the dependency and owner; no zone silently duplicates another's chain or recipient. |
| O14-06 | After its complete campaign checks, fresh review and required UAT accounting, a zone session may push its one campaign PR and merge it through required CI and the merge queue without another owner approval. | Preserve the independent fixer/fresh-reviewer gate. An observed gameplay failure blocks merge. Retire packet and integration worktrees through the repository workflow after merge. |
| O14-07 | A campaign may close with historical mission rows on an explicit evidence hold when compatible build lineage cannot be proved. | Finish the approved active scope. Name each held row, missing evidence and consequence in the zone ledger and PR; do not label a held row Done. |

## Lab and client evidence

| ID | Owner decision | Boundary and record required |
|---|---|---|
| O14-08 | Standing approval for each zone session to take the lab lease, start or restart the client, and point the lab at its reviewed build for UAT. | Coordinate one lease at a time and follow the lab-uat skill and contributor setup. The standing approval satisfies their ask-the-owner gate for these four campaigns; record the build, lease, steps, server evidence and release of the lease. |
| O14-09 | If the lab remains unavailable after documented attempts, a fully reviewed zone PR may merge with specific client rows marked `UATPending`. | Record what was attempted and the exact unrun rows. A failed or observed broken gameplay step is a blocker, not `UATPending`. No seed or replay test is described as client proof. |

## Authored content, balance and pacing

| ID | Owner decision | Boundary and record required |
|---|---|---|
| O14-10 | A coordinator, documentation writer and domain reviewer may approve lore-safe authored quest premises, exact copy, IDs and modest rewards within the existing target of about 5–8 stories per large hub. | Record `CIMMERIA_AUTHORING` provenance in seeds/ledger. The first new mission remains a live-client catalog proof; hold further authored-quest implementation until its journal, dialog, advance and relog behavior passes in the client. |
| O14-11 | New Cimmeria-authored quests have **no player-facing provenance label**. | Seed and ledger still mark the authored origin; avoid copy that presents it as verbatim 2009 material. This supersedes the open player-label question in the 2026-10-13 plan. |
| O14-12 | Coordinators and domain reviewers may choose conservative ambient cast density, safe placements, faction vendor/trainer stock and reward ceilings using comparable seeded tiers and measured client performance. | No new currency or high-tier gear. Record placement/performance and economy reasoning in the relevant packet. |
| O14-13 | Castle, Harset and Tollana coordinators may define persisted per-character repeatable cooldowns, reward limits and anti-farm guards using existing engine patterns, with economy and authority review. | Review and test relog, duplicate reward and multi-character boundaries; document each reset rule. Cellblock remains one-time. |
| O14-14 | Cellblock's first-run core escape pacing target is **30–45 minutes**; optional content adds **at most 10 minutes**. | The old 37-minute, five-death privileged run is evidence, not a target. Measure live first-run pacing before accepting optional additions. |
| O14-15 | Cellblock's second optional aid choice and one-room reactive encounter are conditional. | Include each only if live pacing and safety tests show it improves the escape; otherwise close its packet as intentionally omitted with the test evidence. |

## Zone and cross-zone story contracts

| ID | Owner decision | Boundary and record required |
|---|---|---|
| O14-16 | For Castle's Frost's Letter, use the **Castle→Harset stargate crossing** to advance mission 1360 from step 4037 to 4038. Keep item 3730 until Marsh receives it. | This is a `CIMMERIA_RECONSTRUCTION` because no 2009 trigger was recovered. Castle CS-08a owns the crossing; Harset owns receipt. The Castle bridge alone is not proof of completed hand-in. |
| O14-17 | Harset Marsh's hand-in must **require possession of Frost's Letter** before mission 1360 completes. Expand into separate Harset/engine packet work. | Make consumption and completion one safe transaction, with refusal and stale-writer/recovery guards. Do not preserve the existing complete-without-letter behavior. |
| O14-18 | Reserve **chain ID 1366** for the single Castle CS-08a bridge, following the documented collision scan. | This signs off that individual chain, not the rest of a proposed Castle range. Recheck collisions before any other allocation. |
| O14-19 | Make mission **1360 one-shot**. | Remove its seed-level repeat permission as part of Harset's receipt work and test the persisted one-shot lifecycle. This narrows the safe hand-in design; it does not waive the atomic consume/completion and stale-writer checks. |
| O14-20 | Tollana's specialist may select one coherent active arc from its 23 local and seven cross-zone candidate missions after source/map review, reconstruct inferred links and leave other historical rows held. | Record which rows are active or held and the evidence for each. The 51-row census does not by itself prove a compatible chronology. |
| O14-21 | If Tollana's proposed server-side phase visibility primitive proves unsafe for the client, specialists may seal a simpler reviewed server-authoritative reconstruction. | Preserve mission 621's objective and two-player isolation, with client and authority review; do not apply an unreviewed client patch. |
| O14-22 | The Castle coordinator may approve and allocate individual restoration-chain IDs in **1367–1400** after the documented cross-branch collision audit, without asking for each ID. | This delegates allocation inside that Castle block, not automatic ownership of every number. Record each actual claim in the shared table and Castle ledger. Chain 1366 remains exclusively CS-08a under O14-18. |

## Application

Each zone's first resumed evidence/contract wave reconciles local `BlockedDecision`
rows against these decisions. Remaining unknown IDs, coordinates, build lineage,
client behavior and UAT results are evidence or dependency work for that zone,
not new owner decisions. The coordinator may ask the owner only for a genuinely
new scope or policy choice outside these bounds. The historical worknotes and
dated evidence snapshots remain unchanged; later implementation records cite
the applicable decision ID and its actual test/UAT outcome.
