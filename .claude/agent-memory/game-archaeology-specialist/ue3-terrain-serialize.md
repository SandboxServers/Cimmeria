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
- `DrawScale`, `DrawScale3D` — ABSENT in Castle_CellBlock (default 1.0 / (1,1,1)); must default if absent

**None FName** for Castle_CellBlock = `0x36 0x00 0x00 0x00  0x00 0x00 0x00 0x00` (name index 54).
**GOTCHA**: The `Layers` array contains inner tagged-property sub-blocks each ending in their own None. First None occurrence is inside Layers. Use the LAST None occurrence as the outer terminator. For Terrain_A: inner None at +412, outer None at +805; trailer starts at +813.

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

- Z = Location.Z + (height_u16 / 65535.0) * DrawScale * DrawScale3D.Z * 256.0
- Cell size X = DrawScale * DrawScale3D.X * 256.0 cm per patch
- 20×20 terrain = 5120 cm × 5120 cm

### Phase gate

Issue #46 Phase 1.3 (Terrain decoder in Rust) is UNBLOCKED at 92% confidence. Regression fixture: 25 Castle_CellBlock exports × 20×20 patches × 2 triangles = 20,000 triangles for flat terrain.

### Test asset

`../sgw/Stargate Worlds-QA/Working/SGWGame/CookedPC/Maps/Castle_CellBlock/Castle_CellBlock-00000000.umap`
- 25 Terrain exports (A–Y), 24 at 9372 bytes, 1 (T) at 9328 bytes
- Terrain_T difference: NumPatchesX/Y differs OR one fewer Layer entry (same trailer structure confirmed)

**Why:** Unblocks issue #46 navmesh extraction pipeline — UTerrain binary layout was the blocking unknown at 55% confidence.

**How to apply:** When implementing `terrain.rs` in the navmesh extractor, use this exact sequence; particularly the LAST-None-scan for the outer terminator and the AlphaXSize/AlphaYSize binary-copy consume step.
