# UO-37: Asgard area and Council cast placements (worknote)

> **Type:** Reference (worknote) · **Packet:** [UO-37](../UO-37.md) · **Status:** Sealed points, every point exits 0 against its anchors. No seed or code change.
> **Index:** [omega-work-packets.md](../../omega-work-packets.md) · **Seal:** [UO-16-asgard-seal.md](UO-16-asgard-seal.md) (ruling [UO-16-ruling.md](UO-16-ruling.md))

Labels: `RESTORED_SOURCE` (shipped data), `CIMMERIA_RECONSTRUCTION` (new binding of shipped data), `CIMMERIA_AUTHORING` (new). Coordinates are navmesh probes (`nav_inspect`, 2026-10-19). The art is layout intent only (D-UO15); it never moved a coordinate.

## § Landmarks

| Art spot | World landmark | x / z | Source | Label |
|---|---|---|---|---|
| West small pod, overview (338, 520) px ([omega-concept-art.md](../../omega-concept-art.md) line 113, table line 186) | Art map (approximate, search start only): `x = 36 + (px - 570) * 0.17`, `z = 70 - (py - 140) * 0.17` (UO-19-common-area.md § Landmarks). Art point (338, 520) gives (-3.44, 5.40), world 18. It is a reference point only; no spawn moves to it (art never moves a coordinate). | -3.44, 5.40 | art map | CIMMERIA_RECONSTRUCTION |
| Asgard area (trainer, 12 vendors, hub Thor) | Flat floor at y -45.02 on component 60, west of the Common Area guides and crafting rows (nearest seeded NPC 48 m away). Chosen by the mesh: a 3 m grid over x -51..-27, z 6..33 at y -45 (`probes\grid2.txt`, output `probes\grid2.out.txt`). Floors at y -45.02 hold `h` 0.00 across x -42..-30, z 18..30. | x -42..-30, z 18..30 | navmesh | CIMMERIA_RECONSTRUCTION |
| Council Chamber, overview (580, 138) px ([omega-concept-art.md](../../omega-concept-art.md) § Council Chamber) | None on world 18: UO-16 Form 1 (no reachable chamber; D-UO20). | n/a | UO-16 § Venue | RESTORED_SOURCE (art only) |
| Command Center venue (world 80) | Open floor on component 11 south of the Command Center beam, anchor `pad17_cmd` 6.16 34.52 10.08 (`probes\cmd1.txt`). The chosen floor y 34.66-34.69 lies at x 0..3, z -6. | x 0..3, z -6 | navmesh | CIMMERIA_AUTHORING (staging) |

Anchors: world 18 `modi_seed78` 39.54 -51.91 -129.92 (component 60, floor y -51.82) and `pad14` 34.56 -36.48 55.68 (dy -0.79, floor -35.69). World 80 `pad17_cmd` 6.16 34.52 10.08 (component 11, dy -0.62, floor 35.14).

## § Points

Method: a 3 m lattice (not rings). The floor is flat over the service area (`h` 0.00 at every lattice point used), so a lattice gives exact spacing and no landmark coordinate exists to ring around. Each chosen point is one lattice cell; every pair of chosen points is 3.00 m or more apart. Heading = `atan2(dx, dz)` from the NPC to the walkway (world 18: (-18, 21), east of the row, the Common Area side) or to the hearing spot (world 80: the player arrival, pad17_cmd x 6.16, z 10.08). Heading normalised to [0, 2π), 4 decimals. Nearest columns are 2D distances in x/z from the spawn table (seeds and the UO-17..UO-20 rows): the nearest NPC, the nearest respawner and the nearest ambient in the same world.

