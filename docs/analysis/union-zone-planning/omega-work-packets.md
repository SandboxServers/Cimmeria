# Omega Site work packets

> **Type:** How-to (dispatch contract and ledger)
> **Audience:** the Union Omega coordinator, packet-coders, review-fixers, mission, movement and items advisors
> **Last updated:** 2026-10-18, against `origin/main` @ `d1bba0400`
> **Status:** Planned. Nothing dispatched. The Union campaigns stay held until FZ-99 (U-D03, an orchestrator gate). This file replaces the 2026-10-14 table (UO-00..UO-08); see § Renumbering.
> **Companions:** [Omega campaign](omega.md), [Union execution contract](execution-contract.md), [SGC packets](sgc-work-packets.md), [Dakara packets](dakara-work-packets.md), [Beta packets](beta-work-packets.md), [Asgard route](asgard-route.md), [packet folder](omega-packets/), [resume card](omega-packets/RESUME.md)

## Short answer

Omega is the first common hub. A Human or Free Jaffa arrives on public `Omega_Site` (world **18**) on mission **1655** at step **4940**. The steps are:

1. Talk to Modi (objective 5645).
2. Beam to the Command Center (`Omega_Site_CmdCenter`, world **80**, pad region **17**) on the Asgard transporter, which uses the ring-transport dispatch. The beam advances the mission to step **4942**.
3. Talk to Daniel Jackson (Human, dialog **4641**) or Bra'tac (Free Jaffa, dialog **5025**). Accept completes 1655, offers **1423** or **1500** (`The SGU`), and grants Beta's address (gate **6**).

Beta owns everything after that. The services packets give the hub a personal banker, two registrars (Team and Command) and faction vendors. The ambient packet gives it airmen and Free Jaffa warriors. All of them use shipped monikers on the shipped map. The Asgard route (1522 and 481) stays a stub until the owner rules on U-D08/U-D11.

Everything uses existing engine primitives. Nothing needs a client patch, a new opcode or a code change: the packets are seeds, chain SQL and tests. The one executed-test packet (UO-13) is a Sonnet `rust-gameserver-dev` packet of about 50k; every other code packet is Haiku-sized (at most 40k working tokens).

## Findings

Every finding was checked against the planning baseline `d1bba0400` (an ancestor of `main`); locate each anchor by its text, not its line number. The packet column names the packet that acts on it.

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | Omega has no chain today. `db/resources/Content/Seed/` has no Omega file, and nothing binds dialog set map 7015 (Modi's 6168, `Shutdown`, flags 16777216). | `git grep 7015 db/resources/Content` is empty | UO-04 |
| F2 | 1655's world-18 and world-80 steps exist: S4940 (O5645 Modi, O5646 waypoints) and S4942 (O6056 → T7741 Jackson; O6058 → T7743 Bra'tac). | `db/resources/Missions/Seed/` | UO-04, UO-07, UO-08 |
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
| F21 | O5645 and O5646 (S4940) are both optional, and the engine completes a mission once no required objective is left open on its step. A chain that completes either objective therefore completes all of 1655 early. No Omega chain uses `complete_objective` on 1655. | `mission_objectives.sql:3136,3138`; `crates/content-engine/src/progression.rs:464-506` | UO-04, UO-07..09 |
| F22 | `mission_abandoned` fires after the mission instance is removed, so the abandoned mission already reads `not_active` on that event. INFO "fire_mission_abandoned: matched". | `event_dispatch/mission.rs:217`; `chain_replay_tests/mission_abandoned.rs:50-67` | UO-09 |
| F23 | The 1655 guard: every delivery chain (63820-63822, 63830-63832) carries `mission_status 1655 eq active`, every re-offer chain (63840-63845) carries `eq completed`. The two families never both match one event. | design rule; tested in UO-07..09 and UO-13 | UO-07..09 |
| F24 | A registrar's type comes from its single interaction set: 7447 is Team, 7448 is Command. A template with both sets has no type, so the one Omega registrar moniker needs two NPCs. | `org_registrar.rs`; `live_db_debug_registrars.rs:135-148` | UO-10 |
| F25 | The client names four tiers per Omega weapon and armor vendor (for example 27240-27243 `DN_npc_ven_OmegaSite_HumanWeaponsTier01..04`) and four specialties per crafting vendor. One template per role is a reduction. | `texts.sql:37085-37091` | UO-06, UO-11, UO-12 |
| F26 | No Omega ambient moniker ships. Shipped bodies exist: template 31 `Airman` (faction 1, name 8360, interaction 0) and template 34 `SGC Jaffa`; moniker 26723 `Free Jaffa Warrior` is already reused by template 208 (INFERRED). | `entity_templates.sql:67,69,741-743` | UO-14 |

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
| D-UO8 | Vendor stock and prices for the Tau'ri and Jaffa vendors. | **BlockedDecision** (owner) | It is an economy call. UO-06 proposes it; UO-11/12 wait. |
| D-UO9 | Thor stands in the Command Center as a non-interactive clone (63803). | Decided (revised 2026-10-18) | F7: Thor speaks in 5025. His Asgard role (1522 S4773) stays in UO-16. |
| D-UO10 | A re-offer path lets a player who abandoned 1423/1500 take it again from the same NPC (CIMMERIA_RECONSTRUCTION). | Decided | Otherwise an abandon soft-locks the route. |
| D-UO11 | The letter at departure is the same decision as D-US11. | **BlockedDecision** (owner + SGC) | Omega works either way. If SGC grants 6786, UO-07 adds `remove_item 6786`. |
| D-UO12 | Ambient life is CIMMERIA_AUTHORING placement of shipped bodies and monikers, in its own droppable packet, capped at 24 spawns. | Decided; the cap is HELD on U-D07 | Faithful extension, removable in one revert (F26). |
| D-UO13 | This renumbering supersedes UO-00..UO-08 of 2026-10-14. | Decided | § Renumbering. |
| D-UO14 | The hub banker and the two registrars are clones of the debug-hub rows 370, 330 and 331 (CIMMERIA_RECONSTRUCTION), on the shipped Omega monikers. | Decided | F14, F24. No new code: the existing banker and registrar arms answer the click. |

