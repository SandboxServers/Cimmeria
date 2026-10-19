# Beta Site E1 Union work packets

> **Type:** How-to (dispatch contract)
> **Audience:** Union Beta coordinator, Omega and SGC architects, packet workers and reviewers
> **Last updated:** 2026-10-18 (architect pass against `origin/main` @ `d1bba0400`, an ancestor of current `main`; locate each anchor by its text, not its line number)
> **Status:** Planning. Nothing dispatched. Wave B0 (pilot) is gated on U-D03 (see [Waves](#waves)).
> **Companions:** [Beta E1 campaign](beta.md), [Union execution contract](execution-contract.md), [Omega packets](omega-work-packets.md), [route evidence](evidence.md), [Asgard route](asgard-route.md), [Free Jaffa route](free-jaffa-route.md), [packet folder](beta-packets/)

## Short answer

Beta_Site_Evo_1 (world 23, gate 6) has a sealed map and almost nothing on the server side. There are no NPCs, no content chains, no arrival pin, and two respawners at the origin. Every Beta dialog has `accepts_mission_id` NULL, so each grant needs a chain. This campaign builds the **first visit** for the three Union races:

1. **Safe travel first (pilot).** Gate 6 gets an arrival pin at the map's only PlayerStart. Respawners 6 and 7 get real positions, and the exemption in `live_db_loaders.rs` is removed. A player who arrives on a Union mission learns the way back to Omega (UB-01a, UB-01b).
2. **Cast and anchors.** Beta-only templates 2300-2349 use the client's own display-name strings (RESTORED_SOURCE monikers in `texts.sql`). Spawns are placed against map pins that UB-00b re-probes on the navmesh (UB-02a to UB-02d).
3. **Three race arcs, one mission per packet**, each with its own chain range and seed file:
   - Human: 1423 → 381 → 473 → 478 → 1425 → 1426 → 1427.
   - Free Jaffa: 1500 → 1502 → 1503.
   - Asgard: 481 → 420 → 421 → 1147, plus 443, 477 and 479.

   Each arc stops at its last **Beta-local** step. Offworld tails wait on D-UB5.
4. **Mission 421 is no longer held for locality.** The infirmary is Beta-local with high confidence (F6). Only its object binding is left, and UB-00b seals it.
5. **Authored content is a separate packet you can drop (UB-50).** It is HELD on the label policy, U-D07 and the CB-N05a client proof.

## What other campaigns own (do not touch)

- **Omega (UO-xx)** offers 1423, 1500 and 481. It issues item 3399 (UO-16, HELD on U-D08/U-D11). It owns every Omega-side chain, plus any Jackson, Bra'tac, Thor, Heimdall or High Council actor. Beta never re-offers those missions (D-UB3).
- **SGC (US-xx)** owns Woolsey and Hammond on SGC_W1, and 1180.
- **Dakara (UD/DK)** owns chains 8001-8499 and 63500-63549, templates 440-479 and the Dakara ID block 63500-63799.
- `docs/analysis/four-zone-planning/id-allocation.md` belongs to the coordinator. Beta files a request for an amendment row (UB-00c) and never edits the file.
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
| F7 | Nav components 251 (gate, east valley) and 70 (west valley, the Jaffa camps) have no link of 3 m or less. West-valley access is UNSEALED. | placement manifest §9 | UB-00b, D-UB4 |
| F8 | The Asgard vessel deck (688.2, 151.6, 70.3) is nav island 355 and can be reached only by beam. The Science Facility interior (nav 890) joins 251 across a 0.90 m door gap. The underground complex (1047/1060) can be reached only by ring, so seed nothing there. | placement manifest §9 | UB-02c, UB-30 |
| F9 | `texts.sql` holds the client's own monikers for Beta NPCs and objects: 7366 Carter, 7367 Heintz, 7368 Felger (empty text), 7373 Satterfield, 7374 Ras'nor, 7388 Mimir, 7414 Sick Nine Peaks Warrior, 7905 Pierce, 8552 Debeaux; objects 7376, 7377, 7383, 7392, 7397, 7380, 7381, 7402, 7909, 7957, 8164, 8211, 8489, 8490, 8493, 8497. | `db/resources/Texts/Seed/texts.sql` | UB-02a |
| F10 | `entity_templates.sql` has no row for Felger, Satterfield, Mimir, Ras'nor, Pierce, Debeaux or the sick Jaffa. The cooked client has generic heads plus Carter, Woolsey, Jackson, Hammond and Davis. Mimir uses `BS_Asgard` as Thor (template 64) does. Ras'nor uses `BS_JaffaMale` as Dakara 440 does. | client package census | UB-02a |
| F11 | Every Beta mission row has xp 0 and naq 0 (the AwardXP flag only). Rewards are U-D07. | `missions.sql`, P4 reward audit | D-UB8 |
| F12 | The SGU-family steps (1423, 1425-1427, 1500, 1502-1504) all carry `en=false`. Executor behaviour on a disabled step is unverified. | mission manifest finding 8 | UB-00a (HELD-B12) |
| F13 | There are two flag dialects in Beta `dialog_set_maps`. **Old:** 8388608 blurb, 16777216 story, 33554432 turn-in, 1073741824 object. **New (SGU):** 134217728 offer blurb, 0 offer, 268435456 mid, 536870912 completion. Never mix them in one bind. | mission manifest §4 | all chain packets |
| F14 | `add_dialog_set`'s `slot` is the NPC's **template id**, and the bind is per player (`executor/dialog/mod.rs:166-176`). Beta-only templates keep Beta binds from lighting up Carter or Thor on other worlds. | castle_701 chain 1201 (`slot 149` = Gerschon) | UB-02a, D-UB10 |
| F15 | Main has no phasing primitive. Tollana PH-01a/b are unmerged and `set_phase` does not exist. A shared NPC cannot be hidden or changed for one player. | `git grep set_phase` empty | D-UB11 |
| F16 | Asgard char-creation row 9 is `NON_CANONICAL_BLOCKED_LEGACY` and starts in SGC_W1. The Asgard arc on Beta can be tested by replay, but a player can reach it only after Omega UO-16 (BlockedDecision U-D08/U-D11) delivers it. | `char_creation.sql`; asgard-route.md | D-UB13 |
| F17 | Mismatches with the Omega contract: M1 (who fires the arrival steps), M2 (4643 uses the mid-step flag), M4 (Beta must not re-offer 481), M6 (Heimdall S4381), M7 (446 S1496/S1497), M8 (1425 S4244 Jackson, 1427 S4250 Woolsey), M9 (S4224, S4370 and S1761 are Beta-owned). | mission manifest §9 | [Omega contract](#omega-receiver-contract) |
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
| D-UB4 | West-valley access for the Nine Peaks camp. **A (recommended if UB-00b finds no walk route):** pair ring 148 (component 251) with a new SW ring row (component 70) as a `ring_transport_regions` pair, CIMMERIA_RECONSTRUCTION. **B:** a walk route that UB-00b's gap scan finds; no seed is needed. **C:** hold 1502/1503 until a lab path test. | Owner (ring routing is a design call) | **BlockedDecision** |
| D-UB5 | Offworld tails: 1425 S4244 (Jackson), 1427 S4250 (Woolsey), 1503 S4379 (Bra'tac) and S4381 (Heimdall). **A (default the packets implement):** stop each arc at its last Beta-local step; the mission stays open in the journal. **B:** Omega/SGC receivers, owned by those architects. **C:** authored Beta relays (UB-50, CIMMERIA_AUTHORING). | Owner | **BlockedDecision** (A ships unless the owner picks B or C) |
| D-UB6 | 421 is un-held: BETA_LOCAL (F6). UB-32 dispatches once UB-00b seals the object positions. | Architect; orchestrator confirms against the brief's hold | Decided (confirm) |
| D-UB7 | The FV twins 383, 384 and 1146 are not in P4 rows 121-124. Recommendation: leave them out of the first ship. | Owner | **BlockedDecision** (HELD-B6) |
| D-UB8 | No reward amounts. Missions keep xp 0 / naq 0 until U-D07. | Owner | **BlockedDecision** |
| D-UB9 | ID block: world-prefix ranges for world rows, and 64000-64499 for authored rows (below). The content-chain block 23000-23499 needs a coordinator amendment row in `id-allocation.md`. | Coordinator | **Pending** (UB-00c files the request) |
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
| ring_transport_regions | 2300-2309 (only if D-UB4 = A) | |

Authored rows (UB-50 only, HELD), at most 65535 and outside the pak range 38-1826, next to Dakara 63500-63799:

| Kind | Range | Kind | Range |
|---|---|---|---|
| missions | 64000-64019 | dialog_sets | 64300-64309 |
| steps | 64020-64099 | screens | 64310-64379 |
| objectives | 64100-64179 | buttons | 64380-64419 |
| tasks | 64180-64259 | dialog_set_maps | 64420-64459 |
| dialogs | 64260-64299 | texts | 64460-64499 |

The block is valid only after UB-00c re-checks it against every branch and sibling ledger, and the coordinator merges the amendment.

## Omega receiver contract

Beta consumes these states. UB-00c freezes this table in `docs/analysis/union-beta-e1-continuation/receiver-contract.md`. The ids below are Omega's current packet ids (Omega's 2026-10-18 renumbering); an older Beta draft cited UO-02/03/06 and "UO-07/08 fixtures".

**Fixture ownership (settled).** Omega's **UO-13** exports the *departure* fixtures (`human_departure`, `jaffa_departure` in `chain_replay_tests/union_omega/handoff.rs`): Omega's exact output state. Beta's **UB-10, UB-20 and UB-30** build the *receiver* fixtures: live-DB tests that stage each inbound state (written out in the table) and assert that Beta's receiving chain resolves. Beta's wave B2 can run before UO-13 (Omega W4) merges, so Beta stages the state itself and never imports `handoff`; it may switch to the import once UO-13 is on `main`. Both sides assert the same table row, so a drift shows up as a failing test on whichever side changes first.

| Race | Omega producer | State on arrival at Beta | Beta's receiving chain | Beta's first talk step |
|---|---|---|---|---|
| Human | UO-07 (Jackson 4641, chains 63820-63822) | 1655 completed; 1423 accepted; S4223 active; gate 6 granted | 23020: `advance_step 1423 → 4224` | S4224 Carter, dialog 4643 (UB-10) |
| Free Jaffa | UO-08 (Bra'tac 5025, chains 63830-63832) | 1655 completed; 1500 accepted; S4369 active; gate 6 granted | 23220: `advance_step 1500 → 4370` | S4370 Carter, dialog 5026 (UB-20) |
| Asgard | UO-16 (stub, **HELD**: U-D08/U-D11; the Asgard producer, Thor 1913/1912 and item 3399, does not exist yet) | 481 accepted; S1760 active; item 3399 held. Beta's side is built against this state; no Omega packet delivers it. | 23310: `advance_step 481 → 1761` | S1761 Carter, dialog 1915 (UB-30) |

**HELD, exact question (Asgard producer).** Which Omega packet delivers an Asgard player to Beta with 481 active at S1760 and item 3399 held? UO-16 is a BlockedDecision stub waiting on the owner's U-D08/U-D11 (the Asgard route into Omega, the Council, item 3399). Until it is answered, UB-30 can be coded and replay-tested with archetype 5 (D-UB13), but it is not end-to-end reachable and does not count as route completion.

Mismatches for the Omega architect: M1 (D-UB2), M2 (4643's mid-step flag; Beta displays the dialog by chain, so the glyph is cosmetic), M4 (D-UB3), M6/M8 (D-UB5), M7 (446 is LATER; never on Beta past S5030), HELD-B1b (which 481 task consumes 3399: Beta removes it at S1761 unless the seal says otherwise).

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

Template 2303 (Dr. Pierce, 7905) is reserved for UB-52 (twin 383) and never spawned before D-UB7.

## Packets

| Packet | Role | Size | Deliverable | Status | File |
|---|---|---|---|---|---|
| UB-00a1 | `mission-systems-advisor` | docs, ~30k | Mission seal part 1 (Q1-Q6): per-step matrix, dsm/button/linter table, en=false verdict, F19 trap, quotas, type-4 buttons | BlockedDependency (U-D03) | [UB-00](beta-packets/UB-00.md) |
| UB-00a2 | `mission-systems-advisor` | docs, ~30k | Mission seal part 2 (Q7-Q14): HELD-B1/B1b/B5/B7/B9/B14/B15 verdicts, M2, object-mesh census, mission rows | BlockedDependency (U-D03) | [UB-00](beta-packets/UB-00.md) |
| UB-00b | `movement-teleport-advisor` | docs + nav probes, ~35k | Placement seal: probed anchors, respawners, beam decks, west-valley verdict | BlockedDependency (U-D03) | [UB-00](beta-packets/UB-00.md) |
| UB-00c | `documentation-writer` | docs, ~25k | Continuation ledger, ID re-check, receiver contract, amendment request | BlockedDependency (U-D03) | [UB-00](beta-packets/UB-00.md) |
| UB-01a | `packet-coder` | ~40k | Pilot: gate 6 pin, respawners 6/7, exemption removed, gate guard tests | BlockedDependency UB-00c (ID row) | [UB-01a](beta-packets/UB-01a.md) |
| UB-01b | `packet-coder` | ~38k | Way-back chains 23001-23003 (Omega address), shared Beta test helpers, linter split (F20) | BlockedDependency UB-01a | [UB-01b](beta-packets/UB-01b.md) |
| UB-02a | `packet-coder` | ~40k | Templates 2300-2335 (2303, 2327, 2336 reserved) | BlockedDependency UB-00a2 | [UB-02a](beta-packets/UB-02a.md) |
| UB-02b | `packet-coder` | ~35k | Hub/command spawns and regions (Human cast and objects) | BlockedDependency UB-02a, UB-00b | [UB-02bcd](beta-packets/UB-02bcd.md) |
| UB-02c | `packet-coder` | ~35k | Asgard base, infirmary and fire-site spawns and regions | BlockedDependency UB-02a, UB-00b | [UB-02bcd](beta-packets/UB-02bcd.md) |
| UB-02d | `packet-coder` | ~35k | Nine Peaks spawns and regions (plus a ring pair if D-UB4 = A) | BlockedDecision D-UB4 | [UB-02bcd](beta-packets/UB-02bcd.md) |
| UB-10 | `packet-coder` | ~35k | 1423 receiver and Carter (Human fixture) | BlockedDependency UB-02b, D-UB2 | [UB-10-13](beta-packets/UB-10-13.md) |
| UB-11 | `packet-coder` | ~40k | 381 Closing Shop | BlockedDependency UB-10 | [UB-10-13](beta-packets/UB-10-13.md) |
| UB-12 | `packet-coder` | ~40k | 473 Inventory | BlockedDependency UB-11 | [UB-10-13](beta-packets/UB-10-13.md) |
| UB-13 | `packet-coder` | ~35k | 478 Instant IT | BlockedDependency UB-12 | [UB-10-13](beta-packets/UB-10-13.md) |
| UB-14 | `packet-coder` | ~35k | 1425 SGU Database, to S4243 | BlockedDependency UB-13, HELD-B12 | [UB-14-16](beta-packets/UB-14-16.md) |
| UB-15 | `packet-coder` | ~35k | 1426 Amateur Photography | BlockedDependency UB-14, HELD-B9 | [UB-14-16](beta-packets/UB-14-16.md) |
| UB-16 | `packet-coder` | ~35k | 1427 The Ruins, to S4249 | BlockedDependency UB-15 | [UB-14-16](beta-packets/UB-14-16.md) |
| UB-20 | `packet-coder` | ~30k | 1500 receiver and Carter (Free Jaffa fixture) | BlockedDependency UB-02b, D-UB2 | [UB-20-22](beta-packets/UB-20-22.md) |
| UB-21 | `packet-coder` | ~40k | 1502 The Nine Peaks Jaffa | BlockedDependency UB-02d, UB-20 | [UB-20-22](beta-packets/UB-20-22.md) |
| UB-22 | `packet-coder` | ~40k | 1503 The Antidote, S4380 to S4381 (not dispatched under D-UB5 A) | BlockedDecision D-UB5 | [UB-20-22](beta-packets/UB-20-22.md) |
| UB-30 | `packet-coder` | ~40k | 481 receiver, Carter, beam, Mimir (Asgard fixture) | BlockedDependency UB-02b/c; **HELD** on the Asgard producer UO-16 (U-D08/U-D11), see § Omega receiver contract | [UB-30-36](beta-packets/UB-30-36.md) |
| UB-31 | `packet-coder` | ~35k | 420 Reclamation | BlockedDependency UB-30 | [UB-30-36](beta-packets/UB-30-36.md) |
| UB-32 | `packet-coder` | ~35k | 421 Mechanical Error | BlockedDependency UB-31, D-UB6, HELD-B3 | [UB-30-36](beta-packets/UB-30-36.md) |
| UB-33 | `packet-coder` | ~25k | 1147 Intriguing Data | BlockedDependency UB-32 (stack) | [UB-30-36](beta-packets/UB-30-36.md) |
| UB-34 | `packet-coder` | ~40k | 443 Incendiary (HELD-B14 radio) | BlockedDependency UB-33 (stack), FireSite 23031 | [UB-30-36](beta-packets/UB-30-36.md) |
| UB-35 | `packet-coder` | ~40k | 477 Covert Observations (four-subject partition) | BlockedDependency UB-34 (stack), HELD-B15 | [UB-30-36](beta-packets/UB-30-36.md) |
| UB-36 | `packet-coder` | ~25k | 479 Process Support (Analytical Machine, Consciousness Simulator) | BlockedDependency UB-35 (stack) | [UB-30-36](beta-packets/UB-30-36.md) |
| UB-40 | `packet-coder` | ~30k | World life: SGC Personnel (7450) packing up the base, templates 2308/2309, spawns 23160-23199, no dialog | BlockedDependency UB-02a/b/c, UB-00b Life anchors | [UB-40-52](beta-packets/UB-40-52.md) |
| UB-45a | `packet-coder` | ~35k | Cross-race guard 1: seed-derived walk states, one dialog per click, race negatives | BlockedDependency UB-16, UB-21, UB-36 (UB-22 if dispatched) | [UB-40-52](beta-packets/UB-40-52.md) |
| UB-50 | `packet-coder` | skeleton | Authored relays/rewards (droppable) | BlockedDecision D-UB5 C, D-UB8, H-LBL, CB-N05a | [UB-40-52](beta-packets/UB-40-52.md) |
| UB-51 | `mission-systems-advisor` | docs, ~20k | Offworld-tail handoff notes for Omega/SGC (D-UB5 B) | BlockedDecision D-UB5 | [UB-40-52](beta-packets/UB-40-52.md) |
| UB-52 | `packet-coder` | skeleton | FV twins 383/384/1146 | BlockedDecision D-UB7 | [UB-40-52](beta-packets/UB-40-52.md) |
| UB-90 | `lab-driver` steps, owner approval | lab UAT | Rows UB-U1 to UB-U9 | BlockedDependency B2-B4 merged | [UB-90-99](beta-packets/UB-90-99.md) |
| UB-99 | `documentation-writer` | docs, ~30k | Close-out: content docs, unified UAT, ledger, status docs | BlockedDependency all | [UB-90-99](beta-packets/UB-90-99.md) |
| UB-L | none | hold | Visit 2 / E2 research note | Hold | [UB-90-99](beta-packets/UB-90-99.md) |

Every chain packet also follows [chain-packet-common.md](beta-packets/chain-packet-common.md): its rules, test skeleton, lane commands and telemetry list.

Spawn and region seeds are split per area (`spawnlist_beta_e1_{hub,asgard,ninepeaks,life}.sql` and matching point-set files), so the spawn packets never share a seed file. The FireSite region is point set 23031 (UB-02c), which UB-34 and UB-40 use. Resume card: [beta-packets/RESUME.md](beta-packets/RESUME.md).

## Waves

| Wave | Packets | Gate |
|---|---|---|
| B0 (pilot) | UB-00a1, UB-00a2, UB-00b, UB-00c, UB-01a | U-D03 (or the orchestrator's waiver for docs-only UB-00*). UB-01a stacks on UB-00c's ID row |
| B1 | UB-01b, UB-02a, UB-02b, UB-02c, UB-02d | B0 merged; D-UB4 for UB-02d |
| B2 | UB-10, UB-11, UB-12, UB-13, UB-20, UB-30 | B1 merged; D-UB2 confirmed. The three receivers run in parallel; UB-11 to UB-13 stack in order on their own files |
| B3 | UB-14, UB-15, UB-16, UB-21, UB-31 (UB-22 only if D-UB5 is B or C) | B2 merged; HELD-B9 and HELD-B12 answered by UB-00a. UB-14 to UB-16 stack in order |
| B4 | UB-32, UB-33, UB-34, UB-35, UB-36, UB-40 | B3 merged. UB-32 to UB-36 are one stack (each depends on the one before); UB-40 runs in parallel. One review-fixer reviews the stack in order |
| B5 | UB-45a, UB-45b (stacked); plus UB-50, UB-51, UB-52 only after the owner decides | B4 merged. UB-50/51/52: D-UB5, D-UB7, D-UB8, H-LBL; the architect expands each skeleton before dispatch |
| B6 | UB-90, UB-99 | B4 merged (B5 if dispatched); owner approves lab use |

Stacked packets inside a wave touch different seed files and test modules. They share only `db/database.sql`, `chain_replay_tests/mod.rs`, the two linter files and `content-chains.instructions.md`, and each adds its own line. Expect a merge conflict on those lines and keep both sides.

## HELD items

| ID | Question | Who answers | Blocks |
|---|---|---|---|
| HELD-B1 | Human order after 1423 (381, then 473, then 478 is inferred from labels and levels) | `mission-systems-advisor` in UB-00a | UB-11 to UB-13 gates |
| HELD-B1b | Which 481 task consumes item 3399 | UB-00a, then the Omega architect confirms | UB-30 chain 23312 |
| HELD-B2 | Where Heimdall (1503 S4381) and Bra'tac (S4379) are | Owner via D-UB5 | UB-22 tail |
| HELD-B3 | The 421 object/region binding inside EM-Infirmary01 | `movement-teleport-advisor` in UB-00b | UB-02c, UB-32 |
| HELD-B4 | Woolsey (1427 S4250) and Jackson (1425 S4244) receivers | Owner via D-UB5; SGC and Omega architects if B | UB-14, UB-16 tails |
| HELD-B5 | 1425's data item is missing from the seed | UB-00a (an interact-only step if no item exists) | UB-14 |
| HELD-B6 | Are 383/384/1146 first-visit content? | Owner (D-UB7) | UB-52 |
| HELD-B7 | **Narrowed:** O2080 reads "Use the teleporter at the Asgard Base to teleport up to Mimir's hovering vessel", so the teleporter is the beam (D-UB12). Only the two beam destinations remain | UB-00b (seal § beam); UB-00a Q14 rechecks | UB-30 |
| HELD-B8 | Debeaux position (probe EM-WeaponRange01 at (1055.6, 14.6, 334.3): 477 O2061 names the "Traning Range"); Pierce only for UB-52 | UB-00b | UB-02b |
| HELD-B9 | Which camera item serves 1426 (2766/4830/4832), or none | UB-00a | UB-15 |
| HELD-B10 | Who fires the arrival steps | **Answered**: Beta (D-UB2, confirmed by Omega's D-UO2). Left open only for the Asgard leg, whose producer UO-16 is HELD (U-D08/U-D11) | UB-30 |
| HELD-B16 | Which Omega packet delivers an Asgard player to Beta with 481 active at S1760 and item 3399 held? | Owner (U-D08/U-D11, UO-16) | UB-30 end-to-end reachability |
| HELD-B12 | What the executor does on `en=false` steps | UB-00a (read `missions/progression.rs` plus one replay probe) | UB-10, UB-14 to UB-16, UB-20 to UB-22 |
| HELD-B13 | Reward amounts | Owner (U-D07, D-UB8) | UB-50 |
| HELD-B14 | Mission 443 S1484 "Speak with Mimir over your transmitter": is there a transmitter dialog for 443 (speaker 763)? If not, the step update is the only feedback | UB-00a Q13 | UB-34 (chain 23412) |
| HELD-B15 | Mission 477 subject bind: UB-35 binds the blurb dsm 2457 on the four subject slots by default, because `interaction_type 0` NPCs are clickable only with a live bind (F21). Is there an interaction-only dsm (dialog_id NULL) for 477 instead? | UB-00a Q12 | UB-35 |
| H-LBL | Label policy for authored player-facing copy | Owner | UB-50 |
| H-RING | Ring routing (D-UB4) | Owner | UB-02d, UB-21, UB-22 |
| H-421 | The brief held 421 pending infirmary proof; F6 supplies it | Orchestrator | UB-32 |

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
- a packet needs to edit an Omega, SGC or Dakara file, or `id-allocation.md`.
