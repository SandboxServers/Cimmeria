# Cellblock continuation decision log

> **Type:** Reference. **Audience:** Cellblock coordinators and packet reviewers.
> **Last updated:** 2026-10-17. **Companions:** [Ledger](README.md), [CB-N05 proposed seal](worknotes/cb-n05-seal.md), [owner decisions](../four-zone-planning/owner-decisions-2026-10-14.md).
> Recorded 2026-10-14 against `d3efb7d41`. Source for inherited owner decisions:
> [four-zone index](../four-zone-planning/README.md) and
> [Cellblock packet contract](../four-zone-planning/cellblock-work-packets.md).

| ID | Status | Decision or unresolved question | Owner / blocked packet |
|---|---|---|---|
| CB-D01 | Settled, owner 2026-10-13 | One-time paced escape; restore recoverable content first; no repeatables, service hub or added forced kill. | All packets |
| CB-D02 | Settled policy, O14-14 | First-run core escape target is 30–45 minutes; optional content adds at most 10 minutes. The historical 37m06s/five-death privileged run is an observation, not a normal-player measurement. | Coordinator measures live pacing; N04–N07 |
| CB-D03 | BlockedEvidence | O14-12 delegates conservative ambient cast density and safe placement to the coordinator and domain reviewers. Exact count, tone, copy, template, placement and measured safety remain UNSEALED. | NPC/mission/presentation advisors; N04 |
| CB-D04 | BlockedDependency | O14-10 delegates a lore-safe first story and exact copy/IDs after review. Proposed [CB-N05](worknotes/cb-n05-seal.md) is itemless, zero reward, world-12 *The Remaining Pods*; design review may finish now, but dispatch waits for CB-U09's fresh normal core 30–45 minute baseline and room inside O14-14's 10-minute optional budget. Its live catalog proof precedes further authored quests. The old evidence-item/Castle recipient story moves to separate held CB-N05b. | Coordinator/domain reviewers and measured CB-U09; N05, then N05b |
| CB-D05 | Conditional, O14-15 | A second optional aid choice is allowed only if live pacing and safety evidence show it improves the escape. Exact interaction, copy, reward and placement remain UNSEALED. | Coordinator + domain reviewers after N05 pacing; N06 |
| CB-D06 | Conditional, O14-15 | One-room reactive encounter is allowed only if live pacing and safety evidence show it improves the escape. Difficulty, time/death effect and placement remain UNSEALED. | Combat/NPC advisors after N03 and pacing; N07 |
| CB-D07 | Settled, owner 2026-10-13 | Asset audit owns pod/door/barrier/Straegis presentation; ring campaign owns chain 1109 and ring animation; class-start owns gear. Preserve current cross_world_teleport fallback. | N01 coordinates; no duplicate patches |
| CB-D08 | Settled policy | 642 stays archival; 1654 stays off-route. Historical map aliases 1201–1207 are comparison worlds, never content imports into 12. | Archaeology; N00/N01 |
| CB-D09 | Settled policy, O14-11 | New authored quests have no player-facing provenance label. Seed and ledger identify each new beat as `CIMMERIA_AUTHORING`. | First authored quest |
| CB-D10 | UATPending | N05a catalog insertion merged to `main` in #1734 and is integrated here. The first real authored mission must prove journal, dialog, advance and relog in the live client before further authored-quest implementation. | N05 first proof; N06 held |
| CB-D11 | Review | O14-04 delegates individual ID allocation. [Shared table](../four-zone-planning/id-allocation.md#cb-n05-individual-claims--2026-10-14) claims N05 mission 62050, nested 62051–62056, dialog 60150, screen 96110 and six chains 11000–11005 after dated stock, seed and four-zone scans. They remain held for seal review and require a fresh collision recheck immediately before coding. Map 2795 is reused. | Coordinator; N05 review and every new row |
| CB-D12 | UATPending | O14-08 grants standing lab lease/client/server approval for the four zone sessions, subject to one-lease coordination and evidence recording. Client presentation, route safety, duration and fault reproduction remain untested. O14-09 permits exact `UATPending` rows only after documented lab unavailability; observed gameplay failures block merge. | N01/N02/N03/N08 |
| CB-D13 | BlockedDependency | Historical source-to-build joins are UNSEALED where manifests stop or patch inputs are absent; exact-byte archive integrity does not resolve mission lineage. | Archaeology; any historical import |
| CB-D14 | Review | First category-5 authored dialog 60150 uses the served one-screen, zero-button, speaker-0 UI-type-2 shape; new NPC-speaker/multi-screen overrides remain quarantined after a client crash. Native render and cache/relog proof are required. No dialog close/button event accepts N05. | Resource/client reviewers; N05 |

No advisor approval is implied by this table. Mission/NPC/presentation reviewers
must sign the affected implementation seals. No policy decision is inferred from
a stale sibling ledger or an archive author's imperative text.

## Orchestration decisions since 2026-10-14

The full text, status and reason of each D-CC decision is in the [README table](README.md#orchestration-decisions-d-cc); this list only dates them and says what they settle. They never renumber the CB-D rows above.

| Date | ID | What it settles |
|---|---|---|
| 2026-10-16 | D-CC1 | The adopting orchestrator runs the campaign (owner ruling). |
| 2026-10-16 | D-CC2 | The branch ledger is canonical; the main-checkout overlay stays uncommitted. |
| 2026-10-16 | D-CC3 | Standing lab approval extended to the orchestrator; superseded 2026-10-17 by the owner-runs-every-row directive. |
| 2026-10-16 | D-CC4 | Seed-only `player_loaded` restores are tested at resolve level. |
| 2026-10-16 | D-CC5 | Pilot re-run with measurement (CB-N02s, CB-N02a). |
| 2026-10-16 | D-CC6 | Review split from fixing: read-only reviewer, fixes by the coder, at most 3 rounds. |
| 2026-10-16 | D-CC7 | Drone relog restore sets aggression only (CB-N02d). |
| 2026-10-16 | D-CC8 | Only the integration packet edits the chain-range header and `id-allocation.md`. |
| 2026-10-17 | D-CC9 | CB-N04 uses a gated, fire-once `npc_bark`, not `cell-chatter`. |
| 2026-10-17 | D-CC10 | CB-N04a is built before CB-U08; CB-U08 gates the PR (CB-D03, CB-D12). |
| 2026-10-17 | D-CC11 | Death, difficulty and time impact of a one-room response: **moot** after D-CC19 (CB-D06). |
| 2026-10-17 | D-CC12 | Cast rows are stationary and outside `Castle_Cellblock.Region1`. |
| 2026-10-17 | D-CC13 | CB-N05 is built before CB-U09/U10, which become owner-run rows (CB-D04, CB-D10). |
| 2026-10-17 | D-CC14 | CB-N05's allowlist grows and the packet splits into CB-N05-1 to -7. |
| 2026-10-17 | D-CC15 | CB-N06's existence: decided by D-CC18 (CB-D05). |
| 2026-10-17 | D-CC16 | CB-N07 is split by impact; Tier 0 bark built, Tier 1 held (CB-D06). |
| 2026-10-17 | D-CC17 | The owner checklist stands in for `uat-guide.md`; CB-N08 split into four parts. |
| 2026-10-17 | D-CC18 | CB-N06 included as `CIMMERIA_AUTHORING` content (owner; CB-D05, CB-D09). |
| 2026-10-17 | D-CC19 | CB-N07b omitted; the Tier 0 bark stays (owner; CB-D06). |

D-CC2, D-CC4, D-CC5, D-CC7 and D-CC8 carry the date they were ported or recorded in the ledger (2026-10-16), not necessarily the day they were first decided.

No new CB-D row was opened. The CB-D statuses above still read as of 2026-10-14; the current state of each affected packet is in the [README packet table](README.md#packet-status) and the [CB-N08 acceptance reconciliation](worknotes/cb-n08-acceptance.md).