## ID map (D-UO1)

| Table | IDs | Use |
|---|---|---|
| `content_chains` | 63801 | beam |
| | 63810-63811 | Modi |
| | 63820-63822 | Human |
| | 63830-63832 | Free Jaffa |
| | 63840-63845 | re-offer (63846-63849 spare) |
| | 63850-63899 | services spare |
| | 63900-63949 | authored (HELD, UO-15) |
| | 63950-63999 | Asgard (UO-16) |
| `entity_templates` | 63800-63803 | Jackson (clone of 51), Bra'tac (59), Teal'c (30), Thor (64) |
| | 63810 | Banker (clone of 370) |
| | 63811 | Team registrar (clone of 330) |
| | 63812-63816 | Human Weapons, Human Armor, Human Crafting, Common Crafting, Consumables |
| | 63817-63819 | Jaffa Weapons, Jaffa Armor, Jaffa Crafting |
| | 63820-63829 | Asgard services (HELD, UO-16) |
| | 63830 | Command registrar (clone of 331) |
| | 63831-63859 | spare |
| | 63860-63861 | ambient Airman (clone of 31), Free Jaffa Warrior (clone of 34, name 26723) |
| | 63862-63899 | ambient spare |
| `spawnlist` | 63800-63803 | cast |
| | 63820-63829 | banker, Team registrar and vendors (template + 10) |
| | 63830-63839 | Asgard services (HELD) |
| | 63840 | Command registrar |
| | 63841-63849 | spare |
| | 63880-63903 | ambient (24; 63904-63959 spare) |
| `respawners` | 180 | `Omega Site Gate Plaza` |
| `dialog_sets` | 63800 | `Omega Command Center offers` |
| `dialog_set_maps` | 63800 | Jackson delivery: dialog 4641, `The SGU`, flags 16777216 |
| | 63801 | Bra'tac delivery: dialog 5025, `The SGU`, flags 16777216 |
| | 63802 | Jackson re-offer: dialog 4641, flags 8388608 |
| | 63803 | Bra'tac re-offer: dialog 5025, flags 8388608 |
| `item_lists` | 63802-63809 | one per vendor template, 63800 + (template − 63810) |
| | 63810-63819 | Asgard vendors (HELD) |
| `item_list_items` | 638000-639999 | row = 638000 + (list − 63800) × 100 + k |

