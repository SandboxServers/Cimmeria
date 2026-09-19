# PL-C: interior worlds 68, 69 and 70

> Type: reference. Audience: the owner's post-playtest correction pass, and the coordinator assembling [README.md](README.md). Written 2026-09-19 under [METHOD.md](METHOD.md): placed from map data, not from an in-client walk. **Every coordinate below is a labelled estimate, not a pin.**
>
> Scope: packet H12 (Command Center population, world 68), H14 (interior props, worlds 69 and 70), H15 (interior named regions), and encounter-anchor candidates for the world 69/70 mission fights. Door regions (`*.HarsetDoor`, `Harset.MarketDoor/StorageDoor`) are **not** here — they belong to the world-57 clusters.

## Method validation on this cluster

Three anchors in these maps were authored by the original developers, and all three agree with the `obj_slab` floor heights this cluster derives everything else from. That is what makes the estimates below better than guesses:

| Authored anchor | Value | `obj_slab` floor at that spot | Delta |
|---|---|---|---|
| spawn 222, Anat | y 1.77149999 | 1.70 (dais, x[56.5,69.5] z[69.5,83.0]) | +0.07 |
| point set 2079, `Harset_CmdCenter.HarsetTransition` | y 0.320000291 | 0.32 (entry hall) | 0.00 |
| respawner 21, `Command Center Respawn` | y 0.355 | 0.32 (entry hall) | +0.035 |

Spawn Y in this cluster is therefore **floor + 0.05** throughout: both non-zero deltas are small and positive, and erring high is safe (`is_point_valid` allows +4.0 m above but only −1.2 m below) while erring low risks clipping.

**Heading convention, proved from the seed rather than assumed.** `heading = atan2(dx, dz)`, so 0 = +Z and pi/2 = +X. Lieutenants 235 (x −4.44, heading pi/2) and 236 (x +4.33, heading 3pi/2) face each other across the z = −231 threshold; plaza guards 225/234 (x −18.7, heading ~pi/2) face inward while 228/231 (x +18.6, heading ~3pi/2) do the same from the opposite side. Anat's 3.117 (~pi) therefore means she faces −Z, down the only stair onto her dais — "face the way a visitor arrives". No PL-C row is left at heading 0.

## World 68 room inventory

All from `obj_slab` over `$O\Harset_CmdCenter` (a 5 m grid, refined to 2 m around each placement). World 68 has **no navmesh**, so no on-mesh check is possible anywhere in it; reachability below means "a continuous walkable floor at one height joins these rooms", read off the floor map, not off a mesh.

| Room | Footprint | Floor Y | Landmarks |
|---|---|---|---|
| Entry hall | x[−13.1, 10.6] z[−37.6, 15] | 0.32 | arrival (0, 0.355, −20); point set 2079 at its south end |
| Cross hall (east-west spine) | x[−90, 55] z[17, 32] | 0.32 | joins every other room |
| North hall (terraced processional) | x[−32, 30] z[32, 92] | 0.6 → 1.90 in ~0.2 steps, flat 1.90 platform at z[76,92] | sunken central basin x[−6,6] z[50,76]; two `GA-WaterTower00` at (±30, 1.92, 73) flanking the platform |
| Lab wing (east) | x[27.5, 69] z[−31, 11] | 0.32 | four `GA-PuzzleStation00` consoles at (32.12 / 33.95 / 35.68 / 37.43, 0.32, −27.88); three `GA-WaterTower00` tanks; 26 `GA-Viewscreens00` |
| Ops / war room (west) | x[−56, −28] z[−30, 12] | −0.64, central dais 0.00 with a 0.30 console ridge | eight `GA-Monitor01` wall banks at x −27.0/−57.4, z −3.7/−14.9; gated entrance (two `GA-Fence01` at x −45.3/−38.3, z 4, four TriggerVolumes on them) |
| Anat's dais | x[56.5, 69.5] z[69.5, 83] | 1.70, edged 1.90, columns 3.8 at x 59/67 z 80 | spawn 222; stair up at x[60,66] z[62,70] |
| East hall (contains the dais) | x[45, 81] z[36, 84] | 0.32 | a second 1.70 platform at x[71,81] z[42,56] |
| West ceremonial hall | x[−60, −40] z[38, 70] | 0.32 | two `JF-BrazierCenter01` at (−58.89 / −43.19, 0.32, 50.11), each with a TriggerVolume and a fluid disc above it |

