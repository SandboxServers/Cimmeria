# Union Beta E1 placement seal (UB-00b)

Status: sealed 2026-10-18 by movement-teleport-advisor. Docs plus read-only nav probes; no code or seed edits, no lab use. The owner waived the U-D03/FZ-99 start gate for Beta.

Evidence classes: RESTORED_SOURCE (cooked client map data, read-only census), CIMMERIA_RECONSTRUCTION (existing server code or seeds), CIMMERIA_AUTHORING (a placement chosen here and nav-verified). Every coordinate in the table is CIMMERIA_AUTHORING; the prefab origins quoted in the notes are RESTORED_SOURCE.

## Method and caveats

- Mesh: `data/spaces/beta_site_evo_1.nav`, tiled XRCT, agent radius 0.6, height 1.8, climb 0.6 (see the raw output). Probe tool: `nav_inspect --probes FILE --h-tol 0.3 --v-tol 1.0` (`crates/navmesh-extractor/src/bin/nav_inspect.rs`).
- `dy = probe.y - surface.y`, so snapped y = y - dy; positive dy means the probe floats above the mesh. ARRIVAL (dy +0.70) snaps to -60.83.
- "Reachable from ARRIVAL" is the same-component proxy plus h within 0.3 m. nav_inspect has no path option, so same-component is necessary, not sufficient. Downstream tests must confirm with `NavMesh::is_point_valid` and `find_path`.
- The mesh is speckled, so each point was found by search: standable at the claimed y, and at least 6 of 8 neighbours at 0.6 m on the same component with |dy| < 0.4.
- Authored client coordinates are prefab origins, so each point is a snapped stand point near the origin (the origin itself is often a prefab hole).
- Heading is yaw in radians, facing (dx, dz) = atan2(dx, dz); 1.571 faces +X. Headings face the approach or the object an NPC attends to; they are authoring, not source data. Blank means region centre or not applicable.
- Region semantics (`crates/cell-catalog/src/cell/spawner/regions.rs:30,107`, CIMMERIA_RECONSTRUCTION): a one-point AreaSet is a box at +-r in X/Z; `is_point_in_region` widens it by `GENERIC_REGION_CHECK_THRESHOLD = 1.5` on every axis. "Outside by N" means Chebyshev distance >= r + 1.5 + N.
- Spec: `docs/analysis/union-zone-planning/beta-packets/UB-00.md` section UB-00b (lines 95-152).

## Seal table

Columns: anchor, x, y, z, heading, nav component, snapped y (mesh surface), reachable from ARRIVAL.

