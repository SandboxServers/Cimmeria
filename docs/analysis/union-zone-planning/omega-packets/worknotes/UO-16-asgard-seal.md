# UO-16: Asgard design seal (worknote)

> **Type:** Reference (worknote) · **Packet:** [UO-16](../UO-16.md) · **Status:** Sealed design, no seed or code change. Open owner calls are in § Open questions.
> **Index:** [omega-work-packets.md](../../omega-work-packets.md) · **Route ledger:** [asgard-route.md](../../asgard-route.md)

Labels: `RESTORED_SOURCE` (shipped data, file and line), `CIMMERIA_RECONSTRUCTION` (new binding of shipped data), `CIMMERIA_AUTHORING` (new). A ticket or draft chapter is a claim, not evidence.

## § Source review (AS-E1)

Paths are under the worktree. Line numbers are as read on 2026-10-19.

| # | Fact | Source | Label |
|---|---|---|---|
| S1 | Mission 1522 `The SGU`: level 6, story false, enabled true, `can_fail` true. | `db/resources/Missions/Seed/missions.sql:541` | RESTORED_SOURCE |
| S2 | 1522 step 4480 (index 0) `Report to Otin and the High Council.`, disabled. | `mission_steps.sql:2066` | RESTORED_SOURCE |
| S3 | 1522 step 4773 (index 1) `Report to the Thor.`, disabled. | `mission_steps.sql:2068` | RESTORED_SOURCE |
| S4 | Step 4480's objective is 5212 (blank text, disabled); its task is 7783. This is the Otin/High Council objective. | `mission_objectives.sql:2142`; `mission_tasks.sql:2287` | RESTORED_SOURCE |
| S5 | Step 4773's objective is 5494 (disabled); its task is 7782. This is the return-to-Thor objective. | `mission_objectives.sql:2144`; `mission_tasks.sql:2289` | RESTORED_SOURCE |
| S6 | **ID collision.** Objective 1522 is attached to step 1313, which belongs to mission 358 (`Inform Tynan...`). Task 2100 hangs on that objective. Mission 1522 has no objective of its own with id 1522. | `mission_objectives.sql:4639`; `mission_steps.sql:4191`; `mission_tasks.sql:5113` | RESTORED_SOURCE |
| S7 | Mission 481 `From on High`: level 6, story true, enabled true, `can_fail` false. | `missions.sql:1351` | RESTORED_SOURCE |
| S8 | 481's steps are **1760 (index 0), 1761 (index 1), 1762 (index 2)** only. Step 1760 `Proceed to the Tau'ri Administration Hub on Beta Site`. | `mission_steps.sql:4845`, `:4847`, `:4849` | RESTORED_SOURCE |
| S9 | Correction to the ledger's "steps 1760/2078/2713". 2078 and 2713 are **objective and task ids**, not steps. Objective 2078 sits on step 1760 (task 2713). Step id 2078 belongs to mission 614 (`Investigate the network blockage...`). | `mission_objectives.sql:5315`; `mission_tasks.sql:5801`; `mission_steps.sql:5615` | RESTORED_SOURCE |
| S10 | 481's other objectives: 2079 on 1761 (task 2714), 2080 on 1762 (task 8598), 4015 on 1762 (task 4835). | `mission_objectives.sql:5317`, `:5319`, `:5321`; `mission_tasks.sql:5803`, `:5805`, `:5807` | RESTORED_SOURCE |
| S11 | Item 3399 `Communiqué from the Asgard High Council`; description `Authorizing you to work under Mimir at the SGC's Beta Site.` | `db/resources/Items/Seed/items.sql:11994` | RESTORED_SOURCE |
| S12 | Item 3399 grant sites: none in the Content seeds. The only chain that names 3399 is Beta's `remove_item` on the O2079 hand-in. Objective 3399 and task 4041 are unrelated id collisions. | `Content/Seed/beta_e1_m481_chains.sql:72`; `mission_objectives.sql:4183`; `mission_tasks.sql:4625` | RESTORED_SOURCE (absence checked by grep of `db/`) |
| S13 | Moniker 28128 `Thor (Hologram)` (`DN_npc_int_Thor_GeneticsLab`). No template or speaker row in `db/` uses 28128. | `db/resources/Texts/Seed/texts.sql:38978` | RESTORED_SOURCE |
| S14 | Moniker 28544 `High Councillor Otin` (`DN_npc_mg_Otin_Pertho`). No template, dialog speaker or chain in `db/` uses 28544. | `texts.sql:39870` | RESTORED_SOURCE |
| S15 | Speaker 777 is Thor. Thor speaks in dialog 5025. Recorded as F7. | `db/resources/Dialogs/Seed/dialog_screens.sql:21802 and :21808` (dialog 5025, speaker 777); F7 | RESTORED_SOURCE |
| S16 | Speaker 2509 is the moniker `Preserved Alteran Corpse`, not a character. Never speaker for Otin (rule 7). | `texts.sql:32743` | RESTORED_SOURCE |
| S17 | Otin as a speaker: no shipped `dialog_screens` row has speaker 28544. Otin appears only in text: screen 5779 (speaker 3406) names "High Councillor Otin", screen 5805 is spoken to "Councillor Otin", and a button says "Convince Otin to not disband the SGU." | `dialog_screens.sql:25700`, `:25883`; `dialog_screen_buttons.sql:7920` | RESTORED_SOURCE |
| S18 | No shipped dialog row accepts 481 (no `dialogs.sql` accept column matches 481). The 481 offer is new. | `db/resources/Dialogs/Seed/dialogs.sql` (grep) | RESTORED_SOURCE (absence) |
| S19 | Beta's receiver expects archetype `eq '5'`, `step_status 481 1760 eq active`, then `advance_step 481 → 1761`. | `Content/Seed/beta_e1_m481_chains.sql:16-30` | CIMMERIA_RECONSTRUCTION (Beta side) |
| S20 | Gate 6 grant pattern: Human 63822 completes 1655, accepts 1423 and runs `grant_stargate_address 6`. Jaffa 63832 does the same for 1500. Chain block 63800-63999. | `Content/Seed/union_omega_human_chains.sql:74-95`; `union_omega_jaffa_chains.sql:71-91` | CIMMERIA_RECONSTRUCTION |
| S21 | Chain ids 63950-63999 are unused in `Content/Seed/`. | grep of `Content/Seed/` | RESTORED_SOURCE (absence) |
| S22 | Char-def 9 (Asgard) row sits at `char_creation.sql:59`; it is `NON_CANONICAL_BLOCKED_LEGACY` per the class-start v6 README. F42. | `db/resources/Archetypes/Seed/char_creation.sql:59`; [asgard-route.md](../../asgard-route.md) line 13 | RESTORED_SOURCE |
| S23 | The art's Council Chamber is an overview label at (580, 138), with no world row. The three "Combate Instance" labels are out of scope (D-UO18). | [omega-concept-art.md](../../omega-concept-art.md) § Council Chamber | RESTORED_SOURCE (art layout intent only, D-UO15) |