Flags are 16777216 = `INT_A_STORY_MISSION_ACTIVE` and 8388608 = `INT_A_STORY_MISSION_AVAILABLE` (`crates/entity/src/interaction_flags.rs:68-70`). All maps use `min_level 1` and `'{}'` arrays. Template and spawn ids may repeat across tables; each table has its own key.

## Contracts

### Receiving: C4 from SGC (US-32), and the Dakara legs

| Field | Value |
|---|---|
| Departure | SGC US-32: `advance_step 1655 '4941'`, `advance_step 1655 '4940'`, `cross_world_teleport 'Omega_Site'` |
| State on arrival | 1655 active, S4940 active, O5645 and O5646 open |
| Arrival point | Published by UO-00 from UO-01's probe (first choice (34.70, -51.86, -142.0), on the gate plaza in front of gate 5) |
| Archetypes | 1-4 (Human, through the SGC) or 7 (Free Jaffa, through Dakara 1654 and then the SGC) |
| Letter | None required. D-UO11 = D-US11. |
| Omega's first event | `player_loaded 'Omega_Site'`, which binds Modi (chain 63810) |
| SGC must not | complete O5645 or O5646 by chain (F21); that completes 1655 before Omega runs |
| Abandon of 1655 | SGC's re-offer owns it; Omega has no 1655 re-offer |
| HELD | **U-D04**: whether Omega receives at S4940 with O6055 done, and owns S4940 → S4942 and the S4942 completion |

### Producing: to Beta (UB receivers 23020/23220/23310)

| Route | State Omega leaves | Gate | Beta's first event |
|---|---|---|---|
| Human | 1655 completed; 1423 active at S4223 | gate 6 address granted | Beta 23020 on arrival in world 23 |
| Free Jaffa | 1655 completed; 1500 active at S4369 | gate 6 | Beta 23220 |
| Asgard | 481 at S1760 with item 3399 (UO-16, HELD) | gate 6 | Beta 23310 |

Omega fires no Beta step (D-UO2). Beta chain 23001 grants Omega's gate 5 to a player who reaches Beta first. A player who abandons 1423/1500 gets it back from Jackson or Bra'tac in world 80 (UO-09). UO-13 exports `pub(super)` departure-state fixtures in `chain_replay_tests/union_omega/handoff.rs`, so Beta's replay tests start from Omega's exact output. **Fixture ownership (settled with Beta):** UO-13 owns and exports the *departure* fixtures; Beta's UB-10, UB-20 and UB-30 build the *receiver* fixtures (the inbound state staged, Beta's chain asserted). Beta's wave B2 can run before UO-13 merges, so Beta stages the state itself and may switch to the `handoff` import once UO-13 is on `main`. Both sides assert the same table row above.

## Renumbering (for sibling ledgers)

| Old (2026-10-14) | New |
|---|---|
| UO-00 specialist seal | UO-01 (placements) and UO-00 (docs seal) |
| UO-01 hub travel and safety | UO-03 (respawner) and UO-04 (beam, Modi) |
| UO-02 Human induction | UO-07 |
| UO-03 Free Jaffa induction | UO-08 |
| UO-04/05/06 Asgard | UO-16 (stub, BlockedDecision) |
| UO-07/08 hub life and integration | UO-10..12 (services), UO-13 (handshake and Beta fixtures), UO-14 (ambient) |

Beta's old "UO-07/08 receiver fixtures" citation now means the departure fixtures of **UO-13**; the receiver fixtures stay with Beta (UB-10/UB-20/UB-30). The Asgard route ledger's "Omega UO-05/06" is now **UO-16**.

## Packets

