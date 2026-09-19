# NavBuilder: rebuilding it, and Recast's index limits

> **Last updated**: 2026-09-19
> **Status**: Verified against `bin64/NavBuilder.exe` (rebuilt from
> `deprecated/cpp/src/nav_builder/`) on the 144-chunk Castle extraction.

Split out of [navmesh-build-pipeline.md](navmesh-build-pipeline.md), which
remains the entry point for the pipeline as a whole. This page is the
reference for the builder binary itself: how to rebuild it, the four
fixed-width index spaces inside Recast that silently cap what a single
`rcPolyMesh` can hold, and the measured Castle (World 8) parameter set.

## Building `NavBuilder.exe`

```powershell
tools\build-navbuilder.ps1                      # -> bin64\NavBuilder.exe
tools\build-navbuilder.ps1 -Out C:\tmp\NavBuilder.exe -RecastRoot <recast checkout>
```

`deprecated/cpp-build/projects/NavBuilder.vcxproj` cannot be built any more:
its precompiled header (`deprecated/cpp/src/stdafx.hpp`) pulls in Boost
(python, asio, thread), SOCI and TinyXML, and it links `unified_kernel.lib`
— none of which `setup.ps1` provisions since the Rust rewrite. NavBuilder
itself needs only a logger and three Boost.uBLAS names, so the script
compiles the five `nav_builder/*.cpp` files plus `Recast/Source/*.cpp`
straight into one exe with `cl` (located through `vswhere`), defining
`NAVBUILDER_STANDALONE` and putting
`deprecated/cpp/src/nav_builder/standalone/` first on the include path. That
directory holds a `stdafx.hpp` shim (std headers + a synchronous logger with
the original line format) and `ublas_min.hpp`. The legacy vcxproj build is
untouched by either.

The script refuses to write to a file named `NavBuilder_d.exe`. The wrapper
scripts prefer `bin64/NavBuilder.exe` when it exists.

## Parity with the reference binary

Same input (`castle_int`, 17 interior tiles, 1,177,804 triangles), no
trailing arguments:

| Build | Recast source | Result |
|---|---|---|
| `bin64/NavBuilder_d.exe` (Debug, 2026-03-02) | `recastnavigation-main` snapshot of 2026-02-27 | 15,463 verts / 7,746 polys, 37 s |
| rebuilt, Release | same snapshot (`external/_downloads/recastnavigation-main.zip`) | **byte-identical** (SHA-256 `37C188E8…11EDF5`), 3.2 s |
| rebuilt, Release or Debug | `external/recast` = v1.6.0, what `setup.ps1` provisions today | 15,463 verts / 7,752 polys; same five interior probes in one component |

The reference binary predates the v1.6.0 pin: the bootstrap downloaded
`recastnavigation-main` until 2026-03-24 (commit `a7b7d66e9` switched to the
1.6.0 release for the Detour FFI). Debug and Release builds of the new source
are byte-identical with each other, so the 6-polygon difference is the Recast
version, not floating-point behaviour. Either Recast is fine for production;
use the snapshot only when you need to diff against the reference binary.

`tests/navbuilder_axis_roundtrip.rs` passes 3/3 against the rebuilt binary
(`CIMMERIA_NAVBUILDER=<path>`).

## Recast has four fixed-width index limits, and only one checks itself

A single `rcPolyMesh` — which is all the XRC `.nav` format can hold — is
bounded by three separate `unsigned short` index spaces, and the compact
heightfield it is built from is bounded by a fourth, 24 bits wide. NavBuilder
now checks all four; Recast itself only checks the first and the third.

**1. Contour vertices < 65,534 — checked.** `rcBuildPolyMesh` sums the
vertex counts of every contour *before* de-duplication and fails with
`rcBuildPolyMesh: Too many vertices N` (`RecastMesh.cpp:1014-1016` in v1.6.0).
The final `nverts` is 10–20 % lower than `N`; the rebuilt NavBuilder prints
`N` on its `Contours:` line. Whole-map Castle at the
default parameters: `N = 118,250`.

**2. Adjacency edges ≤ 65,535 — NOT checked.** `buildMeshAdjacency` stores
edge indices as `unsigned short` (`RecastMesh.cpp:75`,
`firstEdge[v0] = (unsigned short)edgeCount`) with no overflow test. One edge
is recorded per polygon side with `v0 < v1`, which comes to about
`0.9 × (nverts + npolys)`. Past the cap the mesh still builds, saves and
loads, but polygon neighbour links are garbage. Measured:

| Build | nverts | npolys | edges | components | interior probes |
|---|---|---|---|---|---|
| 62 chunks, StaticMesh + terrain, defaults | 55,704 | 26,825 | **75,103** | 5,551 | — |
| 144 chunks, `maxSimplificationError=2.0` `minRegionSize=24` | 54,413 | 26,688 | **74,942** | 4,320 | split; Zuritska's cell is a 23 m² island |
| 144 chunks, `maxSimplificationError=2.5` `minRegionSize=24` | 45,115 | 21,799 | 60,850 | 987 | one component |

The first row is the build that was believed to have "squeaked under" the
vertex cap. It did — and was silently corrupt. With `minRegionSize=24` no
component can be smaller than ~50 m², yet the second row has 3,055 of them:
those are fragments of real regions whose links were truncated. The rebuilt
NavBuilder counts edges exactly as Recast does and exits 3 above the cap.
**In practice the binding limit is `nverts + npolys ≲ 72,000`, not
`nverts < 65,534`.**

**3. Region ids are 15-bit — checked only for watershed.** `RC_BORDER_REG`
takes the top bit. `rcBuildRegions` tests for overflow
(`RecastRegion.cpp:1621` in v1.6.0); `rcBuildRegionsMonotone` increments an
`unsigned short id` with no test (`:1361`, `:1469`). Whole-map Castle with
`agentClimb=0.5` and the default monotone partitioning dies with an access
violation (`0xC0000005`) in the partitioning step under the 2026-02 snapshot,
and under v1.6.0 runs to completion with implausible output (10,450 contours
/ 72,567 contour vertices, against 5,592 / 104,022 for watershed on the same
input). **Use `partition=watershed` for anything map-sized.** It costs
nothing measurable here (15 s either way).

**4. Compact-heightfield spans ≤ 16,777,215 — NOT checked, and it is the
limit that bites when you refine `cs`.** `rcCompactCell` packs the index of a
column's first span into a 24-bit bitfield (`Recast.h:336`,
`unsigned int index : 24`) and `rcBuildCompactHeightfield` assigns
`cell.index = currentCellIndex` from a plain `int` with no test. Past
16,777,215 spans every column after the wrap points at the wrong run of
spans; `rcBuildRegions` then finds nothing walkable and the build produces an
**empty mesh with no Recast diagnostic at all** — the log reads
`Regions: 1`, `Contours: 0`, `nverts=0 npolys=0`, and the old binary exited 0
having written a 60-byte `.nav` that loads.

The count is roughly one span per walkable surface per column, so it grows as
`1/cs²`. Measured, cropping the Castle interior with
`bounds=150,500,700,1150`:

| `cs` | grid | spans | result |
|---|---|---|---|
| 0.3 | 1,833 × 2,167 | — | 38,123 / 18,122 |
| 0.25 | 2,200 × 2,600 | — | 45,113 / 21,792 |
| 0.2 | 2,750 × 3,250 | — | exit 3, adjacency-edge cap |
| 0.15 | 3,667 × 4,333 | **18,716,138** | exit 3, span cap (was: silent empty mesh) |

Whole-map Castle at `cs = 0.3` is 4,458 × 4,000 columns and **13,936,045
spans — 83 % of the cap**, so the whole map has no headroom for a finer `cs`
at all. `Castle_CellBlock` with the default ±400 chunk-padded bounds at
`cs = 0.15` is 5,333 × 5,333 and 30,594,915 spans; cropping to the real
geometry with `bounds=-360,-250,110,110` brings it to 3,133 × 2,400 and
9,524,285 spans, which builds (5,008 / 2,440 / 6,691 at
`minRegionSize=8 maxSimplificationError=1.3 ch=0.1`). **A finer Cellblock
build is feasible; it just has to be cropped.**

NavBuilder logs the count on every build:

```text
[11:01:49 INFO    ] Heightfield: 13936045 spans over 4458 x 4000 columns (cap 16777215; rcCompactCell::index is 24-bit)
```

### An empty poly mesh is now a failure

`builder.cpp` used to warn about `npolys == 0` *after* writing the file and
still exit 0. It now exits 3 and writes nothing, so "the command succeeded"
means a mesh exists. That closes the last path by which a build could hand
the server a structurally valid navmesh with no polygons in it — which loads
fine and makes every NPC fall back to straight-line pathing.

## Castle (World 8): what each parameter does