| Anchor | x | y | z | heading | comp | snapped y | reachable from ARRIVAL |
|---|---|---|---|---|---|---|---|
| ARRIVAL | 635.761 | -60.13 | 388.448 | 1.571 | 251 | -60.83 | yes (same component as ARRIVAL) |
| RESPAWN_6 | 633.000 | -60.83 | 399.000 | 4.127 | 251 | -60.83 | yes (same component as ARRIVAL) |
| LIFE_GATE_1 | 626.000 | -60.83 | 399.000 | 2.941 | 251 | -60.83 | yes (same component as ARRIVAL) |
| LIFE_GATE_2 | 642.500 | -60.83 | 374.500 | 5.198 | 251 | -60.83 | yes (same component as ARRIVAL) |
| CARTER | 1011.405 | 13.57 | 511.851 | 2.498 | 251 | 13.57 | yes (same component as ARRIVAL) |
| FELGER | 1007.405 | 13.57 | 511.851 | 2.246 | 251 | 13.57 | yes (same component as ARRIVAL) |
| HUB_COMPUTER | 1018.405 | 13.90 | 506.851 | 5.482 | 251 | 13.90 | yes (same component as ARRIVAL) |
| ADMIN_WORKSTATION | 1018.405 | 13.57 | 509.851 | 5.262 | 251 | 13.57 | yes (same component as ARRIVAL) |
| ADMIN_CENTRE | 1018.405 | 13.90 | 506.851 |  | 251 | 13.90 | yes (same component as ARRIVAL) |
| STORAGE_CENTRE | 1022.327 | 13.37 | 271.180 |  | 251 | 13.37 | yes (same component as ARRIVAL) |
| XENOCAGE_1 | 712.389 | -66.99 | 411.698 | 1.277 | 251 | -66.99 | yes (same component as ARRIVAL) |
| XENOCAGE_2 | 765.860 | -63.85 | 426.136 | 1.277 | 251 | -63.85 | yes (same component as ARRIVAL) |
| XENOCAGE_3 | 826.331 | -36.48 | 446.574 | 1.277 | 251 | -36.48 | yes (same component as ARRIVAL) |
| XENOCAGE_4 | 884.801 | -16.69 | 464.012 | 1.277 | 251 | -16.69 | yes (same component as ARRIVAL) |
| XENOCAGE_5 | 938.272 | -4.48 | 481.450 | 1.277 | 251 | -4.48 | yes (same component as ARRIVAL) |
| CRATE_ARC42 | 1007.015 | 13.37 | 275.820 | 1.864 | 251 | 13.37 | yes (same component as ARRIVAL) |
| CRATE_ARC66 | 1004.015 | 13.43 | 275.820 | 1.818 | 251 | 13.43 | yes (same component as ARRIVAL) |
| SATTERFIELD | 1004.144 | 13.44 | 276.690 | 1.864 | 251 | 13.44 | yes (same component as ARRIVAL) |
| FIRE_SUPPRESSANT_MANAGER | 1019.327 | 13.37 | 271.180 | 1.564 | 251 | 13.37 | yes (same component as ARRIVAL) |
| FIRE_CONTROL_HUB | 1022.327 | 13.37 | 268.180 | 6.274 | 251 | 13.37 | yes (same component as ARRIVAL) |
| LIFE_QM_1 | 1002.433 | 13.44 | 281.388 | 2.045 | 251 | 13.44 | yes (same component as ARRIVAL) |
| LIFE_QM_2 | 999.113 | 13.57 | 273.732 | 1.680 | 251 | 13.57 | yes (same component as ARRIVAL) |
| LIFE_CMD_1 | 1000.000 | 13.57 | 500.000 | 1.353 | 251 | 13.57 | yes (same component as ARRIVAL) |
| LIFE_CMD_2 | 1000.000 | 13.57 | 497.000 | 1.196 | 251 | 13.57 | yes (same component as ARRIVAL) |
| LIFE_HUT_1 | 1033.700 | 7.37 | 361.349 | 0.994 | 251 | 7.37 | yes (same component as ARRIVAL) |
| LIFE_HUT_2 | 1033.700 | 7.37 | 424.563 | 0.974 | 251 | 7.37 | yes (same component as ARRIVAL) |
| RUIN_NOTE | 122.200 | 27.17 | 356.800 | 1.249 | 251 | 27.17 | yes (same component as ARRIVAL) |
| HOLOGRAM_EMITTER | 961.300 | 15.97 | 284.900 | 5.016 | 251 | 15.97 | yes (same component as ARRIVAL) |
| DISABLED_DRONE | 959.800 | 15.97 | 284.900 | 5.078 | 251 | 15.97 | yes (same component as ARRIVAL) |
| INFIRMARY_CENTRE | 961.300 | 15.97 | 284.900 |  | 251 | 15.97 | yes (same component as ARRIVAL) |
| RESPAWN_7 | 953.000 | 16.35 | 287.500 | 1.874 | 251 | 16.35 | yes (same component as ARRIVAL) |
| LIFE_SCI_1 | 622.600 | 20.64 | 777.000 | 0.000 | 251 | 20.64 | yes (same component as ARRIVAL) |
| BEAM_GROUND_DEST | 745.050 | -19.60 | 68.750 | 1.674 | 251 | -19.60 | yes (same component as ARRIVAL) |
| ASGARD_SCANNER_1 | 740.480 | -19.43 | 68.640 | 1.577 | 251 | -19.43 | yes (same component as ARRIVAL) |
| ASGARD_SCANNER_2 | 746.480 | -19.54 | 62.640 | 0.003 | 251 | -19.54 | yes (same component as ARRIVAL) |
| ASGARD_SCANNER_3 | 746.480 | -19.80 | 74.640 | 3.138 | 251 | -19.80 | yes (same component as ARRIVAL) |
| ASGARD_SCANNER_4 | 752.480 | -20.20 | 68.640 | 4.706 | 251 | -20.20 | yes (same component as ARRIVAL) |
| ASGARD_SCANNER_5 | 740.480 | -19.43 | 62.640 | 0.790 | 251 | -19.43 | yes (same component as ARRIVAL) |
| SCIENCE_COMPUTER | 649.700 | 19.77 | 814.100 | 1.571 | 890 | 19.77 | gap-linked: 0.90 m door gap to 251 (find_path to confirm) |
| FOUR_PIECES_CASE | 647.700 | 19.77 | 814.100 | 1.571 | 890 | 19.77 | gap-linked: 0.90 m door gap to 251 (find_path to confirm) |
| BEAM_DECK_DEST | 698.980 | 157.93 | 70.360 | 1.571 | 355 | 157.93 | no (deck, beam only) |
| MIMIR | 696.440 | 157.77 | 67.360 | 0.927 | 355 | 157.77 | no (deck, beam only) |
| ANALYTICAL_MACHINE | 696.440 | 157.77 | 64.360 | 0.588 | 355 | 157.77 | no (deck, beam only) |
| CONSCIOUSNESS_SIM | 699.440 | 157.93 | 67.360 | 0.322 | 355 | 157.93 | no (deck, beam only) |
| SPAWN_151 | 700.480 | 157.91 | 70.365 |  | 355 | 157.91 | no (deck, beam only) |
| RAS_NOR | -545.100 | 6.77 | 141.500 | 4.957 | 70 | 6.77 | no walk route (Q1); beam/ring only |
| SICK_JAFFA_1 | -547.100 | 6.80 | 138.500 | 0.588 | 70 | 6.80 | no walk route (Q1); beam/ring only |
| SICK_JAFFA_2 | -554.100 | 6.77 | 142.500 | 1.681 | 70 | 6.77 | no walk route (Q1); beam/ring only |
| SICK_JAFFA_3 | -552.100 | 6.77 | 138.500 | 1.166 | 70 | 6.77 | no walk route (Q1); beam/ring only |
| JAFFA_BOX | -546.100 | 6.77 | 146.500 | 2.944 | 70 | 6.77 | no walk route (Q1); beam/ring only |
| NINEPEAKS_CENTRE | -549.100 | 6.77 | 142.500 |  | 70 | 6.77 | region centre at the pin, not a stand point (3.50 m off mesh); no walk route (Q1); beam/ring only |
| LIFE_CAMP_1 | -557.100 | 6.77 | 145.500 | 1.893 | 70 | 6.77 | no walk route (Q1); beam/ring only |
| LIFE_CAMP_2 | -562.100 | 6.77 | 150.500 | 2.058 | 70 | 6.77 | no walk route (Q1); beam/ring only |
| LIFE_CAMP_3 | -555.100 | 6.77 | 142.500 | 1.670 | 70 | 6.77 | no walk route (Q1); beam/ring only |
| SW_RING | -791.600 | -51.80 | -92.600 |  | 70 | -51.43 | no walk route (Q1); beam/ring only |
| DEBEAUX | 1034.146 | 13.38 | 314.764 | 3.142 | 251 | 13.38 | yes (same component as ARRIVAL) |

