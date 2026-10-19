# UO-01: Omega navmesh-sealed placements

> **Type:** Reference (worknote) · **Packet:** [UO-01](../UO-01.md) · **Provenance:** CIMMERIA_RECONSTRUCTION (coordinates taken from the shipped client navmesh); art areas are RESTORED_SOURCE layout intent only (D-UO15)

Every row below was probed with `nav_inspect` against `data/spaces/omega_site.nav` (world 18, anchor `modi_seed78` 39.54 -51.91 -129.92) or `data/spaces/omega_site_cmdcenter.nav` (world 80, anchor `pad17_cmd` 6.16 34.52 10.08). `y` is the snapped floor (probe y minus the printed `dy`), re-probed at that y. Later packets copy these coordinates verbatim.

## How to read the table

- **Candidate**: for `hub.*`, `svc.registrar*` and world-80 `cast.*` the letter or grid label from the packet. For other `svc.*` and `amb.*` rows `rR aA` means a point R metres from the packet's prop or area centre at compass bearing A degrees (0 = +z, 90 = +x), because the packet lists the prop, not a standing point.
- **Exit**: `nav_inspect` exit code of that single point against the set anchor (0 = on the mesh, within tolerance, same component). **Exit (pad 14)** repeats the check with pad 14 (34.56 -36.48 55.68) as anchor, proving reachability from the Command Center beam arrival; world 80 has no pad-14 equivalent (`n/a`).
- **Heading** = `atan2(dx, dz)` from the NPC to the thing it faces, in [0, 2pi), 4 decimals. Faces: `hub.*` and registrars face Gate 5 / the arrival point; services face their prop; ambient rows face the area centre (the prop or console); `cast.tealc` faces Bra'tac, other cast face pad 17.
- **Nearest NPC** = nearest other placed point or existing spawn (Modi 78, DHD 41, ring spawns 132-137, Gate 5, pad 17, respawner 26) in the same world, x/z distance.
- **Art area** = nearest of the six anchors in x/z (D-UO15). **Art agrees** compares it with the expected area per key.

## Placements

