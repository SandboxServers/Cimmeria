# UO-18: navmesh-sealed Free Jaffa wing placements (15 points, complete)

> **Type:** Reference (worknote) · **Packet:** [UO-18](../UO-18.md) · **Status:** done, coder session 2. Provenance: CIMMERIA_RECONSTRUCTION (coordinates from the shipped navmesh); art areas are RESTORED_SOURCE layout intent only (D-UO15).

## Done

- All 15 keys placed on `data/spaces/omega_site.nav` (component 60, `h` 0.00, `dy` 0.00 after snapping). Every chosen point exits 0 against both anchors (`modi_seed78`, `pad14`), with `nav_inspect` run from `B:\targets\uo-18\debug\nav_inspect.exe`.
- Spacing checked against the seeded world-18 rows (`git grep` on `db/resources`), the respawner 180 at (34.70, -138.00) and the ambient spawns 63880-63897.
- Probe files are in `.claude\reports\union-omega\probes\` (not committed). Snap rule used: `y` = probe `y` minus `dy`, then re-probed.

## Points

| Key | Spawn | Template | Tag | x | y | z | Heading | Candidate | Exit (Modi) | Exit (pad 14) | Nearest NPC | Nearest respawner | Nearest ambient | Art spot | Art area | Art agrees |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `fj.jweap.t1` | 63853 | 63831 | `UnionOmega_JaffaWeaponsT1` | 390.35 | -84.80 | -44.56 | 4.7124 | UO-01 reuse | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 2.50 (t2) | 367.7 | 108.05 | Weapon (690,724) | free_jaffa | no (south) |
| `fj.jweap.t2` | 63854 | 63832 | `UnionOmega_JaffaWeaponsT2` | 387.85 | -85.02 | -44.56 | 4.7124 | ring, 2.5 m from t1 | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 2.51 (t1) | 365.3 | 105.99 | Weapon (690,724) | free_jaffa | no (south) |
| `fj.jweap.t3` | 63855 | 63833 | `UnionOmega_JaffaWeaponsT3` | 384.85 | -85.02 | -45.56 | 4.7124 | ring, 3.0 m from t2 | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 3.00 (t2) | 362.1 | 102.96 | Weapon (690,724) | free_jaffa | no (south) |
| `fj.jweap.t4` | 63856 | 63834 | `UnionOmega_JaffaWeaponsT4` | 381.85 | -84.80 | -44.56 | 4.7124 | ring, 3.0 m from t3 | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 3.00 (t3) | 359.5 | 101.11 | Weapon (690,724) | free_jaffa | no (south) |
| `fj.jarmor.t1` | 63859 | 63835 | `UnionOmega_JaffaArmorT1` | 216.50 | -63.59 | -66.17 | 0.0000 | UO-01 reuse | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 3.00 (t2) | 195.5 | 38.77 | Armor (797,695) | free_jaffa | no (south) |
| `fj.jarmor.t2` | 63860 | 63836 | `UnionOmega_JaffaArmorT2` | 213.50 | -63.19 | -66.17 | 0.0000 | ring, 3.0 m from t1 | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 3.00 (t1) | 192.7 | 39.36 | Armor (797,695) | free_jaffa | no (south) |
| `fj.jarmor.t3` | 63861 | 63837 | `UnionOmega_JaffaArmorT3` | 210.50 | -63.63 | -66.17 | 0.0000 | ring, 3.0 m from t2 | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 3.00 (t2) | 189.9 | 40.16 | Armor (797,695) | free_jaffa | no (south) |
| `fj.jarmor.t4` | 63862 | 63838 | `UnionOmega_JaffaArmorT4` | 207.50 | -63.25 | -66.17 | 0.0000 | ring, 3.0 m from t3 | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 3.00 (t3) | 187.1 | 40.96 | Armor (797,695) | free_jaffa | no (south) |
| `fj.craft.biomedical` | 63968 | 63839 | `UnionOmega_JaffaCraftingBiomed` | 291.14 | -60.90 | -109.35 | 1.5708 | UO-01 reuse | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 3.00 (electronics) | 258.0 | 6.57 | none (group) | free_jaffa | n/a (no spot) |
| `fj.craft.electronics` | 63969 | 63840 | `UnionOmega_JaffaCraftingElec` | 294.14 | -61.23 | -109.35 | 1.5708 | grid, 3.0 m east of biomedical | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 3.00 (biomedical) | 261.0 | 3.97 | none (group) | free_jaffa | n/a (no spot) |
| `fj.craft.power` | 63970 | 63841 | `UnionOmega_JaffaCraftingPowerSys` | 297.14 | -61.17 | -110.85 | 1.5708 | grid, 3.35 m from electronics | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 3.35 (electronics) | 263.8 | 4.00 | none (group) | free_jaffa | n/a (no spot) |
| `fj.craft.materials` | 63971 | 63842 | `UnionOmega_JaffaCraftingMat` | 300.14 | -61.23 | -109.35 | 1.5708 | grid, 3.35 m from power | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 3.35 (power) | 267.0 | 3.84 | none (group) | free_jaffa | n/a (no spot) |
| `fj.trainer.jaffa` | 63808 | 63808 | `UnionOmega_JaffaTrainer` | 306.00 | -61.23 | -104.00 | 4.7124 | grid, plaza centre estimate | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 4.00 (consumable.1) | 273.4 | 5.39 | Jaffa trainer (819,664) | free_jaffa | partly (east yes, z off) |
| `fj.consumable.1` | 63871 | 63828 | `UnionOmega_Consumables_Jaffa1` | 306.00 | -61.23 | -100.00 | 3.1416 | ring, 4.0 m from trainer | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 4.00 (trainer) | 273.9 | 7.20 | Consumable (782,634) | free_jaffa | partly (north yes) |
| `fj.consumable.2` | 63872 | 63828 | `UnionOmega_Consumables_Jaffa2` | 310.00 | -61.23 | -104.00 | 4.7124 | ring, 4.0 m from trainer | exit 0 (h 0.00, dy 0.00) | exit 0 (h 0.00, dy 0.00) | 4.00 (trainer) | 277.4 | 9.36 | Consumable (882,670) | free_jaffa | partly (east yes) |

Distances in metres. Nearest NPC and nearest ambient are the closest world-18 row by x/z (ambient 63880-63897 are the nearest rows for every key). Heading rule: weapons and armor copy their UO-01 reuse heading; the four crafting vendors copy the biomedical heading (1.5708); the trainer faces the wing entrance and teleporter side (west, 4.7124); consumable.1 faces the trainer (3.1416) and consumable.2 faces the trainer (4.7124). No prop in the landmark table is a vendor counter, so these are walkway-facing choices; the W9 coder may change a heading without moving a point.

Spacing results: every key is at least 2.5 m from every seeded world-18 row and from every UO-01 point; every key is at least 3.0 m from every ambient spawn 63880-63897 (63898-63901 are world 80) (minimum 3.84 m, materials); the respawner 180 is at least 187 m away.

## SQL

### For UO-12

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63853, 390.35, -84.80, -44.56, 4.7124, 18, 63831, 'UnionOmega_JaffaWeaponsT1', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63854, 387.85, -85.02, -44.56, 4.7124, 18, 63832, 'UnionOmega_JaffaWeaponsT2', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63855, 384.85, -85.02, -45.56, 4.7124, 18, 63833, 'UnionOmega_JaffaWeaponsT3', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63856, 381.85, -84.80, -44.56, 4.7124, 18, 63834, 'UnionOmega_JaffaWeaponsT4', NULL, true);
```

