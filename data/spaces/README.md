# Navmeshes (`data/spaces/*.nav`)

One XRC `.nav` per world, loaded at space creation by
`crates/services/src/cell/space_manager/lifecycle.rs`. The file name is the
world name from `entities/spaces.xml`, lower-cased, with spaces turned into
underscores. It is **not** `resources.worlds.client_map`: `SandBox` (world 2)
plays on the `Harset_CmdCenter` client map but loads `sandbox.nav`, which is
a byte copy of `harset_cmdcenter.nav`. A world with no file has no navmesh:
NPCs path in straight lines and the position and line-of-sight checks fail
open.

How a mesh is built, and the Recast limits the parameters below are chosen
around: [docs/engine/navmesh-build-pipeline.md](../../docs/engine/navmesh-build-pipeline.md)
and [docs/engine/navbuilder-recast-limits.md](../../docs/engine/navbuilder-recast-limits.md).
Which worlds enforce containment, and why the rest are advisory:
[docs/architecture/navmesh-containment-modes.md](../../docs/architecture/navmesh-containment-modes.md).

## Provenance

Every mesh except the two Castle ones was built by NA26 on 2026-09-25 from the
cooked client maps: StaticMesh (with prefab archetype resolution and the
mirrored-instance winding fix), Terrain and BSP, with the buried BSP hull-cap
filter, through `extract_map` and the tree-built `NavBuilder.exe`
(Recast 1.6.0). Common to every row: `partition=watershed agentHeight=1.8
agentClimb=0.6 agentRadius=0.6 ch=0.2` (Tollana: `ch=0.3`, see below).
`minRegionSize` is scaled with `cs` so the smallest kept region stays about
52 m² (24 at `cs=0.3`, 16 at 0.45, 12 at 0.6). `bounds` is the crop in
BigWorld metres, `minX,minZ,maxX,maxZ`; where it equals the chunk grid plus
20 m it is only there to clip skybox and backdrop geometry.

