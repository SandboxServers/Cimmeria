---
name: navmesh-onmesh-assertions-are-weak
description: is_point_valid and find_path(..).is_some() both pass for points on the WRONG navmesh component; how to write an on-mesh guard that actually discriminates, plus the nav_inspect-vs-is_point_valid tolerance mismatch
metadata:
  type: project
---

**Read before writing any test that asserts "this coordinate is on the navmesh".** Both obvious primitives are false-positive machines on a real shipped `.nav`.

**Why:** discovered 2026-09-19 seeding `Harset_StorageRm.Storage` (point set 2102, world 70). The first guard asserted `NavMesh::is_point_valid` at five probes and **passed with the region shifted 30 m off the building**.

**How to apply:**

1. **`is_point_valid` only asks "is there a polygon near this point", not "which floor".** `harset_storagerm.nav` has 104 components and three overlap the Storage footprint in Y: the pen-grid floor (component 36, 314 polys, 3055 m²), a disconnected duplicate 1.4 m below it (35/54/55 along each wall), and an **82,249 m² outdoor terrain sheet at the same 0.2-0.4 Y band** (component 0, spanning x/z[-99,199]). Any y≈0.3 point in the whole map answers `true`.
2. **`find_path(a, b).is_some()` is not a connectivity test.** `dtNavMeshQuery::findPath` returns a **partial** corridor to the closest reachable polygon when the destination is on another component, so it answers `Some` across a component boundary. Compare the path's **last waypoint** against the request (2 m tolerance absorbs Detour's end-point snap + string-pull); a partial path stops tens of metres short.
3. **Carry a control that proves the guard still discriminates**: a point 100 m outside the building must read `is_point_valid == true` (it is on the terrain sheet) but must **not** be reachable. Without it the guard silently rots into a tautology.
4. **`nav_inspect --probes` "ok" is a looser gate than the runtime.** nav_inspect uses h-tol 2.0 m / v-tol 3.0 m; `is_point_valid` uses horizontal offset < 2 × agent radius (= 1.2 m for radius 0.6) and dy in [-1.2, +4.0]. A probe reported `ok` with `h=1.93` fails in Rust. Use nav_inspect to find the *shape*, then let the Rust test decide the edges.
5. **Derive a region's footprint by walking the mesh, not from the component's bounding box.** Component 36's bounds are x[16.1,87.5] z[34.1,99.2], but only x[19,84] z[38,97] has *every* metre resolving to 36. A 1 m grid of `nav_inspect` probes printed as a component map is the fast way to find that.
6. **A shipped mesh can be wrong about connectivity, and telemetry is what proves it.** `harset_storagerm.nav` has the Storage pen floor (36) and the upper arrival wing (6) as **separate** components — nothing can path between them — yet real accepted player positions exist on both, and the only route is the descent between them. A rebuilt mesh merges them into one. So "component X and Y are disconnected" from the shipped mesh is a claim about the *mesh*, not about the level. Consequence for content: an NPC spawned across such a seam can never reach the player.

**Player Y sits about 1 m above the nav surface.** On the Storage pen floor the nav band is y 0.2-1.2 and accepted players are at 1.25-1.58; on the upper deck the nav band is 5.2-6.0 and players are at 6.14-7.06. Useful both ways: to sanity-check a derived floor Y, and to set a region ceiling that admits one storey and not the next.

See [navmesh-containment-modes.md](navmesh-containment-modes.md) for the per-world `navmesh_mode` side of this, [telemetry-last-valid-is-mostly-synthetic.md](telemetry-last-valid-is-mostly-synthetic.md) before using `last_valid_*` as evidence, and [navmesh-recast-and-castle-topology.md](navmesh-recast-and-castle-topology.md) for mesh generation.
