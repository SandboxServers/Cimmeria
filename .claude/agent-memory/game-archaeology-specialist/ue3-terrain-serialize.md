---
name: ue3-terrain-serialize
description: ATerrain::Serialize binary layout recovered from SGW.exe for UE3 navmesh extraction (#46) — addresses, trailer layout, decode recipe
metadata:
  type: project
---

## ATerrain::Serialize — SGW.exe RE findings (2026-05-27)

**Primary function**: `ATerrain__vfunc_12` @ `0x007517C0` — confirmed as `ATerrain::Serialize`.

Evidence: FArchive version guards (`param_1[1]` vs `0x103`/`0x107`/`0x15e`/`0x167`), IsLoading/IsSaving checks (`param_1[4]`/`param_1[5]`), calls `UTestIpDrv__vfunc_12` (= `AActor::Serialize` super) as first call.

### Helper function addresses

| Symbol | Address | Role |
|---|---|---|
| `ATerrain__vfunc_12` | `0x007517C0` | `ATerrain::Serialize` |
| `FUN_0075a5b0` | `0x0075A5B0` | `TArray<WORD>` Heights serializer |
| `FUN_0075a700` | `0x0075A700` | `TArray<BYTE>` InfoData/alpha serializer |
| `FUN_0075c9f0` | `0x0075C9F0` | `TArray<TArray<BYTE>>` WeightedTextureMaps serializer |
| `FUN_0075bd80` | `0x0075BD80` | `TArray<FTerrainLayer>` Layers serializer |
| `ATerrain__vfunc_19` | `0x00757240` | PostEditChange — NOT Serialize |

### Export structure

Each Terrain export = **32-byte Actor header** + **UE3 tagged-property stream** (terminated by None FName) + **binary trailer**.

### Tagged properties present (all IntProperty unless noted)

- `NumPatchesX`, `NumPatchesY` — patch count (e.g. 20)
- `NumVerticesX = NumPatchesX + 1`, `NumVerticesY = NumPatchesY + 1` (e.g. 21)
- `AlphaXSize`, `AlphaYSize` — alpha map texel dimensions (e.g. 84)
- `Layers` — ArrayProperty (opaque blob; skip for collision)
- `DrawScale`, `DrawScale3D` — ABSENT in Castle_CellBlock; must default if absent. **`DrawScale` defaults to 1.0, but `DrawScale3D` defaults to `(100, 100, 100)`** for SGW's `ATerrain` class, not the `AActor` `(1,1,1)`. Reading the `AActor` default shrinks the map by 100x. (Corrected 2026-09-19; an earlier revision of this line said `(1,1,1)`.)

**None FName** for Castle_CellBlock = `0x36 0x00 0x00 0x00  0x00 0x00 0x00 0x00` (name index 54).
**SUPERSEDED (2026-09-19, terrain decoder):** the flat tagged-property walk in `crates/upk/src/properties.rs` skips `ArrayProperty` bodies by their declared size, so inner `None` terminators inside `Layers` are never seen and no "last None" search is needed. Kept for anyone hand-walking bytes. **GOTCHA (hand-walk only)**: The `Layers` array contains inner tagged-property sub-blocks each ending in their own None. First None occurrence is inside Layers. Use the LAST None occurrence as the outer terminator. For Terrain_A: inner None at +412, outer None at +805; trailer starts at +813.

### Binary trailer sequence (relative to start of binary trailer)

```
+0x000  INT32 LE    Heights.Num      (= NumVerticesX * NumVerticesY, e.g. 441)
+0x004  UINT16 LE   Heights[0..N-1]  (N * 2 bytes; 0x8000 = flat/neutral)
+0x???  INT32 LE    InfoData.Num     (= same N)
+0x???  UINT8       InfoData[0..N-1] (N bytes; bit 0 = TERRAINFLAG_Invisible)
+0x???  INT32 LE    AlphaXSize       (redundant binary copy — must consume)
+0x???  INT32 LE    AlphaYSize       (redundant binary copy — must consume)
+0x???  INT32 LE    WeightedTextureMaps.Num   (usually 1)
+0x???  INT32 LE    WTM[i].Num       (for each entry; e.g. 7056 = 84*84)
+0x???  UINT8[]     WTM[i].Data      (WTM[i].Num bytes)
+0x???  INT32 LE    WeightMapTextures.Num     (usually 0)
  --- STOP HERE for navmesh/collision extraction ---
+0x???  mixed       lighting GUIDs + foliage proxy data (152 bytes in Castle_CellBlock)
```

### Worked example — Terrain_00000000A, Castle_CellBlock-00000000.umap

- Export serial_offset: 10850 (0x2A62), serial_size: 9372
- NumVerticesX = NumVerticesY = 21 → N = 441
- Outer None at +805 from export start; binary trailer at +813
- Heights: 441 × 0x8000; InfoData: 441 × 0x00; AlphaXSize=AlphaYSize=84; WTM[0].Num=7056; WeightMapTextures.Num=0
- Total trailer: 8559 bytes; 813 + 8559 = 9372 ✓

### World-space conversion

**CORRECTED 2026-09-19 by the Rust decoder landing (worker `nav-terrain`,
branch `navmesh/terrain-decoder`). The three bullets below were wrong; the
struck-through numbers are kept so nobody re-derives them.**

- ~~Z = Location.Z + (height_u16 / 65535.0) * DrawScale * DrawScale3D.Z * 256.0~~
- ~~Cell size X = DrawScale * DrawScale3D.X * 256.0 cm per patch~~
- ~~20×20 terrain = 5120 cm × 5120 cm~~

Correct conversion (UE3 canonical, validated against real data — see
"Real-data validation, Rust decoder" below):

- Local vertex = `(i, j, (height_u16 - 32768) * TERRAIN_ZSCALE)` where
  `TERRAIN_ZSCALE = 1/128`. Then apply the ordinary actor transform:
  scale by `DrawScale * DrawScale3D`, rotate, translate by `Location`.
- `0x8000` ⇒ local Z exactly 0. The old `h/65535*256` form put a flat
  sheet at +128 local units instead of 0.
- Patch spacing = `DrawScale * DrawScale3D.X` cm = **100 cm** in every
  shipped SGW map, so a 20×20 terrain is 2000 × 2000 cm and a 100×100
  terrain is 10000 × 10000 cm (exactly one 100 m chunk).
- **`DrawScale3D` defaults to `(100, 100, 100)` when absent, not
  `(1,1,1)`.** See below.

### Phase gate

Issue #46 Phase 1.3 (Terrain decoder in Rust) is UNBLOCKED at 92% confidence (raised to **96%** after the 2026-09-19 real-data run below closed the property-tag-skip bug and confirmed the trailer layout on a much larger, structurally different sample). Regression fixture: 25 Castle_CellBlock exports × 20×20 patches × 2 triangles = 20,000 triangles for flat terrain.

### Test asset

`../sgw/Stargate Worlds-QA/Working/SGWGame/CookedPC/Maps/Castle_CellBlock/Castle_CellBlock-00000000.umap`
- 25 Terrain exports (A–Y), 24 at 9372 bytes, 1 (T) at 9328 bytes
- Terrain_T difference: NumPatchesX/Y differs OR one fewer Layer entry (same trailer structure confirmed)

### Real-data validation, Castle-000a0002.umap (2026-09-19, `castle.nav` spike, worker `nav-bsp-re`)

Second validation pass, on the *other* Castle map/pipeline (`Maps/Castle/`, not
`Maps/Castle_CellBlock/`) — reveals these are two **different terrain-authoring
conventions**, not one:

- `Castle-000a0002.umap` has **exactly one** `Terrain`-class export (3630,
  522123 bytes) — not "3 Terrain" as an earlier session's fact sheet claimed
  for this tile (see `docs/reverse-engineering/findings/bsp-model-polys-serialize.md`
  for the contradiction note). It has `NumSectionsX=NumSectionsY=5` and
  `NumPatchesX=NumPatchesY=100` (a full 100×100-patch terrain, not 20×20).
  **`NumSectionsX * NumSectionsY = 25` exactly matches this tile's
  `TerrainComponent` export count (25)** — this resolves the "why multiple
  TerrainComponents" question directly: they are spatial/LOD-culling
  partitions of **one** `Terrain` actor's data, driven by `NumSectionsX/Y`,
  not multiple separate terrain actors. `Castle_CellBlock` apparently uses
  the *other* convention instead — 25 separate small (20×20-patch) `Terrain`
  actors, one per grid cell, no `TerrainComponents` subdivision needed. A
  decoder must walk **every** `Terrain`-class export in a chunk regardless
  of count and treat each independently; do not assume a fixed count per
  chunk.
- **Bug found and fixed in the property-tag skip step**: a naive property
  walker that special-cases `BoolProperty` (4-byte inline value, `size==0`)
  correctly, but does NOT special-case `ArrayProperty`'s declared `size` as
  covering its *entire* nested content (ignore the `ue3-package-format.md`
  note about an `ArrayProperty` extra 8-byte inner-type FName tag — that
  extra tag does not apply when reading an array as a raw `size`-byte blob;
  it only matters for a parser that recurses element-by-element. A raw
  byte-skip using the tag's declared `size` field does NOT need it and
  adding it *breaks* alignment). With that fixed, the whole 1620-byte
  property stream (`bIsOverridingLightResolution`, `Layers`(1127B blob),
  `TerrainComponents`(104B blob), `NumSectionsX/Y`, `NumVerticesX/Y`,
  `NumPatchesX/Y`, `AlphaXSize/YSize`, `AlphaMapStyle`, `Tag`, `Location`)
  parsed cleanly to a single outer `None` at byte 1664 — the earlier
  "first None is inside Layers, use the last one" GOTCHA turned out to be
  specific to a *recursive* parser that walks into `Layers`' nested tag
  sub-streams; a flat byte-skip parser (jump by the tag's declared `size`)
  never sees the inner `None`s at all and needs no special-casing.
- Trailer walk was byte-exact: `Heights.Num=10201` (=101×101=`NumVerticesX*Y`
  exactly), `InfoData.Num=10201`, `AlphaXSize/YSize` binary copies both
  matched the property values (404/404), `WeightedTextureMaps.Num=3` (not
  always 1 — this tile has 3 texture layers), each `WTM[i].Num=163216`
  (=404×404=`AlphaXSize*AlphaYSize` exactly, all 3), `WeightMapTextures.Num=0`.
  Consumed 521959 of 522123 bytes; the remaining 164 bytes are the
  lighting-GUID/foliage trailer (not decoded, not needed — same as the
  152-byte trailer on the smaller Castle_CellBlock sample; the size
  difference is expected version/content variance, not a layout error).
- Heights are genuinely non-flat: 10201 samples, 5149 distinct `u16` values,
  range 44226–54199 (not the `0x8000`-centered flat data the
  Castle_CellBlock worked example showed) — real terrain shape, plausible.
- **Could not close the coordinate cross-check** (decoded height vs. a known
  world-8 outdoor point) — no world-8 respawner or spawn-point row in
  `db/resources/` falls inside this chunk's footprint (the 4 world-8
  respawners in `db/resources/Worlds/Seed/respawners.sql` all sit at
  X∈[345,800], Z∈[513,991], outside this chunk's ~X∈[200,300)/Z∈[1000,1100)
  range). `Castle-000a0002.umap` is one of the two *interior* tiles named in
  the campaign's established facts, and its terrain height data may
  represent a basement/ground-cap plane beneath the BSP interior rather
  than a walkable outdoor surface — genuinely unresolved, not just
  unattempted. Needs an outdoor Castle tile + a matching seed coordinate to
  close.

### Real-data validation, Rust decoder (2026-09-19, worker `nav-terrain`)

`crates/upk-objects/src/terrain/` + `crates/navmesh-extractor/src/terrain.rs`.
Trailer layout above confirmed byte-exact on **1744** terrain exports
(1600 Castle_CellBlock + 144 Castle), zero parse failures, plus spot
checks in Harset / Agnos / SGC (600 more, zero failures). Corrections:

- **`DrawScale3D` class default is `(100, 100, 100)`.** 400 of 1600
  Castle_CellBlock terrains and all 144 Castle terrains omit the
  property; the other 1200 write `(100, 100, 200)` explicitly. UE3 only
  serialises a property that differs from the default, so the default
  must be `(100,100,*)` with Z ≠ 200. The Z component is pinned by the
  gate-room/DHD seed point (BW y 55.10): the decoded Castle terrain
  under it is **55.14** at Z=100 and 110.28 at Z=200. The 1200 explicit
  `(100,100,200)` actors are all flat (`0x8000`), so their doubled Z is
  unobservable.
- **`InfoData` visibility is per-QUAD, keyed by the quad's lower-left
  corner vertex** (`ATerrain::IsTerrainQuadVisible`). The last heightmap
  row/column therefore never gates a quad. Lower-left vs any-corner
  policy differs by only 379 quads of 34,327 in Castle_CellBlock, but
  lower-left is the engine's rule.
- `WeightedTextureMaps.Num` is **not** always 1 — Castle tiles ship 3.
- **Zero remainder is not achievable**: every real export has a 92–3304
  byte lighting-GUID/foliage tail after `WeightMapTextures.Num`. The
  decoder records it as `Terrain::lighting_trailer_bytes` and the
  integration tests pin the exact value (152 Castle_CellBlock,
  164 Castle-000a0002) rather than pretending it is decoded.
- Ground truth: decoded Castle_CellBlock terrain = 605,673 m² at BW y 0.0
  over BW x/z ∈ [-400, 400]; the shipped `castle_cellblock.nav`'s
  BW y ≈ 0.2 sheet = 637,283 m² over x/z ∈ [-399.1, 399.2]. The 31,610 m²
  difference is the building footprint that terrain punches out as holes
  and the shipped mesh covers with floor geometry.
- The shipped nav's other two flat sheets (30,499 m² at BW y 94.6,
  22,838 m² at 53.4) are **not terrain** — every Castle_CellBlock terrain
  actor has `Location.Z = 0`. They are upper-storey BSP/StaticMesh floors.

**Why:** Unblocks issue #46 navmesh extraction pipeline — UTerrain binary layout was the blocking unknown at 55% confidence.

**How to apply:** When implementing `terrain.rs`, use a flat byte-skip
property parser (jump by each tag's declared `size`, no `ArrayProperty`
special-casing needed) rather than a recursive one, and walk every
`Terrain`-class export found in a chunk independently — do not assume a
fixed per-chunk count or a fixed patch-grid size (20×20 and 100×100 are
both attested).

**How to apply:** The decoder now exists —
`cimmeria_upk_objects::deserialize_terrain` +
`cimmeria_navmesh_extractor::terrain::collect_terrain_triangles`. Read
those before re-deriving anything here. A LAST-None-scan is **not**
needed: `cimmeria_upk::parse_tagged_properties_with_end` already does the
flat byte-skip and lands on the outer `None` directly. The
AlphaXSize/AlphaYSize binary-copy consume step is real and the decoder
cross-checks the two copies against the tagged-property values as a
drift detector.