| Key | World | x | y | z | Heading | Candidate | Exit | Exit (pad 14) | Nearest NPC | Art area (nearest anchor) | Art agrees |
|---|---|---|---|---|---|---|---|---|---|---|---|
| `hub.arrival` | 18 | 34.70 | -51.28 | -142.00 | 3.1409 | B | 0 | 0 | 4.0 m (hub.respawner) | common_area (Gate 5) | yes |
| `hub.respawner` | 18 | 34.70 | -51.42 | -138.00 | 3.1411 | A | 0 | 0 | 4.0 m (hub.arrival) | common_area (Gate 5) | yes |
| `svc.registrar` | 18 | 30.00 | -51.75 | -130.00 | 2.7683 | x30z-130 | 0 | 0 | 4.5 m (svc.registrar_command) | common_area (Gate 5) | yes |
| `svc.registrar_command` | 18 | 28.00 | -52.03 | -126.00 | 2.7450 | x28z-126 | 0 | 0 | 4.5 m (svc.registrar) | common_area (Gate 5) | yes |
| `svc.banker` | 18 | -88.06 | -48.02 | -102.92 | 0.0000 | r1.75a180 | 0 | 0 | 3.2 m (svc.human_crafting) | sgc (Ring region 12 Left) | no |
| `svc.human_weapons` | 18 | -168.25 | -36.42 | -166.00 | 4.7124 | r1.75a90 | 0 | 0 | 3.3 m (svc.human_armor) | sgc (Ring region 12 Left) | yes |
| `svc.human_armor` | 18 | -169.59 | -36.42 | -168.97 | 3.1416 | r1.75a0 | 0 | 0 | 3.3 m (svc.human_weapons) | sgc (Ring region 12 Left) | yes |
| `svc.human_crafting` | 18 | -87.97 | -48.02 | -99.76 | 4.7124 | r1.75a90 | 0 | 0 | 3.2 m (svc.banker) | sgc (Ring region 12 Left) | n/a |
| `svc.common_crafting` | 18 | -105.12 | -47.82 | -87.93 | 1.5708 | r1.75a270 | 0 | 0 | 20.8 m (svc.human_crafting) | sgc (Ring region 12 Left) | n/a |
| `svc.consumables` | 18 | -147.00 | -38.88 | -169.40 | 4.7124 | r1.75a90 | 0 | 0 | 4.2 m (amb.cafeteria.3) | sgc (Ring region 12 Left) | n/a |
| `svc.jaffa_weapons` | 18 | 390.35 | -84.80 | -44.56 | 4.7124 | r1.75a90 | 0 | 0 | 108.1 m (amb.jaffa_market.3) | free_jaffa (Ring region 16 Right) | yes |
| `svc.jaffa_armor` | 18 | 216.50 | -63.59 | -66.17 | 0.0000 | r1.75a180 | 0 | 0 | 38.8 m (amb.arena.1) | free_jaffa (Ring region 16 Right) | yes |
| `svc.jaffa_crafting` | 18 | 291.14 | -60.90 | -109.35 | 1.5708 | r1.75a270 | 0 | 0 | 6.6 m (amb.jaffa_market.1) | free_jaffa (Ring region 16 Right) | n/a |
| `amb.cafeteria.1` | 18 | -146.25 | -38.02 | -173.73 | 5.7596 | r5a150 | 0 | 0 | 4.4 m (svc.consumables) | sgc (Ring region 12 Left) | yes |
| `amb.cafeteria.2` | 18 | -142.69 | -39.23 | -165.90 | 4.1886 | r7a60 | 0 | 0 | 5.6 m (svc.consumables) | sgc (Ring region 12 Left) | yes |
| `amb.cafeteria.3` | 18 | -150.25 | -39.08 | -166.80 | 2.6183 | r3a330 | 0 | 0 | 4.2 m (svc.consumables) | sgc (Ring region 12 Left) | yes |
| `amb.cafeteria.4` | 18 | -148.75 | -38.60 | -162.40 | 3.1416 | r7a0 | 0 | 0 | 4.6 m (amb.cafeteria.3) | sgc (Ring region 12 Left) | yes |
| `amb.south_bunks.1` | 18 | -79.66 | -30.82 | -210.23 | 4.1888 | r5a60 | 0 | 0 | 4.0 m (amb.south_bunks.3) | common_area (Ring region 11 Bottom) | yes |
| `amb.south_bunks.2` | 18 | -86.49 | -31.02 | -208.40 | 2.6180 | r5a330 | 0 | 0 | 3.7 m (amb.south_bunks.4) | common_area (Ring region 11 Bottom) | yes |
| `amb.south_bunks.3` | 18 | -76.20 | -30.90 | -208.23 | 4.1886 | r9a60 | 0 | 0 | 4.0 m (amb.south_bunks.1) | common_area (Ring region 11 Bottom) | yes |
| `amb.south_bunks.4` | 18 | -83.99 | -31.02 | -205.73 | 3.1416 | r7a0 | 0 | 0 | 3.7 m (amb.south_bunks.2) | common_area (Ring region 11 Bottom) | yes |
| `amb.bunker_outpost.1` | 18 | -215.63 | -35.38 | -39.30 | 4.1888 | r5a60 | 0 | 0 | 7.1 m (amb.bunker_outpost.2) | sgc (Ring region 12 Left) | yes |
| `amb.bunker_outpost.2` | 18 | -217.46 | -35.45 | -46.13 | 5.7596 | r5a150 | 0 | 0 | 7.1 m (amb.bunker_outpost.3) | sgc (Ring region 12 Left) | yes |
| `amb.bunker_outpost.3` | 18 | -224.29 | -35.05 | -44.30 | 1.0472 | r5a240 | 0 | 0 | 7.1 m (amb.bunker_outpost.2) | sgc (Ring region 12 Left) | yes |
| `amb.bunker_outpost.4` | 18 | -223.46 | -35.23 | -35.74 | 2.6178 | r7a330 | 0 | 0 | 8.6 m (amb.bunker_outpost.3) | sgc (Ring region 12 Left) | yes |
| `amb.jaffa_market.1` | 18 | 297.22 | -61.16 | -106.85 | 4.1888 | r5a60 | 0 | 0 | 2.6 m (amb.jaffa_market.4) | free_jaffa (Ring region 16 Right) | yes |
| `amb.jaffa_market.2` | 18 | 297.39 | -60.82 | -117.14 | 5.7594 | r9a150 | 0 | 0 | 7.8 m (amb.jaffa_market.4) | free_jaffa (Ring region 16 Right) | yes |
| `amb.jaffa_market.3` | 18 | 300.68 | -61.23 | -104.85 | 4.1886 | r9a60 | 0 | 0 | 4.0 m (amb.jaffa_market.1) | free_jaffa (Ring region 16 Right) | yes |
| `amb.jaffa_market.4` | 18 | 297.89 | -61.23 | -109.35 | 4.7124 | r5a90 | 0 | 0 | 2.6 m (amb.jaffa_market.1) | free_jaffa (Ring region 16 Right) | yes |
| `amb.arena.1` | 18 | 222.69 | -63.63 | -27.90 | 5.7596 | r5a150 | 0 | 0 | 7.1 m (amb.arena.2) | free_jaffa (Ring region 16 Right) | yes |
| `amb.arena.2` | 18 | 215.86 | -63.63 | -26.07 | 1.0472 | r5a240 | 0 | 0 | 5.8 m (amb.arena.4) | free_jaffa (Ring region 16 Right) | yes |
| `amb.arena.3` | 18 | 226.25 | -63.63 | -20.07 | 4.1886 | r7a60 | 0 | 0 | 7.6 m (amb.arena.4) | free_jaffa (Ring region 16 Right) | yes |
| `amb.arena.4` | 18 | 218.69 | -63.59 | -20.97 | 2.6183 | r3a330 | 0 | 0 | 5.8 m (amb.arena.2) | free_jaffa (Ring region 16 Right) | yes |
| `amb.library.1` | 18 | 99.78 | -26.62 | -237.58 | 5.7596 | r5a150 | 0 | 0 | 5.0 m (amb.library.4) | common_area (Gate 5) | yes |
| `amb.library.2` | 18 | 101.61 | -26.62 | -230.75 | 4.1888 | r5a60 | 0 | 0 | 2.6 m (amb.library.4) | common_area (Gate 5) | yes |
| `amb.library.3` | 18 | 95.78 | -26.62 | -230.65 | 2.6183 | r3a330 | 0 | 0 | 5.8 m (amb.library.2) | common_area (Gate 5) | yes |
| `amb.library.4` | 18 | 102.28 | -26.27 | -233.25 | 4.7124 | r5a90 | 0 | 0 | 2.6 m (amb.library.2) | common_area (Gate 5) | yes |
| `cast.jackson` | 80 | 2.12 | 35.09 | 9.99 | 1.5478 | A | 0 | n/a | 3.5 m (cast.thor) | world_80 | n/a |
| `cast.bratac` | 80 | 10.00 | 34.91 | 13.50 | 3.9848 | A | 0 | n/a | 2.0 m (cast.tealc) | world_80 | n/a |
| `cast.tealc` | 80 | 12.00 | 35.32 | 13.50 | 4.7124 | A | 0 | n/a | 2.0 m (cast.bratac) | world_80 | n/a |
| `cast.thor` | 80 | 2.12 | 35.09 | 13.50 | 2.2733 | A | 0 | n/a | 3.5 m (cast.jackson) | world_80 | n/a |
| `amb.console_ne.1` | 80 | 87.99 | 30.82 | -21.75 | 5.7599 | r3a150 | 0 | n/a | 4.2 m (amb.console_ne.2) | world_80 | n/a |
| `amb.console_ne.2` | 80 | 89.09 | 30.82 | -17.65 | 4.1891 | r3a60 | 0 | n/a | 4.2 m (amb.console_ne.1) | world_80 | n/a |
| `amb.console_nw.1` | 80 | -77.81 | 30.82 | -16.55 | 2.6183 | r3a330 | 0 | n/a | 4.2 m (amb.console_nw.2) | world_80 | n/a |
| `amb.console_nw.2` | 80 | -78.91 | 30.82 | -20.65 | 1.0475 | r3a240 | 0 | n/a | 4.2 m (amb.console_nw.1) | world_80 | n/a |
| `amb.console_se.1` | 80 | 85.89 | 30.82 | 36.63 | 5.7599 | r3a150 | 0 | n/a | 3.0 m (amb.console_se.2) | world_80 | n/a |
| `amb.console_se.2` | 80 | 87.39 | 30.82 | 39.23 | 4.7124 | r3a90 | 0 | n/a | 3.0 m (amb.console_se.1) | world_80 | n/a |
| `amb.console_sw.1` | 80 | -76.81 | 30.82 | 37.73 | 1.0475 | r3a240 | 0 | n/a | 3.0 m (amb.console_sw.2) | world_80 | n/a |
| `amb.console_sw.2` | 80 | -74.21 | 30.82 | 36.23 | 0.0000 | r3a180 | 0 | n/a | 3.0 m (amb.console_sw.1) | world_80 | n/a |

