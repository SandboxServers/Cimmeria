# Omega Site work packets

> **Type:** How-to (dispatch contract and ledger)
> **Audience:** the Union Omega coordinator, packet-coders, review-fixers, mission, movement and items advisors
> **Last updated:** 2026-10-19 (owner decisions and the services re-slice, F36-F44, D-UO27..D-UO37), first written 2026-10-18 against `origin/main` @ `d1bba0400`; developer concept art added the same day @ `eb29b0c2c` (F27-F35, D-UO15..D-UO20)
> **Status:** W1-W5 merged (PRs #1890, #1902, #1908, #1911 and the close-out PR); never walked in a client. On 2026-10-19 the owner answered D-UO8, D-UO11, D-UO16..D-UO19, U-D07, H-LBL, U-D08 and U-D11 (§ Owner decisions 2026-10-19). The services and Asgard work is re-sliced into waves W6-W11 (§ Packets, § Waves): W6 is Ready. Still open for the owner: D-UO32 (inn and battlemaster), D-UO33 (medic name), D-UO34 (1522 and Otin, after the UO-16 seal). Not run: the UO-U1 lab UAT, which the owner runs. This file replaces the 2026-10-14 table (UO-00..UO-08); see § Renumbering.
> **Companions:** [Omega campaign](omega.md), [Union execution contract](execution-contract.md), [SGC packets](sgc-work-packets.md), [Dakara packets](dakara-work-packets.md), [Beta packets](beta-work-packets.md), [Asgard route](asgard-route.md), [developer concept art](omega-concept-art.md), [packet folder](omega-packets/), [resume card](omega-packets/RESUME.md)

## Short answer

Omega is the first common hub. A Human or Free Jaffa arrives on public `Omega_Site` (world **18**) on mission **1655** at step **4940**. The steps are:

1. Talk to Modi (objective 5645).
2. Beam to the Command Center (`Omega_Site_CmdCenter`, world **80**, pad region **17**) on the Asgard transporter, which uses the ring-transport dispatch. The beam advances the mission to step **4942**.
3. Talk to Daniel Jackson (Human, dialog **4641**) or Bra'tac (Free Jaffa, dialog **5025**). Accept completes 1655, offers **1423** or **1500** (`The SGU`), and grants Beta's address (gate **6**).

Beta owns everything after that. Merged so far: a personal banker, two registrars (Team and Command) and an ambient crowd of airmen and Free Jaffa warriors, all on shipped monikers on the shipped map.

The owner's [developer concept art](omega-concept-art.md) (RESTORED_SOURCE layout intent, never coordinates) splits world 18 into a Common Area hub, an SGC wing and a Free Jaffa wing, with a Council Chamber to the north. On 2026-10-19 the owner chose the large roster:

- **Vendors (D-UO8 = B, D-UO16 = B):** every tier the client names (Tiers 1-4 of Human and Jaffa weapons and armor, four crafting specialties for Human, Common and Jaffa, and Consumables), on the shipped monikers, with extra spawns at the art's spots. Wave W8.
- **Trainers (D-UO17 = c):** five class trainers on the global monikers, plus eight crafting "guides" (no crafting-trainer mechanic exists, so they speak one line each; D-UO31). Wave W9.
- **Hub services (D-UO19 = c):** wherever an engine service exists. The Auction House gets the Black Market auctioneer and the Infirmary a healer plus a respawner. The Inn and Battlemaster have no engine service (D-UO32, BlockedDecision). Wave W9.
- **Asgard (U-D08, U-D11, D-UO18 = b):** a remote Council hearing in the Council Chamber, with a databank prologue that issues item 3399. It starts with a design seal (UO-16, W6); its seeds (W10) wait on that seal, on D-UO34 and on a playable Asgard start.

Everything still uses existing engine primitives. Nothing needs a client patch, a new opcode or a code change: the packets are seeds, chain SQL and tests. The Black Market window needs the Cimmeria client patch to draw, but its first-press chat line works without it (D-UO36). Every code packet is Haiku-sized (at most 40k working tokens).

## Findings

Every finding was checked against the planning baseline `d1bba0400` (an ancestor of `main`); locate each anchor by its text, not its line number. The packet column names the packet that acts on it.

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | Omega has no chain today. `db/resources/Content/Seed/` has no Omega file, and nothing binds dialog set map 7015 (Modi's 6168, `Shutdown`, flags 16777216). | `git grep 7015 db/resources/Content` is empty | UO-04 |
| F2 | 1655's world-18 and world-80 steps exist: S4940 (optional O5645 Modi and O5646 waypoints; required O5647 beam controls and O5644, blank text) and S4942 (O6056 → T7741 Jackson; O6058 → T7743 Bra'tac). | `db/resources/Missions/Seed/` | UO-04, UO-07, UO-08 |
| F3 | `add_dialog_set` targets a **dialog_set_map id**, and the slot is the template id. It does not dedupe; `remove_dialog_set` is `retain(dsm_id != id)`. | `executor/dialog/mod.rs:191,195-199,285` | all chain packets |
| F4 | A bound map whose `dialog_id` is set opens on click through `handle_interact`, with no chain. An `interact_tag` chain would short-circuit that, skip the `last_interaction_target` pin, and show the dialog twice. | `dispatch/interact.rs:113-148`; `chain_replay_tests/mission_1324.rs:17-28` | D-UO4 |
| F5 | `remove_dialog_set` must run before `complete_mission`, which awaits `fire_mission_completed` mid-list. | `mission_1324.rs:46-53` | UO-07..09 |
| F6 | Dialogs 4641 and 5025 each have exactly one button, `Accept`, on the final screen (buttons 3439, 3687). 6168 has none, so it fires `dialog_choice` with -1 on close. | `dialog_screens.sql`, `dialog_screen_buttons.sql` | UO-04, UO-07, UO-08 |
| F7 | The 4641 scene is Jackson (speaker 431) with Bra'tac (2323). The 5025 scene is Bra'tac, Teal'c (1333), Thor (777) and Jackson. Every one of the four stands in the Command Center (RESTORED_SOURCE). | `dialog_screens.sql` 104987-105000, 118847-118857 | UO-05 |
| F8 | `fire_dialog_choice` and `fire_teleport_in` set `archetype`. The castle_701 header comment saying otherwise is stale. | `event_dispatch/dialog.rs:113`, `region.rs:158`, `dialog_archetype_tests.rs` | all chain packets |
| F9 | On a beam into world 80, `player_loaded 'Omega_Site_CmdCenter'` fires before `teleport_in '17'`, and both evaluate the pre-event context. | `player_init/mod.rs:532`, `event_dispatch/region.rs:140` | UO-04, UO-07, UO-08 |
| F10 | Archetypes: Human 1-4, Asgard 5, Goa'uld 6, Free Jaffa (Sholva) 7, Jaffa 8. A missing archetype reads -1. | `EArchetype.sql`, `dialog_archetype_tests.rs` | gates |
| F11 | Respawner 26 stands in world 80, 3.9 m east of pad 17. World 18 has none, so a death on the hub sends the player elsewhere. | `respawners.sql:202-207` | UO-03 |
| F12 | The cast templates have `interaction_type 0`, and Modi (65) has no static sets. A per-player map must carry non-zero flags, or the NPC shows no prompt. | `entity_templates.sql:111-127` | UO-05 |
| F13 | Stock dialog set 1477 → 4641 (map 5432) has flags 0, and the set-to-mission mapping 1477/1573/1574 → 1423/1500/1522 is contextual inference. | `dialog_set_maps.sql`; Omega master | D-UO6 |
| F14 | The client ships Omega monikers for a banker (29462 `Storage Officer`), a registrar (29068 `Organization Registrar`) and vendors and trainers 27218-27264. The tree already has a banker clone pattern (370) and registrar patterns (330 Team, 331 Command). | `texts.sql`; `live_db_debug_banker.rs`, `live_db_debug_registrars.rs` | UO-06, UO-10..12 |
| F15 | Item 6786 `Letter to Daniel Jackson` is granted by no seed, and no Bra'tac letter exists. 4641's first screen mentions a letter. | `git grep 6786 db/resources` | D-UO11 |
| F16 | Both navmeshes are committed: `data/spaces/omega_site.nav` and `omega_site_cmdcenter.nav`. `nav_inspect` and the DK `travel.rs` helpers work on them. | `git ls-files data/spaces` | UO-01, UO-03, UO-05 |
| F17 | Seed sequences: the template floor is 1409 and the spawn floor 13869. The dialog sequences already stand above 100000, so 638xx needs no dialog footer. | `live_db_seed_sequences.rs`, `dialog_set_maps.sql:9355` | UO-02 |
| F18 | Seven chain-replay helpers in `mission_1324.rs` are `pub(super)`: `ctx_for`, `with_mission`, `with_step`, `fire_player_loaded`, `fire_dialog_choice`, `fire_mission_completed` and `display_dialog_count`. `labels` filters Harset's range, so Omega needs its own. | `mission_1324.rs:97-260` | UO-02 |
| F19 | SGC's ledger listed stale Omega blocks (12000-12399, 63000-63099, 64000-64199); the last overlapped Beta's 64000-64499. **Fixed** by the planning review pass: `sgc-work-packets.md` § ID allocation now carries a per-zone table with Omega at 63800-63999. UO-00's handoff note asks the coordinator to confirm it. | `sgc-work-packets.md` § ID allocation | UO-00 |
| F20 | Dakara UD-04a gates the Tau'ri as `archetype lte 6`, which also admits Asgard (5) and Goa'uld (6). This is a cross-zone note only; Omega uses the Union Human gate (`gte 1` plus `lte 4`, [execution-contract.md](execution-contract.md) § Shared definitions). | `dakara-packets/UD-04a.md` | report |
| F21 | The engine completes the whole mission once every required objective on its current step is complete (vacuously, on a step with none). S4940 has two required objectives, O5647 and O5644, beside the optional O5645 and O5646. Completing O5645 or O5646 alone completes nothing, but completing O5647 and O5644 completes all of 1655 before the Accept. `advance_step` moves to the next step without completing the mission. So no Omega chain uses `complete_objective` on 1655; the beam uses `advance_step` (D-UO3). *Corrected 2026-10-18 (D-UO25): an earlier text said both S4940 objectives were optional. The rule is unchanged.* | `mission_objectives.sql:3136-3142`; `crates/cell-content/src/cell/missions/progression.rs:463-506`; `crates/entity/src/missions.rs:91` | UO-04, UO-07..09 |
| F22 | `mission_abandoned` fires after the mission instance is removed, so the abandoned mission already reads `not_active` on that event. INFO "fire_mission_abandoned: matched". | `event_dispatch/mission.rs:217`; `chain_replay_tests/mission_abandoned.rs:50-67` | UO-09 |
| F23 | The 1655 guard: every delivery chain (63820-63822, 63830-63832) carries `mission_status 1655 eq active`, every re-offer chain (63840-63845) carries `eq completed`. The two families never both match one event. | design rule; tested in UO-07..09 and UO-13 | UO-07..09 |
| F24 | A registrar's type comes from its single interaction set: 7447 is Team, 7448 is Command. A template with both sets has no type, so the one Omega registrar moniker needs two NPCs. | `org_registrar.rs`; `live_db_debug_registrars.rs:135-148` | UO-10 |
| F25 | The client names four tiers per Omega weapon and armor vendor (for example 27240-27243 `DN_npc_ven_OmegaSite_HumanWeaponsTier01..04`) and four specialties per crafting vendor. One template per role is a reduction. | `texts.sql:37085-37091` | UO-06, UO-11, UO-12 |
| F26 | No Omega ambient moniker ships. Shipped bodies exist: template 31 `Airman` (faction 1, name 8360, interaction 0) and template 34 `SGC Jaffa`; moniker 26723 `Free Jaffa Warrior` is already reused by template 208 (INFERRED). | `entity_templates.sql:67,69,741-743` | UO-14 |
| F27 | The owner supplied developer concept art of world 18 with proposed NPC spots: one overview plus enlargements of the Common Area, the SGC wing and the Free Jaffa wing. It is one map: Council Chamber (north), Common Area hub (centre), SGC (south-west), Free Jaffa (south-east). Labels, kinds (colour) and relative placement are RESTORED_SOURCE; positions are approximate, and the art is undated. | [omega-concept-art.md](omega-concept-art.md) | UO-01 |
| F28 | The art's four teleporters and its hub Command Center match world 18's seeded beam network. Regions 10/11/12/16 each list the other three as destinations, and region 14 beams only to world-80 region 17. The client ships six destination names for these six regions: 26927 Jaffa Plaza, 26973 Human Plaza, 26981 Bank, 26982 Asgard Platform, 26983 Omega Site and 27423 Command Center. Which name belongs to which region is INFERRED. | `ring_transport_regions.sql` (regions 10-17); `texts.sql:36488-37451` | UO-01 |
| F29 | Vendors in the art: General 8, Consumable 8, Weapon 5 and Armor 5 across all three areas, plus Adv. Weapon and Adv. Armor in the north-west pod, and **no crafting vendor**. General, Adv. Weapon and Adv. Armor have no Omega moniker; the client's crafting monikers have no art spot. | [omega-concept-art.md](omega-concept-art.md) § Totals, § Joins | UO-06, UO-11, UO-12 (D-UO16) |
| F30 | Trainers in the art: six class trainers (Asgard in the hub's west pod; Scientist, Archaeologist, Soldier and Commando in the SGC wing; Jaffa in the Free Jaffa wing) and eight crafting trainers (Engineering, Electronics, Biology, Physics and an Adv. of each). The client ships global basic-skill monikers 7988-7990, 7992 and 7993, Omega Asgard trainers 27218/27219, and **no crafting-trainer moniker**. A trainer clone pattern exists: template 301 (`INT_TRAINER` 128, trainer list 1). | `texts.sql` `DN_npc_trn_*`; `entity_templates.sql:1003` | D-UO17 |
| F31 | Hub locations in the art: Bank, Guild Center, Auction House, Inn, Infirmary, Battlemaster and Science Center. The Bank matches banker 29462 and beam name 26981; the Guild Center matches registrar 29068 (INFERRED); the Science Center matches the Omega master's 1425/1426. The other four have no Omega moniker. | [omega-concept-art.md](omega-concept-art.md) § Joins | UO-10; D-UO19 |
| F32 | A Council Chamber with three "Combate Instance" (sic) labels stands north of the hub. Its ship-like silhouette may be the Asgard Capitol Ship (INFERRED). It conflicts with U-D08 (remote hearing), the 81/82 scope line and the Rejected list. | [omega-concept-art.md](omega-concept-art.md) § Council Chamber | D-UO18 |
| F33 | The art draws the Command Center at the hub's north end, directly above the Stargate. Shipped data puts it on world 80 via region 14, which lies on the gate's x (34.6 vs 34.7), 205 m +z of it. | `ring_transport_regions.sql`; F2, F7 | D-UO20, UO-05 |
| F34 | Orientation, INFERRED, with art up = +z: east and west agree (Human props and region 12 to the west, Jaffa props and region 16 to the east), but north and south only partly. The Jaffa props sit north of the gate, while the art puts the wing south of the hub. | [omega-concept-art.md](omega-concept-art.md) § Orientation | UO-01 |
| F35 | The Common Area enlargement labels three small pods differently from the overview: the east pod has Armor only, the south-east pod Weapon, the south-west pod General and Consumable. Unresolved; the overview is primary. | [omega-concept-art.md](omega-concept-art.md) § Cross-check | report (non-blocking) |
| F36 | Trainer list 1 (`Debug ability list`) already holds a full tree per archetype: Soldier 72 abilities, Commando 64, Scientist 59, Archeologist 65, Asgard 66, Goa'uld 62, Sholva (Free Jaffa) 51. A click runs `try_open_trainer` first: `template_trainer_lists` gives the list, then `trainer_abilities[(list, archetype)]`. An empty offer logs WARN `trainer_empty_offering` ("content gap"). | `db/resources/Abilities/Seed/trainer_abilities.sql`; `crates/cell-interactions/src/cell/interactions/trainer.rs:53`; `crates/cell-catalog/src/cell/spawner/abilities.rs:191,235` | UO-31 (D-UO30) |
| F37 | No crafting-trainer mechanic exists: the trainer arm serves `trainer_abilities` only, and only lists 1 and 350 exist. No crafting-trainer moniker ships (F30). Template 1300 (interaction 128, no trainer list) is the precedent for an NPC whose click an `interact_tag` chain claims. | `trainer_ability_lists.sql`; `entity_templates_debug_area_plaza.sql:24`; allowlist `debug_area_plaza_chains.sql` 13000 | UO-35 (D-UO31) |
| F38 | The Black Market auctioneer is a working service. Template 305 (moniker 7133, interaction 4 `INT_AUCTION`) plus an `interact_tag` chain running `open_black_market` sends `onBMOpen` (90) and the chat line "The auctioneer opens the Black Market. (No window? The Black Market needs the Cimmeria client patch.)". On an NPC that is not an auctioneer it says "Nobody here runs the Black Market." and logs `bm.open_refused`. | chains 13008 (Debug Area) and 5030 (Castle); `chain_replay_tests/black_market_auctioneer.rs` | UO-32 (D-UO36) |
| F39 | No engine service exists for an inn (rest) or a battlemaster (PvP queue or instances), and no moniker for either ships. | `texts.sql` (no `Innkeep`, `Bartender` or `Battlemaster` moniker); no executor or interaction arm | D-UO32 |
| F40 | A chain can heal and speak. On an `interact_tag` chain the executor's entity is the clicking player, so `change_stat` with `{"stat_id": 7, "set_to_max": true}` refills that player's health (stat 7 is `HEALTH`) and logs "Content: ChangeStat applied". `npc_bark` sends a seeded screen's text to that player only (method 28) and logs "Content: npc bark". | `event_dispatch/interaction.rs:68`; `executor/stats.rs:74`; `executor/bark.rs:123`; `entity/src/stats/stat_ids.rs:18` | UO-33, UO-35, UO-36 |
| F41 | No medic moniker ships for Omega. The nearest shipped names are 6964 `Castle Medic`, 7000 `Nurse Collins` and 7952 `SGC Scientist`. | `texts.sql:52976` | D-UO33 |
| F42 | An Asgard player cannot be created today: character-creation row 9 is `NON_CANONICAL_BLOCKED_LEGACY` ([asgard-route.md](asgard-route.md), AS-E1). The Council monikers 28128 `Thor (Hologram)` and 28544 `High Councillor Otin` ship. | `asgard-route.md`; `texts.sql` | UO-16, D-UO34 |
| F43 | The old ID map cannot hold every tier. Its five Tau'ri and three Jaffa template ids, ten spawn ids and `× 100` list-row rule (which passes the 639999 floor after list 63819) are too small. Free ids, checked 2026-10-19: templates 63804-63809 and 63812-63859; spawns 63804-63819, 63822-63872 and 63960-63971; lists 63800-63828; dialog 63900 and screens 63901-63907 (stock screens collide only at 63847-63887); respawner 181. | `git grep` over `db/resources` | D-UO27 |
| F44 | Existing guards bind every new spawn. Every spawn 63800-63999 must stand on the hub mesh, reach Modi and keep 1.5 m from the others. Each ambient spawn keeps 3.0 m from every service spawn, and spawns 63880-63959 may hold ambient templates only. The services test keeps 2.5 m between service NPCs and 3.0 m from respawner 180. `live_db_omega_writes_no_interact_tag_chain` refuses every `interact_tag` chain in 63800-63999, so the services chains need an amendment. | `spawner_tests/union_omega/mod.rs:170`, `ambient.rs:136-199`; `live_db_union_omega_services.rs:47-48`; `chain_replay_tests/union_omega/arrival.rs:214` | UO-17..UO-20, UO-24 |

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-UO1 | Omega's block is **63800-63999** for every table (§ ID map). | Decided; UO-00 seals the row | It sits between Dakara (63500-63799) and Beta (64000-64499) with no gap or overlap. |
| D-UO2 | Omega fires no Beta step. It hands over 1423/1500 at their first step and gate 6, and Beta's receivers 23020/23220/23310 take it from there. | Decided; confirms D-UB2 | One owner per step. |
| D-UO3 | The beam advances 1655 to 4942 on `teleport_in '17'` in world 80 while 4940 is active. | Decided | It is the first server event that proves the player reached the Command Center. |
| D-UO4 | The induction runs on `dialog_choice` (the Accept), with clicks opening the bound map. There are no `interact_tag` chains. | Decided | F4, F6. |
| D-UO5 | The Command Center cast are new clone templates 63800-63803. The stock rows 51/59/30/64 are never touched. | Decided | Siblings bind the stock templates elsewhere (Jackson, Bra'tac). |
| D-UO6 | Omega uses its own dialog set 63800 with maps 63800-63803, not stock 1477/1573. | Decided | F13: the stock join is inferred, and its flags are 0. |
| D-UO7 | Hub respawner **180** goes on the gate plaza. | Decided | F11. |
| D-UO8 | Vendor stock and prices: **option B, every tier.** Each faction role gets one template per tier (T1-T4) on its shipped tier moniker, with its own list. | **Decided** (owner, 2026-10-19) | It is an economy call. UO-06 proposed the stock rule; D-UO29 sets the price formula. W8 builds it (UO-11, UO-12, UO-27..UO-30). |
| D-UO9 | Thor stands in the Command Center as a non-interactive clone (63803). | Decided (revised 2026-10-18) | F7: Thor speaks in 5025. His Asgard role (1522 S4773) stays in UO-16. |
| D-UO10 | A re-offer path lets a player who abandoned 1423/1500 take it again from the same NPC (CIMMERIA_RECONSTRUCTION). | Decided | Otherwise an abandon soft-locks the route. |
| D-UO11 | The letter at departure is the same decision as D-US11: **the SGC grants letter 6786 to Humans only, and Omega adds no `remove_item`.** | **Decided** (owner, 2026-10-19, the default) | Omega works either way, and the letter stays in the player's bag as a keepsake. UO-07 needs no change. |
| D-UO12 | Ambient life is CIMMERIA_AUTHORING placement of shipped bodies and monikers, in its own droppable packet, capped at 24 spawns. | Decided; the cap of 24 is set by D-UO26 (architect, the owner may overrule) | Faithful extension, removable in one revert (F26). |
| D-UO13 | This renumbering supersedes UO-00..UO-08 of 2026-10-14. | Decided | § Renumbering. |
| D-UO14 | The hub banker and the two registrars are clones of the debug-hub rows 370, 330 and 331 (CIMMERIA_RECONSTRUCTION), on the shipped Omega monikers. | Decided | F14, F24. No new code: the existing banker and registrar arms answer the click. |
| D-UO15 | The concept art is layout intent only. UO-01 tags every placement key with its art area (`common_area`, `sgc`, `free_jaffa`, `council`, `world_80`) and lists each disagreement in its worknote's § Art conflicts. The art never moves a coordinate, and a disagreement goes to the orchestrator, never to a coder. | Decided | F27, F34: the art is schematic and undated. The navmesh stays the coordinate source. |
| D-UO16 | Vendor roster: **option B.** The moniker-led vendors stand in their faction's art area, with extra spawns of the same templates at the art's other spots of that kind. `svc.consumables` and `svc.common_crafting` are re-picked in the Common Area. | **Decided** (owner, 2026-10-19) | F29. D-UO28 places the tiers; UO-17..UO-20 seal the points. |
| D-UO17 | Trainers: **(c)**, class trainers (clones of template 301 on monikers 7988/7989/7992/7993/7990) plus crafting trainers. | **Decided** (owner, 2026-10-19) | F30, F36, F37. D-UO30 gives the class trainers list 1; D-UO31 makes the crafting trainers guides, since no crafting-trainer mechanic exists. W9 builds both (UO-31, UO-35). |
| D-UO18 | The Council Chamber: **(b)**, the venue for the Asgard remote hearing. The three combat instances stay out of scope. | **Decided** (owner, 2026-10-19) | F32. UO-16 seals the hearing; UO-41/UO-42 build it after D-UO34. |
| D-UO19 | Inn, Infirmary, Battlemaster and Auction House: **(c)**, authored service NPCs wherever an engine service exists. | **Decided** (owner, 2026-10-19) | F31, F38-F40. The auction house (Black Market) and the infirmary (heal plus respawner) have an engine service; the inn and the battlemaster have none (D-UO32). |
| D-UO20 | The art's hub "Command Center" is read as the region-14 beam. The cast stays in world 80. | **Accepted** (owner, 2026-10-19) | F33: the shipped mission, dialog and ring rows all put the Command Center in world 80. |

D-UO21..D-UO26 are in § Overnight autonomous run 2026-10-18. D-UO27..D-UO38 follow.

## Owner decisions 2026-10-19

| Item | Owner's answer | Where it lands |
|---|---|---|
| D-UO8 | B, every tier | W8: UO-11, UO-12, UO-27..UO-30 |
| D-UO16 | B: the moniker-led vendors plus extra spawns at the art's spots; `svc.consumables` and `svc.common_crafting` re-picked in the Common Area | UO-17..UO-19 seal the points; W8 seeds them |
| D-UO17 | (c): class trainers (clones of template 301) plus crafting trainers | UO-31 (class), UO-35 (crafting guides, D-UO31) |
| D-UO18 | (b): the Council Chamber is the venue for the Asgard remote hearing | UO-16 seal; UO-41/UO-42 |
| U-D08 / U-D11 | A remote hearing plus a databank prologue that issues item 3399 | UO-16 seal; UO-41..UO-49 |
| D-UO19 | (c): authored service NPCs wherever an engine service exists | UO-32 (auction), UO-33 + UO-34 (infirmary), UO-36 (inn and battlemaster, D-UO32) |
| D-UO11 / D-US11 | The default: letter 6786 for Humans only, no `remove_item` | nothing to build |
| D-UO20, D-UO25, D-UO26 | Accepted | nothing to build |
| U-D07 (rest) | Authored content rewards: XP 50 × level × factor, naquadah 10 × level × factor; side activities at 50 % | the Asgard missions (UO-16 seal, W10). Hub services give no reward (D-UO37). |
| H-LBL | No visible label. Provenance lives in SQL comments only. | UO-15 and every authored row |
| D-UO32 | **Decided (owner, 2026-10-19): (a).** Build nothing for the inn and the battlemaster. | UO-36 skipped |
| D-UO33 | **Decided (owner, 2026-10-19):** 6964 `Castle Medic`. | UO-33 |
| UO-98 | **Accepted (owner, 2026-10-19):** an additional status-doc change, beyond the close-out's one. | UO-98 |
| D-UO34 | **Still deferred** (BlockedDecision). Default per UO-38 ruling R4 and UO-16 ruling R1: mission 1522 and Otin stay deferred; 63838-63839 stay reserved and unsealed. | UO-41, UO-42 |

Decisions the architect made to slice the work. The owner may overrule each one; the cost is in the last column.

| ID | Decision | Status | Reason | Cost if overruled |
|---|---|---|---|---|
| D-UO27 | **The re-carve.** § ID map replaces the old template, spawn and list carve. The old `item_list_items` rule (`× 100`) becomes `× 50`, so 29 lists of up to 20 rows fit under the 639999 floor. | Decided (architect) | F43: the old map holds one tier per role. No Omega vendor row has merged, so nothing moves. | none until W8 merges |
| D-UO28 | **Tier placement.** Each faction's T1-T4 stand together in its own wing. The Common Area and the pods carry one extra spawn of a weapon or armor tier: T1 at the plain Weapon or Armor spot, T4 at the Adv. Weapon or Adv. Armor spot. The art's General spots stay unserved (no General moniker ships), and there is no T5 (no T5 moniker ships). | Decided (architect, owner may overrule) | D-UO16 B: "extra spawns of the same templates". The art draws more spots than the client names roles. | a placement-only follow-up packet |
| D-UO29 | **Price rule.** Weapons 200 × tc, armor 100 × tc, crafting 25 × tc (UO-06 Q2), where tc is the item's `tech_comp`. Consumables keep 25/50/100 at T1 (Slappack, Antidote, Mark III Stimpack) and 25 × tc above. Heavy weapons (UO-06 Q4) are stocked only at their own tier, and only at tier 2 or higher. Never stock sell-list-2 designs 21, 55 or 3437, and stock only `flags & 1024` designs. | Decided (architect, owner may overrule) | One formula across every tier keeps the economy checkable by a test. | re-price the lists in one seed packet |
| D-UO30 | **Class trainers use trainer list 1.** It already offers each player its own archetype's tree (F36), so the five templates differ only in body, moniker and place. | Decided (architect, owner may overrule) | No per-class list exists, and splitting list 1 is new authored data. The list is named "Debug ability list"; the name is internal and never shown. | a seed packet with per-class lists |
| D-UO31 | **Crafting trainers are guides.** No crafting-trainer mechanic exists (F37). Each of the eight art spots gets a guide (moniker 7952 `SGC Scientist`, interaction 128) whose click runs an `npc_bark` chain naming the vendors of that specialty. Mapping (inferred): Biology → Biomedical, Electronics → Electronics, Engineering → Power Systems, Physics → Materials. | Decided (architect, owner may overrule) | The owner asked for crafting trainers; this is the nearest thing the engine serves with first-press feedback and no code. | drop UO-35, or a code campaign for a crafting mechanic |
| D-UO32 | **Inn and battlemaster.** No engine service and no moniker exists (F39). Recommend **BlockedDependency**: build nothing until a rest or PvP-queue service exists. UO-36 is written for the alternative, a bark-only placeholder on moniker 8360 `Airman` that says what the building is. | **Decided** (owner, 2026-10-19): (a), UO-36 skipped | D-UO19 (c) says "wherever an engine service exists", which excludes both. A placeholder could look like a broken service. | - |
| D-UO33 | **Medic name.** Default moniker 6964 `Castle Medic` (an existing, shipped name). Alternatives: 7000 `Nurse Collins`, 7952 `SGC Scientist`. | **Decided** (owner, 2026-10-19): 6964 | F41. | change one `name_id` |
| D-UO34 | **Mission 1522 and Otin.** After the UO-16 seal: is 1522 sealed (left as shipped, unreachable) or replaced by the hearing, and does Otin (28544) speak at the hearing? | **BlockedDecision** (owner), asked after UO-16 merges | F42. The seal lists the options with evidence. | - |
| D-UO35 | **Service interaction bits.** Medic, inn and battlemaster use `NON_A_STORY_MISSION_AVAILABLE` (134217728), as template 390 does; guides use `TRAINER` (128), as template 1300 does; the auctioneer keeps `AUCTION` (4). UO-U1 checks the icon each one shows. | Decided (architect, owner may overrule) | These bits make the client show a use prompt; the chain answers the click. | change one column |
| D-UO36 | **The Auction House is the Black Market auctioneer** (a spawn of stock template 305). The Black Market window needs the Cimmeria client patch; without it the chat line is the first-press feedback. | Decided (architect, owner may overrule) | F38: the only auction service the engine has. | drop UO-32 |
| D-UO37 | **Bodies and rewards.** Every service template copies template 1302's row and column list. Human vendors, trainers, guides, the medic, the inn and the battlemaster keep 1302's body without `,WP-Human.WP_SMG_1A`; only the Human Weapons vendors keep the SMG. Jaffa templates take template 100's `body_set` and `components` with `skin_tint` 0. Asgard templates take template 64's `body_set` and `components` with `skin_tint` 0. Trainers take 301's trainer columns: `interaction_type` 128, `trainer_ability_list_id` 1, all four item lists NULL. Hub services give no XP, items or naquadah. | Decided (architect, owner may overrule; body rule refined 2026-10-19) | One body rule keeps 40-odd templates reviewable. U-D07's numbers apply to quests, not to services. | a seed-only follow-up |
| D-UO38 | **Asgard Programs.** No vendor bit for programs exists, so the four Asgard Programs vendors (27252-27255) carry `INT_VENDOR_WEAPONS` (16384) and the weapons price, 200 × tc. Asgard Armor carries `INT_VENDOR_ARMOR` (8192) at 100 × tc. The four Asgard crafting vendors carry their specialty's craft bit and share lists 63817-63820. UO-38 defines which items count as Asgard programs and armor (they have no visual component) and records the rule. Asgard armor is the 369 designs named `<Fyrstyr\|Annarr\|Pridi\|Fjordi\|Fimm> <Absorption\|Phasic\|Regeneration> <slot>` (the client's `DN_Armor_Asgard_*` names); Asgard programs are the 205 `DN_Weap_Program_*` designs (UO-38 ruling R1, R2). | Decided (architect, owner may overrule) | The weapons bit is the nearest store type the client already draws; programs are the Asgard's weapons. | change four `interaction_type` values and re-price four lists |

## ID map (D-UO1)

Re-carved 2026-10-19 (D-UO27). Merged rows are marked (merged); every other row is reserved for the packet named.

| Table | IDs | Use |
|---|---|---|
| `content_chains` | 63801 | beam (merged) |
| | 63810-63811 | Modi (merged) |
| | 63820-63822 | Human (merged) |
| | 63830-63832 | Free Jaffa (merged) |
| | 63840-63845 | re-offer (merged; 63846-63849 spare) |
| | 63850 | auctioneer `UnionOmega_Auctioneer` (UO-32) |
| | 63851 | medic `UnionOmega_Medic` (UO-33) |
| | 63852-63855 | guides Biology, Electronics, Engineering, Physics (UO-35) |
| | 63856-63857 | inn, battlemaster (UO-36, D-UO32) |
| | 63858-63869 | services spare |
| | 63870-63949 | spare (authored) |
| | 63950-63999 | Asgard (UO-41..UO-49, after the UO-16 seal) |
| `entity_templates` | 63800-63803 | Jackson (clone of 51), Bra'tac (59), Teal'c (30), Thor (64) (merged) |
| | 63804-63808 | class trainers Soldier 7988, Scientist 7989, Commando 7992, Archaeologist 7993, Jaffa 7990 (UO-31) |
| | 63809 | Asgard trainer 27218 (UO-39) |
| | 63810, 63811 | Banker (clone of 370), Team registrar (clone of 330) (merged) |
| | 63812-63815 | Human Weapons T1-T4, monikers 27240-27243 (UO-11) |
| | 63816-63819 | Human Armor T1-T4, 27244-27247 (UO-27) |
| | 63820-63823 | Human Crafting Biomedical, Electronics, Power Systems, Materials, 27248-27251 (UO-29) |
| | 63824-63827 | Common Crafting, same order, 27236-27239 (UO-29) |
| | 63828 | Consumables 27264 (UO-30) |
| | 63829 | crafting guide 7952 (UO-35) |
| | 63830 | Command registrar (clone of 331) (merged) |
| | 63831-63834 | Jaffa Weapons T1-T4, 27224-27227 (UO-12) |
| | 63835-63838 | Jaffa Armor T1-T4, 27228-27231 (UO-28) |
| | 63839-63842 | Jaffa Crafting, same order as Human, 27232-27235 (UO-29) |
| | 63843-63846 | Asgard Programs T1-T4, 27252-27255 (UO-39) |
| | 63847-63850 | Asgard Armor T1-T4, 27256-27259 (UO-40) |
| | 63851-63854 | Asgard Crafting, 27260-27263 (UO-40) |
| | 63855 | medic 6964 (UO-33, D-UO33) |
| | 63856, 63857 | inn, battlemaster (UO-36, D-UO32) |
| | 63858, 63859 | Thor (Hologram) 28128, High Councillor Otin 28544 (UO-42, D-UO34) |
| | 63860-63861 | ambient Airman (clone of 31), Free Jaffa Warrior (clone of 34, name 26723) (merged) |
| | 63862 | Thor (Hologram) 28128, hub 481 offer (UO-42, UO-16 ruling R1) |
| | 63863-63899 | spare |
| `spawnlist` | 63800-63803 | cast (merged) |
| | 63804-63808 | class trainers (UO-31) |
| | 63809 | Asgard trainer (UO-39) |
| | 63810-63817 | crafting guides (UO-35): six in the Common Area, Adv. Physics and Adv. Biology in the SGC wing |
| | 63818 | medic (UO-33) |
| | 63819 | auctioneer, stock template 305, tag `UnionOmega_Auctioneer` (UO-32) |
| | 63820, 63821 | banker, Team registrar (merged) |
| | 63822, 63823 | inn, battlemaster (UO-36) |
| | 63824-63835 | Asgard vendors (UO-39, UO-40) |
| | 63836-63839 | Council cast (UO-42): 63836 Thor, 63837 Otin (D-UO34); 63838-63839 reserved, unsealed (UO-38 ruling R4) |
| | 63840 | Command registrar (merged) |
| | 63841-63846 | Human Weapons: T1-T4 in the SGC wing, T1 at the east pod Weapon spot, T4 at the NW pod Adv. Weapon spot (UO-11) |
| | 63847-63852 | Human Armor: the same pattern at the east pod Armor and NW pod Adv. Armor spots (UO-27) |
| | 63853-63857 | Jaffa Weapons: T1-T4 in the Free Jaffa wing, T1 at the Common Area Weapon spot (UO-12); 63858 spare |
| | 63859-63863 | Jaffa Armor: the same pattern, T1 at the Common Area Armor spot (UO-28); 63864 spare |
| | 63865-63872 | Consumables: Common Area 3, SGC wing 3, Free Jaffa wing 2 (UO-30) |
| | 63873 | Asgard hub Thor, world 18, tag `UnionOmega_AsgardThor` (UO-42, UO-16 ruling R1) |
| | 63874-63879 | spare |
| | 63880-63901 | ambient (merged, 22 spawns); **63880-63959 may hold ambient templates only** (F44) |
| | 63960-63971 | crafting vendors: Human 4 in the SGC wing, Common 4 in the Common Area, Jaffa 4 in the Free Jaffa wing (UO-29) |
| `respawners` | 180 | `Omega Site Gate Plaza` (merged) |
| | 181 | `Omega Infirmary`, world 18 (UO-34) |
| `dialog_sets` | 63800 | `Omega Command Center offers` (merged) |
| `dialog_set_maps` | 63800 | Jackson delivery: dialog 4641, `The SGU`, flags 16777216 |
| | 63801 | Bra'tac delivery: dialog 5025, `The SGU`, flags 16777216 |
| | 63802 | Jackson re-offer: dialog 4641, flags 8388608 |
| | 63803 | Bra'tac re-offer: dialog 5025, flags 8388608 |
| `missions` | 63800-63805 | Asgard prologue P1 63800, P2 63801, P3 63802, P3-S 63803, P4 63804, P5 63805 (UO-16 ruling R2, UO-48) |
| `mission_steps` | 63810-63829 | 11 steps, in mission then step order (UO-48) |
| `mission_objectives` | 63830-63849 | one per step, same order (UO-48) |
| `mission_tasks` | 63850-63869 | one per objective, same order (UO-48) |
| `dialogs` | 63900 | `Union Omega authored barks (never displayed)`, the holder of the bark screens (UO-24) |
| `dialog_screens` | 63901-63904 | guide lines Biology, Electronics, Engineering, Physics (UO-35) |
| | 63905 | medic line (UO-33) |
| | 63906, 63907 | inn, battlemaster lines (UO-36) |
| `item_lists` | 63800-63803 | Human Weapons T1-T4 |
| | 63804-63807 | Human Armor T1-T4 |
| | 63808-63811 | Jaffa Weapons T1-T4 |
| | 63812-63815 | Jaffa Armor T1-T4 |
| | 63816 | Consumables |
| | 63817-63820 | crafting Biomedical, Electronics, Power Systems, Materials; shared by every crafting template of that specialty, Asgard included |
| | 63821-63824 | Asgard Programs T1-T4 |
| | 63825-63828 | Asgard Armor T1-T4 |
| `item_list_items` | 638000-639419 | row = 638000 + (list − 63800) × 50 + k, k = 0..19 (at most 20 rows per list) |

Flags are 16777216 = `INT_A_STORY_MISSION_ACTIVE` and 8388608 = `INT_A_STORY_MISSION_AVAILABLE` (`crates/entity/src/interaction_flags.rs:68-70`). All maps use `min_level 1` and `'{}'` arrays. Template and spawn ids may repeat across tables; each table has its own key. Stock dialog screens 63847-63887 exist; the bark screens 63901-63907 do not collide (F43).

## Contracts

### Receiving: C4 from SGC (US-32), and the Dakara legs

| Field | Value |
|---|---|
| Departure | SGC US-32: `advance_step 1655 '4941'`, `advance_step 1655 '4940'`, `cross_world_teleport 'Omega_Site'` |
| State on arrival | 1655 active; S4943, S4944 and S4941 completed (O6055 completed by SGC's `advance_step 1655 '4940'`); S4940 active with O5644, O5645, O5646 and O5647 open (D-UO25) |
| Arrival point | World 18: x 34.70, y -51.28, z -142.00, heading 3.1409 (faces gate 5), on the gate plaza in front of gate 5 (sealed by UO-01, `nav_inspect` exit 0; [worknote § C4](omega-packets/worknotes/UO-01-placements.md#c4)) |
| Archetypes | 1-4 (Human, through the SGC) or 7 (Free Jaffa, through Dakara 1654 and then the SGC) |
| Letter | None required. D-UO11 = D-US11. |
| Omega's first event | `player_loaded 'Omega_Site'`, which binds Modi (chain 63810) |
| SGC must not | run `complete_objective` on 1655 or `advance_step 1655 '4942'`, or complete 1655 (F21, D-UO25): S4940 onward is Omega's |
| Abandon of 1655 | SGC's re-offer owns it; Omega has no 1655 re-offer |
| Step ownership | **Decided (D-UO25, U-D04):** Omega receives at S4940 with O6055 done, and owns S4940 → S4942 (63801) and the S4942 completion on the Accept (UO-07, UO-08). SGC owns S4943, S4944, S4941 and the C4 departure. |

### Producing: to Beta (UB receivers 23020/23220/23310)

| Route | State Omega leaves | Gate | Beta's first event |
|---|---|---|---|
| Human | 1655 completed; 1423 active at S4223 | gate 6 address granted | Beta 23020 on arrival in world 23 |
| Free Jaffa | 1655 completed; 1500 active at S4369 | gate 6 | Beta 23220 |
| Asgard | 481 at S1760 with item 3399 (design sealed by UO-16; built by UO-42, BlockedDependency on D-UO34 and a playable Asgard start) | gate 6 | Beta 23310 |

Omega fires no Beta step (D-UO2). Beta chain 23001 grants Omega's gate 5 to a player who reaches Beta first. A player who abandons 1423/1500 gets it back from Jackson or Bra'tac in world 80 (UO-09). UO-13 exports `pub(super)` departure-state fixtures in `chain_replay_tests/union_omega/handoff.rs`, so Beta's replay tests start from Omega's exact output. **Fixture ownership (settled with Beta):** UO-13 owns and exports the *departure* fixtures; Beta's UB-10, UB-20 and UB-30 build the *receiver* fixtures (the inbound state staged, Beta's chain asserted). Beta's wave B2 can run before UO-13 merges, so Beta stages the state itself and may switch to the `handoff` import once UO-13 is on `main`. Both sides assert the same table row above.

## Renumbering (for sibling ledgers)

| Old (2026-10-14) | New |
|---|---|
| UO-00 specialist seal | UO-01 (placements) and UO-00 (docs seal) |
| UO-01 hub travel and safety | UO-03 (respawner) and UO-04 (beam, Modi) |
| UO-02 Human induction | UO-07 |
| UO-03 Free Jaffa induction | UO-08 |
| UO-04/05/06 Asgard | UO-16 (design seal), then UO-37..UO-42 (W10); the hand-off to Beta 23310 is **UO-42** |
| UO-07/08 hub life and integration | UO-10..12 (services), UO-13 (handshake and Beta fixtures), UO-14 (ambient) |

Beta's old "UO-07/08 receiver fixtures" citation now means the departure fixtures of **UO-13**; the receiver fixtures stay with Beta (UB-10/UB-20/UB-30). The Asgard route ledger's "Omega UO-05/06" is now **UO-16** (design) and **UO-42** (the departure seeds and the `asgard_departure` fixture).

**2026-10-19 re-slice.** UO-11 and UO-12 keep their ids but now cover one role in four tiers each (Human Weapons, Jaffa Weapons). UO-15 is the authored copy for the service barks; UO-16 is the Asgard design seal. New ids UO-17..UO-42 follow § Packets; UO-43..UO-49 are reserved for the Asgard prologue packets the UO-16 seal slices; UO-98 is the second close-out. No old id is reused for different work.

## Packets

| ID | Wave | Role | Size | Scope | Status | File |
|---|---|---|---|---|---|---|
| UO-01 | W1 | `movement-teleport-advisor` | ~35k | Nav-probe every coordinate, tag each with its concept-art area, then write the placement worknote | Merged (#1890) | [UO-01](omega-packets/UO-01.md) |
| UO-02 | W1 | `packet-coder` | ~40k | Scaffold: empty seed files, includes, test modules and a visitor guard | Merged (#1890) | [UO-02](omega-packets/UO-02.md) |
| UO-00 | W1 | `documentation-writer` | ~30k | ID row, C4 arrival point, sibling handoff note | Merged (#1902) | [UO-00](omega-packets/UO-00.md) |
| UO-03 | W2 | `packet-coder` | ~30k | Respawner 180 and the hub placement test | Merged (#1902) | [UO-03](omega-packets/UO-03.md) |
| UO-04 | W2 | `packet-coder` | ~40k | Beam 63801 and Modi 63810/63811 | Merged (#1902) | [UO-04](omega-packets/UO-04.md) |
| UO-05 | W2 | `packet-coder` | ~40k | Cast templates and spawns 63800-63803, dialog set and maps | Merged (#1902) | [UO-05](omega-packets/UO-05.md) |
| UO-06 | W2 | `items-systems-advisor` | ~35k | Services seal: the stock proposal for D-UO8 and the roster options for D-UO16 | Merged (#1902); a worknote, D-UO8 and D-UO16 stay open | [UO-06](omega-packets/UO-06.md) |
| UO-10 | W2 | `packet-coder` | ~35k | Banker 63810 and registrars 63811 (Team), 63830 (Command) | Merged (#1902) | [UO-10](omega-packets/UO-10.md) |
| UO-07 | W3 | `packet-coder` | ~40k | Human induction 63820-63822 | Merged (#1908) | [UO-07](omega-packets/UO-07.md) |
| UO-08 | W3 | `packet-coder` | ~40k | Free Jaffa induction 63830-63832 | Merged (#1908) | [UO-08](omega-packets/UO-08.md) |
| UO-09 | W3 | `packet-coder` | ~35k | Re-offer 63840-63845 | Merged (#1911) | [UO-09](omega-packets/UO-09.md) |
| UO-14 | W3 | `packet-coder` | ~35k | Ambient airmen and Free Jaffa warriors (droppable) | Merged (#1911); 22 spawns, cap 24 (D-UO26) | [UO-14](omega-packets/UO-14.md) |
| UO-13 | W4 | `rust-gameserver-dev` (Sonnet) | ~50k | Executed handshake tests and Beta fixtures | Done in the close-out PR (reviewed, ship) | [UO-13](omega-packets/UO-13.md) |
| UO-U1 | W5, W11 | the owner, in the lab (a Sonnet lead may grade the owner's logs afterwards) | 4 runs, then R5 | Client walk of both routes, re-offer, services; § R5 adds the W8-W9 services | UATPending, not run; **the owner runs it** (agents never use the lab) | [UO-U1](omega-packets/UO-U1.md) |
| UO-99 | W5 | `documentation-writer` | ~35k | Close-out: chains, zone audit, UAT, status (once) | Done in the close-out PR, overnight form (no UO-U1 results, no vendors) | [UO-99](omega-packets/UO-99.md) |
| UO-15 | W6 | `documentation-writer` | ~25k | Authored copy: the bark lines 63901-63907 and their speaker names, no label (H-LBL) | Ready | [UO-15](omega-packets/UO-15.md) |
| UO-16 | W6 | `mission-systems-advisor` | ~40k | Asgard design seal: the Council hearing, the databank prologue (item 3399), 481 and gate 6, options for D-UO34 | Ready | [UO-16](omega-packets/UO-16.md) |
| UO-17 | W6 | `movement-teleport-advisor` | ~35k | Placement seal, SGC wing (21 points) | Ready | [UO-17](omega-packets/UO-17.md) |
| UO-18 | W6 | `movement-teleport-advisor` | ~30k | Placement seal, Free Jaffa wing (15 points) | Ready | [UO-18](omega-packets/UO-18.md) |
| UO-19 | W6 | `movement-teleport-advisor` | ~30k | Placement seal, Common Area vendors and auctioneer (14 points) | Ready | [UO-19](omega-packets/UO-19.md) |
| UO-20 | W6 | `movement-teleport-advisor` | ~30k | Placement seal, Common Area buildings: medic, respawner 181, inn, battlemaster, six guides | Ready; starts after UO-19 returns | [UO-20](omega-packets/UO-20.md) |
| UO-21 | W7 | `items-systems-advisor` | ~35k | Stock seal: Human Weapons and Armor lists 63800-63807 | Ready (needs no W6 output) | [UO-21](omega-packets/UO-21.md) |
| UO-22 | W7 | `items-systems-advisor` | ~35k | Stock seal: Jaffa Weapons and Armor lists 63808-63815 | Ready (needs no W6 output) | [UO-22](omega-packets/UO-22.md) |
| UO-23 | W7 | `items-systems-advisor` | ~30k | Stock seal: Consumables 63816, crafting 63817-63820 | Ready (needs no W6 output) | [UO-23](omega-packets/UO-23.md) |
| UO-24 | W7 | `packet-coder` | ~40k | Scaffold: seed markers, two new seed files, includes, linter lines, the `interact_tag` guard amendment, stub test files | Ready | [UO-24](omega-packets/UO-24.md) |
| UO-25 | W7 | `packet-coder` | ~30k | cell-catalog vendor test helper `live_db_union_omega_vendor_common.rs` | after UO-24 | [UO-25](omega-packets/UO-25.md) |
| UO-26 | W7 | `packet-coder` | ~30k | chain-replay services helper `union_omega/services_common.rs` | after UO-24 | [UO-26](omega-packets/UO-26.md) |
| UO-11 | W8 | `packet-coder` | ~40k | Human Weapons T1-T4: templates 63812-63815, lists 63800-63803, spawns 63841-63846 | BlockedDependency (W6, W7) | [UO-11](omega-packets/UO-11.md) |
| UO-12 | W8 | `packet-coder` | ~40k | Jaffa Weapons T1-T4: templates 63831-63834, lists 63808-63811, spawns 63853-63857 | BlockedDependency (W6, W7) | [UO-12](omega-packets/UO-12.md) |
| UO-27 | W8 | `packet-coder` | ~40k | Human Armor T1-T4: templates 63816-63819, lists 63804-63807, spawns 63847-63852 | BlockedDependency (W6, W7) | [UO-27](omega-packets/UO-27.md) |
| UO-28 | W8 | `packet-coder` | ~40k | Jaffa Armor T1-T4: templates 63835-63838, lists 63812-63815, spawns 63859-63863 | BlockedDependency (W6, W7) | [UO-28](omega-packets/UO-28.md) |
| UO-29 | W8 | `packet-coder` | ~40k | Crafting: 12 templates (Human, Common, Jaffa × four specialties), lists 63817-63820, spawns 63960-63971 | BlockedDependency (W6, W7) | [UO-29](omega-packets/UO-29.md) |
| UO-30 | W8 | `packet-coder` | ~30k | Consumables: template 63828, list 63816, spawns 63865-63872 | BlockedDependency (W6, W7) | [UO-30](omega-packets/UO-30.md) |
| UO-31 | W9 | `packet-coder` | ~35k | Class trainers 63804-63808 on trainer list 1 | BlockedDependency (W6, W7; merges after W8) | [UO-31](omega-packets/UO-31.md) |
| UO-32 | W9 | `packet-coder` | ~35k | Auctioneer: spawn 63819, chain 63850 `open_black_market` | BlockedDependency (W6, W7; merges after W8) | [UO-32](omega-packets/UO-32.md) |
| UO-33 | W9 | `packet-coder` | ~40k | Medic: template 63855, spawn 63818, chain 63851 (heal + bark 63905) | BlockedDependency (W6, W7; merges after W8); name 6964 (D-UO33) | [UO-33](omega-packets/UO-33.md) |
| UO-34 | W9 | `packet-coder` | ~30k | Infirmary respawner 181 and its death-position test | BlockedDependency (W6, W7; merges after W8) | [UO-34](omega-packets/UO-34.md) |
| UO-35 | W9 | `packet-coder` | ~40k | Crafting guides: template 63829, spawns 63810-63817, chains 63852-63855, screens 63901-63904 | BlockedDependency (W6, W7; merges after W8) | [UO-35](omega-packets/UO-35.md) |
| UO-36 | W9 | `packet-coder` | ~35k | Inn and battlemaster placeholders: templates 63856/63857, spawns 63822/63823, chains 63856/63857, screens 63906/63907 | Skipped (D-UO32 (a): build nothing) | [UO-36](omega-packets/UO-36.md) |
| UO-37 | W10 | `movement-teleport-advisor` | ~30k | Asgard placement seal: trainer, 12 vendors, hub Thor, Council cast | BlockedDependency (UO-16) | [UO-37](omega-packets/UO-37.md) |
| UO-38 | W10 | `items-systems-advisor` | ~30k | Asgard stock seal: lists 63821-63828 | BlockedDependency (UO-16) | [UO-38](omega-packets/UO-38.md) |
| UO-39 | W10 | `packet-coder` | ~40k | Asgard trainer 63809 and Programs vendors 63843-63846 | BlockedDependency (UO-37, UO-38) | [UO-39](omega-packets/UO-39.md) |
| UO-40 | W10 | `packet-coder` | ~40k | Asgard Armor and Crafting vendors 63847-63854 | BlockedDependency (UO-39's commit; W8 merged for lists 63817-63820) | [UO-40](omega-packets/UO-40.md) |
| UO-41 | W10 | stub | - | The hearing chains (63950-63999), sliced by the UO-16 seal | BlockedDependency (D-UO34, a playable Asgard start) | [UO-41](omega-packets/UO-41.md) |
| UO-42 | W10 | stub | - | Thor and Otin templates, 481 offer, gate 6, the `asgard_departure` fixture | BlockedDependency (D-UO34, a playable Asgard start) | [UO-42](omega-packets/UO-42.md) |
| UO-98 | W11 | `documentation-writer` | ~35k | Close-out 2: chains, zone audit, UAT checklist, status docs (once) | BlockedDependency (W8, W9 merged) | [UO-98](omega-packets/UO-98.md) |

## Waves

| Wave | Packets | Review | Gate |
|---|---|---|---|
| W1 (merged #1890, #1902) | UO-01, UO-02, then UO-00 | one review-fixer for UO-02 and UO-00; UO-01 is a worknote the orchestrator checks for every key | none (U-D03 lifted 2026-10-18) |
| W2 (merged #1902) | UO-03, UO-04, UO-05, UO-06, UO-10 | one review-fixer (UO-06 is a worknote) | W1 merged |
| W3 (merged #1908, #1911) | UO-07, UO-08, UO-09, UO-14 | one review-fixer | W2 merged; UO-09 after UO-07/08 |
| W4 (UO-13 done) | UO-13 (UO-11/UO-12 moved to W8 by the 2026-10-19 re-slice) | one review-fixer | W3 merged |
| W5 (UO-99 done; UO-U1 not run) | UO-U1, then UO-99 | UO-99 gets one documentation review | W4 merged; the owner runs UO-U1. UO-99 ran first, in its overnight form |

Waves W6-W11 (2026-10-19) follow the current review loops (the `campaign-packet` skill § 4): a packet loop per packet, then one wave loop per wave. Each wave ships as **one PR from an integration worktree** `uo-w<N>`: `ops` merges each packet branch into it with `merge-noff.ps1` in the merge order below, the wave loop reviews the integrated tree, and one PR ships it. Retire the packet worktrees with `rm-worktree.ps1 --merged-into <PR> <name>...` once the wave PR merges.

| Wave | Packets, in merge order | Gate | Notes |
|---|---|---|---|
| W6, seals A (docs) | UO-15, UO-16, UO-17, UO-18, UO-19, UO-20 | none; Ready now | Worknotes only. UO-20 starts after UO-19 returns (it reads UO-19's points for the 2.5 m spacing). |
| W7, stock seals and scaffold | UO-24, UO-25, UO-26, UO-21, UO-22, UO-23 | none; Ready now, may run beside W6 | UO-25 and UO-26 branch from UO-24's commit (`ops` branches at that SHA). The seals need no W6 output. |
| W8, vendors | UO-11, UO-12, UO-27, UO-28, UO-29, UO-30 | W6 and W7 merged | Each reads its rows from the UO-17..UO-19 placement worknotes and the UO-21..UO-23 stock worknotes. |
| W9, services | UO-31, UO-32, UO-33, UO-34, UO-35, then UO-36 if D-UO32 says build | W6 and W7 merged; ships after the W8 PR merges | May code beside W8; the integration branch rebases on `main` after W8 lands. UO-33 uses moniker 6964 unless D-UO33 is answered first. |
| W10, Asgard | UO-37, UO-38, UO-39, UO-40; UO-41, UO-42 are stubs | UO-37 and UO-38: the UO-16 seal merged. `uo-w10` is cut from `main` after the W8 PR merges (UO-40 needs lists 63817-63820); UO-39 branches from `uo-w10` once UO-37 and UO-38 are in it, and UO-40 branches at UO-39's commit (`ops` branches at that SHA). UO-41/UO-42 also wait on D-UO34 and a playable Asgard start | UO-24 made no Asgard test stub, so UO-39 adds two `mod` lines (the only W8-W10 `mod.rs` edit). UO-43..UO-49 are reserved for the prologue packets the UO-16 seal slices. |
| W11, close | UO-U1 § R5 (the owner's lab run), then UO-98 | W8 and W9 merged | UO-98 is close-out 2: the reopened phase's one status-doc change, plus UO-99's deferred rows. |

The U-D04 and U-D07 merge holds were lifted (D-UO25, D-UO26, decided by the architect overnight 2026-10-18 and accepted by the owner on 2026-10-19), and UO-04, UO-07, UO-08 and UO-14 merged on the normal gate. If the owner overrules D-UO26 later, revert UO-14's one commit (or trim spawns to the new cap).

Parallel coders never edit the same lines. UO-02 and, for W8-W10, UO-24 create every shared file with one `-- [UO-xx] begin` / `-- [UO-xx] end` marker pair per packet, and each packet writes only between its own pair. UO-24 also adds every `mod` line and a stub file for each new W8-W9 test file, so no W8-W9 packet edits a `mod.rs` or `tests/mod.rs`. The one exception is UO-39, which adds the two Asgard test modules (W10 runs after W8 and W9, so nothing races it). The two shared test helpers (UO-25, UO-26) are written once, before any vendor or service packet.

**Two cross-file edits to watch.** UO-24 adds one `||` line after `interact_tag_linter/allowlist.rs:217` (Beta's `beta_e1` line); if another zone's packet (the live SGC `us-*` work, for one) adds its own `||` line there first, keep every line on rebase. UO-32 amends the expected vector in `live_db_debug_auctioneer.rs:161-174`, which asserts where template 305 stands; nothing else may touch those lines.

Every packet inlines its rules, the chain contract (chain packets) and its lane commands. Packet-coder skips CLAUDE.md, so nothing a packet needs lives only here.

## HELD and blocked

| Item | Exact question | Who answers | Blocks |
|---|---|---|---|
| U-D04 | **Decided (D-UO25)** by the architect overnight 2026-10-18: yes to both; Omega receives at S4940 with O6055 done and owns S4940 → S4942 and the S4942 completion. The owner may overrule. | architect (owner may overrule) | nothing |
| **D-UO32** | **Decided (a), 2026-10-19.** The inn and the battlemaster have no engine service and no moniker (F39). **(a)** BlockedDependency: build nothing until a rest or PvP-queue service exists (recommended). **(b)** A bark-only placeholder each (UO-36: moniker 8360 `Airman`, one authored line saying what the building is, no service). | owner | UO-36 |
| **D-UO33** | **Decided: 6964.** Which shipped name does the Omega medic wear: 6964 `Castle Medic` (default), 7000 `Nurse Collins` or 7952 `SGC Scientist`? | owner | UO-33 (uses 6964 if unanswered when W9 starts) |
| **D-UO34** | **Open, asked after UO-16 merges.** Is mission 1522 sealed (left as shipped and unreachable) or replaced by the hearing, and does High Councillor Otin (28544) speak at the hearing? | owner | UO-41, UO-42 |
| U-D07 (rest) | **Decided 2026-10-19:** authored copy is allowed; authored missions pay XP 50 × level × factor and naquadah 10 × level × factor, side activities 50 %. Services pay nothing (D-UO37). | owner (answered) | nothing |
| H-LBL | **Decided 2026-10-19:** no visible label; provenance in SQL comments only. | owner (answered) | nothing |
| D-UO8 | **Decided 2026-10-19:** option B, every tier. | owner (answered) | nothing |
| D-UO11 / D-US11 | **Decided 2026-10-19:** the default, letter 6786 for Humans only, no `remove_item`. | owner (answered) | nothing |
| D-UO16 | The [concept art](omega-concept-art.md) draws General, Consumable, Weapon and Armor vendors in every area (8/8/5/5), Adv. Weapon and Adv. Armor, and no crafting vendor. Which roster does Omega get? **A:** the plan as written, eight moniker-led templates with one spawn each, standing in their faction's art area (Human in the SGC wing, Jaffa in the Free Jaffa wing, Common Crafting and Consumables in the Common Area). **B:** A, plus extra spawns of the same templates at the art's other spots of that kind; same templates and lists, new spawn ids, and the architect re-slices UO-11/12. **C:** B, plus authored General, Adv. Weapon and Adv. Armor vendors, which have no Omega moniker: new templates, names and stock, CIMMERIA_AUTHORING, scaled under U-D07. In every option: the art draws no crafting vendor, so do the three crafting templates stay, and where? | owner (answered 2026-10-19: **B**) | nothing |
| D-UO17 | The art draws six class trainers and eight crafting trainers; the plan has none. **(a)** None. **(b)** Class trainers only: Soldier, Scientist, Commando and Archaeologist in the SGC wing, Jaffa in the Free Jaffa wing, cloned from template 301 on the global monikers 7988/7989/7992/7993/7990. **(c)** Also the eight crafting trainers, which have no moniker anywhere and may need a mechanic the tree lacks. The Asgard trainer waits on UO-16. | owner (answered 2026-10-19: **c**) | nothing |
| D-UO18 | The art draws a Council Chamber with three combat instances north of the hub. **(a)** Out of scope (the plan; worlds 81/82 and the physical Council stay held). **(b)** It is the venue for the Asgard remote hearing (folds into U-D08 / UO-16). **(c)** A later instance campaign. | owner (answered 2026-10-19: **b**) | nothing |
| D-UO19 | The art draws an Inn, Infirmary, Battlemaster and Auction House in the hub, which § Rejected additions excludes for lack of a moniker. **(a)** Keep them rejected. **(b)** Location only: ambient or set dressing at those buildings, no service NPC. **(c)** Authored service NPCs wherever an engine service exists. | owner (answered 2026-10-19: **c**) | nothing; the inn and battlemaster half is D-UO32 |
| U-D08 / U-D11 | **Decided 2026-10-19:** a remote hearing in the Council Chamber plus a databank prologue that issues item 3399. The open half is D-UO34. | owner (answered) | nothing |
| U-D03 | **Lifted 2026-10-18** by the owner: the Union campaigns were held until FZ-99; Omega was built overnight. | owner (answered) | nothing |

These items are HELD and block nothing:

- **O5646 waypoints:** no data.
- **H1 DHD dial-back:** world 18 has no DHD dial-home chain.
- **H2:** whether world 80 is private.
- **H6:** gating on pad 14.
- **Gate 5:** its row sits 0.41 m under the dais.
- **1425 S4244:** the Jackson tail (D-UB5 / HELD-B4).
- **1655:** the `step_enabled` flags.
- **Otin:** moniker 28544 ships (F42); whether he speaks at the hearing is D-UO34. **Heimdall:** no evidence.
- **Worlds 81/82:** out of scope; D-UO18 (b) uses the Council Chamber only as the hearing venue, not its combat instances.
- **The art's General vendor spots:** no General moniker ships; they stay unserved (D-UO28).
- **Concept-art pod labels (F35):** the Common Area enlargement and the overview disagree on three small pods.
- **Beam names (F28):** which of the six shipped destination names belongs to which ring region.
- **UAT death row H9:** the legacy `.kill` GM command is not registered (`docs/guides/unified-uat.md:905`); UO-U1 marks H9 BLOCKED if no self-damage command exists.

## Rejected additions

Each of these was rejected for lack of evidence, or because it would break a rule. The owner's 2026-10-19 answers reopened four of them; those lines say what replaced the rejection.

- **Woolsey at Omega:** 5025 says he must not discover the operation.
- **Mission 631** on Omega.
- **A "Stargate Plaza" area** from 1513.
- **Otin** with speaker 2509 (an unnamed speaker). His moniker 28544 is a different matter: D-UO34.
- **Any content** in worlds 81/82. The Council Chamber is now the hearing venue (D-UO18 b); its combat instances stay rejected.
- **Ambient VO or barks** with no shipped line, on the ambient crowd. *Reopened for services:* U-D07 allows authored copy, so the guides, medic and (if D-UO32 says so) inn and battlemaster speak UO-15's lines.
- **A guild registrar:** the Guild Center is served by the registrars (29068, UO-10). *Reopened:* the auctioneer (UO-32, D-UO36). The inn and battlemaster are D-UO32.
- **A medic:** *reopened* (D-UO19 c); UO-33 builds one on a shipped name (D-UO33).
- **Hostile Free Jaffa Attacker (208)** as ambient.
- **Ambient patrols:** no path data; static spawns only.
- **`interact_tag` chains on the induction NPCs:** they would display the dialog twice (F4). Services may use them in chains 63850-63869 only, on NPCs with no bound dialog set (UO-24's guard).
- **`complete_objective` on 1655:** completing S4940's required O5647 and O5644 completes the mission before the Accept (F21).
- **A crafting-trainer mechanic:** none exists (F37); the guides stand in (D-UO31).

## Overnight autonomous run 2026-10-18

The owner lifted the U-D03 gate the same day, so Omega ran unattended from W1. The U-D04 and U-D07 merge holds were settled by D-UO25 and D-UO26 below. New decision ids continue after D-UO20.

| Id | Decision | Status | Why |
|---|---|---|---|
| D-UO21 | `svc.banker` keeps its navmesh coordinate (-88.06, -48.02, -102.92). UO-01's nearest-anchor rule puts it in the `sgc` area (Ring region 12 Left, 122.55 m), while the concept art draws the Bank in the Common Area (Ring region 10 Center, 125.51 m). Recorded as a known art disagreement. | Decided (D-UO15: art is layout intent, never a coordinate) | The point is the only reachable standing point beside the bank prop. A 3 m margin between two of six anchors is inside the proxy's error. No other placement row disagrees ([worknote § Art conflicts](omega-packets/worknotes/UO-01-placements.md#art-conflicts)). |
| D-UO22 | Bridges E and W (world 80) have no placement rows. UO-11, UO-12 and UO-14 must not look them up and must not seed `amb.bridge_e.*` or `amb.bridge_w.*`. | Decided | They are separate navmesh components (12 and 26) with no route from pad 17 (component 11): `nav_inspect --gap-pair 11,12` and `11,26` find none, and a probe exits 3. Seeding there needs a nav patch. |
| D-UO23 | The UO-01 C4 arrival heading is corrected to **3.1409** (it faces gate 5). UO-00 publishes it in § Contracts → Receiving. | Decided | The worknote's table row and § C4 agree on 3.1409. |
| D-UO24 | Generated Tauri schema files in a build worktree were discarded to unblock the merge. | Done | Build output only; no content or seed impact. |
| D-UO25 | **U-D04 settled.** Omega receives 1655 at S4940 with S4941/O6055 already completed. SGC's C4 chain fires `advance_step 1655 '4941'`, then `advance_step 1655 '4940'`, then `cross_world_teleport 'Omega_Site'`. `advance_step` completes the old step's open objectives, so O6055 arrives completed and O5644-O5647 arrive open. Omega owns S4940 → S4942 (chain 63801 on `teleport_in '17'`, D-UO3) and the S4942 completion: the Jackson or Bra'tac Accept completes 1655 and offers 1423 or 1500 (UO-07, UO-08). SGC owns S4943, S4944, S4941 and the departure, and never fires S4942, `complete_objective` on 1655, or the 1655 completion. The UO-04, UO-07 and UO-08 merge hold is lifted, and no packet text changes. | Decided by architect overnight 2026-10-18, owner may overrule | Every source already agrees, so there is nothing left to choose. (1) The SGC ledger K5 C4 row (`sgc-work-packets.md`) lists exactly these departure rows and "1655/4940 active", with Omega as the receiver. (2) The US-32 step-owner table gives 4943/4944 to SGC, 4941 to the C4 window, and 4940 and 4942 to Omega, and says "never `complete_objective` on 1655 at the SGC". (3) US-33 ends the Free Jaffa at 4940 plus one Travel. (4) `execution-contract.md` § ownership: SGC owns the departure; Omega owns arrival, beam platform, the Jackson or Bra'tac delivery and the onward offer. (5) `free-jaffa-route.md` gives Omega 1655 S4942/O6058/T7743. (6) Shipped rows: 4941 holds only O6055 (travel), 4940 holds the world-18 beam objectives, and 4942 holds the world-80 deliveries O6056/O6058 (`mission_objectives.sql:3134-3146`). (7) `advance_step` (`progression.rs`) completes the old step's objectives. The SGC ledger gets a matching line (`sgc-work-packets.md` D-US12). Cost if the owner overrules: one follow-up packet re-keys the step ids in UO-04/07/08's seeds and tests. |
| D-UO26 | **U-D07, the ambient cap, settled for UO-14 only.** UO-14 may merge with at most **24** ambient spawns (63880-63903) of templates 63860 (Airman, clone of 31) and 63861 (Free Jaffa Warrior, clone of 34, moniker 26723). They are static, inert (no dialog, chain, interaction, loot, reward or XP), use shipped bodies and monikers only, and carry the CIMMERIA_AUTHORING placement label. The rest of U-D07 stays HELD: any authored copy or barks (UO-15, H-LBL) and any reward on authored content. | Decided by architect overnight 2026-10-18, owner may overrule | (1) The owner's four-zone direction, which U-D07 names as its starting reference, asks for "ambient NPC density" (`four-zone-planning/README.md`, MMO fullness). (2) Precedent: the Castle population campaign merged 24 friendly static spawns in world 8, with no dialog, on the same CIMMERIA_AUTHORING basis (`castle-population/README.md`), and the owner ruled on its follow-up decisions without objecting to the count. (3) UO-14 adds no text, reward, chain or interaction, so none of U-D07's reward or labelling questions apply to it. (4) It is droppable: one revert removes it, and no other packet reads 63860-63903. Cost if the owner overrules: revert UO-14, or trim the spawn rows to the new cap and change `AMBIENT_CAP`. SGC's D-US3 (authored copy, US-40) is a different question and stays BlockedDecision. |

## Provenance and stop conditions

**Provenance labels:**

| Label | Applies to |
|---|---|
| RESTORED_SOURCE | Mission, dialog and moniker rows the client ships: 1655, 4641, 5025, 6168, the speakers, the monikers, templates 31 and 34. The developer concept art's labels, kinds and relative placement ([omega-concept-art.md](omega-concept-art.md)), never its positions. |
| CIMMERIA_RECONSTRUCTION | Glue that makes shipped content run: the chains, clone templates, maps, respawner, placements, services and re-offer. |
| CIMMERIA_AUTHORING | Anything new: the ambient placement (UO-14), authored copy (UO-15), the extra vendor spawns (D-UO28), the guides (D-UO31), the medic's heal and line, any inn or battlemaster placeholder, and the Asgard hearing (UO-16). |

Every SQL block carries its label in a comment. Players see no label (H-LBL, 2026-10-19): no tag in any text a player reads.

**Stop the campaign if:**

- a faction can complete another faction's 1655 objective;
- a faction receives another faction's `The SGU` mission;
- one click shows two dialogs;
- a relog or a ring round trip leaves two entries bound on one NPC;
- 1655 completes before the Accept;
- Beta cannot consume Omega's departure state.

A green Omega-only suite is a checkpoint, not route completion. The route is done when UO-U1 has walked both factions in the client.

## Overnight run 2026-10-18/19

This is the close-out ledger of every decision made without the owner during the overnight run. Each one carries its reason, and the owner may overrule any of them. D-UO21 to D-UO26 are also in § Overnight autonomous run 2026-10-18 above; the rest are recorded here for the first time. What merged: PRs #1882 (concept art), #1890 (W1), #1902 (W2), #1908 (W3a), #1911 (W3b), and the close-out PR (UO-13 tests and UO-99 docs). None of it has been run in a client.

| Decision | What was decided | Why | Owner may overrule |
|---|---|---|---|
| U-D04 (D-UO25) | Omega receives 1655 at S4940 with O6055 done, and owns S4940 to S4942 and the S4942 completion. SGC owns S4943, S4944, S4941 and the departure. This also corrects F21: S4940 requires O5647 and O5644. The SGC ledger carries a matching D-US12 row. | The architect ruled it so Omega's merge did not wait on the SGC ledger. | Yes. A follow-up would re-key the step ids in UO-04, UO-07 and UO-08. |
| U-D07 (D-UO26) | At most 24 inert ambient spawns (UO-14 shipped 22). The rest of U-D07 stays HELD: authored copy (UO-15), H-LBL, and any reward on authored content. | The ambient crowd carries no copy, chain, loot or XP, so it adds nothing to rule on and drops by deleting the UO-14 blocks. | Yes. Revert UO-14 to drop the crowd. |
| D-UO21 | `svc.banker` keeps its navmesh point (-88.06, -48.02, -102.92) although the concept art draws the Bank in the Common Area. | The nearest-anchor rule put it in the `sgc` area at 122.55 m against 125.51 m. That 3 m margin is inside the proxy's error, and the art gives layout intent, never coordinates (D-UO15). | Yes. |
| D-UO22 | Bridges E and W (world 80) get no placement rows. | They are navmesh components 12 and 26; `nav_inspect --gap-pair 11,12` and `11,26` find no route from pad 17. | Yes, if the owner wants unreachable set dressing. |
| UO-14 nav-guard fix | The UO-02 scaffold helper `reaches()` now follows Detour `Partial` paths hop by hop (16 at most) and still refuses another component. | It took one 256-polygon corridor, so points about 260 m from Modi came back `Partial` and read as another component. The coder had dropped all six Free Jaffa spawns. The fix restored spawns 63892 to 63897, and a follow-up test refuses a hub-island point. | Yes. |
| UO-14 allowlist deviation | That review fix edited the scaffold test helper in `spawner_tests/union_omega/mod.rs`, which sits outside UO-14's allowlist. | The bug lived in the shared helper, and fixing it in UO-14's own files would have left a wrong guard in place. | Yes. |
| UO-08 `mission_id` | Chains 63830 to 63832 used `scope_id` and an `add_dialog_set` `mission_id` of 1500. They now use 1655, the mission the delivery runs under, as UO-07 does. A seed comment was corrected separately. | The review found the mismatch against UO-07's pattern. The wrong-state test now covers it. | Yes. |
| UO-04 spec drift | The spec said both S4940 objectives were optional. The shipped data has required O5647 and O5644 plus optional O5645 and O5646. | D-UO25 corrects F21. UO-04's guard stays, since it keeps Omega safe if the data changes. The UO-04 seed header comment still carries the older wording, which is doc debt (no seed edit in this PR). | Yes. |
| D-UO24 | Generated Tauri schema files were discarded to unblock the #1908 merge. | They are build output that leaked into the diff (`src-tauri/gen/schemas`, `tools/ContentEditor/gen/schemas`); no content or seed impact. | No effect on content. |
| Squash-merge retires | The Omega worktrees were force-retired after their PRs squash-merged. | A squash merge leaves the branch commits unmerged by ancestry, so the normal retire check refuses them. | No effect on content. |
| UO-99 overnight form | UO-99 ran without UO-U1 results and without the vendors. It left `ring-transport-system.md`, `gap-analysis/new-systems.md` and `death-respawn-system.md` untouched. | UO-U1 H4 has not passed, and `death-respawn-system.md` lists no per-zone respawners. | Yes. Rerun the skipped rows after UO-U1. |

## Blockers and owner decisions

The owner answered the 2026-10-18 list on 2026-10-19 (§ Owner decisions 2026-10-19). Open now, in priority order:

1. **D-UO32, inn and battlemaster.** Decided (a), 2026-10-19: build nothing. UO-36 is skipped.
2. **D-UO33, the medic's name.** Decided: 6964 `Castle Medic`.
3. **D-UO34, mission 1522 and Otin.** Asked after the UO-16 seal merges, with the seal's options. Blocks UO-41 and UO-42.
4. **Owner review, optional:** D-UO28 (tier placement), D-UO29 (price rule), D-UO30 (list 1 for class trainers), D-UO31 (crafting guides), D-UO35 (interaction bits), D-UO36 (auction is the Black Market) and D-UO37 (bodies, no service rewards). Each is decided and each lists its cost if overruled.
5. **External dependency:** a playable Asgard start (character-creation row 9 is `NON_CANONICAL_BLOCKED_LEGACY`, asgard-route AS-E1). Blocks UO-41/UO-42 and any Asgard UAT row.

These HELD items block nothing built: O5646 waypoints, the H1 DHD dial-back, H2, H6, gate 5 under the dais, F35 and F28.

## UO-U1 live UAT

The lab UAT has not run, and the lab is shared, so ask the user before using it. The recipe is the [`lab-uat` skill](../../../.claude/skills/lab-uat/SKILL.md), [live-research-lab.md](../../guides/live-research-lab.md) and the [UO-U1 packet](omega-packets/UO-U1.md) (fixtures R1 to R4 through `server_stage_fixture`). The checklist form is [unified-uat.md § Union Omega](../../guides/unified-uat.md#union-omega). Results go to `omega-packets/worknotes/UO-U1-results.md`, which does not exist yet. The vendor and service rows (§ R4 S3 and § R5) cannot run until W8 and W9 merge. UO-U1 stays a lab UAT the owner runs; agents never use the lab.

The lab owes two observations that only a client run can settle. Both came from the UO-13 reviews:

- `interactions_count = 1` on every `Content: adding dialog set` line.
- `Content: removing dialog set` before `Content: completing mission`.

When UO-U1 passes, rerun the UO-99 rows it gates: `ring-transport-system.md` and `gap-analysis/new-systems.md` (pad 14 to 17, H4).
