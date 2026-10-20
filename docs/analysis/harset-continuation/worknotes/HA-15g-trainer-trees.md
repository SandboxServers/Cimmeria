# HA-15g trainer trees: drafts, roster need, list reconciliation

Status: **PROVISIONAL** (O18-13). Offline design note (PU-07). No IDs are allocated here (HA-D09); every template, list and spawn number below is a placeholder for the coordinator.

Labels follow the ledger: FACT (checked in the repo or a tool run), INFERENCE, PROVISIONAL, DECISION, MAINTAINER DECISION.

## 1. Summary

- FACT: the workbook export has 439 nodes over 7 archetypes and 21 branches. Seeded `trainer_abilities` carries 439 rows in list 1 and 6 in list 350. The runtime keys by `(list_id, archetype)` and filters by the player's archetype, so one list can hold every archetype.
- FACT: HA-15f cites the original omega lists as "1-6" and "350-353". Only lists 1 and 350 exist in the seeds or in git history; 2-6 and 351-353 never did. Treat the citation as unverified.
- PROVISIONAL: the 19 existing trainer monikers (6 Global 7988-7993, 11 TBD_Harset 7994-8004, 2 Omega Asgard 27218, 27219) map onto a 19-template split of the workbook (section 5). HA-15g's "about 21" (its section 1 item 6) is the figure for all service NPCs, not a trainer count; its section 7 table gives 6 basic trainers and about 19 templates in all at first seal. Moniker 20186 (`DN_npc_trn_Global_Trainer`, "Archetype Skills Trainer") is a generic 20th trainer moniker, used only by debug template 301.
- Roster: 9 candidate IDs 261-269 (`id-allocation.md` line 35). First seal needs 19 templates (6 trainers plus 13 non-trainer, HA-15g section 7), a gap of 10. The full roster with all 19 trainers needs 32, a gap of 23. That is a coordinator call under O14-04 (section 6).
- BLOCKER for HA-15c (not for this note): `generate_seed.py --from-json --check` exits 2 because ability 1984 is both the Heritage #1 node and a CS-02 Free Jaffa starter (`ALLOWED_STARTER_COLLISIONS`, generate_seed.py lines 117-121). The generator also emits only list 1 (`TRAINER_DEBUG_LIST_ID = 1`), so multi-list output needs a generator extension.

## 2. Inputs and method

- Workbook export: `docs/analysis/ability-trees/source/trainer_server_export.json` (read only). `tools/ability_trees/generate_seed.py` was run with `--check` only; no seed output was regenerated or committed.
- Seeds read: `db/resources/Abilities/Seed/trainer_abilities.sql`, `trainer_ability_lists.sql`, `db/resources/Entities/Seed/entity_templates.sql` (templates 25, 301, 360 carry lists), `db/resources/Texts/Seed/texts.sql` (monikers).
- Runtime read: `crates/cell-interactions/src/cell/interactions/trainer.rs`.
- Placement probes: `nav_inspect` against `data/spaces/harset.nav`, read only (section 8).

## 3. Engine facts that constrain the design

- FACT: `trainer_abilities(list_id, archetype, ability_id)`; both `trainer_abilities.list_id` and `entity_templates.trainer_ability_list_id` have FKs to `trainer_ability_lists(list_id)` (`db/resources/_foreign_keys.sql`).
- FACT: `evaluate_train` computes the `trainable` byte with the D-AT01 branch-spend counted across the archetype, not across the list. Nodes missing from the offered list are hidden, not greyed (audit A-23). Splitting a branch over two trainers therefore does not break spend gating; it only changes which trainer shows which node.
- FACT: a player whose archetype has no rows in the clicked trainer's list gets an empty window and a `trainer_empty_offering` WARN. A wrong-archetype click is not an error condition the client can explain.
- FACT: the two trainer templates in the seeds, 301 and 360, use `interaction_type` 128, faction 1, level 1 and no `speaker_id`. Template 25 (also list 1) has `interaction_type` 0 and is named "Interaction Debug NPC - DO NOT USE"; it is not a pattern to copy. Quest speakers must not double as trainers.
- FACT: monikers cannot be invented without a client change. `texts.sql` holds 20 `DN_npc_trn_` monikers: the 19 in section 1 plus 20186 (generic, template 301). Those 20 are the trainer vocabulary in the seed.

