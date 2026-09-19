---
name: ue3-staticmesh-extraction
description: UE3 StaticMeshActor → StaticMeshComponent → StaticMesh resolution gotchas in SGW cooked packages
metadata:
  type: feedback
---

# UE3 StaticMesh extraction in SGW cooked .umap chunks

Confirmed against Castle_CellBlock chunk fffefffd (469 StaticMeshActors)
while implementing Phase 1.2 of the navmesh-extractor.

## Tagged-property offset by class

Different UE3 object classes have different binary header sizes before
the tagged-property block starts:

| Class kind | Offset to tagged props | Notes |
|---|---|---|
| AActor subclasses (`StaticMeshActor`, `Brush`, `Terrain`, ...) | **32** | 32-byte cooked Actor header |
| `StaticMesh` (the asset) | **4** | 4-byte NetIndex prefix |
| `StaticMeshComponent` and other Components | **8** | NetIndex + 4-byte component-specific prefix |

The 4-vs-8 byte difference between StaticMesh and StaticMeshComponent
is **the trap**: `upk-objects/static_mesh.rs` uses offset 4 (correct
for StaticMesh-as-asset); naively reusing that for Component parsing
returns 0 properties and the walker emits 0 instances. Verify with a
diagnostic that probes offsets {0, 4, 8, 16} and pick the first one
that yields the expected `StaticMesh` ObjectProperty.

## Cross-package resolution

`StaticMeshActor.StaticMeshComponent` → positive export index of a
local `StaticMeshComponent` export.
`StaticMeshComponent.StaticMesh` → negative import index when the mesh
lives in another `.upk` (the common case for SGW).

To recover the mesh's home package, walk the import's `package_index`
chain back to the root (where `package_index == 0`); the root import's
`object_name` is the `.upk` stem (e.g. `CA-Arch`, `Em-Props`).

Build a `PackageIndex` from `crates/upk-objects/src/package_index.rs`
to map `(package_name, object_name)` → `ExportLocation`. Cost: ~50
seconds on a cold cache for SGW's ~5000 packages. Cache via
`PackageIndex::save` / `::load` to `package_index.bin` (the binary's
default output name).

## Archetype-based actors — SHIPPED, and the obvious recipe is wrong

15% of Castle and 19% of Castle_CellBlock `StaticMeshActor` exports
DON'T have a direct `StaticMesh` ref on their cooked component — they
inherit it from a prefab archetype. Symptoms:

- Actor's `archetype` field is a negative import (e.g. `-462`).
- Actor's component is a stub carrying only per-instance overrides
  (`CullDistance`, `CachedCullDistance`, `IrrelevantLights`).
- Walking the archetype chain lands at a `Prefab` import in a content
  package (e.g. `Em-Props.upk:EM-WallLight02_Pf0`).

Implemented in `staticmesh/archetype/`; all 961 Castle stubs resolve.
Three things this note previously got wrong or omitted:

1. **There are TWO archetype chains and they answer different
   questions.** The *component's* `Archetype` carries `StaticMesh`.
   The *actor's* carries `bCollideActors` / `Rotation` /
   `DrawScale3D`. The recipe "find the template's
   `StaticMeshActor.StaticMeshComponent.StaticMesh`" — what this file
   used to say — **does not work**: the template actor has
   `CollisionComponent` and no `StaticMeshComponent` at all.
2. **Template object names are NOT unique.** Every SGW prefab names
   its component `StaticMeshComponent0`; `Em-Props.upk` holds 218 of
   them (and 278 `RB_BodySetup`). `PackageIndex` keys on
   `(package, object_name)` and cannot disambiguate — match the full
   dotted `Outer` path inside the package instead. The path's first
   component after the package name IS unique (a top-level `Prefab`
   export), which is what lets the name-keyed index still find the
   *file*. No index format change was needed.
3. **`Location` must never be inherited** — a template actor's is its
   offset *inside the prefab* (`(128, -2031.99, 0)`). Rotation/scale
   inherit fine (23 Castle instances inherit `DrawScale3D=(1,1,1.2)`).

## `bCollideActors=false` — render-only geometry WITH real kDOP data

The trap that matters more than the archetype chain itself.
`AActor::bCollideActors` defaults true and the cooker omits defaults,
so the property appears ONLY on non-colliding actors. **The cook does
not strip collision from their `StaticMesh`** — the kDOP tree is
present and populated, so nothing below the mesh layer can tell such
an actor from a wall.

Castle: 26 of 86 prefab templates (inherited by 374 instances) plus
1,196 chunk-local actors = **1,570 actors**. 17 of the 26 templates
are `bHidden=true, Group=PrecipPlanes` — flat weather cards sitting in
tent/bunker/guardhouse **doorways**. The rest are icicles, floor signs,
wall panels, hoses, pipes, cameras, crates, wall lights.

Emitting them split Castle's exterior navmesh from one walkable
component into three and left the Stargate DHD with no floor. Honour
the flag on direct actors too, not just prefab ones — most of them are
direct. Byte-level detail:
`docs/engine/ue3-package-format.md` §"Prefab archetypes".

## kDOP collision triangles

`StaticMesh` has a `kDOPTree` after the bounds + body-setup fields:

```
nodes_count: i32
nodes: count * 32 bytes (6 floats bbox + 2 u32 children)
tris_count: i32
tris: count * 8 bytes (3 u16 vertex indices + 1 u16 material)
```

The triangle vertex indices reference the **LOD0 vertex buffer's
position array**. For SGW cooked meshes the kDOP triangles ARE the
collision representation; LOD0 indices are the render triangulation
(usually equivalent but heavier). Phase 1.2 prefers kDOP, falls back
to LOD0 when the kDOP array is empty.

## Chunk filename → world position

`<MapName>-<HEX8>.umap`. The 8 hex digits unpack as:
- Low u16 = signed i16 = `positionX_` in NavBuilder (UE3 Y axis)
- High u16 = signed i16 = `positionZ_` in NavBuilder (UE3 X axis)

One patch = 100 BW units = 10,000 UE3 cm along each horizontal axis.

**Important**: SGW actor `Location` fields are ALREADY in world space.
The chunk filename tells NavBuilder which patch the OBJ describes but
does NOT contribute a translation offset during geometry extraction.
Don't double-translate.

## OBJ axis convention (deferred verification)

NavBuilder's `Mesh::loadOBJ` swizzles on read: `v.x = obj_z/100,
v.y = obj_y/100, v.z = obj_x/100`. The extractor emits raw UE3 cm and
trusts NavBuilder to swizzle — but the cube round-trip described in
the deep dive (emit known cube, run NavBuilder, compare bmin/bmax) has
not been run as of Phase 1.2. Open the OBJ in Blender before kicking
off Phase 2 NavBuilder.

## Master .umap files

SGW map directories contain both `<Name>-<HEX8>.umap` chunks AND a
`<Name>.umap` master file without hex suffix. The master is a
streaming-level placeholder; orchestrators that iterate `*.umap` must
filter by the chunk-naming pattern or they'll trip on the master file.