The seven big exteriors (Agnos, Beta_Site_Evo_1, Dakara_E1, Lucia,
Menfa_Dark, Menfa_Light, Tollana) were rebuilt by NA28 the same day from the
same NA26 extraction as **tiled** meshes (`tile=128`, the `XRCT` layout; see
[navmesh-build-pipeline.md §10](../../docs/engine/navmesh-build-pipeline.md#10-tiled-builds-na28-2026-09-25)),
whole, none of them cropped. For those rows "verts / polys / edges" are
summed over the tiles and "spans" is the largest tile's, which is what the
24-bit cap applies to.

"Probes" is `NavMesh::is_point_valid` over the world's seeded `spawnlist`,
`respawners`, `ring_transport_regions`, `stargates` and `point_set_points`
rows, leaving out the `(0, 0, 0)` placeholder gates. Many of those rows are
area-set corners and prefab origins rather than places to stand, so read the
column as a comparison between meshes, not as a coverage figure.

| File | cs / mse / crop | verts / polys / edges | spans | comps | size | build | probes | mode |
|---|---|---|---|---|---|---|---|---|
| `agnos.nav` | 0.45 / 2.5 / chunk grid, 2,173 tiles | 187,678 / 78,567 / 226,311 | 74 K | 6,075 | 10.6 MB | 36 s | none seeded | advisory |
| `agnos_library.nav` | 0.3 / 1.3 / chunk grid | 4,283 / 2,228 / 6,074 | 4.5 M | 57 | 0.25 MB | 4 s | none seeded | advisory |
| `beta_site_evo_1.nav` | 0.3 / 2.5 / chunk grid, 4,353 tiles | 150,161 / 71,141 / 192,554 | 76 K | 1,119 | 13.4 MB | 32 s | 7/7 | advisory |
| `castle.nav` | 0.3 / 2.5 / chunk grid | 40,068 / 19,815 / 55,101 | — | 549 | 3.4 MB | — | 62/78 | advisory |
| `castle_cellblock.nav` | 0.3 / 1.3 / ±400 | 3,039 / 1,658 / — | — | 17 | 0.18 MB | — | 46/92 | **enforce** |
| `dakara_e1.nav` | 0.3 / 3.0 / chunk grid, 2,773 tiles | 79,838 / 36,238 / 98,047 | 47 K | 699 | 7.3 MB | 23 s | 3/3 | advisory |
| `dakara_e1_storyrm.nav` | 0.3 / 1.3 / chunk grid | 239 / 115 / 325 | 0.1 M | 6 | 15 KB | 0.1 s | none seeded | advisory |
| `harset.nav` | 0.3 / 1.3 / chunk grid | 29,768 / 15,287 / 42,379 | 8.6 M | 372 | 1.9 MB | 9 s | 51/67 | advisory |
| `harset_cmdcenter.nav`, `sandbox.nav` | 0.3 / 1.3 / chunk grid | 1,130 / 570 / 1,580 | 0.4 M | 14 | 64 KB | 0.6 s | 14/20 | advisory |
| `harset_market.nav` | 0.3 / 1.3 / chunk grid | 1,352 / 683 / 1,897 | 1.2 M | 29 | 76 KB | 0.8 s | 3/5 | advisory |
| `harset_storagerm.nav` | 0.3 / 1.3 / chunk grid | 1,195 / 560 / 1,618 | 0.2 M | 16 | 65 KB | 0.5 s | 5/5 | advisory |
| `ihpet_crater_dark.nav` | 0.3 / 2.5 / chunk grid | 29,770 / 14,838 / 40,970 | 16.7 M | 370 | 2.6 MB | 20 s | 3/3 | advisory |
| `ihpet_crater_light.nav` | 0.3 / 2.5 / `-300,-1300,800,40` | 30,059 / 15,126 / 41,563 | 16.1 M | 371 | 2.6 MB | 19 s | 3/3 | advisory |
| `lucia.nav` | 0.45 / 2.5 / chunk grid, 2,711 tiles | 142,195 / 67,365 / 184,582 | 54 K | 1,955 | 11.9 MB | 47 s | 25/26 | advisory |
| `menfa_dark.nav` | 0.3 / 2.5 / chunk grid, 1,901 tiles | 102,017 / 47,485 / 128,529 | 100 K | 864 | 7.0 MB | 26 s | 32/36 | advisory |
| `menfa_light.nav` | 0.3 / 2.5 / chunk grid, 1,906 tiles | 86,659 / 40,244 / 108,998 | 100 K | 785 | 6.2 MB | 15 s | 0/1 (gate prefab origin) | advisory |
| `omega_site.nav` | 0.3 / 1.3 / chunk grid | 24,621 / 12,892 / 35,542 | 3.8 M | 247 | 1.7 MB | 7 s | 15/15 | advisory |
| `omega_site_cmdcenter.nav` | 0.3 / 1.3 / chunk grid | 4,728 / 2,409 / 6,660 | 3.6 M | 45 | 0.27 MB | 2.5 s | 3/3 | advisory |
| `sgc.nav` | 0.3 / 1.3 / chunk grid | 1,064 / 506 / 1,412 | 0.2 M | 21 | 57 KB | 0.5 s | 2/2 | advisory |
| `sgc_w1.nav` | 0.3 / 1.3 / chunk grid | 3,091 / 1,508 / 4,195 | 0.5 M | 68 | 0.17 MB | 1.1 s | 22/28 | advisory |
| `sewer_falls.nav` | 0.3 / 1.3 / chunk grid | 13,317 / 6,831 / 18,727 | 9.7 M | 139 | 0.77 MB | 7.5 s | none seeded | advisory |
| `tollana.nav` | 0.45 / 2.5 / chunk grid, 1,554 tiles, `ch=0.3` | 96,197 / 45,497 / 124,613 | 115 K | 1,215 | 5.8 MB | 30 s | 5/5 | advisory |
| `tollana_curia.nav` | 0.3 / 1.3 / chunk grid | 64 / 33 / 95 | 0.1 M | 1 | 4 KB | 0.1 s | none seeded | advisory |

Recast's caps, for reading the table: spans ≤ 16,777,215, contour vertices
< 65,534, adjacency edges ≤ 65,535. Build time is NavBuilder alone; loading
the chunk OBJs adds 5-30 s on the big maps.

### Kept, not rebuilt

- `castle_cellblock.nav` and `castle.nav` were built from the client maps on
  2026-09-19 with this pipeline. NA26 rebuilt both and measured the result
  against them: Cellblock 3,041 verts / 1,661 polys, covering 1,657 of the 1,658
  old polygon centroids, and the same 64/64 accepted telemetry positions and 176/182 rejected ones;
  Castle 39,640 verts / 19,584 polys, probes 61/78 against 62/78. Neither is an
  improvement, so the committed files stay, and so do the dozens of tests
  pinned to them. Cellblock is the only meshed world on `enforce`: its mesh
  has been walked under containment since 2026-09-19.

### Replaced

- `harset.nav` (2012, 19,345 polys, 1,939 components). Against real
  positions from SigNoz (`movement.validation_reject`, September 2026,
  round-number teleport points removed): of the 9,344 distinct rejected
  client positions the old mesh accepts 497 and the new one 3,719; of the 43
  distinct `last_valid` positions the old mesh accepts all 43 and the new one
  37. The six it loses are raised platforms at about y -59 / -61
  (for example (9.5, -58.8, 59.4) and (34.6, -61.3, -70.7)) where
  `obj_slab` finds no source geometry at all, so an actor class the
  extractor does not decode is the likely cause. Probes go from 18/67 to
  51/67; all five ring pads and the Command Center door are now on-mesh.
- `harset_storagerm.nav` (2012, agent 0.6 / 0.9). Of 24 distinct accepted
  positions the new mesh keeps 16 against the old 23. Three of the seven it
  loses are near-origin login noise (|x|, |z| < 2); the other four sit 1-2 m
  above or below the new floor, where the old mesh had an under-floor layer
  and prop tops its 0.6 m agent could stand on. Rejected
  positions: 614 against 631 of 1,069. A regression by the letter of the
  check, which is one of the reasons world 70 is advisory.
- `sgc_w1.nav` (2012, agent 0.6 / 0.9). No navmesh rejects in the
  telemetry to compare. Probes 18/28 → 22/28; 69 % of the old polygons'
  centroids are on the new mesh, the rest are low-clearance and prop-top
  surfaces a 1.8 m agent cannot use.
- `agnos.nav` (2012). The old file covered 1,603 m² in one strip at
  z -1000..-800, outside NA26's crop. NA28's whole-map mesh covers it again:
  4,048 of the 2012 mesh's 4,061 polygon centroids (99.7 %) are valid on it.

### The big exteriors are tiled

A single `rcPolyMesh` cannot carry the big outdoor maps at `cs=0.3`, so NA26
shipped Dakara_E1 and both Menfa maps whole at `cs=0.6` and Agnos, Lucia,
Tollana and Beta_Site_Evo_1 cropped (`-800,-300,200,700`,
`-900,-400,700,900`, `-800,-600,300,400` and `-100,0,1200,1200`). NA28
replaced all seven with tiled meshes: the four cropped ones whole at NA26's
`cs`, the three coarse ones at `cs=0.3`, same `maxSimplificationError`,
`minRegionSize` and `ch` rules as NA26. Tollana still needs `ch=0.3`: one
prop at y -1728 puts the city more than 8,191 height cells (the 13-bit
`rcSpan` limit) above `bmin` at `ch=0.2`, and tiling does not change that,
since every tile keeps the map's Y range.

Measured against the NA26 mesh each replaced. "Coverage" is the share of
the old mesh's polygon centroids (dropped onto its surface, and valid on
it) that `is_point_valid` accepts on the new one. "Probe groups" is how many
components the in-tolerance probes land in under `nav_inspect`.

| Map | NA26 → NA28 | Coverage | Probes | Probe groups |
|---|---|---|---|---|
| Agnos | crop, 0.45 → whole, 0.45 | 99.31 % (2012 strip: 99.7 %) | none seeded | — |
| Beta_Site_Evo_1 | crop, 0.3 → whole, 0.3 | 99.43 % | 7/7 → 7/7 | 2 → 2 |
| Dakara_E1 | whole, 0.6 → whole, 0.3 | 99.13 % | 3/3 → 3/3 | 1 → 1 |
| Lucia | crop, 0.45 → whole, 0.45 | 99.22 % | 20/26 → 25/26 | 5 → 8 |
| Menfa_Dark | whole, 0.6 → whole, 0.3 | 99.86 % | 32/36 → 32/36 | 18 → 11 |
| Menfa_Light | whole, 0.6 → whole, 0.3 | 99.87 % | 0/1 → 0/1 | — |
| Tollana | crop, 0.45 → whole, 0.45 | 99.70 % | 5/5 → 5/5 | 1 → 1 |

No probe the old mesh accepted is rejected by the new one. Lucia's five old
probe groups are intact, with the same members; the three new ones hold the
six probes (three ring spawns and three point-set rows) that were out of tolerance on the cropped mesh and now have
a floor under them. On Menfa_Dark the finer `cs` cuts the probe groups from
18 to 11.

Where the rest of the coverage goes: the same crop of Beta_Site_Evo_1 built
tiled keeps 99.74 % of the single mesh's centroids (and 27,731 polygons
against 23,497: tile seams cost about 18 % more polygons and bytes). The
remainder on the whole-map build is voxel-grid phase: the heightfield now
starts at the chunk grid instead of the crop corner, so every cell boundary
moves, and a 116 m catwalk at x ≈ 279.5, z 340-466 that eroded to two cells
wide on the old grid erodes to one on the new, which puts it under
`minRegionSize`. Most of the other misses are `below_surface` on steep
terrain, where the two meshes' detail triangles disagree by more than the
1.2 m below-surface gate. Component counts grow with the area covered;
none of the seven has a component under 10 m² (NavBuilder's seam filter;
without it Agnos had 1,262).