SPAWN_151 and SW_RING are reference rows, not anchors: spawn 151 is the deck beam spawn, SW_RING is the Q1 ring candidate.

## Regions (centre, radius, height)

The box is +-r in X/Z and the 4th corner carries +h (regions.rs). Centres are table rows.

| Region | centre | r | h | Containment checks (Chebyshev) |
|---|---|---|---|---|
| Storage | STORAGE_CENTRE (1022.327, 13.37, 271.180), the EM-Quartermaster01 origin | 20 | 6 | Satterfield 18.2 and crates 15.3-18.3 inside. FLAG: it nests FireSite entirely, so a player in the fire zone is also in Storage |
| AdminBuilding | ADMIN_CENTRE (1018.405, 13.90, 506.851) | 14 | 6 | Carter 7.0, Felger 11.0, HubComputer 0, AdminWorkstation 3.0 inside. LIFE_CMD_1/2 at 18.4 are outside (needs more than 15.5) |
| Infirmary | INFIRMARY_CENTRE (961.300, 15.97, 284.900) | 10 | 6 | Emitter 0, drone 1.5 inside. RESPAWN_7 at 8.3 is inside too (Q5) |
| FireSite | (1022.3, 14.5, 271.2), the consumer default | 12 | 6 | Manager 3.0 and hub 3.0 inside. Satterfield 18.2 >= 15.5 + 2: outside by 4.7. LIFE_QM_1 19.9 and LIFE_QM_2 23.2 >= 17.5 (r + 1.5 + 4): outside |
| NinePeaks | NINEPEAKS_CENTRE (-549.1, 6.77, 142.5) | 20 | 6 | Nine Peaks NPCs within 9 m; LIFE_CAMP_1..3 within 13 m |

## Q1: west valley 251 vs 70

No route. Raw (`nav_inspect --gap-pair`, h<=500 m, v<=150 m):

```
component 251 (612686 m^2, 15124 polys) <-> 70 (361261 m^2, 8652 polys)
    direct approaches, closest first:
      h=483.86 m  dy=-26.60 m   (115.0, 37.4, 147.5) -> (-359.6, 10.8, 53.3)
```

Searches at h<=80/v<=40 and h<=200/v<=100 found no direct link. The only chain is 251, 883, 907, 926, 978, 0, 91, 70: 7 hops, widest 18.48 m, total 74.81 m, with 15-17.8 m vertical hops, so not walkable. Verdict: no route; nearest ring candidate on 70 is -791.6, -51.8, -92.6 (SW_RING, component 70, h 0.00, dy -0.37). It pairs with ring 148 (146.716, 8.591, 403.273, nav 251) in UB-02d Part B. Nine Peaks content is reached by beam or ring only. D-UB4 stays with the owner; not chosen here.

## Q2: HELD-B3 infirmary prefab

Census of the cooked map (RESTORED_SOURCE, read-only extraction): EM-Infirmary01 and EM-Infirmary01_Interior are both at (961.281, 14.320, 284.851) on tile 00020009, so the prefab is on tile 00020009. That the "SGC Administrative Complex Infirmary" text maps to this prefab is an inference (a name match, no source link): INFERRED. Neighbours: EM-Infirmary00-Tower (955.33, 14.38, 259.47), EM-Infirmary00-Generator (980.36, 14.32, 247.91).

- HologramEmitter: (961.300, 15.97, 284.900), component 251, interior floor y 15.97.
- DisabledDrone: (959.800, 15.97, 284.900), 1.5 m west of the emitter.
- Covering region: Infirmary, centre (961.3, 15.97, 284.9), r 10, h 6.
- Door side: the nav alone cannot locate the door (interior block spans x about 955-963, z about 268-291). RESPAWN_7 is on the ARRIVAL-facing west side; INFERRED.

## Q3: HELD-B7 beam destinations

- BEAM_DECK_DEST (698.98, 157.93, 70.36): component 355, 1.50 m from spawn 151 (x, z as seeded; seeded y is 157.949997, the 157.91 used in the probe row is the snapped mesh y), standable, h 0.00.
- BEAM_GROUND_DEST (745.05, -19.60, 68.75): component 251, 1.50 m from spawn 149 (x, z as seeded; seeded y is -19.2099991, the -19.67 used in the probe is the snapped mesh y, so the seed floats 0.46 m above the mesh), standable, h 0.00.
- Both can be stood on. The deck (355) is not ARRIVAL's component: beam only.
- Deck walk path: MIMIR (696.44, 157.77, 67.36), ANALYTICAL_MACHINE (696.44, 157.77, 64.36) and CONSCIOUSNESS_SIM (699.44, 157.93, 67.36) are all on 355 with BEAM_DECK_DEST and spawn 151, so a same-component walk exists from the beam arrival to Mimir (proxy; confirm with find_path).
- Pads (RESTORED_SOURCE, census; values are agent-reported from `archetype_census` and were not re-run in the B0 review, which re-ran only the nav probes): ASG-TeleportBase00 ground pad (746.480, -19.920, 68.640), console (742.3, -19.5, 68.6); deck pad (700.440, 157.240, 70.360), console (696.26, 157.66, 70.32); ASG-MedStructure00 (716.0, 39.52, 70.4). These match spawns 149 and 151.

