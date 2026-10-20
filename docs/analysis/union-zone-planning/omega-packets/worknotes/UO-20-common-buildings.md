# UO-20: Common Area service and guide placements (worknote)

## Done

- Landmarks: `extract_actors` (13,091 actors, `--json`) and `archetype_census` (261 positioned meshes, 106 inside x -110..170, z -230..70) on `Omega_Site`. Actor and mesh names are generic (`StaticMeshActor`, `PrefabInstance`, `ASG-Props:...`), so no infirmary, inn, battlemaster or pod archetype is named. Search starts came from the art map (F34), the same linear map UO-19 used. Probe files are under `.claude\reports\union-omega\probes\`, not committed.
- Probes: `ca_buildings_pass1.txt` and `ca_buildings_pass2.txt` (ring candidates, 1.5, 1.75 and 2.0 m, 12 bearings, two passes at snapped heights); `ca_medgrid_pass1.txt` and `ca_medgrid_pass2.txt` (1 m grid x 26..62, z -10..26); `ca_neg_pass1.txt` and `ca_neg_pass2.txt` (1 m grid x 58..90, z 14..42); `ca_final_probes.txt` (anchors first, all chosen points and the death probe; exit 0).
- The Infirmary, Battlemaster and Inn art spots are on the upper level (y about -36) or in comp 181 pockets. The mesh does not reach the Infirmary art spot, so the medic and Battlemaster sit at the nearest comp-60 walkway (see § Art conflicts).
- Spacing checked against UO-19's § Points and § Science Center, the seeded world-18 spawns in the box (63820, 63821, 63840, 78, 41, 132, 133, 135), respawner 180 (34.70, -138.00), and ambient 63880-63897 (world 18).

## § Points

| Key | Spawn | Template | Tag | x | y | z | Heading | Candidate | Exit (Modi) | Exit (pad 14) | Nearest NPC | Nearest respawner | Nearest ambient | Art spot | Art area | Art agrees |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ca.guide.biology` | 63810 | 63829 | `UnionOmega_Guide_Biology` | 32.00 | -35.97 | -22.00 | 0.1132 | UO-19 free spot (32,-22) | ok | ok | 63966 (7.21) | 181: 25.96 (180: 116.0) | 63896 (183.91) | Science Center, Biology (548,676) → (32.26, -21.10) | common_area | yes, 0.94 m |
| `ca.guide.electronics` | 63811 | 63829 | `UnionOmega_Guide_Electronics` | 80.00 | -31.90 | 24.00 | 4.2270 | e_80_24 | ok | ok | 63813 (3.00) | 181: 46.07 (180: 168.2) | 63896 (144.79) | NE large pod, Electronics (832,410) → (80.54, 24.10) | common_area | yes, 0.54 m |
| `ca.guide.adv_electronics` | 63812 | 63829 | `UnionOmega_Guide_AdvElectronics` | 11.89 | -43.87 | 45.20 | 2.6778 | advelec_1.5_270 | ok | ok | 135 (25.00) | 181: 50.16 (180: 184.6) | 63896 (216.06) | NW standalone pod (437,286) → (13.39, 45.20) | common_area | yes, 1.50 m |
| `ca.guide.engineering` | 63813 | 63829 | `UnionOmega_Guide_Engineering` | 80.00 | -32.51 | 27.00 | 4.1768 | e_80_27 | ok | ok | 63811 (3.00) | 181: 47.51 (180: 171.1) | 63896 (145.86) | NE large pod, Engineering (806,368) → (76.12, 31.24) | common_area | no, 5.75 m (2.5-3.5 m from 63811 forces it) |
| `ca.guide.adv_engineering` | 63814 | 63829 | `UnionOmega_Guide_AdvEngineering` | 63.37 | -44.10 | 43.17 | 3.7310 | adveng_1.5_180 | ok | ok | 63813 (23.20) | 181: 46.98 (180: 183.4) | 63896 (167.47) | NE standalone pod (731,289) → (63.37, 44.67) | common_area | yes, 1.50 m |
| `ca.guide.physics` | 63815 | 63829 | `UnionOmega_Guide_Physics` | 44.00 | -36.06 | -22.00 | 5.8756 | UO-19 free spot (44,-22) | ok | ok | 63810 (12.00) | 181: 25.50 (180: 116.4) | 63896 (171.91) | Science Center, Physics (618,676) → (44.16, -21.10) | common_area | yes, 0.91 m |
| `ca.medic` | 63818 | 63855 | `UnionOmega_Medic` | 39.00 | -44.56 | 7.00 | 3.7129 | g_39_7 | ok | ok | 63823 (2.83) | 181: 4.00 (180: 145.1) | 63896 (179.93) | Infirmary (618,507) → (44.16, 7.60) | common_area | no, 5.19 m (art spot is comp 181) |
| `ca.inn` | 63822 | 63856 | `UnionOmega_Inn` | 32.00 | -44.45 | 8.00 | 2.8387 | g_32_8 | ok | ok | 63823 (5.10) | 181: 8.60 (180: 146.0) | 63896 (186.99) | Inn (546,507) → (31.92, 7.60) | common_area | yes, 0.41 m |
| `ca.battlemaster` | 63823 | 63857 | `UnionOmega_Battlemaster` | 37.00 | -44.33 | 9.00 | 3.4125 | g_37_9 | ok | ok | 63818 (2.83) | 181: 6.32 (180: 147.0) | 63896 (182.27) | Battlemaster (588,477) → (39.06, 12.70) | common_area | no, 4.23 m (art spot is comp 181) |
| `ca.respawner_infirmary` | respawner 181 | - | name `Omega Infirmary` | 39.00 | -44.90 | 3.00 | - | g_39_3 | ok | ok | 63818 (4.00); Modi 132.9 m; C4 arrival 145.1 m | - | 63896 (179.2) | inside or beside the Infirmary; 6.9 m from the Infirmary art spot | common_area | n/a |