## § Venue

- The hearing's venue is the art's Council Chamber (D-UO18 b). Nothing in the seeds or UO-01's placement worknote names a world-18 chamber. UO-01 has no row north of the hub. The navmesh (`data/spaces/omega_site.nav`) was probed on 2026-10-19 (below).
- **Probe 2026-10-19 (Pass A, no hits):** 99 points, x -120..180 step 30, z 100..500 step 50, y -45, v-tol 40, anchor `modi_seed78` (`nav_inspect`, output `.claude\reports\union-omega\probes\council-pass-a.log`). 0 confirmed hits on component 60 at z ≥ 200. 0 in-tolerance lines at z ≥ 200 on any component: every such line is out of tolerance, on components 226, 235, 237, 239, 243, 244 and 246, with dy +231 to +429 m. 3 in-tolerance lines at the hub's north edge (z 100: component 60 at x 150 and x 180; component 223 at x -90). Pass B has nothing to confirm.
- **Decision (Form 1, D-UO20):** The Council Chamber has no reachable geometry, so the world-80 fallback stands (D-UO20). The hearing plays in the existing world-80 Command Center geometry, reached by ring region 14 (beam). The Council is shown as hologram actors on the set. The probe found no reachable chamber, so the fallback is the venue for this design.
- Label: CIMMERIA_AUTHORING for the staging; the beam is RESTORED_SOURCE (`ring_transport_regions.sql`, see [omega-concept-art.md](../../omega-concept-art.md) line 198).
- Cast count: the Council cast is two named speakers, Thor (63836) and Otin (63837, only if D-UO34 says he speaks); UO-37 sealed both points. 63838-63839 stay reserved for a speaker the owner may add under D-UO34 (UO-38 ruling R4). Area name only here.

