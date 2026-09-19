---
name: navmesh-probe-and-bsp-traps
description: Navmesh extractor traps — nav_inspect's locate is a false-negative source, the NavBuilder walkable-normal convention, and why a geometric-only BSP filter deletes real floors
metadata:
  type: project
---

Read before diagnosing "the navmesh is missing a floor" or writing any
geometric filter in `crates/navmesh-extractor/src/bsp/`.

## A missing floor is usually the probe, not the mesh

`NavGraph::locate` picks the best polygon whose **XZ footprint contains**
the probe and only then lets the caller apply tolerances. On a stacked
mesh a buried sheet that spans a whole chunk therefore beats the floor
0.6 m to the side, and the probe reports a 41 m drop while standing on
the mesh. Use `NavGraph::locate_within(p, h_tol, v_tol)` — closest
within both tolerances, falling back to `locate`.

**Why:** three of the eleven Castle probes (`throne_room`, `opcore`,
`armory`) read OUT OF TOLERANCE this way. A whole worker-day went into
hunting a BSP winding bug that did not exist.

**How to apply:** before blaming extraction for a missing floor, re-run
the probe with the tolerance-first resolver, and cross-check with a 1 m
grid over the room rather than the single recorded point — a recorded
point can sit inside a pillar.

## NavBuilder's walkable convention, derived once

OBJ column order is `v <ue.X> <ue.Z> <ue.Y>`; `loadOBJ` swizzles
`(x,y,z) -> (z/100, y/100, x/100)` **and** reverses every face. Both are
odd permutations, so they cancel with the writer's Y/Z swap and leave
`N_recast.y = -n_ue3.z` of the *emitted* order — walkable when the
emitted UE3 right-hand normal points **down**. Getting the parity wrong
by one negation inverts every conclusion about which surfaces are
floors. The fixture in `tests/navbuilder_axis_roundtrip.rs::floor_quad`
is the ground truth to check against.

## A geometric BSP filter needs physical evidence

"Drop the outer plane of the model" is a plausible rule that is wrong.
On `Maps/Castle` it removes 246,195 m^2 of buried CSG hull skin; on
`Maps/Castle_CellBlock` the same rule removes two of the three large
sheets the shipped `data/spaces/castle_cellblock.nav` actually contains,
because that interior sits *above* its terrain instead of 45 m under it.

**Why:** the "outer plane" of a model is the topmost floor whenever the
model is a free-standing structure.

**How to apply:** pair any such rule with a physical condition — here,
the chunk's own terrain proving the face is buried (`bsp::TerrainCeiling`).
And always re-run the rule against a **second map** before believing it;
Castle alone would have shipped the false positive.

## Buried sheets do not cost polygons

Removing 87,709 m^2 of flat unreachable sheet from the 144-chunk Castle
build moved the mesh from 45,115 verts / 21,799 polys to 45,200 /
21,801 — 85 vertices *more*. A huge flat sheet is a handful of polygons;
deleting it lets the geometry underneath contour separately. Do not
reach for geometry filtering to fix a Recast budget overflow.
