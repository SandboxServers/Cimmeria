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
| 1.2 — StaticMesh instancing | **shipped** — modules `transform`, `staticmesh`; see `tests/staticmesh_castle_cellblock.rs` (actor-walk) and `tests/extract_map_castle_cellblock.rs` (full `extract_map` OBJ output) |
| 1.2b — coverage report + floor probe + CLI | **shipped** — modules `coverage`, `floor_probe`, binary `extract_map`; see `tests/castle_coverage_and_probe.rs` and [Measured Castle coverage](#measured-castle-coverage) |
| 1.2c — prefab archetypes + `bCollideActors` | **shipped** — module `staticmesh/archetype`, binary `archetype_census`; see `tests/archetype_castle.rs` and [Prefab archetypes](#prefab-archetypes) |
| 1.3 — Terrain decoder | **shipped** — module `terrain`, wired into `extract_map`; see `tests/terrain_castle.rs` and [Phase 1.3 — Terrain](#phase-13--terrain) |
| 1.4 — BSP `Model` / `Polys` decoder | **shipped** — `cimmeria_upk_objects::model` + module `bsp`, wired into `extract_map`; see `tests/bsp_castle_model_decode.rs`, `tests/bsp_castle_floor_evidence.rs`, `tests/bsp_castle_hull_cap.rs` and [Phase 1.4 — BSP](#phase-14--bsp) |
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
- `nav_components.rs` — `.nav` connectivity: flood fill, per-component
  stats, and `locate_within`, the tolerance-first probe resolver.
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
  scan over every actor. See [Prefab archetypes](#prefab-archetypes).
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
on disk by `tests/extract_map_castle_cellblock.rs`.
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

## Measured Castle coverage

Measured 2026-09-19 against `CookedPC/Maps/Castle` (144 chunks) with a
full `PackageIndex` (2,821,598 exports across 5,019 packages).

The actor table below is the **`StaticMeshActor` path only**. It is not
the map's triangle budget: Terrain and BSP contribute separately and
outweigh it by 2:1. The per-source split is the second table.

| `StaticMeshActor` accounting | Before 1.2c | After 1.2c |
|---|---|---|
| `StaticMeshActor` exports | 6,430 | 6,430 |
| ...mesh reference recovered | 5,469 (85.1%) | 6,430 (100%) |
| ...of those, via the prefab archetype chain | 0 | 961 |
| ...emitted as triangles | 5,469 (85.1%) | 4,860 (75.6%) |
| ...suppressed, `bCollideActors = false` | 0 | 1,570 (24.4%) |
| ...unresolved | 961 `archetype_stub_component` | 0 |

Both changes land in 1.2c and they pull in opposite directions: the
archetype chain *adds* 961 actors, and the collision gate *removes*
1,570 (961 - 374 archetype ones that are non-colliding, plus 1,196
chunk-local actors that were always being emitted wrongly). Net, 609
fewer actors reach the soup and the map is more correct for it — see
[`bCollideActors`](#bcollideactors--the-1570-actors-that-should-never-have-been-there).

| Triangles by source | Before 1.2c | After 1.2c |
|---|---|---|
| `StaticMeshActor` | 1,292,291 | 1,254,597 |
| `Terrain` | 2,878,890 | 2,878,890 |
| BSP `Model` | 6,810 | 6,810 |
| **Total in the OBJs** | **4,177,991** | **4,140,297** |

Extraction wall clock ~3 s either way; 33 prefab packages opened across
the map, 86 distinct archetype paths behind 961 stub actors.

Whole-map `.nav`, both built with
`NavBuilder chunked <dir> <out> nav partition=watershed agentHeight=1.8`
`agentClimb=0.6 minRegionSize=24 maxSimplificationError=2.5`:

| | Before 1.2c | After 1.2c |
|---|---|---|
| verts / polys / adjacency edges | 45,209 / 21,805 / 60,926 | 40,093 / 19,824 / 55,132 |
| connected components | 997 | 553 |
| walkable XZ area | 997,253 m² | 814,909 m² |
| bounds x | -124.62 … 1212.86 | -15.66 … 1200.00 |
| probe result | 11/11 ok, 3 components | 11/11 ok, 3 components |

The 11 probes stay in the same three groups (interior / exterior /
throne room) and three of them get *tighter*: `stargate` 0.45 m → 0.00 m,
`armory` 1.49 m → 0.00 m, `checkpoint_bravo` dy +0.62 → +0.22. The
edge count is well inside NavBuilder's 65,535 cap, with more headroom
than before.

### Prefab archetypes

`PrefabInstance` exports do **not** own their `StaticMeshActor`s through
the export table's `Outer` chain — the map-wide `prefab_outer_actors`
count is 0; every actor is outered straight to `PersistentLevel`. What
marks them is the export table's `Archetype` field, and each actor has
*two* chains: the component's (which carries `StaticMesh`) and the
actor's (which carries `bCollideActors` and any rotation/scale the
instance omits). Following the actor's for the mesh is a dead end — the
template actor has `CollisionComponent` and no `StaticMeshComponent`.
The byte-level write-up is in
[docs/engine/ue3-package-format.md](../../docs/engine/ue3-package-format.md#prefab-archetypes--the-cooked-staticmeshcomponent-stub).

In `Castle-000a0002` the correspondence is exact: 147 `PrefabInstance`
exports, 147 archetype-instanced `StaticMeshActor`s, 147 stubs — all 147
now resolve, 125 to geometry and 22 to a collision veto.

`Castle_CellBlock` is **not** archetype-free, contrary to an earlier
note: 2,098 `StaticMeshActor` exports split 1,699 direct / 399
archetype-instanced (19.0%), of which 216 emit geometry.

What the 961 Castle stubs turn out to be — the question the census
exists to answer — is **decorative clutter and cover, no traversal
geometry**. Ranked by instances, of 46 distinct meshes that survive the
collision gate:

| Instances | Mesh | Tris each | Footprint m² | Walkable m² |
|---|---|---|---|---|
| 73 | `Em-Props:EM-ComputerTower00` | 12 | 25 | 13 |
| 50 | `EM-Cover:EM-Cover_Concrete_High_I03` | 138 | 224 | 89 |
| 44 | `Em-Props:EM-ViewScreen02` | 142 | 26 | 4 |
| 38 | `Em-Props:EM-LockerMed00` | 192 | 146 | 59 |
| 36 | `CA-Arch:CA-Cell_Doorway01` | 124 | 611 | 296 |
| 36 | `Em-Props:EM-ViewScreen03` | 136 | 17 | 5 |
| 27 | `EM-Cover:EM-Cover_Concrete_Med_I03` | 138 | 121 | 48 |
| 27 | `Em-Props:EM-StandingLightFrost04` | 472 | 183 | 54 |
| 23 | `EM_Earth_Military:EM-OutdoorHeater00` | 666 | 87 | 40 |
| 23 | `CA-Interior:CA-hallway_decor_torch_01` | 180 | 105 | 45 |
| 20 | `Em-Props:EM-StandingLightFrost01` | 1,968 | 232 | 83 |
| 1 | `EM-Buildings:EM-Bunker_Frost00` | 2,748 | 2,838 | 1,054 |

205,490 triangles over 587 emitted instances. Searching the full set for
`*Stair*`, `*Ramp*`, `*Floor*`, `*Bridge*`, `*Catwalk*`, `*Step*`,
`*Platform*` returns **nothing**. The only names that could plausibly
affect traversal are:

- `CA-Arch:CA-Cell_Doorway01` — 36 instances, all at BigWorld y 66.791,
  i.e. exactly the Interrogation Block floor plane the playtest walked
  (`zuritska_cell` is y 66.79). These are the cell-door thresholds:
  296 m² of walkable surface at the player's feet in the one room the
  README's floor probe measured at 0.1% StaticMesh coverage.
- `Em-Props:EM-Elevator00` — 3 instances at BW (762.22, 29.76, 418.88),
  (930.81, 24.57, 440.21), (591.14, 21.09, 570.85). Static shells, not
  movers; the lift *car* is not in the StaticMesh set.
- `EM-Buildings:EM-Bunker_Frost00` and `EM-GuardHouse00` — building
  shells that were simply absent before.

The missing 15% was therefore **not** the reason Castle's interior has
no floor. That remains BSP (Phase 1.4).

### Where Castle's stairs actually are

`archetype_census` also scans **every** actor, archetype-instanced or
not, for a mesh name matching `elevator`, `lift`, `platform`, `stair`,
`ramp`, `ladder`, `step`, `catwalk`, `bridge`, `walkway`, `door`,
`gate` or `hatch`, and reports it with its merged `bCollideActors`.
The question it exists to answer is whether a hole in the navmesh is
missing geometry or a scripted link.

For Castle the answer is that the vertical circulation was never
missing — it is all in the **direct** set, collision on, and has been
in every OBJ this crate has ever written:

| Mesh | Where (BigWorld) |
|---|---|
| `CA-Props:CA-Stair00` ×3 | (348.64 / 355.04 / 361.44, 46.24, 846.34) |
| `CA-Props:CA-Stair00` ×3 | (348.64 / 355.04 / 361.44, 54.40, 884.32) |
| `CA-Interior:CA-large_hallway_ramp_a_00` | (355.04, 55.04, 863.48) |
| `CA-Props:CA-Stair00`, `CA-small_hallway_ramp_a_00` | (218.80, 59.34, 931.84), (239.68, 68.96, 931.84) |
| `HT-Props:HT-Stair00` ×4 | (296.91, 47.69, 752.50) → (306.13, 41.93, 761.74) |
| `CA-Props:Ca-ThroneStairs` | (350.93, 36.64, 653.01) |
| `EM-Cover:EM-PlatformRamp_00` ×4 | (335.92 / 374.40, 46.16, 809.92–822.52) |

The two `CA-Stair00` flights bracket the 12.00 m interior storey step
(floors at BigWorld y 43.2 and 55.2) that `nav_inspect --gaps` reports
as unbridgeable — so that gap is a Recast question, not an extraction
one.

The three `Em-Props:EM-Elevator00` shells are archetype-instanced and
each pairs with a direct `EM-Elevator_Pad00` a metre away, at
(588.0, 21.1, 564.3), (768.8, 29.8, 415.7) and (937.3, 24.6, 437.1) —
all on the **exterior** lower level, none at an interior storey
boundary. Three more `EM-Elevator00` in `00040009` have no pad.

### `bCollideActors` — the 1,570 actors that should never have been there

Chasing the archetype chain surfaced a second, larger defect that
predates it. `AActor::bCollideActors` defaults to `true` and the cooker
omits defaults, so the property appears only on actors the level author
made non-colliding. The cook does **not** strip collision from their
`StaticMesh`: the kDOP tree is present, so nothing downstream of the
mesh can tell them apart from a wall.

| Where the flag is set | Actors |
|---|---|
| prefab template (26 of 86 templates), inherited by the instance | 374 |
| the chunk-local actor itself | 1,196 |
| **total suppressed** | **1,570** |

17 of the 26 templates are `bHidden = true, Group = PrecipPlanes` —
flat cards placed in tent, bunker and guardhouse **doorways** so snow
renders there. The 1,196 direct ones are icicles, floor signs, wall
panels, hoses, pipes, security cameras, supply crates and wall lights.

Emitting them is not cosmetic. Measured: with the archetype chain on and
the gate *off*, Castle's exterior splits from one walkable component
into three and the `gate_room_dhd` probe loses its floor entirely
(nearest polygon 4.79 m away). With the gate on, the exterior is one
component again and every probe is inside tolerance.

The gate applies to direct actors as well as prefab ones, which is why
the emitted-actor count *falls* from 5,469 to 4,860 even though 961 more
actors now resolve.

Sanity check against the one shipped reference navmesh: rebuilding
`Castle_CellBlock` gives 2,338 verts / 1,283 polys / 17 components /
674,708 m² walkable, against `data/spaces/castle_cellblock.nav`'s
2,778 / 1,479 / 50 / 712,506 m². Within 5% on area with a third of the
fragments — and the shipped mesh was built for a 0.6 m agent, so some of
the difference is the 1.8 m agent culling low spaces, not lost geometry.

### Class census and `collision_risk`

`coverage_classes.tsv`'s `decoded` column is three-valued
(`coverage::DecodeStatus`): `yes` for a class a walker enumerates
directly, `via-owner` for one whose geometry reaches the soup through
another export, `no` for one nothing reads. `collision_risk` is the
intersection of "could carry collision" and `no`, so it shrinks as
phases land instead of freezing at the Phase 1.2 answer. As of 1.4 the
risk set is `BrushComponent`, `ModelComponent`, `Polys`, `InterpActor`,
`KActor`, `FracturedStaticMeshActor`, `StaticMeshCollectionActor`.

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
`castle.nav`**, and it has since shipped — see
[Phase 1.4 — BSP](#phase-14--bsp). A StaticMesh-only build produces
walls and props floating over a mostly absent floor: the throne room's
floor is BSP, and removing the BSP plane at BW y 38.08 drops its 1 m
grid coverage from 381/841 points to 27/841.

### Phase 1.3 — Terrain

`terrain::collect_terrain_triangles` walks every `Terrain`-class export
in a chunk. Two authoring conventions are in the SGW data set and both
are handled by walking the exports rather than assuming a layout:

| Map | Terrain actors per chunk | Patch grid | Components |
|---|---|---|---|
| `Castle` | 1 | 100 x 100 | `NumSections` 5 x 5 = 25 `TerrainComponent` |
| `Castle_CellBlock` | 25, on a 2000 cm grid | 20 x 20 | — |

Both are 100 cm per patch. Three things that are easy to get wrong:

- **`DrawScale3D` defaults to `(100, 100, 100)`** for SGW's `ATerrain`
  class, not `(1, 1, 1)`. A cooked terrain that does not override it
  carries no tagged property, so reading the UE3 `AActor` default
  shrinks the map by 100x.
- **Height is `(h - 32768) / 128`**, actor-local, before the transform.
- **Holes** are `TID_Visibility_Off`, keyed by the quad's **lower-left**
  vertex, and are skipped. That is what leaves the building footprints
  open for the interior floors underneath to show through.

Winding is chosen so the UE3 right-hand-rule normal points **down**,
matching the StaticMesh kDOP convention NavBuilder expects (see
[NavBuilder interop](#navbuilder-interop--settled) point 3).

Measured: `Castle_CellBlock`, 1,600 terrain actors -> 1,211,346
triangles -> 605,673 m^2 of surface at BW y 0.0, against 637,283 m^2 at
BW y 0.2 in the shipped `data/spaces/castle_cellblock.nav`. `Castle`,
144 actors -> 2,878,890 triangles, 555 hole quads, 0 parse failures.

### Phase 1.4 — BSP

`bsp::collect_bsp_triangles` walks every `Model` export and classifies
it by the **class of the export that owns it**, not by any actor
property — ownership can't dangle:

| Owner | Treatment |
|---|---|
| `Level` | world space, no transform — the compiled CSG world geometry |
| `Brush`, `BlockingVolume` | actor transform, with `PrePivot` subtracted before scale/rotate |
| `TriggerVolume`, `DynamicTriggerVolume` | excluded — query volumes whose hulls span doorways |
| any other `*Volume` | excluded conservatively and reported by name |
| package root | the editor builder brush; always a 108-byte stub |

Fans are emitted **reversed** relative to the node vertex order — the
node pools agree with the authored surface normal, and NavBuilder wants
UE3's render convention, which is its negation. `bsp::EMIT_REVERSED`
carries the measurement.

Findings on the Castle data set:

- 16 of 144 chunks carry BSP world geometry — exactly the chunks that
  carry `ModelComponent` exports. 7,952 triangles before the hull-cap
  filter, 6,810 after.
- Every `Brush`-owned `Model` in Castle is a 108-byte stub, so the
  actor-transform path emits nothing there. It is implemented and
  unit-tested but untested against real non-empty data.
- The persistent `<Map>.umap` holds **no** BSP world geometry, so a
  chunk walker misses nothing by skipping it.
- `ModelComponent` is render-only. BSP collision is served by `UModel`'s
  own node tree, and no Castle component disables collision for its
  slice (`model_components_do_not_gate_bsp_collision` checks that).

#### The buried hull skin

The interior is carved out of large additive blocks, and CSG leaves
those blocks' outer skin in the BSP: a top plane at BW y 79.36 and a
bottom plane at BW y 26.88 across 13 chunks, 122,801 m^2 and
123,394 m^2 respectively — 68% of the map's near-horizontal BSP area.
The top plane faces up, so Recast rasterises it into 87,709 m^2 of
walkable surface (8.1% of the whole-map mesh) sealed under terrain at
BW y 93-204.

`bsp::hull_cap` drops it, on two conditions that must **both** hold:
the face is on its model's outer Z plane facing outward, **and** the
chunk's own terrain lies above every one of its vertices. The burial
half is load-bearing: the geometric half alone also removes two of the
three large sheets the shipped `castle_cellblock.nav` contains, because
`Castle_CellBlock`'s interior sits above its terrain rather than under
it. The filter is inert unless `BspOptions::terrain_ceiling` is
supplied, and `ExtractOptions::keep_hull_caps` turns it off for
measurement.

Two things the skin is **not** responsible for, both measured:

- It is not why `throne_room` and `opcore` read as missing floors —
  that was `NavGraph::locate` preferring any polygon whose XZ footprint
  covered the probe over one within tolerance beside it. Fixed by
  `locate_within`; both probes then land on their floor with the skin
  still present.
- It does not relieve Recast's polygon budget. Whole-map, 144 chunks:
  45,115 verts / 21,799 polys with the skin, 45,200 / 21,801 without.
  Removing a single huge flat sheet *costs* 85 vertices, because the
  geometry underneath then contours separately.

### The PrefabInstance gap — closed in 1.2c

Map-wide: 863 `PrefabInstance` exports against 963 archetype-instanced
actors, of which 2 used to resolve (their cooked component happened to
carry its own `StaticMesh`). All 963 resolve now. See
[Prefab archetypes](#prefab-archetypes) for what they turned out to be
and [`bCollideActors`](#bcollideactors--the-1570-actors-that-should-never-have-been-there)
for why 374 of them are deliberately not emitted.

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
  every Castle brush `Model` is an empty stub.
- **Non-`StaticMeshActor` classes that own a `StaticMeshComponent`**
  are still dropped by the walker's class filter: 14 `InterpActor`s in
  Castle, which the archetype resolver handles unchanged when pointed
  at them. 11 are security-camera heads, 1 an antenna, 1 a shelf box —
  and 1 is `GLB-Global:GLB-RingTransporter00` at BigWorld
  (466.45, 70.06, 991.55), the only Castle actor that sets
  `bCollideActors` / `bBlockActors` / `bPathColliding` explicitly. A
  mover's cooked `Location` is its editor-time pose, not necessarily
  where it rests at runtime, which is why widening the filter is a
  judgement call rather than an oversight.
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

A skipped test is not a pass, so the walkers are also covered without
any of that, through the synthetic packages in [`test_support`](src/test_support/):

| Layer | Where | Runs in CI |
|---|---|---|
| chain control flow — loops, depth budget, missing package vs missing export, collision veto ordering | `staticmesh/archetype/tests.rs`, against a `fetch` closure | yes |
| chain over real package bytes — import chain ↔ dotted outer path, both property offsets, cross-package mesh keys | `staticmesh/archetype_walk_tests.rs`, against `test_support::prefab_package` | yes |
| coverage arithmetic and TSV shape | `coverage/tests.rs` | yes |
| the cooked SGW shapes themselves | `tests/archetype_castle.rs` | only with the client tree |

`tests/archetype_castle.rs` pins `Castle-000a0002` at 147
archetype-instanced actors → 125 emitted + 22 collision-vetoed, plus
102 direct actors vetoed, and asserts the balance invariant. Both
guards were revert-proved: disabling the `bCollideActors` gate fails
`a_template_with_collision_off_is_skipped_not_emitted` and all three
real-data tests; disabling archetype resolution fails eight of the
eleven package-backed tests and two of the three real-data ones.