## Q4: HELD-B8 Debeaux and the location texts

- Census (RESTORED_SOURCE): EM-WeaponRange01 is at (1034.146, 13.175, 314.764), tile 0003000a. The spec pin (1055.6, 14.6, 334.3) is 29 m away, nothing in the census sits at it, and it probes on component 583 (dy -11.73, out of tolerance), not 251. Use the census origin.
- DEBEAUX: (1034.146, 13.375, 314.764), component 251, h 0.00, dy +0.00, heading 3.142 (authoring).
- Other location texts: Carter "Central Admin" is at the Command Post (EM-CommandPost01 and interior at (1017.405, 13.269, 503.851), tile 0005000a): consistent. Felger "Admin Storage" has no storage prefab of its own and is seated in the Command Post beside Carter. Satterfield "Artifact Storage" is seated at the Quartermaster (EM-Quartermaster01 (1022.327, 13.262, 271.180), tile 0002000a), the only storage-like prefab. That is the one conflict: Storage then nests FireSite. Pierce stays reserved; no anchor is placed for him.

## Q5: respawners

- RESPAWN_6 (gate side): (633.000, -60.83, 399.000), component 251, h 0.00, dy -0.00, reachable from ARRIVAL. Gate pin (617.866, -59.314, 388.971) and DHD pin (629.596, -60.913, 381.289) are both on 251.
- RESPAWN_7 (infirmary door side): (953.000, 16.35, 287.500), component 251, h 0.00, dy -0.00, about 8.3 m from the interior centre toward ARRIVAL. Door side INFERRED. It is inside the Infirmary region (8.3 < 11.5); intended, but nothing the region triggers may misfire on respawn.
- Both are seeded at (0,0,0) today; this seal proposes the positions. LIFE_GATE_1/2 are about 18.1 m and 14.6 m from the DHD pin and about 14.4 m and 15.5 m from ARRIVAL (review re-measure), so both clear the 6 m minimum from either.

## Other notes for consumers

- Science: EM-ScienceFac01_Front is at (622.647, 19.521, 796.279), tile 00070006. The spec pin z 768.3 is 28 m south of it, a probable transcription slip. The door gap is (651.7, 19.8, 814.1). SCIENCE_COMPUTER (649.7) and FOUR_PIECES_CASE (647.7) are 1.1 m and 3.1 m inside the edge of component 890 (the Science interior). 890 meets 251 across a 0.90 m door gap, (650.8, 19.8, 814.1) to (651.7, 19.8, 814.1) (`--gap-pair 890,251`). That gap is narrower than the 1.2 m agent width, so the find_path crossing is not guaranteed: UB-02 must confirm it. LIFE_SCI_1 is on 251, 19.3 m south of the Front origin.
- Ruins: the spec pin probes on component 609 (h 6.60). RUIN_NOTE is the nearest clear 251 point, (122.2, 27.17, 356.8), 6.3 m from the pin.
- Asgard: ASGARD_SCANNER_1..5 are on 251 around the ground pad, 6 m out.
- Life huts: LIFE_HUT_1/2 sit about 3 m from the EM-Hut_Med01 origins (1036.706, 7.122, 363.349) and (1036.718, 7.133, 426.563).
- Camp: the pin (-549.1, 6.5, 142.5) is near a torch at (-546.6, 6.56, 152.07). A separate Jaffa tent cluster (MilitaryTent00 at (-631.5, 1.3, 140.4), (-610.1, 1.35, 138.8), (-619.1, 1.33, 152.9)) lies about 70 m west at y about 1.3; not used.
- Xeno cages: five points along the gate-to-Command-Post line, all component 251, surface climbing from -66.99 to -4.48; all 40 line samples were 251.
- Not decided here: cell ownership and AoI fan-out of arrivals (bigworld-engine-advisor, aoi-witness-advisor), and D-UB4.

## Raw probe output

The header and component table are identical for every run and shown once.