### For UO-28

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63859, 216.50, -63.59, -66.17, 0.0000, 18, 63835, 'UnionOmega_JaffaArmorT1', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63860, 213.50, -63.19, -66.17, 0.0000, 18, 63836, 'UnionOmega_JaffaArmorT2', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63861, 210.50, -63.63, -66.17, 0.0000, 18, 63837, 'UnionOmega_JaffaArmorT3', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63862, 207.50, -63.25, -66.17, 0.0000, 18, 63838, 'UnionOmega_JaffaArmorT4', NULL, true);
```

### For UO-29

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63968, 291.14, -60.90, -109.35, 1.5708, 18, 63839, 'UnionOmega_JaffaCraftingBiomed', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63969, 294.14, -61.23, -109.35, 1.5708, 18, 63840, 'UnionOmega_JaffaCraftingElec', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63970, 297.14, -61.17, -110.85, 1.5708, 18, 63841, 'UnionOmega_JaffaCraftingPowerSys', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63971, 300.14, -61.23, -109.35, 1.5708, 18, 63842, 'UnionOmega_JaffaCraftingMat', NULL, true);
```

### For UO-30

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63871, 306.00, -61.23, -100.00, 3.1416, 18, 63828, 'UnionOmega_Consumables_Jaffa1', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63872, 310.00, -61.23, -104.00, 4.7124, 18, 63828, 'UnionOmega_Consumables_Jaffa2', NULL, true);
```

### For UO-31

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63808, 306.00, -61.23, -104.00, 4.7124, 18, 63808, 'UnionOmega_JaffaTrainer', NULL, true);
```

## Landmarks

