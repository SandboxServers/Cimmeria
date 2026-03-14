---
name: SGW v486 StaticMesh Binary Format
description: Complete binary layout of UStaticMesh serialized data in SGW cooked packages, verified from AN-Antenna00 hex dump
type: reference
---

# SGW v486 StaticMesh Binary Format

Verified from hex dump analysis of AN-Props.upk export 1334 (AN-Antenna00, 19836 bytes).

## Top-level layout (after tagged properties)

1. **FBoxSphereBounds** (28 bytes): `origin[3f32] + extent[3f32] + radius[f32]`
2. **BodySetup** (4 bytes): `i32` object reference (matches BodySetup tagged property)
3. **kDOPTree**: collision acceleration structure
   - Node count (`i32`) + `count * 32` bytes of kDOPNode
   - Triangle count (`i32`) + `count * 8` bytes of kDOPCollisionTriangle
4. **InternalVersion** (`i32`): typically 15 for SGW
5. **LODModels count** (`i32`)
6. Per LOD: see below

## kDOPNode (32 bytes)
```
min_x: f32, min_y: f32, min_z: f32  (bounding volume min)
max_x: f32, max_y: f32, max_z: f32  (bounding volume max)
child0: u32                          (left child or leaf start)
child1: u32                          (right child or leaf count)
```

## kDOPCollisionTriangle (8 bytes)
```
v1: u16, v2: u16, v3: u16  (vertex indices)
material: u16               (material index)
```

## Per-LOD layout

1. **RawTriangles**: `FUntypedBulkData` v486 header (16 bytes). Always empty in cooked packages (flags=0, count=0, size=0).
2. **Elements** array: `count(i32)` + `count * 28` bytes (7 x i32 per element)
3. **VertexBuffer**: `stride(i32) + num_vertices(i32) + bulk_count(i32) + bulk_count * stride` bytes inline
4. **IndexBuffer**: `num_vertices(i32) + index_count(i32) + index_count * 2` bytes (u16 indices)
5. **Edges**: `header(i32, always 0) + count(i32) + count * 16` bytes (FMeshEdge)
6. **Trailing data**: ShadowTriangleDoubleSided count + misc (not parsed, not needed for rendering)

## FStaticMeshElement (28 bytes = 7 x i32)

Verified: 2-, 3-, and 4-section meshes all parse correctly with 7 fields.
SGW v486 omits `bEnableShadowCasting` and `MaterialIndex` from stock UE3's 9-field layout.

```
material_ref: i32          (package object reference, negative = import)
enable_collision: i32      (UBOOL)
old_enable_collision: i32  (UBOOL)
first_index: i32
num_triangles: i32
min_vertex_index: i32
max_vertex_index: i32
```

## Vertex Format (40 bytes, full-precision UVs)
```
+0:  position     3 x f32 (12 bytes)
+12: tangent_w    f32 (bitangent sign: 0.0 or 1.0)
+16: tangent_x    FPackedNormal (4 bytes: XYZW as u8, maps [0,255] to [-1,+1])
+20: tangent_y    FPackedNormal (4 bytes)
+24: tangent_z    FPackedNormal (4 bytes) -- this IS the vertex normal
+28: color        u32 RGBA (4 bytes)
+32: uv           2 x f32 (8 bytes)
```

This is a COMBINED vertex buffer -- position + normals + UV are interleaved, not in separate buffers as in some UE3 versions. The stride=40 field before vertex data confirms this.

## Double-sided meshes

AN-Antenna00 has 348 vertices but MaxVertexIndex=173 (index buffer references 0..173).
The first 174 vertices have tangent_w=0.0, the second 174 have tangent_w=1.0.
This appears to be UE3's double-sided geometry: front faces + back faces with flipped normals.

## Key cross-references
- FUntypedBulkData v486: 16-byte header (flags:u32 + count:i32 + size:i32 + offset:i32)
- FPackedNormal unpack: `component = byte / 127.5 - 1.0`
- FMeshEdge (16 bytes): `vertex0:i32, vertex1:i32, face0:i32, face1:i32`
