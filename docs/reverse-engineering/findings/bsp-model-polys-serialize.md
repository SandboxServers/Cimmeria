# UModel / UPolys / FBspNode / FPoly Serialize — Client Binary Analysis

> **Last updated**: 2026-09-19
> **Source**: SGW.exe Ghidra decompilation, `castle.nav` spike (issue #46), worker `nav-bsp-re`
> **Confidence**: HIGH for the wire layout (byte-exact validated against real `Castle-000a0002.umap` bytes across seven `Model` exports and six `Polys` exports, 108..298460 and 840..45080 bytes respectively). MEDIUM for the semantic *names* of a handful of trailing fields that are structurally located but not decompiled field-by-field (they lie outside the collision-relevant portion of the format and were deliberately not chased further).
> **Issue**: [#46](https://github.com/SandboxServers/Cimmeria/issues/46) — navmesh extraction; unblocks `crates/navmesh-extractor` Phase 1.4 (`Model`/`Polys` BSP decoder)

---

## Summary

Castle interior tiles carry two independent sources of BSP-shaped collision
geometry: the persistent level's own compiled `Model` (world CSG geometry —
almost certainly the floors/walls/ceilings the playtest needs), and one small
`Model` per placed `Brush`/`TriggerVolume` actor (each actor's own standalone
convex shape). **Cooked QA `.umap` packages do NOT strip `UPolys`** — every
`Model`'s `Polys` object reference resolves to a live, fully-populated
`UPolys` export with real `FPoly` data. This was the single most important
open question going in (`crates/navmesh-extractor/README.md` rated it ~50%
confidence, "needs Ghidra trace of `UModel::Serialize`") and it is now
resolved: BSP collision geometry is recoverable either from the
`Nodes`/`Surfs`/`Verts`/`Points` arrays (the BSP-tree face fragments actually
used for rendering/collision) or, redundantly, from `Polys` (the original,
un-split CSG brush polygons) — the two are consistent copies of the same
underlying surfaces. A decoder can use whichever is more convenient; `Nodes`
is what the recipe below uses since it doesn't require re-deriving polygon
clipping.

`UModel::Serialize` (`UModel__vfunc_12_Serialize` @ `0x008eeff0`) and
`UPolys::Serialize` (`UPolys__vfunc_12_Serialize` @ `0x00807de0`) were
decompiled along with every helper they call, and the resulting field-order
hypothesis was validated by hand-walking real bytes from
`Castle-000a0002.umap` with a throwaway Python script (not committed — see
"Tooling" below) until every tested export's byte count landed exactly on
its `serial_size`. `FBspNode`'s `iVertPool`/`iSurf` field offsets (needed to
turn a node into actual triangle geometry) were confirmed by a bounds-check
sweep across all 399 real nodes in the persistent-level `Model`: every single
node's `iVertPool`/`NumVertices` pair stayed within `Verts.Num()` and every
`iSurf` stayed within `Surfs.Num()` — a coincidental universal pass at that
offset pairing is not plausible by chance.

---

## The central question: is `Polys` stripped in cooked packages?

**No.** Confirmed empirically on `Castle-000a0002.umap` (Epic ver 486,
licensee ver **8** — note this diverges from `docs/engine/ue3-package-format.md`'s
stated `licensee_ver = 6`; that document was written against a different
sample package, and both values are apparently in circulation across SGW's
cooked content. The wire *format* is identical either way; only the
version-gate constants in `UModel`/`UPolys::Serialize` depend on Epic ver,
which is 486 in both cases).

