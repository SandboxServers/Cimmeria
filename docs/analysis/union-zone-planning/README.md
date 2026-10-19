# Union zone restoration campaign plans

> **Type:** Planning reference
> **Audience:** Future independent main sessions, campaign reviewers and the owner
> **Last updated:** 2026-10-14
> **Companions:** [evidence](evidence.md), [execution contract](execution-contract.md), [Free Jaffa route](free-jaffa-route.md), [Asgard route](asgard-route.md), [first four-zone plan](../four-zone-planning/README.md)

**Status: planning only.** This PR prepares four future, independently coordinated campaigns. It starts no zone implementation, takes no lab lease and changes no player-facing content. **Do not dispatch any of these campaigns until the Cellblock, Castle, Harset and Praxis Tollana continuations, including their shared FZ-99 close-out, are complete.** At that point each new main session rechecks `origin/main`, the prior ledgers and active branches before calling a packet Ready.

The four zone plans are [SGC](sgc.md), [Dakara_E1](dakara.md), [Omega Site](omega.md), and [Beta Site E1](beta.md). The [Free Jaffa](free-jaffa-route.md) and [Asgard](asgard-route.md) route briefs connect their distinct starts through these zone owners. The first shared Union hub is **Omega**; the first common field map is **Beta E1**. Tollana belongs to the Praxis progression and is outside this Union PR. See the [source reconciliation](evidence.md).

Each zone also has a future dispatch contract: [SGC packets](sgc-work-packets.md), [Dakara packets](dakara-work-packets.md), [Omega packets](omega-work-packets.md) and [Beta E1 packets](beta-work-packets.md). Omega also has the owner-supplied [developer concept art](omega-concept-art.md) of its hub layout. Their labels name bounded packet families and evidence gates; they reserve no database IDs and authorize no implementation before the start gate.

The owner chose **Human first for implementation**, and now requires plans for **all three** Union starts through Beta E1. Free Jaffa and Asgard have different missions and different entry states; their implementation packets wait until the Human baseline works. The owner chose a [post-Pertho Asgard SGU prologue](asgard-post-pertho.md) in existing map geometry. The [Human prologue v2](../sgw-handoff-pack-v1.2/pack/references/world_content/SGW_SGU_Human_Full_Starter_Prologue_to_Omega_v2.md) governs the Human route. Do not merge the three `The SGU` missions merely because the title matches.

## Campaign ownership

| Main session | Worlds and first outcome | Handoff and prerequisite |
|---|---|---|
| Union SGC | World 58 `SGC_W1` attack for Human; SGC-side 1570/1651/1654–1655 for Human and Free Jaffa. Preserve existing 1559–1562 work. | Dakara_E1 owns its local story; SGC sends both routes to Omega through distinct 1655 variants. Asgard does not use this starter. |
| Union Dakara_E1 | Worlds 61/62, Free Jaffa level-1 start and Human SG-18 visit, shared Dakara war/betrayal/climax. Continue the existing DK ledger rather than rebuild merged packets. | Consume Human 1570 from SGC, send/receive 1651 support and send 1654 aftermath to SGC. Free Jaffa's level-1 bridge is authored and separately approved. |
| Union Omega | World 18 hub and world 80 Command Center; distinct Human 1423, Free Jaffa 1500 and Asgard induction. The selected Asgard post-Pertho prologue uses existing geometry and a remote Council scene. | Consume Human/Jaffa 1655 from SGC and a private Asgard entry; send three distinct first-visit states to Beta. World 81/82 gaps remain outside first convergence. |
| Union Beta E1 | World 23 first arrival: Human closure/research, Free Jaffa Nine Peaks/antidote, Asgard Mimir/science. | Consume 1423/1500/Asgard 481 handoffs. Test simultaneous shared-world play and safe returns. Closed-Beta and E2 are later research holds. |

These are **four independent zone-owner main sessions**, not four consecutive Human-only stops. Two optional race-route integration sessions use the [Free Jaffa](free-jaffa-route.md) and [Asgard](asgard-route.md) briefs to prove the full journey after zone packets merge; they do not become competing owners of zone files. A zone can be planned independently while a packet remains `BlockedDependency` until an upstream handoff is merged and tested. No session copies another campaign's chain or spawn to make its own plan self-contained.

