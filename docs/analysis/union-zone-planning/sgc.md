# Union SGC restoration campaign

> **Type:** Candidate campaign and packet families
> **Scope:** `SGC_W1` world 58, SGU Human start and the source-backed Free Jaffa return legs
> **Status:** Planning only; dispatch waits for the first four-zone continuations and FZ-99
> **Companions:** [evidence](evidence.md), [execution contract](execution-contract.md), [SGC work packets](sgc-work-packets.md), [Dakara_E1 rebuild](../dakara-e1-rebuild/README.md)

## Playable outcome

A new SGU Human completes the SGC attack, takes mission 1570 to Dakara_E1, returns when missions 1651 and 1654 require Hammond/Carter, and leaves on 1655 for Omega. A Free Jaffa whose Dakara story reaches 1651 can perform only the intended SGC support, aftermath and Omega-transfer legs. Human and Free Jaffa receive their own 1655 dialogue and letters. The Asgard post-Pertho start does not use SGC_W1.

The [SGC starter master](../sgw-handoff-pack-v1.2/pack/references/world_content/SGW_Earth_SGC_SGU_Human_Starter_Dev_Master_v1.md) covers the local Human attack; the [full Human prologue v2](../sgw-handoff-pack-v1.2/pack/references/world_content/SGW_SGU_Human_Full_Starter_Prologue_to_Omega_v2.md) restores the Dakara and return links. The [Dakara Free Jaffa master](../sgw-handoff-pack-v1.2/pack/references/world_content/SGW_Dakara_Free_Jaffa_Starter_Dev_Master_v1.md) provides the Jaffa's SGC support and exit context. The recovered SGC_W1 production map and gate/Kismet exist, but original grant edges, some actor placements and reward rules are incomplete. World 86 `SGC` and world 87 `SGC_W2` are separate research candidates and are not silent replacements for world 58.

## Ownership and sequence

| Segment | SGC owner | Receiving/producing contract |
|---|---|---|
| Human start, 1559–1569 plus local 1567/1571 | Finish the attack, elevator/firearm, Carter, self-destruct, combat and Gate Room chain. Retain and regression-test merged 1559–1562 work. | Only Human starter eligibility; Asgard and Free Jaffa get no attack grant on entry. |
| Mission 1570 outward | SGC owns Harriman, dial, travel and once-only persisted departure. | Dakara_E1 owns the SG-18 search and all Dakara-local steps. |
| Mission 1651 return | SGC owns Hammond/Carter support, Naquadah generator grant and return travel. | Dakara_E1 owns the assault before and after the SGC leg, preserving mission state across both crossings. |
| Missions 1654–1655 | SGC owns aftermath, race-specific Hammond dialogue, the distinct Jackson/Bra'tac letter and Omega departure. | Omega owns arrival, Command Center transport, delivery and faction-specific SGU introduction. |

The existing [Dakara_E1 rebuild](../dakara-e1-rebuild/README.md) explicitly excludes SGC-side 1570/1651/1654/1655. Its ledger is a dependency, not an invitation to duplicate world 61/62 chains here. A future SGC coordinator first reads the current Dakara ledger and agrees on exact step and item ownership before sealing any SGC return packet.

## Candidate packet families

| Packet | Owner and scope | Ready when / proof |
|---|---|---|
| US-00 source, status and route seal | Mission specialist inventories current 1559–1571 and 1651/1654/1655 chains, client mission/step/objective/dialog IDs, actor tags, rewards, source build and SGC map placements. Reconcile Dakara's ledger and Class Start v6. Freeze exact handoff states and ID ranges in a new SGC continuation ledger. | First packet. `activate / reconstruct / hold` per beat; no implementation from a moniker alone. |
| US-01 existing Human start acceptance | Walk or specify bounded UAT for merged 1559–1562; repair only verified defects in first grant, elevator, firearm, Carter handoff, relog and wrong-archetype gating. | US-00; chain replay and client-visible first press. Do not reopen working starter chains for cosmetic parity without evidence. |
| US-02 Human crisis continuation | Restore 1563–1569 and embedded 1567/1571 in small mission-sized children: virus/failsafe, Jaffa tactics, Gate Room/ordnance. Place props/enemies from the map; do not substitute Pertho's same-named Self-Destruct dialog set. | US-00/01; each child proves journal step order, combat credit, item authority, reward once and relog. |
| US-03 Dakara crossings | Implement only SGC-local 1570 and 1651. Validate gate arrival/departure, mission persistence, Naquadah generator acquisition and the correct Human/Free Jaffa dialogue variant. | Dakara producer/receiver fixtures signed; travel test plus chain replay across both worlds. |
| US-04 aftermath and Omega departure | Complete 1654/1655 SGC legs for Human and Free Jaffa, including Jackson versus Bra'tac letter, Harriman dial and once-only transfer. | US-03, Omega arrival fixture; wrong-faction and duplicate-item refusal tests. |
| US-05 services and authored activity | After story UAT, survey return-safe SGC services and limited training/briefing side activity. Any new quest needs its own ID, client catalog presentation, lore review and owner-approved copy. Keep the crisis instance's pacing intact. | Core story accepted; proposed additions stay `BlockedDecision` until exact concepts and reward budget are sealed. |
| US-06 integrated acceptance | Run fresh Human and Free Jaffa characters through their SGC-local steps, two-player phase checks, death/respawn, relog and the real Dakara/Omega handoffs. Documentation Writer closes the SGC ledger and tester guide. | All packets; live-client steps require owner lab approval. Unobserved client outcomes remain `UATPending`. |

The continuation ledger splits US-02/03/04 into prescriptive packets with file/function allowlists, source IDs, placement class, tests and build-lane commands after US-00. One worktree and database per implementation packet; one reviewed campaign PR after integration. No chain ID, spawn position or item reward is claimed in this planning document.

## Stop conditions for the coordinator

- Do not claim the first SGC attack is complete because 1559–1562 run; 1563–1569 still need their own local chains.
- Do not mark Free Jaffa as completing the Human tutorial. Their SGC presence is a later mission visit, with independent mission state.
- Do not teleport across a missing gate transition to make the journal appear complete. A receiving world must accept the persisted mission state.
- Do not expose the Asgard holding-state SGC_W1 start as canonical once the post-Pertho extension is enabled.
