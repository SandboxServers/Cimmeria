# HA-01a1 — interior landing geometry check (Market 69, Storage 70)

> Type: research worknote. Audience: coordinator, mission and world advisors. Date: 2026-10-18.
> Packet: [HA-01a1](../work-packets.md#ha-01a1--interior-landing-geometry-check). Inputs: [BSP door probe](HA-01a-bsp-door-probe.md), [HA-02 revalidation](HA-02-offline-revalidation.md). Candidates: [placements.md](../placements.md).

Offline and read-only. No lab, no client, no server, no Ghidra, no DB. One foreground lane build, then
scratch runs outside the repo. No seed, code or ID was changed and none is allocated here. Nothing below
has been tested in game.

## Result: PARTIAL, with three changes to the HA-01a estimates

Labels: FACT (measured here, tool and number given), INFERENCE (confidence stated), DECISION (an owner
record), PROVISIONAL (a draft for the coordinator's O14-12 call).

- FACT: all three estimated landings sit on real floor at the heights the probe guessed (within 0.01 m
  for Market, 0.08 m for Storage), with 7.7 m or more of headroom. The floor-height question is closed.
- FACT: only one landing is a poor stand point. Market door 1006's landing has a free radius of 1.0 m
  on the mesh, below the 1.5 m pad, and the mesh stops 2.4 m short of that arch. The other two have
  3.5 m and 4.0 m.
- FACT: the world label is settled. World 69 is `harset_market.nav`, world 68 is `harset_cmdcenter.nav`.
  The HA-02 heading is a typo, and its numbers were taken on the Market mesh.
- INFERENCE (medium): E03 should be dropped, the Market pocket box shrunk as HA-02 said, and Storage
  region 2122 left alone in favour of a separate upper-wing region. Details in [Recommendations](#recommendations-for-the-o14-12-call).

## Method

1. Lane build, once, foreground, from the worktree: `pwsh tools/build-lane/lane.ps1 cargo build -p cimmeria-navmesh-extractor --release --bin extract_map --bin obj_slab`. Status ok, exit 0, 80 s, first try. Log: `C:/Users/Steve/AppData/Local/cimmeria-build/logs/zone-harset/20261018-142724-35416.log`.
2. `extract_map extract` on `Harset_Market` and `Harset_StorageRm` from the read-only cooked root into a scratch directory outside the repo. Both exit 0 and both coverage reports are balanced (`balanced` and `sources_balanced` both yes).
3. `obj_slab --column`, `--at`, `--line` and `--levels` at the three landings on those OBJ chunks.
4. `nav_inspect` (the debug binary HA-02 already built; no second build) with `--probe` and `--probes` at the same points, tolerances tightened to horizontal 0.3 m and vertical 0.6 m for pad checks. A scratch script made a 0.5 m grid and a free-radius map (the largest disc of mesh-valid cells around a point).
5. Coordinates are BigWorld metres as in the door probe. Y is up.

Coverage FACT: Market main tile 164,885 triangles (144,548 StaticMesh, 19,702 terrain, 635 BSP, 51 hull cap); 246 collision-disabled actors skipped. Storage main tile 269,259 triangles (253,944 StaticMesh, 253 BSP, 11 hull cap), none skipped. `skip_interp_actor_*` is 0 in both, so doors that are InterpActors are not in these OBJs; there is no InterpActor in either map's class census.

## The three landings

| Item | Estimate (HA-01a) | True floor y (obj_slab column) | Headroom above floor | Mesh at the point | Free radius on mesh | Verdict |
|---|---|---|---|---|---|---|
| Market, door 1006 | (89.5, 3.6, 60.3) | 3.61 | ceiling 17.60 (13.99 m) | valid, component 5, h 0.00, dy -0.15 | 1.0 m | floor OK, too tight |
| Market, door 1551 | (19.9, 4.8, 9.0) | 4.81 | ceiling 13.28 (8.47 m) | valid, component 5, h 0.00, dy -0.15 | 3.5 m at (20.0, 9.0) | PASS |
| Storage, door 28 | (61.4, 5.2, 5.2) | 5.12 | ceiling 12.80 (7.68 m) | valid, component 4, h 0.00, dy -0.12 | 4.0 m | PASS |

All FACT. Notes:

- The Market column at (89.5, 60.3) also lists surfaces at y 1.80 and 0.00. Those are lower decks below the plaza floor, not obstructions. Nothing lies between 3.61 and 17.60.
- Door 1551's corridor floor is a doubled surface at y 4.81 (two coincident faces). It is a plain floor.
- Storage: the surface at y 3.84 (facing down) under the landing is the underside of the upper-wing slab, and 5.12 is its top. The landing stands on the slab, above the pen, not in it.
- The nav mesh floor sits slightly below the OBJ floor (0.11 to 0.16 m in Market, 0.04 to 0.12 m in Storage). That is the 0.2 m cell height, not a geometry fault. The landing y should be the OBJ floor value, as the table shows.
- The pad check (8 points at 1.5 m and 8 at 3.0 m, all at the true floor y) passes on every point for 1551 and the Storage landing. For 1006 the point at 1.5 m to the south-east and four of the eight points at 3.0 m fall off the mesh.

## Market door 1006: what is around the landing

- FACT: the mesh around the landing is a strip, not a hall. At z 60 to 61.5 it runs x 78 to 92. East of x about 90.5 (for z below 59.5) and x about 92.5 (for z 60 to 61.5) there is no walkable mesh. A prop cluster fills x 89.4 to 95.6, z 52.7 to 59.7; surfaces in it top out 0.6 to 1.9 m above the plaza floor (levels at y 4.2 to 5.5, 1,000-plus small triangles). The nearest hole edge to the landing is about 1.0 to 1.3 m away.
- FACT: the best open ground nearby is at (80.0, 3.61, 61.0) with a free radius of at least 4.0 m (the scan caps at 4.0). It is 9.5 m west of the estimate and 17.3 m from the arch.
- FACT: the mesh reaches no closer to the arch than about x 95.5 (at z 62.5 to 63.5), 2.4 m from the arch origin (97.28, 3.52, 60.32). No cell at the arch's own z, x 93 to 98, is walkable. The mesh is advisory in the seed, so this does not stop travel, but a player landing there has a 1 m disc of validated ground.
- FACT (odd): `obj_slab --line` finds a zero-thickness occupied cell at exactly x 97.25 to 97.30, running the whole of z 34.5 to 98.6 and every band from y 3.6 to about 15, including the arch's own z 60.3. Lines at x 96.5, 97.0, 97.2, 97.35, 97.6, 98.0 and 99.0 along z are all clear. The thin plane lies on the arch origin x.
- INFERENCE (low): this is a boundary plane of the Market room (BSP hull cap or a thin wall) and it seals the arch opening in collision, so the arch is decorative or its door is a Kismet actor that is not baked. Alternative: a degenerate sliver the occupancy grid reads as a wall. The slab around z 59.6 to 61 has no steep triangles, which argues for the sliver. It does not affect any landing, which lie 7 m or more inside. It does affect any claim that 1006 is walk-through.

## Market door 1551: corridor

- FACT: the corridor at z 3 and z 9 is 12.5 m wide, x 13.75 to 26.25 (thin walls at x about 13.65 and 26.3), centred at x 20.0. Floor 4.81, ceiling 13.28.
- FACT: along x 19.9 the collision is clear from z 0.75 northwards for 29 m, with narrow occupied cells at z -1.0 to 0.15 and 0.6 to 0.75. The arch origin is z 1.28. The south end is sealed 1.3 to 2.3 m behind the arch.
- FACT: the best disc is at (20.0, 4.81, 8.5), free radius at least 4.0 m. The estimate (19.9, 9.0) is 3.5 m, which also passes the pad.
- FACT: HA-02's safe sub-box x[17,24] z[9,21] and the shrink x[16,24] z[9,22] both sit inside the 12.5 m corridor with at least 2.2 m to each wall. No geometry reason to shrink further.
- INFERENCE (low): of the two Market arches, 1551 offers a clean, walled-in landing in a straight passage and 1006 faces a prop field. That is a statement about the landing, not about which arch is the street door; that choice stays the one-glance owner check in HA-01a.

## Storage door 28

- FACT: the landing is on the upper-wing slab top (y 5.12, slab x 41 to 76.8, z -2.6 to 17.9). The floor runs to z -2.6, the arch origin z, and nothing in the band y 5.32 to 6.92 blocks x 61.4 from z -6 to 20 (26 m clear). Along x at z 5.2 and z 2.0 it is clear for the full 55 m scanned (x 35 to 90).
- FACT: free radius 4.0 m at (61.4, 5.12, 5.2) and at (61.5, 5.12, 5.0) and (61.5, 5.12, 5.5). Component 4, the same component as the pen (E08 to E12) and the other upper-wing points, per HA-02.
- FACT: a prop cluster, x 44.4 to 54.1, z 0.9 to 10.5, rises 0.9 to 1.4 m above the slab (surfaces at y 6.03 to 6.50). It is 7 m or more west of the landing and does not touch it.

## World label

- FACT: `db/resources/Worlds/Seed/worlds.sql` line 167 gives world 69 `world='Harset_Market'`, `client_map='Harset_Market'`, and the comment above it names `data/spaces/harset_market.nav`. Line 291 gives world 68 `Harset_CmdCenter`, comment `harset_cmdcenter.nav`. Line 175 gives world 70 `Harset_StorageRm`, `harset_storagerm.nav`.
- FACT: I probed E01 to E07 from [placements.md](../placements.md) on both meshes with `nav_inspect`. On `harset_market.nav`: E04 h 1.44, E06 h 1.78, E07 h 1.56, E03 on component 15 at dy -14.11, the rest on component 5 with dy about -0.1. These are the HA-02 numbers. On `harset_cmdcenter.nav` all seven are out of tolerance by 11 to 35 m in y.
- FACT: so the HA-02 section headed "World 69 (harset_cmdcenter.nav)" was measured on the Market mesh; the heading is a label error, not a data error. HA-02's World 68 section (the 6006 landing) is the one that belongs to `harset_cmdcenter.nav`. This note does not edit HA-02; the coordinator should correct the heading.
- INFERENCE (high): the repository's own world naming settles this. It needs no client check.

## Recommendations for the O14-12 call

All INFERENCE or PROVISIONAL, for the coordinator. Each proposed placement is CIMMERIA_RECONSTRUCTION of an interior arrival; none claims a retail coordinate. `ID needed` for any new point-set or region row; no ID is chosen here.

1. **Landings (PROVISIONAL, confidence medium).**
   - Door 1551: arrival (20.0, 4.81, 8.5), free radius 4.0 m.
   - Storage door 28: arrival (61.4, 5.12, 5.2), free radius 4.0 m. Use y 5.12, not 5.2.
   - Door 1006: if it is the chosen exit, arrival (80.0, 3.61, 61.0), free radius at least 4.0 m, 17 m from the arch. The original (89.5, 3.61, 60.3) validates but leaves 1.0 m of mesh and sits beside a prop field. A closer point needs a prop-clearing decision, which is a map change and so a **MAINTAINER DECISION**.
2. **E03 (high): drop it.** (94.5, 3.65, 44.9) is on a different component (15) at about 14 m below the plaza on the mesh. The OBJ column there shows floor at 3.61 with a 0.10 m double surface and a stair-like face at y 8.42 above it, and the Market mesh has no walkable cell there. Move rather than drop only if a placement is needed there, and then re-measure (HA-02's rule).
3. **Market pocket box (high): shrink to about x[16,24] z[9,22]**, as HA-02 recommended. This note finds the corridor 12.5 m wide, so the shrink is a mesh-coverage limit, not a geometry one. The corridor is also outside set 2121 (x[30,100]), which is expected for a spawn set.
4. **Storage region 2122 (medium): do not raise Y max to cover E13 to E15. Add a separate upper-wing region instead.** The "ceiling 4.00" is not a pen ceiling in general. At E15 (z 40.7) it is the slab underside (3.84); at E10 (z 59.5) the pen ceiling is 8.47. Raising Y to cover the slab top (5.12) plus the 1.5 m pad would also admit the space above the pen's north end. An upper-wing region over the slab (the HA-02 grid box x[40,76] z[0,16] is the starting point) keeps the two levels apart. E15 (50.8, 5.22, 40.7) is at the slab's southern edge and needs its own check against that region.
5. **E13 (medium): use the HA-02 snap (50.65, 5.24, 5.67).** E13 itself (49.3, 5.22, 5.8) is inside the prop cluster: the column there has a surface at y 6.03, only 0.91 m above the slab, under the 1.8 m agent height. The snap point's column is clear.

Not decided here: which Market arch is the exit; the two world-57 door positions; spawn 308 and the five raised points (HA-02 left them open); whether to clear props.

## Limits

- Landings are validated against baked collision OBJs and the shipped advisory meshes. Door actors that are InterpActors are not baked. Level geometry the extractor skips (246 collision-disabled actors in Market) could still matter in game.
- `obj_slab --line` is a 0.1 m occupancy grid. The thin-plane reading at x 97.28 is low confidence.
- `nav_inspect` locate is Y-biased (HA-02). Pad checks pass the true floor y, a 0.3 m horizontal and 0.6 m vertical tolerance, and require component 5 or 4, so a lower deck cannot satisfy them. A "fail" at tight tolerance means "not validated", not "blocked".
- Free radius is measured on a 0.5 m grid, capped at 4.0 m.
- A passing check proves mesh and geometry agree. It does not prove the client places the player there, nor that clearance matches the client's capsule.

## Tests the implementation owes

Named for the implementer to adopt or rename. Each guard must fail when its change is reverted.

| Proposed test | What it asserts | Revert that must fail it |
|---|---|---|
| `harset_interior_landings_stand_on_the_mesh` | Each chosen landing is valid on the live nav for its world (69 market, 70 storagerm) at 0.3 m / 0.6 m, with a pad of eight points at 1.5 m. Includes a nonvacuity control: a point known off the mesh (E03) is rejected | Replace a landing with the old estimate for door 1006 (89.5, 3.61, 60.3) or E03; the pad or control fails |
| `harset_interior_landings_reach_their_spawn_set` | A path from each landing reaches a point in the spawn set it serves (`find_path`, last waypoint within 2 m XZ, as `interior_regions.rs` does) | Point a landing at the wrong component |
| `harset_world_navmesh_files_match_world_ids` | World 69 maps to `harset_market.nav`, 68 to `harset_cmdcenter.nav`, 70 to `harset_storagerm.nav` | Swap two world-to-mesh entries |
| Upper-wing region containment | E14, E15 and the E13 snap lie inside the new upper-wing region and outside the pen region 2122 | Remove the region or drop its Y range |

Seed-side tests follow the live-DB rules in TESTING.md: seed reload in the worktree database only, and a skipped live-DB pass is not acceptance.

## Telemetry the implementation owes

- On each interior arrival: world id and name, landing coordinate, the validator's verdict (valid, snapped, recovered, suppressed), and the distance to the nearest mesh edge. A silent `CorrectionSuppressed` after an arrival is the failure shape for an off-mesh landing; it must be visible in the structured log.
- Region entry for 2121 and 2122 and the new upper-wing region, with the entity's world and position.
- Pair every id with its name, as the campaign's instrumentation rules require.

## Evidence kept outside the repo

Scratch directory (not committed): a session scratch folder outside the repository, named `ha01a1`. It holds the two extracted OBJ sets, the coverage TSVs, and the probe and grid scripts. It is session scratch and may be removed.