| Key | Spawn | World | Template | Tag | x | y | z | Heading | Candidate | Exit per anchor | Nearest NPC (m) | Nearest respawner (m) | Nearest ambient (m) | Art agrees |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| asg.trainer | 63809 | 18 | 63809 | UnionOmega_AsgardTrainer | -39.00 | -45.02 | 21.00 | 1.5708 | lattice 3 m | 0 (modi, pad14) | 56.35 (63812) | 80.05 (180) | 186.64 | no (38.83 m, mesh-valid at both) |
| asg.programs.t1 | 63824 | 18 | 63843 | UnionOmega_AsgardProgramsT1 | -42.00 | -45.02 | 24.00 | 1.6952 | lattice 3 m | 0 (modi, pad14) | 57.91 (63812) | 83.68 (180) | 184.81 | no (42.81 m, mesh-valid at both) |
| asg.programs.t2 | 63825 | 18 | 63844 | UnionOmega_AsgardProgramsT2 | -42.00 | -45.02 | 27.00 | 1.8158 | lattice 3 m | 0 (modi, pad14) | 56.88 (63812) | 84.48 (180) | 185.86 | no (44.20 m, mesh-valid at both) |
| asg.programs.t3 | 63826 | 18 | 63845 | UnionOmega_AsgardProgramsT3 | -42.00 | -45.02 | 30.00 | 1.9296 | lattice 3 m | 0 (modi, pad14) | 55.99 (63812) | 85.38 (180) | 186.95 | no (45.74 m, mesh-valid at both) |
| asg.programs.t4 | 63827 | 18 | 63846 | UnionOmega_AsgardProgramsT4 | -39.00 | -45.02 | 30.00 | 1.9757 | lattice 3 m | 0 (modi, pad14) | 53.11 (63812) | 82.54 (180) | 189.74 | no (43.24 m, mesh-valid at both) |
| asg.armor.t1 | 63828 | 18 | 63847 | UnionOmega_AsgardArmorT1 | -39.00 | -45.02 | 27.00 | 1.8491 | lattice 3 m | 0 (modi, pad14) | 54.05 (63812) | 81.61 (180) | 188.66 | no (41.61 m, mesh-valid at both) |
| asg.armor.t2 | 63829 | 18 | 63848 | UnionOmega_AsgardArmorT2 | -36.00 | -45.02 | 27.00 | 1.8925 | lattice 3 m | 0 (modi, pad14) | 51.23 (63812) | 78.75 (180) | 191.47 | no (39.07 m, mesh-valid at both) |
| asg.armor.t3 | 63830 | 18 | 63849 | UnionOmega_AsgardArmorT3 | -36.00 | -45.02 | 24.00 | 1.7359 | lattice 3 m | 0 (modi, pad14) | 52.37 (63812) | 77.88 (180) | 190.46 | no (37.50 m, mesh-valid at both) |
| asg.armor.t4 | 63831 | 18 | 63850 | UnionOmega_AsgardArmorT4 | -39.00 | -45.02 | 24.00 | 1.7127 | lattice 3 m | 0 (modi, pad14) | 55.13 (63812) | 80.78 (180) | 187.63 | no (40.13 m, mesh-valid at both) |
| asg.craft.biomedical | 63832 | 18 | 63851 | UnionOmega_AsgardCraftingBiomed | -36.00 | -45.02 | 21.00 | 1.5708 | lattice 3 m | 0 (modi, pad14) | 53.66 (63812) | 77.13 (180) | 189.48 | no (36.10 m, mesh-valid at both) |
| asg.craft.electronics | 63833 | 18 | 63852 | UnionOmega_AsgardCraftingElec | -33.00 | -45.02 | 21.00 | 1.5708 | lattice 3 m | 0 (modi, pad14) | 51.00 (63812) | 74.22 (180) | 192.33 | no (33.42 m, mesh-valid at both) |
| asg.craft.power | 63834 | 18 | 63853 | UnionOmega_AsgardCraftingPowerSys | -30.00 | -45.02 | 21.00 | 1.5708 | lattice 3 m | 0 (modi, pad14) | 48.38 (63812) | 71.31 (180) | 195.18 | no (30.80 m, mesh-valid at both) |
| asg.craft.materials | 63835 | 18 | 63854 | UnionOmega_AsgardCraftingMat | -33.00 | -45.02 | 24.00 | 1.7682 | lattice 3 m | 0 (modi, pad14) | 49.64 (63812) | 75.00 (180) | 193.29 | no (34.92 m, mesh-valid at both) |
| asg.thor (hub, UO-16 ruling R1) | 63873 | 18 | 63862 | UnionOmega_AsgardThor | -36.00 | -45.02 | 18.00 | 1.4056 | lattice 3 m | 0 (modi, pad14) | 55.08 (63812) | 76.49 (180) | 188.55 | no (34.91 m, mesh-valid at both) |
| council.thor | 63836 | 80 | 63858 | UnionOmega_Council_Thor | 0.00 | 34.66 | -6.00 | 0.3658 | grid 3 m, floor snapped | 0 (pad17_cmd) | 16.13 (63800) | 18.86 (26) | 78.52 | n/a |
| council.otin | 63837 | 80 | 63859 | UnionOmega_Council_Otin | 3.00 | 34.69 | -6.00 | 0.1940 | grid 3 m, floor snapped | 0 (pad17_cmd) | 16.01 (63800) | 17.45 (26) | 81.50 | n/a |