Art areas: every row's nearest anchor is Ring region 14 (34.575, 55.698), at 25-78 m, or Ring region 10 Center (34.41, -75.46), at 53-110 m; both are `common_area`.

Probe-level check: `ca_final_probes.txt`, 13 probes (anchors and 11 points), exit 0, every row `component=60 h=0.00 dy=0.00 ok` (the anchors dy -0.09 and -0.79).

## § SQL

Heading is `atan2(dx, dz)` from each NPC to the hub centre (34.5, 0), normalised to [0, 2π). No entrance or walkway coordinate is named for the buildings, so every service NPC and guide faces the hub centre (the UO-19 precedent).

### For UO-33

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63818, 39.00, -44.56, 7.00, 3.7129, 18, 63855, 'UnionOmega_Medic', NULL, true); -- ca.medic
```

### For UO-34

```sql
INSERT INTO respawners (respawner_id, world_id, name, pos_x, pos_y, pos_z) VALUES (181, 18, 'Omega Infirmary', 39.00, -44.90, 3.00);
```

### For UO-35

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63810, 32.00, -35.97, -22.00, 0.1132, 18, 63829, 'UnionOmega_Guide_Biology', NULL, true); -- ca.guide.biology
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63811, 80.00, -31.90, 24.00, 4.2270, 18, 63829, 'UnionOmega_Guide_Electronics', NULL, true); -- ca.guide.electronics
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63812, 11.89, -43.87, 45.20, 2.6778, 18, 63829, 'UnionOmega_Guide_AdvElectronics', NULL, true); -- ca.guide.adv_electronics
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63813, 80.00, -32.51, 27.00, 4.1768, 18, 63829, 'UnionOmega_Guide_Engineering', NULL, true); -- ca.guide.engineering
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63814, 63.37, -44.10, 43.17, 3.7310, 18, 63829, 'UnionOmega_Guide_AdvEngineering', NULL, true); -- ca.guide.adv_engineering
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63815, 44.00, -36.06, -22.00, 5.8756, 18, 63829, 'UnionOmega_Guide_Physics', NULL, true); -- ca.guide.physics
```

