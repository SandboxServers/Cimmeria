# Union Omega Site restoration campaign

> **Type:** Candidate campaign and packet families
> **Scope:** Shared SGU hub, worlds 18 and 80; first arrival of Human, Free Jaffa and Asgard
> **Status:** Planning only; dispatch waits for the first four-zone continuations and FZ-99
> **Companions:** [evidence](evidence.md), [execution contract](execution-contract.md), [Omega work packets](omega-work-packets.md), [Asgard post-Pertho decision](asgard-post-pertho.md)

## Playable outcome

Human and Free Jaffa arrive through mission 1655, ride the correct Omega beam connection, deliver distinct letters to Daniel Jackson and Bra'tac, and receive their own `The SGU` missions 1423 and 1500. A new Asgard completes the owner-selected post-Pertho prologue in existing map geometry, receives a remote Council ruling, enters the public Omega hub and receives a Beta-bound Asgard mission. All three can reach Beta E1 without sharing the wrong faction's grant, dialog, reward or tutorial state.

The [Omega master](../sgw-handoff-pack-v1.2/pack/references/world_content/SGW_Omega_Site_Dev_Master_v1.md) describes worlds 18/80, the first-arrival variants, and missions 1423/1500/1522. Its direct mission-to-dialog-set join remains partly inferred. Main and Command Center production maps survive; Ruins 81 and StorageRm 82 have world rows but no named production map in the audited handoff, so this first-convergence campaign does not assume they load. Current Omega runtime is mostly transport; a mission row or speaker name is not an implemented hub service.

## Ownership and source boundary

| Arrival | Producer | Omega-local receiver and Beta handoff |
|---|---|---|
| Human | SGC world 58 owns 1655 departure and the Jackson letter. | Omega owns world 18 arrival, Command Center route, objective 6056 delivery, 1423 eligibility and Beta departure. |
| Free Jaffa | Dakara worlds 61/62 and SGC world 58 own the completed war, aftermath and 1655 departure with the Bra'tac variant. | Omega owns its letter delivery, 1500 eligibility and Beta departure. The Human 1423 offer must refuse. |
| Asgard | New, separately labeled [post-Pertho prologue](asgard-post-pertho.md); original Pertho/Council physical-world quests are held. | Omega owns private training instance, remote hearing, public-hub entry, reconciled 1522 story and source-backed Beta 481 handoff. The new route must not auto-complete stock Pertho missions. |

Omega is the **first hub convergence**, while Beta E1 is the **first common field map**. This campaign owns local state and departure only; Beta owns every Beta-local step. Later Omega returns may be scoped in a follow-on wave after first convergence is playable, but must be attached to their actual offworld producer and not made up as generic hub visits.

## Candidate packet families

| Packet | Owner and scope | Ready when / proof |
|---|---|---|
| UO-00 source and placement seal | Inventory world 18/80 maps, beams, current seed, mission/dialog joins for 1655, 1423, 1500 and 1522, and each faction's exact inbound state. Reconcile `Omega_Site_CmdCenter` with the Capitol Ship wording. Freeze actor tags, entry pins, ID claims and provenance in a new Omega continuation ledger. | Root packet; source mismatch stays open rather than silently relabeling the map. |
| UO-01 hub travel and shared safety | Complete arrival, beam platform, Command Center access, respawn and return routing; make faction/public phase boundaries server-authoritative. | UO-00; movement/transport test, relog and two-player witness check. |
| UO-02 Human induction | Restore Jackson letter completion, leadership dialogue and 1423 first offer without double credit or an early Beta grant. | Signed SGC 1655 handoff; Human-only chain replay and client journal/dialog check. |
| UO-03 Free Jaffa induction | Restore Bra'tac letter completion and 1500, preserving Free Jaffa political/medical setup for Beta. | Signed SGC/Dakara 1655 handoff; Free-Jaffa positive, Human/Asgard negative tests. |
| UO-04 Asgard space and starter proof | Survey a private instance using existing Omega geometry; prove load, actor isolation, navmesh/collision, respawn and transition to public Omega. No new map package is planned. | UO-00 and owner post-Pertho decision; small live-client probe after explicit lab approval. If the alias fails, reseal placement before content packets. |
| UO-05 Asgard authored prologue | Implement the [route brief](asgard-post-pertho.md): reintegration, rescued consciousness databanks, custody and containment, combat-drone tutorial and consequential rescue choice. New IDs/text, explicit authored provenance and level pacing; retain original Pertho mission rows untouched. | UO-04; split by story act; chain replay, live-DB joins, reward once, relog, faction and two-player isolation. |
| UO-06 remote Council and SGU entry | Stage a remote hearing in Command Center geometry, reconcile 1642/1522's Omega intent, and give the Asgard the correct Beta entry without impossible physical-Council objectives. | UO-05 and exact speaker/dialog seal; client presentation and wrong-faction tests. |
| UO-07 hub life and services | Add source-compatible leadership, travel, trainer/vendor and bounded repeatable or side-story activity after the first route works. New content follows the shared authored-catalog proof and an exact story/reward decision. | UO-02/03/06 accepted; economy, service and spawn placement tests. |
| UO-08 integrated acceptance | Run three fresh race-specific routes from inbound state to accepted Beta handoff, with relog at letter/remote-hearing boundaries and two players in distinct phases. Close ledger, mission docs and tester guide. | Beta receiver fixtures ready; client run requires owner lab approval. |

UO-05 is a separate design-heavy specialist packet family inside the Omega campaign; it may become its own implementing workstream after the Human path, but it cannot create a competing Omega world owner. Its future packet seal must pick exact coordinates from the client map, exact new IDs and rewards against current branches, and text that does not pretend Omega is Pertho or a physical Council chamber.

## Stop conditions for the coordinator

- Never treat three identically titled `The SGU` missions as one shared mission. Their IDs and faction recipients differ.
- Never attach Council-world 59 travel objectives or original Pertho Kismet to Omega geometry merely because the story beat is useful.
- Never open the public hub with private tutorial enemies or rescue states visible to unrelated players.
- Never mark the path complete on an Omega-only UAT: Beta must accept each faction's outbound mission state.
