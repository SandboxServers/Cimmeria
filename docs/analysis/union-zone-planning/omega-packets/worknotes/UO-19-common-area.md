# UO-19: Common Area vendor placements (worknote, part 3)

> **Type:** Reference (worknote, handoff) · **Packet:** [UO-19](../UO-19.md) · **Status:** all 14 points sealed; SQL and sections written

## Done

- Builds from earlier sessions: `extract_actors`, `archetype_census`, `build-package-index`, `nav_inspect` (built exe `B:\targets\uo-19\debug\nav_inspect.exe`).
- Grid probe (2 m, x -110..170, z -230..70, probe y -40): `probes\grid_out.txt`, parsed in PowerShell. Floor y = -40 - dy. Walkable = component 60 and h ≤ 0.5.
- Picks by greedy search in spec order on comp-60 cells with 2 m clearance (the 4 orthogonal cells walkable), Euclidean spacing ≥ 2.5 m from every chosen key, from both fixed beams (34.575, 55.698) and (34.41, -75.46), and from the Science Center guide spots.
- NW pod keys moved (see § Rejected): no comp-60 walkway within 19 m of the overview pod; the nearest comp-60 walkways (-98,-60) and (-100,-60) are nearest-anchor `sgc`, so the keys moved to the nearest `common_area` walkway toward the pod.
- Probes: `probes\ca_vendors_modi_first.txt` and `probes\ca_vendors_pad_first.txt` (anchors first, 14 points). Both exit 0; all 16 rows `component=60 h=0.00 ok`. Not committed.
- Science Center guide spots probed: `probes\ca_guides.txt`, exit 0, all four candidates `component=60 h=0.00 ok`.
- Seeded world-18 check: `git grep ", 18, [0-9]+, '"` over `db/resources/Worlds/Seed/`. The row `(3, 22, 4, 20)` in `stargates.sql` is a stargate address, not a spawn, and is excluded from the spawn checks.

## § Points

Key, Spawn, Template, Tag, x, y, z, Heading, Candidate, Exit (Modi), Exit (pad 14), Nearest NPC, Nearest respawner (180), Nearest ambient, Art spot, Art area, Art agrees. Heading = atan2(34.5 - x, 0 - z) (see § Landmarks). Exits: `nav_inspect` h 0.00, component 60, both anchors, exit 0.

| Key | Spawn | Template | Tag | x | y | z | Heading | Candidate | Exit (Modi) | Exit (pad 14) | Nearest NPC | Nearest respawner | Nearest ambient | Art spot | Art area | Art agrees |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `ca.hweap.t1` | 63845 | 63812 | `UnionOmega_HumanWeaponsT1_Common` | 82.00 | -44.19 | -2.00 | 4.7545 | 82,-2 | ok | ok | 135 (74.7) | 144.0 | 136.0 (63896) | east pod Weapon (842,541) → (82.2, 1.8) | common_area | yes, 3.8 m |
| `ca.hweap.t4` | 63846 | 63815 | `UnionOmega_HumanWeaponsT4_Common` | -62.00 | -45.84 | -64.00 | 0.9852 | -62,-64 | ok | ok | 63820 (46.8) | 121.8 | 130.0 (63881) | NW pod Adv. Weapon, overview (-96, -41) | common_area | no, moved 41.0 m (F35) |
| `ca.harmor.t1` | 63851 | 63816 | `UnionOmega_HumanArmorT1_Common` | 78.00 | -44.12 | 8.00 | 4.5305 | 78,8 | ok | ok | 135 (64.5) | 152.3 | 142.0 (63896) | east pod Armor (825,513) → (79.2, 6.6) | common_area | yes, 1.8 m |
| `ca.harmor.t4` | 63852 | 63819 | `UnionOmega_HumanArmorT4_Common` | -64.00 | -45.75 | -68.00 | 0.9666 | -64,-68 | ok | ok | 63820 (42.4) | 121.0 | 125.6 (63881) | NW pod Adv. Armor, overview (-100, -36) | common_area | no, moved 48.2 m (F35) |
| `ca.jweap.t1` | 63857 | 63831 | `UnionOmega_JaffaWeaponsT1_Common` | 30.00 | -45.02 | 0.00 | 1.5708 | 30,0 | ok | ok | 135 (55.9) | 138.1 | 187.7 (63896) | hub Weapon (510,531) → (25.8, 3.5) | common_area | yes, 5.0 m |
| `ca.jarmor.t1` | 63863 | 63835 | `UnionOmega_JaffaArmorT1_Common` | 48.00 | -44.46 | -2.00 | 4.8595 | 48,-2 | ok | ok | 135 (59.2) | 136.6 | 169.6 (63896) | hub Armor (646,532) → (48.9, 3.4) | common_area | yes, 5.5 m |
| `ca.consumable.1` | 63865 | 63828 | `UnionOmega_Consumables_Common1` | 56.00 | -39.68 | 20.00 | 3.9631 | 56,20 | ok | ok | 135 (41.6) | 159.4 | 166.4 (63896) | Consumable (672,429) → (53.3, 20.9) | common_area | yes, 2.8 m |
| `ca.consumable.2` | 63866 | 63828 | `UnionOmega_Consumables_Common2` | 24.00 | -44.57 | -2.00 | 1.3826 | 24,-2 | ok | ok | 135 (58.7) | 136.4 | 193.4 (63896) | Consumable (497,555) → (23.6, -0.6) | common_area | yes, 1.5 m |
| `ca.consumable.3` | 63867 | 63828 | `UnionOmega_Consumables_Common3` | 52.00 | -44.17 | -2.00 | 4.8262 | 52,-2 | ok | ok | 135 (60.3) | 137.1 | 165.6 (63896) | Consumable (668,557) → (52.7, -0.9) | common_area | yes, 1.3 m |
| `ca.craft.biomedical` | 63964 | 63824 | `UnionOmega_CommonCraftingBiomed` | 24.00 | -37.17 | -28.00 | 0.3588 | 24,-28 | ok | ok | 132 (48.6) | 110.5 | 191.9 (63896) | none (group W of Science Center pod (582,717) → (38, -28)) | common_area | n/a |
| `ca.craft.electronics` | 63965 | 63825 | `UnionOmega_CommonCraftingElec` | 26.00 | -37.72 | -30.00 | 0.2761 | 26,-30 | ok | ok | 132 (46.2) | 108.3 | 189.9 (63896) | none (group) | common_area | n/a |
| `ca.craft.power` | 63966 | 63826 | `UnionOmega_CommonCraftingPowerSys` | 28.00 | -37.12 | -28.00 | 0.2281 | 28,-28 | ok | ok | 132 (47.9) | 110.2 | 187.9 (63896) | none (group) | common_area | n/a |
| `ca.craft.materials` | 63967 | 63827 | `UnionOmega_CommonCraftingMat` | 26.00 | -36.52 | -26.00 | 0.3160 | 26,-26 | ok | ok | 132 (50.2) | 112.3 | 189.9 (63896) | none (group) | common_area | n/a |
| `ca.auctioneer` | 63819 | 305 | `UnionOmega_Auctioneer` | 30.00 | -44.91 | -8.00 | 0.5124 | 30,-8 | ok | ok | 135 (63.9) | 130.1 | 186.7 (63896) | Auction House (540,594) → (30.9, -7.2) | common_area | yes, 1.2 m |