### For UO-36

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63822, 32.00, -44.45, 8.00, 2.8387, 18, 63856, 'UnionOmega_Inn', NULL, true); -- ca.inn
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63823, 37.00, -44.33, 9.00, 3.4125, 18, 63857, 'UnionOmega_Battlemaster', NULL, true); -- ca.battlemaster
```

## § Death probes

- `INFIRMARY_DEATH` (38.00, -44.67, 6.00), on component 60, h 0.00, 1.41 m from 63818. Distance to respawner 181 (39.00, 3.00): 3.16 m. Distance to respawner 180 (34.70, -138.00): 144.0 m. A death here resolves to 181.

## § Landmarks

Census and actor list name none of the Infirmary, Inn, Battlemaster or the pods. The art spots map to the hub by the linear map (F34), used as search starts only.

| Art spot | World landmark used | x, z | Source |
|---|---|---|---|
| Infirmary (618,507) | none named; art map start (44.16, 7.60) lies on the comp-181 upper pocket | 44.16, 7.60 | art map; mesh picks 39, 7 |
| Inn (546,507) | none named; art map start | 31.92, 7.60 | art map; mesh picks 32, 8 |
| Battlemaster (588,477) | none named; art map start (comp-181 pocket) | 39.06, 12.70 | art map; mesh picks 37, 9 |
| NE large pod, Electronics (832,410) | none named; art map start | 80.54, 24.10 | art map; mesh picks 80, 24 |
| NE large pod, Engineering (806,368) | none named; art map start | 76.12, 31.24 | art map; mesh picks 80, 27 |
| NE standalone pod (731,289) | none named; art map start | 63.37, 44.67 | art map; mesh picks 63.37, 43.17 |
| NW standalone pod (437,286) | none named; art map start | 13.39, 45.20 | art map; mesh picks 11.89, 45.20 |
| Science Center Biology / Physics | UO-19 free spots | 32, -22 / 44, -22 | UO-19 § Science Center |
| Pad 14 | `ASG-TeleportBase00` census instance | 34.56, 55.68 | census |

## § Rejected

- Infirmary and Battlemaster art spots: all 36 ring probes at 1.5 m around (44.16, 7.60) and all 36 around (39.06, 12.70) are comp 181 at y about -36.0. Not on Modi's component; the mesh decides.
- Engineering at the art spot (76.12, 31.24): the ring points on comp 60 are 4.9-6.5 m from the Electronics point (80, 24) and fail the 2.5-3.5 m pod spacing. Chosen (80, 27), 3.0 m from Electronics.
- Ring points on comp 181, 174, 187 and 191 near Electronics and the medic: fail the anchors.
- Medic and Battlemaster points within 2.5 m of 63823 and 63818 were not used; the chosen pair is 2.83 m apart.

## § Art conflicts

- Infirmary (medic 63818): the art spot (44.16, 7.60) is on comp 181 (upper pocket); the walkable comp-60 point nearest it is (39, 7), 5.2 m away. The mesh decides (rule 8).
- Battlemaster (63823): art spot (39.06, 12.70) is comp 181; the chosen point (37, 9) is 4.2 m away.
- NE large pod, Engineering (63813): art spot (76.12, 31.24) is 5.8 m from the chosen (80, 27), forced by the 2.5-3.5 m pod spacing from Electronics (63811).
- NW and NE standalone pods (63812, 63814) and Electronics (63811): within 1.5 m of the art map.
- Respawner 181: the art has no spot for it; it sits 4.0 m from 63818 and 6.9 m from the Infirmary art spot.

## § nav_inspect summaries

- Pass 1, ring candidates at y -40: 254 probes, components 247, exit 3 (expected: mixed components at the wrong floor).
- Pass 2, snapped heights: 161 probes, exit 0, 159 ring points comp 60 h 0.00 ok.
- Medic grid pass 1: 1,369 probes, exit 3 (expected); comp 60 787, comp 181 518, comp 187 56, comp 172 10. Pass 2 at snapped heights: 762 probes, exit 0, 756 comp 60 h 0.00 ok.
- Guide grid (NE pods) pass 1: 957 probes; pass 2: 870 rows, exit 0, 850 comp 60 h 0.00 ok.
- Final check: 13 probes (modi_seed78, pad14, the 11 points), exit 0, all comp 60 h 0.00 ok.

## Next step

- UO-33, UO-34, UO-35, UO-36 read the § SQL and § Death probes blocks. Not yet committed as seed rows.
