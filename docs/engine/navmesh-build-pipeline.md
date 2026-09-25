# Navmesh Build Pipeline (UE3 → OBJ → NavBuilder → `.nav`)

> **Last updated**: 2026-09-25 (§10, tiled builds)
> **Status**: Verified end-to-end against the prebuilt `NavBuilder_d.exe` and the shipped 2013 `castle_cellblock.nav`. §2.5 and §6 (gap finding and classification) measured on the 144-chunk Castle extraction the same day; the builder reference moved to [navbuilder-recast-limits.md](navbuilder-recast-limits.md) and the Castle connectivity analysis to [castle-navmesh-connectivity.md](castle-navmesh-connectivity.md).

How a cooked UE3 map becomes a `data/spaces/<space>.nav` that
`crates/entity/src/navigation/` can load, and the exact conventions each
stage imposes. Every claim below was measured against
`bin64/NavBuilder_d.exe` on 2026-09-19; the C++ source is
`deprecated/cpp/src/nav_builder/`.

```text
CookedPC/Maps/<Map>/<Map>-XXXXXXXX.umap
        │  crates/navmesh-extractor  (StaticMesh → triangles → OBJ)
        ▼
<outdir>/chunks/<hex8>o.obj              CRLF, Y-up, centimetres
        │  NavBuilder chunked <chunks> <out.nav> nav [key=value ...]
        ▼
<out>.nav                                XRC poly mesh, BigWorld metres
        │  nav_inspect (connectivity + probe gate)
        ▼
data/spaces/<space>.nav
```

