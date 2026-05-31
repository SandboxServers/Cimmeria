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

## Archetype-based actors

About 20% of Castle_CellBlock `StaticMeshActor` exports DON'T have a
direct `StaticMesh` ref on their cooked component — instead they
inherit it from a prefab archetype. Symptoms:

- Actor's `archetype` field is a negative import (e.g. `-462`).
- Actor's component is a 76-byte stub with only `CullDistance`
  override properties.
- Walking the archetype chain through the imports lands at a `Prefab`
  import in a content package (e.g. `Em-Props.upk:EM-WallLight02_Pf0`).

Resolving these properly means opening the prefab package, finding the
template's `StaticMeshActor.StaticMeshComponent.StaticMesh`, and using
THAT as the mesh ref. The current navmesh-extractor walker silently
skips them — deferred Phase 1.2-extension work.

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