49 rows: 37 in world 18 and 12 in world 80. Every row exits 0.

## C4

The one arrival coordinate for UO-00 to publish (candidate B, outside the gate's event-horizon cylinder, on the +z plaza side where the dais is level):

- World 18: **x 34.70, y -51.28, z -142.00**, heading 3.1409 (faces Gate 5; same value as the table row), component 60.
- 13.0 m from Modi (39.54, -129.92), 11.0 m from the DHD (42.60, -134.31), 7.1 m from the gate row (34.705, -149.092), 4.0 m from respawner 180 (`hub.respawner`).
- Probe y -51.86 printed `dy` -0.58 (the floor is 0.58 m above the packet's guess); the y above is the snapped floor.

## Rejected

Candidates that failed or were not used, with the reason. "Off the mesh" means the point sits outside every polygon in x/z (`h` > 0) or beyond the 0.05 m floor tolerance after re-snapping; those are never used even when `nav_inspect` calls them `ok` inside its own 2 m tolerance.

### World 18 hub

- `hub.arrival` A (34.53, -50.23, -146.59) and gate-front (34.70, -51.91, -138.0): both pass (exit 0) but were not needed; B passed first. A sits 2.5 m from the gate row, in the volume the earlier crossing note warned about, so it stays the last resort.
- `hub.respawner` B (37.0, -51.86, -136.0) and C (31.0, -51.86, -136.0): both pass but were not needed; A (34.70, -138.0) is 4.0 m from the arrival point (rule: at least 3 m).
- Registrar grid (x 26-32, z -134..-118, step 2): 30 of 36 points usable. Not on a polygon: (26,-118), (28,-124), (28,-122), (28,-120), (28,-118), (30,-126). The chosen pair is (30,-130) and (28,-126), 4.5 m apart, 9.5 m and 11.5 m from Modi.
- Banker: the packet y (-51.96) is 3.94 m below the floor (-48.02), so every first-pass probe at that y was OUT OF TOLERANCE (v-tol 3.0). The prop coordinate is a model origin, not a floor. All world-18 probes were re-snapped (y minus `dy`) and re-run before selection.
- Human Weapons: the prop point itself snaps onto component 95, not the hub component 60. 21 of 36 ring points around it are also on other components; the chosen point (-168.25, -166.00) is on component 60 and 1.75 m from the prop. Treat the prop as unreachable and the chosen standing point as the NPC position.

Service ring candidates per service (36 each: radii 1.5, 1.75, 2.0 at 12 bearings), rejected in total:

| Service | Another component | Off the mesh | Different floor level |
|---|---|---|---|
| banker | 14 | 0 | 0 |
| human_weapons | 21 | 0 | 0 |
| human_armor | 0 | 8 | 0 |
| human_crafting | 0 | 20 | 0 |
| common_crafting | 0 | 30 | 0 |
| consumables | 0 | 2 | 0 |
| jaffa_weapons | 0 | 17 | 0 |
| jaffa_armor | 3 | 19 | 0 |
| jaffa_crafting | 0 | 28 | 0 |

Among the survivors, the one nearest 1.75 m from the prop (ties broken towards Gate 5) that keeps 2.5 m from every earlier placement was taken. `svc.common_crafting` had only 6 usable ring points.

Ambient ring candidates (48 per area: radii 3, 5, 7, 9 at 12 bearings), rejected: cafeteria 19 off the mesh; south_bunks 27 off; bunker_outpost 14 off; jaffa_market 1 other component and 25 off; arena 12 off; library 14 other component and 18 off. Points within 3 m of any service point, or 2.5 m of an earlier point, were skipped, and the floor had to be within 0.8 m of the area's median.

### World 80

- Cast: every packet candidate passed (all component 11, exit 0). The first candidate (A) was taken for all four; B and C remain valid fallbacks. Distances: Jackson 4.0 m from pad 17, Bra'tac 5.1 m, Teal'c 6.8 m and Thor 5.3 m (rule: 3 m); nearest to respawner 26 is Bra'tac at 3.5 m (rule: 1.5 m); cast spacing 2.0 m minimum (rule: 1.5 m).
- **Bridges E and W: no ambient placement.** Bridge E (144.78, 10.22) is on component 12 and bridge W (-134.60, 10.22) on component 26. The pad/respawner component is 11. `nav_inspect --gap-pair 11,12` and `11,26` report no route within 3 m horizontally or vertically at any hop count, so a probe there exits 3. Candidates around both bridges also landed on components 23, 35, 22 and 31 (walkable islands at other heights). `amb.bridge_e.*` and `amb.bridge_w.*` therefore have no row.
- Consoles: 25 ring candidates landed on other components and 63 were off the mesh; two points per console were taken at radius 3 m, spaced 3.0-4.2 m apart.

## Art conflicts

- `svc.banker`: nearest anchor area **sgc** (Ring region 12 Left, 122.55 m); expected **common_area**. Distances to the two nearest anchors: Ring region 12 Left 122.55 m, Ring region 10 Center 125.51 m.

The disagreement is the architect's predicted one. The mesh position is kept (the bank prop is at (-88.06, -101.17); the chosen standing point is 1.75 m to its -z side). The nearest-anchor rule is a coarse proxy with only six anchors, so the 3 m margin between ring 12 and ring 10 is inside its error: the orchestrator should rule whether to keep the point, which is physically the only reachable standing point beside that prop.

No other row disagrees. Rows marked `n/a` are consumables (the art draws them in every area) and the three crafting vendors (the art draws none); their areas are recorded only: consumables, human crafting and common crafting all fall in `sgc`, jaffa crafting in `free_jaffa`.

## nav_inspect summaries

One run per set, anchor first; every set exits 0. Packet probe sets are re-run here at the final coordinates.

```text
hub_arrival: anchor modi_seed78; 2 probes (incl. anchor), 2 ok, max|dy| 0.09, exit 0
hub_respawner: anchor modi_seed78; 2 probes (incl. anchor), 2 ok, max|dy| 0.09, exit 0
hub_registrar: anchor modi_seed78; 3 probes (incl. anchor), 3 ok, max|dy| 0.09, exit 0
hub_services: anchor modi_seed78; 10 probes (incl. anchor), 10 ok, max|dy| 0.09, exit 0
hub_ambient: anchor modi_seed78; 25 probes (incl. anchor), 25 ok, max|dy| 0.09, exit 0
hub_services_from_pad14: anchor pad14; 10 probes (incl. anchor), 10 ok, max|dy| 0.79, exit 0
hub_all_from_pad14: anchor pad14; 38 probes (incl. anchor), 38 ok, max|dy| 0.79, exit 0
w80_cast: anchor pad17_cmd; 5 probes (incl. anchor), 5 ok, max|dy| 0.62, exit 0
w80_ambient: anchor pad17_cmd; 9 probes (incl. anchor), 9 ok, max|dy| 0.62, exit 0
```

Raw output of the first run (candidate scan, hub arrival and respawner candidates, anchor modi_seed78) for reference: `poly=6000 component=60` for Modi, DHD, arrival B/gate-front, respawner A/B/C; `poly=5789 component=60` for arrival A; pad 14 `component=60 dy=-0.79 ok`. The world-80 anchor `pad17_cmd` is `component=11 dy=-0.62 ok`; mesh 18 has 247 components (component 60 holds 124,722 m2), mesh 80 has 45.

## Spacing check

Existing world-18 and world-80 rows from `git grep -n -E ", (18|80), [0-9]+, '" db/resources/Worlds/Seed/`: spawns 78 (Modi), 41 (DHD), 132-136 (ring beams, world 18) and 137 (world 80 beam), plus the Gate 5 row and respawner 26. The nearest existing row to any new world-18 point is Modi or the DHD (the registrars, 9.5 m and above). Every chosen point keeps at least 2.5 m from every other (world 80 cast 1.5 m); the tightest world-18 pairs are `amb.jaffa_market.1`/`.4`, `amb.library.2`/`.4` and `amb.library.4`/`.2` at 2.6 m, and `svc.banker`/`svc.human_crafting` at 3.2 m.

## Notes for later packets

- World 80 is instanced per arrival and the whole mesh floor is about Y 35. Use the y values in the table; a world-18-style Y lands far under the floor.
- Bridges E and W (world 80) are not reachable from pad 17; do not seed NPCs there without a nav patch.
- `svc.human_weapons` prop is on a separate component (95); keep the NPC at the listed standing point.
