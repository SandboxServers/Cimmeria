# UO-17: SGC wing placement seal (worknote, session 2)

> **Type:** Reference (worknote) · **Packet:** [UO-17](../UO-17.md) · **Status:** done, 21 points sealed · **Provenance:** CIMMERIA_RECONSTRUCTION (client navmesh and map); art is RESTORED_SOURCE layout intent only (D-UO15)

## Status

- All 21 points exit 0 against both anchors (`modi_seed78`, `pad14`): component 60, `h` 0.00, `dy` -0.00, status ok. Last check: nav_inspect final re-probe, `exit=0` for both runs (fail count 0).
- Heights were snapped to the probe floor (`y` = probe y minus printed `dy`) and re-probed. Four points reuse UO-01 standing points unchanged.
- Probe files and the package index are in `.claude\reports\union-omega\probes\`, not committed.

## § Points

| Key | Spawn | Template | Tag | x | y | z | Heading | Candidate | Exit (Modi) | Exit (pad 14) | Nearest NPC | Nearest respawner | Nearest ambient | Art spot | Art area | Art agrees |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `hweap.t1` | 63841 | 63812 | `UnionOmega_HumanWeaponsT1` | -168.25 | -36.42 | -166.00 | 4.7124 | UO-01 svc.human_weapons | 0 | 0 | 3.01 m (hweap.t2) | 204.87 m (180) | 18.02 m (cafe3) | north-row Weapon | sgc (Ring 12 Left) | yes |
| `harmor.t1` | 63847 | 63816 | `UnionOmega_HumanArmorT1` | -169.59 | -36.42 | -168.97 | 3.1416 | UO-01 svc.human_armor | 0 | 0 | 3.26 m (hweap.t1) | 206.62 m (180) | 19.46 m (cafe3) | north-row Armor | sgc (Ring 12 Left) | yes |
| `craft.biomedical` | 63960 | 63820 | `UnionOmega_HumanCraftingBiomed` | -87.97 | -48.02 | -99.76 | 4.7124 | UO-01 svc.human_crafting | 0 | 0 | 2.98 m (craft.electronics) | 128.49 m (180) | 3.16 m (banker) | none (group near svc.human_crafting) | sgc (Ring 12 Left) | yes |
| `cons.1` | 63868 | 63828 | `UnionOmega_Consumables_Sgc1` | -147.00 | -38.88 | -169.40 | 4.7124 | UO-01 svc.consumables | 0 | 0 | 4.16 m (cafe3) | 184.39 m (180) | 4.16 m (cafe3) | NW-block north face (289,572) | sgc (Ring 12 Left) | yes |
| `hweap.t2` | 63842 | 63813 | `UnionOmega_HumanWeaponsT2` | -166.00 | -36.42 | -164.00 | 5.0644 | 2.14 m from N-row weapons target (-165.25, -166.00) | 0 | 0 | 3.01 m (hweap.t1) | 202.38 m (180) | 16.00 m (cafe3) | north-row Weapon | sgc (Ring 12 Left) | yes |
| `harmor.t2` | 63848 | 63817 | `UnionOmega_HumanArmorT2` | -166.00 | -36.42 | -169.00 | 5.7364 | 0.75 m from N-row armor target (-165.25, -169.00) | 0 | 0 | 3.59 m (harmor.t1) | 203.08 m (180) | 15.90 m (cafe3) | north-row Armor | sgc (Ring 12 Left) | yes |
| `hweap.t3` | 63843 | 63814 | `UnionOmega_HumanWeaponsT3` | -112.00 | -47.82 | -92.00 | 2.7008 | 0.00 m from central block target (-112.00, -92.00) | 0 | 0 | 3.00 m (hweap.t4) | 153.74 m (180) | 26.31 m (banker) | SE-block Weapon (355,634) | sgc (Ring 12 Left) | yes |
| `hweap.t4` | 63844 | 63815 | `UnionOmega_HumanWeaponsT4` | -112.00 | -47.82 | -95.00 | 1.8514 | 0.00 m from central block target (-112.00, -95.00) | 0 | 0 | 3.00 m (hweap.t3) | 152.87 m (180) | 25.22 m (banker) | SE-block Weapon (355,634) | sgc (Ring 12 Left) | yes |
| `harmor.t3` | 63849 | 63818 | `UnionOmega_HumanArmorT3` | -108.00 | -47.82 | -92.00 | 3.7390 | 0.00 m from central block target (-108.00, -92.00) | 0 | 0 | 3.00 m (harmor.t4) | 149.93 m (180) | 22.73 m (banker) | SE-block Armor (368,663) | sgc (Ring 12 Left) | yes |
| `harmor.t4` | 63850 | 63819 | `UnionOmega_HumanArmorT4` | -108.00 | -47.82 | -95.00 | 4.5152 | 0.00 m from central block target (-108.00, -95.00) | 0 | 0 | 3.00 m (harmor.t3) | 149.04 m (180) | 21.46 m (banker) | SE-block Armor (368,663) | sgc (Ring 12 Left) | yes |
| `craft.electronics` | 63961 | 63821 | `UnionOmega_HumanCraftingElec` | -85.00 | -48.02 | -100.00 | 4.3472 | 0.24 m from craft group target (-84.97, -99.76) | 0 | 0 | 2.83 m (craft.power) | 125.59 m (180) | 4.23 m (banker) | none (group near svc.human_crafting) | common_area (Ring 10 Center) | no |
| `craft.power` | 63962 | 63822 | `UnionOmega_HumanCraftingPowerSys` | -83.00 | -48.02 | -98.00 | 4.1527 | 2.04 m from craft group target (-81.97, -99.76) | 0 | 0 | 2.83 m (craft.electronics) | 124.31 m (180) | 7.06 m (banker) | none (group near svc.human_crafting) | common_area (Ring 10 Center) | no |
| `craft.materials` | 63963 | 63823 | `UnionOmega_HumanCraftingMat` | -78.00 | -48.20 | -99.00 | 4.4999 | 1.23 m from craft group target (-78.97, -99.76) | 0 | 0 | 5.10 m (craft.power) | 119.26 m (180) | 10.80 m (banker) | none (group near svc.human_crafting) | common_area (Ring 10 Center) | no |
| `cons.2` | 63869 | 63828 | `UnionOmega_Consumables_Sgc2` | -104.00 | -47.82 | -100.00 | 0.5493 | 0.00 m from central east target (-104.00, -100.00) | 0 | 0 | 6.40 m (harmor.t4) | 143.81 m (180) | 16.21 m (banker) | central block, east face | sgc (Ring 12 Left) | yes |
| `cons.3` | 63870 | 63828 | `UnionOmega_Consumables_Sgc3` | -96.00 | -48.03 | -110.00 | 1.0514 | 0.00 m from SE south target (-96.00, -110.00) | 0 | 0 | 10.64 m (banker) | 133.67 m (180) | 10.64 m (banker) | SE block, south end | sgc (Ring 12 Left) | yes |
| `trn.soldier` | 63804 | 63804 | `UnionOmega_SoldierTrainer` | -80.00 | -48.02 | -125.00 | 4.4621 | 0.00 m from east trainer row target (-80.00, -125.00) | 0 | 0 | 3.00 m (trn.scientist) | 115.43 m (180) | 23.51 m (banker) | Soldier (432,667) | common_area (Ring 11 Bottom) | no |
| `trn.scientist` | 63805 | 63805 | `UnionOmega_ScientistTrainer` | -80.00 | -48.02 | -128.00 | 4.6464 | 0.00 m from east trainer row target (-80.00, -128.00) | 0 | 0 | 3.00 m (trn.soldier) | 115.14 m (180) | 26.34 m (banker) | Scientist (279,605) | common_area (Ring 11 Bottom) | no |
| `trn.commando` | 63806 | 63806 | `UnionOmega_CommandoTrainer` | -80.00 | -48.02 | -131.00 | 4.8353 | 0.00 m from east trainer row target (-80.00, -131.00) | 0 | 0 | 3.00 m (trn.scientist) | 114.91 m (180) | 29.21 m (banker) | Commando (432,693) | common_area (Ring 11 Bottom) | no |
| `trn.archaeologist` | 63807 | 63807 | `UnionOmega_ArchaeologistTrainer` | -80.00 | -48.02 | -134.00 | 5.0158 | 0.00 m from east trainer row target (-80.00, -134.00) | 0 | 0 | 3.00 m (trn.commando) | 114.77 m (180) | 32.11 m (banker) | Archaeologist (305,678) | common_area (Ring 11 Bottom) | no |
| `guide.physics` | 63816 | 63829 | `UnionOmega_Guide_Physics` | -125.00 | -46.57 | -120.00 | 3.6465 | 0.00 m from tent target (-125.00, -120.00) | 0 | 0 | 2.83 m (guide.biology) | 160.71 m (180) | 40.70 m (banker) | Adv. Physics (207,605) | sgc (Ring 12 Left) | yes |
| `guide.biology` | 63817 | 63829 | `UnionOmega_Guide_Biology` | -123.00 | -46.43 | -122.00 | 2.1377 | 2.83 m from tent target (-125.00, -124.00) | 0 | 0 | 2.83 m (guide.physics) | 158.51 m (180) | 39.81 m (banker) | Adv. Biology (231,624) | sgc (Ring 12 Left) | yes |

## § SQL

### For UO-11

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63841, -168.25, -36.42, -166.00, 4.7124, 18, 63812, 'UnionOmega_HumanWeaponsT1', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63842, -166.00, -36.42, -164.00, 5.0644, 18, 63813, 'UnionOmega_HumanWeaponsT2', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63843, -112.00, -47.82, -92.00, 2.7008, 18, 63814, 'UnionOmega_HumanWeaponsT3', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63844, -112.00, -47.82, -95.00, 1.8514, 18, 63815, 'UnionOmega_HumanWeaponsT4', NULL, true);
```

