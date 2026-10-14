# Harset ID proposals and collision record

> Type: reference. Audience: four-zone coordinator. Updated: 2026-10-14.
> Audit base: `d3efb7d41aad0b6008397f112d224ce7430d7acf`.
> Authoritative shared reservations: [four-zone ID table](../four-zone-planning/id-allocation.md).

These are **proposed claims**, held pending coordinator integration into the shared four-zone table and exact graph approval. This parallel packet does not edit that shared file. “Free” means absent from inspected committed seed and branch snapshots; it is not authorization to invent an actor or interaction. Recheck at seal, including new claims from sibling evidence packets and uncommitted changes in their owned ledgers through coordinator review.

## Current Harset chain inventory

All 73 current Harset chains are unique; 71 enabled, two disabled (567). The workspace seed has 349 unique chain IDs. Harset counts and exact IDs match the planning table on this base.

| Seed | Count | Taken IDs |
|---|---:|---|
| `harset_space_chains.sql` | 7 | 6001–6007 |
| `harset_goauld_chains.sql` | 27 | 6101–6127 |
| `harset_jaffa_chains.sql` | 18 | 6301, 6303–6308, 6331, 6333–6342 |
| `harset_opcore_chains.sql` | 21 | 6501–6505, 6511–6516, 6518–6523, 6525–6528 |

6309–6310 remains a historical arrival-dialog reservation; do not reuse it. Existing point sets 2108/2109 remain reserved historically for Bar/HoldingPens and are preferred over new IDs for those named regions. Existing templates and mission/step/objective IDs are reused only for their established actors/graphs, not unrelated authored content.

## Exact candidate claims for coordinator integration

| Packet / graph | Chain candidates | Other exact candidates | Gate |
|---|---|---|---|
| HA-01b (provisional 1664) | 6008–6020 (all thirteen IDs) | Spawn 360; template 260, proposed `Harset_Kaerlia` | Identity/costume/dialog/faction proof and Castle exact accept trigger. No allocation within the graph is sealed yet. |
| HA-01a Market/Storage doors | 6021–6024: hub→Market, Market→hub, hub→Storage, Storage→hub respectively | Point sets 2130–2133, four-point boxes 2560–2575 in matching groups of four; optional clickable spawns 361–364 only if the sealed affordance needs them | Four thresholds/arrivals plus world/clearance proof. Arrival points are not guessed from these IDs. |
| HA-01a follow-up capacity | 6025–6032 (eight IDs held) | No additional actor implied | Use only after an explicit graph amendment |
| HA-06b/HA-09 Bar | None yet; faction ranges below | Existing reserved point set 2108; candidate points 2576–2579 and spawn 365 `Harset_BarAnchor` / existing prop template 248 | No bar location evidence |
| HA-09 social speakers | Faction range below | Spawns 366 `Harset_HaughtyGoauld` / 210; 367 `Harset_AngryJaffa` / 206 | HA-01a doors and placement |
| HA-04/HA-13c Lethander | Faction ranges below | Spawn 368 `Harset_Lethander` / 46 | Stall/recipient location proof; no shared hostile mutation |
| HoldingPens, if required | None | Existing reserved point set 2109; candidate points 2580–2583; spawn 369 held without tag/template | Prove a mission consumer before adding a prop |
| HA-11 scarab anchors | 6151–6165 | Spawns 370–375 in order OpsYard/OverflowYard/BlackstockOffice/PetbeExterior/Fountain/Bookseller; template 240; sets 2136–2141 and cylinder points 2584–2589 | Exact sites and which objective uses interact/region/item; office may be world 68 |
| HA-07 four anchors | 6531–6550 | Spawns 376–379 in order OpsCenter/Research/Guardhouse/ScienceTent; template 248; sets 2142–2145 and points 2590–2593 | Objective/ability geometry and consumption |
| Future placement capacity | None | Sets 2134/2135 and 2146–2159; points 2594–2679; templates 261–269 | Unassigned candidate capacity, not a graph claim |

