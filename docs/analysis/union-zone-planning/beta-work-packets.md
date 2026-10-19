# Beta Site E1 Union work packets

> **Type:** How-to (dispatch contract)
> **Audience:** Union Beta coordinator, Omega and SGC architects, packet workers and reviewers
> **Last updated:** 2026-10-18 (architect pass against `origin/main` @ `d1bba0400`, an ancestor of current `main`; locate each anchor by its text, not its line number)
> **Status:** Wave B0 integrated (UB-00a1 review pass 2 open). ARCH-B1 (2026-10-18) ruled D-UB4 to D-UB9 and folded the seal findings into the packets; wave B1a (UB-01a, UB-02a, UB-DP) is Ready. Live dispatch state: [continuation ledger](../union-beta-e1-continuation/README.md).
> **Companions:** [Beta E1 campaign](beta.md), [Union execution contract](execution-contract.md), [Omega packets](omega-work-packets.md), [route evidence](evidence.md), [Asgard route](asgard-route.md), [Free Jaffa route](free-jaffa-route.md), [packet folder](beta-packets/)

## Short answer

Beta_Site_Evo_1 (world 23, gate 6) has a sealed map and almost nothing on the server side. There are no NPCs, no content chains, no arrival pin, and two respawners at the origin. Every Beta dialog has `accepts_mission_id` NULL, so each grant needs a chain. This campaign builds the **first visit** for the three Union races:

1. **Safe travel first (pilot).** Gate 6 gets an arrival pin at the map's only PlayerStart. Respawners 6 and 7 get real positions, and the exemption in `live_db_loaders.rs` is removed. A player who arrives on a Union mission learns the way back to Omega (UB-01a, UB-01b).
2. **Cast and anchors.** Beta-only templates 2300-2349 use the client's own display-name strings (RESTORED_SOURCE monikers in `texts.sql`). Spawns are placed against map pins that UB-00b re-probes on the navmesh (UB-02a to UB-02d).
3. **Three race arcs, one mission per packet**, each with its own chain range and seed file:
   - Human: 1423 → 381 → 473 → 478 → 1425 → 1426 → 1427.
   - Free Jaffa: 1500 → 1502 → 1503.
   - Asgard: 481 → 420 → 421 → 1147, plus 443, 477 and 479.

   Each arc stops at its last **Beta-local** step (D-UB5 A).
4. **Mission 421 is no longer held for locality.** The infirmary is Beta-local with high confidence (F6). Only its object binding is left, and UB-00b seals it.
5. **Authored content is a separate packet you can drop (UB-50).** It is HELD on the owner's label policy (H-LBL), U-D07 and the CB-N05a client proof, and the first ship does not need it (D-UB5 A, D-UB8).

## What other campaigns own (do not touch)

- **Omega (UO-xx)** offers 1423, 1500 and 481. It issues item 3399 (UO-16, HELD on U-D08/U-D11). It owns every Omega-side chain, plus any Jackson, Bra'tac, Thor, Heimdall or High Council actor. Beta never re-offers those missions (D-UB3).
- **SGC (US-xx)** owns Woolsey and Hammond on SGC_W1, and 1180.
- **Dakara (UD/DK)** owns chains 8001-8499 and 63500-63549, templates 440-479 and the Dakara ID block 63500-63799.
- `docs/analysis/four-zone-planning/id-allocation.md` belongs to the coordinator. Beta filed an amendment request (UB-00c); the coordinator approved it (D-UB9) and ARCH-B1 applied the row on the coordinator's behalf. No Beta packet edits the file.
- Closed-Beta Visit 2 (1807-1826) and E2 (world 60) are UB-L research only.