Not used: `council.extra.1` (63838) and `council.extra.2` (63839). UO-16 § Hearing names only Thor and Otin, so the packet's rule writes "not used" here. (UO-16 § Venue calls the cast "four spawns"; that count is noted under § Art conflicts.)

Spacing: every pair of the 14 world-18 points is at least 3.00 m apart. The nearest seeded world-18 NPC is 48.38 m away, the nearest respawner (180 or 181) 71.31 m away, and the nearest ambient (63880-63901) 184.81 m away. The two world-80 points are 3.00 m apart, 16.01 m or more from the nearest seeded NPC (Jackson, 63800), and 78 m or more from the nearest ambient. Respawner 26 is 17.45 m from the nearest council row.

Ids and tags: spawn ids 63809, 63824-63837 and 63873 and the sixteen tags are not used by any seed or worknote row (checked against `spawnlist.sql`, `spawnlist_union_omega.sql` and the UO-17..UO-20 worknote SQL). Spawn 63873 is inside the spare band 63873-63879 (ruling R1).

## § SQL

Rows are in spawn order, one block per consumer. Each block is a sealed copy: UO-39, UO-40 and UO-42 paste these rows and choose nothing.

### For UO-39

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63809, -39.00, -45.02, 21.00, 1.5708, 18, 63809, 'UnionOmega_AsgardTrainer', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63824, -42.00, -45.02, 24.00, 1.6952, 18, 63843, 'UnionOmega_AsgardProgramsT1', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63825, -42.00, -45.02, 27.00, 1.8158, 18, 63844, 'UnionOmega_AsgardProgramsT2', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63826, -42.00, -45.02, 30.00, 1.9296, 18, 63845, 'UnionOmega_AsgardProgramsT3', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63827, -39.00, -45.02, 30.00, 1.9757, 18, 63846, 'UnionOmega_AsgardProgramsT4', NULL, true);
```

### For UO-40

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63828, -39.00, -45.02, 27.00, 1.8491, 18, 63847, 'UnionOmega_AsgardArmorT1', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63829, -36.00, -45.02, 27.00, 1.8925, 18, 63848, 'UnionOmega_AsgardArmorT2', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63830, -36.00, -45.02, 24.00, 1.7359, 18, 63849, 'UnionOmega_AsgardArmorT3', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63831, -39.00, -45.02, 24.00, 1.7127, 18, 63850, 'UnionOmega_AsgardArmorT4', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63832, -36.00, -45.02, 21.00, 1.5708, 18, 63851, 'UnionOmega_AsgardCraftingBiomed', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63833, -33.00, -45.02, 21.00, 1.5708, 18, 63852, 'UnionOmega_AsgardCraftingElec', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63834, -30.00, -45.02, 21.00, 1.5708, 18, 63853, 'UnionOmega_AsgardCraftingPowerSys', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63835, -33.00, -45.02, 24.00, 1.7682, 18, 63854, 'UnionOmega_AsgardCraftingMat', NULL, true);
```