Use [tools/build-navmesh.sh](../../tools/build-navmesh.sh) or
[tools/build-navmesh.ps1](../../tools/build-navmesh.ps1) to drive all three
stages; they encode the guards described under
[Failure modes](#3-failure-modes-navbuilder-will-not-tell-you-about).

## 1. The coordinate mapping

**This is the one thing to get right.** A mesh built with the wrong column
order loads perfectly happily and paths NPCs into walls.

| Stage | Transform | Source |
|---|---|---|
| Extractor writes OBJ | `obj = (ue_x, ue_z, ue_y)` — swap UE3's Y and Z, keep centimetres | must change, see §1.2 |
| `Mesh::loadOBJ` reads OBJ | `bw = (obj_z / 100, obj_y / 100, obj_x / 100)` | `mesh.cpp:106-108` |
| **Net** | **`bw = (ue_y / 100, ue_z / 100, ue_x / 100)`** | measured |

In words: UE3 is Z-up centimetres, BigWorld is Y-up metres, and the two
horizontal axes are exchanged. UE3 `X` is the BigWorld *depth* axis `z`;
UE3 `Y` is the BigWorld *lateral* axis `x`.

The net map is the cyclic permutation `(x, y, z) → (y, z, x)`, which is a
**rotation, not a reflection** — handedness is preserved end to end, and no
winding compensation is needed (see §1.3).

### 1.1 Evidence

*Synthetic round-trip* — [`tests/navbuilder_axis_roundtrip.rs`](../../crates/navmesh-extractor/tests/navbuilder_axis_roundtrip.rs)
authors an L-shaped floor plus a ramp in chunk `0x000a0003` (grid X = 3,
Z = 10), deliberately asymmetric in all three axes, and runs the real
`NavBuilder_d.exe`:

| Authored (UE3 cm) | Predicted (BW) | Measured (BW) |
|---|---|---|
| `Y ∈ [30000, 34000]` | `x ∈ [300, 340]` | `x ∈ [300.90, 339.00]` |
| `Z ∈ [5000, 5300]` | `y ∈ [50, 53]` | `y ∈ [50.20, 53.00]` |
| `X ∈ [100000, 107500]` | `z ∈ [1000, 1075]` | `z ∈ [1000.90, 1074.10]` |

The 0.6–1.1 m inset is Recast eroding the walkable area by `agentRadius`
(0.6 m → 2 cells at `cs = 0.3`) and then simplifying the contour.

*Real data* — the library extraction of `Castle_CellBlock` (1699
StaticMeshActors, 391,075 triangles) spans UE3 `X ∈ [-24032, -3046]`,
`Y ∈ [-35188, -3593]`, `Z ∈ [2210, 13556]` cm, predicting BW
`x ∈ [-351.9, -35.9]`, `z ∈ [-240.3, -30.5]`. The shipped 2013
`castle_cellblock.nav`'s non-ground components occupy BW `x ∈ [-384.4, -36.1]`
and `z ∈ [-239.8, -22.3]`. The two horizontal spans are 316 m / 210 m
respectively; swapping them would compare 210 against 348 and 316 against
217, which does not fit. The mapping is confirmed on real geometry.

*Chunk-grid corroboration* — `Castle_CellBlock` has chunk ids with both
halves spanning `-4 ..= 3`, and the shipped `.nav` has
`bmin = (-400, −500, −400)`, `bmax = (400, 500.6, 400)`: exactly
`index * 100` and `(index + 1) * 100` on both horizontal axes. NavBuilder
names the low `u16` `positionX_` and the high `u16` `positionZ_`
(`chunk.cpp:27-28`), and those names are correct **BigWorld** names:

- low `u16` → UE3 `Y` → **BW `x`**
- high `u16` → UE3 `X` → **BW `z`**

> The module doc in `crates/navmesh-extractor/src/chunk_id.rs` currently
> states the opposite (low → BW Z, high → BW X). The UE3 labels there are
> right; the BW labels are transposed.

### 1.2 What the extractor emits

`crates/navmesh-extractor/src/obj.rs::write_obj_into` emits

```text
v <ue.x> <ue.z> <ue.y>\r\n
```

— UE3's up-axis moved off column 1, and CRLF throughout. Both are load
bearing and both are pinned by tests:

- **Column order.** Writing raw UE3 columns puts UE3's up-axis on BW `x`, so
  every floor rasterises as a vertical wall and NavBuilder produces a
  structurally valid but completely empty `.nav` (measured: `npolys = 0`).
- **CRLF.** See §1.4 — with bare LF, every `f` line whose third index is a
  single digit loses that index and the face is dropped.

The face winding, the `o` group lines and the 1-based index numbering are
emitted verbatim from the UE3 data; none of them needs compensating (§1.3).

Anything else that writes chunk OBJs — or reads them back, as
`crates/navmesh-extractor/src/obj_slab.rs` does (§6.2) — has to use the same
convention.

### 1.3 Winding

`Mesh::loadOBJ` reverses every triangle on read (`mesh.cpp:124-129`,
`(faces[i], faces[i-1], faces[0])`). Combined with the rotation of §1, the
Recast normal of a UE3 triangle `(a, b, c)` is `N = −P(n)` where `n` is the
right-hand-rule normal in UE3 space and `P` is the axis permutation, so

```text
N.y = −n.z          →   Recast walkability requires  n_ue3.z < 0
```

UE3's raw StaticMesh index order satisfies that for floor *tops* — SGW's
cooked meshes are wound for a left-handed front face, so the right-hand-rule
normal of an up-facing surface points down. Measured over the whole
`Castle_CellBlock` extraction, bucketing near-horizontal triangle area by
height:

| normal | dominant heights (BW y) | interpretation |
|---|---|---|
| `n.z < 0` | 65, 54, 100, 48, 55, 75, 66, 32, 25, 24 | matches the shipped mesh's walkable storeys (24.8–45.6, 53.4, 65.6–74.2, 94.6) |
| `n.z > 0` | 131–135 (17.5 k m² of it), then 70, 47 | the roof cap |

Building with the winding reversed confirms it: the mesh grows from 3,935
to 18,685 m² of "walkable" area, but coverage of the shipped mesh *drops*
from 7.6 % to 2.8 % and mean vertical error rises from 0.25 m to 0.84 m —
the extra area is the roof and the undersides of slabs.

**Emit UE3's index order verbatim. Do not reverse it.**

### 1.4 Line endings — CRLF is mandatory

`Mesh::loadOBJ`'s face parser loops `while (pos < line.length() - 1)`
(`mesh.cpp:115`), so the last token on an `f` line needs one trailing
character to be consumed. The original C++ exporter wrote CRLF
(`mesh_exporter.cpp:56`, `"f %d %d %d\r\n"`) and the `\r` supplied it.

With bare LF, any `f` line whose **third index is a single digit** loses
that index; the face then has only two vertices and is dropped silently. In
practice that is the first three faces of every file (`f 1 2 3`, `f 4 5 6`,
`f 7 8 9`). Measured on the six-triangle fixture: CRLF → 14 polys, LF → 6
polys, same geometry.

`writeln!` emits `\n` on Windows too, so this does not come for free.

## 2. NavBuilder input layout

```text
NavBuilder <chunked|whole> <input> <output> <nav|obj> [key=value ...]
```

Two binaries exist. `bin64/NavBuilder_d.exe` is the 2026-03 reference build:
exactly four arguments, hard-coded Recast parameters, exit code 0 on every
failure. The rebuilt `NavBuilder.exe` (§6) takes the trailing parameters
documented in [§2.5](#25-recast-parameters-and-exit-codes), forwards Recast's
own log, and exits non-zero when it fails. With no trailing arguments the two
produce byte-identical output (§6.2).

### 2.1 Filenames

`MapChunk::load` (`chunk.cpp:19-30`) is handed the path **without** the
`.obj` extension and requires:

- the last character of the stem to be `'o'`, and
- the eight characters before it to be hex digits.

It then appends `".obj"` itself. So the on-disk name is
`<8 hex digits>o.obj` — for example `fffefffdo.obj`. This is BigWorld's
outdoor-chunk naming (`o` = *outside chunk*), which is why the convention
predates the UE3 data. `ChunkId::obj_filename()` already produces it.

`chunked` mode additionally filters on `fname.length() > 12`
(`builder.cpp:262`); a 13-character `<hex8>o.obj` passes with nothing to
spare. `whole` mode takes the stem directly and applies the same `'o'`
test, so a combined map-level OBJ must *also* be named `<hex8>o.obj` to be
usable in `whole` mode.

The two modes are otherwise identical: `whole` loads one file, `chunked`
globs `*.obj` in a directory and loads each. Both treat the geometry the
same way, so the choice is only about how the extractor emitted its output.
Prefer `chunked` — the per-chunk files are what the extractor writes, and
`whole` gives the chunk-bounds union no chance to be meaningful.

### 2.2 The chunk id does *not* move geometry

`MapChunk::exportVertices` passes `offsetX = offsetZ = 0.0f` and an identity
matrix (`chunk.cpp:45-57`, consumed at `mesh.cpp:167`). The decoded
`(positionX_, positionZ_)` is used **only** to widen the Recast bounding box
(`builder.cpp:96-110`).

Confirmed empirically: the fixture in chunk `0x000a0003` authored at BW
`x ≈ 300` came back at BW `x ≈ 300`, not `600`.

**OBJ vertices must therefore be world-absolute UE3 centimetres.** They
already are — SGW cooked actors carry world-space `Location`.

### 2.3 Bounds padding, and a source/binary discrepancy

The committed `chunk.cpp:27-29` sets `positionX_ = (int16)chunkId` and
`sizeX_ = sizeZ_ = 100`, i.e. it treats the chunk *index* as if it were
already in BigWorld units. The prebuilt `NavBuilder_d.exe` matches the
committed source: the chunk-`0x000a0003` fixture produced
`bmin = (3, 50, 10)` / `bmax = (340, 53.6, 1075)` — the `3` and `10` are the
raw indices, and `bmax[0] = 340` only because the geometry exceeded
`3 + 100`.

The 2013 binary did not do this. `castle_cellblock.nav` has
`bmin/bmax = ±400` on both horizontal axes over a chunk index range of
`-4 ..= 3`, which is `index * 100`. The missing `* 100` is a regression in
the committed source relative to whatever produced the shipped meshes.

Consequences are cosmetic-but-annoying rather than corrupting: the bounds
are unioned with the real vertex bounds, so geometry is never clipped. What
you lose is (a) bit-comparability of `bmin`/`bmax` with the shipped files,
and (b) a little wasted heightfield. The rebuilt NavBuilder (§6) deliberately
leaves `chunk.cpp` alone so that its default output stays byte-identical to
the reference binary; use `bounds=` (§2.5) when you need exact extents.

### 2.4 `o Terrain_*` groups are skipped

`Mesh::loadOBJ` drops every vertex and face between an `o` line whose name
starts with `Terrain_` and the next `o` line (`mesh.cpp:88-96`). The
extractor currently tags chunks `Chunk_<hex8>`, so nothing is skipped
today — but **Phase 1.3 must not name its terrain groups `Terrain_…`**, or
the geometry that matters most (see §4) will be silently discarded.

### 2.5 Recast parameters and exit codes

Rebuilt `NavBuilder.exe` only. Parameters follow the four positional
arguments as `key=value`, `--key=value` or `--key value`. Every default is
the constant that used to be hard-coded in `builder.cpp`.

| Key | Default | Meaning |
|---|---|---|
| `cs`, `ch` | `0.3`, `0.2` | voxel size, metres. Do not raise `cs`: doorways close (§4) |
| `agentHeight` | `0.6` | clearance, metres → `walkableHeight = ceil(h / ch)`. Written to the `.nav` header |
| `agentClimb` | `0.9` | max step, metres → `walkableClimb = floor(c / ch)`. Written to the header |
| `agentRadius` | `0.6` | erosion, metres → `walkableRadius = ceil(r / cs)`. Written to the header **and used by the server** (§6.4) |
| `slope` | `45` | max walkable slope, degrees |
| `maxEdgeLen` | `12` | max contour edge, metres; `0` disables splitting |
| `maxSimplificationError` | `1.3` | contour deviation, **voxels** (× `cs` for metres) |
| `minRegionSize` | `8` | cell *side*; squared internally (RecastDemo convention). `8` → 64 cells → 5.76 m² |
| `mergeRegionSize` | `20` | same convention |
| `maxVertsPerPoly` | `6` | 3–6; Detour's `DT_VERTS_PER_POLYGON` is 6 |
| `detailSampleDist`, `detailSampleMaxError` | `6`, `1` | multiples of `cs` / `ch` |
| `partition` | `monotone` | `monotone` or `watershed` |
| `bounds` | — | `minX,minZ,maxX,maxZ` crop, BigWorld metres. Overrides the vertex/chunk bounds union on X and Z; Recast clips triangles to it. Quote it in PowerShell |
| `tile` | `0` | tile side in cells, `16`–`4096`. `0` writes the single-mesh XRC layout, byte-identical to before; anything else writes the tiled `XRCT` layout (§10) |
| `threads` | `4` | tiled build workers, `1`–`64`. The output does not depend on it |
| `seamFilter` | `1` | tiled only: drop the small islands left along tile seams (§10). `0` is for diagnosis |

| Exit | Meaning |
|---|---|
| 0 | `.nav` (or `.obj`) written. An **empty** mesh is still exit 0, with a `WARNING` line |
| 1 | usage: wrong argument count, unknown key, malformed or out-of-range value, bad mode/format |
| 2 | internal error (exception — e.g. the input directory cannot be enumerated, a face index out of range) |
| 3 | Recast build failed, including both 16-bit caps in §6.3, or no chunk OBJs found |
| 4 | output file could not be opened or written |

A crash inside Recast (§6.3, monotone region overflow) surfaces as the usual
Windows `0xC0000005`, which is also non-zero.

On success the last log line is the one to read:

```text
[07:28:17 INFO    ] Navmesh: nverts=15463 npolys=7746 edges=21000 (caps: 65534 verts, 65535 edges) detailVerts=37495 detailTris=23325
```

Recast's own diagnostics are forwarded with a `Recast:` prefix.
`delaunayHull: Removing dangling face` is benign and rate-limited to three
lines plus a total. `rcBuildContours: Bad outline for region N` and
`rcBuildRegions: N overlapping regions` are **not** benign — Recast logs
them as errors but carries on, and the named region is dropped or
mis-stitched.

## 3. Failure modes NavBuilder will not tell you about

The first row applies to the reference `NavBuilder_d.exe` only; the rebuilt
binary reports it through its exit code (§2.5). The rest apply to both.

| Symptom | Cause | Guard |
|---|---|---|
| Process exits **0**, no output file (reference binary) | every failure path in the old `exportNavmesh` logs `FAULT` and returns `void` | the wrapper scripts check the exit code **and** test for a non-empty output file |
| `Could not triangulate contours`, nothing else (reference binary) | Recast's 16-bit vertex cap; the old binary discards Recast's `Too many vertices N` line | rebuilt binary prints it; see §6.3 |
| `.nav` builds and loads, but has thousands of components and named points that were connected in a smaller build no longer are | Recast's **unchecked** 16-bit adjacency-edge cap (§6.3). The reference binary writes the corrupt mesh | rebuilt binary counts edges and exits 3 |
| `Failed to create heightfield`, no output | an OBJ in the input directory whose stem is not `<hex8>o`. The `else` branch of `MapChunk::load` sets only `chunkId_`; `positionX_`, `positionZ_`, `sizeX_`, `sizeZ_` stay **uninitialised**, and `builder.cpp:98-109` folds the garbage into `bmin`/`bmax` | the wrapper scripts reject any non-conforming `*.obj` in the chunk directory before invoking NavBuilder |
| Same as above, in `chunked` mode | the combined `<map>.obj` that `extract_map` writes next to the per-chunk files. It is also loaded, *and* it duplicates every triangle | keep chunk OBJs in their own subdirectory (the wrappers use `<out>/chunks/`) |
| `.nav` with `npolys = 0` | wrong OBJ column order (§1.2), or an all-inverted mesh | `nav_inspect` reports `components 0` |
| `.nav` looks fine, NPCs still stuck | disconnected components | `nav_inspect --probe` |

## 4. Calibration: StaticMesh-only vs the shipped Castle Cellblock mesh

Extraction (`staticmesh::extract_chunk` over all 64 chunks, PackageIndex
cache present):

| Metric | Value |
|---|---|
| chunks enumerated / with geometry | 64 / 9 |
| StaticMeshActors resolved / unresolved | 1699 / 0 |
| triangles emitted | 391,075 |

NavBuilder output vs `data/spaces/castle_cellblock.nav`:

| Metric | Shipped (2013) | StaticMesh-only (2026-09-19) |
|---|---|---|
| polygons | 1,479 | 331 |
| vertices | 2,778 | 907 |
| connected components | 50 | 83 |
| total walkable XZ area | 712,506 m² | 3,935 m² |
| … of which one flat ground sheet | 637,283 m² at `y = 0.2` | — (absent) |
| non-ground walkable area | 75,223 m² | 3,935 m² |
| `bmin` / `bmax` x | −400 / 400 | −351.89 / 99.00 |
| `bmin` / `bmax` z | −400 / 400 | −240.32 / 99.00 |
| `bmin` / `bmax` y | −500 / 500.6 | 22.10 / 136.16 |

(The `99.00` upper bounds are §2.3's chunk padding: the highest
geometry-bearing chunk index is `−1`, so `−1 + 100 = 99`.)

Overlap, sampling all 1,217 non-ground reference polygon centroids against
the new mesh:

| Tolerance | Covered |
|---|---|
| ≤ 1.0 m horizontal, ≤ 2.0 m vertical | 92 / 1217 (**7.6 %**), 2,565 m² |
| ≤ 3.0 m horizontal, ≤ 3.0 m vertical | 124 / 1217 (10.2 %) |
| mean \|Δy\| over covered polys | 0.246 m |

Read that as **precision ≈ 65 %, recall ≈ 3.4 %** by area: where the
StaticMesh-only mesh exists it sits within a quarter of a metre of the
original walkable surface, but it covers almost none of it.

### What is missing, and why it is not Recast tuning

`Castle_CellBlock-fffefffd.umap` — the densest chunk — contains 469
`StaticMeshActor` and **25 `Terrain` / 25 `TerrainComponent`** exports, and
no `Brush`/`Model` at all. The shipped mesh's two largest non-ground
components are perfectly flat sheets (30,499 m² at `y = 94.6`; 22,838 m² at
`y = 53.4`) — the signature of terrain, not of prop meshes.

**Terrain decode (extractor Phase 1.3) is the critical path for a usable
Castle navmesh.** BSP (Phase 1.4) is *not* needed for Castle Cellblock,
though `Castle/` itself does carry ~220 `Brush` per interior chunk and may
still need it.

### Recast parameters — observations only, not retuned here

Current values (`builder.cpp:74-88`): `cs 0.3`, `ch 0.2`, `agentHeight 0.6`,
`agentClimb 0.9`, `agentRadius 0.6`, slope 45°, `minRegionArea 8²`,
`mergeRegionArea 20²`, monotone partitioning.

- `agentHeight = 0.6 m` is a third of an SGW humanoid. It only gates
  vertical clearance, so the effect is to make crawlspaces, shelf tops and
  under-furniture walkable. Raising it to ~1.9 m
  (`walkableHeight = 10` cells) would prune a lot of the 83 junk components.
- `agentClimb (0.9 m) > agentHeight (0.6 m)` is internally inconsistent —
  an agent that can step higher than it is tall. Recast uses it in
  `rcFilterLedgeSpans` and `rcFilterLowHangingWalkableObstacles`, so this
  currently welds separate storeys of stairs together.
- `cs = 0.3 m` is coarse for interiors. After the 0.6 m radius erosion a
  doorway narrower than ~1.4 m can close entirely, which is a plausible
  contributor to the 83-component fragmentation.
- `minRegionArea = 5.76 m²` leaves a long tail of one-polygon islands (the
  shipped `harset.nav` has 1,939 components, so this is not new).

None of these were changed for Castle Cellblock. They *were* measured on
Castle (World 8) once Terrain and BSP landed — see §6.4 for what each one
actually does to the mesh and the recommended set.

## 5. `nav_inspect`

```text
nav_inspect <file.nav>
    [--probe NAME=X,Y,Z]...   named probe point, BigWorld coordinates
    [--probes FILE]           probe list: "NAME X Y Z" or "X Y Z", # comments
    [--h-tol METRES]          max horizontal gap to a polygon (default 2.0)
    [--v-tol METRES]          max |vertical| gap        (default 3.0)
    [--max-components N]      fail if the mesh has more than N regions
    [--gaps]                  gap report between the probes' components (§6)
    [--gap-pair A,B]...       gap report between two component ids
    [--gap-h METRES]          gap search radius, horizontal (default 3.0)
    [--gap-v METRES]          gap search radius, vertical   (default 3.0)
    [--gap-count N]           approaches listed per pair    (default 5)
    [--quiet]                 suppress the per-component table
```

| Exit | Meaning |
|---|---|
| 0 | all probes resolved, all in one component |
| 1 | usage / IO / parse error |
| 2 | a probe had no polygon within tolerance |
| 3 | probes resolved into more than one component |
| 4 | `--max-components` exceeded |

Connectivity is read from Recast's own per-edge adjacency
(`polys[p*nvp*2 + nvp + e]`), which is exactly what `dtCreateNavMeshData`
copies into the runtime tile — so a component split here is a component
split at runtime.

### Reading the output

```console
$ nav_inspect data/spaces/castle_cellblock.nav --quiet \
      --probe ground=0,0.2,0 --probe cell=-100,34.6,-100
components  50 (total walkable XZ area 712505.8 m^2)

probes (h-tol 2.00 m, v-tol 3.00 m)
  ground               poly=207    component=0    h=0.00 m  dy=-0.00 m  ok
  cell                 poly=1205   component=8    h=0.00 m  dy=-0.00 m  ok
nav_inspect: probes span 2 components: [("ground", 0), ("cell", 8)]
# exit 3
```

**`--max-components` is not a useful global gate.** The shipped meshes are
already heavily fragmented — `castle_cellblock.nav` has 50 components and
`harset.nav` has 1,939 — because props, ledges and separate storeys each
become their own island. CA14's criterion has to be *"these named points
are mutually reachable"*, which is what `--probe` tests, not *"the mesh is
one region"*.

### Probe location is tolerance-first

`NavGraph::locate_within` (`nav_components/mod.rs`) considers only polygons
inside **both** tolerances and then picks the nearest in 3-D. The naive
"nearest in XZ" rule (`NavGraph::locate`, still there for the off-mesh
fallback) prefers any polygon whose XZ footprint contains the probe over a
nearer polygon on the right storey, so on a map with terrain and BSP hull
skins under the interiors a probe sitting 1–2 m outside its floor polygon
resolved to a sheet tens of metres below and was reported
`OUT OF TOLERANCE` while standing on the mesh. That affected `armory`,
`throne_room` and `opcore` on the whole-map Castle build; all three read
`ok` now.

The remaining thing to watch is the opposite: a probe reported `ok` at a
horizontal distance close to `--h-tol` is *not* on a polygon. On the
recommended Castle set `armory` reads `h=1.49 m ok` because its own 11 m²
ring pad was below `minRegionSize` and got deleted, so the probe snapped to
the interior floor 1.49 m away. **Read the `h=` column, not just `ok`.**

## 6. When probes land in different components: find the gap, then classify it

`nav_inspect` exiting 3 tells you the mesh is split. It does not tell you
*where*, and the useless way to find out is to measure component centroid to
component centroid — on Castle that reports "these two regions are 300 m
apart" for a route that is actually broken in four specific places.

### 6.1 `nav_inspect --gaps` — where the rims nearly meet

`--gaps` indexes every **boundary edge** (a polygon edge with no in-mesh
neighbour, i.e. the rim of an island), buckets them into an XZ hash grid, and
reports, for each pair of components that come within `(--gap-h, --gap-v)`:

- the closest approaches between the two rims, best first, with the BigWorld
  coordinates of both sides, the horizontal gap and the signed vertical step;
- a **chain search** — the cheapest sequence of intermediate components that
  would link the two if every gap under the thresholds were bridged.

Horizontal and vertical are reported separately because they have different
causes: a horizontal gap is erosion or missing geometry, a vertical one is
`agentClimb` or a ledge. A pair whose rims overlap in XZ reports `h=0.00` and
the whole obstacle in `dy` — that is the stacked-storey shape.

The chain minimises the **widest** hop first and the total second, because
the actionable number is the worst thing you have to bridge, not the sum. A
route broken in three places therefore reads as three short gaps with named
waypoints, not one impossible jump:

```console
$ nav_inspect castle.nav --probes castle_probes.txt --quiet --gaps \
      --gap-h 6 --gap-v 6

  component 218 (23186 m^2, 1114 polys) <-> 405 (37214 m^2, 1344 polys)
    direct: nothing within the search radius
    chain: 4 hop(s), widest 3.43 m, total 5.12 m
       218 -> 306  h= 0.30 m  dy= -0.40 m   at (723.2, 28.4, 459.9)  [306 (407 m^2, 5 polys)]
       306 -> 308  h= 0.00 m  dy= +4.20 m   at (715.1, 26.8, 468.0)  [308 (1560 m^2, 37 polys)]
       308 -> 352  h= 3.43 m  dy= +2.29 m   at (676.2, 18.7, 488.5)  [352 (1306 m^2, 15 polys)]
       352 -> 405  h= 1.39 m  dy= -1.31 m   at (622.7, 24.0, 508.5)  [405 (37214 m^2, 1344 polys)]
```

Start at `--gap-h 3 --gap-v 3` and widen until a chain appears; the value at
which it does is itself the answer ("nothing under 6 m links these"). The
search is on the whole graph — the chain needs the intermediates even when
only two components were named — and costs about 0.15 s on the 21,805-polygon
Castle mesh. Gap reporting never changes the exit code.

Implementation and unit tests: `crates/navmesh-extractor/src/nav_components/gaps/`.

### 6.2 `obj_slab` — what the source geometry is doing there

A gap's coordinates are only half an answer. `obj_slab` reads the chunk OBJs
the navmesh was built from and measures a box of BigWorld space, so the gap
can be classified against the geometry rather than guessed at:

```text
obj_slab <chunk-dir>
    [--at NAME=X,Y,Z[,HALF_XZ[,HALF_Y]]]...   box centred on a point
    [--box NAME=X0,Y0,Z0,X1,Y1,Z1]...        explicit box
    [--column X,Z]...                        surfaces stacked at a point
    [--line X0,Z0,X1,Z1]                     free runs across a line
    [--band YLO,YHI]                         occupancy band
    [--levels BUCKET_METRES]                 horizontal area by height
    [--cell METRES]  [--tilt DEGREES]  [--margin METRES]
```

Exit `2` means a box came back empty — no source geometry there at all,
which is itself a classification. A chunk's id encodes its 100 m grid cell,
so only the chunks that can touch a box are opened (9 of 144 for a typical
query, ~0.3 s instead of a 400 MB scan); `--margin` widens that test for
actors that overhang their owning chunk.

The classification each measurement supports:

| Cause | Measure with | Signature |
|---|---|---|
| (a) opening narrower than the erosion budget | `--line` across the doorway with `--band` at knee-to-head height | widest clear run under ~1.5 m. `agentRadius 0.6` at `cs 0.3` erodes 2 cells (0.6 m) **per side** |
| (b) step or ledge over `agentClimb` | `--column` at the gap | consecutive surfaces more than 0.6 m apart |
| (c) slope over 45° | `--at` + the `footprint` line | area concentrated in the `ramp 45-60` / `steep>60` buckets. Confirm by rebuilding a crop with `slope=60` |
| (d) geometry not extracted | `--at` returning `EMPTY`, or a `--levels` band with no surfaces | nothing where the client clearly has something. Cross-check `coverage.tsv`'s `skip_archetype_stub_component` and `undecoded_InterpActor` for that chunk |
| (e) something blocking | `--line` fully blocked, plus `--column` showing a floor on both sides | a door mesh, BSP face or collision blocker standing in an otherwise walkable opening |
| (f) low ceiling | `headroom` via `--column` | next surface less than `agentHeight` (1.8 m) above the floor |
| (g) region simplification | rebuild a crop: `bounds=<box>` at `maxSimplificationError=1.3 minRegionSize=8` | the link appears in the cropped build. Cropped builds have plenty of edge budget |

Rebuilding a crop is the decisive test for (c) and (g) and the elimination
test for everything else: if a link does not appear at
`cs=0.15 ch=0.1 agentRadius=0.15 minRegionSize=2 maxSimplificationError=1.3`,
no parameter will produce it and the cause is (d) or (e).

Implementation and unit tests: `crates/navmesh-extractor/src/obj_slab/`.

## 7. Castle (World 8) connectivity

Moved to its own reference page:
**[castle-navmesh-connectivity.md](castle-navmesh-connectivity.md)**. It
covers where the eleven named probes land, the mirrored-instance bug that
used to split the interior into two storeys, why the 108-byte `Brush`-owned
`Model`s are correct cooked data rather than a decoder gap, the terrain
shelves that still separate the exterior from the keep, and the classes
that have been ruled out as the answer.

## 8. Rebuilding NavBuilder, and Recast's index limits

Moved to its own reference page:
**[navbuilder-recast-limits.md](navbuilder-recast-limits.md)**. It covers
`tools/build-navbuilder.ps1`, parity with the 2026-03 reference binary, the
four fixed-width index spaces that cap a single `rcPolyMesh` (contour
vertices, adjacency edges, region ids, and the 24-bit compact-heightfield
span index), the measured Castle (World 8) parameter table, and what to do
when a build stops fitting.

## 9. Every world (NA26, 2026-09-25)

All 23 client maps the server creates spaces for (every `entities/spaces.xml`
world except `SandBox`, which loads a copy of `harset_cmdcenter.nav`) were
extracted and built with this pipeline. The per-map parameters, sizes,
component counts, probe results and the old-vs-new telemetry comparison are
in the provenance table in [data/spaces/README.md](../../data/spaces/README.md);
that table is the reference, this section is the method.

**Extraction.** `extract_map <cooked-root> <Map> <out>/chunks <index>` per map,
with the shared `PackageIndex` cache. The biggest map (Lucia, 891 chunks,
29.6 M triangles, 3.1 GB of OBJ) takes 38 s; all 23 together about 15 GB of
scratch OBJ. Never commit it.

**Parameters.** The agent is the Castle one for every map
(`partition=watershed agentHeight=1.8 agentClimb=0.6`, radius 0.6).
`bounds=` is always passed: at least the chunk grid plus 20 m, because a
stray skybox or backdrop actor otherwise stretches the heightfield (the
unclipped Omega_Site_CmdCenter build was 16,347 × 16,347 columns for a
500 × 300 m map). Then the first rung of this ladder that fits every cap
([navbuilder-recast-limits.md](navbuilder-recast-limits.md)):

| Rung | `cs` | `minRegionSize` | `maxSimplificationError` |
|---|---|---|---|
| 1 | 0.3 | 24 | 1.3 |
| 2 | 0.3 | 24 | 2.5 |
| 3 | 0.45 | 16 | 2.5 |
| 4 | 0.6 | 12 | 2.5 |
| 5 | 0.6 | 12 | 3.0 |
| 6 | 0.75 | 10 | 3.0 |
| 7 | 0.9 | 8 | 3.5 |

`minRegionSize` is a cell count, so it shrinks as `cs` grows to keep the
smallest kept region near 52 m². Where no rung fits the whole map (Agnos,
Lucia, Tollana), or only the coarsest does (Beta_Site_Evo_1), the build is
cropped to the window with the most seeded content and chunk geometry and
the ladder is re-run on the crop.

**Results.** Every interior and the mid-sized exteriors fit at `cs=0.3`.
Dakara_E1 and both Menfa maps fit whole only at `cs=0.6`, where a doorway
narrower than about 2 m can close. The four cropped maps have no mesh
outside their crop.

**Validation.** Each mesh is probed with the server's own
`NavMesh::is_point_valid` over the world's seeded spawn, respawner, ring,
gate and point-set rows, and every replaced mesh is scored against the old
one on the real player positions in SigNoz's `movement.validation_reject`
rows (`last_valid_*` as accepted positions, `client_*` as rejected ones,
round-number teleport points dropped). The `movement.position_sample` rows
are DEBUG and were not exported, so the accepted set is small.

**A fifth unchecked Recast limit.** `rcSpan` stores heights in 13 bits
(`RC_SPAN_HEIGHT_BITS`), and rasterization clamps every span to 8,191 cells
above `bmin.y` without a diagnostic. At `ch=0.2` that is 1,638 m of vertical
extent. Tollana has one prop at y -1728, so the whole city was clamped onto
one ceiling and the build "succeeded" with a single 5 km² sheet at y -90.
NavBuilder now exits 3 when `(bmax.y - bmin.y) / ch > 8191`
(`tests/navbuilder_axis_roundtrip.rs::a_vertical_extent_past_the_13_bit_span_height_is_refused`);
raise `ch` (Tollana ships at `ch=0.3`). `bounds=` does not crop Y.

**Superseded for the big exteriors.** NA28 rebuilt the four cropped maps
whole and Dakara_E1 and both Menfa maps at `cs=0.3` with the tiled mode in
§10, so no shipped mesh is cropped any more. The ladder above still describes
how every other world was built.

## 10. Tiled builds (NA28, 2026-09-25)

Every Recast index cap in
[navbuilder-recast-limits.md](navbuilder-recast-limits.md) is per
`rcPolyMesh`. `tile=<cells>` builds one `rcPolyMesh` per tile, so the caps
apply per tile and a whole outdoor map at `cs=0.3` fits: Beta_Site_Evo_1's
largest 128-cell tile has 76,159 spans, 540 contour vertices and 512
adjacency edges, against caps of 16.7 M, 65,534 and 65,535.

```bash
NavBuilder chunked <chunks> <out.nav> nav partition=watershed agentHeight=1.8 agentClimb=0.6 \
    cs=0.3 minRegionSize=24 maxSimplificationError=2.5 bounds=<chunk grid + 20 m> tile=128 threads=6
```

**What the builder does.** It computes the whole-map config exactly as the
single-mesh build does (bounds, the 13-bit height check, the derived cell
counts), then follows RecastDemo's `Sample_TileMesh`:

1. Mark triangle walkability once, then bin every triangle into the tiles
   its XZ box touches, border included. Unwalkable triangles are kept; they
   are the walls.
2. Per tile, on `threads` workers: a heightfield of `tile + 2 × border`
   cells, `border = walkableRadius + 3`, then the same pipeline as a single
   mesh (`recast_pipeline.cpp`) with `borderSize` passed to the region
   builder. That makes `rcBuildPolyMesh` mark the edges on the tile's sides
   as portals (`0x8000 | side`) instead of boundaries. A tile with no
   triangles or nothing walkable is skipped. Each non-empty tile logs one
   line: triangles, spans, regions, contour vertices, `nverts`, `npolys`,
   edges. Recast's own messages carry a `Recast Tile x,y:` prefix.
3. **Seam filter** (`tile_seam_filter.cpp`). `rcBuildRegions` never drops
   a small region that touches the tile border, because it cannot see
   whether the region continues next door. So every small island that
   straddles a seam survives: Agnos in 128-cell tiles came out with 1,262
   components under 10 m², against none in the single-mesh build. The
   filter joins the tiles the way Detour will (below), sums each connected
   component's compact spans per region (what `rcBuildRegions` measures),
   and deletes every component under `minRegionSize² × cs²`, the
   single-mesh threshold. It counts spans rather than polygon area because
   contour simplification narrows thin walkways, and polygon area would
   drop walkways the region builder keeps.
4. Check the poly-ref budget: a 32-bit `dtPolyRef` is salt, tile and
   polygon index, and `dtNavMesh::init` refuses fewer than 10 salt bits, so
   `bits(tiles) + bits(largest tile's polygons) ≤ 22`. Beta_Site_Evo_1 is
   13 + 8. Past it the build exits 3; use bigger tiles.
5. Write the tiles in row-major order. The file does not depend on the
   thread count (`tests/navbuilder_tiled.rs` builds with 1 and 4 workers
   and compares bytes).

`tile=0` (the default) runs the old code path. Its output is byte-identical
to the pre-NA28 builder: Castle_CellBlock, Harset_CmdCenter and SGC, each at
the defaults and at NA26's parameters, hash the same, and the INFO lines
match.

**The file.** Detour's usual multi-tile file (RecastDemo's `MSET`) stores
each tile as the bytes `dtCreateNavMeshData` produced, which is Detour's
in-memory layout, and `addTile` checks little beyond its magic. The tiled
XRC layout keeps Recast's arrays on disk instead, so tiles go through the
same capped, streaming reader as a single mesh and the Detour serialisation
stays inside the loader:

```text
"XRCT", version u32 = 1
agentHeight, agentClimb, agentRadius     3 × f32
orig                                     3 × f32   dtNavMeshParams::orig
tileWidth, tileHeight                    2 × f32   metres
ntiles, maxTilePolys                     2 × u32
ntiles × { tileX i32, tileY i32, <the single-mesh layout from nverts on> }
```

A single-mesh file starts with `agentHeight` as an `f32`; `"XRCT"` read that
way is about 3.4e12, so the loader tells the two apart by the first four
bytes.

**The loader** (`crates/entity/src/navigation/load_tiled.rs`) checks the
version, the tile count (at most 65,536), `maxTilePolys` and the poly-ref
budget before it allocates anything, initialises a `dtNavMesh` from
`dtNavMeshParams`, then reads each tile under per-tile caps (`nverts`,
`npolys` ≤ 0xfffe, `nvp` ≤ 6, `npolys` ≤ `maxTilePolys`), builds it with
`dtCreateNavMeshData` at its `(tileX, tileY)` and adds it. Detour links the
portal edges of neighbouring tiles itself (`connectExtLinks`), so every
query in `NavMesh` works across tile borders unchanged. The fingerprint
hashes the whole file as before and records `tiles`; `bmin`/`bmax` are the
union of the tiles'. The synthetic two-tile tests in
`navigation/tests/tiled.rs` cover a path, a height query, a sight line, a
slide and a recovery across the border, and the same fixture with the
portal markers removed, which must come back as two islands.

**`nav_inspect`** reads both layouts. For a tiled file
`NavGraph::from_tiled` links portal edges with Detour's own test: edges on
facing sides of neighbouring tiles, on the same line within 0.01 m,
overlapping by more than 0.01 m at each end, with heights crossing or within
`2 × agentClimb` (`overlapSlabs`). The seam filter uses the same test in
C++; on Beta_Site_Evo_1 both count 2,332 components before filtering.

**Choosing the tile.** 128 cells at `cs=0.3` is 38.4 m, which puts the
biggest map (Beta_Site_Evo_1, 4,352 tiles) at 13 tile bits and 8 poly bits.
Smaller tiles add seam vertices and polygons and use more tile bits; larger
ones buy nothing, since no tile of a real map comes near a cap. The seven
NA28 meshes and their numbers are in
[data/spaces/README.md](../../data/spaces/README.md).

## Cross-references

- [data/spaces/README.md](../../data/spaces/README.md) — per-world parameters, validation and containment mode
- [castle-navmesh-connectivity.md](castle-navmesh-connectivity.md) — where Castle's probes land, the mirrored-instance fix, and what is still split
- [castle-extraction-measurements.md](castle-extraction-measurements.md) — what the extractor recovers from Castle, per source and per class
- [navbuilder-recast-limits.md](navbuilder-recast-limits.md) — rebuilding NavBuilder, Recast's four index limits, the Castle parameter table
- [crates/navmesh-extractor/README.md](../../crates/navmesh-extractor/README.md) — extractor phases and status
- [cover-extraction.md](cover-extraction.md) — the same crate's `cover_extract` tool: world-space cover nodes from the chunks, using this axis mapping
- [ue3-package-format.md](ue3-package-format.md) — the `.umap` container this all starts from
- `deprecated/cpp/src/nav_builder/` — NavBuilder source (`builder.cpp`, `chunk.cpp`, `mesh.cpp`, `mesh_exporter.cpp`; the Recast pipeline in `recast_pipeline.cpp`, the tiled mode in `tiled_builder.cpp` and `tile_seam_filter.cpp`, both file layouts in `xrc_writer.cpp`)
- `crates/entity/src/navigation/` — runtime loader (Detour FFI)