Spacing: smallest key-to-key distance 2.83 m (craft biomed/electronics, consecutive diagonal step). Fixed beams 135 and 132, respawner 180, arrival (34.70, -142.00), Modi 78, DHD 41 and the seeded NPCs are all ≥ 40 m away. Ambients 63880-63901 are ≥ 125 m away. Heading of the two NW keys and the other twelve is from the probe-verified positions.

## § SQL

One block per consumer, rows in spawn order.

### For UO-11

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63845, 82.00, -44.19, -2.00, 4.7545, 18, 63812, 'UnionOmega_HumanWeaponsT1_Common', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63846, -62.00, -45.84, -64.00, 0.9852, 18, 63815, 'UnionOmega_HumanWeaponsT4_Common', NULL, true);
```

### For UO-12

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63857, 30.00, -45.02, 0.00, 1.5708, 18, 63831, 'UnionOmega_JaffaWeaponsT1_Common', NULL, true);
```

### For UO-27

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63851, 78.00, -44.12, 8.00, 4.5305, 18, 63816, 'UnionOmega_HumanArmorT1_Common', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63852, -64.00, -45.75, -68.00, 0.9666, 18, 63819, 'UnionOmega_HumanArmorT4_Common', NULL, true);
```

### For UO-28

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63863, 48.00, -44.46, -2.00, 4.8595, 18, 63835, 'UnionOmega_JaffaArmorT1_Common', NULL, true);
```

### For UO-29

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63964, 24.00, -37.17, -28.00, 0.3588, 18, 63824, 'UnionOmega_CommonCraftingBiomed', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63965, 26.00, -37.72, -30.00, 0.2761, 18, 63825, 'UnionOmega_CommonCraftingElec', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63966, 28.00, -37.12, -28.00, 0.2281, 18, 63826, 'UnionOmega_CommonCraftingPowerSys', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63967, 26.00, -36.52, -26.00, 0.3160, 18, 63827, 'UnionOmega_CommonCraftingMat', NULL, true);
```

### For UO-30

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63865, 56.00, -39.68, 20.00, 3.9631, 18, 63828, 'UnionOmega_Consumables_Common1', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63866, 24.00, -44.57, -2.00, 1.3826, 18, 63828, 'UnionOmega_Consumables_Common2', NULL, true);
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63867, 52.00, -44.17, -2.00, 4.8262, 18, 63828, 'UnionOmega_Consumables_Common3', NULL, true);
```

### For UO-32

```sql
INSERT INTO spawnlist (spawn_id, x, y, z, heading, world_id, template_id, tag, set_name, is_stationary) VALUES (63819, 30.00, -44.91, -8.00, 0.5124, 18, 305, 'UnionOmega_Auctioneer', NULL, true);
```

## § Science Center

