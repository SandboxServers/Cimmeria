# Navmesh Build Pipeline (UE3 → OBJ → NavBuilder → `.nav`)

> **Last updated**: 2026-09-19
> **Status**: Verified end-to-end against the prebuilt `NavBuilder_d.exe` and the shipped 2013 `castle_cellblock.nav`. §2.5 and §6 (tunable NavBuilder, Recast 16-bit limits, Castle parameter set) measured on the 144-chunk Castle extraction the same day.

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

### 1.2 Required change to the extractor

`crates/navmesh-extractor/src/obj.rs` writes raw UE3 columns today
(`v {v[0]} {v[1]} {v[2]}`, line 97). That puts UE3's up-axis on BW `x`, so
every floor rasterises as a vertical wall and NavBuilder writes a
structurally valid but **completely empty** `.nav` (measured:
`npolys = 0`). Two changes are needed, both inside `write_obj_into`:

1. **Column order** — emit `v <x> <z> <y>`:

   ```rust
   // UE3 is Z-up, the OBJ NavBuilder expects is Y-up.
   writeln!(w, "v {} {} {}", v[0], v[2], v[1])?;
   ```

2. **Line endings** — the whole file must be CRLF (see §1.4).

Nothing else changes: the face winding, the `o` group lines and the 1-based
index numbering are all already correct.

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

### A caveat when a probe reads `OUT OF TOLERANCE`

`NavComponents::locate` (`nav_components.rs:273-301`) prefers **any** polygon
whose XZ footprint contains the probe over a nearer polygon on the right
storey, and only then applies `--v-tol`. On a map with terrain under the
interiors, a probe that sits 1–2 m outside its floor polygon (inside the
0.6 m erosion margin, or on top of a prop) resolves to the terrain tens of
metres below and is reported `OUT OF TOLERANCE`, even though a floor polygon
is well within `--h-tol`. On the whole-map Castle mesh this affects `armory`
(h = 1.49 m to the interior component, reported `dy = -69.38 m`),
`throne_room` and `opcore`. Treat `OUT OF TOLERANCE` with a large `dy` as
"check by hand", not as "floor missing".

## 6. Rebuilding NavBuilder, Recast's 16-bit limits, and the Castle parameter set

### 6.1 Building `NavBuilder.exe`

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

### 6.2 Parity with the reference binary

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

### 6.3 Recast has three 16-bit limits, and only one is checked

A single `rcPolyMesh` — which is all the XRC `.nav` format can hold — is
bounded by three separate `unsigned short` index spaces.

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

### 6.4 Castle (World 8): what each parameter does

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
exterior component (23,051 m²); the interior five plus `opcore` share
another; `throne_room` is in a third. That is geometry (doors, ring
transports), not tuning — it is identical at every parameter set above.

### 6.5 When it stops fitting

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

- [crates/navmesh-extractor/README.md](../../crates/navmesh-extractor/README.md) — extractor phases and status
- [ue3-package-format.md](ue3-package-format.md) — the `.umap` container this all starts from
- `deprecated/cpp/src/nav_builder/` — NavBuilder source (`builder.cpp`, `chunk.cpp`, `mesh.cpp`, `mesh_exporter.cpp`)
- `crates/entity/src/navigation/` — runtime loader (Detour FFI)