| ID | Wave | Role | Size | Scope | Status | File |
|---|---|---|---|---|---|---|
| UO-01 | W1 | `movement-teleport-advisor` | ~35k | Nav-probe every coordinate, then write the placement worknote | Ready | [UO-01](omega-packets/UO-01.md) |
| UO-02 | W1 | `packet-coder` | ~40k | Scaffold: empty seed files, includes, test modules and a visitor guard | Ready | [UO-02](omega-packets/UO-02.md) |
| UO-00 | W1 | `documentation-writer` | ~30k | ID row, C4 arrival point, sibling handoff note | BlockedDependency UO-01 | [UO-00](omega-packets/UO-00.md) |
| UO-03 | W2 | `packet-coder` | ~30k | Respawner 180 and the hub placement test | BlockedDependency UO-01, UO-02 | [UO-03](omega-packets/UO-03.md) |
| UO-04 | W2 | `packet-coder` | ~40k | Beam 63801 and Modi 63810/63811 | BlockedDependency UO-02; merge waits on U-D04 | [UO-04](omega-packets/UO-04.md) |
| UO-05 | W2 | `packet-coder` | ~40k | Cast templates and spawns 63800-63803, dialog set and maps | BlockedDependency UO-01, UO-02 | [UO-05](omega-packets/UO-05.md) |
| UO-06 | W2 | `items-systems-advisor` | ~35k | Services seal: the stock proposal for D-UO8 | BlockedDependency UO-01 | [UO-06](omega-packets/UO-06.md) |
| UO-10 | W2 | `packet-coder` | ~35k | Banker 63810 and registrars 63811 (Team), 63830 (Command) | BlockedDependency UO-01, UO-02 | [UO-10](omega-packets/UO-10.md) |
| UO-07 | W3 | `packet-coder` | ~40k | Human induction 63820-63822 | BlockedDependency UO-04, UO-05; merge waits on U-D04 | [UO-07](omega-packets/UO-07.md) |
| UO-08 | W3 | `packet-coder` | ~40k | Free Jaffa induction 63830-63832 | BlockedDependency UO-04, UO-05; merge waits on U-D04 | [UO-08](omega-packets/UO-08.md) |
| UO-09 | W3 | `packet-coder` | ~35k | Re-offer 63840-63845 | BlockedDependency UO-07, UO-08 | [UO-09](omega-packets/UO-09.md) |
| UO-14 | W3 | `packet-coder` | ~35k | Ambient airmen and Free Jaffa warriors (droppable) | BlockedDependency UO-01, UO-02, UO-10; merge waits on U-D07 | [UO-14](omega-packets/UO-14.md) |
| UO-11 | W4 | `packet-coder` | ~40k | Tau'ri vendors 63812-63816 | BlockedDecision D-UO8; BlockedDependency UO-06, UO-10 | [UO-11](omega-packets/UO-11.md) |
| UO-12 | W4 | `packet-coder` | ~30k | Jaffa vendors 63817-63819 | BlockedDecision D-UO8; BlockedDependency UO-06, UO-10 | [UO-12](omega-packets/UO-12.md) |
| UO-13 | W4 | `rust-gameserver-dev` (Sonnet) | ~50k | Executed handshake tests and Beta fixtures | BlockedDependency UO-07, UO-08, UO-09 | [UO-13](omega-packets/UO-13.md) |
| UO-U1 | W5 | Sonnet UAT lead + Haiku `lab-driver` | 4 runs | Client walk of both routes, re-offer, services | UATPending; ask the user first | [UO-U1](omega-packets/UO-U1.md) |
| UO-99 | W5 | `documentation-writer` | ~35k | Close-out: chains, zone audit, UAT, status (once) | BlockedDependency all, UO-U1 results | [UO-99](omega-packets/UO-99.md) |
| UO-15 | none | stub | - | Authored copy and chatter | HELD (U-D07, H-LBL) | [UO-15](omega-packets/UO-15.md) |
| UO-16 | none | stub | - | Asgard route | BlockedDecision (U-D08, U-D11) | [UO-16](omega-packets/UO-16.md) |

## Waves

| Wave | Packets | Review | Gate |
|---|---|---|---|
| W1 | UO-01, UO-02, then UO-00 | one review-fixer for UO-02 and UO-00; UO-01 is a worknote the orchestrator checks for every key | U-D03 (FZ-99) |
| W2 | UO-03, UO-04, UO-05, UO-06, UO-10 | one review-fixer (UO-06 is a worknote) | W1 merged |
| W3 | UO-07, UO-08, UO-09, UO-14 | one review-fixer | W2 merged; UO-09 after UO-07/08 |
| W4 | UO-11, UO-12, UO-13 | one review-fixer | W3 merged; D-UO8 option A for UO-11/12 |
| W5 | UO-U1, then UO-99 | UO-99 gets one documentation review | W4 merged (or its packets dropped); the user's yes for UO-U1 |

UO-04, UO-07 and UO-08 may be coded and reviewed before U-D04 is answered, but they **merge only after** it. If the answer changes the receive state, the review-fixer rewrites the step ids in those three packets' tests and seeds. No other packet changes. UO-14 likewise merges only after U-D07; if the owner rejects ambient life, drop the packet.

