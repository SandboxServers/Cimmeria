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
| 0 — `.nav` round-trip smoke | **shipped** — see `tests/it/nav_roundtrip_castle_cellblock.rs` |
| 1.1 — crate scaffold | **shipped** — modules `chunk_id`, `geometry`, `obj`, `umap`, `nav_roundtrip` |
| 1.2 — StaticMesh instancing | **shipped** — modules `transform`, `staticmesh`; see `tests/it/staticmesh_castle_cellblock.rs` (actor-walk) and `tests/it/extract_map_castle_cellblock.rs` (full `extract_map` OBJ output) |
| 1.2b — coverage report + floor probe + CLI | **shipped** — modules `coverage`, `floor_probe`, binary `extract_map`; see `tests/it/castle_coverage_and_probe.rs` |
| 1.2c — prefab archetypes + `bCollideActors` | **shipped** — module `staticmesh/archetype`, binary `archetype_census`; see `tests/it/archetype_castle.rs` |
| 1.3 — Terrain decoder | **shipped** — module `terrain`, wired into `extract_map`; see `tests/it/terrain_castle.rs` |
| 1.4 — BSP `Model` / `Polys` decoder | **shipped** — `cimmeria_upk_objects::model` + module `bsp`, wired into `extract_map`; see `tests/it/bsp_castle_model_decode.rs`, `tests/it/bsp_castle_floor_evidence.rs`, `tests/it/bsp_castle_hull_cap.rs` |
| 2 — NavBuilder rebuild + Castle_CellBlock acceptance | follow-up |
| 3 — Recast tuning (`cs=0.15`, `ch=0.1`, `agentClimb=0.5`) | follow-up |
| 4 — Roll out to remaining 23 maps | follow-up |
| 5 — Validation (per-map smoke + regression fixture) | follow-up |

## Module layout

- `lib.rs` — public entry points (`extract_map`, `extract_map_with_report`), error type.
- `chunk_id.rs` — chunk filename decoding (`<MapName>-<HEX8>.umap` →
  `(positionX, positionZ)` per `chunk.cpp:21-29`).
- `umap.rs` — wraps `cimmeria_upk::Package` for chunk enumeration.
- `geometry.rs` — `TriangleSoup` accumulator plus `triangles_in`, which
  reads a face range back out so a later extraction phase can see what
  an earlier one pushed (the BSP hull-cap filter needs the terrain).
- `obj.rs` — Wavefront OBJ writer **and** reader, plus the
  `ue3_to_obj` / `obj_to_ue3` axis swap and the CRLF discipline
  NavBuilder's parser requires. `read_obj_as_ue3` is what the floor
  probe uses, so the probe runs against the artifact NavBuilder
  consumes.
- `nav_roundtrip.rs` — Phase 0 XRC `.nav` reader / writer pair.
- `nav_tiled.rs` — the tiled `XRCT` `.nav` reader / writer, and
  `NavFile`, which reads either layout.
- `transform.rs` — actor-to-world `ActorTransform` math
  (`Location` + UE3 `Rotator` + `DrawScale` + `DrawScale3D`).
- `staticmesh/` — Phase 1.2 walker. `mod.rs` enumerates
  `StaticMeshActor` exports and builds the soup; `mesh_ref.rs` resolves
  `StaticMeshComponent.StaticMesh` and classifies every failure into a
  `coverage::SkipReason`; `archetype/` follows a prefab-instanced
  actor's two archetype chains (`chain.rs` is the pure walk, `mod.rs`
  the package I/O around it).
- `terrain.rs` — Phase 1.3 `Terrain` walker; see below.
- `bsp/` — Phase 1.4. `mod.rs` classifies and places every `Model`
  export and emits its collision triangles; `hull_cap.rs` holds the
  buried-outer-skin filter and the `TerrainCeiling` it needs.
- `nav_components/` — `.nav` connectivity: flood fill, per-component
  stats, and `locate_within`, the tolerance-first probe resolver.
  `tiled.rs` builds the same graph from a tiled file, linking tile
  portals with Detour's own test.
- `coverage/` — per-chunk extraction accounting: skip reasons,
  per-source triangle tallies, archetype/prefab counts, export-class
  census with a per-class `DecodeStatus`, TSV emitters.
- `floor_probe/` — "is there walkable geometry under this world
  point?", run under a family of candidate UE3→BigWorld axis mappings.
  `mod.rs` is the geometry, `report.rs` the TSV and point-file I/O.
- `bin/extract_map/` — the `extract_map` CLI (below): `args.rs`,
  `extract_mode.rs`, `probe_mode.rs`.
- `bin/nav_inspect.rs` — `.nav` acceptance gate: component table plus
  named probe points. NavBuilder exits 0 even when it writes nothing,
  so "the command succeeded" proves nothing.