### For UO-27

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63847, -169.59, -36.42, -168.97, 3.1416, 18, 63816, 'UnionOmega_HumanArmorT1', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63848, -166.00, -36.42, -169.00, 5.7364, 18, 63817, 'UnionOmega_HumanArmorT2', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63849, -108.00, -47.82, -92.00, 3.7390, 18, 63818, 'UnionOmega_HumanArmorT3', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63850, -108.00, -47.82, -95.00, 4.5152, 18, 63819, 'UnionOmega_HumanArmorT4', NULL, true);
```

### For UO-29

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63960, -87.97, -48.02, -99.76, 4.7124, 18, 63820, 'UnionOmega_HumanCraftingBiomed', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63961, -85.00, -48.02, -100.00, 4.3472, 18, 63821, 'UnionOmega_HumanCraftingElec', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63962, -83.00, -48.02, -98.00, 4.1527, 18, 63822, 'UnionOmega_HumanCraftingPowerSys', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63963, -78.00, -48.20, -99.00, 4.4999, 18, 63823, 'UnionOmega_HumanCraftingMat', NULL, true);
```

### For UO-30

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63868, -147.00, -38.88, -169.40, 4.7124, 18, 63828, 'UnionOmega_Consumables_Sgc1', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63869, -104.00, -47.82, -100.00, 0.5493, 18, 63828, 'UnionOmega_Consumables_Sgc2', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63870, -96.00, -48.03, -110.00, 1.0514, 18, 63828, 'UnionOmega_Consumables_Sgc3', NULL, true);
```

### For UO-31

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63804, -80.00, -48.02, -125.00, 4.4621, 18, 63804, 'UnionOmega_SoldierTrainer', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63805, -80.00, -48.02, -128.00, 4.6464, 18, 63805, 'UnionOmega_ScientistTrainer', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63806, -80.00, -48.02, -131.00, 4.8353, 18, 63806, 'UnionOmega_CommandoTrainer', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63807, -80.00, -48.02, -134.00, 5.0158, 18, 63807, 'UnionOmega_ArchaeologistTrainer', NULL, true);
```