Input: all 144 chunks (`StaticMesh` + `Terrain` + BSP), 4,179,133 triangles,
grid 4,458 × 4,000. "Interior-5" = `zuritska_cell`, `romney_corridor`,
`comms_room`, `nid_guard_116`, `armory` resolve (h-tol 2 m, v-tol 3 m) to one
common component. Build time is wall-clock for NavBuilder alone, Release.

| Parameters (others default) | contour verts | nverts / npolys / edges | components | Interior-5 | s |
|---|---|---|---|---|---|
| *(defaults, monotone)* | 118,250 | fails: vertex cap | — | — | 17 |
| `minRegionSize=16` / `24` / `32` / `50` | 107,499 / 100,216 / 93,475 / 82,556 | fails | — | — | 15 |
| `maxEdgeLen=0` | 115,211 | fails | — | — | 16 |
| `mergeRegionSize=40` | 111,827 | fails | — | — | 18 |
| `maxSimplificationError=2.0` | 84,268 | fails | — | — | 16 |
| `partition=watershed` | 116,814 | fails | — | — | 16 |
| `agentHeight=1.8 agentClimb=0.5` (monotone) | — | **crash**, limit 3 | — | — | 9 |
| W = `partition=watershed agentHeight=1.8 agentClimb=0.6` | 110,845 | fails | — | — | 16 |
| W + `minRegionSize=24 maxSimplificationError=2.0` | 63,655 | 54,413 / 26,688 / 74,942 — **corrupt**, limit 2 | 4,320 | **no** | 15 |
| **W + `minRegionSize=24 maxSimplificationError=2.5`** | 54,355 | **45,115 / 21,799 / 60,850** | **987** | **yes** (17,006 m²) | **15** |
| W + `minRegionSize=32 maxSimplificationError=2.2` | — | 46,379 / 22,931 / 63,816 | 682 | yes | 14 |
| W + `minRegionSize=32 maxSimplificationError=2.5` | — | 41,739 / 20,460 / 56,766 | 682 | yes | 16 |
| W + `minRegionSize=16 maxSimplificationError=2.8` | — | 45,343 / 21,430 / 59,662 | 1,532 | yes | 17 |
| W + `maxSimplificationError=3.0` | — | 48,124 / 21,952 / 60,626 | 2,718 | yes | 18 |
| default agent + `partition=watershed minRegionSize=24 maxSimplificationError=2.5` | — | 48,290 / 23,365 / 65,239 | 1,073 | yes | 15 |

Reading it:

- **`maxSimplificationError` is the only strong lever** (−29 % contour
  vertices from 1.3 → 2.0). `minRegionSize` needs to reach 50 (a 225 m²
  threshold) for the same effect, `maxEdgeLen` and `mergeRegionSize` are
  worth 3–5 %, and monotone-vs-watershed is a wash. The vertices are in the
  *boundaries* of large regions — 1.08 km² of bumpy terrain with 45° cut-outs
  — not in the thousands of small islands.
- **Cost of `maxSimplificationError=2.5`** (0.75 m of contour deviation, of
  which the 0.6 m erosion margin absorbs most): rasterising the interior
  component over `x[200,520] z[800,1100] y[40,80]` against an otherwise
  identical 1.3 build gives 16,474 m² baseline, 152 m² lost (0.9 %), 321 m²
  gained (1.9 %). At 3.0 it is 211 / 467 m².
- **Cost of `minRegionSize=24`**: in the interior crop it removes 433 islands
  totalling 7,900 m² (2 % of the area), every one under 52 m². Most are prop
  tops. Any *sealed* room smaller than 52 m² goes with them; none of the
  current probes is in one.
- **Component histogram**, whole map, before → after the recommended set
  (before = the corrupt 2.0 build, the only whole-map mesh with small
  regions that exists): `<10 m²` 2,161 → 0; `10–50 m²` 894 → 1;
  `50–500 m²` 1,069 → 800; `>500 m²` 196 → 186.

Agent values:

- `agentHeight=1.8` removes crawl-spaces and under-furniture floor: interior
  tiles go 577 → 474 components and 329,798 → 312,909 m², all five interior
  probes unaffected. With the default `agentClimb=0.9 > agentHeight=0.6`,
  watershed logs `rcBuildRegions: 2 overlapping regions` and
  `rcBuildContours: Multiple outlines for region N` on the whole map — the
  storey-welding the mismatch was suspected of. Both errors disappear at
  1.8 / 0.6.