- The four crafting vendors take the **west** side of the Science Center pod (38, -28): biomedical (24, -28), electronics (26, -30), power (28, -28), materials (26, -26). Consecutive steps are 2.83 m (diagonal), all pairs ≥ 2.83 m. Nearest group member to the pod centre is 10 m.
- Free guide spots for UO-20 (probed, exit 0, component 60, h 0.00, both anchors): **Biology (32, -22)**, y -35.97, and **Physics (44, -22)**, y -36.06. Both are ≥ 7.2 m from the nearest crafting vendor (28, -28) and 1 m south of the art spots (32,-21) and (44,-21), which fall between grid rows -22 and -20 (both rows walkable: (32,-20) y -35.89, (44,-20) y -36.02).

## § Landmarks

The census and the actor list name no Auction House, no Weapon or Armor counter, no Consumable prop and no pod within the hub box, so no landmark matches these art spots. The mapping used is the approximate linear map from the Common Area enlargement (F34: the art does not affine-map, so it is only a search start):

`x = 36 + (px - 570) * 0.17`, `z = 70 - (py - 140) * 0.17`

Landmark anchors used for orientation:

| Art spot | World landmark used | x, z | Source |
|---|---|---|---|
| East pod Weapon / Armor | comp-60 walkway east of the hub centre | 82.2, 1.8 / 79.2, 6.6 | art map; mesh picks 3.8 m / 1.8 m away |
| NW pod Adv. Weapon / Adv. Armor | nearest `common_area` comp-60 walkway toward the pod | -62, -64 / -64, -68 | overview pod (-96, -41) and (-100, -36), F35 |
| Hub Weapon / Armor | hub walkway west / east of the N-S axis | 25.8, 3.5 / 48.9, 3.4 | art map |
| Consumables 1/2/3 | art map | 53.3, 20.9 / 23.6, -0.6 / 52.7, -0.9 | art map |
| Auction House | art map | 30.9, -7.2 | art map |
| Science Center pod | art map | 38, -28 | art map |
| Hub axis / pad 14 | pad 14 prop (34.56, -36.48, 55.68), beam 135 (34.575, 55.698) | 34.5, 0 (facing point) | census, seed |

Heading facing point: (34.5, 0), the hub centre. No counter prop is named for the Weapon or Armor counters, so each vendor faces the hub centre (a deviation from "counter or prop": none is named).

## § Rejected

- **NW pod, original picks (-98,-62) and (-102,-62)** (and (-98,-60), (-100,-60), the nearest comp-60 cells, 19-24 m from the pod): these are comp 60 and pass the anchors, but their nearest-anchor area is `sgc` (Ring region 12 Left (-210.15, -92.33) at about 116 m; Ring region 10 Center at about 133 m). Rejected by the art-area rule.
- **NW pod cells on comp 133** (for example (-106, -74), 34.5 m from the pod): fail the Modi anchor (component 133).
- **Pod cells within 15 m of the overview pod**: none exist on comp 60 (grid).
- **Non-60 teal component** in the hub middle (about x 18..54, z 0..40): fails the anchors.
- **pick.txt grid picks** (80,-8), (72,8), (32,-2), (52,-8), (58,18), (32,-6), (50,-10), (32,-10): replaced by spacing-aware picks (82,-2), (78,8), (30,0), (48,-2), (56,20), (24,-2), (52,-2), (30,-8) after the 2.5 m rule.
- **Craft cells east of the Science Center pod**: they would sit between the pod and the guides; the chosen group is west only.
- **stargate address row (3, 22, 4, 20)**: not a spawn; excluded from the spacing set.

## § Art conflicts

- NW pods: the overview spots (-96, -41) and (-100, -36) have no comp-60 walkway within 19 m, and the nearest comp-60 walkways are `sgc` by the nearest-anchor rule. The NW keys sit 41.0 m and 48.2 m from the overview spots, on the nearest `common_area` walkway. The art's pod position is therefore not a sealed point; the mesh decides (rule 8).
- Hub Weapon, Armor, Consumables and Auction House spots: mesh picks are 1.2-5.5 m from the mapped art spot. The art is denser than the mesh allows.
- Biology and Physics art spots (32, -21) and (44, -21) fall between grid rows; the guide spots are (32, -22) and (44, -22), probed exit 0.

## § nav_inspect summaries

- `ca_vendors_modi_first.txt` (`modi_seed78`, then `pad14`, then 14 points): exit 0. All 16 rows `component=60 h=0.00 m ok`.
- `ca_vendors_pad_first.txt` (`pad14`, then `modi_seed78`, then 14 points): exit 0. All 16 rows `component=60 h=0.00 m ok`.
- `ca_guides.txt` (both anchors, then four Science Center guide candidates): exit 0, all `component=60 h=0.00 m ok`.
- Probe files, stdout and stderr: `.claude\reports\union-omega\probes\`, not committed.
- Lane status: no cargo run; `nav_inspect` run directly from `B:\targets\uo-19\debug\` (built earlier through `lane.ps1`). No `status=` line is produced for these runs.

## Next step

None for UO-19. The coder reports and commits this worknote. A reviewer may check the § Points rows against `probes\ca_run_*.txt`.