### For UO-35

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63816, -125.00, -46.57, -120.00, 3.6465, 18, 63829, 'UnionOmega_Guide_Physics', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63817, -123.00, -46.43, -122.00, 2.1377, 18, 63829, 'UnionOmega_Guide_Biology', NULL, true);
```

## § Art conflicts


The seven keys below have nearest anchor other than sgc (D-UO15 rule); the mesh decided, they are kept:

- `craft.electronics`, `craft.power`, `craft.materials`: nearest anchor Ring region 10 Center (common_area) 121.91, 119.55, 114.85 m; sgc anchor (Ring region 12 Left) 125.38, 127.28, 132.32 m. The art draws no crafting vendor here.
- `trn.soldier`, `trn.scientist`, `trn.commando`, `trn.archaeologist`: nearest anchor Ring region 11 Bottom (common_area) 102.88, 100.49, 98.13, 95.81 m; sgc anchor 134.19, 134.95, 135.77, 136.66 m. The east trainer row sits in the Common Area's nearest-anchor cell, so the art's trainer row does not match the mesh side of the wing.

## § Landmarks

Census: `archetype_census` over `Maps\Omega_Site` (positions TSV, nav x/z = bw x/z, checked against spawn 134 on `ASG-TeleportBase00` at (-210.24, -92.32)). The art pixel to nav calibration is inferred, not verified; the worknote session 1 cluster z values (-40 to -60) disagree with the census (z -75 to -135), so the census is used.

| Art block | World landmark (nav x, z) | Source |
|---|---|---|
| North row (weapon and armor t1-t2) | UO-01 `svc.human_weapons` / `svc.human_armor` at (-168, -166..-169), next to `EM-WallLight01` x5 near (-173, -160) | UO-01 worknote; census |
| Central block (weapon and armor t3-t4, consumable.2) | `EM-AC_unit00` x4 (-117, -102), `EM-Hut_Med01` (-110.5, -87.5), `EM-PropaneTank00` (-100, -93.4) | census |
| SE block, south end (consumable.3) | `EM-SecurityCam01_Arm` x4 (-93, -108), `EM-ComputerTower00` (-91.4, -94.1) | census |
| Crafting group | `EM-Quartermaster01` (-88.06, -101.17) with UO-01 `svc.human_crafting` | census; UO-01 |
| Trainer row (east edge) | no named prop at x -80, z -125..-134; nearest `JF-HighWallArch_ruined00` | inferred |
| Guides (Adv. Physics, Adv. Biology) | `EM-Tent_large00` (-133, -135) area; points at (-125, -120) and (-123, -122) | inferred |

## § Rejected

- Grid 1 m over x -225..-65, z -215..-45, probed at floors -37 and -48 (55,062 probes; 12,548 on component 60 with `h` 0.00 and `dy` within 3 m). Floors were snapped as probe y minus `dy`.
- Component 95: `UO-01` weapons prop floor (-39.42) and other stacked floors are on other components; not used.
- Hweap.t2 first candidate (-166.00, -166.00) was 2.25 m from `hweap.t1`, rejected by the 2.5 m rule; the next nearest survivor (-166.00, -164.00) was taken.
- Respawner 181 (Common Area, UO-20) is not in the seed tree; its distance is not measured here.
- Spacing checks ran against the seed rows, the UO-01 standing points (not yet seeded), spawns 132-134, Modi and respawner 180. Zero violations of 2.5 m (NPC and pair) and 3.0 m (ambient).

## § nav_inspect summaries

- Anchor line, modi run: modi_seed78          poly=6000   component=60   h=0.00 m  dy=-0.09 m  ok
- Anchor line, pad14 run: pad14                poly=10235  component=60   h=0.00 m  dy=-0.79 m  ok
- Both runs: exit 0; 21 of 21 points ok.