| Model export | Owner | `Polys` objref | Resolves to |
|---|---|---|---|
| 405 (108B, builder brush) | ROOT | 884 | export 883, `Polys`, 840B, 6 FPoly elements |
| 406 (108B, per-brush) | Brush export 56 | 885 | export 884, `Polys`, 1160B, 8 elements |
| **443 (298460B, persistent-level world geometry)** | **`PersistentLevel`** | **929** | **export 928, `Polys`, 45080B, 332 FPoly elements** |
| 444 (108B) | `PersistentLevel` | 928 | export 927 |
| 453 (2872B, TriggerVolume's own Model) | TriggerVolume export 3656 | 932 | export 931 |
| 463 (12884B) | TriggerVolume export 3666 | 942 | export 941 |

Every reference resolves to a real, non-empty `Polys` export whose own
`Serialize` byte-walk consumes its `serial_size` exactly. `UPolys::Serialize`
does branch on an archive flag (`param_1[6]`, decompiled as an `if`/`else` in
`UPolys__vfunc_12_Serialize`), but **both branches serialize the full
`FPoly` element stream** — the branch only changes *how the in-memory array
is grown* while loading (bulk-resize vs. incremental grow), not whether the
data exists on disk. Initial suspicion that this flag gated a cook-time
strip was wrong; ruled out by decompiling both branches' target helpers
(`FUN_00807ba0` and the inline fast path) and finding both call the same
per-element serializer (`FPoly_Serialize`, ex-`FUN_008ee3e0`).

---

## `UModel::Serialize` wire layout

Field order confirmed byte-exact against exports 405, 406, 443, 444, 453,
463 of `Castle-000a0002.umap` (sizes 108, 108, 298460, 108, 2872, 12884 —
every one consumed to exactly `serial_size`, zero bytes remaining).

Header: standard non-Actor/non-Component UObject export — 4-byte `NetIndex`
prefix, then the tagged-property stream (terminated by a `None` FName; empty
for every `Model` export sampled).

| Field | Type | Wire size | Notes |
|---|---|---|---|
| `Bounds` | `FBoxSphereBounds` | 28B (7×4B raw) | Origin.xyz, BoxExtent.xyz, SphereRadius — no version gate, no TArray header |
| `Vectors` | `TArray<FVector>` | 4B count + 12B/elem | Direction/normal pool |
| `Points` | `TArray<FVector>` | 4B count + 12B/elem | Position pool — what `FVert.pVertex` indexes into |
| `Nodes` | `TArray<FBspNode>` | 4B count + **68B/elem** | BSP tree; see layout below. `IsLoading`-only fixup: `NodeFlags &= 0x1f` per node (editor legacy compat, not a wire field) |
| *(ArVer > 0x140)* | objref | 4B | Unidentified — always present for SGW (ArVer=486) |
| `Surfs` | `TArray<FBspSurf>` | 4B count + **56B/elem** (ArVer>0x1a0) | See layout below; custom per-element serialize but fixed stride |
| `Verts` | `TArray<FVert>` | 4B count + **24B/elem** | Bulk-copied (no object refs inside `FVert`); canonical `pVertex(4)+iSide(4)+ShadowTexCoord(8)+BackfaceShadowTexCoord(8)` fits the stride but individual sub-fields were not decompiled (MEDIUM — only `pVertex`, first 4 bytes, matters for our decoder and is HIGH by UE3-convention + stride-fit) |
| `NumSharedSides` | `INT` | 4B | Raw |
| `NumZones` | `INT` | 4B | Count into a **fixed** `Zones[64]` array — NOT a TArray; only `NumZones` entries are on the wire |
| `Zones[NumZones]` | `FZoneProperties` | 24B/elem | `ZoneActor` objref(4) + `LastRenderTime` f32(4) + `Connectivity` QWORD(8) + `Visibility` QWORD(8). Not collision-relevant, skip. |
| **`Polys`** | objref | **4B** | **Confirmed present, unconditional, always resolves to a real `Polys` export (see above)** |
| `LeafHulls` | `TArray<INT>` | 4B count + 4B/elem | Convex-hull plane indices |
| `Leaves` | `TArray<FLeaf>` (4B/elem) | 4B count + 4B/elem | `FLeaf` is a single `INT` (`iZone`) in this build |
| `RootOutside` | `UBOOL` | 4B | |
| `Linked` | `UBOOL` | 4B | |
| `PortalNodes` | `TArray<INT>` | 4B count + 4B/elem | |
| *(unidentified)* | `TArray<16B-elem>` | 4B count + 16B/elem | Structurally confirmed (consistent across all 6 samples); semantic identity unconfirmed. Safe to skip-by-count for a collision decoder. |
| *(ArVer >= 0x14d)* | `INT`, "NumUniqueVertices"(?) | 4B | Present for SGW; on the one large sample (export 443) its value (1896) exactly equals the following array's element count — consistent with it being a genuine `NumUniqueVertices` used to size a following per-vertex table |
| *(unidentified)* | `TArray<40B-elem>` | 4B count + 40B/elem | Count == `NumUniqueVertices` in the one large sample checked. Likely a per-unique-vertex lighting/shadow cache. Safe to skip-by-count. |

If `ArVer < 0x14d` (333) **and** the archive is loading, the client takes an
early-return legacy-fixup branch (`FSkeletalMeshObjectCPUSkin__unknown_008eeaf0`
— a misnamed/folded symbol, almost certainly an unrelated function whose code
happened to be identical at link time) and stops before the trailing fields.
Irrelevant for SGW: every sampled package has `ArVer = 486`.

### `FBspNode` (68 bytes / `0x44`)

Only the fields needed to reconstruct collision triangles were pinned down;
the rest is the canonical UE3 SDK layout by gap-fit (not independently
byte-verified) and is marked accordingly.

| Offset | Field | Confidence | Evidence |
|---|---|---|---|
| `+0x00` | `Plane` (`FPlane`, 16B) | MEDIUM | Consistent with a helper call passing the node base pointer into a plane-read routine (`FUN_00827360`), not independently field-verified |
| `+0x10`..`+0x17` | `ZoneMask`(8) (canonical gap-fit only) | LOW-MEDIUM | Gap-fit against `FBspNode`'s canonical UE3 SDK member order; not decompiled field-by-field |
| **`+0x18`** | **`iVertPool`** (`INT`, index into `Verts[]`) | **HIGH** | Bounds-check sweep: 399/399 real nodes in export 443 have `0 <= iVertPool` and `iVertPool + NumVertices <= Verts.Num()` |
| **`+0x1c`** | **`iSurf`** (`INT`, index into `Surfs[]`) | **HIGH** | Same sweep: 399/399 have `0 <= iSurf < Surfs.Num()` |
| `+0x28` | `iChild[3]` (3×`INT`, Front/Back/Plane) | HIGH | Directly confirmed in decompile of `UModel_PointClassify_BspWalk` (ex-`FUN_009a7350`): `*(int*)(node+iVar3*4+0x28)` |
| **`+0x3a`** | **`NumVertices`** (`BYTE`) | **HIGH** | Same walker: `*(char*)(node+0x3a)==0` is the "pure splitter, not a face" test |
| **`+0x3b`** | **`NodeFlags`** (`BYTE`) | **HIGH** | Confirmed twice: the `IsLoading` fixup mask (`&= 0x1f`) in `UModel::Serialize`, and the same walker's `flags & 0x21` check |

In the one large sample walked (`Castle-000a0002.umap` export 443), **all
399 nodes have `NumVertices > 0`** — there are no pure BSP-splitter nodes in
this tile; every node is also a face-carrying leaf. That may not generalize
to other tiles/maps.

### `FBspSurf` (56 bytes / `0x38`, when `ArVer > 0x1a0`/416)

Fully decompiled field-by-field (`FBspSurf_Serialize`, ex-`FUN_008ed970`).

| Offset | Type | Field |
|---|---|---|
| `+0x00` | objref(4) | `Material` |
| `+0x04` | `DWORD`(4) | `PolyFlags` |
| `+0x08` | `INT`(4) | `pBase` (vertex-pool index, plane base point) |
| `+0x0c` | `INT`(4) | `vNormal` (Vectors[] index) |
| `+0x10` | `INT`(4) | `vTextureU` (Vectors[] index) |
| `+0x14` | `INT`(4) | `vTextureV` (Vectors[] index) |
| `+0x18` | `INT`(4) | `iBrushPoly` (index into the owning Brush's `Polys->Element`) |
| `+0x1c` | objref(4) | `Actor` (owning `ABrush*`) |
| `+0x20..0x2c` | `FPlane`(16) | `Plane` (X,Y,Z,W floats) |
| `+0x30` | `FLOAT`(4) | `LightMapScale` |
| `+0x34` | `INT`(4), ArVer>0x1a0 gated | unidentified (defaults to 3 if absent) |

On the one large tile sampled, only **two distinct `PolyFlags` values**
appear across all 399 faces: `0xE00` (294 faces) and `0x200` (105 faces).
Neither sets bit 0 (`PF_Invisible`) or bit 3 (`PF_NotSolid`) under the
canonical UE3 `EPolyFlags` bit assignments, so — for this tile at least —
no faces need to be filtered out on solidity grounds. **The exact bit-to-flag
mapping was not independently re-derived from this binary** (it's assumed
from public UE3 SDK knowledge); a general-purpose decoder should still
implement the `PF_Invisible`/`PF_NotSolid`/`PF_Portal` checks defensively,
but this specific tile's data doesn't exercise them.

### `UPolys::Serialize` / `FPoly` (variable length)

`Polys->Element` is a `TLazyArray<FPoly>`-shaped field. Its header is
**three** raw `i32` values (12 bytes), not the two you'd expect from a plain
`TArray`:

```
Count           i32   -- element count
legacy_max      i32   -- read into a scratch local; this->Max is explicitly
                         restored to its pre-call value immediately after,
                         so this is DISCARDED by the loader (a vestige of an
                         older on-disk TArray<T> format that stored Max too)
legacy_objref   i32   -- read via the archive's object-ref vtable slot, also
                         discarded. Empirically, in every one of 6 samples,
                         this value equals (this export's own 1-based object
                         index + 1) -- almost certainly a cooker/linker
                         artifact rather than a meaningful reference.
```

Then `Count` × `FPoly`, each variable length:

| Offset (relative) | Field | Size |
|---|---|---|
| `+0x00` | `Base` (`FVector`) | 12B |
| `+0x0c` | `Normal` (`FVector`) | 12B |
| `+0x18` | `TextureU` (`FVector`) | 12B |
| `+0x24` | `TextureV` (`FVector`) | 12B |
| `+0x30` | `Vertices` — `i32` count + count×`FVector`(12B) | 4B + 12×N |
| (+var) | `PolyFlags` (`u32`) | 4B |
| (+var) | `Actor` objref (`ABrush*`) | 4B |
| (+var) | `ItemName` (`FName`: index i32 + instance i32) | 8B |
| (+var) | `Material` objref (`UMaterialInterface*`) | 4B |
| (+var) | unidentified `i32` (×2) | 8B |
| — | **`+0x58`-relative gap** | **0B — genuinely not on the wire.** Confirmed by exhaustive byte-walk: including this gap as a serialized 4 bytes misaligns every subsequent `FPoly` element in every multi-element export tested. |
| (+var) | unidentified `i32` | 4B |
| (+var, ArVer>0x1a1 gated) | unidentified `i32` | 4B |

**Total wire size per `FPoly` = `88 + 12×NumVertices`** — *not*
`sizeof(FPoly) = 100 + 12×NumVertices`, because the in-memory C++ struct's
100-byte size (used only by the `CountBytes()` memory-stats call at the top
of `UPolys::Serialize`) includes the 12-byte in-memory `TArray` header for
`Vertices` (replaced on the wire by a 4-byte count) and the un-serialized
`+0x58` gap. Confirmed byte-exact against 6 real `Polys` exports (840B/6
elements up to 45080B/332 elements; every
sample checked had `NumVertices=4` per face — consistent with box/quad-brush
geometry).

---

## Brush classification (`Castle-000a0002.umap`)

| Class | Count | `Model` size range | Relevance |
|---|---|---|---|
| `Model` owned by `PersistentLevel` | **1** (export 443) | 298460B — **399 Nodes, 239 Surfs, 622 Points, 6135 Verts, 332 Polys** | **This is the compiled world CSG geometry** — the floors/walls the navmesh needs |
| `Model` owned by `PersistentLevel`, tiny | 8 more (444, and 8 others of the 69 total) | 108B, empty | Vestigial/builder-brush-adjacent, no geometry |
| `Model` owned by an individual `Brush` actor | 46 | 108..777B | See below |
| `Model` owned by an individual `TriggerVolume` actor | 21 | 108..12884B | Trigger bounds — **excluded per worker rules ("triggers do not [matter for nav]")** |
| `Model` owned by `ROOT` (builder brush) | 1 (export 405) | 108B | Editor-only, no CSG'd content |

> **Measured correction (2026-09-19, worker `nav-bsp-decoder`)**: the
> per-`Brush` row above overstates what is on disk. Across **all 144**
> `Maps/Castle/Castle-*.umap` chunks, *every* `Brush`-owned `Model`
> (225 of them) and *every* root-owned builder-brush `Model` (144) is
> exactly **108 bytes — an empty stub with zero Nodes**. The only
> non-stub `Model`s in the whole map are 16 `Level`-owned ones (i.e.
> only 16 of 144 tiles carry BSP world geometry at all), 50
> `TriggerVolume`-owned and 15 `DynamicTriggerVolume`-owned. For
> `Castle-000a0002.umap` specifically the ownership split is root 1 /
> `Level` 10 (one real, nine stubs) / `Brush` 37 (all stubs) /
> `TriggerVolume` 21 (all real) = 69, which also corrects the "46"
> brush figure above. The practical consequence is the opposite of the
> section's conclusion: the 47 `Brush` actors contribute **no**
> geometry of their own — their shapes were CSG'd into the
> persistent-level `Model`, which is the more ordinary reading of
> `CsgOper = CSG_Active` and removes the need for hypothesis (a). The
> extractor still walks brush-owned `Model`s (via the actor transform,
> with `PrePivot` subtracted first) so non-Castle maps are covered, but
> that path emits nothing on Castle and is therefore untested against
> real non-empty brush data. Note also that this document numbers
> exports 0-based (`443`) while `inspect-export` and the Rust tests
> number them 1-based (`444`).

**No `BlockingVolume` actors exist in this tile** (`Castle-000a0002.umap`) —
searched the full class histogram, zero hits. Every `Volume`-derived class
present is `TriggerVolume` (21), which the worker rules explicitly say does
not matter for nav. Widening the scan to all 144 chunks adds exactly one
more volume class — `DynamicTriggerVolume` (15 across the map) — and
still no `BlockingVolume`. `crates/navmesh-extractor/src/bsp.rs`
excludes both trigger classes, includes `Brush` and `BlockingVolume` by
name, and excludes any *other* class ending in `Volume` conservatively
while reporting it by name so a new one cannot slip in silently.

The 47 `Brush`-class actors all carry `CollisionComponent` pointing at their
own `BrushComponent` (confirmed via tagged-property read on all 47) and
`bHidden = False`, meaning **these are live, visible, collidable pieces of
geometry at runtime** — not CSG source brushes retained only for editor
rebuild. Their `CsgOper` byte property reads `0x00` (`CSG_Active` under the
canonical `ECsgOper` enum) for 46 of 47, and is entirely absent (falls back
to the class default) for the remaining one. `CSG_Active` is nominally
reserved for the level's *one* special "red builder brush," so this is
either (a) evidence that SGW repurposes standalone `Brush` actors as
lightweight placed geometry (cell bars, railings, door frames — simple
shapes authored as brushes instead of static meshes, left un-CSG'd into the
main `Model` and rendered/collided individually via their own small `Model`
+ actor transform, the same pattern already implemented for
`StaticMeshActor` in `crates/navmesh-extractor/src/staticmesh.rs`), or (b) a
cooker quirk where the `CsgOper` property simply isn't meaningful for
already-cooked brushes and got left at its class default. **Open question —
not resolved by this session.**

> **Superseded (2026-09-19, worker `nav-bsp-decoder`)**: this paragraph
> originally concluded "the geometry is real and should be extracted;
> it's additional to, not a duplicate of, the persistent-level
> `Model`." The whole-map decode above disproves that for Castle —
> every one of the 225 `Brush`-owned `Model`s is a 108-byte empty
> stub, so there is no per-brush geometry to add, and the brushes'
> shapes are already inside the persistent-level `Model`. Extracting
> brush-owned `Model`s *as well* on a map where they are non-empty
> would risk duplicating that CSG'd geometry; the extractor walks the
> path for non-Castle maps but it is untested against real non-empty
> brush data. Hypothesis (b) is the reading the measurement supports.

### Persistent-level `Model` triangle/area estimate

Fan-triangulating all 399 `Nodes` (all have `NumVertices > 0` in this tile,
so no BSP-splitter-only nodes to skip) via `Verts[iVertPool..+NumVertices]`
→ `Points[FVert.pVertex]`:

- **1,098 triangles**
- **~57,918 m²** total surface area (walls + floors + ceilings combined —
  not net floor footprint; a 100×100-unit tile's footprint alone is
  10,000 m², so this includes substantial wall/ceiling area on top of floor,
  plausible for a multi-room interior)
- Both observed `PolyFlags` values (`0xE00`, `0x200`) leave `PF_Invisible`
  and `PF_NotSolid` bits clear, so nothing in this tile needed filtering out

This confirms the persistent-level `Model` alone carries substantial,
plausible floor/wall geometry — strong evidence it is indeed the interior
tile's primary collision source.

---

## Rust decoder — shipped in `crates/upk-objects/src/model/`

> **Status (2026-09-19, worker `nav-bsp-decoder`)**: implemented. The
> recipe below is now a description of live code, not a plan.
> [`crates/upk-objects/src/model/`](../../../crates/upk-objects/src/model/)
> holds `deserialize_model` / `deserialize_polys` (`parse/mod.rs`), the
> decoded types plus `Model::triangulate` and `Model::surf_normal`
> (`types.rs`), and byte-exact fixtures (`parse/tests.rs`). The consumer
> is
> [`crates/navmesh-extractor/src/bsp.rs`](../../../crates/navmesh-extractor/src/bsp.rs)
> (`collect_bsp_triangles`). Real-data validation lives in
> `crates/navmesh-extractor/tests/it/bsp_castle_model_decode.rs`, which
> self-skips without the cooked client tree.

1. **Parse `Model` export**: header(4) → tagged-property skip (reuse
   `cimmeria_upk::parse_tagged_properties` machinery) → `Bounds`(28 raw) →
   `Vectors`(TArray, 12B/elem, discard) → `Points`(TArray, 12B/elem, **keep**)
   → `Nodes`(TArray, 68B/elem, **keep**) → conditional 4B field (ArVer>0x140,
   always true for SGW, discard) → `Surfs`(TArray, 56B/elem, **keep** —
   at minimum `PolyFlags`@+4 for solidity filtering) → `Verts`(TArray,
   24B/elem, **keep** — only `pVertex`@+0 matters) → `NumSharedSides`(4,
   discard) → `NumZones`(4) + `Zones[NumZones]`(24B/elem, discard) →
   `Polys` objref(4, discard for the Nodes-based path; or follow it and use
   `UPolys::Element` instead — either source gives equivalent geometry) →
   `LeafHulls`(TArray 4B/elem, discard unless building convex-hull-based
   collision instead of triangle soup) → `Leaves`(TArray 4B/elem, discard)
   → `RootOutside`(4, discard) → `Linked`(4, discard) → `PortalNodes`(TArray
   4B/elem, discard) → skip-by-count the two unidentified trailing TArrays
   (16B/elem, then conditional 4B + 40B/elem) to consume the export exactly
   (useful as a self-check: final position must equal `serial_size`).
2. **Triangulate**: for each `Nodes[i]` with `NumVertices > 0`: read
   `iSurf`, optionally skip if `Surfs[iSurf].PolyFlags` has `PF_Invisible`
   (bit 0) or `PF_NotSolid` (bit 3) set (defensive — not exercised by the
   one tile sampled); read `iVertPool`/`NumVertices`, gather
   `Points[Verts[iVertPool+k].pVertex]` for `k in 0..NumVertices`, fan-
   triangulate `(0, k, k+1)` for `k in 1..NumVertices-1`.
3. **Actor transform**: for the persistent-level `Model` (owner =
   `PersistentLevel`), geometry is already in world space (no actor
   transform to apply — unlike `StaticMeshActor`). For a `Model` owned by an
   individual `Brush`/`TriggerVolume` actor, apply that actor's
   `Location`/`Rotation`/`DrawScale`/`DrawScale3D` exactly as
   `staticmesh.rs` already does for `StaticMeshActor` — same
   `ActorTransform` type, same `transform_from_actor_props` helper, just a
   different actor-class filter (`Brush`, excluding `TriggerVolume` and any
   future `*Volume` classes other than `BlockingVolume`).
4. **Skip criteria**: skip the persistent-level `Model` export entirely if
   `Nodes.Num() == 0` (an empty/placeholder tile). Skip individual `Brush`
   actors whose class is `TriggerVolume` (or any `*Volume` subclass other
   than `BlockingVolume`, none observed in this tile but the worker rules
   call them out as relevant if seen elsewhere).

**Effort estimate (historical)**: the format is now fully specified for the
collision-relevant fields (Task A's stated blocker is resolved). Writing
`model.rs` mirroring `staticmesh.rs`'s structure (a `Model` reader + a
`Node`→triangle walker) is a half-day to one-day task including tests,
comparable in scope to Phase 1.2's `staticmesh.rs`/`transform.rs` pair —
*not* the multi-day "Ghidra trace" the README's Phase 1.4 status implied
before this session, since the trace is done. Building a live-fixture unit
test needs the same pattern already used for `staticmesh_castle_cellblock.rs`
(self-skip when the client tree is absent) but pointed at
`Castle-000a0002.umap` (or `Castle_CellBlock`, whichever ships as the
project's canonical fixture — see `docs/analysis/castle-rebuild/` for
current tile naming). A `Polys`-reader (`polys.rs`) is optional — the
`Nodes` path alone is sufficient and was the one triangulated in this
session's validation.

**Outcome**: the implementation took the `Nodes` path as recommended and
landed a `UPolys` reader anyway as a cross-check (both live in the same
module rather than a separate `polys.rs`). The fixture tile is
`Castle-000a0002.umap`. Every number in this document's
"Persistent-level `Model` triangle/area estimate" reproduced exactly
from Rust: 399 nodes, 622 Points, 6135 Verts, 239 Surfs, **1,098
triangles**, `PolyFlags` split 294 x `0xE00` / 105 x `0x200`, zero
out-of-range node indices, and all 69 `Model` plus all 69 `Polys`
exports consumed byte-exact. One field-order detail is worth recording:
an **empty `Model` is exactly 108 bytes** (4 NetIndex + 8 `None` +
28 Bounds + seventeen 4-byte scalar/count fields), which makes the
108-byte stubs an arithmetic self-check on the whole layout table.

---

## Split-out appendices

Two subjects that grew alongside this finding now live next to it:

- [`terrain-serialize-real-data-validation.md`](terrain-serialize-real-data-validation.md) — the `ATerrain::Serialize` recipe validated against `Castle-000a0002.umap`.
- [`castle-bsp-geometry-location.md`](castle-bsp-geometry-location.md) — which Castle packages actually hold collidable BSP, measured over all 144 chunks.


## Open questions

| Question | Why it matters | What would resolve it |
|---|---|---|
| Semantic identity of the `+0x8c`-relative single objref field (ArVer>0x140 gate) | Currently just skipped; if it's e.g. `LightingLevel` it's irrelevant to collision, but unconfirmed | Decompile the object-ref-consuming archive vtable slot 0x18's callee, or find a reference to this field elsewhere in the binary (e.g. lighting-build code) |
| Semantic identity of the two unidentified trailing `TArray`s (16B-elem, 40B-elem) | Currently skip-by-count only; harmless for collision but leaves the format description incomplete | Search `UnModelLight.cpp`/`UnModelRender.cpp`-anchored functions for consumers of these offsets (the string anchors exist — `0x0190f91c`/`0x0190ef48` — but their sole xrefs are `FUN_00486000` assert calls, not useful) |
| Exact `PolyFlags` bit semantics in this SGW build | Assumed from public UE3 SDK knowledge, not independently re-derived here. **Still open.** The decoder keeps the assumption in two named tables in `crates/upk-objects/src/model/types/mod.rs`: `REPORTED_POLY_FLAGS` (every bit whose per-flag triangle count is reported, whether or not it is filtered) and `NON_COLLIDING_POLY_FLAGS` (the subset the default filter actually drops), so a wrong bit shows up as an implausible drop count rather than as deleted geometry. `PF_Invisible` sits in the first table only — it controls visibility, not collision, and an invisible *solid* surface still blocks. On the Castle map the filter currently excludes **zero** triangles. | Find a function that branches on specific `PolyFlags` bits (e.g. render-time visibility culling) and decompile it |
| Why 46/47 `Brush` actors read `CsgOper = CSG_Active` (0) | Determines whether these are "real" placed geometry (as this session concludes) or a cooking/versioning artifact | Cross-check against a non-cooked/editor-build package if one becomes available, or find the runtime code path that reads `Brush.CsgOper` post-cook (if any exists, it's dead code — the geometry pattern strongly suggests these ARE used, whatever the property says) |
| ~~Does `crates/navmesh-extractor`'s canonical fixture tile use `Castle_CellBlock` or `Castle-000a0002`?~~ | **Resolved**: `Castle-000a0002.umap`, per `crates/navmesh-extractor/tests/it/bsp_castle_model_decode.rs`. | — |
| BSP node winding runs *opposite* to UE3's render/collision winding | A fan emitted in node vertex-pool order has its right-hand-rule normal **agreeing** with the authored surface normal (400 agree / 11 disagree over `Castle-000a0002`'s near-horizontal faces), whereas `StaticMesh` kDOP triangles are wound clockwise in UE3's left-handed basis so their right-hand-rule normal is the negation. Emitting BSP fans unreversed puts every floor at the known-walkable height at `n_ue3.z > 0`, which NavBuilder reads as a ceiling, so `bsp.rs` reverses BSP fans (`EMIT_REVERSED`). | Confirmed empirically; a decompile of `FPoly::CalcNormal` / the BSP build path would explain *why* the two conventions differ |

---

## Evidence trail

- `UModel::Serialize` = `UModel__vfunc_12_Serialize` @ `ghidra://SGW.exe@0x008eeff0` (renamed from `UModel__vfunc_12`; plate comment carries the full field table)
- `UPolys::Serialize` = `UPolys__vfunc_12_Serialize` @ `ghidra://SGW.exe@0x00807de0` (renamed from `UPolys__vfunc_12`; plate comment carries the full field table)
- `FPoly` per-element serialize = `FPoly_Serialize` @ `ghidra://SGW.exe@0x008ee3e0` (renamed from `FUN_008ee3e0`)
- `FBspSurf` per-element serialize = `FBspSurf_Serialize` @ `ghidra://SGW.exe@0x008ed970` (renamed from `FUN_008ed970`)
- BSP point-classify walker (confirms `iChild`@+0x28, `NumVertices`@+0x3a, `NodeFlags`@+0x3b) = `UModel_PointClassify_BspWalk` @ `ghidra://SGW.exe@0x009a7350` (renamed from `FUN_009a7350`; references the `.\Src\UnModelCollision.cpp` string anchor at `0x01901f10`)
- `Vectors`/`Points` array helper = `FUN_008f0760` @ `0x008f0760`; `Nodes` array helper = `FUN_008f0e10` @ `0x008f0e10`; `Surfs` array helper = `FUN_008f0a50` @ `0x008f0a50`; `Verts` array helper = `FUN_008f0ec0` @ `0x008f0ec0`; `LeafHulls`/`PortalNodes` array helper = `FUN_008f0d40` @ `0x008f0d40`; `Leaves` array helper = `FUN_008f0f70` @ `0x008f0f70`; 16B-elem trailing array helper = `FUN_00605720` @ `0x00605720`; 40B-elem trailing array helper chain = `FUN_009e9ff0` @ `0x009e9ff0` → `FUN_009eb990` @ `0x009eb990`
- `FPoly.Vertices` sub-array helper = `FUN_00604220` @ `0x00604220`
- Generic raw-serialize/byteswap-aware helper = `FUN_0047f0e0` @ `0x0047f0e0` (used throughout; equivalent to `FArchive::Serialize(void*, INT)`)
- Source-path string anchors: `.\Src\UnModel.cpp` (×5, `0x018f01d4` etc.), `.\Src\UnBsp.cpp` (×8, `0x01995e10` etc.), `.\Src\UnModelCollision.cpp` (`0x01901f10`), `.\Src\UnModelRender.cpp` (`0x0190ef48`), `.\Src\UnModelLight.cpp` (`0x0190f91c`/`0x0190f968`)
- Real-data validation: `Castle-000a0002.umap` at
  `Stargate Worlds-QA/Working/SGWGame/CookedPC/Maps/Castle/Castle-000a0002.umap`
  (read-only client tree), exports 405/406/443/444/453/463 (`Model`) and
  883/884/928/929/932/942 (`Polys`), all consumed byte-exact by the
  validation script described under "Tooling" below.

## Tooling

Two throwaway Python scripts (`walk_model.py`, `walk_polys.py`) were written
in the worker's scratch directory
(`C:\Users\Steve\AppData\Local\Temp\cimmeria-castle\navmesh\nav-bsp-re\`)
to hand-validate the layout above against real package bytes. They import
`tools/upk_parser.py` for header/name/import/export-table parsing and LZO
decompression, then walk the `Model`/`Polys` serial data field-by-field,
asserting the final read position equals `serial_size`. **Not committed** —
they're single-purpose validation scripts, not reusable pipeline code (the
real decoder belongs in `crates/upk-objects` per the recipe above, following
`crates/upk-objects/src/static_mesh/parse/mod.rs`'s pattern, not a Python
port of these scripts).

## 2009-vs-2026 notes

The original UE3 licensee fork's `UModel`/`UPolys::Serialize` are
hand-written native C++ (not UnrealHeaderTool-generated), which is why field
order doesn't track C++ declaration order (the 64-slot `Zones` array sits
early in memory layout but late in serialize order, etc.) — normal for
engine-internal classes of this vintage. Cimmeria's decoder doesn't need to
reproduce this quirk; it only needs to reproduce the *wire* order, which
this document specifies directly.