## Ledger

`is_point_valid` cannot be run in worlds 68 or 69 (no mesh); the "Checks run" column says so rather than leaving the reader to assume it passed.

| ID | What | World | X, Y, Z, heading | Evidence class | Confidence | Checks run | How to verify in-client | How to correct |
|---|---|---|---|---|---|---|---|---|
| PL-C-01 | spawn 340 Ba'al, template 42, `CmdCenter_Baal` | 68 | 2.0, 1.95, 88.0, 3.14159 | MAP-GEOMETRY + INFERRED | MEDIUM | floor 1.90 at (2, 88) and across z[76,92]; no navmesh; heading = face −Z down the processional ramp; 20 m clear of every other row | Arrive at (0, 0.355, −20), walk +Z through the entry hall, cross hall and terraced ramp; Ba'al should be standing at the top of the ramp facing you | `spawnlist.sql` spawn 340. Competing reading: beside Anat at (65.0, 1.75, 77.17), same heading |
| PL-C-02 | spawn 341 Anat's Royal Guard, template 209, `CmdCenter_RoyalGuard` | 68 | 58.5, 1.75, 77.0, 3.14159 | MAP-GEOMETRY + AUTHORED-adjacency | MEDIUM-HIGH | floor 1.70; 3.47 m from spawn 222; clear of the 3.8-high columns at x 59/67 z 80; no navmesh | Stand at Anat and look west along the dais; the guard is at her right hand facing the stair | `spawnlist.sql` spawn 341 |
| PL-C-03 | spawn 342 Anat's symbiote tank, template 245, `CmdCenter_SymbioteTank` | 68 | 66.0, 1.75, 77.0, 3.14159 | MAP-GEOMETRY + SPEC-DESCRIPTIVE | MEDIUM | floor 1.70; 4.03 m from spawn 222; not co-located with any of the six map `GA-PuzzleStation00` actors, which draw the same mesh; no navmesh | Climb the dais stair at x~63 z~66; the tank is on Anat's left and should be clickable | `spawnlist.sql` spawn 342 |
| PL-C-04 | spawn 343 Moh'katan, template 54, `CmdCenter_Mohkatan` | 68 | −5.0, 0.37, −8.0, 2.74680 | MAP-GEOMETRY + INFERRED | MEDIUM | floor 0.32; inside the 2079-confirmed hall width x[−13.1,10.6]; heading = face the arrival point; off the corridor centre line; no navmesh | Arrive from world 57; Moh'katan is 13 m ahead and slightly left, already facing you, and should offer 1324 | `spawnlist.sql` spawn 343 |
| PL-C-05 | spawn 344 Col Marsh, template 10, `CmdCenter_Marsh` | 68 | −49.5, −0.59, −14.0, 1.57080 | MAP-LANDMARK + MAP-GEOMETRY | MEDIUM | floor −0.64; heading = face the central briefing dais; no navmesh | Enter the west room through the fence gate at z~4 and descend; Marsh is on the far side of the briefing dais, facing it | `spawnlist.sql` spawn 344 |
| PL-C-06 | spawn 345 Capt Copplemann, template 48, `CmdCenter_Copplemann` | 68 | −35.5, −0.59, −14.0, 4.71239 | MAP-LANDMARK + MAP-GEOMETRY | MEDIUM | floor −0.64; Marsh's mirror across the dais; 15 m apart; no navmesh | As PL-C-05; Copplemann faces Marsh across the dais | `spawnlist.sql` spawn 345 |
| PL-C-07 | spawn 346 Blackstock, template 214, `CmdCenter_Blackstock` | 68 | −50.0, −0.59, 6.0, 2.76109 | INFERRED | **LOW** | floor −0.64; clear of the entrance ramp at x[−44,−40]; no navmesh; the spec's "office" is **unresolved** — no room in the decoded geometry is an office | Enter the west room; Blackstock is just inside on the left, facing the dais | `spawnlist.sql` spawn 346. Also decide against H14's `Harset_Blackstock` in world 57 — same template 214, only one should ship |
| PL-C-08 | spawn 347 Nerus, template 53, `CmdCenter_Nerus` | 68 | 34.8, 0.37, −25.5, 0.18963 | MAP-LANDMARK + MAP-GEOMETRY | MEDIUM-HIGH on the room, MEDIUM on the metre | floor 0.32; 2.4 m in front of the four-console row at z −27.88; heading = face the lab's west doorway; inside point set 2120; no navmesh | Take either lab doorway off the cross hall; Nerus is at the console row at the far wall, facing you | `spawnlist.sql` spawn 347 |
| PL-C-09 | spawn 348 Opheltes, template 215, `CmdCenter_Opheltes` | 68 | −10.0, 0.45, 36.0, 3.14159 | INFERRED | **LOW** | floor 0.40; clear of the z[32,36] pillars at x −18/−22; no navmesh; **nothing in any chain, mission row or spec line places Opheltes** | He is on the first terrace of the north ramp, facing back toward the cross hall | `spawnlist.sql` spawn 348 |
| PL-C-10 | spawn 349 Athena, template 44, `CmdCenter_Athena` | 68 | −18.0, 1.95, 84.0, 2.49809 | INFERRED | **LOW** | floor 1.90; off the processional centre line; no navmesh | On the north hall's top platform, west of Ba'al, facing the ramp | `spawnlist.sql` spawn 349. Not in the H12 scope list — added because H32 (1363 step 4047) needs a spawn and the tag registry already reserves it. Flag if 1363 wants her elsewhere |
| PL-C-11 | spawn 222 Anat — `tag` filled to `CmdCenter_Anat` | 68 | unchanged (61.969, 1.7715, 77.172, 3.11705) | AUTHORED | HIGH | no coordinate changed; the registry requires the tag on the existing row, not on a second spawn | Anat is where she always was | `spawnlist.sql` spawn 222 |
| PL-C-12 | point set 2120 `Harset_CmdCenter.Lab`, points 2540-2543 | 68 | box x[27.5, 69.0] z[−31.0, 11.0], floor 0.32, ceiling 10.02 | MAP-LANDMARK + MAP-GEOMETRY | HIGH on the room, MEDIUM on the edges | contains the console row and all three tanks; excludes the cross hall so entry is an edge crossing; walls confirmed (no floor at x 25, x 70, z −32); no navmesh | Walk into the lab from the cross hall with 1241 step 3609 active; the scan objective should tick | `point_sets.sql` 2120 + `point_set_points.sql` 2500-2503 |
| PL-C-13 | point set 2121 `Harset_Market.Marketplace`, points 2544-2547 | 69 | box x[30, 100] z[30, 100], floor 3.60, ceiling 13.60 | MAP-GEOMETRY + MAP-MARKER | HIGH on the floor plane, **MEDIUM on where to stop it** | the whole of world 69 is one 3.60 platform, and 217 of its 274 `SGWSpecCoverNode` actors sit on it at y 3.6-3.7; excludes the 4.80 torch-lit pocket at x[12,26] z[9,21], read as the vestibule the Harset door opens into; world 69 has **no navmesh** | Enter the Market from world 57 with 1352 step 4007 active; the objective should tick as you leave the first room, not on arrival | `point_sets.sql` 2121 + points 2544-2547. **If the world-57 Market door lands somewhere other than that pocket, this is the one number to revisit** |
| PL-C-14 | point set 2122 `Harset_StorageRm.Storage`, points 2548-2551 | 70 | box x[19.0, 84.0] z[38.0, 97.0], floor 0.30, ceiling 10.30 | MAP-GEOMETRY + on-mesh | HIGH | every metre of the box resolves to navmesh component 36 (the pen-grid floor, 314 polys, 3055.7 m2); centre and all four edge midpoints are mutually path-reachable; all 19 pen gates are inside; excludes the upper wing at x[45,75] z[0,40] (floor 5.10, component 6) | Descend from the upper corridor into the pen grid with 1580 step 4700 active | `point_sets.sql` 2122 + points 2548-2551 |

