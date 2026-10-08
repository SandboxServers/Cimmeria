# Dakara_E1 placements: the single ledger

> Type: reference. Audience: the owner before and after a playtest; DK-04, DK-05 and the mission packets that take coordinates from here. Written 2026-10-06 (packet DK-02) against `main` @ `ddd549797`, client build QA4046. Method and evidence classes: the [Harset METHOD](../../harset-rebuild/placements/METHOD.md), reused unchanged (decision OD-DK03).

**Nothing here was walked in a client.** Every coordinate was estimated from the owner's cooked maps (`Dakara_E1`, `Dakara_E1_StoryRm`), the shipped navmeshes and the seed, and carries an evidence class and a confidence. The zone has no spawn, region or script actor of any kind, so no coordinate is a recovered fact except the gate, the DHD, the two ring transporters and the room's authored respawner. The owner corrects the rest in one pass after M2, using the row ids below.

| File | Scope | Rows |
|---|---|---|
| [A-world61-story-placements.md](A-world61-story-placements.md) | command, healing and Moh'katan tents and their flaps, the shared return point, Command Terminal, Rak'nor, three SG-18 sites, the two wall gates, the two Ha'tak plazas, the Superweapon courtyard and its two power supplies | 16 (2 HIGH for position, 1 MEDIUM, 3 LOW-MEDIUM, 10 LOW) |
| [B-world62-tent-interior.md](B-world62-tent-interior.md) | the story tent: respawner 25, two exit flaps, Bra'tac, Moh'katan, Moh'katan's Terminal, the Ba'al hologram | 7 (1 HIGH for walkable, 1 MEDIUM, 2 LOW-MEDIUM, 3 LOW) |
| [C-landmark-census.md](C-landmark-census.md) | evidence pack: conversion check, landmark census, the four furnished camps, tent groups, navmesh components, cover | no rows |
| [data/](data/) | `Dakara_E1_landmarks.tsv` (763 rows), `Dakara_E1_arch_meshes.tsv` and `_arch_positions.tsv` (the `archetype_census` output), `Dakara_E1_tent_groups.tsv`, `Dakara_E1_StoryRm_props.tsv`, `Dakara_E1_probes.txt` (every row's navmesh probe) | n/a |

The reproduction commands, the tools that ran and did not run, and the open questions: the [DK-02 worknote](../worknotes/DK-02.md).

## Headline findings

1. **The city has a wall with three gates, and the shipped navmesh has no link through any of them.** Component 279 (the walled city, holding the plaza) and component 324 (everything outside) come within 1.5 to 3.5 m of each other only at three `JF-HighWallArch00` openings. Everything the story puts outside the wall (the HB camp, the far tent camps) cannot be reached by an NPC path from the plaza. Detail: [the wall and its gates](#the-wall-and-its-gates).
2. **The nearest tents are not the tents the audit listed.** Four furnished military camps stand 42 to 125 m from the gate (camps A to D). They are direct static meshes with no trigger volumes, so the audit's trigger-based tent groups, which start 200 m out, missed them. Camp A is 51 m from the gate, holds three adjacent tents, and fits Rak'nor's "just to the east of the Stargate, next to the healing tent". [Census](C-landmark-census.md#the-four-furnished-camps).
3. **World 62 has two door markers and one furnished tent**, and the story strings name two exit flaps. The two tents on world 61 are adjacent, so DK-04's single shared return point works: PL-DK-A-04. [Cluster B](B-world62-tent-interior.md#what-this-answers-for-dk-04).
4. **No `covernodes_*.pak` holds Dakara_E1 sets.** They hold prefab and mesh templates with no map key. The map itself has 9,847 `SGWSpecCoverNode` actors in 1,643 cover sets; nothing is seeded. [Cover](C-landmark-census.md#cover-task-4).
5. **The Superweapon courtyard has a candidate** (a column ring, a bridge and the map's only two `Tol-IonCannon00`, matching "two turret power supplies"), and the **SG-18 sites have a candidate area** (the map's only `HB-` Tau'ri-flavoured props). Both are naming inferences: LOW.

## The wall and its gates

`JF-HighWallArch00` appears 14 times; three of them sit on the line between components 279 and 324:

| Opening | Arches | Gap |
|---|---|---|
| x = -212, z = 136 | `JF-HighWallArch00` (-209.3, -21.8, 135.7) + `JF-GuardWallArch00` (-215.0, -14.2, 135.6) | 1.50 m |
| x = 411, z = 203 | `JF-HighWallArch00` (408.7, -21.8, 202.2) + `JF-GuardWallArch00` (414.3, -14.2, 203.4) | 1.82 m |
| x = 72, z = 441 | `JF-HighWallArch00` (72.1, -21.1, 441.7), single | 2.64 m |

The first two are a matched pair 623 m apart on the x axis and 67 m apart in z, symmetric about x = 99.7 (the gate prefab's x to within 3 m). They are the best candidates for mission 1647's "Western Gate" and "Eastern Gate" (rows PL-DK-A-10 and A-11). The third, a lone arch at the far end of the city, has no mission string. **Consequence:** DK-20 and the mission 1647 packet must not rely on NPC paths between the two sides, and the navmesh needs a link or a rebuild before an outside hostile can reach a gate defender.

## The axis question

The mission strings say "Eastern" and "Western". **The map does not say which end of any axis is which.** No prefab is named for a compass direction, the map has no `PlayerStart`, and the minimap art was not read (see the worknote). What the map does give:

- The two wall gates differ in **BigWorld x** (-212 and 411); their z differs by 67 m. If "Western" and "Eastern" name them, the pair lies along x. That is MEDIUM for the axis, and says nothing about the sign.
- Rak'nor's line plus the nearest equipped camp puts camp A, at **+x** from the gate, as the command tent's camp. If that reading is right, east is +x. It is one inference from one sentence; camp B at -x is as close and lacks medical boxes, which is the only thing favouring A.
- UE3's editor convention would make east +x in UE space, which is BigWorld +z. That points the other way, so no convention settles it.

So: **no row in this ledger assumes a compass direction.** The gate rows are named by sign of x, and the owner's playtest settles the names in one edit. The "Eastern tents" of the mission text are in the No idea list.

## No idea

These are **unplaced**. Nothing was invented to fill them. Each names what was searched and what was missing.

1. **Naquadah Repository and Loth'ta's camp** (missions 1570 step 4902, 1645, 1648). No mesh, prefab or actor name in the map mentions a repository, storage, naquadah, a mine or a silo (the full per-mesh name list was read: 350 direct meshes and every prefab template name). The only place with Goa'uld military hardware is an isolated outpost at x 261 to 341, z 23 to 130 (`GA-HeavyArtillary00` at (282.4, -17.1, 88.2), `GA-Barracks00` at (261.2, -21.8, 22.8), `GA-Container00` clusters at (282, 83) and (341, 84), the map's densest cover: 378 and 364 nodes in two 60 m cells at (273, 92) and (324, 89)). That is a **lead**, not evidence: nothing says a Jaffa camp or a repository stands there, and mission 1645's optional "follow the path" objective has no path in the data. *Cheapest fix:* the owner names the place in a client; a point set follows.
2. **The five Drop Locations** (mission 1646, dialog 5821). Nothing in the map is distinguishable as a scout drop. The 96 `HT-Campfire00` and 235 `JF-Brazier00` props are too common to be markers. They need five points of the owner's choosing, ideally inside the walled city (component 279) so the player can reach them.
3. **The "Eastern tents"** (mission text). 39 tent groups exist ([tent_groups.tsv](data/Dakara_E1_tent_groups.tsv)), none named; with no axis the choice among them is open. The nearest groups (G01 to G05) are camps A to D and the merchant rows.
4. **Which wall gate is the Western one** and which the Eastern. Placed by sign of x (A-10, A-11); the names wait on [the axis question](#the-axis-question).
5. **The tents' door sides.** The three camp-A tents are direct static meshes with no visible doorway; the flap rows are approach points on the plaza side, not doors. *Cheapest fix:* the prefab tents carry `DoorwayPrecipitationPlanes` stubs whose offsets, applied to the same mesh, would give the door side; this packet did not extract them (collision-off stubs, outside the standard tools).
6. **Which of T1, T2, T3 is the command tent, the Med Tent and Moh'katan's tent.** Their contents overlap; the assignment in A-01 to A-03 is the reading with the most evidence and is interchangeable.
7. **Vocuum** (`_int_Vocuum`, mission 1653's optional objective). No mesh, no cover cluster, no description.
8. **Ring Control** (`_int_RingControl`). No console mesh stands by either ring prefab; the only extra ring mesh (`GLB-RingTransporterBase_TC00` at (140.2, -2.4, -493.6)) is a lone base with no Kismet and is 540 m from both rings. The ring pad itself (A-12, A-13) is the only place the object can be.
9. **The third wall arch's role** (72.1, -21.1, 441.7). It is a gate-shaped opening with no mission string.
10. **Whether the Superweapon courtyard is on world 61 at all.** World 65 `Dakara_Superweapon` has no map in the client (the audit's finding), and "Dakara Superweapon Courtyard" is a discovery string of `DakaraE1`, so A-14 assumes it is on 61. If a later build moves it, A-14 to A-16 are void.
11. **Hostiles, spawn clusters and encounter anchors for missions 1647 and 1653.** Out of scope for this packet; DK-20 and OD-DK05. The cover data above (cover density at the gates and the courtyard) is the only input found.
12. **Loth'ta, the Jaffa Captain (dialogs 6106/6107), Ba'al's hologram on world 61.** No evidence places them; the Jaffa Captain, if he stands at the plaza, shares Rak'nor's area (A-06).

## Check these first in a playtest

| Row | What | Why it is weak |
|---|---|---|
| PL-DK-A-01 to A-03 | the three tent flaps | LOW: the assignment of three tents is interchangeable and the door sides are unknown |
| PL-DK-A-07 to A-09 | the three SG-18 sites | LOW: an inference from a mesh prefix; all outside the wall, none reachable by path |
| PL-DK-A-15, A-16 | the two Turret Power Supply props | LOW: inferred from "two" and "Ion Cannon" |
| PL-DK-A-10, A-11 | the two wall gates | LOW-MEDIUM: geometry says they are gates; the names are an axis guess |
| PL-DK-B-03 | the second exit flap | LOW: the inner door's role is inferred |

The strong rows: the Ha'tak plazas (A-12, A-13: prefab positions exact, platform on the mesh at a constant 0.19 m offset), Rak'nor's plaza spot (A-06: the floor is the DHD's own slab, the identity is the weak part), respawner 25 (B-01), the Ba'al hologram platform (B-07).

## What this corrects in the audit

- **Cover nodes:** "at least 8,022, 99 tiles with parse errors" is wrong; the Rust reader parses all 401 tiles with no error and finds **9,847** (1,643 sets, 36 chunks).
- **Nearest tents:** the audit's six trigger-based groups start 200 m from the gate. Four furnished camps stand 42 to 125 m out ([census](C-landmark-census.md#the-four-furnished-camps)).
- **Rings "under" the Ha'taks:** they are 35 to 40 m to one side of them.
- **The gate prefab "static mesh at (96.3, -12.5, 252.6)":** that is the gate ring pieces at y -12.5; the base mesh is at (96.5, -16.85, 250.9) and the prefab origin at (99.7, -16.85, 252.7). The row is a stand point 1.69 m above the base. Not a defect.

Confirmed unchanged: 401 + 2 tiles, the actor census, no `PlayerStart`, 699 and 6 navmesh components, the story room's contents, the Kismet scope, the Ha'tak sequence names.

## How to correct these in one pass

After the owner's playtest, send corrected coordinates keyed by row id (`PL-DK-A-nn`, `PL-DK-B-nn`). Nothing is generated and nothing is seeded yet: DK-04 and DK-05 write the seed rows from this ledger, so a correction before they land edits this file, and one after edits the seed row each packet names. Rows are not ids in any seed.
