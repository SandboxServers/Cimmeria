# Four-zone content planning

Status: **campaign packet planning only**, 2026-10-13. No zone implementation campaign was started and no lab lease was taken.

This index coordinates proposed continuations for [Cellblock](cellblock.md), [Castle](castle.md), [Harset](harset.md), and [Tollana](tollana.md). Their packet contracts are [Cellblock](cellblock-work-packets.md), [Castle](castle-work-packets.md), [Harset](harset-work-packets.md), and [Tollana](tollana-work-packets.md). The [execution contract](execution-contract.md) governs branch topology, budget, wave CI, independent fixer/fresh-reviewer rounds and the one-PR-per-campaign closeout. Each zone plan points to an existing campaign ledger before proposing work. The implementation lead must reconcile active branches and packet statuses immediately before dispatch; these plans are not an assertion that old ledgers are current.

The [story and oral-history source guide](story-research.md) recovers the earlier Dial the Gate video research from local Claude history and sets the provenance rules for authored quests. The [independent verification](verification.md) checks packet premises against current seed/code; the [additional briefs inventory](downloads-evidence.md) records other Lomiada material found in Downloads.

The current map has **63 packet families**: eleven Cellblock, eleven Castle, 23 Harset and 18 Tollana. Mandatory child splits and conditional recovered side-story subsets add dispatch units; a family is not one guaranteed Haiku run. Research, architecture and UAT scopes go to specialists. Implementation workers target 50–60k working tokens with under 100k billed tokens per packet across coding, review and fixes. Evidence and owner-decision gates must seal exact IDs, coordinates, client presentation and allowlists before dispatch.

## Owner direction

| Decision | Direction |
|---|---|
| Restoration vs new story | Restore recoverable 2009 content first, then add explicitly labeled, lore-safe quests. |
| MMO fullness | Cover core story, quest hubs, services, ambient NPC density, and repeatable activity where pacing supports it. |
| Cellblock | Keep the escape paced; no new repeatable activities. |
| Tollana Curia | Stage the climax in a suitable area of world 19; world 88 is a flat stub. |
| Castle shape | Castle can be a returnable hub with services and repeatable activity; Cellblock is the one-time escape. Medical mission 1520 is for human/Op-CORE, symbiote mission 1521 for Jaffa. |
| Harset historical scope | Re-evaluate all 73 ZIP-linked mission records before any new Harset implementation; retain per-player Market and Storage encounters. |
| Harset older revisions | Hold older-revision candidates until build lineage proves a compatible arc. |
| Harset active set | Plan up to 33 active records: 29 ZIP-linked core plus four existing campaign records, with mission 567 conditional on its item grant. |
| Missing SQLite | The handoff author did not provide the named databases. Use the ZIP CSVs and repo evidence, and leave unverifiable joins open. |
| Tollana scope | Plan all four paths: OPC, Goa'uld, Loyalist Jaffa and DRK. |
| Authored content | After restoration UAT, target about 5–8 lore-safe side stories per large hub plus repeatables; start with useful supplies and modest XP/currency rewards. Cellblock remains one-time. |

## Evidence contract

Every mission and encounter packet uses one of three provenance classes **per story beat**, not just per quest. **`RESTORED_SOURCE`** means an exact 2009 mission, step, objective, dialog or asset relation supports that beat; the packet cites its file, ID and build when known. **`CIMMERIA_RECONSTRUCTION`** means the historical destination or goal is supported but a missing placement, transition or mechanic has been rebuilt to make it playable; the packet states which part is inferred and seeks a source-compatible design review. The staged world-19 Curia approach is in this class. **`CIMMERIA_AUTHORING`** means a new quest, character exchange or activity created for the living MMO; it receives a new ID, lore review and an explicit authored label. A reused historical NPC or prop does not turn a new story into restoration. No packet may call a reconstructed or authored beat a recovered 2009 fact, and a missing build lineage keeps the affected beat held. Each future ledger records the class, source citation, remaining uncertainty and approving advisor before coder dispatch.

The supplied `00_FINAL_SUMMARY.txt` reports 24 recovered world dossiers. The matching `SGW_FINAL_DEVELOPER_HANDOFF.zip` was found in Downloads and has the four-zone CSV/dossier set. Its master handoff describes `SGW_FINAL_RE.sqlite` and `SGW_ALL_FINDINGS_ARCHIVE.sqlite`, but neither database was provided. The owner confirmed this omission; their absence is an evidence limit, not a planning blocker. Treat the included CSV rows as source-backed at their stated build and relation, not as proof of currently playable server behavior. Source-name VO, UI and asset associations remain candidates. Older `docs/content/zone-audit.md` and `docs/content/external-data-analysis.md` contain historical snapshot counts; compare against the current seed and later campaign work before treating a gap as open.

## Packet sizing and dispatch gate

Each Haiku implementation packet targets **50–60k working tokens** with a measured **under-100k total packet bill** across implementation, review and fixes. The packet documents give source-backed behavior, files, tests, branches and stop rules. Specialist gate packets must freeze unresolved IDs, coordinates, build lineage, client catalog entries, exact dialog/copy and item grants into a signed manifest before the matching Haiku packet becomes Ready. Use one worker per worktree and test database. Merge packet commits into a campaign feature branch; run local CI-equivalent checks and fresh adversarial review at each wave, send accepted fixes to a separate fixer and the repaired diff to a new reviewer, for at most three rounds or until no issue remains. Block unresolved findings. Open one remote PR for the **complete campaign** only after its final local CI and review pass, then wait for remote CI and the merge queue. The four investigations and packet writing do not start implementation.

The likely shared sequence is: reconcile sources and old campaign states; survey and approve placement; restore original faction chains; populate service and ambient loops; add approved authored content; then run cross-zone client UAT and close out status docs. A zone's first packet must remove stale assumptions from its old ledger before later packets can be called Ready.

The [Harset 73-record review](harset-73-mission-review.csv) is complete at CSV/seed evidence level: 29 current-core records, 39 older-revision holds, two Tollana handoffs and three offworld holds. The older campaign's 36-row scope included four records outside this ZIP set and the three offworld rows. The owner approved the resulting up-to-33 active set, with mission 567 conditional on its item grant. Its ledger still needs a status and dependency reset before implementation. The [Tollana 51-record census](tollana-mission-census.csv) supplies the corresponding faction inventory for that zone.