## Encounter anchor candidates (ledger only — nothing seeded, no chains)

The mission packets for the Marketplace assault (1348), the infiltrator fight (1241), and the Storage fights (1365, 1375, 1580) are not written. These are **candidates for whoever writes them**, derived from where the original designers put cover: 274 `SGWSpecCoverNode` actors in the Market and 353 in Storage, single-link clustered in XZ at a 4 m radius. Every hostile in 69/70 is a mission-scoped `spawn_entity` in the player's own instance, never a spawnlist row (H03), so none of these is a seed row today.

Confidence is **MEDIUM at best** for "a fight was designed here" and **LOW** for any individual stand spot: a cover node says "a designer expected someone to take cover at this spot", not which mission or which side.

### Harset_Market (world 69), floor 3.60

| ID | Candidate | Centre | Nodes | Read |
|---|---|---|---|---|
| PL-C-E01 | main arena, south-east | (81.9, 3.65, 40.6) | 30 | the densest cover in the map (88 nodes within 8 m linkage, x[50,96] z[35,63]); the obvious 1348 assault ground |
| PL-C-E02 | arena west flank | (59.9, 3.65, 39.0) | 16 | same arena, across it from E01 — a plausible defender line |
| PL-C-E03 | arena far corner | (94.5, 3.65, 44.9) | 10 | against the east wall; a plausible wave entry |
| PL-C-E04 | east lane | (92.6, 3.65, 58.8) | 10 | links E01 to E05 |
| PL-C-E05 | north-east quarter | (81.8, 3.65, 95.2) | 10 | second largest 8 m cluster (47 nodes, x[71,96] z[64,97]) |
| PL-C-E06 | central-west stalls | (39.3, 3.65, 63.0) | 18 | third cluster (49 nodes, x[35,61] z[55,80]); a plausible 1241 scan spot |
| PL-C-E07 | vestibule exit | (20.0, 3.84, 32.9) | 7 | the only cover near the 4.80 pocket, i.e. just inside the Marketplace region edge — the natural spot for a first contact |