- `bin/archetype_census.rs` — what the prefab-archetype set actually
  *is*, per mesh: instance count, triangles, world footprint and how
  much of it Recast would accept as floor, plus a traversal-keyword
  scan over every actor. `geometry.rs` is the area arithmetic, `census.rs`
  the walk, `report.rs` the formatting.
- `cover/` — world-space cover nodes (`SGWSpecCoverNode` actors and
  `StaticMeshActor.CoverNodeArray` components): `walk.rs` decodes one
  package, `grouping.rs` forms cover sets, `sql.rs` renders the seeds.
  `bin/cover_extract.rs` is the CLI; see
  [docs/engine/cover-extraction.md](../../docs/engine/cover-extraction.md).
- `occluder/` — inputs for the `cimmeria-occluder` line-of-sight grid
  (NA27): `mod.rs` walks a map's chunks in memory and hands each chunk's
  triangles over in BigWorld metres, split into terrain and StaticMesh
  plus BSP. `exact.rs` is the exact segment tracer used as ground truth.
  `sweep.rs` samples navmesh point pairs and scores the occluder and a
  navmesh ray against the tracer. `bin/occluder_extract/` is the CLI:
  `build` writes the shipped paged `.occ`, trimmed to the explorable area
  (`explorable.rs`: the navmesh components holding an entry point),
  `measure` reports size, RAM, build time and
  accuracy per cell size, and `probe` prints one segment's verdict.
  Results: [the NA27 worknote](../../docs/analysis/npc-ai-restoration/worknotes/na27-occluder-phase1.md).
- `bin/obj_slab.rs` — column / free-run / level-histogram / slope
  queries over the chunk OBJs. `--levels` is the "is there a staircase
  between these two storeys" question.
- `test_support/` — synthetic UE3 package fixtures behind the
  `test-support` feature, so the walkers have CI coverage without the
  cooked client tree. `prefab_fixtures.rs` writes the prefab-template
  `.upk` and the cooked instance that resolves against it.

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

`coverage.tsv` breaks the triangle count down by source —
`staticmesh_triangles`, `terrain_triangles`, `terrain_quads_holed`,
`terrain_parse_failures`, `bsp_triangles`, `bsp_hull_cap_triangles`,
`bsp_models_failed` — under the invariant

```text
triangles == staticmesh_triangles + terrain_triangles + bsp_triangles
          == the OBJ's `f` line count
```

reported per row as `sources_balanced` and asserted against the bytes
on disk by `tests/it/extract_map_castle_cellblock.rs`.
`bsp_hull_cap_triangles` counts faces the hull-cap filter *removed*, so
it is deliberately outside the sum.
`probe` writes `probe_mappings.tsv` (mapping ranking) and
`probe_points.tsv` (per-point detail).

