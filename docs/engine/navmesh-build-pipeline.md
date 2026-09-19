# Navmesh Build Pipeline (UE3 → OBJ → NavBuilder → `.nav`)

> **Last updated**: 2026-09-19
> **Status**: Verified end-to-end against the prebuilt `NavBuilder_d.exe` and the shipped 2013 `castle_cellblock.nav`. §2.5, §6 (gap finding and classification) and §7 (Castle connectivity) measured on the 144-chunk Castle extraction the same day; the builder reference moved to [navbuilder-recast-limits.md](navbuilder-recast-limits.md).

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

## 7. Castle (World 8): why the probes sit in three components

Measured 2026-09-19 on the 144-chunk extraction, recommended parameter set,
and re-measured against the same extraction with `bCollideActors = false`
props suppressed (40,093 / 19,824 / 55,132, 553 components). **The grouping
is identical in both**, and so is every number this section turns on:

| Component (props on / suppressed) | Area | Probes |
|---|---|---|
| 218 / 116 | 23,186 → 22,844 m² | `gate_room_dhd`, `stargate`, `bunker_muelbach`, `checkpoint_bravo` |
| 754 / 435 | 17,022 → 16,415 m² | `zuritska_cell`, `romney_corridor`, `comms_room`, `nid_guard_116`, `opcore`, `armory` |
| 405 / 250 | 37,214 → 36,451 m² | `throne_room` |

### 7.1 405 ↔ 754 — a storey boundary, not a tuning problem

The two components' rims overlap in XZ at many points around
`(276…292, ·, 865…886)` with **`h = 0.00 m` and `dy = +12.00 m` exactly**.
`obj_slab --column 282,875` shows why: a floor at `y = 43.2`, a slab
underside at `51.2`, and the interior floor at `55.2`. They are two storeys
of the same building, 12 m apart, with a 4 m slab between them.

Nothing links them at any parameter set. Tested, all on the interior crop
`bounds=150,500,700,1150`, and all still split:

| Build | Result |
|---|---|
| `minRegionSize=8 maxSimplificationError=1.3` (rules out (g)) | split |
| `slope=60` (rules out (c)) | split |
| `agentClimb=1.2` (rules out (b)) | split |
| `agentRadius=0.3` (rules out (a)) | split |
| `agentHeight=1.2` (rules out (f)) | split |
| `slope=60 agentClimb=1.5 agentRadius=0.2 agentHeight=1.2` | split, and `comms_room` splits from `zuritska_cell` as under-floor crawl space becomes walkable |
| tight crop `bounds=190,830,470,960` at `cs=0.15 ch=0.1 agentRadius=0.15 minRegionSize=2` | split, still exactly 12.00 m |

`obj_slab --levels 1.0` over the overlap does find near-horizontal surface
at every metre between 43 and 56 (517 m² at 44–45, 1,217 m² at 47–48,
3,042 m² at 51–52, 4,284 m² at 54–55), so the building has intermediate
levels — but none of it is connected to either storey.

> **The direct 12 m approach is not where the connection is.** A
> traversal-keyword scan of all 6,430 Castle `StaticMeshActor`s found two
> flights of three `CA-Props:CA-Stair00` segments at
> `x = 348.64 / 355.04 / 361.44`, `z = 846.34` (`y 46.24`) and `z = 884.32`
> (`y 54.40`) — **30 m east of the box above**, with
> `CA-Interior:CA-large_doorway_open_a_00` at the foot and head of each and
> a `CA-large_hallway_ramp_a_00` between. All are direct actors with
> collision on, so all are already in the OBJ and in the mesh.
>
> The gap finder pointed at the wrong place because its hop cost weighted
> only the horizontal gap, and two floors of one building overlap in XZ:
> the storey jump reported `h = 0.00` and therefore scored as **free**.
> `Approach::bridge_size` is now `max(horizontal, |vertical|)` and the
> 12 m jump costs 12. Regression test:
> `gaps::tests::a_stacked_storey_jump_does_not_beat_a_real_route`.

With the cost fixed, the cheapest bridge between the two on the whole-map
mesh runs through the stair spine, not through the slab: `250 → 430`
(`h = 3.61 m`, `dy = +6.80 m`, at `(362.3, 48.4, 842.7)`) then `430 → 435`
(`h = 8.10 m`, `dy = +7.20 m`, at `(362.0, 63.0, 872.7)`).