### Harset_StorageRm (world 70), pen-grid floor 0.30

All of these sit inside point set 2122 and on navmesh component 36.

| ID | Candidate | Centre | Nodes | Read |
|---|---|---|---|---|
| PL-C-E08 | west pen aisle | (26.4, 0.35, 71.0) | 38 | largest tight cluster; inside the 8 m cluster of 146 that spans the whole west half |
| PL-C-E09 | east pen aisle | (72.6, 0.30, 69.7) | 34 | mirrors E08; a two-sided fight reads as E08 vs E09 |
| PL-C-E10 | centre floor | (49.3, 0.42, 59.5) | 26 | the middle of the grid, between the z 53.7 and z 58.3 gate lines |
| PL-C-E11 | north wall | (52.7, 0.35, 93.9) | 30 | a wide shallow band at z[92,97] — a plausible last-stand line |
| PL-C-E12 | south threshold | (58.0, 0.30, 37.7) | 10 | at the region's south edge, below the upper wing: the most plausible **wave entry** into the pen grid |

### Harset_StorageRm upper wing (world 70), floor 5.10, navmesh component 6

| ID | Candidate | Centre | Nodes | Read |
|---|---|---|---|---|
| PL-C-E13 | upper corridor head | (49.3, 5.22, 5.8) | 40 | the densest cover anywhere in world 70; at the far (z~0) end of the arrival corridor |
| PL-C-E14 | upper corridor east | (70.7, 5.17, 9.5) | 17 | the corridor's side branch |
| PL-C-E15 | upper corridor mouth | (50.8, 5.22, 40.7) | 15 | where the corridor meets the descent into the pen grid — a choke point, and the counterpart to E12 |

## No idea

Per the METHOD.md NO-IDEA rule, these are **left unseeded** rather than given an invented coordinate.