## Findings

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | World 23 has five spawns: DHD 40, ring points 148 and 152, and Asgard beam bases 149 (ground) and 151 (deck). It also has gate 6 with no arrival pin and region 1004. No NPC, no chain file and no `ring_transport_regions` rows. | `spawnlist.sql`, `stargates.sql:79`, `respawners.sql:79-81` | UB-01a, UB-02* |
| F2 | Respawners 6 (`Respawner`) and 7 (`Respawner_bet_Infirmary`) sit at (0,0,0). `seeded_respawners_are_not_at_the_world_origin` exempts them through `UNAUTHORED_BY_DESIGN`. | `crates/cell-catalog/src/cell/spawner/tests/live_db_loaders.rs:251-256` | UB-01a |
| F3 | The only PlayerStart is (635.761, -60.134, 388.448), on nav component 251 and identical in all five dossier builds. It sits 17.9 m out on gate 6's +X facing axis (yaw 1.571). | placement manifest §10.1 | UB-01a |
| F4 | Archetypes: Any 0, Soldier 1, Commando 2, Scientist 3, Archeologist 4, Asgard 5, Goauld 6, Sholva (Free Jaffa) 7, Jaffa 8. A missing archetype reads as -1, so a Human gate is `gte 1` plus `lte 4`. | `cell-catalog/src/cell/spawner/abilities.rs:41-43`; `content-engine/src/conditions/mod.rs:333-338` | all chain packets |
| F5 | `fire_dialog_choice` and `fire_dialog_open` now set `archetype` (CS-01b). Race gates therefore work on `dialog_choice` chains. The "archetype never matches" note in `castle_701_chains.sql:107-112` is stale. | `event_dispatch/dialog.rs:39-43`, `dialog_archetype_tests.rs` | all chain packets |
| F6 | The 421 infirmary is **BETA_LOCAL** (high confidence): moniker `A01_BetaE1_ASG_04MechanicalError` resolves to world 23. Mimir's 1987/1988 say "human infirmary". Twin 383 starts at the infirmary. EM-Infirmary01 is on tile 00020009 at (961.3, 14.3, 284.9), with an interior volume at (961.3, 16.9, 284.9), on nav 251. | mission manifest §8; placement manifest §4 | UB-32 (D-UB6) |
| F7 | Nav components 251 (gate, east valley) and 70 (west valley, the Jaffa camps) have no link of 3 m or less. UB-00b confirmed no walk route; D-UB4 is ruled A (ring pair 148 to SW_RING, UB-02d). | placement manifest §9; `placement-seal.md` Q1 | UB-02d, UB-21 |
| F8 | The Asgard vessel deck (688.2, 151.6, 70.3) is nav island 355 and can be reached only by beam. The Science Facility interior (nav 890) joins 251 across a 0.90 m door gap. The underground complex (1047/1060) can be reached only by ring, so seed nothing there. | placement manifest §9 | UB-02c, UB-30 |
| F9 | `texts.sql` holds the client's own monikers for Beta NPCs and objects: 7366 Carter, 7367 Heintz, 7368 Felger (empty text), 7373 Satterfield, 7374 Ras'nor, 7388 Mimir, 7414 Sick Nine Peaks Warrior, 7905 Pierce, 8552 Debeaux; objects 7376, 7377, 7383, 7392, 7397, 7380, 7381, 7402, 7909, 7957, 8164, 8211, 8489, 8490, 8493, 8497. | `db/resources/Texts/Seed/texts.sql` | UB-02a |
| F10 | `entity_templates.sql` has no row for Felger, Satterfield, Mimir, Ras'nor, Pierce, Debeaux or the sick Jaffa. The cooked client has generic heads plus Carter, Woolsey, Jackson, Hammond and Davis. Mimir uses `BS_Asgard` as Thor (template 64) does. Ras'nor uses `BS_JaffaMale` as Dakara 440 does. | client package census | UB-02a |
| F11 | Every Beta mission row has xp 0 and naq 0 (the AwardXP flag only). Rewards are U-D07. | `missions.sql`, P4 reward audit | D-UB8 |
| F12 | The SGU-family steps (1423, 1425-1427, 1500, 1502-1504) all carry `en=false`. UB-00a1 Q3: the server never reads the flag, so chains drive these steps unchanged (journal display: UB-U1b). | mission manifest finding 8; `mission-seal.md` Q3 | all SGU-family chain packets |
| F13 | There are two flag dialects in Beta `dialog_set_maps`. **Old:** 8388608 blurb, 16777216 story, 33554432 turn-in, 1073741824 object. **New (SGU):** 134217728 offer blurb, 0 offer, 268435456 mid, 536870912 completion. Never mix them in one bind. | mission manifest §4 | all chain packets |
| F14 | `add_dialog_set`'s `slot` is the NPC's **template id**, and the bind is per player (`executor/dialog/mod.rs:166-176`). Beta-only templates keep Beta binds from lighting up Carter or Thor on other worlds. | castle_701 chain 1201 (`slot 149` = Gerschon) | UB-02a, D-UB10 |
| F15 | Main has no phasing primitive. Tollana PH-01a/b are unmerged and `set_phase` does not exist. A shared NPC cannot be hidden or changed for one player. | `git grep set_phase` empty | D-UB11 |
| F16 | Asgard char-creation row 9 is `NON_CANONICAL_BLOCKED_LEGACY` and starts in SGC_W1. The Asgard arc on Beta can be tested by replay, but a player can reach it only after Omega UO-16 (BlockedDecision U-D08/U-D11) delivers it. | `char_creation.sql`; asgard-route.md | D-UB13 |
| F17 | Mismatches with the Omega contract: M1 (who fires the arrival steps), M2 (4643 uses the mid-step flag), M4 (Beta must not re-offer 481), M6 (Heimdall S4381), M7 (446 S1496/S1497), M8 (1425 S4244 Jackson, 1427 S4250 Woolsey), M9 (S4224, S4370 and S1761 are Beta-owned). **M3 and M5 are retired:** they came from mission manifest §9, which is not in the repo, and no packet or contract cites them. A new mismatch takes M10 and up. | mission manifest §9 | [Omega contract](#omega-receiver-contract) |
| F18 | The fires at the Quartermaster (1022.5, 13.9, 271.1) are static map emitters, and nothing toggles them. Mission 443's "contain the fire" can only be an interact or a region at that cluster. | placement manifest §8.1 | UB-34 |
| F19 | **complete_objective trap.** Completing the last required objective of the current step completes the **whole mission**. Mid-mission chains use `advance_step`, turn-ins use `complete_mission`, and multi-objective steps (473 S1724, 477 S1744) use the partition pattern P6. | `crates/cell-content/src/cell/missions/progression.rs:463-506` | UB-12, UB-30, UB-35 |
| F20 | `interact_tag_linter.rs` is 713 lines, over the 700 hard cap, and every Beta chain packet adds allowlist rows to it. The dialog-button linter's `CHAIN_FILES` is a fixed-size array. UB-01b splits the first into a folder (with a `beta_e1.rs` slice) and turns the second into a slice. | `crates/content-engine/tests/it/` | UB-01b |
| F21 | **Interact fallback.** A click first runs trainer, `interact_tag` and `interact_template` chains. Only if none handled it does the server auto-open the first per-player bind's dialog for that template. A bound turn-in is therefore shown, without completing anything, whenever no chain claims the click. So every quota turn-in needs a "not yet" sibling, one slot never carries two dialog binds, and an NPC template with `interaction_type 0` can be clicked only while a bind is live. | `crates/cell-interactions/src/cell/interactions/dispatch/interact.rs:113-135`; `chain_replay_tests/mission_1361/bind_hygiene.rs` | UB-11, UB-31, UB-35, UB-40, UB-45b |

## Decisions

| ID | Decision | Owner | Status |
|---|---|---|---|
| D-UB1 | Race gates: Human = the Union Human gate (`execution-contract.md` § Shared definitions): `archetype gte 1` **and** `lte 4`; Free Jaffa `eq 7`; Asgard `eq 5` (F4). Resolves HELD-B11. | Architect | Decided |
| D-UB2 | Beta fires the arrival steps 1423 S4223, 1500 S4369 and 481 S1760, on `player_loaded Beta_Site_Evo_1` gated on that step being active. They are idempotent across relog. Omega departs the player with the step active and fires nothing for it (M1). | Orchestrator, with the Omega architect | **Decided**: Omega's D-UO2 and its Producing contract (omega-work-packets.md) say the same (it fires no Beta step; Beta's receivers 23020/23220/23310 take over). The Asgard leg stays HELD behind UO-16 (U-D08/U-D11). |
| D-UB3 | Beta never offers 1423, 1500 or 481. Omega does (M4). | Architect | Decided |
| D-UB4 | West-valley access for the Nine Peaks camp. **A:** pair ring 148 (component 251) with a new SW ring row (component 70) as a `ring_transport_regions` pair, CIMMERIA_RECONSTRUCTION. **B:** a walk route. **C:** hold 1502/1503 until a lab path test. | Architect (ARCH-B1, delegated by the orchestrator) | **Decided: A** (high confidence). UB-00b found no walk route between nav 251 and 70, and the client map ships a real ring prefab with its matinee at SW_RING, so A restores a route the map already draws; server-side seeds only, no client patch. Lab row UB-U8 |
| D-UB5 | Offworld tails: 1425 S4244 (Jackson), 1427 S4250 (Woolsey), 1503 S4379 (Bra'tac) and S4381 (Heimdall). **A (default the packets implement):** stop each arc at its last Beta-local step; the mission stays open in the journal. **B:** Omega/SGC receivers, owned by those architects. **C:** authored Beta relays (UB-50, CIMMERIA_AUTHORING). | Architect (ARCH-B1) | **Decided: A** (high confidence). B and C need Omega/SGC receivers or authored copy (H-LBL) that do not exist; A strands no one, because each mission stays open at a talk step that names its offworld NPC. UB-22 is not dispatched and UB-51 is held |
| D-UB6 | 421 is un-held: BETA_LOCAL (F6). UB-32 dispatches once UB-00b seals the object positions. | Architect; orchestrator confirms against the brief's hold | **Decided (confirmed by ARCH-B1)**. UB-00b sealed the infirmary objects inside region `Infirmary`; H-421 is closed |
| D-UB7 | The FV twins 383, 384 and 1146 are not in P4 rows 121-124. They are left out of the first ship. | Architect (ARCH-B1) | **Decided: out** (high confidence). No first-visit row needs them and they would need a fresh step matrix; templates 2303, 2327 and 2336 stay reserved. UB-52 is closed |
| D-UB8 | No reward amounts. Missions keep xp 0 / naq 0 until U-D07. | Architect (ARCH-B1) | **Decided: xp 0 / naq 0** (high confidence). `mission-seal-held.md` Q11: all 17 missions already carry 0 and no reward group, so the first ship needs no seed edit. The amounts themselves stay an owner call (U-D07, UB-50) |
| D-UB9 | ID block: world-prefix ranges for world rows, and 64000-64499 for authored rows (below). The content-chain block 23000-23499 needs a coordinator amendment row in `id-allocation.md`. | Coordinator | **Decided: approved** by the coordinator (the orchestrator), 2026-10-18. ARCH-B1 applied the row to `id-allocation.md`, with the authored screens re-carved to 64124-64193 |
| D-UB10 | Per-player binds only (`add_dialog_set` with `slot = template`). Beta chains never call `set_interaction_type` on a shared NPC. Objects may carry a static template interaction bit (see the contract). | Architect | Decided |
| D-UB11 | No phasing (F15). All per-player state is mission state plus per-player binds. No `set_visible` or despawn on a shared actor. | Architect | Decided |
| D-UB12 | Asgard beam: `interact_tag` on spawn 149 (ground) and 151 (deck), then `move_entity {"use_player": true}` between the two decks. Destinations come from UB-00b probes. CIMMERIA_RECONSTRUCTION. Whether 481 S1762's "teleporter" is this beam is HELD-B7. | Architect | Decided (structure) |
| D-UB13 | Asgard packets ship and replay-test with archetype 5. Players can reach them only after Omega UO-16 (BlockedDecision U-D08/U-D11, F16). | Architect | Decided |
| D-UB14 | Authored content (relays, rewards, ambient lines) lives only in UB-50 and its own seed file, so it can be dropped. | Architect | Decided |
| D-UB15 | Strict linear offer order per race, so a shared NPC has at most one live Beta state per player. A cross-race guard proves it (UB-45a, UB-45b). | Architect | Decided (HELD-B1 can reorder) |

## ID block (D-UB9)

World-prefix rows (world 23):

| Kind | Range | Sub-blocks |
|---|---|---|
| content_chains | 23000-23499 | travel 23000-23019; 1423 23020-23039; 381 23040-23069; 473 23070-23099; 478 23100-23129; 1425 23130-23159; 1426 23160-23189; 1427 23190-23219; 1500 23220-23239; 1502 23240-23279; 1503 23280-23309; 481 and beam 23310-23339; 420 23340-23359; 421 23360-23389; 1147 23390-23404; 443 23405-23434; 477 23435-23459; 479 23460-23469; reserved 23470-23479 (UB-40 ships no chains); authored 23480-23499 |
| spawnlist | 23000-23199 | hub 23000-23059; Asgard 23060-23109; Nine Peaks 23110-23159; life 23160-23199 |
| point_sets | 23000-23099 | hub 23000-23029; Asgard 23030-23059; Nine Peaks 23060-23079; travel 23080-23099 |
| point_set_points | 23000-23299 | hub 23000-23099; Asgard 23100-23179; Nine Peaks 23180-23259; travel 23260-23299 |
| entity_templates | 2300-2349 | NPCs 2300-2319 (cast 2300-2307, 2303 reserved; life 2308-2309, UB-40); objects 2320-2349 (2320-2335 used, 2327 and 2336 reserved) |
| respawners | re-author 6 and 7 in place; any new ones 2300-2309 | |
| ring_transport_regions | 2300-2309 | the ring pair 2300/2301 (UB-02d, D-UB4 = A) |

Authored rows (UB-50 only, HELD), at most 65535 and outside the pak range 38-1826, next to Dakara 63500-63799:

| Kind | Range | Kind | Range |
|---|---|---|---|
| missions | 64000-64019 | dialog_sets | 64300-64309 |
| steps | 64020-64099 | screens | 64124-64193 |
| objectives | 64100-64179 | buttons | 64380-64419 |
| tasks | 64180-64259 | dialog_set_maps | 64420-64459 |
| dialogs | 64260-64299 | texts | 64460-64499 |

UB-00c re-checked the block against every branch and sibling ledger, and the coordinator approved it (D-UB9). ARCH-B1 re-carved the screens from 64310-64379 to 64124-64193: stock screens 64341 (dialog 2002) and 64373-64375 (dialog 3122) sit in the old range. `screen_id` is the second column of `dialog_screens`, and the UB-00c head-of-tuple scan only read the first. Every other authored table keys its id in the first column.

## Omega receiver contract

Beta consumes these states. UB-00c freezes this table in `docs/analysis/union-beta-e1-continuation/receiver-contract.md`. The ids below are Omega's current packet ids (Omega's 2026-10-18 renumbering); an older Beta draft cited UO-02/03/06 and "UO-07/08 fixtures".

