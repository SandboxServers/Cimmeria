---
name: harset-nav-does-not-cover-upper-quarters
description: harset.nav only covers the stargate plaza; the Jaffa Zone, OP-CORE, shield towers and palace terrace have no mesh at their real floor, so "is_point_valid on every spawn" is an unsatisfiable acceptance criterion there
metadata:
  type: project
---

`data/spaces/harset.nav` does not model world 57's upper quarters at the floor
height the geometry actually has. Verified 2026-09-19 with `nav_inspect
--h-tol 1.2 --v-tol 4.0` (the gate `NavGraph::is_point_valid` applies:
`agent_radius * 2` = 1.2 m horizontal, `JUMP_HEIGHT_TOLERANCE` +4.0 m) plus
`obj_slab` for the true floor.

**Why:** the mesh has 1,939 connected components for 686,461 m2. Component 187
(24,771 m2, x[-130.8, 118.5] y[-71.8, -37.6] z[-208.1, 74.8]) is the stargate
plaza plus the merchant street and is the only usable hub component. Everything
above it is fragments or nothing.

**How to apply:**

- A ring probe at radii 0-12 m around *every* named Jaffa Zone landmark
  (`JF-Tent00/01/02/03`, `GA-Barracks01`, `JF-MilitaryTent00`,
  `JF-HighWallArch00`, both `TOL-FluidPlaneCircle_Flat00` fountains, and
  `FirstBug`'s own position) found **no** on-mesh point at y = -41.28, the
  floor `obj_slab` reports at all of those columns. Same at all three
  `GA-Tow*` shield towers and on the ring-left-top palace terrace.
- The AUTHORED rows prove it is the mesh and not the placements: Petbe
  (spawn 223) is `dy = -10.99 m`, `FirstBug` (spawn 224) `dy = -10.37 m`, and
  four of the five authored ring pads are off-mesh too — including
  `HarsetRingLeft` (spawn 128, `dy = -9.37 m`), which players demonstrably
  reach because the ring puts them there.
- So **do not** write "`is_point_valid` for every world-57 spawn" as a test.
  It cannot pass. Write a biconditional over an explicit exception table
  instead (a row recorded on-mesh must stay on-mesh, a row recorded off-mesh
  must stay off-mesh) plus an on-mesh control so a mesh that failed to load
  cannot pass the off-mesh half vacuously. See
  `crates/services/src/cell/spawner_tests/harset/world57_placement.rs`.
- World 57 is `navmesh_mode = 'advisory'` (H53), so off-mesh does not
  rubber-band a player. It does kill NPC pathing, which is why everything
  placed in those quarters is `is_stationary = true`.
- `nav_inspect` uses `locate_within` (tolerance-first), so it does not suffer
  the stacked-sheet false negative that `NavGraph::locate` does — but when
  nothing is within tolerance it still prints the best XZ match, so read the
  `dy` rather than the component id.

Related: [[navmesh-containment-modes]], [[navmesh-probe-and-bsp-traps]],
[[navmesh-recast-and-castle-topology]].