- **The sarcophagus (H12 scope).** No `entity_templates` row exists — H11 seeded prop templates 240-248 and none is a sarcophagus. The audit records it as "asset strings only, no actor" ([audit.md](../audit.md), Command_Center row), and `archetype_census` finds no sarcophagus prefab in `Harset_CmdCenter`. Nothing in any written chain interacts with one. Searched: `entity_templates.sql`, the three `harset_*_chains.sql` files, `Harset_CmdCenter_arch_meshes.tsv`, the extracted actor list (1,645 actors, no matching name). Missing: a template and any position evidence.
- **The lab consoles (H12 scope).** Same missing template. Additionally, the map **already draws** four `GA-PuzzleStation00` consoles in the lab at (32.12 / 33.95 / 35.68 / 37.43, 0.32, −27.88), so an invented prop template would double a mesh that is already on screen. The only written consumer of the lab is mission 1241 step 3609, which is a *region* scan, and point set 2120 covers it. If a later packet needs a clickable console, the four positions above are where to put it — that part is HIGH confidence, it is only the template that is missing.
- **Every world 69 / 70 spawnlist prop (H14 scope).** Nothing seeded, and this is a design finding rather than missing evidence: the tag registry assigns every interior prop the packet names — Replitech crate (241), storage container (242), Devlin's device (247), beacon (246) — to a mission-scoped `spawn_entity` in the player's own instance, and Lethander's stall is `Harset_Lethander` (template 46) in world **57**, not in 69. Worlds 69 and 70 are one instance per player, so a spawnlist row there would appear in every instance of every player regardless of mission state, which is the opposite of what all five missions want. That leaves only ambient scenery, which has no mission consumer and no position evidence — inventing it would be the Romney mistake with extra steps. **If the coordinator disagrees**, the cover-node clusters above are the best available positions.
- **`Harset_Market` / `Harset_StorageRm` arrival points.** Not in this cluster's scope (the doors are the world-57 cluster's), but PL-C-13's edge choice depends on the Market door landing in the south-west 4.80 pocket. Flagged rather than assumed.
- **Which of the three Tau'ri officers stands where inside the ops room.** The room is MEDIUM-confidence; the assignment of Marsh / Copplemann / Blackstock to specific spots within it is arbitrary, and correcting it is a one-line edit each.

## Tests

All in `crates/services/src/cell/spawner/tests/harset/`:

- `cmdcenter_population.rs` — five live-DB guards: the world-68 roster is exactly the eleven rows above, nothing there uses a hostile template, every row is stationary with `respawn_secs` and a non-zero heading, the four chain-referenced tags resolve, and `load_spawns_from_db` returns the full roster (the row counts prove the seed; only the loader round-trip proves the cell would spawn it).
- `interior_regions.rs` — three live-DB guards: all three sets survive `load_regions_from_db` as `AreaSet` in the right world with four corners, each box contains its room *and* excludes the room its visitors arrive from, and the world-70 box stands on one connected piece of `harset_storagerm.nav`.

**Why the world-70 guard is a reachability test and not `is_point_valid`.** `harset_storagerm.nav` has 104 components and three of them overlap the Storage footprint in Y: the pen-grid floor, a disconnected duplicate 1.4 m beneath it, and an 82,249 m2 outdoor terrain sheet at the same 0.2-0.4 Y band. A region dragged clean off the building still answers "on-mesh" at y 0.3 — it just answers from the terrain, which is exactly what happened on the first draft of this guard. `dtNavMeshQuery::findPath` is not the fix on its own either: it returns a *partial* corridor to the closest reachable polygon, so `find_path(...).is_some()` is `true` across a component boundary. The guard compares the path's last waypoint against the request, and carries a control asserting that a point 100 m outside the building reads on-mesh but is **not** reachable — so the test proves its own discriminating power rather than asserting it in a comment.

Revert-verified by mutating the DB one property at a time and confirming the matching guard fails for the right reason: heading zeroed, `is_stationary` cleared, `respawn_secs` nulled, a template flipped to faction 10, a row deleted, Anat's tag cleared, point set 2120's `type` changed, and point set 2122's footprint shifted 30 m south.