## 4. Reconciliation of lists 1 and 350

| Item | List 1 | List 350 |
|---|---|---|
| Description | Debug ability list | Pet trainer (debug hub) |
| `trainer_abilities` rows | 439 (the full workbook) | 6 |
| Sequence | `setval('trainer_ability_lists_list_id_seq', 1)` | n/a |

- FACT: list 1 is the generator's output and equals the workbook node-for-node. It is a debug list: every archetype's whole tree is offered by whichever template points at it.
- FACT: list 350 is hand-authored pet-trainer content, outside the generator. The generator must keep carrying it over unchanged. It holds six `ARCHETYPE_Goauld` rows (1643, 1644, 1645, 1652, 1654, 2826) and is attached to template 360 (`Debug Hub - Pet Trainer`, moniker 8000). Nodes 1643 and 2826 are the Servant Lord root and capstone (section 5).
- INFERENCE: HA-15f's "1-6 / 350-353" reads like the omega's range, i.e. six player lists and four pet lists. No seed or git history backs it. Do not reserve those IDs on that basis.
- DECISION (proposed): keep list 1 as the debug/GM list; add the per-trainer lists as new rows allocated by the coordinator. Do not repurpose 1 or 350.

## 5. Tree partition (PROVISIONAL)

Rule: Basic trainers take unlock level <= 20; Advanced trainers take level >= 25, split per branch group where the moniker names one. Level 20 is the cut because the workbook has no node at levels 21-24, and counts per level up to 20 are 196 nodes vs 243 above. That gap exists because workbook levels are multiples of 5, so it marks the level grid rather than a proven cluster boundary. LOW confidence on the cut itself: a different grouping (for example 15) would also be defensible, and the omega's real boundary is unknown.

| Template (moniker) | name_id | Archetype | Branch scope | Level | Nodes |
|---|---|---|---|---|---|
| Soldier Basic | 7988 | Soldier | all | 1-20 | 33 |
| Scientist Basic | 7989 | Scientist | all | 1-20 | 27 |
| Jaffa Basic | 7990 | Free Jaffa / Shol'va | all | 1-20 | 23 |
| Goa'uld Basic | 7991 | Goa'uld | all | 1-20 | 27 |
| Commando Basic | 7992 | Commando | all | 1-20 | 28 |
| Archaeologist Basic | 7993 | Archaeologist | all | 1-20 | 29 |
| Soldier Advanced | 7994 | Soldier | all | 25-50 | 39 |
| Scientist Advanced | 7995 | Scientist | Medical, Robotics | 25-50 | 21 |
| Scientist Support | 7996 | Scientist | Support | 25-50 | 11 |
| Jaffa Tau'ri | 7997 | Free Jaffa / Shol'va | Tau'ri | 30-50 | 4 |
| Jaffa Advanced | 7998 | Free Jaffa / Shol'va | Heritage, Tactics | 25-50 | 24 |
| Goa'uld Ashrak | 7999 | Goa'uld | Ashrak | 25-50 | 12 |
| Goa'uld Advanced | 8000 | Goa'uld | Battle Lord, Servant Lord | 25-50 | 23 |
| Commando Advanced | 8001 | Commando | Demolitions, Precision / Marksmanship | 25-50 | 24 |
| Commando Infiltration | 8002 | Commando | Stealth / Infiltration | 25-50 | 12 |
| Archaeologist Advanced | 8003 | Archaeologist | Anthropology, Archaeology | 25-50 | 25 |
| Archaeologist Support | 8004 | Archaeologist | Sociology | 25-50 | 11 |
| Asgard Basic | 27218 | Asgard | all | 1-20 | 29 |
| Asgard Advanced | 27219 | Asgard | all | 25-50 | 37 |

