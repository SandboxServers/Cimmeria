---
title: "Beta Site E1 (world 23)"
type: reference
audience: engineers
last_updated: 2026-10-19
---

# Beta Site E1 (world 23)

> **Last updated**: 2026-10-19 (UB-99 close-out)
> **Type**: Reference
> **Audience**: Engineers and content authors touching `Beta_Site_Evo_1`
> **Companions**: [Union Beta E1 ledger](../analysis/union-beta-e1-continuation/README.md), [packet index](../analysis/union-zone-planning/beta-work-packets.md), [mission-chains.md](mission-chains.md#beta-site-e1-world-23), [reconstruction-map.md](reconstruction-map.md), [zone-audit.md](zone-audit.md), [unified UAT guide](../guides/unified-uat.md#beta_site_evo_1), [placement seal](../analysis/union-beta-e1-continuation/placement-seal.md), [mission seal](../analysis/union-beta-e1-continuation/mission-seal.md)

World 23 is the first shared Union field map. A Human, a Free Jaffa and an Asgard each arrive from Omega with a different mission active and play a different arc on the same map. This page is the reference for what the server builds there. The decisions behind it (D-UB1 to D-UB15) live in the [packet index](../analysis/union-zone-planning/beta-work-packets.md#decisions) and the [ledger](../analysis/union-beta-e1-continuation/README.md#decisions); this page does not repeat them.

**What was tested.** Every seed row, chain and spawn below is covered by live-database tests (`beta_e1` modules), the interact-tag linter, the dialog-patch agreement test and the cross-race guard. **Nothing has been run in the lab or the client.** Every "UB-U" row in the [unified UAT guide](../guides/unified-uat.md#beta_site_evo_1) is pending.

## Arrival and the way back

| Item | Value | Label |
|---|---|---|
| Gate | stargate 6; arrival pin at the map's only PlayerStart (635.761, -60.134, 388.448), nav component 251, 17.9 m out on the gate's +X axis | RESTORED_SOURCE position, CIMMERIA_RECONSTRUCTION pin |
| Respawners | 6 `Respawner` at (633, -60.83, 399); 7 `Respawner_bet_Infirmary` at (953, 16.35, 287.5). The `UNAUTHORED_BY_DESIGN` exemption in `live_db_loaders.rs` is gone | CIMMERIA_RECONSTRUCTION |
| Way back | chains 23001-23003 grant the Omega address so a player who lands on a Union mission can dial home | CIMMERIA_RECONSTRUCTION |
| Offers | Beta never offers 1423, 1500 or 481. Omega owns them (D-UB3). Beta fires the arrival steps (D-UB2) | - |

## Race gates

| Race | Condition | Why |
|---|---|---|
| Human | `archetype gte 1` and `lte 4` | A missing archetype reads as -1, so one bound is not enough |
| Free Jaffa | `archetype eq 7` | Sholva |
| Asgard | `archetype eq 5` | The Asgard char-creation row is `NON_CANONICAL_BLOCKED_LEGACY` and starts in SGC_W1 |

Dialog binds are per player: `add_dialog_set`'s `slot` is the NPC's template id (D-UB10). Main has no phasing primitive, so a shared NPC cannot change for one player (D-UB11). Instead every dialog chain on a shared NPC is made exclusive by race, mission status or step status.

## Cast and templates 2300-2309

Beta-only templates keep Beta binds from lighting up the same NPC on other worlds. Display names are the client's own monikers from `texts.sql` (RESTORED_SOURCE). Seeds: `entity_templates_beta_e1.sql`, `entity_templates_beta_e1_life.sql`.

| Template | NPC | name_id | Notes |
|---|---|---|---|
| 2300 | Lt. Col. Carter | 7366 | Speaker 708; look copied from template 33. Gives all three races their arc |
| 2301 | Dr. Felger | 7368 | Speaker 710; the client moniker text is empty, so the template sets `display_name` |
| 2302 | Captain Satterfield | 7373 | |
| 2303 | Dr. Pierce | 7905 | **Reserved, never spawned.** The FV twins are out (D-UB7) |
| 2304 | Lt. Debeaux | 8552 | Stands at the training range |
| 2305 | Mimir | 7388 | Thor's look (template 64) |
| 2306 | Ras'nor | 7374 | Loth'ta look (440), no staff |
| 2307 | Sick Nine Peaks Warrior | 7414 | Level 5 |
| 2308 | SGC Personnel | 7450 | Life NPC, no dialog |
| 2309 | Jaffa Non Combatant | 7409 | Life NPC, no dialog |

Objects take templates 2320-2335: 2320 xeno cage (7376), 2321 Four Pieces case, 2322 hub computer, 2323 admin workstation, 2324 science computer, 2325 crate wreckage, 2326 Jaffa tools box, 2328 fire suppressant manager, 2329 fire control hub, 2330 Asgard scanner, 2331 analytical machine, 2332 hologram emitter, 2333 disabled drone, 2334 consciousness simulator, 2335 "Note". 2327 and 2336 are reserved. The block is 2300-2349.

## Spawns, regions and the ring pair

Spawn ids 23000-23199, one seed file per area (`spawnlist_beta_e1_{hub,asgard,ninepeaks,life}.sql`). All placements are CIMMERIA_RECONSTRUCTION against map pins that the placement seal re-probed on the navmesh.

| Area | Spawn ids | Contents |
|---|---|---|
| Hub and command | 23000-23021 | Carter 23000, Felger 23001, Satterfield 23002, Debeaux 23003, xeno cages 23010-23014, crates 23015-23016, hub computer 23017, admin workstation 23018, science computer 23019, Four Pieces case 23020, RuinNote 23021 (re-sealed: pathfinding caps at 256 polys) |
| Asgard vessel | 23060-23071 | Mimir 23060 on the deck, scanners 23061-23065, analytical machine 23066, consciousness simulator 23067, hologram emitter 23068, drone 23069, fire objects 23070-23071 |
| Nine Peaks | 23110-23115 | Ras'nor 23110, three sick warriors 23111-23113, Jaffa tools box 23114, SW ring 23115 (template 3) |
| World life | 23160-23172 | Template 2308 at the gate, command post, hut, quartermaster and science front; 2309 in the camp |

Regions are point sets: Storage 23000 (r20), AdminBuilding 23001 (r14), Infirmary 23030 (r10), FireSite 23031 (r12), NinePeaks 23060 (r20), RingTravelHub 23080 and RingTravelSW 23081.

**The west valley is a separate nav component (70) from the gate valley (251), with no walkable link.** A server-only ring pair fixes that without a client patch (D-UB4 A): ring regions 2300 (hub ring 148, at (146.716, 8.77, 403.273)) and 2301 (SW_RING, at (-791.6, -51.43, -92.6)). The Science Facility interior joins 251 across a 0.9 m door gap, and the underground complex is reachable only by ring, so nothing is seeded there.

## The three arcs

Chain ids are 23000-23499; the full table is in [mission-chains.md](mission-chains.md#beta-site-e1-world-23).

| Race | Missions | Stop point (D-UB5 A) |
|---|---|---|
| Human | 1423 → 381 Closing Shop → 473 Inventory → 478 Instant IT → 1425 SGU Database → 1426 Amateur Photography → 1427 The Ruins | 1425 stays open at S4244 (Jackson) and 1427 at S4250 (Woolsey). Neither NPC exists on Beta. 1426 completes |
| Free Jaffa | 1500 → 1502 The Nine Peaks Jaffa | 1503 The Antidote is not built (UB-22 held). 1502 completes |
| Asgard | 481 → 420 Reclamation → 421 Mechanical Error → 1147 Intriguing Data, plus 443 Incendiary, 477 Covert Observations, 479 Process Support | 481 and the seven Beta missions complete locally |

Offer order is strictly linear per race (D-UB15), and every Beta mission pays xp 0 and naq 0 (D-UB8).

**The Asgard arc cannot be reached by a real player yet.** Omega's UO-16 must deliver an Asgard player with 481 at S1760 and item 3399. It is held behind owner decisions U-D08 and U-D11 (HELD-B16). The arc is tested by archetype-5 replays only (D-UB13).

## The Asgard beam

The deck (nav island 355) is reachable only by beam. `interact_tag` on the beam base plus `move_entity` (D-UB12, CIMMERIA_RECONSTRUCTION) does the move.

| Direction | Destination |
|---|---|
| Up to the deck (`BEAM_DECK_DEST`) | (698.98, 157.93, 70.36) |
| Down to the ground (`BEAM_GROUND_DEST`) | (745.05, -19.60, 68.75) |

The existing spawns `BetaSite_00000007` (149, ground) and `BetaSite_Beam_000007_Ship` (151, deck) carry the clicks.

## The 421 infirmary and other local checks

The 421 infirmary is Beta-local with high confidence (D-UB6). `INFIRMARY_CENTRE` is (961.300, 15.97, 284.900), radius 10, on nav 251. The Quartermaster fires (1022.5, 13.9, 271.1) are static map emitters, so 443's "contain the fire" is an interact or a region at that cluster, never a fire that goes out.

## Provenance labels

| Label | Meaning on Beta |
|---|---|
| RESTORED_SOURCE | Rows from the seeds or the client: missions, steps, dialogs, dsm rows, items, text monikers, map pins |
| CIMMERIA_RECONSTRUCTION | Glue the source implies: every chain, the arrival pin, respawner positions, NPC placements, the beam move, the way-back grant, the ring pair |
| CIMMERIA_AUTHORING | New content with no source. Only UB-50 may carry it, and UB-50 is held on the owner's label policy (H-LBL), U-D07 and the CB-N05a client proof. **None ships** |

## What is not built

- 1503 The Antidote (UB-22 held) and the Omega/SGC receiver notes (UB-51 held), both D-UB5 A.
- Twin missions 383, 384 and 1146 (UB-52 closed, D-UB7).
- Authored relays and rewards (UB-50). Owner questions H-LBL, U-D07 (xp and naq for the 17 missions) and HELD-B16 stay open.
- Closed-Beta Visit 2 (1807-1826) and Beta Site E2 (world 60): research only.

## Related

- [Unified UAT: Beta_Site_Evo_1](../guides/unified-uat.md#beta_site_evo_1) lists every row still to run.
- [id-allocation.md](../analysis/four-zone-planning/id-allocation.md) holds the Beta ID block (D-UB9).
