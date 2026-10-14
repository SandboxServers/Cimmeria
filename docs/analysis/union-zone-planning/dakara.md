# Union Dakara_E1 restoration and route completion

> **Type:** Candidate continuation campaign and packet families
> **Scope:** Dakara_E1 world 61 and Story Room world 62, Free Jaffa start and Human SG-18 visit
> **Status:** Planning only; new continuation dispatch waits for the first four-zone continuations and FZ-99
> **Companions:** [existing Dakara_E1 rebuild ledger](../dakara-e1-rebuild/README.md), [evidence](evidence.md), [execution contract](execution-contract.md), [Dakara work packets](dakara-work-packets.md)

## Playable outcome

A level-1 Free Jaffa begins on Dakara_E1, receives the owner's approved, explicitly authored bridge to the recovered level-3–5 Dakara story, survives the betrayal and Superweapon defense, and reaches SGC/Omega. A Human arriving from SGC on mission 1570 can join the supported shared Dakara crisis and return on the correct SGC legs. The two characters may be present together without receiving each other's first-grant, dialog, reward or transport state.

The [Free Jaffa starter master](../sgw-handoff-pack-v1.2/pack/references/world_content/SGW_Dakara_Free_Jaffa_Starter_Dev_Master_v1.md), [multi-visit Dakara master](../sgw-handoff-pack-v1.2/pack/references/world_content/SGW_Dakara_Multi-Visit_Dev_Master_v1.md) and [Human prologue v2](../sgw-handoff-pack-v1.2/pack/references/world_content/SGW_SGU_Human_Full_Starter_Prologue_to_Omega_v2.md) support the story. Worlds 61/62 have production maps, gate and navmesh. The [existing rebuild ledger](../dakara-e1-rebuild/README.md) already contains an evidence audit, placement estimates, cast/prop groundwork and a free-Jaffa escape from the dead-end start. It also records the key conflict: the project starts Free Jaffa at level 1 on Dakara, while the surviving mission 1570/1645–1655 material is level 3–5 and assumes earlier SGC events. This plan does not retroactively call the level-1 bridge original SGW content.

## Continue the existing campaign, do not duplicate it

The future main session first audits the current state of `docs/analysis/dakara-e1-rebuild/`, its merged PRs, live UAT and active worktrees. Adopt its finished maps, navmesh, placements, actor templates, tent travel and start infrastructure. The existing ledger owns world-61/62 mission work and remains the execution authority for any packet already active. If that ledger finishes the full route before this continuation begins, this campaign becomes a narrow acceptance/handoff pass, not a second rebuild. The pause on **new Union continuations** does not rewrite work already underway in the historical Dakara campaign.

| Segment | Dakara owner | SGC/Omega contract |
|---|---|---|
| Free Jaffa levels 1–2 | Author a race-specific scouting bridge about divided Jaffa loyalties in world 61 only after the exact story and client mission-catalog seal. Foreshadow the withdrawal-orders dispute without revealing Moh'katan's betrayal. No borrowed Human SGC attack completion. | SGC's later support visit receives a character whose Dakara story is actually active. |
| Human 1570 entry | Receive the SGC-produced 1570 gate departure and play SG-18 search/report on Dakara. | SGC owns Harriman and outbound/return control. |
| Shared 1645–1650 | Restore withdrawal orders, scouts, gate defense, Loth'ta dispute, Moh'katan betrayal and tent-interior scenes with distinct race dialogue. | Keep source mission IDs and state; do not auto-complete Jaffa-only context for the Human. |
| 1651 temporary support | Dakara owns the send/return around renewed attack. | SGC owns Hammond/Carter and Naquadah generators. |
| 1652–1653 climax and 1654 exit | Dakara owns local Fireball, Superweapon and gate departure. | SGC owns aftermath/Shutdown; Omega receives 1655 from SGC. |

Later Dakara E2/E3 and the base world 24 are outside this first-convergence scope. Their named map packages are absent from the audited handoff; the source has separate later faction arcs that must not be pulled into the starter instance.

## Candidate packet families

| Packet | Owner and scope | Ready when / proof |
|---|---|---|
| UD-00 status, source and contract reset | Read existing DK packets/worknotes, current seed and recent branches; mark each mission/placement `Done`, `UATPending` or remaining. Reconcile the Human 1570 entry and SGC-side step ownership. Freeze ID ranges and a mission-by-mission route matrix. | First continuation packet. No already merged DK packet is reimplemented without a reproduced defect. |
| UD-01 Free Jaffa level-1 bridge | Write the owner-approved short, lore-safe authored introduction that gets a new Jaffa to the recovered crisis at the intended level, teaching basic staff/combat/transport without pretending SGC missions occurred. | Exact story, reward and authored-catalog client seal; new IDs and wrong-faction tests. |
| UD-02 peaceful crisis chain | Complete 1570 Dakara steps and 1645/1646/1648–1650 from the current DK state, splitting by mission and already decided tent/placement contracts. | UD-00 and existing DK coordinates; chain replay, live-DB joins, client interaction/choice and relog proof. |
| UD-03 defense and Superweapon | Reconstruct the missing combat designs for 1647/1653 and support 1652's devices/air attack. Keep enemy level, spawns, reset and two-player safety bounded. | Existing DK gate GDK1/owner combat decision, AI/nav and event evidence; combat, item and death tests plus client UAT. |
| UD-04 SGC crossings and end state | Own Dakara-local 1651 send/receive and 1654 exit, with persisted quest state and once-only handoff to SGC. | Signed SGC receiver fixtures; movement, replay and cross-world relog tests. |
| UD-05 joint acceptance | Run Free Jaffa from creation and Human from SGC 1570 through the shared Dakara crisis, temporary SGC loop and departure; test simultaneous players, wrong-race grants and tent instance isolation. Close the continuation ledger/tester guide. | All local packets and SGC fixtures; lab lease/client requires owner approval. |

UD packets are **remaining-work labels**, not a request to restart the DK-01–DK-05 sequence. The future coordinator maps each proposed family onto exact existing DK packet status, then dispatches only unowned work. Every implementation child specifies files/functions, mission and actor IDs, test type, build-lane checks and a separate worktree. Source-backed mission text may coexist with reconstructed placements and encounters; provenance is recorded per beat.

## Stop conditions for the coordinator

- A fresh Free Jaffa cannot be sent into 1570 with completed SGC steps that the character never played without an explicitly documented variant or skip mechanism.
- Do not solve the level gap by altering original mission levels in seed without a reviewed progression/reward design.
- Do not duplicate the SGC-side 1651, 1654 or 1655 chains in Dakara files.
- Do not call Dakara E2/E3 part of this early shared map because all six factions reach Dakara again later.
