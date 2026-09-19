---
name: umodel-bsp-serialize
description: UModel/UPolys/FBspNode/FBspSurf/FPoly binary serialize layout recovered from SGW.exe for navmesh extraction (#46) — addresses, byte-exact layout, Polys-not-stripped finding
metadata:
  type: project
---

## UModel::Serialize / UPolys::Serialize — SGW.exe RE findings (2026-09-19)

Full finding doc: `docs/reverse-engineering/findings/bsp-model-polys-serialize.md`.
This memory is the short-form pointer + the facts most likely to save a future
session from re-deriving them.

**Primary functions** (renamed in Ghidra, plate comments carry full field tables):

| Symbol | Address | Role |
|---|---|---|
| `UModel__vfunc_12_Serialize` (ex-`UModel__vfunc_12`) | `0x008eeff0` | `UModel::Serialize` |
| `UPolys__vfunc_12_Serialize` (ex-`UPolys__vfunc_12`) | `0x00807de0` | `UPolys::Serialize` |
| `FPoly_Serialize` (ex-`FUN_008ee3e0`) | `0x008ee3e0` | per-`FPoly` element serialize |
| `FBspSurf_Serialize` (ex-`FUN_008ed970`) | `0x008ed970` | per-`FBspSurf` element serialize |
| `UModel_PointClassify_BspWalk` (ex-`FUN_009a7350`) | `0x009a7350` | point-vs-BSP classify; confirms `iChild`/`NumVertices`/`NodeFlags` node offsets |

**The central question is answered: cooked QA `.umap` packages do NOT strip
`Polys`.** Every `Model.Polys` object reference resolves to a real, populated
`UPolys` export. Confirmed on 6 samples in `Castle-000a0002.umap` including the
persistent-level Model's own Polys (332 FPoly elements, 45080 bytes).

**`UModel::Serialize` field order** (byte-exact validated, 7 Model exports
108..298460 bytes, zero remainder every time): `Bounds`(28B raw) →
`Vectors`(TArray FVector 12B/elem) → `Points`(TArray FVector 12B/elem) →
`Nodes`(TArray FBspNode **68B/elem**) → [ArVer>0x140: 4B objref] →
`Surfs`(TArray FBspSurf **56B/elem**) → `Verts`(TArray FVert **24B/elem**) →
`NumSharedSides`(4B) → `NumZones`(4B) + fixed `Zones[NumZones]`(24B/elem, NOT
a TArray) → **`Polys` objref(4B)** → `LeafHulls`(TArray INT 4B/elem) →
`Leaves`(TArray 4B/elem) → `RootOutside`(4B) → `Linked`(4B) →
`PortalNodes`(TArray INT 4B/elem) → unidentified TArray 16B/elem → [ArVer>=0x14d:
4B "NumUniqueVertices"?] → unidentified TArray 40B/elem.

**FBspNode (68B/0x44), node-relative offsets** — confirmed via a 399/399
bounds-check sweep (real Nodes data, every `iVertPool+NumVertices <=
Verts.Num()` and `iSurf < Surfs.Num()`) plus the BSP walker decompile:
`iVertPool`@+0x18 (i32, → Verts[]), `iSurf`@+0x1c (i32, → Surfs[]),
`iChild[3]`@+0x28 (3×i32), `NumVertices`@+0x3a (u8, 0 = pure splitter node),
`NodeFlags`@+0x3b (u8; `&0x1f` IsLoading fixup applied client-side, not a wire
transform).

**FBspSurf (56B, ArVer>0x1a0)**: `Material`objref(4)@0, `PolyFlags`u32(4)@4,
`pBase`(4)@8, `vNormal`(4)@0xc, `vTextureU`(4)@0x10, `vTextureV`(4)@0x14,
`iBrushPoly`(4)@0x18, `Actor`objref(4)@0x1c, `Plane` FPlane(16)@0x20,
`LightMapScale`f32(4)@0x30, unidentified(4)@0x34.

**FPoly / UPolys.Element** is `TLazyArray<FPoly>`-shaped: header = **3** raw
i32 (not 2) — `Count`, then two fields the loader reads into scratch and
DISCARDS (`this->Max` explicitly restored right after; the third field
empirically always equals this export's own 1-based index + 1, a cooker
artifact). Per-`FPoly` wire size = **88 + 12×NumVertices** bytes — NOT
`sizeof(FPoly)=100+12N` (100 is the in-memory C++ size incl. the Vertices
TArray's 12-byte header and a 4-byte `+0x58`-relative gap that is never
serialized at all — confirmed by exhaustive byte-walk, including it breaks
every multi-element export).

**Brush classification, `Castle-000a0002.umap`**: 1 persistent-level Model
(export 443, 399 Nodes / 239 Surfs / 622 Points / 6135 Verts / 332 Polys,
298460B) = the compiled world CSG geometry, all 399 nodes are face-carrying
(no pure splitters), ~1098 triangles / ~57918 sq-m surface area. 47 `Brush`
actors each carry their own small Model (108-777B) and have
`CollisionComponent` pointing at their own `BrushComponent` + `bHidden=False`
— live collidable placed geometry, NOT CSG source brushes (their `CsgOper`
reads `CSG_Active`=0 for 46/47, open question why). 21 `TriggerVolume`
actors — exclude per nav rules. 0 `BlockingVolume` in this tile.

Reference sample: `licensee_ver=8` for `Castle-000a0002.umap`, vs. `licensee_ver=6`
in `docs/engine/ue3-package-format.md`'s sample — both exist in SGW's cooked
content; `file_ver=486` (Epic version, the one that gates `Ar.Ver()` checks
in native Serialize overrides) is consistent across both.

**Validation method**: two throwaway Python scripts
(`walk_model.py`/`walk_polys.py`, NOT committed) using `tools/upk_parser.py`
for header/LZO parsing, hand-walking real export bytes until consumed byte
count == `serial_size` exactly. This is the pattern to reuse for any future
UE3 native-Serialize recovery — decompile first for the field-order
hypothesis, then always close the loop against real bytes; two arithmetic
slip-ups (a wrong `+4` gap-skip, and `sizeof()`≠wire-size confusion between
`100` and `88+12N` for FPoly) were only caught this way.

**Why:** Unblocks issue #46 navmesh extraction pipeline Phase 1.4 — the
`Model`/`Polys` BSP decoder was previously ~50% confidence, "needs Ghidra
trace." That trace + real-data validation is now done; only the Rust
implementation (`model.rs` in `crates/upk-objects`, mirroring
`static_mesh/parse/mod.rs`) remains.

**How to apply:** When implementing `model.rs`, use the field table above
directly. Triangulate via `Nodes` (NumVertices>0) → `Verts[iVertPool+k].pVertex`
→ `Points[...]`, fan-triangulate. Apply actor transform only for `Brush`-
owned Models (not the persistent-level Model, which is already world-space);
exclude `TriggerVolume`/other non-`BlockingVolume` volume classes.
