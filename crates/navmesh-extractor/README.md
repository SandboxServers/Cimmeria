# cimmeria-navmesh-extractor

Extracts UE3 `.umap` chunk geometry to Wavefront `.obj` files for the
C++ NavBuilder Recast pipeline. The output `.nav` files are loaded at
runtime by `crates/entity/src/navigation.rs`.

## Pipeline

```text
SGW cooked maps (UE3 .umap)
        │
        ▼
cimmeria-upk / cimmeria-upk-objects    ← header, name/import/export tables,
        │                                tagged-property parsing, StaticMesh
        ▼                                LODs + kDOP collision
this crate (navmesh-extractor)
        │     emits  data/navmesh_inputs/<map>/<XXXXYYYY>o.obj
        ▼
deprecated/cpp/src/nav_builder         ← Recast PolyMesh + DetailMesh,
        │                                XRC writer
        ▼                                (xrcSavePolyMesh)
data/spaces/<map>.nav
        │
        ▼
crates/entity/src/navigation.rs        ← runtime loader, Detour FFI
```

The decision to keep the C++ NavBuilder as a build-time tool — rather
than porting Recast into Rust right now — is captured in the
asset-pipeline navmesh deep dive. The recurrent thread: 80% of the
work is **extracting collision geometry from UE3 chunks**, not
running Recast. Get the geometry pipeline right first; port the
Recast wrapper later if it buys anything.

## Phase status

| Phase | Status |
|---|---|
| 0 — `.nav` round-trip smoke | **shipped** — see `tests/nav_roundtrip_castle_cellblock.rs` |
| 1.1 — crate scaffold | **shipped** — modules `chunk_id`, `geometry`, `obj`, `umap`, `nav_roundtrip` |
| 1.2 — StaticMesh instancing | **shipped** — modules `transform`, `staticmesh`; see `tests/staticmesh_castle_cellblock.rs` (actor-walk) and `tests/extract_map_castle_cellblock.rs` (full `extract_map` OBJ output); archetype-based actors deferred |
| 1.3 — Terrain decoder | follow-up (recipe in `.claude/agent-memory/game-archaeology-specialist/ue3-terrain-serialize.md`) |
| 1.4 — BSP `Model` / `Polys` decoder | follow-up (needs Ghidra trace) |
| 2 — NavBuilder rebuild + Castle_CellBlock acceptance | follow-up |
| 3 — Recast tuning (`cs=0.15`, `ch=0.1`, `agentClimb=0.5`) | follow-up |
| 4 — Roll out to remaining 23 maps | follow-up |
| 5 — Validation (per-map smoke + regression fixture) | follow-up |

## Module layout

- `lib.rs` — public entry point (`extract_map`), error type.
- `chunk_id.rs` — chunk filename decoding (`<MapName>-<HEX8>.umap` →
  `(positionX, positionZ)` per `chunk.cpp:21-29`).
- `umap.rs` — wraps `cimmeria_upk::Package` for chunk enumeration.
- `geometry.rs` — `TriangleSoup` accumulator, terrain triangulation
  helper (stub for Phase 1.3).
- `obj.rs` — Wavefront OBJ writer (NavBuilder-compatible).
- `nav_roundtrip.rs` — Phase 0 XRC `.nav` reader / writer pair.
- `transform.rs` — actor-to-world `ActorTransform` math
  (`Location` + UE3 `Rotator` + `DrawScale` + `DrawScale3D`).
- `staticmesh.rs` — Phase 1.2 walker: enumerates `StaticMeshActor`
  exports, resolves their `StaticMeshComponent.StaticMesh` reference
  via a `PackageIndex`, applies the actor transform, and pushes
  collision triangles into the chunk's soup.

## End-to-end usage

```rust
use cimmeria_navmesh_extractor::extract_map;
use cimmeria_upk_objects::PackageIndex;

// Build once, reuse across maps. ~50 seconds on a cold cache.
let index = PackageIndex::build("path/to/CookedPC".as_ref())?;
index.save("package_index.bin".as_ref())?;

// Per map:
extract_map(
    "path/to/CookedPC/Maps/Castle_CellBlock".as_ref(),
    "out/castle_cellblock".as_ref(),
    Some(&index),
)?;
```

Passing `None` for the index runs in degraded mode — actors are walked
and counted, but no triangles are emitted. Useful in CI when the asset
bundle is missing.

## Known unknowns

- **OBJ axis convention.** NavBuilder's `loadOBJ` does
  `v.x = obj_z/100, v.y = obj_y/100, v.z = obj_x/100`. The intent is
  UE3-Z-up → BW-Y-up + cm → BW-units. The extractor emits OBJ vertices
  as raw UE3 cm and trusts NavBuilder to swizzle, but **the cube
  round-trip described in Phase 0.3 of the deep dive has not been run
  yet** — flagged here for the next implementer.
- **`Terrain` binary trailer** — recipe is documented at 92% confidence
  in agent memory but has not been exercised against a real export.
- **`Model` / `Polys` BSP decoder** — confidence ~50%, needs Ghidra
  trace of `UModel::Serialize`. Deferred.

## Testing

```bash
cargo test -p cimmeria-navmesh-extractor
```

The round-trip integration test self-skips when `data/spaces/castle_cellblock.nav`
is not present (same pattern as `crates/entity/src/navigation.rs` tests).