## Worlds without a mesh

The server only creates spaces for the 24 worlds in `entities/spaces.xml`,
and all 24 now have a file. The other 67 `resources.worlds` rows (mission
test maps, `Tol-Alpha` / `Ca-Alpha` pockets, and worlds whose map is not in
the 2009 client such as Egypt, Pen-Lai, Pertho, SGC_W2 and Dakara_E2/E3)
have no client map directory under `CookedPC/Maps`, so there is nothing to
build from. `Login_Map` is skipped on purpose.

## Size

`data/spaces` went from 6.7 MB (6 files) to 36.5 MB (24 files) with NA26,
and to 76.9 MB with NA28's seven tiled meshes (62.2 MB of it). The largest
file is `beta_site_evo_1.nav` at 13.4 MB; it loads in about 0.1 s.

## Occluders (`*.occ`, NA27)

Each world in this directory also ships a `<world>.occ`, its
collision-geometry occluder for server-side line of sight
([#784](https://github.com/SandboxServers/Cimmeria/issues/784), decision
D-NA13). The file name follows the `.nav` rule: the world name lower-cased,
with spaces replaced by underscores. A world with no `.occ` keeps the
navmesh ray, so the file is optional.

### Build

```bash
python tools/occluder_entry_points.py > entry_points.tsv
target/release/occluder_extract build \
  --cooked-root "<CookedPC>" --map <ClientMap> --index <package_index.bin> \
  --nav data/spaces/<world>.nav --entry-points entry_points.tsv \
  --out data/spaces/<world>.occ
```

The inputs are:

- the client map's collision triangles: StaticMesh, BSP and terrain, the
  same set `extract_map` gives NavBuilder;
- this directory's `.nav` for the world;
- the entry points, which come from two places:
  - the seeds, via `tools/occluder_entry_points.py`: spawnlist rows,
    respawners, stargates and their arrival points, ring transport regions,
    and chain `cross_world_teleport` / `move_waypoint` targets;
  - the map itself: its `PlayerStart`, `SGWStargate` and `SGWTeleporter`
    actors.

Only the navmesh components that hold an entry point are kept. That set is
then grown to any component within 5 m horizontally and 3 m vertically (a
door or stair gap), and the coverage is those components plus a 15 m
margin. Geometry outside the coverage is not stored. Triangles that cross
its edge are clipped to it.

Five worlds have no entry point that lands on their mesh, so they keep
every component. Agnos_Library, Dakara_E1_StoryRm, Sewer_Falls and
Tollana_Curia have no seed rows and no PlayerStart. Menfa_Light's one
stargate row sits 190 m below its mesh. (The two `*` chain waypoints are
tried on every world and belong to the Castle maps.)

The files are 64 m pages, each compressed on its own. The server keeps them
packed and unpacks only the pages within 132 m of a player
(`space_manager::occlusion`). Two builds of the same map are byte-identical.

### Size

Built 2026-09-25 against the `.nav` files from NA26 (#794) and the seeds
after NA29 (#795), at 0.5 m cells. The untrimmed columns are the phase-1
build: the whole map, unpaged. "One player" is the resident RAM with one
player standing at the world's first entry point on the grid.

| World | Triangles | Trimmed | Entry points on the mesh | Components kept | Untrimmed file (MB) | Untrimmed RAM (MB) | Shipped file (MB) | Pages | RAM, all unpacked (MB) | RAM, one player (MB, pages) | Unpack mean / max (µs) | Query (µs) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Castle_CellBlock | 1.53 M | 1.13 M | 35/36 | 13/17 | 0.3 | 7.2 | 0.3 | 23 | 5.1 | 3.3 (15) | 255 / 726 | 1.70 |
| Castle | 4.14 M | 2.32 M | 50/51 | 94/549 | 5.3 | 23.9 | 3.2 | 98 | 16.4 | 5.6 (23) | 457 / 1384 | 1.61 |
| Agnos | 22.27 M | 18.33 M | 1/3 | 588/1538 | 57.3 | 287.6 | 13.0 | 226 | 58.6 | 5.9 (22) | 983 / 2183 | 3.52 |
| Agnos_Library | 1.57 M | 0.00 M | 0/2 (none; all kept) | 57/57 | 0.9 | 8.4 | 1.0 | 104 | 7.9 | 4.0 (22) | 604 / 3634 | 3.37 |
| Beta_Site_Evo_1 | 17.98 M | 13.45 M | 7/9 | 300/601 | 13.3 | 67.9 | 5.0 | 253 | 29.7 | 2.3 (22) | 352 / 787 | 2.09 |
| Dakara_E1 | 13.83 M | 2.23 M | 2/4 | 326/845 | 11.4 | 51.9 | 10.2 | 774 | 48.3 | 4.5 (23) | 1074 / 4990 | 4.08 |
| Dakara_E1_StoryRm | 0.05 M | 0.00 M | 0/2 (none; all kept) | 6/6 | 0.0 | 0.2 | 0.0 | 9 | 0.2 | - (no entry point) | - | - |
| Harset | 3.37 M | 0.00 M | 43/45 | 273/372 | 3.7 | 21.6 | 3.8 | 224 | 21.6 | 7.0 (24) | 735 / 2187 | 1.40 |
| Harset_CmdCenter | 0.23 M | 0.01 M | 13/15 | 7/14 | 0.3 | 2.6 | 0.3 | 12 | 2.5 | 2.5 (11) | 406 / 910 | 1.79 |
| Harset_Market | 0.33 M | 0.00 M | 1/3 | 15/29 | 0.2 | 1.8 | 0.2 | 36 | 1.8 | 1.7 (22) | 158 / 1175 | 1.99 |
| Harset_StorageRm | 0.28 M | 0.00 M | 1/3 | 10/16 | 0.3 | 1.7 | 0.3 | 8 | 1.7 | 1.7 (8) | 454 / 1171 | 3.12 |
| Ihpet_Crater_Dark | 5.20 M | 2.35 M | 2/4 | 155/370 | 3.5 | 26.5 | 2.7 | 104 | 21.0 | 2.2 (19) | 267 / 694 | 1.40 |
| Ihpet_Crater_Light | 5.20 M | 2.33 M | 2/4 | 159/371 | 3.5 | 26.6 | 2.7 | 105 | 21.1 | 2.2 (19) | 326 / 801 | 2.16 |
| Lucia | 29.63 M | 19.13 M | 18/26 | 269/568 | 22.9 | 92.6 | 9.8 | 548 | 43.4 | 5.5 (23) | 666 / 1901 | 2.31 |
| Menfa_Dark | 14.86 M | 0.07 M | 28/31 | 222/629 | 13.4 | 206.8 | 13.2 | 744 | 160.6 | 5.7 (22) | 592 / 1609 | 4.10 |
| Menfa_Light | 8.54 M | 0.03 M | 0/3 (none; all kept) | 526/526 | 8.9 | 173.0 | 8.7 | 749 | 126.5 | 5.0 (23) | 425 / 954 | 2.21 |
| Omega_Site | 2.93 M | 1.04 M | 13/15 | 83/247 | 3.4 | 14.5 | 2.9 | 81 | 13.1 | 5.9 (23) | 599 / 1260 | 2.95 |
| Omega_Site_CmdCenter | 0.48 M | 0.15 M | 2/4 | 5/45 | 48.1 | 681.7 | 0.7 | 20 | 6.4 | 6.2 (16) | 632 / 1383 | 2.76 |
| SGC | 0.20 M | 0.00 M | 1/3 | 9/21 | 0.2 | 1.5 | 0.2 | 9 | 1.5 | 1.5 (9) | 252 / 1069 | 1.08 |
| SGC_W1 | 0.55 M | 0.00 M | 23/29 | 30/68 | 0.5 | 4.2 | 0.5 | 37 | 4.2 | 1.4 (9) | 229 / 931 | 1.56 |
| Sewer_Falls | 2.76 M | 0.00 M | 0/2 (none; all kept) | 139/139 | 2.4 | 17.2 | 2.4 | 224 | 17.2 | 3.6 (22) | 308 / 1253 | 1.92 |
| Tollana | 19.32 M | 13.09 M | 5/7 | 189/417 | 23.2 | 183.6 | 9.5 | 306 | 74.8 | 2.8 (15) | 394 / 1042 | 2.17 |
| Tollana_Curia | 0.02 M | 0.00 M | 0/2 (none; all kept) | 1/1 | 0.0 | 0.0 | 0.0 | 9 | 0.0 | - (no entry point) | - | - |

The `.occ` files total 90.6 MB. Omega_Site_CmdCenter used to fill a
4.7 km grid from a few enormous triangles; clipping them to the explorable
area took it from 682 MB to 6.4 MB of RAM. Query cost is the mean of 2,000
random eye-height segments within 30 m of the player, with warm pages.
Measurements and the design are in
[the NA27 worknote](../../docs/analysis/npc-ai-restoration/worknotes/na27-occluder-phase1.md).