Scanned the Omega_Site map in the cooked root (`SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC\Maps\Omega_Site`, 132 chunks): `extract_actors` on the map directory (13,091 actors, `--json`) and `archetype_census` (261 positioned meshes, 47 inside x 150..420, z -160..0; `CIMMERIA_PACKAGE_INDEX` = a copy of the UO-19 package index). Names are generic, so a landmark is named only where a census prop origin lies within 3 m of the placed point. The art is not scaled, so each art spot is matched by its placed point. The shipped Jaffa props sit north of the gate while the art puts the wing south (F34).

| Art spot | World landmark used | x, z | Source |
|---|---|---|---|
| Teleporter (717,661) | `ASG-Buildings:ASG-TeleportBase00`, exact | 191.96, -122.66 | census row `Omega_Site-fffe0001`; region 16 Right is (191.98, -122.78) |
| Weapon (690,724) | `JF-Buildings:JF-MilitaryTent00` (t1 1.43 m, t2 2.75 m) | 390.21, -45.98 | census row `Omega_Site-ffff0003` |
| Armor (797,695) | none named; nearest open walkway (216.50, -66.17) | 216.50, -66.17 | nearest prop `JF-Tent03`, 5.08 m |
| Consumable (782,634) | none named; nearest open walkway (306.00, -100.00) | 306.00, -100.00 | nearest prop `JF-MedBuilding01`, 23.04 m |
| Consumable (882,670) | none named; nearest open walkway (310.00, -104.00) | 310.00, -104.00 | nearest prop `JF-MedBuilding01`, 19.93 m |
| Jaffa trainer (819,664) | none named; nearest open walkway (306.00, -104.00) | 306.00, -104.00 | nearest prop `JF-MedBuilding01`, 19.06 m |
| Crafting group (biomedical, electronics, power, materials) | none named; nearest open walkway (291.14, -109.35) | 291.14, -109.35 | nearest prop `JF-MedBuilding01`, 18.58 m (biomedical); 13.80 m (power) |

The Weapon and Teleporter rows name a landmark within 3 m. No point was moved; the mesh still decides (see § Art conflicts).

## Rejected

- Crafting ring search at 1.5 m (session 1): no walkable point on the four axes. Accepted points come from a 0.5 m and 2 m grid at 2.5-3.5 m.
- Power at (294.14, -112.35) and (294.14, -106.35), and materials at (291.14, -106.35), (291.14, -112.35): `h` 1.92-1.98 or 0.31-0.35 (off the walkable cell).
- Power at (297.14, -109.35) and (294.64, -106.35): within 3.0 m of ambient 63892 (2.50 m and 2.63 m).
- Trainer candidates on the ring centre estimated from the art: the 2 m grid at x 276-312, z -128..-92 shows component 122 (not 60) at z -118..-128, and out-of-tolerance `dy` (about -5.7 m) at z -98..-128 for x 276-290. The trainer is placed at (306, -104) on component 60.

## Art conflicts

The art has no scale, so offsets are read from the Teleporter (art (717,661) = region 16 Right (191.98, -122.78)). Art up = +z, art right = +x.

- Weapons: the art puts the weapon vendor south-west of the teleporter, at (690,724). The mesh puts the weapon tiers 198 m east and 78 m north of the teleporter (z -44 to -85). Disagreement on north-south and east-west. Recorded, not moved (the mesh decides).
- Armor: the art puts the armor vendor south-east of the teleporter, at (797,695). The mesh puts the armor tiers north-east (about 25 m east, 57 m north). Disagreement on north-south.
- Trainer and consumables: east of the teleporter in both the art and the mesh (about 114-119 m east). The mesh puts them 19-23 m north of the teleporter line; the art puts the trainer 3 px south. Partial disagreement on north-south.
- Crafting: no art spot (group placed on the mesh near UO-01 `svc.jaffa_crafting`).
- Art spots for the two General vendors stay unserved (D-UO28).

## nav_inspect summaries

- Anchors: `modi_seed78` exit 0 (h 0.00, dy -0.09); `pad14` exit 0 (h 0.00, dy -0.79).
- Verification runs (anchor first in each file, all 15 keys): exit 0 against both anchors; every key `h=0.00`, component 60, `dy` between -0.00 and +0.00 after snap.
- Probe files: `fj_verify_*`, `fj_verify2_*`, `fj_verify5_*` in `.claude\reports\union-omega\probes\` (not committed).

## Last check

`B:\targets\uo-18\debug\nav_inspect.exe data/spaces/omega_site.nav --probes ... --gaps` exit 0 for each verification set against `modi_seed78` and `pad14` (status=ok; the probes are reads only, no cargo run this session).
