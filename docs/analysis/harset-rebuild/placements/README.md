# Harset placements: the single ledger

> Type: reference. Audience: the owner, before and after a playtest. Written 2026-09-19. Method and evidence classes: [METHOD.md](METHOD.md).

**Nothing here was walked in a client.** Every coordinate below was estimated from the cooked maps, the navmeshes and real-player telemetry, and carries an evidence class and a confidence. The owner stopped waiting for the in-client placement session (M0) and asked for labelled estimates plus one list to correct after a playtest. This is that list.

The rows themselves live in three files, one per worker, in the same column layout (ID, what, world, X/Y/Z/heading, evidence class, confidence, checks run, how to verify in-client, how to correct):

| File | Scope | Ledger rows |
|---|---|---|
| [A-arrival-and-travel.md](A-arrival-and-travel.md) | gate-3 arrival pin, respawners 20/22/23, ring pads, chain 6007 and the 1361 abandon twin | 7 (6 HIGH, 1 MEDIUM) |
| [B-world57-population-and-regions.md](B-world57-population-and-regions.md) | world 57 population (15 spawns) and named regions (8) | 23 (2 MEDIUM-HIGH, 17 MEDIUM, 4 LOW) |
| [C-interiors-68-69-70.md](C-interiors-68-69-70.md) | Command Center population (10 new spawns), 3 interior regions, 15 encounter-anchor candidates (ledger only) | 14 placed rows plus 15 candidates |

## NO IDEA: what could not be placed, and why

These are **unseeded**. Nothing was invented to fill them. Each entry names what was searched and what was missing.

1. **Which doorway leads to the Market and which to the Storage Room, and the interior side of both doors.** The world-57 hub has 20+ doorway meshes and 255 `TriggerVolume` actors, but every volume is named just `TriggerVolume` and sits on a prop origin (guard posts, doorway meshes, awnings, fences): they are per-prop collision volumes, not door triggers. The Market map has no doorway mesh at all (three mesh families: two lights and a brazier), and its only trigger volume sits 5.7 m below the floor. The Storage map has fences only; 19 of its 22 volumes are on fence origins and 3 are at an unexplained height. Nothing in the map, seed or surviving scripts says which doorway is which. A wrong door pair is a soft-lock, not a cosmetic error, so none was seeded. *Cheapest fix:* stand in each doorway in-client and read two coordinates. *Alternative:* decode the interiors' BSP, since both maps are almost entirely BSP and the threshold is a brush face. Interior arrival points for both doors are unseeded for the same reason.
2. **`Harset.Bar` region and the `Harset_BarAnchor` prop** (mission 1352 objective 4008, mission 1374 anchor). No mesh, actor or document names a bar or tavern.
3. **`Harset.HoldingPens` region.** No location, and the tag registry has no holding-pen tag at all. If a mission needs pens, the registry needs a row first.
4. **Blackstock in world 57.** The audit puts him in an "office", which is the Command Center; he is placed there (LOW). If he turns out to be outdoors he goes next to Hansen and Jacobs.
5. **Vendor and trainer NPCs as inert props.** No generic vendor or trainer template exists and the vendor design gate (GH2) is open. The spec only says "Tau'ri vendors on the OP-CORE side lower level", which is a 130 x 230 m deck.
6. **`Harset_Lethander`'s stall.** No stall mesh is distinguishable among 34 merchant tents.
7. **`Harset_HaughtyGoauld` and `Harset_AngryJaffa`** (mission 1374). They are positioned relative to the Market and Storage doors, so they wait on item 1.
8. **The six mission-1243 `Harset_Scarab_*` anchors and the four mission-1362 anchors** (Operations Center, Research, Guardhouse, Science Tent). Out of scope for this pass. B recorded candidate evidence for several: the fountain meshes, the merchant-tent street, `EM-Cover_Guardpost_Med01`, the `EM-Tent_*` rows and `EM-Quartermaster01` at (188.51, -41.22, 156.71).
9. **The sarcophagus and the lab consoles in the Command Center.** No entity template exists for either and the map has no sarcophagus mesh. The map already draws four `GA-PuzzleStation00` consoles in the lab at (32.12 / 33.95 / 35.68 / 37.43, 0.32, -27.88); if a mission needs a clickable console, that is where it goes (HIGH on the position, only the template is missing).
10. **Ambient props in worlds 69 and 70.** Deliberately not seeded: every interior prop a mission uses is a mission-scoped `spawn_entity` in the player's own instance, and a spawnlist row would show in every player's instance regardless of mission state.
11. **Which Tau'ri officer stands where inside the ops room.** The room is MEDIUM; the assignment of Marsh, Copplemann and Blackstock to specific spots inside it is arbitrary and is a one-line edit each.