**And the stairs still do not join the halls.** Cropping to
`bounds=320,770,410,920` and probing the lower hall (`355.04, 48.4, 830`)
against the upper (`355.04, 55.2, 885`):

| Build | Components | Probes joined? |
|---|---|---|
| `minRegionSize=24 mse=2.5` | 15 | no, and no chain under 3 m |
| `minRegionSize=8 mse=1.3` | 69 | no |
| `minRegionSize=2 mse=1.3` | 138 | no |
| `minRegionSize=2 mse=1.3 cs=0.15 ch=0.1 agentRadius=0.3` | 275 | no |
| `minRegionSize=1 mse=0.8 cs=0.1 ch=0.05 agentRadius=0.3` | 328 | no |

So it is not `minRegionSize` eating the treads either, even though they are
small (20–40 m² per 0.5 m of height). Nor is it slope: the same box measures
1,077 m² of `walkable ≤ 45°` against 1.2 m² of 45–60° and 8.9 m² over 60°.

What `obj_slab --levels 0.5` shows is that **each flight only spans about
five metres** — the lower one climbs 46.0 → 51.5, the upper 54.5 → 59.5 —
while the halls sit at 48.4 and 55.2. Neither flight bridges 48.4 → 55.2 by
itself, and there is a ~3 m dead band at 51.5–54.5 with nothing in it but
single-triangle slabs (393 m² from 1 triangle at 52.0–52.5 is a ceiling, not
a tread).

Two candidates remain, and distinguishing them needs eyes on the level
rather than more builds:

1. **The flights serve within-storey level changes**, and the real route
   between 48.4 and 55.2 is somewhere else entirely — or does not exist on
   foot, which is what the seed data's silence would then mean.
2. **Per-`Brush` BSP that we decode to nothing.** Note that "BSP is
   undecoded" is *false* and not the candidate: the level `Model`'s node
   tree is read, and `Castle-00080003` — the stair spine's own tile —
   contributes **878 BSP triangles** to the OBJ already, the second-largest
   of the 16 chunks that carry any (6,810 map-wide). What is empty is the
   other half: every `Brush`-owned `Model` in Castle decodes to a
   **108-byte stub**, 38 of them in `00080003` and 540 map-wide. Either the
   cooker genuinely empties a brush's `Model` once CSG is baked into the
   level `Model` — in which case those 878 triangles are all there is and
   BSP is not the connector — or the 108 bytes are a header we mis-parse
   and there is per-brush geometry being dropped in exactly this tile. One
   `Brush` export hexdump separates the two; twenty more builds will not.

Three classes that are **not** candidates, because Castle has zero exports
of any of them: `StaticMeshCollectionActor`, `KActor`,
`FracturedStaticMeshActor`, `BlockingVolume`.

Two candidates already ruled out:

- **Prefab-archetype StaticMeshActors.** The 33 actors resolved inside
  `x[250,320] y[40,60] z[850,900]` are all set dressing — computer towers,
  view screens, torches, a locker, a wall light, waist-high concrete cover.
  Four of the cover blocks sit at `y = 43.20` and `y = 55.40`, which
  independently confirms that both storeys are real and populated and that
  the 12 m spacing is not an extraction artefact.
- **`InterpActor` movers.** All 14 in Castle resolve to 11
  `EM-SecurityCam01_Top` heads, one `EM-Antenna00`, one `EM-ShelfBox10` and
  one `GLB-RingTransporter00`. **There is no lift, elevator or door among
  them**, so extracting them (they are excluded today — the class filter is
  `== "StaticMeshActor"`) would not close this gap or any other. The nine
  `EM-Elevator00` / `EM-Elevator_Pad00` instances in Castle *are*
  StaticMeshActors, already extracted, and all sit at `y 20–30` on the
  exterior level — two of them on the `116 ↔ 250` side of the map, which is
  where an off-mesh link would go if one is ever added.

### 7.2 218 ↔ 405 — terrain cliffs

The exterior and the mid plateau are separated by terrain, not by a door.
`obj_slab --column 622.7,496…504` measures the bank between them at
**45–58°**, and the chain hops are dominated by `h=0.00` approaches with
`dy` of 3–8 m: cliffs. Relaxing to `slope=60` or `slope=70` on a crop
covering the corridor shortens the chain from 13 hops to 9 and drops the
worst horizontal gap from 2.72 m to 1.62 m, but never joins them, because
the remaining hops are 7.87 m and 6.81 m vertical.