## § Prologue

Five prologue missions for levels 1-5, public world 18 with per-player state (no private alias, so no AS-E2 lab proof is needed). Rewards: U-D07 as defined in [union-beta-e1-continuation/reward-proposal.md](../../../union-beta-e1-continuation/reward-proposal.md) line 24: XP = 50 × level × factor, naquadah = 10 × level × factor, rounded to 5. The factor is 1.0 for difficulty 1 and 1.5 for difficulty 2. Side activities pay 50 %. Rounding to 5 takes halves up. Labels are CIMMERIA_AUTHORING unless noted.

| Mission | Level / difficulty / factor | Steps (what the player does) | Where | XP | Naq |
|---|---|---|---|---|---|
| P1 Post-Evacuation Briefing | 1 / 1 / 1.0 | S1 receive the briefing; S2 take delivery of the rescued databank (item grant, no 3399) | World 18, per-player | 50 | 10 |
| P2 Verify Custody | 2 / 2 / 1.5 | S1 verify databank integrity; S2 sign the custody log | World 18, per-player | 150 | 30 |
| P3 Contain the Threat | 3 / 2 / 1.5 | S1 locate the contamination; S2 contain it (one bounded encounter) | World 18, per-player | 225 | 45 |
| P3-S Side: Recovered Index | 3 / 2 / 1.5, side | S1 find the surviving index in the hub archive | World 18, per-player | 115 (225 × 50 % = 112.5, rounded up to 115) | 25 (45 × 50 % = 22.5, rounded up to 25) |
| P4 Bounded Rescue Choice | 4 / 2 / 1.5 | S1 choose one of two databanks to restore (a bounded flag); S2 return the other to custody | World 18, per-player | 300 | 60 |
| P5 Remote Council Hearing | 5 / 2 / 1.5 | S1 attend the hearing (UO-41); S2 the ruling step, which issues item 3399 and ends the prologue | World 80 Command Center set (§ Venue) | 375 | 75 |

- The step that issues item 3399 is P5 S2, the ruling's Accept (see § Hearing). The prologue ends with 3399 in the bag.
- Mission count: five, plus the optional side P3-S (six `missions` ids, 63800-63805). The side activity carries no item.

## § Hearing

- **Speakers:** Thor, template 63858 on moniker 28128 `Thor (Hologram)`. Otin (63859 on 28544) speaks only under a D-UO34 option (§ D-UO34 options). No speaker 2509 (S16). Speaker 777 is the stock Thor speaker and is a reference for lines (S15); the hologram uses 63858. The hearing Thor never offers 481 (§ Departure).
- **Dialog shape:** three screens (the databank custody, scarce-body rights, SGU assignment), each with one line from the Council, and one screen with a single **Accept** button (the `Accept` button row pattern at `dialog_screen_buttons.sql:6748`, RESTORED_SOURCE; CIMMERIA_AUTHORING for the new screen).
- **What the ruling changes:** one bounded state flag (`asgard_ruling`, three values) that a single follow-up line reads on the hub. It never blocks the hub or any hub service.
- **Event that starts it:** a `teleport_in` to world 80 at P5 S1 (the same pattern as 63820), with the mission active and step active.

## § Departure (481, gate 6, the producing row)

- **Offer predicate:** archetype `eq '5'` (S19); `mission_status 481 not_active` and not completed (once only, the same guard as 63820); item 3399 held. The item condition type is not in the chain vocabulary seen in `Content/Seed/` (conditions use `world`, `mission_status`, `step_status`, `archetype`), so the held check is an UO-42 implementation question, not decided here.
- **Who offers:** the hub Thor, template 63862 on moniker 28128, spawn 63873 (`asg.thor`, tag `UnionOmega_AsgardThor`), world 18, in the Asgard area beside trainer 63809. He offers 481 once P5 is complete, and again after an abandon (same predicate, D-UO10). The hearing Thor 63858 never offers 481 (UO-16 ruling R1).
- **Offer chain:** `accept_mission 481`, then the step at 1760 is active (verify in UO-42 that accept lands on index 0), then `grant_stargate_address 6` (S20 pattern). Item 3399 is not granted here; it was issued by P5.
- **Departure state (matches [omega-work-packets.md](../../omega-work-packets.md) § Producing, line 242):** 481 at S1760, item 3399 held, gate 6 granted. Beta's first event is 23310 (S19), which waits on `step_status 481 1760 eq active`. Omega fires no Beta step (D-UO2).