### For UO-42

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63836, 0.00, 34.66, -6.00, 0.3658, 80, 63858, 'UnionOmega_Council_Thor', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63837, 3.00, 34.69, -6.00, 0.1940, 80, 63859, 'UnionOmega_Council_Otin', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63873, -36.00, -45.02, 18.00, 1.4056, 18, 63862, 'UnionOmega_AsgardThor', NULL, true);
```

The hearing Thor (63836) and Otin (63837, only under D-UO34) are world 80. The hub Thor (63873) is world 18 and offers 481 (ruling R1). Template 63862 belongs to UO-42.

## § Rejected

- Ring candidates (1.5, 1.75 and 2.0 m at 12 bearings) were not used: no landmark has a world coordinate (§ Landmarks), so there was nothing to ring around. The lattice is the candidate method for this packet.
- Lattice cells that failed: (-42, 18) `h` 1.81; (-45, 24) `h` 0.45; (-36, 30) `h` 1.13; (-33, 27) `h` 0.62; (-30, 24) `h` 0.10. Each has a non-zero `h` (the horizontal distance to the polygon), so it is not inside a polygon and is not used.
- Floors other than -45.02 on the same x/z were rejected: component 60 at dy +3.42 and -8.98 (other levels), components 182, 189, 193 and 149 (other floors or walkable islands). The points were never snapped onto those floors.
- World 80: the floors at dy -0.14 to -0.96 (y 34.5 snapped) and the cells beside the Command Center beam (6.12, 9.99) were not used.

## § Art conflicts

Quoted in full from the probe run:

1. **Venue, world.** The art draws the Council Chamber on world 18 at the top centre of the overview (580, 138). The seal puts the cast on world 80 (Command Center set, D-UO20, UO-16 § Venue Form 1). The mesh has no reachable chamber on world 18 (Pass A and Pass B of UO-16 ruling R3). The mesh decides; the conflict stays open for the owner.
2. **Venue, count.** UO-16 § Venue says "the Council cast is four spawns (spawnlist 63836-63839)". UO-16 § Hearing names two speakers (Thor and Otin). This packet sealed the two named points and wrote "not used" for `council.extra.*`. Owner call.
3. **Trainer.** The art puts one Asgard trainer in the west small pod (338, 520) px. The UO-19 art map gives (-3.44, 5.40) (§ Landmarks). The chosen trainer point (-39, 21) is 38.83 m from it, and the 14 world-18 rows lie 30.80 to 45.74 m from it. The coordinates are kept; the disagreement is recorded here.
4. **Vendors.** The art draws no Asgard vendor (UO-16 § Asgard area). The twelve vendor points are placed by the mesh alone, from the shipped monikers.

## § nav_inspect summaries

Every set was run through the lane (`pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-navmesh-extractor --bin nav_inspect -- ...`) and every point alone against both anchors with the built exe.

- `set18` (`data/spaces/omega_site.nav`, anchors modi_seed78 and pad14, 14 points, h-tol 2.0, v-tol 40): exit 0, 16 of 16 probes `ok` (lane `status=ok exit=0`).
- Each of the 14 world-18 points alone with both anchors: exit 0, `h=0.00`, `dy=+0.00`, component 60.
- `set80` (`data/spaces/omega_site_cmdcenter.nav`, anchor pad17_cmd, two points): exit 0, 3 of 3 `ok` (lane `status=ok exit=0`).
- Each world-80 point alone with the anchor: exit 0, `h=0.00`, `dy=-0.00` or `+0.00`, component 11.

Probe files: `.claude\reports\union-omega\probes\` (not committed).

## § Deviations

- The packet's Contract lists 13 service rows and a cast of 2 to 4. The hub Thor (63873, template 63862, tag `UnionOmega_AsgardThor`) is added from UO-16 ruling R1, which the orchestrator asked for before dispatch. The packet's Acceptance line is therefore 14 service rows and 2 cast rows.
- Candidates are a 3 m lattice, not rings (§ Points).
