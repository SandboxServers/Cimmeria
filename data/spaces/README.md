# Navmeshes (`data/spaces/*.nav`)

One XRC `.nav` per world, loaded at space creation by
`crates/services/src/cell/space_manager/lifecycle.rs` (file name = lower-cased
world name). A world with no file has no navmesh: NPCs path in straight lines
and the position / line-of-sight checks fail open.

| File | Built | Agent (height / climb / radius) | Notes |
|---|---|---|---|
| `castle_cellblock.nav` | 2026-09-19, rebuilt from the cooked client maps | 1.8 / 0.6 / 0.6 | StaticMesh + Terrain + BSP extraction (`crates/navmesh-extractor`, PR #683), NavBuilder `partition=watershed agentHeight=1.8 agentClimb=0.6 minRegionSize=24 maxSimplificationError=1.3`. 3,039 verts / 1,658 polys / 17 components. Replaces the 2013 mesh (2,778 / 1,479 / 50), which is in git history. |
| `castle.nav` | 2026-09-19, built from the cooked client maps | 1.8 / 0.6 / 0.6 | StaticMesh + Terrain + BSP extraction (`crates/navmesh-extractor`), NavBuilder `partition=watershed agentHeight=1.8 agentClimb=0.6 minRegionSize=24 maxSimplificationError=2.5`. 40,068 verts / 19,815 polys / 549 components. World 8 is seeded `navmesh_mode = 'advisory'`: the mesh is used for pathing, line of sight and height only, never as a containment gate — exterior and interior are still separate regions. |
| `agnos.nav`, `harset.nav`, `harset_storagerm.nav`, `sgc_w1.nav` | 2012-2014, original emulator | 0.6 / 0.9 / 0.6 | Not rebuilt yet. |

Why the Cellblock mesh was replaced: over three days of colo play the server
snapped players back 212 times in Castle_CellBlock. Scored with the same gate
`NavMesh::is_point_valid` uses, 110 of those positions are valid on the 2013
mesh and 195 on the rebuilt one; the largest cluster is a corridor at
(-191..-202, 54.8, -112..-120) that the 2013 mesh ends 1-2 m short of.
