# Asgard post-Pertho SGU prologue

> **Type:** Planning decision and future campaign brief
> **Status:** Owner selected this direction on 2026-10-14. No implementation is authorized by this note.
> **Prerequisites:** Finish the first four-zone continuations and FZ-99; implement the Union Human route first.

## Decision

The Asgard extension begins **after the evacuation of Pertho**. It uses new Cimmeria missions in existing client map geometry to bring an Asgard character to the public Omega Site hub and then the recovered first Beta Site visit. Pertho's original mission chain and the in-person High Council map remain historical source material for a later restoration. The new prologue must never be described as the 2009 Pertho starter or as a recovered Council visit.

This follows the owner's choice of the post-Pertho SGU prologue over staging adapted Pertho rescue missions in unrelated geometry. It does not change the Human-first implementation order.

## Source boundary

The [Pertho starter master](../sgw-handoff-pack-v1.2/pack/references/world_content/SGW_Pertho_Asgard_Starter_Dev_Master_v1.md) preserves missions 1617–1641 and optional 1665–1667, but its audit found no named production UMAP/MapData for worlds 83–85. The [High Council master](../sgw-handoff-pack-v1.2/pack/references/world_content/SGW_Asgard_High_Council_Dev_Master_v1.md) likewise found no production map for world 59. Its early hearings and returns have source-backed story evidence, while several later return triggers exist only in the progression aid.

The original Council dispute concerns preserved Pertho records, public knowledge of the shortage of suitable Asgard bodies, and a threatened loss of body rights (High Council master, Dialog 5805). The new character did not perform the original Pertho rescue, so the story seal must give the remote hearing its **own** cause and consequence while respecting those political stakes. Do not display Dialog 5805 as if the new player were its original addressee.

The [Omega master](../sgw-handoff-pack-v1.2/pack/references/world_content/SGW_Omega_Site_Dev_Master_v1.md) places all three Union paths at Omega before early Beta, with the Asgard route tied to 1642/1522. The [Beta master](../sgw-handoff-pack-v1.2/pack/references/world_content/SGW_Beta_Site_Complete_Dev_Master_v1.md) places the early Asgard field arc at Beta E1 world 23, starting with mission 481 and Mimir's work. The existing Omega main and Command Center maps and Beta E1 map survive. `entities/spaces.xml` already declares Omega world 18 and instanced Command Center world 80. [Debug Area](../debug-area/README.md) proves that a separate Cimmeria world can load an existing client map, but an Asgard alias and its private-instance behavior need their own client proof.

## Proposed playable route

| Stage | Space and level target | New story and player action | Source relationship |
|---|---|---|---|
| 1. Reintegration | Private use of existing Omega Command Center geometry; level 1 | A newly reintegrated Asgard receives a briefing from Rekyr or another source-backed contact. Pertho has already been evacuated. Introduce the combat drone, SGU controls and the problem of safeguarding evacuated consciousnesses. | New prologue. The player is **not** credited with completing Pertho's historical rescue missions. Reuse a recovered line only after checking its location and speaker context. |
| 2. Recovery and containment | Candidate private world loading existing Omega main geometry; levels 1–5 | Protect rescued consciousness databanks transferred after Pertho's evacuation, verify their integrity and chain of custody, contain a threat to them, and make a bounded rescue decision. Author encounters and training around the actual surveyed Omega terrain. | New missions inspired by Pertho's evidenced themes; no original Pertho step, coordinate, Kismet event or mission ID is silently rebound to Omega. |
| 3. Council ruling | Existing Omega Command Center geometry; roughly level 5 | Conduct a **remote** hearing through a hologram or communicator. The Council rules on custody of the rescued minds, scarce-body rights and the character's SGU assignment; Thor supports the assignment. | New presentation and mission logic. It is not a physical visit to world 59 and must not execute the original Council's physical travel objectives or attribute the earlier leak to this character. |
| 4. SGU induction | Public Omega world 18 and Command Center world 80; level 5–6 | Hand off to SGU leadership, unlock the normal Asgard hub services and receive the first Beta assignment. Preserve Human and Free Jaffa eligibility boundaries. | Reconcile the recovered Asgard 1522 story with the new hearing. Any incompatible objective gets a new authored mission/step rather than being shown with false location text. |
| 5. Beta entry | Beta_Site_Evo_1 world 23; level 6 | Enter the original early Asgard field arc: mission 481, Carter, Mimir, Nine Peaks investigation and faction-specific Beta work. | Restore source-backed Beta-local content. The prologue ends at the accepted Beta handoff; the Beta campaign owns its local runtime. |

The staging area is a **proposal**, not an invented placement seal. The first implementation packet must survey Omega's playable surfaces, transporters, cover, population and coordinates. If a private alias of the Omega map cannot keep the tutorial's NPCs and state isolated, choose another already shipped map only after documenting its visual fit and client proof. The public hub must not expose low-level tutorial enemies or Pertho aftermath to unrelated players.

## Implementation and evidence packets

1. **Story and ID seal:** Write the new mission graph and exact dialogue, mark every line `CIMMERIA_AUTHORING` or source-backed quotation, choose reward/level targets, and reserve IDs against current `main` and the first four-zone allocation. Record how 1642/1522 and 481 are activated or held; do not alter stock quest text merely to make the new map fit.
2. **Space proof:** Survey Omega main and Command Center in the client after explicit lab approval. Prove private alias entry, collision/navmesh, respawn, beam travel, client world/map presentation and return to public Omega. The live-tested Debug Area alias is a precedent, not proof for this alias.
3. **Prologue runtime:** Add Asgard-only creation/start placement, authored missions, NPCs, interactables, combat/tutorial targets, items, rewards, persistence and telemetry. An Asgard must not start in the current SGC_W1 placeholder or receive the Human starter.
4. **Remote Council and SGU handoff:** Author the hearing and Omega leadership exchange, test the exact branch and relog state, then grant the source-backed Beta entry at the right level. The Beta campaign owns mission 481's local steps.
5. **Validation:** Chain replay and live-DB guards cover new grants, steps, reward once, abandon/retry, relog, travel and wrong-faction refusals. Two-player checks prove phase isolation. After lab approval, create an Asgard, complete the prologue, reach public Omega and accept the Beta quest in the real client. Record any unavailable client or map behavior as pending.

## Open design and source gates

- The owner chose rescued consciousness databanks as the recovery focus. Seal their exact custody threat, containment encounter, Council ruling and moral choice during the story packet. Keep the player a post-evacuation participant; do not claim they performed the original Pertho rescue or data leak.
- Decide whether the remote hearing uses only surviving Council speakers or a new representative. Do not assign an unresolved historic speaker ID by guesswork.
- Confirm whether original mission 1522 can retain any local Omega steps without showing an impossible physical-Council objective. Otherwise author a distinct induction mission and hold the incompatible stock steps.
- Survey available Omega geometry before naming a quarantine area or promising a specific encounter location. Its current content is transport-focused, and placement, population and navmesh require restoration.
- If Pertho/High Council maps are later recovered, preserve this prologue as a clearly identified alternate entry until the owner decides how characters migrate. Do not silently replace completed characters' mission history.

**Completion boundary:** An Asgard can start, understand the Pertho aftermath, finish the new training and Council decision, join the SGU at Omega, and accept the first recovered Beta Site mission without a GM command, manual database edit or borrowed Human/Jaffa quest state.