```bash
# 3. What is in the prefab-archetype set, per mesh.
CIMMERIA_PACKAGE_INDEX=/tmp/package_index.bin \
cargo run -p cimmeria-navmesh-extractor --release --bin archetype_census -- \
  "/path/to/SGWGame/CookedPC" Castle \
  [--positions <TSV>] [--meshes <TSV>]
```

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
        // `ExtractOptions` grows knobs (`skip_terrain`, `skip_bsp`,
        // `keep_hull_caps`); take their defaults rather than pinning
        // a field list that a later phase will break.
        ..Default::default()
    },
)?;
println!("{:?}", coverage.totals());
```

`extract_map` is the same call without the report. Passing `None` for
the index runs in degraded mode — actors are walked and counted, but no
triangles are emitted. Useful in CI when the asset bundle is missing.

## Measured results

The measurements this crate produced against the SGW data set -- coverage,
the prefab-archetype census, `bCollideActors`, the class census, terrain and
BSP findings -- live in
[docs/engine/castle-extraction-measurements.md](../../docs/engine/castle-extraction-measurements.md).

Where the resulting navmesh's components end up, and the mirrored-instance
bug that used to split them, is
[docs/engine/castle-navmesh-connectivity.md](../../docs/engine/castle-navmesh-connectivity.md).

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

- **`EPolyFlags` bit meanings** are taken from the public UE3 SDK and
  were *not* re-derived from this build's binary. The triangulator
  therefore reports a per-bit exclusion breakdown alongside its output
  (`BspTriangulation::excluded_by_flag`), so a wrong mapping shows up as
  an implausible drop count instead of a silent hole in the navmesh.
  Change `NON_COLLIDING_POLY_FLAGS`, not the triangulator.
- **`FBspNode::NodeFlags`** is decoded and histogrammed but nothing is
  excluded on it. `NF_NotCsg` is the obvious candidate and is the wrong
  one: every node of a brush-local `Model` is outside the level's CSG
  set, so excluding it would delete all per-`Brush` geometry.
- **Actor-placed BSP** (`Brush` / `BlockingVolume` with a non-empty
  `Model`) is implemented but has no real data to validate against —
  every Castle brush `Model` decodes to a 108-byte stub, and that is
  **correct cooked data, not a decoder gap**: the export table records
  `serial_size = 108`, and the bytes are bounds + zero counts + a live
  `Polys` reference. 36 of the 38 brushes in `Castle-00080003` are
  `CSG_Subtract`, so their shape is already in the level `Model`. See
  [castle-navmesh-connectivity.md](../../docs/engine/castle-navmesh-connectivity.md)
  §3. The path stays untested against real non-empty data until a map
  turns up that has some.
- **Non-`StaticMeshActor` classes that own a `StaticMeshComponent`** —
  **NA36 (2026-09-25) widened the walker, and then made the risky part
  opt-in.** `staticmesh::MESH_ACTOR_CLASSES` now includes `KActor` and
  `FracturedStaticMeshActor` alongside `StaticMeshActor`,
  unconditionally — both have zero exports across all 23 shipped maps
  today, but are true `AStaticMeshActor` siblings and cost nothing to
  support. `InterpActor` is different: `OPT_IN_MESH_ACTOR_CLASSES`
  gates it behind `ExtractOptions::include_interp_actors` /
  `--include-interp-actors`, **default off**. The first NA36 pass
  walked `InterpActor` unconditionally and a same-day follow-up walked
  it back after a reviewer pointed out the failure mode: **a mover's
  cooked `Location` is its editor-time pose, not necessarily where it
  rests at runtime** — a closed door baked into a `.nav` seals the
  doorway, and baked into a `.occ` blocks sight through an opening a
  player can actually see through. A 23-map classification of every
  resolved `InterpActor`'s mesh name
  (`docs/engine/navmesh-build-pipeline.md` §11) found the risk is real
  but a minority: 754 `InterpActor`s resolve a mesh across all 23 maps,
  of which ~51 (7%) are literal doors or a Stargate's rotating chevron
  mechanism, ~124 (16%) are security-camera heads (ambiguous — the
  mount is static, only the head plausibly rotates), and the remaining
  ~579 (77%) are load-bearing static-shaped props: 332
  `GLB-RingTransporter00` ring-transport platforms alone (players stand
  on these; Castle's own instance previously had **zero** collision
  geometry at all), plus streetlamps, floating lights, antennas and
  parked vehicles. Harset's 31 `InterpActor`s were checked by hand and
  are all in the safe category, so `harset.nav`/`harset.occ` ship built
  with the flag on; every other map defaults to off until someone does
  the same per-map check (see the flag-usage table in
  [navmesh-build-pipeline.md §11](../../docs/engine/navmesh-build-pipeline.md#11-mesh-actor-class-gap-interpactor--kactor--fracturedstaticmeshactor-na36-2026-09-25)).
- **`Polys` is never read.** BSP collision comes from `UModel`'s node
  tree, so this is believed correct rather than known correct.
- **Terrain coordinate cross-check** — the height-vs-known-outdoor-point
  check is still open; no matching seed point was found in the interior
  tile sampled.

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

`tests/it/navbuilder_axis_roundtrip.rs` additionally takes
`CIMMERIA_NAVBUILDER` (default: the `bin64/NavBuilder_d.exe` reference
binary found by walking up from the crate). One case in it asserts the
parameter validation in `deprecated/cpp/src/nav_builder/build_params.hpp`
and therefore only means anything against a binary built from *this*
tree, which no probe can detect — say so explicitly:

```bash
tools/build-navbuilder.ps1 -Out $TMP/NavBuilder.exe
CIMMERIA_NAVBUILDER=$TMP/NavBuilder.exe CIMMERIA_NAVBUILDER_FROM_TREE=1 \
  cargo test -p cimmeria-navmesh-extractor --test it navbuilder_axis_roundtrip
```

A skipped test is not a pass, so the walkers are also covered without
any of that, through the synthetic packages in [`test_support`](src/test_support/):

| Layer | Where | Runs in CI |
|---|---|---|
| chain control flow — loops, depth budget, missing package vs missing export, collision veto ordering | `staticmesh/archetype/tests.rs`, against a `fetch` closure | yes |
| chain over real package bytes — import chain ↔ dotted outer path, both property offsets, cross-package mesh keys | `staticmesh/archetype_walk_tests.rs`, against `test_support::prefab_package` | yes |
| coverage arithmetic and TSV shape | `coverage/tests.rs` | yes |
| the cooked SGW shapes themselves | `tests/it/archetype_castle.rs` | only with the client tree |

`tests/it/archetype_castle.rs` pins `Castle-000a0002` at 147
archetype-instanced actors → 125 emitted + 22 collision-vetoed, plus
102 direct actors vetoed, and asserts the balance invariant. Both
guards were revert-proved: disabling the `bCollideActors` gate fails
`a_template_with_collision_off_is_skipped_not_emitted` and all three
real-data tests; disabling archetype resolution fails eight of the
eleven package-backed tests and two of the three real-data ones.