There are **zero** prefab-archetype actors in
`x[600,740] y[15,35] z[450,500]`, so the archetype gap contributes nothing
here either. On the props-suppressed mesh the chain shortens to three hops
(widest 3.12 m) around the same three places — `(715.0, 26.7, 467.9)` with
a 4.25 m ledge, `(675.6, 18.6, 488.3)` with a 3.12 m horizontal gap, and
`(622.4, 24.0, 508.2)` — which is where to look if this one is ever worth
bridging by hand.

### 7.3 The armory is a ring drop zone, not a walk-in room

`db/resources/Worlds/Seed/ring_transport_regions.sql` has exactly one row
for world 8: region 34, `Castle_ArmoryRingDropZone`, at
`(466.365, 70.397, 991.466)` — which is the `armory` probe, to three
decimal places — with an empty `destination_region_ids`. The row that
targets it is region 33, `Cellblock_ArmoryRingSwitch`, in **world 12**
(`required_mission_id` 688). So the armory is reached by a cross-world ring
transport, and its 11 m² pad sits 1.50 m from the interior floor.

That is evidence about one probe, not about the whole interior: the other
five interior probes are in the same component as each other and are reached
on foot from each other. It does **not** show how a player gets from the gate
room to the interior, and nothing in the seed data does — there is no second
ring region for world 8. The expectation that all three groups are walkable
is therefore neither confirmed nor refuted by the seed.

The one `InterpActor` with collision flags set explicitly
(`bCollideActors` / `bBlockActors` / `bPathColliding`, all true) is
`GLB-RingTransporter00` at `(466.45, 70.06, 991.55)` — the same pad. It is
not extracted, but the floor under it is, so adding it would change nothing
about connectivity; it would only raise the pad by the transporter's own
thickness.

### 7.4 What would actually close these gaps

In order of likelihood, and none of it is Recast tuning:

1. **Walk it in the client.** The stair spine at `x 348–361`, `z 840–890`
   is the place to look: three flights, doorways at each end, and a mesh
   that refuses to connect them at `cs = 0.1`. Either the route exists and
   something about the collision hull is wrong, or it does not and §7.3's
   silence in the seed is the answer.
2. **Settle the 108-byte `Brush`-owned `Model` stub** (crate README, Known
   unknowns). 38 of them sit in the stair tile. Hexdump one export: either
   the cooker empties it after CSG bake, which closes BSP as a candidate,
   or we are dropping real geometry here. This is a one-afternoon question
   and it gates candidate 1's interpretation.
3. **Accept that they are separate**, and give the cell a per-region
   navmesh or an off-mesh link table. Both need server-side loader work.
   The two `EM-Elevator00` + `EM-Elevator_Pad00` pairs at
   `(588.0, 21.1, 564.3)` and `(768.8, 29.8, 415.7)` are the natural
   anchors on the exterior side.

Extracting `InterpActor`s is **not** on this list: the 14 in Castle are 11
cameras, an antenna, a shelf box and a ring transporter.

## 8. Rebuilding NavBuilder, and Recast's index limits

Moved to its own reference page:
**[navbuilder-recast-limits.md](navbuilder-recast-limits.md)**. It covers
`tools/build-navbuilder.ps1`, parity with the 2026-03 reference binary, the
four fixed-width index spaces that cap a single `rcPolyMesh` (contour
vertices, adjacency edges, region ids, and the 24-bit compact-heightfield
span index), the measured Castle (World 8) parameter table, and what to do
when a build stops fitting.

## Cross-references

- [navbuilder-recast-limits.md](navbuilder-recast-limits.md) — rebuilding NavBuilder, Recast's four index limits, the Castle parameter table
- [crates/navmesh-extractor/README.md](../../crates/navmesh-extractor/README.md) — extractor phases and status
- [ue3-package-format.md](ue3-package-format.md) — the `.umap` container this all starts from
- `deprecated/cpp/src/nav_builder/` — NavBuilder source (`builder.cpp`, `chunk.cpp`, `mesh.cpp`, `mesh_exporter.cpp`)
- `crates/entity/src/navigation/` — runtime loader (Detour FFI)