## § D-UO34 options

Evidence for 1522: S1-S6. The stock mission is enabled, but its two steps and objectives are disabled, no shipped dialog accepts it, and its Otin step is the only mention of Otin's role (S17). Its objective id 1522 is also a collision (S6).

| Option | What it is | Evidence | Cost | What it changes in the slice |
|---|---|---|---|---|
| (a) **Seal 1522** | Left as shipped and never offered. The hearing is the new prologue P5. | S1-S6; nothing accepts it (S18) | None to seeds | 1522 stays unused. The Asgard route uses only 481 plus the new prologue and hearing. |
| (b) Replace 1522's Otin step with the remote hearing | Rewrite step 4480 as the hearing step (new text) and keep 1522 as the carrier. | S2 text; S4 objective | A shipped-seed edit (RESTORED_SOURCE to CIMMERIA_RECONSTRUCTION), and the S6 collision must be fixed first | 1522 becomes the hearing carrier. UO-41 writes to a stock id. |
| (c) Offer 1522 as shipped | Otin's objective needs a physical Council report. | S4, S5 | High: world 59 has no map (asgard-route § Gates 3) | Not recommended. It shows an impossible place. |

**Otin call:** (no) Otin does not speak; the hearing is Thor 63858 and Otin is named only by text in the ruling. (yes) Otin speaks as 63859 on 28544. Evidence: no shipped speaker row for 28544 (S14, S17); his only lines are names in other people's dialog. A speaking Otin is a new template with no speaker evidence.

**Recommendation:** option (a) for 1522, and Otin does not speak. The hearing is the new prologue P5, Thor 63858 speaks, and 1522 is left unreached. This is the lowest-risk slice, needs no seed edit to shipped mission rows, and fixes no new evidence. Owner call (see § Open questions 1-2).

## § Asgard area

- **Trainer:** the art draws one Asgard class trainer in the **west small pod** at overview (338, 520) ([omega-concept-art.md](../../omega-concept-art.md) line 113, table line 186). Trainer template 63809 on spawnlist 63809 (UO-39) carries the shipped trainer 27218 (F14 range 27218-27264).
- **Vendors:** the art draws no Asgard vendor (line 186 count is 1 Asgard, the trainer). The 12 Asgard vendors (27252-27263) come from the shipped client monikers (F14). The split is RESTORED_SOURCE: texts.sql monikers 27252-27255 (Programs tiers 1-4), 27256-27259 (Armor tiers 1-4) and 27260-27263 (Crafting: Biomedical, Electronics, Power Systems, Materials Components). Spawnlist 63824-63835 (UO-37 seals), templates 63843-63854.
- **Placement keys UO-37 seals:** trainer (spawnlist 63809), 4 Programs (63824-63827), 4 Armor (63828-63831), 4 Crafting (63832-63835), Council cast (63836, 63837; 63838-63839 reserved), hub Thor `asg.thor` (spawn 63873, template 63862).
- **Art vs shipped:** the art gives positions for the trainer only; the vendor positions are for UO-37 to prove on the navmesh.

## § Slice

Packet ids UO-41..UO-49, about 40k working tokens each (none is M). Chain ids are from 63950-63999 (S21). "Waits on D-UO34" means the packet cannot be built until the owner answers; "playable Asgard start" is F42 (replay tests can stage archetype 5; live runs wait).

