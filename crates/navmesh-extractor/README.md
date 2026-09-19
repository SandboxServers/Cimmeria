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
| 1.2b — coverage report + floor probe + CLI | **shipped** — modules `coverage`, `floor_probe`, binary `extract_map`; see `tests/castle_coverage_and_probe.rs` and [Measured Castle coverage](#measured-castle-coverage) |
| 1.3 — Terrain decoder | follow-up (recipe in `.claude/agent-memory/game-archaeology-specialist/ue3-terrain-serialize.md`) |
| 1.4 — BSP `Model` / `Polys` decoder | follow-up (needs Ghidra trace) — **on the critical path for Castle**, see below |
| 2 — NavBuilder rebuild + Castle_CellBlock acceptance | follow-up |
| 3 — Recast tuning (`cs=0.15`, `ch=0.1`, `agentClimb=0.5`) | follow-up |
| 4 — Roll out to remaining 23 maps | follow-up |
| 5 — Validation (per-map smoke + regression fixture) | follow-up |

## Module layout

- `lib.rs` — public entry points (`extract_map`, `extract_map_with_report`), error type.
- `chunk_id.rs` — chunk filename decoding (`<MapName>-<HEX8>.umap` →
  `(positionX, positionZ)` per `chunk.cpp:21-29`).
- `umap.rs` — wraps `cimmeria_upk::Package` for chunk enumeration.
- `geometry.rs` — `TriangleSoup` accumulator, terrain triangulation
  helper (stub for Phase 1.3).
- `obj.rs` — Wavefront OBJ writer **and** reader, plus the
  `ue3_to_obj` / `obj_to_ue3` axis swap and the CRLF discipline
  NavBuilder's parser requires. `read_obj_as_ue3` is what the floor
  probe uses, so the probe runs against the artifact NavBuilder
  consumes.
- `nav_roundtrip.rs` — Phase 0 XRC `.nav` reader / writer pair.
- `transform.rs` — actor-to-world `ActorTransform` math
  (`Location` + UE3 `Rotator` + `DrawScale` + `DrawScale3D`).
- `staticmesh/` — Phase 1.2 walker. `mod.rs` enumerates
  `StaticMeshActor` exports and builds the soup; `mesh_ref.rs` resolves
  `StaticMeshComponent.StaticMesh` and classifies every failure into a
  `coverage::SkipReason`.
- `coverage.rs` — per-chunk extraction accounting: skip reasons,
  archetype/prefab counts, export-class census, TSV emitters.
- `floor_probe/` — "is there walkable geometry under this world
  point?", run under a family of candidate UE3→BigWorld axis mappings.
  `mod.rs` is the geometry, `report.rs` the TSV and point-file I/O.
- `bin/extract_map.rs` — the `extract_map` CLI (below).

## Command-line usage

```bash
# 1. Extract a whole map. The index cache is built on first use
#    (~45 s over the ~5000 packages in CookedPC) and reloaded after.
#    Positional shorthand, for wrapper scripts:
#      extract_map <cooked-root> <map-name> <out-dir> <index-path>
cargo run -p cimmeria-navmesh-extractor --release --bin extract_map -- \
  extract \
  --cooked-root "/path/to/SGWGame/CookedPC" \
  --map Castle \
  --out  /tmp/castle-obj \
  --index /tmp/package_index.bin \
  [--chunk-filter 000a0002] \
  [--report <TSV>] [--classes <TSV>] [--combined /tmp/whole/castle.obj]

# 2. Floor-probe the OBJs it wrote, under all 48 candidate axis
#    mappings (or a named subset).
cargo run -p cimmeria-navmesh-extractor --release --bin extract_map -- \
  probe \
  --obj-dir /tmp/castle-obj \
  [--mapping all|+Y+Z+X[,+Z+Y+X]] \
  [--points <TSV>] [--report <TSV>] [--detail <TSV>] \
  [--below 1.5] [--above 0.5] [--neighbourhood 5.0]
```

`extract` writes `<chunkid>o.obj` per chunk, `coverage.tsv` (one row
per chunk plus a `TOTAL` row) and `coverage_classes.tsv` (every export
class, with a `collision_risk` flag for the ones we don't decode).
`probe` writes `probe_mappings.tsv` (mapping ranking) and
`probe_points.tsv` (per-point detail).

The whole-map OBJ is **opt-in** via `--combined`, and its path must sit
outside `--out`. NavBuilder's `chunked` mode globs `*.obj` and derives
each file's chunk bounds from a `<hex8>o` stem; one file that doesn't
match leaves those bounds uninitialised, the build dies with "Failed to
create heightfield", nothing is written — and NavBuilder still exits 0.
`extract_map_with_report` refuses such a path rather than let that
happen quietly.

A mapping label reads left to right as "BigWorld x from …, BigWorld y
(up) from …, BigWorld z from …": `+Y+Z+X` means
`bw = (ue.Y, ue.Z, ue.X) / 100`.

A probe points file is
`label<TAB>HIGH|MEDIUM<TAB>x<TAB>y<TAB>z[<TAB>source]`; without
`--points` the built-in Castle set is used
(`floor_probe::castle_probe_points`).

## Library usage

```rust
use cimmeria_navmesh_extractor::{extract_map_with_report, ExtractOptions};
use cimmeria_upk_objects::PackageIndex;

// Build once, reuse across maps. ~45 seconds on a cold cache.
let index = PackageIndex::build("path/to/CookedPC".as_ref())?;
index.save("package_index.bin".as_ref())?;

// Per map:
let coverage = extract_map_with_report(
    "path/to/CookedPC/Maps/Castle".as_ref(),
    "out/castle".as_ref(),
    ExtractOptions {
        index: Some(&index),
        chunk_filter: None,   // e.g. Some("000a0002")
        combined_obj: None,   // if set, must be outside the output dir
    },
)?;
println!("{:?}", coverage.totals());
```

`extract_map` is the same call without the report. Passing `None` for
the index runs in degraded mode — actors are walked and counted, but no
triangles are emitted. Useful in CI when the asset bundle is missing.

## Measured Castle coverage

Measured 2026-09-19 against `CookedPC/Maps/Castle` (144 chunks) with a
full `PackageIndex` (2,821,598 exports across 5,019 packages).
Extraction wall clock **2.9 s**; 144 MB of per-chunk OBJ plus a 150 MB
combined OBJ.

| Metric | Value |
|---|---|
| Chunks processed | 144 |
| Chunks that produced geometry | 62 |
| Exports | 38,304 |
| `StaticMeshActor` exports | 6,430 |
| ...resolved to collision triangles | 5,469 (85.1%) |
| ...skipped, all as `archetype_stub_component` | 961 (14.9%) |
| Triangles emitted | 1,292,291 |

Every skipped actor falls into exactly one bucket: its cooked
`StaticMeshComponent` carries no `StaticMesh` property because the
actor was instanced from a prefab archetype in another package. There
were no index misses, no decode failures and no collision-free meshes.

### Undecoded classes still in the map

| Class | Exports | Chunks |
|---|---|---|
| `TerrainComponent` | 3,600 | 144 |
| `Terrain` | 144 | 144 |
| `ModelComponent` (built BSP surfaces) | 844 | 16 |
| `Polys` | 761 | 144 |
| `Model` | 761 | 144 |
| `BrushComponent` | 617 | 144 |
| `Brush` | 540 | 144 |
| `PrefabInstance` | 863 | 31 |
| `InterpActor` | 14 | 4 |

### Interior floors are BSP, not StaticMesh

The question the coverage numbers exist to answer. Probing a grid over
the floor plane of two rooms a player demonstrably walked (the CLI's
`probe` mode, mapping `+Y+Z+X`, default ±1.5/0.5 window):

| Room | Grid points | With a StaticMesh floor | With any triangle in the column |
|---|---|---|---|
| Interrogation Block (`Castle-000a0002`, y = 66.79, 2-unit grid) | 1,365 | 2 (**0.1%**) | 1,365 (100%) |
| Level-5 Communications (`Castle-00080002`, y = 55.20, 1-unit grid) | 506 | 100 (**19.8%**) | 130 (25.7%) |

The Interrogation Block — the tile the 2026-09-18 playtest walked, and
the one holding the Zuritska cell — has a triangle in every single
column and a walkable surface at the right height in two of them. The
geometry over those columns is the roof and the ceiling; the floor is
simply not in the StaticMesh set.

Of the three HIGH-confidence playtest points, only the Level-5 comms
room has a StaticMesh surface at the player's feet; the Zuritska cell
and the Romney corridor have hundreds of wall triangles within 5 units
and no floor at all.

> Walkability here means what Recast means, which is *not* the
> right-hand normal of the order the extractor emits: NavBuilder's
> `loadOBJ` reverses each face (`mesh.cpp:123-128`), so
> `N_recast.y = -n_ue3.z`. Measuring with the naive sign reports the
> Interrogation Block at 4.3% instead of 0.1% — the difference is
> entirely ceiling triangles, and for a multi-storey interior the wrong
> answer looks perfectly plausible. `floor_probe::recast_up` models the
> reversal; `recast_up_is_the_negation_of_the_emitted_order_normal`
> pins it.

All sixteen chunks carrying `ModelComponent` exports — i.e. BSP
surfaces that were actually built, as opposed to the empty default
`Model`/`Polys` pair every chunk ships — are interior tiles
(`00040009`, `0004000a`, `00050007`, `00060003`, `00070002`–`4`,
`00080002`–`4`, `00090002`–`4`, `000a0002`–`4`). **The BSP decoder
(Phase 1.4) is therefore on the critical path for a usable
`castle.nav`.** A StaticMesh-only build would produce walls and props
floating over a mostly absent floor.

### The PrefabInstance gap

`PrefabInstance` exports do **not** own their `StaticMeshActor`s
through the export table's `Outer` chain — the map-wide
`prefab_outer_actors` count is 0; every actor is outered straight to
`PersistentLevel`. The prefab's actors *are* separately exported and
*are* counted in the 6,430, but each one's cooked
`StaticMeshComponent` is the ~76-byte archetype stub, so its mesh
reference is unrecoverable without opening the archetype's package.

In `Castle-000a0002` the correspondence is exact: 147 `PrefabInstance`
exports, 147 archetype-instanced `StaticMeshActor`s, 147
`archetype_stub_component` skips, 0 resolved. Map-wide it is 863
`PrefabInstance` against 963 archetype actors of which 2 resolve.
Closing this gap means resolving `ExportEntry::archetype` through the
`PackageIndex` and merging the template component's properties —
tractable, and worth roughly 15% more actors.

## NavBuilder interop — settled

Four things about the OBJ hand-off were unknown or wrong until the
`nav-axis` worker measured them end to end against the real
`NavBuilder_d.exe`. They are now pinned by code and tests in this
crate; the NavBuilder-side write-up is
[docs/engine/navmesh-build-pipeline.md](../../docs/engine/navmesh-build-pipeline.md).

1. **Axis order.** The confirmed world mapping is
   `bw = (ue.Y, ue.Z, ue.X) / 100`, matching
   `docs/analysis/castle-rebuild/worknotes/ca05.md` §"Axis/scale
   calibration". Solving NavBuilder's `loadOBJ`
   (`v.x = obj_z/100, v.y = obj_y/100, v.z = obj_x/100`) for that, the
   OBJ must be written as `v <ue.X> <ue.Z> <ue.Y>` — see
   [`obj::ue3_to_obj`]. Emitting raw `(X, Y, Z)` feeds BigWorld's up
   axis from UE3's horizontal Y and produces an empty 72-byte `.nav`
   with `npolys = 0`. This crate's floor probe agrees independently: of
   48 candidate permutation/sign mappings, only `+Y+Z+X` puts Castle's
   geometry near the probe set (9 of 12 points within 5 BigWorld units,
   several within 0.05); `+Z+Y+X` — the raw-UE3 emission — leaves the
   nearest triangle 59–804 units away from every one of them.
2. **CRLF, always.** NavBuilder's face parser loops
   `while (pos < line.length() - 1)` (`mesh.cpp:115`), so an
   LF-terminated `f` line loses its last index whenever that token is a
   single digit. One fixture: 14 polys with CRLF, 6 with LF. The writer
   emits `\r\n` explicitly on every platform.
3. **Winding is verbatim.** `loadOBJ` pushes each face reversed
   (`mesh.cpp:123-128`), so Recast's normal is the negation of the
   emitted order's. Keeping the kDOP list's native index order is what
   makes floors walkable; reversing it makes the roof walkable.
4. **No stray `*.obj` in the chunk directory**, and never an
   `o Terrain_*` group — NavBuilder skips those groups wholesale
   (`mesh.cpp:88-96`). Chunk ids do **not** offset vertices: actor
   `Location` is already world-absolute (`Castle-00060003` decodes to
   `(positionX=3, positionZ=6)` and its `Ca-ThronePillar00` lands at
   BigWorld `(353.42, 38.17, 636.00)` with nothing added).

## Known unknowns

- **`Terrain` binary trailer** — recipe is documented at 92% confidence
  in agent memory but has not been exercised against a real export.
- **`Model` / `Polys` BSP decoder** — confidence ~50%, needs Ghidra
  trace of `UModel::Serialize`. Deferred, but see "Interior floors are
  BSP" above: for Castle this is the blocking item, not a nice-to-have.

## Testing

```bash
cargo test -p cimmeria-navmesh-extractor
```

Integration tests self-skip (printing `SKIPPED` and why) when the
cooked asset bundle, the `PackageIndex` cache, or
`data/spaces/castle_cellblock.nav` is absent — same pattern as
`crates/entity/src/navigation.rs`. Point them at a non-default
checkout with:

```bash
CIMMERIA_COOKED_PC=/path/to/SGWGame/CookedPC \
CIMMERIA_PACKAGE_INDEX=/path/to/package_index.bin \
  cargo test -p cimmeria-navmesh-extractor
```