Parallel coders never edit the same lines: UO-02 creates every shared file with one `-- [UO-xx]` marker section per packet, and each packet writes only below its own marker. The cell-catalog `tests/mod.rs` gets one `mod` line from each of UO-10, UO-11 and UO-12. That is a trivial merge, which the review-fixer resolves.

Every packet inlines its rules, the chain contract (chain packets) and its lane commands. Packet-coder skips CLAUDE.md, so nothing a packet needs lives only here.

## HELD and blocked

| Item | Exact question | Who answers | Blocks |
|---|---|---|---|
| **U-D04** | Does Omega receive 1655 at S4940 with O6055 done (SGC fires `advance_step 4941`, then `4940`, before the teleport)? Does Omega own S4940 → S4942 and the S4942 completion? | owner, with the SGC coordinator | the merge of UO-04, UO-07, UO-08 |
| **U-D07** | What reward and scale may Cimmeria-authored content carry at Omega? How many ambient NPCs (proposed cap 24), and any XP, items or naquadah (proposed none)? | owner | UO-14's merge, UO-15 |
| H-LBL | How must authored copy be labelled to players (a provenance tag in the text, a tooltip, or nothing)? | owner | UO-15 |
| D-UO8 | May Omega's vendors sell UO-06's proposed stock (option A: one moniker per role), option B (every tier, a new template block), or nothing? | owner | UO-11, UO-12 |
| D-UO11 / D-US11 | Does the SGC grant 6786 `Letter to Daniel Jackson` to Humans, and anything to the Free Jaffa? | owner + SGC | one optional `remove_item` row in UO-07 |
| U-D08 / U-D11 | The Asgard route into Omega (1522, the Council, item 3399, 481); see UO-16 for the exact questions | owner | UO-16 |
| U-D03 | The Union campaigns are held until FZ-99. | orchestrator | W1 dispatch |

These items are HELD and block nothing:

- **O5646 waypoints:** no data.
- **H1 DHD dial-back:** world 18 has no DHD dial-home chain.
- **H2:** whether world 80 is private.
- **H6:** gating on pad 14.
- **Gate 5:** its row sits 0.41 m under the dais.
- **1425 S4244:** the Jackson tail (D-UB5 / HELD-B4).
- **1655:** the `step_enabled` flags.
- **Otin and Heimdall:** no evidence.
- **Worlds 81/82:** out of scope.
- **UAT death row H9:** the legacy `.kill` GM command is not registered (`docs/guides/unified-uat.md:905`); UO-U1 marks H9 BLOCKED if no self-damage command exists.

## Rejected additions

Each of these was rejected for lack of evidence, or because it would break a rule.

- **Woolsey at Omega:** 5025 says he must not discover the operation.
- **Mission 631** on Omega.
- **A "Stargate Plaza" area** from 1513.
- **Otin** with speaker 2509 (an unnamed speaker).
- **Any content** in worlds 81/82.
- **Ambient VO or barks** with no shipped line.
- **An inn, auctioneer, guild registrar or battlemaster** with no Omega moniker.
- **A medic:** no moniker.
- **Hostile Free Jaffa Attacker (208)** as ambient.
- **Ambient patrols:** no path data; static spawns only.
- **`interact_tag` chains:** they would display the dialog twice (F4).
- **`complete_objective` on 1655:** it completes the mission early (F21).

## Provenance and stop conditions

**Provenance labels:**

| Label | Applies to |
|---|---|
| RESTORED_SOURCE | Mission, dialog and moniker rows the client ships: 1655, 4641, 5025, 6168, the speakers, the monikers, templates 31 and 34. |
| CIMMERIA_RECONSTRUCTION | Glue that makes shipped content run: the chains, clone templates, maps, respawner, placements, services and re-offer. |
| CIMMERIA_AUTHORING | Anything new: the ambient placement (UO-14), authored copy (UO-15). |

Every SQL block carries its label in a comment.

**Stop the campaign if:**

- a faction can complete another faction's 1655 objective;
- a faction receives another faction's `The SGU` mission;
- one click shows two dialogs;
- a relog or a ring round trip leaves two entries bound on one NPC;
- 1655 completes before the Accept;
- Beta cannot consume Omega's departure state.

A green Omega-only suite is a checkpoint, not route completion. The route is done when UO-U1 has walked both factions in the client.