```
header      tiled: tiles=4353 (38.4 x 38.4 m) nverts=150162 npolys=71138 max_tile_polys=193 cs=0.3 ch=0.2
agent       height=1.8 climb=0.6 radius=0.6
components  1119 (total walkable XZ area 4486982.9 m^2)
  251  15124      612686.2   [97.9, 1126.6] / [-103.2, 139.4] / [-136.3, 886.4]
  70   8652       361261.2   [-1141.4, -350.9] / [-51.6, 49.6] / [-429.4, 342.5]
probes (h-tol 0.30 m, v-tol 1.00 m)
=== A251 exit=0
  ARRIVAL              poly=42402  component=251  h=0.00 m  dy=+0.70 m  ok
  RESPAWN_6            poly=42399  component=251  h=0.00 m  dy=-0.00 m  ok
  LIFE_GATE_1          poly=42376  component=251  h=0.00 m  dy=-0.00 m  ok
  LIFE_GATE_2          poly=42403  component=251  h=0.00 m  dy=-0.00 m  ok
  CARTER               poly=48469  component=251  h=0.00 m  dy=-0.00 m  ok
  FELGER               poly=48470  component=251  h=0.00 m  dy=-0.00 m  ok
  HUB_COMPUTER         poly=48560  component=251  h=0.00 m  dy=+0.00 m  ok
  ADMIN_WORKSTATION    poly=48558  component=251  h=0.00 m  dy=-0.00 m  ok
  ADMIN_CENTRE         poly=48560  component=251  h=0.00 m  dy=+0.00 m  ok
  STORAGE_CENTRE       poly=37216  component=251  h=0.00 m  dy=-0.00 m  ok
  XENOCAGE_1           poly=44285  component=251  h=0.00 m  dy=+0.00 m  ok
  XENOCAGE_2           poly=44299  component=251  h=0.00 m  dy=-0.00 m  ok
  XENOCAGE_3           poly=44420  component=251  h=0.00 m  dy=-0.00 m  ok
  XENOCAGE_4           poly=46219  component=251  h=0.00 m  dy=+0.00 m  ok
  XENOCAGE_5           poly=46327  component=251  h=0.00 m  dy=-0.00 m  ok
  CRATE_ARC42          poly=37177  component=251  h=0.00 m  dy=-0.00 m  ok
  CRATE_ARC66          poly=37178  component=251  h=0.00 m  dy=-0.00 m  ok
  SATTERFIELD          poly=37178  component=251  h=0.00 m  dy=-0.00 m  ok
  FIRE_SUPPRESSANT_MANAGER poly=37216  component=251  h=0.00 m  dy=-0.00 m  ok
  FIRE_CONTROL_HUB     poly=37219  component=251  h=0.00 m  dy=-0.00 m  ok
  LIFE_QM_1            poly=37178  component=251  h=0.00 m  dy=-0.00 m  ok
  LIFE_QM_2            poly=37158  component=251  h=0.00 m  dy=-0.00 m  ok
  LIFE_CMD_1           poly=48427  component=251  h=0.00 m  dy=-0.00 m  ok
  LIFE_CMD_2           poly=48426  component=251  h=0.00 m  dy=-0.00 m  ok
  LIFE_HUT_1           poly=40886  component=251  h=0.00 m  dy=-0.00 m  ok
  LIFE_HUT_2           poly=44601  component=251  h=0.00 m  dy=-0.00 m  ok
  RUIN_NOTE            poly=40121  component=251  h=0.00 m  dy=-0.00 m  ok
  HOLOGRAM_EMITTER     poly=37109  component=251  h=0.00 m  dy=-0.00 m  ok
  DISABLED_DRONE       poly=37110  component=251  h=0.00 m  dy=-0.00 m  ok
  INFIRMARY_CENTRE     poly=37109  component=251  h=0.00 m  dy=-0.00 m  ok
  RESPAWN_7            poly=37107  component=251  h=0.00 m  dy=-0.00 m  ok
  LIFE_SCI_1           poly=57377  component=251  h=0.00 m  dy=+0.00 m  ok
  BEAM_GROUND_DEST     poly=28217  component=251  h=0.00 m  dy=+0.00 m  ok
  ASGARD_SCANNER_1     poly=28132  component=251  h=0.00 m  dy=-0.00 m  ok
  ASGARD_SCANNER_2     poly=26541  component=251  h=0.00 m  dy=+0.00 m  ok
  ASGARD_SCANNER_3     poly=28261  component=251  h=0.00 m  dy=-0.00 m  ok
  ASGARD_SCANNER_4     poly=28262  component=251  h=0.00 m  dy=-0.00 m  ok
  ASGARD_SCANNER_5     poly=26483  component=251  h=0.00 m  dy=-0.00 m  ok
=== S890 exit=0
  SCIENCE_COMPUTER     poly=58610  component=890  h=0.00 m  dy=-0.00 m  ok
  FOUR_PIECES_CASE     poly=58610  component=890  h=0.00 m  dy=-0.00 m  ok
=== D355 exit=0
  BEAM_DECK_DEST       poly=28051  component=355  h=0.00 m  dy=+0.00 m  ok
  MIMIR                poly=28053  component=355  h=0.00 m  dy=-0.00 m  ok
  ANALYTICAL_MACHINE   poly=28053  component=355  h=0.00 m  dy=-0.00 m  ok
  CONSCIOUSNESS_SIM    poly=28051  component=355  h=0.00 m  dy=+0.00 m  ok
  SPAWN_151            poly=28051  component=355  h=0.00 m  dy=-0.00 m  ok
=== N70 exit=2
  RAS_NOR              poly=30826  component=70   h=0.00 m  dy=-0.00 m  ok
  SICK_JAFFA_1         poly=29103  component=70   h=0.00 m  dy=+0.00 m  ok
  SICK_JAFFA_2         poly=30825  component=70   h=0.00 m  dy=-0.00 m  ok
  SICK_JAFFA_3         poly=29101  component=70   h=0.00 m  dy=-0.00 m  ok
  JAFFA_BOX            poly=30832  component=70   h=0.00 m  dy=-0.00 m  ok
  NINEPEAKS_CENTRE     poly=30826  component=70   h=3.50 m  dy=-0.00 m  OUT OF TOLERANCE
  LIFE_CAMP_1          poly=30823  component=70   h=0.00 m  dy=-0.00 m  ok
  LIFE_CAMP_2          poly=30814  component=70   h=0.00 m  dy=-0.00 m  ok
  LIFE_CAMP_3          poly=30825  component=70   h=0.00 m  dy=-0.00 m  ok
  SW_RING              poly=19815  component=70   h=0.00 m  dy=-0.37 m  ok
  DEBEAUX              poly=39188  component=251  h=0.00 m  dy=+0.00 m  ok   (separate run)
  RANGE_SPEC_PIN       poly=40918  component=583  h=0.00 m  dy=-11.73 m  OUT OF TOLERANCE   (spec pin, rejected)
```

The N70 run exits 2 only because NINEPEAKS_CENTRE (the camp pin, a region centre) is 3.50 m off the mesh; that is expected.