- FACT: per-archetype node totals check out against the workbook (Soldier 72, Scientist 59, Jaffa 51, Goa'uld 62, Commando 64, Archaeologist 65, Asgard 66; 439 in all). Every node lands in exactly one template.
- LOW confidence: the moniker-to-branch mapping for the Advanced and Support trainers is inferred from the names. "Archaeologist Support" = Sociology and "Scientist Support" = Support are the weakest; "Jaffa Tau'ri" = Tau'ri is the strongest (a named branch).
- INFERENCE: Tau'ri has only 8 nodes (levels 1, 5, 15, 20, 30, 35, 45, 50), so the Jaffa Tau'ri trainer carries the four at level 30+ and the Basic Jaffa trainer carries the first four. Node 1 of the branch is `2104 Lock and Load` at level 1.

### Branch shape (workbook)

| Archetype | Branch | Nodes | <=20 | >20 | Root | Capstone |
|---|---|---|---|---|---|---|
| Soldier | Automatic Weapons | 22 | 9 | 13 | 598 Quick Burst | 1477 All Out Assault |
| Soldier | Heavy Weapons | 25 | 11 | 14 | 612 Area Burst | 1486 Rain of Steel |
| Soldier | Command | 25 | 13 | 12 | 1642 Stance: Soldier | 1893 Stance: Commander |
| Commando | Demolitions | 23 | 10 | 13 | 642 Anti-Personnel Mine | 1868 Tactical Nuke |
| Commando | Stealth / Infiltration | 22 | 10 | 12 | 646 Stealth I | 1357 Execution |
| Commando | Precision / Marksmanship | 19 | 8 | 11 | 655 Snap Shot | 1359 Red Mist |
| Scientist | Medical | 22 | 10 | 12 | 948 Medical Attention: Battlefield Heal | 1224 Deployable: Med Station |
| Scientist | Support | 20 | 9 | 11 | 1013 Personal Shield | 1235 Shield: Universal |
| Scientist | Robotics | 17 | 8 | 9 | 962 Summon Turret: Prototype | 1213 Summon Turret: Dual Turrets |
| Archaeologist | Anthropology | 21 | 9 | 12 | 778 Conversation: Mediator | 1184 Indistinguishable |
| Archaeologist | Sociology | 20 | 9 | 11 | 643 Induce Caution: Humanoid | 777 Induce Mass Panic: Humanoid |
| Archaeologist | Archaeology | 24 | 11 | 13 | 802 Reveal Mini-Games | 1195 Flawless Victory |
| Asgard | Attack Programs | 24 | 11 | 13 | 1533 Attack Protocol: Apply Damage | 3173 AttackProtocol:TeleportationStrike |
| Asgard | Defensive Programs | 21 | 9 | 12 | 1561 Defensive Protocol: Reroute Energy: Basic | 3265 DefensiveProtocol: AreaShield |
| Asgard | Scientific Programs | 21 | 9 | 12 | 1554 Science Protocol: Synaptic Clarity | 3246 ScienceProtocol: TimeShift |
| Free Jaffa / Shol'va | Heritage | 21 | 9 | 12 | 1984 Staff Swing | 2043 Weapon of Terror |
| Free Jaffa / Shol'va | Tau'ri | 8 | 4 | 4 | 2104 Lock and Load | 2126 MS020_080817_WeaponSpecialization:Ranged |
| Free Jaffa / Shol'va | Tactics | 22 | 10 | 12 | 1723 Stance: Courage | 2073 Battlefield Warrior |
| Goa'uld | Ashrak | 21 | 9 | 12 | 2853 Cloak I | 2862 Inevitable End |
| Goa'uld | Battle Lord | 21 | 9 | 12 | 1639 Ribbon Device:Destruction Beam | 2718 Wrath of God |
| Goa'uld | Servant Lord | 20 | 9 | 11 | 1643 Summon Jaffa | 2826 Summon Straegis |

- FACT: the Basic rows equal the "<=20" column sums per archetype (Soldier 33, Scientist 27, Jaffa 23, Goa'uld 27, Commando 28, Archaeologist 29, Asgard 29). The Advanced rows equal the ">20" sums once Jaffa Tau'ri (4) is split out of Jaffa's 28.
- FACT: moniker 8000 is already in use. Template 360 (`Debug Hub - Pet Trainer`, list 350) carries name_id 8000 "Goa'uld Advanced Skills", and list 350 already holds Servant Lord nodes 1643 and 2826. The Goa'uld Advanced template above would reuse that name. Debug hub and Harset are separate worlds, so the duplicate is not a runtime clash, but the coordinator should confirm the reuse (section 4).

## 6. Roster need and the O14-04 call

- FACT: 19 trainer templates cover the full tree with the existing monikers (17 per-archetype/branch plus 2 Asgard). `id-allocation.md` line 35 offers 9 IDs (261-269).
- FACT (HA-15g section 7): first seal takes 6 basic trainers and 13 non-trainer templates (6 vendors, 1 consumables, 2 crafting, 3 mission reward, 1 registrar), 19 in all; advanced trainers are deferred there. Full roster (19 trainers plus 13): 32 templates.
- Gap: 10 at first seal, 23 for the full roster. Options for the coordinator (none chosen here):
  - A. Extend the template range: 10 more IDs for first seal, 23 for the full roster (cleanest; one moniker per template as the omega did).
  - B. Ship the 9 for trainers only, reusing list 1 for the rest; the 13 non-trainer templates still need IDs from elsewhere. Loses the per-branch story.
  - C. Drop Asgard (2) and Jaffa Tau'ri (1) for now: 16 trainers, 29 with the 13 non-trainer templates, still 20 over the 9 IDs.
- INFERENCE: option A is the only one that preserves the omega layout. The cost is ID space, not code.
- Not decided here: spawn count per template. HA-15g plan section 4.1 holds the slot geometry; this note only adds the roster arithmetic.
- Asgard has no Harset trainer in the HA-15g plan (open item). Without it the trainer need is 17 and the full roster 30, 21 over the 9 IDs.

## 7. Seeding path for HA-15c (hand-off, not done here)

1. Extend `generate_seed.py` with a list-spec (list_id, archetype, branch set, level range) so it can emit several lists. Today it emits only list 1 and carries other lists over verbatim.
2. Resolve the 1984 collision first: ability 1984 is Heritage #1 and a CS-02 Free Jaffa starter, so `--check` exits 2. Either list it in `ALLOWED_STARTER_COLLISIONS` with the runtime `credited_grants` rationale, or drop it from the Heritage root. That is an HA-15c decision, not made here.
3. New rows go in `db/resources/Abilities/Seed/` and `Entities/Seed/entity_templates.sql` only (no migrations).
4. Keep list 1 and list 350 untouched.

## 8. Placement probes (offline, `nav_inspect`, `harset.nav`)

- INFERENCE (no recorded run): bank 314 candidate (-186, -41.28, 160) is on navmesh component 11 at 0.58 m horizontal offset, with a lower floor (component 236) 8.85 m directly below. A tolerance wider than about 1 m can snap it to the wrong floor. Probe sensitivity is high; the HA-02 precedent (component membership as the connectivity surrogate) applies. `data/spaces/harset.nav` is tracked in git, so the probe can be re-run through the lane: `cargo run -p cimmeria-navmesh-extractor --bin nav_inspect -- data/spaces/harset.nav --probe bank314=-186,-41.28,160`. Record the output here before promoting this to FACT.
- INFERENCE (not checked): that the trainer ring numbers reproduce on this mesh has no recorded run and no ring coordinates in this note.
- NOT DONE: `find_path` runs, clearance against doors and props. No occluder or prop data was checked. Treat every position as unverified until a lab pass.

## 9. Open items

1. Level-20 cut and the Support-branch moniker mapping are low confidence (section 5).
2. Asgard has no Harset trainer placement.
3. Wrong-archetype click gives an empty window plus a WARN; a gating mitigation (greeting text per template) needs a text, which needs a client change if the moniker is new.
4. Whether a Basic trainer should also offer the `credited_grants` creation-granted nodes.
5. Pet list 350 and the full-debug list 1 stay as is; no decision on retiring list 1 in Harset.

## 10. Hand-offs and tests owed

- Coordinator: O14-04 roster size (section 6); ID allocation (HA-D09).
- HA-15c: generator list-spec, 1984 fix, seed rows.
- HA-15b/HA-15g: spawn placement for the chosen roster.
- Tests owed when seeds land: each template's list covers exactly its archetype/branch/level scope; every workbook node appears in exactly one Harset trainer list; no template has an empty list for its intended archetype (the `trainer_empty_offering` WARN must not fire in a lab pass).
- Telemetry: `trainer_empty_offering` count per template in the lab pass.
