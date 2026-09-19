# Navmesh Build Pipeline (UE3 → OBJ → NavBuilder → `.nav`)

> **Last updated**: 2026-09-19
> **Status**: Verified end-to-end against the prebuilt `NavBuilder_d.exe` and the shipped 2013 `castle_cellblock.nav`.

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
        │  bin64/NavBuilder_d.exe chunked <chunks> <out.nav> nav
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
NavBuilder_d.exe <chunked|whole> <input> <output> <nav|obj>
```

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
and (b) a little wasted heightfield. Fixing `chunk.cpp` would need a C++
rebuild, which this spike did not attempt.

### 2.4 `o Terrain_*` groups are skipped

`Mesh::loadOBJ` drops every vertex and face between an `o` line whose name
starts with `Terrain_` and the next `o` line (`mesh.cpp:88-96`). The
extractor currently tags chunks `Chunk_<hex8>`, so nothing is skipped
today — but **Phase 1.3 must not name its terrain groups `Terrain_…`**, or
the geometry that matters most (see §4) will be silently discarded.

## 3. Failure modes NavBuilder will not tell you about

| Symptom | Cause | Guard |
|---|---|---|
| Process exits **0**, no output file | every failure path in `exportNavmesh` logs `FAULT` and returns `void` (`builder.cpp:119`, `139`, `153`, …) | the wrapper scripts test for a non-empty output file; never trust `$?` |
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

None of these were changed. Re-tune **after** Terrain lands: with 3 % of the
walkable surface present, any parameter sweep is measuring noise.

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

## Cross-references

- [crates/navmesh-extractor/README.md](../../crates/navmesh-extractor/README.md) — extractor phases and status
- [ue3-package-format.md](ue3-package-format.md) — the `.umap` container this all starts from
- `deprecated/cpp/src/nav_builder/` — NavBuilder source (`builder.cpp`, `chunk.cpp`, `mesh.cpp`, `mesh_exporter.cpp`)
- `crates/entity/src/navigation/` — runtime loader (Detour FFI)
