# Castle (World 8) navmesh connectivity

Why Castle's named probe points land where they do, what used to split
them, and what is still split. Split out of
[navmesh-build-pipeline.md](navmesh-build-pipeline.md) §7, which is the
reference for the pipeline itself and for the tools quoted here
(`nav_inspect --gaps`, `obj_slab`).

Measured 2026-09-19 against the 144-chunk `Castle` extraction at the
recommended parameter set
(`partition=watershed agentHeight=1.8 agentClimb=0.6 minRegionSize=24`
`maxSimplificationError=2.5`).

## 1. Where the probes sit now

| | Value |
|---|---|
| verts / polys / adjacency edges | 40,068 / 19,815 / 55,101 |
| connected components | 549 |
| probe groups | **2** — exterior, and everything indoors |

The eleven named probes resolve into two groups, not three. `throne_room`
is in the same component as `zuritska_cell`, `romney_corridor`,
`comms_room`, `nid_guard_116`, `opcore` and `armory`; that component is
53,556 m², up from 17,006 m² before the fix in §2. The remaining split is
exterior ↔ interior (§4).

## 2. Resolved: the mirrored hallway ramp

For several rounds of measurement the interior read as **two** storeys
12.00 m apart with nothing between them, and the split survived every
Recast parameter set down to `cs=0.1 ch=0.05 minRegionSize=1`. It was not
a tuning problem and it was not missing geometry. The connector was in
every OBJ this crate has ever written, facing the wrong way.

`CA-Interior:CA-large_hallway_ramp_a_00` at BigWorld
`(355.04, 55.04, 863.48)` is authored with `DrawScale3D = (-1, 1, 1)` — a
mirrored instance. Mirroring reverses triangle winding; NavBuilder decides
walkability from winding alone (`mesh.cpp:123-128`, see
[navmesh-build-pipeline.md](navmesh-build-pipeline.md) §1.3); and the
walker transformed the three vertices and emitted them in their original
order. The ramp's tread therefore reached Recast as a **ceiling**.

`transform::ActorTransform::apply_triangle` now swaps two vertices when
the scale determinant is negative. 198 of Castle's 6,436
`StaticMeshActor`s are mirrored, and 93 of `Castle_CellBlock`'s 2,098;
three of Castle's sit on the stairwell centreline. Regression test:
`transform::tests::a_mirrored_instance_keeps_its_treads_facing_the_same_way`.

With the fix a cropped stairwell build yields one component spanning
y 47.4 → 56.4 across z 836 → 894, and on the whole map `throne_room`
merges into the interior component.

### What the stair spine is

A traversal-keyword scan over all 6,436 `StaticMeshActor`s
(`archetype_census`) finds the vertical circulation in the **direct** set,
collision on:

| Mesh | Where (BigWorld) |
|---|---|
| `CA-Props:CA-Stair00` ×3 | (348.64 / 355.04 / 361.44, 46.24, 846.34) |
| `CA-Props:CA-Stair00` ×3 | (348.64 / 355.04 / 361.44, 54.40, 884.32) |
| `CA-Interior:CA-large_hallway_ramp_a_00` | (355.04, 55.04, 863.48) |
| `CA-Props:CA-Stair00`, `CA-small_hallway_ramp_a_00` | (218.80, 59.34, 931.84), (239.68, 68.96, 931.84) |
| `HT-Props:HT-Stair00` ×4 | (296.91, 47.69, 752.50) → (306.13, 41.93, 761.74) |
| `CA-Props:Ca-ThroneStairs` | (350.93, 36.64, 653.01) |
| `EM-Cover:EM-PlatformRamp_00` ×4 | (335.92 / 374.40, 46.16, 809.92–822.52) |

Each flight spans about five metres of rise — the lower climbs
46.0 → 51.5, the upper 54.5 → 59.5 — while the halls sit at 48.4 and 55.2.
The ramp is what bridges the 51.5 → 54.5 band, which is exactly why losing
it to a winding flip cost the whole storey connection.

### A gap-finder bug found on the way

The gap finder originally pointed 30 m west of the stair spine, because
its hop cost weighted only the horizontal gap and two floors of one
building overlap in XZ: a storey jump reported `h = 0.00` and scored as
free. `Approach::bridge_size` is now `max(horizontal, |vertical|)`, so a
12 m jump costs 12. Regression test:
`gaps::tests::a_stacked_storey_jump_does_not_beat_a_real_route`.

## 3. Resolved: the 108-byte `Brush`-owned `Model`s are correct data

Every `Brush`-owned `Model` in Castle decodes to a 108-byte stub — 38 in
`Castle-00080003` (the stair tile) alone, 540 map-wide — and that was the
leading suspect for missing per-brush geometry in exactly the tile the
storey step sat in. It is not a decoder gap:

- the export table itself records `serial_size = 108`, so there is nothing
  being truncated on read;