| Future campaign | Status | Next transition |
|---|---|---|
| Union SGC | `BlockedDependency` | First four-zone close-out, then US-00 source and map seal. |
| Union Dakara_E1 | `BlockedDependency` | First four-zone close-out and current DK ledger reconciliation, then UD-00. |
| Union Omega | `BlockedDependency` | First four-zone close-out, then UO-00; runtime packets await upstream receiving contracts. |
| Union Beta E1 | `BlockedDependency` | First four-zone close-out, then UB-00; runtime packets await Omega producers. |

## Planning decisions and open gates

| ID | Decision or gate | State |
|---|---|---|
| U-D01 | Restore recoverable 2009 mission, dialog, travel and world relations before adding lore-safe content. Classify each beat using the first four-zone [evidence contract](../four-zone-planning/README.md#evidence-contract). | Standing owner direction from the first four-zone plan. |
| U-D02 | Plan all three starts through first Beta E1 play; implement the Human path first, then Free Jaffa and Asgard extensions with their own entry, faction and negative tests. | Owner decisions, 2026-10-14. |
| U-D03 | The four new campaigns wait for the first four zone continuations and FZ-99 close-out. | Owner direction, 2026-10-14. |
| U-D04 | SGC owns its local 1570/1651/1654–1655 legs; Dakara owns worlds 61/62; Omega owns receiving 1655 state. Seal the exact step IDs against the existing DK ledger. | Planning boundary; exact step contract remains a future evidence packet. |
| U-D05 | Beta E1 first visit is the convergence boundary for these four campaigns. Closed-Beta Visit 2 and late E2 are future research, not hidden completion gates. | Owner decision, 2026-10-14. |
| U-D06 | Exclude speculative Union Tollana from this PR; Tollana remains in the earlier Praxis four-zone plan. | Owner decision, 2026-10-14. |
| U-D07 | Scale and reward policy for new Union side activity after restoration. | Future story/reward seal; first-four-zone direction is the starting reference. |
| U-D08 | Start the later Asgard extension after Pertho's evacuation, using new content in existing client map geometry and a remote Council hearing. Preserve original Pertho and physical-Council quests for possible future restoration. | Owner decision, 2026-10-14; [route brief](asgard-post-pertho.md). |
| U-D09 | Author a short Dakara introduction for the level-1 Free Jaffa start, earning entry to the recovered level-3–5 story without claiming unplayed SGC missions. | Owner decision, 2026-10-14; exact mission graph and reward curve are future seals. |
| U-D10 | The authored Free Jaffa bridge scouts divided Jaffa loyalties and foreshadows the later withdrawal-orders crisis without prematurely revealing Moh'katan's betrayal. | Owner story choice, 2026-10-14. |
| U-D11 | Center the new Asgard recovery prologue on rescued consciousness databanks. The remote Council hearing addresses their custody and scarce-body politics; the new character did not cause the original Pertho data leak. | Owner story choice, 2026-10-14. |

The source seals, map surveys, ID claims, client text, faction predicates and test fixtures are deliberately future packet outputs. This planning PR reserves no IDs and claims no coordinate that has not been checked against the current client and navmesh. The first four-zone plan's [ID allocation table](../four-zone-planning/id-allocation.md) remains authoritative for its campaigns. Each Union ID-lock packet must choose non-overlapping ranges against that table and the then-current seed; none may consume the first four's reserved blocks.

## What completion means

Each zone campaign gets its own continuation ledger, feature branch, packet worknotes and one complete campaign PR, following the [execution contract](execution-contract.md). Its story pass must be playable from its inbound handoff through its outbound handoff, with first-press feedback, relog and wrong-faction guards. The two route briefs define the end-to-end acceptance across owners. A mission definition in SQL, an archive name or a passing compile does not count as a playable quest. Client UAT still needs the owner's explicit lab approval. When it cannot run, the ledger records `UATPending` and the PR states what was not observed.

The [Lomiada review handoff](handoffs/lomiada-review.md) gives a compact briefing, source boundaries and concrete review questions for this planning PR.