Candidate spawn IDs 360–379, template IDs 260–269, sets 2130–2159 and points 2560–2679 are free in this baseline. The Bar/pens candidates deliberately retain 2108/2109 rather than silently consuming 2134/2135. Tags are proposals until their actor is source/placement-approved; 369 has no invented holding-pen tag.

## Per-mission chain reservations

These existing range reservations are carried forward unchanged. Every integer in each range is absent from the current seed. They establish exclusive mission ownership; the coordinator still freezes each individual chain's trigger/conditions/action order before coder dispatch.

| Packet | Mission | Candidate chains |
|---|---:|---|
| HA-03 | 1325 | 6311–6330 |
| HA-04 | 1343 / 1347 | 6346–6365 / 6366–6380 |
| HA-05 | 1348 | 6381–6400 |
| HA-06a/b/c | 1351 / 1352 / 1353 | 6401–6415 / 6416–6440 / 6441–6470 |
| HA-07 | 1362 / 1363 | 6531–6550 / 6551–6570 |
| HA-08a | 1365 | 6571–6590 |
| HA-08b/c | 1375 / 1580 | 6646–6665 / 6691–6710 |
| HA-09 | 1377 / 1374 | 6666–6680 / 6621–6645 |
| HA-10a/b/c | 1371 / 1372 / 1410 | 6591–6605 / 6606–6620 / 6681–6690 |
| HA-11 | 741 / 1243 | 6131–6150 / 6151–6165 |
| HA-12 | 1240 / 1241 | 6166–6180 / 6181–6200 |
| HA-13a/b/c | 1244 / 1245 / 1246 | 6201–6215 / 6216–6230 / 6231–6245 |
| HA-14 | 1247 / 1322 | 6246–6260 / 6261–6275 |

Tollana alone owns recipient range 9800–9999; Harset proposes no IDs there. No authored mission, step, objective, item or dialog ID is allocated here. HA-17/18 must audit stock cooked missions plus all 77 reviewed IDs and the additional 1664 interface, and wait for CB-N05a client insertion proof. Existing dialog IDs are not free slots.

## Collision verification and limits

The audit reads INSERT column names and handles multi-row VALUES and quoted commas; IDs in JSON/descriptions are not counted. `point_set_points` is keyed by **point_id**, its second column, not set_id. The preliminary first-column scan was discarded; repeated set_id is legal and is not a collision.

| Seed key | Rows / distinct keys | Duplicates |
|---|---|---:|
| content_chains.chain_id | 349 / 349 | 0 |
| spawnlist.spawn_id | 632 / 632 | 0 |
| point_sets.set_id | 101 / 101 | 0 |
| point_set_points.point_id | 202 / 202 | 0 |
| entity_templates.template_id | 449 / 449 | 0 |
| dialog_set_maps.dialog_set_map_id | 4673 / 4673 | 0 |
| dialogs.dialog_id | 5418 / 5418 | 0 |
| items.item_id | 6074 / 6074 | 0 |
| missions.mission_id | 1041 / 1041 | 0 |
| mission_steps.step_id | 3480 / 3480 | 0 |
| mission_objectives.objective_id | 4037 / 4037 | 0 |
| mission_tasks.task_id | 4358 / 4358 | 0 |

A second committed-state scan enumerated **85 registered worktree branch refs**, compared each to the base for the five candidate seed-table files, and scanned changed committed seed snapshots on 72 refs. No collision was found for all 495 chain candidates (including every carried mission range), candidate actors/templates/regions/points or historical sets 2108/2109. Four campaign integration heads and evidence-worker refs were at the recorded base when inspected. This excludes uncommitted sibling work and durable claims written after the scan; the coordinator must reconcile those before freezing. It also excludes unrelated unregistered branches. No live database state was asserted.

Existing Harset relevant tags are globally unique in spawnlist. Referenced existing templates for placement/interface actors are present. Census IDs are distinct; ranges above do not overlap each other or taken chains. Static reservation checks are not runtime regressions. The exact checks and their outcomes are recorded in [HA-00b worknote](worknotes/HA-00b.md).