## Check these first in a playtest (the weakest rows)

| Row | What | Why it is weak |
|---|---|---|
| PL-B-12 | Shield Controls prop (spawn 311) and its region (PL-B-20, set 2104) | LOW, the weakest row: the evidence for where the controls sit is thin |
| PL-B-15 / PL-B-19 | Petbe's quarters search object and region (spawn 313, set 2103) | LOW-MEDIUM; see the row for its evidence |
| PL-C (Blackstock, Opheltes, Athena) | Command Center NPCs (spawns 346, 348, 349) | LOW, inferred; no navmesh in world 68 |
| PL-A-03 | Market respawner (48.0, 3.61, 78.0) | MEDIUM, floor evidence only: world 69 has no navmesh at all |
| PL-C encounter anchors | 15 candidate points in Market and Storage | INFERRED from cover-node clusters, MEDIUM at best, no chains authored |

The strong rows are the gate-3 arrival pin (-5.0, -68.99, 33.0, yaw 3.141; dropped on 2026-09-25 by NPC-AI NA29, so travellers now arrive on the gate row), the world-57 respawner (-8.0, -68.99, 34.0) and the world-70 respawner (50.0, 0.0, 44.0): each stands on the true floor. The two world-57 rows are inside the main plaza navmesh component near where real players stood; the world-70 row is on-mesh in the room's own component with the floor matching the fence props, but has no player-position evidence.

## The systemic finding: `harset.nav` is the defect

Every worker hit the same wall. `data/spaces/harset.nav` does not model the upper quarters of world 57 at the floor height the geometry has, and it splits the plaza into about ten tiny components:

- A ring of probes around every named Jaffa Zone landmark found **no** on-mesh point at the real floor (y = -41.28). The two authored spawns in that quarter (Petbe, FirstBug) are off-mesh by 10 to 11 m, and so are four of the five authored ring pads. The pads' rows are correct: the map has a ring-platform surface within 0.04 m of all five authored heights. Only pad 4 is on-mesh.
- Chain 6007's coordinate (0, -67.6, -231) has a real floor 0.04 m below it, but nothing within about 20 m is on the mesh, so "wait for an on-mesh pin" was waiting on a mesh rebuild, not a playtest.
- World 70 is `enforce` against a mesh whose largest component is an 82,249 m2 whole-map ground plane, and world 69 still has a degenerate bounding box (H-B11).
- World 70's shipped mesh puts the pen floor (component 36) and the upper arrival wing (component 6) in separate components, so nothing can path between them, although real players stood on both and walked between them; the rebuilt mesh merges them. Consequence for whoever writes missions 1365, 1375, 1580 and 1245: on the mesh the server loads today, a mission-scoped `spawn_entity` in the arrival corridor cannot path to a player on the pen floor. Use encounter anchors PL-C-E08 to E12 (pen floor), not E13 to E15. Also, the Storage region's ceiling is 4.00, set from real player positions: a taller box would make a player still on the deck above already "in Storage", so the `enter_region` edge would never fire when they descend (PL-C-14).
- What keeps all of this working today is `navmesh_mode = 'advisory'` on world 57. `H14`'s acceptance line "on-mesh for every world-57 spawn" therefore cannot be met, and the shipped guards instead record each row's on/off-mesh verdict and **will fail when the mesh is rebuilt**, forcing these rows to be re-checked rather than silently passing.

A rebuilt Harset mesh exists (374 components instead of 1,939, from the Castle navmesh work) but loses 12 positions real players stood on, so it is not a drop-in yet.

## How to correct these in one pass

After a playtest, send the corrected coordinates keyed by row ID (`PL-A-nn`, `PL-B-nn`, `PL-C-nn`). Each ledger row names the seed file and the row to edit. Nothing is generated: the values live in `db/resources/Worlds/Seed/{stargates,respawners,spawnlist}.sql` and `db/resources/Events/Seed/{point_sets,point_set_points}.sql`. Id ranges used: spawn 300-314 (B) and 340-349 (C); point sets 2100-2107 (B) and 2120-2122 (C); point ids 2500-2519 (B) and 2540-2551 (C). Ids 2108 and 2109 are reserved for the Bar and Holding Pens regions if they are ever placed.

## What is still not started

The blocked mission packets (H23-H28, H32-H37, H42-H46) and H05 are not written. They needed these placements first and can start now, apart from anything that depends on the unplaced items above.