Science gap run (raw):

```
component 890 (2623 m^2, 12 polys) <-> 251 (612686 m^2, 15124 polys)
      h= 0.90 m  dy= +0.00 m   (650.8, 19.8, 814.1) -> (651.7, 19.8, 814.1)
    chain: 1 hop(s), widest 0.90 m, total 0.90 m
```

## Addendum UB-00b-2 (2026-10-19): ring 148 and SW_RING walk reachability, re-probe for UB-02d

Status: read-only nav probes by movement-teleport-advisor; no seed or code edits, no lab. Evidence classes: RESTORED_SOURCE (the shipped `data/spaces/beta_site_evo_1.nav` and the prefab origin of ring 148), CIMMERIA_RECONSTRUCTION (the server's `NavMesh::find_path` and the 6-of-8 stand method, run through a throwaway example that was deleted afterwards; nothing of it is committed).

### Verdict

Ring 148 is NOT on an island. The seal's ring point is walk-reachable from ARRIVAL, and so is SW_RING from the Nine Peaks camp. UB-02d's failing assertion is a test-helper artefact, not a placement fault. The seed values the coder used are correct.

### Why the test failed (CIMMERIA_RECONSTRUCTION)

`NavMesh::find_path` (`crates/entity/src/navigation/mod.rs:62`, `MAX_POLY_PATH = 256`) truncates the corridor at 256 polygons. The tiled mesh has small polygons, so a route of about 500 m ends early and the status is `Partial` with the last waypoint short of the goal. The `reaches` helper in `crates/cell-world/src/cell/spawner_tests/beta_e1/nav.rs` requires `PathStatus::Ok` and a last waypoint within 2 m, so every long route reads "different component".

Direct `find_path`, ring (146.716, 8.77, 403.273):

| route | status | last waypoint to goal |
|---|---|---|
| ARRIVAL -> ring | Partial | 220.07 m short, last (355.0, -62.0, 409.1) |
| ring -> ARRIVAL | Partial | 172.76 m short |
| CARTER -> ring | Partial | 435.27 m short |
| ARRIVAL -> CARTER (about 400 m, fewer polys) | Ok | 0.09 m |

Chaining legs (restart `find_path` from the previous leg's last waypoint) reaches the goal:

- ARRIVAL -> ring: 3 legs, final distance 0.00 m. Hops: ARRIVAL -> W1 (355.00, -62.03, 409.10) Ok; W1 -> W2 (169.60, 7.57, 386.00) Ok; W2 -> ring Ok (6 waypoints).
- ring -> ARRIVAL: 2 legs, final distance 0.70 m (ARRIVAL's own y is 0.70 above its snapped surface -60.83).
- Reverse hops are not all symmetric: W2 -> W1 and W1 -> ARRIVAL come back Partial (41 m and 71 m short) because the corridor cap bites again. A test must chain, or use hops, in the direction it needs, and should not assume symmetry.

### Ring 148 stand point (CIMMERIA_RECONSTRUCTION, 6-of-8 method)

- The prefab origin itself is a good stand point: (146.716, 8.770, 403.273), 6-of-8 neighbours at 0.6 m on the same surface with |dy| < 0.4, `is_point_valid` true. Snapped surface y is 8.770 (the coder's 8.77 is right; the seal's raw 8.591 was 0.18 m under the surface).
- Scan of +-18 m around it at 0.6 m steps: 1047 standable points, 889 walk-reachable from the ring (chain end), 158 not. The nearest unreachable standable point is 12.73 m from the ring, at (144.916, 7.811, 415.873), a separate pocket to the south. Every standable point within 12.7 m of the ring is reachable, which covers a region of r 2.53 + 1.5 check threshold = 4.03 m with a wide margin.
- Row to use: ring 2300 position (146.716, 8.770, 403.273), nav 251; point 23260 the same; travel set 23080 centre y 8.770, r 2.53, h 1.77 (unchanged from the coder's work).

### SW_RING stand point (CIMMERIA_RECONSTRUCTION)

- (-791.600, -51.430, -92.600): the seal probe reported dy -0.37 at y -51.80, so the snapped surface is -51.43 (seal-table "snapped y"); the coder's -51.43 is right. Use -51.430.
- camp (Ras'nor, -545.1, 6.77, 141.5) -> SW_RING: Ok, 24 waypoints, 0.00 m short, single leg. SW_RING -> camp: Ok, 20 waypoints. Both directions pass a plain `find_path` with no chaining.
- Scan of +-18 m: 2965 standable points, all 2965 reachable. No island risk on the SW side.

### What UB-02d should do (test and seeds)

1. Seeds: keep the coder's values (ring 2300 at 146.716, 8.770, 403.273; ring 2301 at -791.600, -51.430, -92.600; point sets and radii as written). No seed change is needed.
2. Test: replace the single-shot `reaches` for long routes. Two accepted options:
   - Hop test (preferred, deterministic): assert Ok and end within 2 m for ARRIVAL -> W1, W1 -> W2 and W2 -> ring with W1 (355.00, -62.03, 409.10) and W2 (169.60, 7.57, 386.00), and ring -> ARRIVAL via the ring -> (485.50, -97.23, 465.20) -> ARRIVAL chain. Name the waypoints as authored constants with a comment that `MAX_POLY_PATH` truncates a single query.
   - Chain helper: restart `find_path` from the last waypoint while the status is Partial and progress is above 0.5 m, cap at 8 legs, require a final Ok within 2 m. The helper must live in the test (nav.rs belongs to UB-02b), and a revert guard must show it fails for a point on an island (use the 12.73 m pocket (144.916, 7.811, 415.873) as the negative control: it must NOT chain to the ring).
3. The camp -> SW_RING direction needs no chaining; the existing `reaches` is enough.
4. Do not widen the 2.0 m tolerance and do not drop to a same-component proxy; the point of the test is a real walk.

### Side finding (not part of the ring decision)

RUIN_NOTE (122.2, 27.17, 356.8) is sealed as "yes (same component as ARRIVAL)", but chained `find_path` from ARRIVAL and from ring 148 both stall at (137.61, 7.77, 370.28), 28.2 m from it and about 19 m below it. The seal's proxy (same component, h within 0.3) holds; a real walk does not. It looks like an upper ruin level on component 251 that the corridor search cannot enter from the valley floor (or a gap larger than the agent climb). Whoever places the ruin note must re-probe with chained `find_path` before relying on a walk; this addendum does not decide it. The same chained check has not been run for the other 251 anchors in the seal table; ARRIVAL -> CARTER is confirmed Ok.

### Not decided here

D-UB4 stays ruled A (hub ring 148 <-> SW_RING). Cell ownership and AoI fan-out for ring arrivals: bigworld-engine-advisor and aoi-witness-advisor. The ring FSM timeout path is a separate UB-02d obligation (ring-transport state machine, owned in the ring seed/handler, not this probe).

## Addendum UB-00b-3 (2026-10-19): UB-02b hub anchors, chained walk re-check from ARRIVAL

Status: read-only nav probes by movement-teleport-advisor; no seed or code edits, no lab. Evidence classes: RESTORED_SOURCE (the shipped `data/spaces/beta_site_evo_1.nav`, the Science door gap and prefab origins quoted in UB-00b), CIMMERIA_RECONSTRUCTION (the server's `NavMesh::find_path`, `is_point_valid` and the 6-of-8 stand method, run through a throwaway example that was deleted afterwards; nothing of it is committed). Trigger: UB-02b review (B1b) reported spawn 23021 RuinNote as standable but not walk-reachable.

### Verdict

- RuinNote (23021) is NOT misplaced. The seal point (122.200, 27.17, 356.800) is walk-reachable from ARRIVAL. The stall at (137.61, 7.77, 370.28) is a property of the greedy chain, not of the placement. UB-00b-2's guess ("an upper ruin level the corridor search cannot enter") was wrong in its conclusion: the upper level is entered by a different route that the greedy chain never takes. No coordinate change; the test needs authored hops.
- Two anchors genuinely fail: ScienceComputer (23019) and FourPiecesCase (23020). Both sit on nav component 890 behind the 0.90 m door gap (narrower than the 1.2 m agent), so no `find_path` leg crosses it. Replacements below.
- The other 15 anchors (13 spawns, 2 region centres) pass: one direct `find_path` leg from ARRIVAL, status Ok, end within 0.30 m (Xeno1 0.60 m, see the y note). Nothing needs chaining except RuinNote.

### Why the greedy chain stalls (CIMMERIA_RECONSTRUCTION)

`find_path` returns `Partial` ending at the visited polygon nearest the goal in straight-line distance. Restarting from that endpoint (the UB-00b-2 chain) is greedy: toward the ruin it climbs the valley floor to W1 (355.00, -62.03, 409.10), then to the lower slope at (137.61, 7.77, 370.28), 19 m under the pin and a local minimum. The real route detours away from the goal first. Goal-directed legs exist: chaining toward an intermediate target on the upper level succeeds (ARRIVAL -> (131.80, 27.49, 324.40) in 4 legs), and from there the pin is one Ok leg. A test must therefore use authored hops, not "restart from the last waypoint".

Rule for every future chained test: a chain that stalls is not evidence of an island. Establish reachability with hops that each return Ok, and keep a negative control that really is an island.

### RuinNote hop list (RESTORED_SOURCE nav, CIMMERIA_RECONSTRUCTION hops)

Each leg is a single `find_path` with status Ok and last waypoint within 2.0 m (measured 0.00, 0.00, 0.00, 0.07 m):

| leg | from | to |
|---|---|---|
| 1 | ARRIVAL (635.761, -60.134, 388.448) | H1 (394.300, -84.630, 388.700) |
| 2 | H1 | H2 (169.600, -0.571, 324.400) |
| 3 | H2 | H3 (131.200, 27.570, 320.300) |
| 4 | H3 | RUIN_NOTE (122.200, 27.170, 356.800) |

- Standability: (122.200, 27.104, 356.800) passes 6-of-8 and `is_point_valid`; the seed y 27.17 is 0.07 above the surface. All 28 standable points in a 3.6 m box around the pin are Ok from H3.
- Direct ARRIVAL -> pin is `Partial`, 254.73 m short (corridor cap plus the same local minimum). A single-shot or greedy-chain test cannot pass this anchor whatever the seed is.
- The reverse is not symmetric: pin -> H3 -> H2 is Ok, but H2 -> H1 (74.47 m short) and H1 -> ARRIVAL (17.66 m short) are Partial. Do not assert the reverse walk.
- Route sanity: straight-path segments top out at slope 0.92 (about 43 degrees) on legs 2 and 3, 0.21 on leg 4. No off-mesh links were involved in this probe's output; the corridor is plain polygon adjacency.
- Label: placement RESTORED_SOURCE only to the extent UB-00b already sealed it (prefab-origin census, INFERRED Barak'Ka ruin); the stand point and hops are CIMMERIA_RECONSTRUCTION. Heading 1.249 unchanged.

Test guidance (replaces the "standable only" exemption for 23021): assert the four legs above with `PathStatus::Ok` and a 2.0 m end tolerance; keep the 2.0 m tolerance. Revert guard: the old greedy-chain result (137.605, 7.770, 370.280) is the negative control for "a greedy helper must not be used here" and the Science interior below is the negative control for a real island.

### Science pair: replacements (CIMMERIA_RECONSTRUCTION)

Seal rows SCIENCE_COMPUTER (649.700, 19.77, 814.100) and FOUR_PIECES_CASE (647.700, 19.77, 814.100) are 2.00 m and 4.00 m short of reachable: ARRIVAL -> (675.400, 15.770, 806.600) -> door edge (651.700, 19.770, 814.100) is Ok, the next leg across the 0.90 m gap is not (Partial, 2.00 m and 4.00 m short; the computer sits exactly on the 2.0 m tolerance edge, so it fails `< 2.0`).

Replacement stand points, outside the door on component 251 (each verified: standable 6-of-8, `is_point_valid`, ARRIVAL -> S1 Ok and S1 -> point Ok, end distance 0.00 m):

| Anchor | x | y | z | heading | note |
|---|---|---|---|---|---|
| SCIENCE_COMPUTER | 652.900 | 19.770 | 814.100 | 1.571 | 1.20 m east of the gap's 251 edge (651.70), 3.2 m east of the original |
| FOUR_PIECES_CASE | 654.900 | 19.770 | 814.100 | 1.571 | 2.00 m further east, same 2.0 m spacing as the original pair |

Hops: ARRIVAL (635.761, -60.134, 388.448) -> S1 (675.400, 15.770, 806.600) -> anchor. Leg 1 Ok 0.00 m (33 waypoints); leg 2 Ok 0.00 m (S1 -> 652.900: 12 waypoints, 10 for 654.900).

Radius and region guidance: neither anchor has a region. If a Science trigger is later added, centre it on SCIENCE_COMPUTER's new point with r >= 4.0 so a player standing at either object, or at the door edge (651.7), is inside after the 1.5 m check threshold; keep r under 10 so it does not reach the south wall. The interact range of the two objects was not measured: moving them 3.2 m and 5.2 m out of the building is a fidelity cost, and the interior positions are the better match to the client's prefab (UB-00b Other notes: the 890 interior is the Science facility). Alternative if the owner prefers fidelity: keep the interior rows, keep them standable-only, and assert only that ARRIVAL -> S1 -> door edge (651.700, 19.770, 814.100) is Ok. That is an owner choice (door gap versus strict walk gate); this addendum recommends the exterior rows because the packet gate is a strict `find_path` walk.

Label: the interior seal positions stay RESTORED_SOURCE-derived (prefab census); the exterior rows are CIMMERIA_RECONSTRUCTION and must be recorded as a deviation from the prefab's inside placement.

### Every other UB-02b anchor (direct ARRIVAL -> anchor, one `find_path` leg each)

All Ok, last waypoint distance from the seeded point in brackets: Carter 23000 (0.09), Felger 23001 (0.11), Satterfield 23002 (0.00), Debeaux 23003 (0.01), Xeno1 23010 (0.60), Xeno2 23011 (0.17), Xeno3 23012 (0.00), Xeno4 23013 (0.26), Xeno5 23014 (0.00), CrateArc42 23015 (0.00), CrateArc66 23016 (0.00), HubComputer 23017 (0.00), AdminWorkstation 23018 (0.00), region Storage centre (0.00), region AdminBuilding centre (0.00; same point as HubComputer). Point sets 23000 and 23001 hold one centre point each; both pass. A test can use a plain `reaches` for these; none needs a hop.

Minor y note (not a reachability failure): XENOCAGE_1 23010 is seeded at y -66.99, 0.60 m under the server mesh surface (-66.386 at that x, z), which fails the 6-of-8 stand check and shows as the 0.60 m end distance. `is_point_valid` still accepts it (below-surface tolerance). Optional hardening: y -66.386. Carter, Felger and Xeno4 differ by 0.09 to 0.26 m and need nothing.

### What UB-02b (and the test) should do

1. Seeds: change 23019 to (652.900, 19.770, 814.100) and 23020 to (654.900, 19.770, 814.100), headings 1.571; keep 23021 as seeded. Optionally y -66.386 for 23010. The in-file comments for 23019 to 23021 must say which are CIMMERIA_RECONSTRUCTION deviations.
2. Test: drop the `23019..=23021` standable-only exemption. the 13 passing spawns plus 2 regions use `standable_and_connected`; RuinNote uses the four-hop assertion; the Science rows use the S1 hop. Update the `HUB` constants to the new coordinates.
3. Do not widen the 2.0 m tolerance and do not use the greedy restart-from-last-waypoint chain for pass/fail on RuinNote: it fails there and would pass nothing.
4. Placement-seal table rows RUIN_NOTE, SCIENCE_COMPUTER and FOUR_PIECES_CASE: the "reachable from ARRIVAL" column should read "walk (hops H1..H3)" and "no: 890 behind a 0.90 m gap; exterior replacement" respectively; this addendum supersedes UB-00b-2's RUIN_NOTE side finding.

### Not decided here

Interact range and prefab fidelity of the Science objects (owner and mission-systems-advisor). Cell ownership and AoI of the hub objects (bigworld-engine-advisor, aoi-witness-advisor). Whether the client lets a player through the Science door (needs lab or a client check; the server's `is_point_valid` accepts the interior either way).
