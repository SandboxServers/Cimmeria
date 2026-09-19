# Castle extraction measurements

What `cimmeria-navmesh-extractor` actually recovers from `Castle` and
`Castle_CellBlock`, per source and per export class, and the
investigations behind each number. Split out of
[../../crates/navmesh-extractor/README.md](../../crates/navmesh-extractor/README.md),
which keeps the pipeline overview, the phase table and the CLI.

Where the resulting navmesh's *connectivity* goes, and why, is
[castle-navmesh-connectivity.md](castle-navmesh-connectivity.md).

Measured 2026-09-19 against `CookedPC/Maps/Castle` (144 chunks) with a
full `PackageIndex` (2,821,598 exports across 5,019 packages).

## Coverage

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
1,570 (374 archetype ones that are non-colliding, plus 1,196 chunk-local
actors that were always being emitted wrongly). Net, 609 fewer actors
reach the soup and the map is more correct for it — see
[`bCollideActors`](#bcollideactors--the-1570-actors-that-should-never-have-been-there).

| Triangles by source | Before 1.2c | After 1.2c |
|---|---|---|
| `StaticMeshActor` | 1,292,291 | 1,254,597 |
| `Terrain` | 2,878,890 | 2,878,890 |
| BSP `Model` | 6,810 | 6,810 |
| **Total in the OBJs** | **4,177,991** | **4,140,297** |

Extraction wall clock ~3 s either way; 33 prefab packages opened across
the map, 86 distinct archetype paths behind 961 stub actors.

### Whole-map `.nav`

All builds below use
`NavBuilder chunked <dir> <out> nav partition=watershed agentHeight=1.8`
`agentClimb=0.6 minRegionSize=24 maxSimplificationError=2.5`, so the only
variable is what the extractor put in the OBJs.

| | Before 1.2c | After 1.2c |
|---|---|---|
| verts / polys / adjacency edges | 45,209 / 21,805 / 60,926 | 40,093 / 19,824 / 55,132 |
| connected components | 997 | 553 |
| walkable XZ area | 997,253 m² | 814,909 m² |
| bounds x | -124.62 … 1212.86 | -15.66 … 1200.00 |
| probe result | 11/11 ok, 3 groups | 11/11 ok, 3 groups |

Both columns predate the mirrored-instance fix. The **current baseline**,
same parameters, is **40,068 / 19,815 / 55,101** over **549** components,
with the eleven probes in **two** groups rather than three — see
[castle-navmesh-connectivity.md](castle-navmesh-connectivity.md) §2. The
edge count is well inside NavBuilder's 65,535 cap either way.

Three probes got *tighter* at 1.2c: `stargate` 0.45 m → 0.00 m, `armory`
1.49 m → 0.00 m, `checkpoint_bravo` dy +0.62 → +0.22.

## Prefab archetypes

`PrefabInstance` exports do **not** own their `StaticMeshActor`s through
the export table's `Outer` chain — the map-wide `prefab_outer_actors`
count is 0; every actor is outered straight to `PersistentLevel`. What
marks them is the export table's `Archetype` field, and each actor has
*two* chains: the component's (which carries `StaticMesh`) and the
actor's (which carries `bCollideActors` and any rotation/scale the
instance omits). Following the actor's for the mesh is a dead end — the
template actor has `CollisionComponent` and no `StaticMeshComponent`.
The byte-level write-up is in
[ue3-package-format.md](ue3-package-format.md#prefab-archetypes--the-cooked-staticmeshcomponent-stub).

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
  floor probe measured at 0.1% StaticMesh coverage.
- `Em-Props:EM-Elevator00` — 3 instances at BW (762.22, 29.76, 418.88),
  (930.81, 24.57, 440.21), (591.14, 21.09, 570.85). Static shells, not
  movers; the lift *car* is not in the StaticMesh set. Each pairs with a
  direct `EM-Elevator_Pad00` a metre away, at (588.0, 21.1, 564.3),
  (768.8, 29.8, 415.7) and (937.3, 24.6, 437.1) — all on the exterior
  lower level. Three more `EM-Elevator00` in `00040009` have no pad.
- `EM-Buildings:EM-Bunker_Frost00` and `EM-GuardHouse00` — building
  shells that were simply absent before.

The missing 15% was therefore **not** the reason Castle's interior has
no floor. That is BSP (Phase 1.4), and where the vertical circulation
turned out to be is
[castle-navmesh-connectivity.md](castle-navmesh-connectivity.md) §2.

## `bCollideActors` — the 1,570 actors that should never have been there

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

## Class census and `collision_risk`

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

## Interior floors are BSP, not StaticMesh

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
`castle.nav`**, and it has since shipped. A StaticMesh-only build
produces walls and props floating over a mostly absent floor: the throne
room's floor is BSP, and removing the BSP plane at BW y 38.08 drops its
1 m grid coverage from 381/841 points to 27/841.

## Phase 1.3 — Terrain

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
matching the StaticMesh kDOP convention NavBuilder expects.

Measured: `Castle_CellBlock`, 1,600 terrain actors → 1,211,346
triangles → 605,673 m² of surface at BW y 0.0, against 637,283 m² at
BW y 0.2 in the shipped `data/spaces/castle_cellblock.nav`. `Castle`,
144 actors → 2,878,890 triangles, 555 hole quads, 0 parse failures.

## Phase 1.4 — BSP

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
- Every `Brush`-owned `Model` in Castle is a 108-byte stub, and that is
  **correct cooked data, not a decoder gap** — see
  [castle-navmesh-connectivity.md](castle-navmesh-connectivity.md) §3.
  The actor-transform path is therefore implemented and unit-tested but
  has no real non-empty data to run against.
- The persistent `<Map>.umap` holds **no** BSP world geometry, so a
  chunk walker misses nothing by skipping it.
- `ModelComponent` is render-only. BSP collision is served by `UModel`'s
  own node tree, and no Castle component disables collision for its
  slice (`model_components_do_not_gate_bsp_collision` checks that).

### The buried hull skin

The interior is carved out of large additive blocks, and CSG leaves
those blocks' outer skin in the BSP: a top plane at BW y 79.36 and a
bottom plane at BW y 26.88 across 13 chunks, 122,801 m² and
123,394 m² respectively — 68% of the map's near-horizontal BSP area.
The top plane faces up, so Recast rasterises it into 87,709 m² of
walkable surface (8.1% of the whole-map mesh) sealed under terrain at
BW y 93–204.

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
- It does not relieve Recast's polygon budget. Whole-map, 144 chunks,
  on the **pre-1.2c** extraction (`bCollideActors = false` actors still
  emitted): 45,115 verts / 21,799 polys with the skin, 45,200 / 21,801
  without. Removing a single huge flat sheet *costs* 85 vertices,
  because the geometry underneath then contours separately.

## The PrefabInstance gap — closed in 1.2c

Map-wide: 863 `PrefabInstance` exports against 963 archetype-instanced
actors, of which 2 used to resolve (their cooked component happened to
carry its own `StaticMesh`). All 963 resolve now.

## Cross-references

- [castle-navmesh-connectivity.md](castle-navmesh-connectivity.md) —
  where the probes land, and what used to split them
- [navmesh-build-pipeline.md](navmesh-build-pipeline.md) — the pipeline
  and the axis convention
- [navbuilder-recast-limits.md](navbuilder-recast-limits.md) — the
  Recast index limits and the Castle parameter table
- [ue3-package-format.md](ue3-package-format.md) — the `.umap` container
- [../../crates/navmesh-extractor/README.md](../../crates/navmesh-extractor/README.md)
  — extractor phases, module layout and CLI