- `agentClimb=0.5` is **too low at `ch = 0.2`**: it floors to 2 voxels
  (0.4 m). With all 144 chunks loaded, `comms_room` / `nid_guard_116` split
  from `zuritska_cell` / `romney_corridor` (4,372 m² vs 12,330 m²
  components) and total walkable area drops 10 %. `0.6` (3 voxels) restores
  the single 16,8xx m² component; `0.7` is identical to `0.6`.
- `agentRadius` stays `0.6`. `crates/entity/src/navigation/mod.rs` passes
  height and climb straight to `dtCreateNavMeshData` and nothing else reads
  them, but `is_point_valid` gates on `agent_radius * 2.0` horizontally and
  `agent_radius * BELOW_SURFACE_TOLERANCE_FACTOR` below the surface
  (`mod.rs:456-461`). Halving the radius would halve the player
  movement-validation tolerance as a side effect.

**Recommended Castle set** (`--preset castle` / `-Preset castle`):

```text
partition=watershed agentHeight=1.8 agentClimb=0.6 minRegionSize=24 maxSimplificationError=2.5
```

7 % headroom under the edge cap. Against v1.6.0 Recast the same set gives
45,124 / 21,803 / 60,862 and the same probe result. The 62 geometry-bearing
chunks alone come to 25,278 / 11,889 / 33,319.

What the recommended mesh does **not** do: connect all eleven probes. The
gate room, stargate, `bunker_muelbach` and `checkpoint_bravo` share one
exterior component (23,186 m²); the interior five plus `opcore` share
another (17,022 m²); `throne_room` is in a third (37,214 m²). That is
geometry, not tuning — see
[navmesh-build-pipeline.md §7](navmesh-build-pipeline.md#7-castle-world-8-why-the-probes-sit-in-three-components)
for the measurements that rule tuning out.

The whole-map sweep that confirms no parameter set does better (all six
builds put the eleven probes in **three or more** groups):

| Parameters (plus `partition=watershed agentHeight=1.8 agentClimb=0.6`) | nverts / npolys / edges | components | probe groups |
|---|---|---|---|
| **`minRegionSize=24 maxSimplificationError=2.5`** (recommended) | **45,209 / 21,805 / 60,926** | **997** | **3** |
| `minRegionSize=32 maxSimplificationError=2.5` | 41,808 / 20,458 / 56,815 | 689 | 3 |
| `minRegionSize=16 maxSimplificationError=2.8` | 45,433 / 21,434 / 59,728 | 1,544 | 3 |
| `minRegionSize=8 maxSimplificationError=3.0` | 48,223 / 21,961 / 60,702 | 2,732 | 4 |
| `minRegionSize=8 maxSimplificationError=3.5` | 44,123 / 20,064 / 54,781 | 2,723 | 4 |
| `minRegionSize=4 maxSimplificationError=3.5` | 47,282 / 21,058 / 56,999 | 3,691 | 4 |

The rows that go to **four** groups are not worse meshes — they are the ones
that keep the 11 m² `Castle_ArmoryRingDropZone` pad, which `minRegionSize=24`
deletes. Under the recommended set the `armory` probe then resolves onto the
main interior floor 1.49 m away and reads `ok`; that is the right runtime
answer (a player ringing in steps off the pad onto the floor) but it is a
tolerance result, not a hit. Do not read `armory ... h=1.49 m ok` as "the
pad is in the mesh".

## When it stops fitting

More geometry (or a finer `cs`) will push the edge count back over. In order
of cost:

1. `minRegionSize=32` — 56,766 edges, 13 % headroom, same probe result.
2. `bounds=minX,minZ,maxX,maxZ` — crop to the region NPCs can reach.
   `bounds=150,550,650,1150` (the interior complex) is 20,743 / 10,152 /
   28,490 at **un-degraded** `maxSimplificationError=1.3 minRegionSize=8`.
   The extractor need not change; Recast clips to the box.
3. One `.nav` per region of interest, or a tiled Detour mesh — both need
   server-side loader work and are out of scope here.

Decimating terrain in the extractor does **not** help with the caps: they
count output contour vertices, which depend on the shape of the walkable
boundary at `cs`, not on input tessellation. It would only trim the 2–3 s of
rasterisation out of a 15 s build.

## Cross-references

- [navmesh-build-pipeline.md](navmesh-build-pipeline.md) — the pipeline, the
  axis mapping, `nav_inspect` and the gap finder
- `deprecated/cpp/src/nav_builder/` — NavBuilder source
- [tools/build-navbuilder.ps1](../../tools/build-navbuilder.ps1) — the build script
- `crates/entity/src/navigation/` — runtime loader (Detour FFI)
