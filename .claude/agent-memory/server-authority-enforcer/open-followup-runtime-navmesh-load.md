---
name: open-followup-runtime-navmesh-load
description: NavMesh::load in cimmeria-entity has the same unguarded count*stride pattern that PR #426 fixed in the build-time tool — worth a follow-up issue
metadata:
  type: project
---

**Open follow-up** (not blocking any current PR): `crates/entity/src/navigation.rs::NavMesh::load` is the runtime navmesh loader and is the production read path for `.nav` files at server startup / space load. It has the EXACT same unguarded patterns that PR #426 hardened in the build-time tool:

```rust
let mut verts = vec![0u16; (nverts * 3) as usize];
let mut polys = vec![0u16; (npolys * nvp * 2) as usize];
let mut detail_meshes = vec![0u32; (detail_nmeshes * 4) as usize];
let mut detail_verts = vec![0.0f32; (detail_nverts * 3) as usize];
let mut detail_tris = vec![0u8; (detail_ntris * 4) as usize];
```

**Why:** PR #426 only hardened the build-time tool's parser; the runtime loader was pre-existing and out of scope.

**How to apply:** When opening a follow-up issue, point at [pattern-checked-alloc-size.md](pattern-checked-alloc-size.md) as the canonical helper shape to port over. The threat model for the runtime loader is "operator with file write access to `data/spaces/`", which is a higher trust boundary than the build-time tool but still not free — a server that crashes / OOMs on a malformed .nav at startup is a denial-of-service vector if an attacker can compromise the asset deploy pipeline.

**Note on duplication:** Porting the helpers means either (a) duplicating them in cimmeria-entity, (b) extracting them into cimmeria-common, or (c) having cimmeria-entity depend on cimmeria-navmesh-extractor's helper crate (cyclic — extractor depends on entity? probably not). Option (b) is the cleanest — the helpers are domain-agnostic.