| Id | Role | Scope | Chain ids | Depends on |
|---|---|---|---|---|
| UO-41 | Chains (hearing) | Hearing state, the `asgard_ruling` flag, the 3-screen dialog, Thor's lines; the teleport start | 63950-63959 | UO-16; **D-UO34** (Otin line) |
| UO-42 | Templates and offer | Thor 63858 (28128); Otin 63859 (only if D-UO34 says yes); hub Thor template 63862 and spawn 63873; the 481 offer (§ Departure); gate 6 grant; the `asgard_departure` fixture in `chain_replay_tests/union_omega/handoff.rs` | 63960-63969 | UO-16; **D-UO34** (Otin half); playable Asgard start for live runs |
| UO-43 | Prologue P1 | Briefing and databank receipt | 63970-63972 | UO-16, UO-48 |
| UO-44 | Prologue P2 | Verify custody | 63973-63975 | UO-43 |
| UO-45 | Prologue P3 and side | Contain the threat; the side index | 63976-63980 | UO-44 |
| UO-46 | Prologue P4 | Bounded rescue choice and its flag | 63981-63985 | UO-45 |
| UO-47 | Prologue P5 ruling | Hearing ruling step; item 3399 issue; ends the prologue | 63986-63989 | UO-41, UO-46, UO-48 |
| UO-48 | Prologue rows | Mission, step, objective and text rows (ids reserved in § IDs); no chain; plus each mission's client copy, a `MISSION_ADDITIONS` entry pinned by a test (precedent: Dakara mission 63100, `crates/resources/src/base/mission_additions_dakara.rs`); the architect splits it if it exceeds 40k | none | UO-16 |
| UO-49 | Reward and acceptance | U-D07 reward chains for P1-P5 and the side; replay acceptance | 63990-63999 | UO-43..UO-47 |

- **Waiting on D-UO34:** UO-41 and the Otin half of UO-42. Prologue UO-43..UO-46 do not wait on it.
- **Waiting on a playable Asgard start (F42):** live runs of UO-41..UO-49. Replay builds do not wait.

## § IDs (used and still needed)

| Table | Reserved | Used by this design | Still needed |
|---|---|---|---|
| `content_chains` | 63950-63999 | Hearing 63950-63959; offer and gate 6 63960-63969; prologue 63970-63989; rewards 63990-63999 | none |
| `entity_templates` | 63858, 63859, 63862 (hub Thor, UO-16 ruling R1) | Thor 63858; Otin 63859 (D-UO34); hub Thor 63862 (UO-16 ruling R1) | none |
| `spawnlist` | 63836-63839, 63873 (hub Thor) | Council cast; hub Thor 63873 | none |
| `spawnlist` / `entity_templates` | 63809 | Asgard trainer 27218 (UO-39) | none |
| `entity_templates` | 63843-63854 | Asgard vendors (UO-39, UO-40) | none |
| `spawnlist` | 63824-63835 | Asgard vendors (UO-37) | none |
| `item_lists` | 63821-63828 | Asgard Programs and Armor (UO-38) | none |
| missions, steps, objectives, tasks, dialogs, texts | `missions` 63800-63805; `mission_steps` 63810-63829; `mission_objectives` 63830-63849; `mission_tasks` 63850-63869 | P1 63800, P2 63801, P3 63802, P3-S 63803, P4 63804, P5 63805; 11 steps, 11 objectives and 11 tasks, one each in mission then step order from the first id | dialogs, screens, buttons, texts and reward rows: the architect allocates them in the full packets |

## § Open questions

1. **D-UO34, mission 1522:** (a) seal it as shipped and unreached, (b) replace its Otin step with the remote hearing (a shipped-seed edit), or (c) offer it as shipped (not recommended: impossible place). Recommendation: (a).
2. **D-UO34, Otin:** does High Councillor Otin (28544) speak as template 63859, or is the hearing Thor-only? Recommendation: Thor only.
3. **Venue:** the probe found no reachable chamber on world 18; the hearing uses the world-80 Command Center (D-UO20). Owner: accept, or commission new geometry (out of scope).
4. **Prologue placement:** public world 18 with per-player state (recommended, no lab proof) or a private alias (needs AS-E2 lab proof and owner approval).
5. **Prologue shape:** five missions with the hearing as P5 (recommended), or the hearing as a separate mission after P4.
6. **Side activity:** keep P3-S (recommended), or drop it.
7. **Held item check:** the chain engine has no item condition in the vocabulary seen; UO-42 must confirm the held check or the owner accepts a flag instead.

## Ledger corrections

- S9: the ledger's "steps 1760/2078/2713" should read "step 1760 (objective 2078, task 2713)". No step 2078 or 2713 belongs to 481.
- S6 and S12: objective ids collide with step ids (1522, 3399, 4773). UO-41 and UO-47 must key rows by table, never by number alone.