- the 108 bytes are bounds (28) plus all-zero array counts plus a live
  `Polys` reference;
- the brush's box lives in that `Polys` export (840 bytes = 6 quads);
- 36 of the 38 brushes in `Castle-00080003` are `CSG_Subtract` — the
  carved rooms — so their shape is already in the level `Model` as the
  room's walls, floor and ceiling.

Brush 11 in that tile is the stairwell shaft itself: a subtractive box at
BigWorld x[339.7, 370.4] y[47.0, 65.0] z[845.6, 881.3], carved through
both storeys, with doorway brushes at each end (y 47–55 at z 844–846,
y 55.2–65 at z 881–883).

`Castle-00080003` already contributes **878** BSP triangles to the OBJ,
the second-largest of the 16 chunks that carry any (6,810 map-wide).

## 4. Still split: exterior ↔ interior

The gate area and the keep are separated by terrain, not by a door.
`nav_inspect --gaps --gap-h 12 --gap-v 8` finds a three-hop chain of
terrain shelves with 2.4–3.4 m steps:

```text
(725.6, 30.4, 462.9) → (675.6, 18.6, 488.3) → (622.4, 24.0, 508.2)
```

`obj_slab --column 622.7,496…504` measures the bank between them at
**45–58°**. Relaxing to `slope=60` or `slope=70` on a crop covering the
corridor shortens the chain and narrows the worst horizontal gap, but
never joins the two, because the remaining hops are vertical — 7.87 m and
6.81 m in the original measurement. There are **zero** prefab-archetype
actors in `x[600,740] y[15,35] z[450,500]`, so the archetype gap
contributes nothing here either.

This is outdoor ground, so the three coordinates above are where to look
if it is ever worth bridging by hand.

## 5. The armory is a ring drop zone, not a walk-in room

`db/resources/Worlds/Seed/ring_transport_regions.sql` has exactly one row
for world 8: region 34, `Castle_ArmoryRingDropZone`, at
`(466.365, 70.397, 991.466)` — the `armory` probe to three decimal places
— with an empty `destination_region_ids`. The row that targets it is
region 33, `Cellblock_ArmoryRingSwitch`, in **world 12**
(`required_mission_id` 688). The armory is reached by a cross-world ring
transport, and its 11 m² pad sits 1.50 m above the interior floor.

The one `InterpActor` in Castle with collision flags set explicitly
(`bCollideActors` / `bBlockActors` / `bPathColliding`, all true) is
`GLB-RingTransporter00` at `(466.45, 70.06, 991.55)` — the same pad. It is
not extracted, but the floor under it is, so adding it would change
nothing about connectivity.

## 6. Classes that are not the answer

Ruled out, with the evidence, so they do not get re-investigated:

- **`StaticMeshCollectionActor`, `KActor`, `FracturedStaticMeshActor`,
  `BlockingVolume`** — Castle has zero exports of any of them.
- **`InterpActor` movers.** All 14 in Castle resolve to 11
  `EM-SecurityCam01_Top` heads, one `EM-Antenna00`, one `EM-ShelfBox10`
  and one `GLB-RingTransporter00`. There is no lift, elevator or door
  among them. The nine `EM-Elevator00` / `EM-Elevator_Pad00` instances in
  Castle *are* `StaticMeshActor`s, already extracted, and all sit at
  y 20–30 on the exterior level.
- **Prefab-archetype `StaticMeshActor`s.** The 33 actors resolved inside
  `x[250,320] y[40,60] z[850,900]` are all set dressing — computer
  towers, view screens, torches, a locker, a wall light, waist-high
  concrete cover. Four cover blocks sit at y = 43.20 and y = 55.40, which
  independently confirms both storeys are real and populated.

## 7. What would close the remaining gap

1. **Decide whether the exterior ↔ interior split is real.** The three
   shelf coordinates in §4 are outdoor terrain; walking the route in the
   client settles whether a player is meant to climb there at all.
2. **Accept that they are separate**, and give the cell a per-region
   navmesh or an off-mesh link table. Both need server-side loader work.
   The `EM-Elevator00` + `EM-Elevator_Pad00` pairs at `(588.0, 21.1,
   564.3)` and `(768.8, 29.8, 415.7)` are the natural anchors on the
   exterior side.

Recast tuning is not on this list, and neither is extracting
`InterpActor`s.

## Cross-references

- [navmesh-build-pipeline.md](navmesh-build-pipeline.md) — the pipeline,
  the axis convention, and the `nav_inspect` / `obj_slab` tooling
- [navbuilder-recast-limits.md](navbuilder-recast-limits.md) — rebuilding
  NavBuilder, Recast's index limits, the Castle parameter table
- [castle-extraction-measurements.md](castle-extraction-measurements.md) —
  what the extractor recovers from Castle, per source and per class
- [../../crates/navmesh-extractor/README.md](../../crates/navmesh-extractor/README.md)
  — extractor phases and how to run it