**Fixture ownership (settled).** Omega's **UO-13** exports the *departure* fixtures (`human_departure`, `jaffa_departure` in `chain_replay_tests/union_omega/handoff.rs`): Omega's exact output state. Beta's **UB-10, UB-20 and UB-30** build the *receiver* fixtures: live-DB tests that stage each inbound state (written out in the table) and assert that Beta's receiving chain resolves. Beta's wave B2 can run before UO-13 (Omega W4) merges, so Beta stages the state itself and never imports `handoff`; it may switch to the import once UO-13 is on `main`. Both sides assert the same table row, so a drift shows up as a failing test on whichever side changes first.

| Race | Omega producer | State on arrival at Beta | Beta's receiving chain | Beta's first talk step |
|---|---|---|---|---|
| Human | UO-07 (Jackson 4641, chains 63820-63822) | 1655 completed; 1423 accepted; S4223 active; gate 6 granted | 23020: `advance_step 1423 → 4224` | S4224 Carter, dialog 4643 (UB-10) |
| Free Jaffa | UO-08 (Bra'tac 5025, chains 63830-63832) | 1655 completed; 1500 accepted; S4369 active; gate 6 granted | 23220: `advance_step 1500 → 4370` | S4370 Carter, dialog 5026 (UB-20) |
| Asgard | UO-16 (stub, **HELD**: U-D08/U-D11; the Asgard producer, Thor 1913/1912 and item 3399, does not exist yet) | 481 accepted; S1760 active; item 3399 held. Beta's side is built against this state; no Omega packet delivers it. | 23310: `advance_step 481 → 1761` | S1761 Carter, dialog 1915 (UB-30) |

**HELD, exact question (Asgard producer).** Which Omega packet delivers an Asgard player to Beta with 481 active at S1760 and item 3399 held? UO-16 is a BlockedDecision stub waiting on the owner's U-D08/U-D11 (the Asgard route into Omega, the Council, item 3399). Until it is answered, UB-30 to UB-36 are still dispatched, coded and replay-tested with archetype 5 (D-UB13); they are not end-to-end reachable and do not count as route completion.

Mismatches for the Omega architect: M1 (D-UB2), M2 (4643's mid-step flag; Beta displays the dialog by chain, so the glyph is cosmetic), M4 (D-UB3), M6/M8 (D-UB5), M7 (446 is LATER; never on Beta past S5030), HELD-B1b (answered: Beta removes 3399 at S1761, on the O2079 hand-in; `mission-seal-held.md` Q7).

## Spawn tag contract

Chains key on these tags. The spawn packets create them.

| Tag | Template | Packet |
|---|---|---|
| `BetaE1_Carter` | 2300 Lt. Col. Carter (name_id 7366) | UB-02b |
| `BetaE1_Felger` | 2301 Felger (7368; `display_name` 'Dr. Felger') | UB-02b |
| `BetaE1_Satterfield` | 2302 Satterfield (7373) | UB-02b |
| `BetaE1_Debeaux` | 2304 Lt. Debeaux (8552) | UB-02b |
| `BetaE1_Mimir` | 2305 Mimir (7388) | UB-02c |
| `BetaE1_RasNor` | 2306 Ras'nor (7374) | UB-02d |
| `BetaE1_SickJaffa_1..3` | 2307 Sick Nine Peaks Warrior (7414) | UB-02d |
| object tags `BetaE1_<Object>[_n]` | 2320-2336 (UB-02a table) | UB-02b/c/d |
| `BetaSite_00000007`, `BetaSite_Beam_000007_Ship` | existing spawns 149 and 151 (template 3) | UB-30 |

Template 2303 (Dr. Pierce, 7905) stays reserved and is never spawned: D-UB7 keeps the twins out, and UB-52 is closed.

## Packets

| Packet | Role | Size | Deliverable | Status | File |
|---|---|---|---|---|---|
| UB-00a1 | `mission-systems-advisor` | docs, ~30k | Mission seal part 1 (Q1-Q6): per-step matrix, dsm/button/linter table, en=false verdict, F19 trap, quotas, type-4 buttons; change rows C1-C6 | Review (pass 2: the R3 finding C6) | [UB-00](beta-packets/UB-00.md) |
| UB-00a2 | `mission-systems-advisor` | docs, ~30k | Mission seal part 2 (Q7-Q14): HELD-B1/B1b/B5/B7/B9/B14/B15 verdicts, M2, object-mesh census, mission rows; a2 changes 1-6 | Integrated | [UB-00](beta-packets/UB-00.md) |
| UB-00b | `movement-teleport-advisor` | docs + nav probes, ~35k | Placement seal: probed anchors, respawners, beam decks, west-valley verdict | Integrated | [UB-00](beta-packets/UB-00.md) |
| UB-00c | `documentation-writer` | docs, ~25k | Continuation ledger, ID re-check, receiver contract, amendment request | Integrated | [UB-00](beta-packets/UB-00.md) |
| UB-01a | `packet-coder` | ~40k | Pilot: gate 6 pin, respawners 6/7, exemption removed, gate guard tests | Ready (B1a) | [UB-01a](beta-packets/UB-01a.md) |
| UB-02a | `packet-coder` | ~40k | Templates 2300-2335 (2303, 2327, 2336 reserved), with the Q10 mesh columns | Ready (B1a) | [UB-02a](beta-packets/UB-02a.md) |
| UB-DP | `packet-coder` | ~38k | Beta dialog patches: Accept onto the final screen of 1997 and 3228 (C5), undrawable More Info stripped from 1916 and 3905 (C6) | Ready (B1a) | [UB-DP](beta-packets/UB-DP.md) |
| UB-01b | `packet-coder` | ~38k | Way-back chains 23001-23003 (Omega address), shared Beta test helpers, linter split (F20) | BlockedDependency UB-01a (B1b) | [UB-01b](beta-packets/UB-01b.md) |
| UB-02b | `packet-coder` | ~35k | Hub/command spawns and regions (Human cast and objects) | BlockedDependency UB-02a (B1b) | [UB-02bcd](beta-packets/UB-02bcd.md) |
| UB-02c | `packet-coder` | ~35k | Asgard base, infirmary and fire-site spawns and regions | BlockedDependency UB-02b (stack, B1b) | [UB-02bcd](beta-packets/UB-02bcd.md) |
| UB-02d | `packet-coder` | ~40k | Nine Peaks camp and the ring pair 2300/2301 (D-UB4 = A) | BlockedDependency UB-02b (stack, B1b) | [UB-02bcd](beta-packets/UB-02bcd.md) |
| UB-10 | `packet-coder` | ~35k | 1423 receiver and Carter (Human fixture) | BlockedDependency B1b (UB-01b, UB-02b) | [UB-10-13](beta-packets/UB-10-13.md) |
| UB-11 | `packet-coder` | ~40k | 381 Closing Shop | BlockedDependency UB-10 (stack) | [UB-10-13](beta-packets/UB-10-13.md) |
| UB-12 | `packet-coder` | ~40k | 473 Inventory | BlockedDependency UB-11 (stack) | [UB-10-13](beta-packets/UB-10-13.md) |
| UB-13 | `packet-coder` | ~35k | 478 Instant IT | BlockedDependency UB-12 (stack) | [UB-10-13](beta-packets/UB-10-13.md) |
| UB-14 | `packet-coder` | ~35k | 1425 SGU Database, to S4244 | BlockedDependency UB-13 | [UB-14-16](beta-packets/UB-14-16.md) |
| UB-15 | `packet-coder` | ~35k | 1426 Amateur Photography (camera item 4832) | BlockedDependency UB-14 (stack) | [UB-14-16](beta-packets/UB-14-16.md) |
| UB-16 | `packet-coder` | ~30k | 1427 The Ruins, to S4250 | BlockedDependency UB-15 (stack) | [UB-14-16](beta-packets/UB-14-16.md) |
| UB-20 | `packet-coder` | ~30k | 1500 receiver and Carter (Free Jaffa fixture) | BlockedDependency B1b (UB-01b, UB-02b) | [UB-20-22](beta-packets/UB-20-22.md) |
| UB-21 | `packet-coder` | ~40k | 1502 The Nine Peaks Jaffa | BlockedDependency UB-20, UB-02d | [UB-20-22](beta-packets/UB-20-22.md) |
| UB-22 | `packet-coder` | ~30k | 1503 The Antidote, S4380 to S4381 | Hold: not dispatched (D-UB5 A) | [UB-20-22](beta-packets/UB-20-22.md) |
| UB-30 | `packet-coder` | ~40k | 481 receiver, Carter, beam, Mimir (Asgard fixture) | BlockedDependency B1b (UB-01b, UB-02b, UB-02c). End-to-end reachability waits on HELD-B16 (owner); the packet does not | [UB-30-36](beta-packets/UB-30-36.md) |
| UB-31 | `packet-coder` | ~35k | 420 Reclamation | BlockedDependency UB-30, UB-DP | [UB-30-36](beta-packets/UB-30-36.md) |
| UB-32 | `packet-coder` | ~35k | 421 Mechanical Error | BlockedDependency UB-31 | [UB-30-36](beta-packets/UB-30-36.md) |
| UB-33 | `packet-coder` | ~25k | 1147 Intriguing Data | BlockedDependency UB-32 (stack), UB-DP | [UB-30-36](beta-packets/UB-30-36.md) |
| UB-34 | `packet-coder` | ~40k | 443 Incendiary | BlockedDependency UB-33 (stack) | [UB-30-36](beta-packets/UB-30-36.md) |
| UB-35 | `packet-coder` | ~40k | 477 Covert Observations (four-subject partition) | BlockedDependency UB-34 (stack), UB-DP | [UB-30-36](beta-packets/UB-30-36.md) |
| UB-36 | `packet-coder` | ~25k | 479 Process Support (Analytical Machine, Consciousness Simulator) | BlockedDependency UB-35 (stack), UB-DP | [UB-30-36](beta-packets/UB-30-36.md) |
| UB-40 | `packet-coder` | ~30k | World life: SGC Personnel (7450) and Jaffa non-combatants (7409), templates 2308/2309, spawns 23160-23199, no dialog | BlockedDependency B1b (UB-02a to UB-02d) | [UB-40-52](beta-packets/UB-40-52.md) |
| UB-45a | `packet-coder` | ~35k | Cross-race guard 1: seed-derived walk states, one dialog per click, race negatives | BlockedDependency UB-16, UB-21, UB-36, UB-40 | [UB-40-52](beta-packets/UB-40-52.md) |
| UB-45b | `testing-validation-engineer` | ~40k | Cross-race guard 2: login binds equal in-session binds on every walk transition | BlockedDependency UB-45a (stack) | [UB-40-52](beta-packets/UB-40-52.md) |
| UB-50 | `packet-coder` | skeleton | Authored relays/rewards (droppable; not needed for the first ship) | BlockedDecision (owner: H-LBL, U-D07; CB-N05a) | [UB-40-52](beta-packets/UB-40-52.md) |
| UB-51 | `mission-systems-advisor` | docs, ~20k | Offworld-tail handoff notes for Omega/SGC (D-UB5 B) | Hold (D-UB5 A) | [UB-40-52](beta-packets/UB-40-52.md) |
| UB-52 | none | closed | FV twins 383/384/1146 | Closed (D-UB7: out) | [UB-40-52](beta-packets/UB-40-52.md) |
| UB-90 | `lab-driver` steps, owner approval | lab UAT | Rows UB-U1 to UB-U10 | BlockedDependency B5 merged; owner approves lab use | [UB-90-99](beta-packets/UB-90-99.md) |
| UB-99 | `documentation-writer` | docs, ~30k | Close-out: content docs, unified UAT, ledger, status docs | BlockedDependency all | [UB-90-99](beta-packets/UB-90-99.md) |
| UB-L | none | hold | Visit 2 / E2 research note | Hold | [UB-90-99](beta-packets/UB-90-99.md) |
Every chain packet also follows [chain-packet-common.md](beta-packets/chain-packet-common.md): its rules, test skeleton, lane commands and telemetry list.

Spawn and region seeds are split per area (`spawnlist_beta_e1_{hub,asgard,ninepeaks,life}.sql` and matching point-set files), so the spawn packets never share a seed file. The FireSite region is point set 23031 (UB-02c), which UB-34 and UB-40 use. Resume card: [beta-packets/RESUME.md](beta-packets/RESUME.md).

## Waves

| Wave | Packets | Gate |
|---|---|---|
| B0 (pilot) | UB-00a1, UB-00a2, UB-00b, UB-00c | Integrated (UB-00a1 review pass 2 open; it blocks nothing below, because ARCH-B1 already folded C1-C6 into the packets) |
| B1a | UB-01a, UB-02a, UB-DP | Ready. Three independent worktrees off `uzp-e1`, all in parallel; one review, then merge into `uzp-e1` |
| B1b | UB-01b, UB-02b in parallel; then UB-02c and UB-02d stacked on `uzp-ub02b` | UB-01a and UB-02a merged (UB-DP may merge late, but before B3) |
| B2 | UB-10 to UB-13 (one stack), UB-20, UB-30 | B1b merged. Three independent lanes (Human, Free Jaffa, Asgard) in parallel |
| B3 | UB-14 to UB-16 (one stack), UB-21, UB-31, UB-40 | B2 merged and UB-DP merged. Four lanes in parallel |
| B4 | UB-32 to UB-36 (one stack) | B3 merged. One review-fixer reviews the stack in order |
| B5 | UB-45a, then UB-45b (stacked) | B4 merged |
| B6 | UB-90 (lab), then UB-99 | B5 merged; the owner approves lab use before UB-90 |

Waves run in order; inside a wave every lane runs in parallel. B1a must merge before B1b because UB-01b and UB-02b build on UB-01a and UB-02a. From B2 to B4 the three race lanes are independent, but each wave gets exactly one packet-review-fixer pass, so a lane's next packet waits for its wave's review and merge. UB-22 (D-UB5 A), UB-51 (held) and UB-52 (closed) are never dispatched; UB-50 waits on the owner.

**Naming (ARCH-B1).** Worktree name = branch name = `uzp-ub<id>`, lowercase with no inner dash: `uzp-ub01a`, `uzp-ub02a`, `uzp-ubdp`, `uzp-ub10`, `uzp-ub45b`. Each branches from `uzp-e1`, except a stacked packet, which branches from its parent's branch. Reports go to `<worktree>\.claude\reports\union-beta-e1\<packet>-coder.md`.
Stacked packets inside a wave touch different seed files and test modules. They share only `db/database.sql`, `chain_replay_tests/mod.rs`, the two linter files and `content-chains.instructions.md`, and each adds its own line. Expect a merge conflict on those lines and keep both sides.

## HELD items

| ID | Question | Status and answer | Blocks |
|---|---|---|---|
| HELD-B1 | Human order after 1423 (381, then 473, then 478 is inferred from labels and levels) | **Answered**: confirmed (`mission-seal-held.md` Q7); lab walk UB-U3c | none |
| HELD-B1b | Which 481 task consumes item 3399 | **Answered**: removed at S1761 on the O2079 hand-in, chain 23312 as written (`mission-seal-held.md` Q7) | none |
| HELD-B2 | Where Heimdall (1503 S4381) and Bra'tac (S4379) are | Moot under D-UB5 A (UB-22 not dispatched) | UB-22 if revived |
| HELD-B3 | The 421 object/region binding inside EM-Infirmary01 | **Answered**: INFIRMARY_CENTRE (961.300, 15.97, 284.900), r 10, h 6; both objects inside (`placement-seal.md`) | none |
| HELD-B4 | Woolsey (1427 S4250) and Jackson (1425 S4244) receivers | **Ruled**: none on Beta (D-UB5 A); the missions stay open at those steps | none |
| HELD-B5 | 1425's data item is missing from the seed | **Answered**: item 4833 exists but no step consumes it; interact-only steps, no item (`mission-seal-held.md` Q8) | none |
| HELD-B6 | Are 383/384/1146 first-visit content? | **Ruled**: no (D-UB7); UB-52 closed | none |
| HELD-B7 | The teleporter is the beam (D-UB12); the two beam destinations | **Answered**: BEAM_DECK_DEST (698.98, 157.93, 70.36), BEAM_GROUND_DEST (745.05, -19.60, 68.75) (`placement-seal.md` § beam; `mission-seal-held.md` Q8, Q14) | none |
| HELD-B8 | Debeaux position (477 O2061 names the "Traning Range"); Pierce only for UB-52 | **Answered**: sealed at the training range (`placement-seal.md` Q4); Pierce moot (D-UB7) | none |
| HELD-B9 | Which camera item serves 1426 (2766/4830/4832), or none | **Answered**: item 4832, granted on accept and removed on upload (`mission-seal-held.md` Q8) | none |
| HELD-B10 | Who fires the arrival steps | **Answered**: Beta (D-UB2, confirmed by Omega's D-UO2). The Asgard leg's producer is HELD-B16 | none |
| HELD-B12 | What the executor does on `en=false` steps | **Answered**: works; the server never reads the flag (`mission-seal.md` Q3); journal display is UB-U1b | none |
| HELD-B13 | Reward amounts | **Owner** (U-D07). The first ship keeps xp 0 / naq 0 (D-UB8) | UB-50 |
| HELD-B14 | Mission 443 S1484: is there a transmitter dialog for 443? | **Answered**: none; the step update is the feedback (`mission-seal-held.md` Q13) | none |
| HELD-B15 | Mission 477 subject bind: is there an interaction-only dsm for 477? | **Answered**: none; keep dsm 2457 (`mission-seal-held.md` Q12); live check UB-U5c | none |
| HELD-B16 | Which Omega packet delivers an Asgard player to Beta with 481 active at S1760 and item 3399 held? | **Owner** (U-D08/U-D11, UO-16). BlockedDecision | Asgard end-to-end reachability only; UB-30 to UB-36 still dispatch (D-UB13) |
| H-LBL | Which label does player-facing CIMMERIA_AUTHORING copy carry on Beta? | **Owner**. BlockedDecision | UB-50 |
| H-RING | Ring routing (D-UB4) | **Ruled**: A, the ring pair (ARCH-B1) | none |
| H-421 | The brief held 421 pending infirmary proof; F6 supplies it | **Closed**: D-UB6 confirmed | none |
## Rejected additions

| Addition | Reason |
|---|---|
| Physical Jackson, Bra'tac, Thor, Heimdall, Woolsey or High Council actors on Beta | They live on Omega or the SGC. Placing them would fake an offworld step (D-UB5; stop condition) |
| Seeding the underground complex (1047/1060) or its four consoles | It can be reached only by ring, the ring routing is lost, and no first-visit step needs it (F8) |
| A fire that goes out | The emitters are static and nothing toggles them (F18) |
| Hatak attack encounter | An FX anchor in the sky only; no ship mesh, no seed, and no first-visit step |
| Seeding the old-build seventh ring or the whirlcam teleporter | Older builds only (placement manifest §10.2) |
| Beta re-offering 1423/1500/481 | Omega owns the offers (D-UB3) |
| 446 past S5030, 402, 419, 449, 458-461, 854, 1300, 1504, 1525 | LATER content; outside the first visit |
| Vendors or trainers | No map or source evidence |
| Reward amounts | U-D07 is open (D-UB8) |

## Provenance labels

Every seed row or doc claim a Beta packet adds carries one label in a comment:

- **RESTORED_SOURCE:** rows from the seeds or the client: missions, steps, dialogs, dsm rows, items, text monikers, map pins (PlayerStart, buildings, beam bases).
- **CIMMERIA_RECONSTRUCTION:** glue that restores behaviour the source implies: every chain, the arrival pin, respawner positions, NPC placements against building pins, the beam move, the way-back grant, any ring pair.
- **CIMMERIA_AUTHORING:** new content with no source. It lives only in UB-50 and its own seed file.

## Stop conditions

Stop the Beta work if:

- a race receives another race's `The SGU` mission or completes another race's step;
- a `03_All_Beta_Missions` locality label is the only world proof for a chain;
- the gate, beam or ring cannot return a player (the way-back guard fails);
- a physical Council or Omega objective is credited on Beta;
- E2 (world 60) or Visit 2 (1807-1826) state leaks into first-visit chains;
- a packet needs to edit an Omega, SGC or Dakara file, or `id-allocation.md` (the D-UB9 row was applied once by ARCH-B1 for the coordinator).
