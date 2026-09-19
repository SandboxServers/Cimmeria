---
name: ue3-bsp-model-decode
description: UE3 UModel/UPolys decode traps — empty Model is exactly 108 bytes, BSP node winding is OPPOSITE StaticMesh, Castle brush Models are all stubs, ModelComponent is render-only
metadata:
  type: project
---

Learned while implementing `crates/upk-objects/src/model/` (castle.nav
spike, issue #46). Applies to any UE3 `.umap` BSP work.

**Why:** the RE finding
(`docs/reverse-engineering/findings/bsp-model-polys-serialize.md`) was
byte-accurate on the wire layout but wrong about *where the geometry
lives* and silent about winding — either mistake alone ships an
unwalkable navmesh.

**How to apply:** read before touching `Model`, `Polys`, `FBspNode`, or
anything that emits BSP triangles into a triangle soup.

## An empty `Model` is exactly 108 bytes

4 NetIndex + 8 `None` terminator + 28 Bounds + seventeen 4-byte
scalar/count fields. Free arithmetic self-check on the whole field-order
table: if your layout says anything but 108 for a stub, the field list
is wrong. Same trick for `Polys`: 24 bytes (4 + 8 + a **three**-i32
`Element` header, not two).

## BSP node winding is the OPPOSITE of StaticMesh winding

A fan emitted in `Verts[iVertPool..]` order has its right-hand-rule
normal **agreeing** with the authored surface normal
(`Vectors[vNormal]`) — measured 400 agree / 11 disagree over
`Castle-000a0002`'s near-horizontal faces. UE3 render/collision
triangles (what `StaticMesh` kDOP gives you) are wound clockwise in
UE3's left-handed basis, so their right-hand-rule normal is the
*negation* of the surface normal. NavBuilder wants the latter (walkable
when `n_ue3.z < 0`).

So **BSP fans must be reversed** before joining a soup that also holds
StaticMesh triangles (`bsp::EMIT_REVERSED`). Unreversed, all 111 floor
triangles at Castle's known-walkable height came out `n_ue3.z > 0` —
NavBuilder reads the whole interior as ceilings.

Corollary: any "is there a floor here" probe must use the **authored**
surface normal, never a cross product of the emitted triangle, or the
answer silently depends on the winding decision.

## Classify a `Model` by its OWNER export's class, not by a property

`Model.package_index` points straight at the owner and cannot dangle.
`Level` → world space, no transform. `Brush`/`BlockingVolume` → actor
transform with `PrePivot` subtracted *first*
(`FTranslationMatrix(-PrePivot) * Scale * Rotation * Translation`).
`TriggerVolume`/`DynamicTriggerVolume` → exclude; their hulls span
doorways and would seal the navmesh.

## In `Maps/Castle`, brush-owned Models all DECODE to empty — cause unproven

Across all 144 chunks: every one of the 225 `Brush`-owned and 144
root-owned (builder brush) `Model`s comes out of the decoder as a
108-byte stub. Only 16 `Level`-owned Models carry geometry, and those
16 tiles are exactly the ones with `ModelComponent` exports (the
interiors). The finding's claim that the 47 Brush actors are "live
collidable geometry" is wrong, and its "108..777B" size range for them
does not match the data.

**Qualify this before relying on it.** "Decodes to 108 bytes" is
measured; "brush shapes were CSG'd into the level Model" is the
*inference*, and nobody has tested the alternative — that the 108
bytes are a header we mis-parse and there is per-brush geometry we
drop. Two reasons that now matters:

- It is the **leading candidate for Castle's missing interior storey
  connector** (2026-09-19). `nav-connectivity` showed the halls at BW
  y 48.4 and 55.2 do not join at any Recast parameter set down to
  `cs=0.1 ch=0.05 minRegionSize=1`, and the `CA-Stair00` flights only
  span ~5 m each so they are within-storey. `Castle-00080003` — the
  tile that step sits in — has 38 `Brush` against 878 already-emitted
  BSP triangles. 540 `Brush` exports map-wide.
- Everything else is ruled out: `Model` IS decoded (6,810 triangles
  over 16 chunks), and `StaticMeshCollectionActor`, `KActor`,
  `FracturedStaticMeshActor` and `BlockingVolume` have **zero Castle
  exports**.

One `Brush`-owned `Model` export hexdump settles it. If it holds real
geometry, note the build is already at 83% of Recast's unchecked
24-bit `rcCompactCell::index` span cap at `cs=0.3`, so 540 brush hulls
may force `bounds=`-cropped region meshes — see
`docs/engine/navbuilder-recast-limits.md`.

## The persistent `<MapName>.umap` has no BSP

`umap::enumerate_chunks` deliberately skips the master file (no
`-<HEX8>` suffix), which makes it a tempting suspect for missing
geometry. It isn't: both `Castle.umap` and `Castle_CellBlock.umap` hold
three `Model`s (root builder brush, level's own, one TriggerVolume's)
and the first two are stubs. Castle_CellBlock's big flat nav sheets are
in the CHUNKS' level Models.

## `ModelComponent` is render-only

It lists node indices of the level Model (`UnModelRender.cpp`).
Collision goes through `UModel`'s own `iChild[3]` tree
(`UnModelCollision.cpp`) and never consults a component. Verified the
only escape hatch: all 844 Castle ModelComponents declare only
`CachedCullDistance`, `CullDistance`, `bForceDirectLightMap`,
`bAcceptsLights`, `bAcceptsDynamicLights`, `LightingChannels` — no
collision flag at all. Safe to ignore.

## Keep the `PolyFlags` filter as a reported table

The `EPolyFlags` bit meanings are assumed from the public UE3 SDK, not
re-derived from SGW.exe. `Model::triangulate` reports a per-flag
triangle exclusion count for *every* table entry whether or not the
active filter uses that bit, so a wrong assumption surfaces as an
implausible drop count instead of a hole. On Castle the only observed
values are `0xE00` (1,626 nodes) and `0x200` (1,389), and the filter
excludes zero triangles.

Same hazard family as [[bincode-persisted-cache-format]]: a wrong
assumption that decodes without erroring.
