# NA27 phase 1: collision-geometry occluder, format and measurement

Issue [#784](https://github.com/SandboxServers/Cimmeria/issues/784). Measured 2026-09-25 on the 23 playable client maps (not `Login_Map`).

**Outcome: no-go under the size budget.** The owner's rule was: if the biggest world fits in about 10 MB on disk and 50 MB of RAM, ship every world; if not, stop after phase 1 and commit nothing under `data/spaces/`. The biggest world is Agnos. At the chosen 0.5 m cell it needs 57.3 MB on disk and 288 MB of RAM. Even at 1.0 m it needs 22.4 MB and 98 MB. No `.occ` file is committed, and the server's line of sight is unchanged.

The format, the builder and the `occluder_extract` measurement tool are committed (`crates/occluder`, `crates/navmesh-extractor/src/occluder/`, `bin/occluder_extract`). A later decision can pick them up: a per-world budget, a streaming loader, or a trimmed format.

## The format

`cimmeria-occluder` stores a world's collision triangles as two layers, in BigWorld metres.

- **Geometry: a column grid.** StaticMesh and BSP triangles are rasterised into square XZ cells, 16 x 16 cells to a tile. Each cell holds a list of solid spans. A span is a Y range, quantised to 0.1 m and rounded outwards. It also carries the sub-cell rectangle (1/16 of the cell) that the solid part occupies. Overlapping spans merge only when the merged box adds under 2% of phantom volume. So a floor under the whole cell and a wall along one edge of it stay two spans, and the wall does not fill the cell.
- **Terrain: an exact heightfield.** It stores the 1 m lattice vertex heights at 1 cm, with per-patch hole and diagonal flags. A query tests the real terrain triangles. Terrain that is not on the lattice falls back to the geometry layer. On every map except Agnos (1,353 triangles) and Beta_Site_Evo_1 (180 triangles), no terrain fell back.
- **Coverage.** Only tiles within 2 m of a floor-like surface are kept. Floor-like means within 45 degrees of level, either winding. A query endpoint outside the coverage reads `OffGrid`, which maps to `LineOfSight::Unknown`.
- **File.** The file is a small header (magic `CMOC`, version, Y quantisation, source-triangle hash and a build label), then one zlib stream per layer. Spans are delta-coded as varints. `flate2` is already a workspace dependency, so no new crate is needed.

The segment test walks the cells the eye-to-eye segment crosses (Amanatides-Woo). It clips the segment to each span's rectangle and compares the exact Y. Two fuzz tests check that the grid never reports clear where an exact segment/triangle test reports blocked. One uses random boxes; the other uses thin sheets at random angles, at 0.25, 0.5 and 1.0 m cells. Making the rectangle non-conservative fails both.

## Accuracy

The sweep recreates NA16's method. Pairs are sampled on the navmesh, 4-30 m apart horizontally, with `|dy| <= 4`. A 1.5 m eye height is added at both ends. The truth is an exact segment test against the same raw triangles.

- **Castle_CellBlock:** 4,000 pairs, taken from the interior components only (every mesh component of at most 10,000 m², which drops the map-sized terrain sheet).
- **Castle:** 2,000 pairs, from components of at most 100,000 m².

Both use the meshes on `main`.

| map | cell | truly blocked | occluder false clear | occluder false block (share of truly clear) | wrong given `Blocked` | same, without rays grazing within 0.1 m | navmesh wrong given `Blocked` | navmesh false clear (share of truly blocked) |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| Castle_CellBlock | 0.25 m | 1,259 | 0 | 18 (0.66%) | 1.41% | 0.16% | 38.0% | 0.2% |
| Castle_CellBlock | 0.5 m | 1,259 | 0 | 34 (1.24%) | 2.63% | 0.79% | 38.0% | 0.2% |
| Castle_CellBlock | 1.0 m | 1,259 | 0 | 57 (2.08%) | 4.33% | 2.25% | 38.0% | 0.2% |
| Castle | 0.25 m | 378 | 0 | 16 (0.99%) | 4.06% | 1.05% | 49.1% | 23.3% |
| Castle | 0.5 m | 378 | 0 | 17 (1.05%) | 4.30% | 1.31% | 49.1% | 23.3% |
| Castle | 1.0 m | 378 | 0 | 19 (1.17%) | 4.79% | 1.56% | 49.1% | 23.3% |

- **Chosen cell: 0.5 m.** It is the smallest cell under 2% error in both directions, read as the share of truly clear pairs called blocked and of truly blocked pairs called clear. Castle_CellBlock fails that at 1.0 m (2.08%).
- **Grazing rays.** Under NA16's per-verdict reading (the share of `Blocked` answers that are wrong), Castle stays at about 4% at every cell size. Almost all of those false blocks are rays that pass within 0.1 m of a wall edge. With those excluded, every cell size is under 2%.
- **Castle's navmesh is worse than NA16 found for the Cellblock.** It is wrong on 49% of its `Blocked` answers, and it calls 23% of truly blocked pairs clear. That second figure means seeing through walls.
- **Endpoint clearance.** A 0.3 m endpoint clearance was tried and dropped. It removed no false blocks and added 3 false clears on Castle.

**Query cost:** 1.9 µs per segment (mean of 20,000-40,000 sweep segments, release build). **Load:** zlib decode takes 8 ms for Castle_CellBlock and 62 ms for Castle.

## Size, every map

The "grid" column is the geometry layer's dimensions in cells, over its bounding rectangle. Build time is for the 0.5 m grid, after the triangle walk. The walk itself takes 0.4 s for Castle_CellBlock and 10 s for Lucia.

| map | triangles | terrain patches | 0.25 m grid | 0.25 file MB | 0.25 RAM MB | 0.5 m grid | 0.5 file MB | 0.5 RAM MB | 0.5 build s | 1.0 file MB | 1.0 RAM MB |
|---|---:|---:|---|---:|---:|---|---:|---:|---:|---:|---:|
| Castle_CellBlock | 1.53 M | 0.61 M | 3248x3248 | 0.7 | 20.8 | 1632x1632 | 0.3 | 7.2 | 0.5 | 0.2 | 3.5 |
| Castle | 4.14 M | 1.43 M | 4896x4832 | 11.0 | 72.0 | 2464x2432 | 5.3 | 23.9 | 2.4 | 3.2 | 10.7 |
| Agnos | 22.27 M | 6.90 M | 12048x9248 | 159.3 | 1005.5 | 6032x4640 | 57.3 | 287.6 | 28.6 | 22.4 | 98.1 |
| Agnos_Library | 1.57 M | 0.35 M | 2448x2704 | 2.0 | 24.4 | 1232x1360 | 0.9 | 8.4 | 1.0 | 0.5 | 3.8 |
| Beta_Site_Evo_1 | 17.98 M | 6.13 M | 12048x8448 | 21.6 | 178.1 | 6032x4240 | 13.3 | 67.9 | 8.5 | 9.5 | 36.3 |
| Dakara_E1 | 13.83 M | 3.83 M | 8048x8048 | 20.8 | 137.1 | 4032x4032 | 11.4 | 51.9 | 8.4 | 7.2 | 26.5 |
| Dakara_E1_StoryRm | 0.05 M | 0.01 M | 448x448 | 0.1 | 0.6 | 240x240 | 0.0 | 0.2 | 0.0 | 0.0 | 0.1 |
| Harset | 3.37 M | 0.72 M | 3248x3648 | 7.8 | 66.3 | 1632x1824 | 3.7 | 21.6 | 2.7 | 1.9 | 8.8 |
| Harset_CmdCenter | 0.23 M | 0.00 M | 896x608 | 0.6 | 8.9 | 464x320 | 0.3 | 2.6 | 0.2 | 0.2 | 0.8 |
| Harset_Market | 0.33 M | 0.09 M | 1248x1248 | 0.5 | 5.2 | 624x624 | 0.2 | 1.8 | 0.2 | 0.1 | 0.8 |
| Harset_StorageRm | 0.28 M | 0.01 M | 448x512 | 0.6 | 5.3 | 240x256 | 0.3 | 1.7 | 0.2 | 0.2 | 0.7 |
| Ihpet_Crater_Dark | 5.20 M | 1.53 M | 4448x5648 | 6.0 | 79.2 | 2224x2832 | 3.5 | 26.5 | 2.4 | 2.3 | 11.9 |
| Ihpet_Crater_Light | 5.20 M | 1.53 M | 4448x5648 | 6.0 | 79.6 | 2224x2832 | 3.5 | 26.6 | 2.4 | 2.3 | 11.9 |
| Lucia | 29.63 M | 8.66 M | 13248x10848 | 39.2 | 226.9 | 6624x5440 | 22.9 | 92.6 | 12.2 | 15.9 | 52.1 |
| Menfa_Dark | 14.86 M | 2.70 M | 11456x6048 | 26.0 | 754.3 | 5744x3024 | 13.4 | 206.8 | 24.9 | 8.1 | 64.8 |
| Menfa_Light | 8.54 M | 2.70 M | 11456x6336 | 17.0 | 644.3 | 5744x3168 | 8.9 | 173.0 | 14.3 | 5.6 | 52.7 |
| Omega_Site | 2.93 M | 0.59 M | 4848x3136 | 6.9 | 44.1 | 2432x1568 | 3.4 | 14.5 | 1.8 | 1.9 | 6.2 |
| Omega_Site_CmdCenter | 0.48 M | 0.06 M | 18800x18816 | 134.2 | 2661.1 | 9408x9424 | 48.1 | 681.7 | 44.0 | 16.7 | 178.5 |
| SGC | 0.20 M | 0.00 M | 672x368 | 0.4 | 4.9 | 336x192 | 0.2 | 1.5 | 0.2 | 0.1 | 0.5 |
| SGC_W1 | 0.55 M | 0.00 M | 1792x1520 | 0.9 | 14.2 | 912x768 | 0.5 | 4.2 | 0.4 | 0.3 | 1.4 |
| Sewer_Falls | 2.76 M | 0.72 M | 3744x3248 | 5.2 | 54.2 | 1872x1632 | 2.4 | 17.2 | 1.6 | 1.1 | 7.1 |
| Tollana | 19.32 M | 4.85 M | 9648x11520 | 50.6 | 587.3 | 4832x5776 | 23.2 | 183.6 | 17.5 | 11.4 | 70.2 |
| Tollana_Curia | 0.02 M | 0.01 M | - | 0.0 | 0.0 | - | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |

At 0.5 m, 15 of the 23 worlds fit the budget. The eight that do not are Agnos, Beta_Site_Evo_1, Dakara_E1, Lucia, Menfa_Dark, Menfa_Light, Omega_Site_CmdCenter and Tollana. Every Castle world, every Harset world, and SGC, SGC_W1 and Sewer_Falls fit.

## What did not help, and what might

- **Trimming coverage to the navmesh footprint.** This was measured with NA26's rebuilt meshes (`--coverage-nav`). It cut Castle from 23.9 to 23.2 MB of RAM and Beta_Site_Evo_1 from 67.9 to 60.7 MB. The rebuilt outdoor meshes cover nearly the whole terrain, so the footprint is not much smaller than the map.
- **A finer Y quantum or less merging.** Y resolution does not drive the false blocks. At 0.05 m the Castle_CellBlock false blocks stayed at 43. Merging less took them from 43 to 34, which is the shipped setting.
- **Omega_Site_CmdCenter is a pathology, not a big map.** It has only 0.48 M triangles, but its grid is 4.7 km on a side and holds 108 M spans. A few enormous triangles fill every cell. A cap on triangle extent, or dropping triangles far from any walkable surface, would likely fix it. That was not tried.
- **Options for a later decision:**
  - Ship the 15 worlds that fit, with a per-world budget rather than the biggest-world rule.
  - Keep tiles compressed in RAM and decode them through an LRU cache. This fixes RAM, not disk.
  - Store the geometry layer only in a height band around the walkable surfaces.

## Reproduce

```bash
L=/c/Users/Steve/AppData/Local/Temp/cimmeria-castle
$L/lane.sh cargo build -p cimmeria-navmesh-extractor --release --bin occluder_extract
target/release/occluder_extract measure \
  --cooked-root "<CookedPC>" --map Castle_CellBlock --index <package_index.bin> \
  --nav data/spaces/castle_cellblock.nav --max-component-area 10000 \
  --cells 0.25,0.5,1.0 --report phase1.tsv --pairs-out pairs
```

`--pairs-out` writes every pair with the three verdicts and, for a disagreement, how close the clear segment passes to geometry (`near`, in metres).
