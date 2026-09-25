---
name: npc-ground-clamp-and-detour-traps
description: NA11 ground clamp — the Cellblock floor-then-ramp fixture, a self-referential floor metric, Detour moveAlongSurface/BV-edge traps, and the cc build.rs no-rebuild trap for detour_wrapper.cpp
metadata:
  type: project
---

Facts found while landing NA11 (2026-09-25, branch `npcai/na11-ground-clamp`).

- **Fixture leg.** `find_path` from the Cellblock guard spawn (-289.465, 68.542, -154.276) to (-315.6, 73.6, -191.4) returns 3 corners: a 3.4 u flat leg, then ONE 42.8 u leg from Y 68.6 to 73.6 whose chord floats 1.95 u over the floor at t=0.39. The first 6 ticks of that leg are on level 68.6; a detail-mesh bump (vy 0.07-0.34 u/s) starts at tick 7 — a "flat floor vy" test must stop before it.
- **The per-tick floor metric is self-consistent by construction** once the clamp exists (`get_navmesh_height(x, y_npc, z)` re-reads the same storey). It still discriminates the old lerp (234 of 34,412 sweep ticks > 0.3 u, worst -4.67). Don't read "worst 0.000" as proof of anything beyond "no lerp fallback fired".
- **`DT_STRAIGHTPATH_ALL_CROSSINGS`** leaves the XZ track identical and only adds corners; each corner is an arrival snap that forfeits the rest of that tick's step (+3.5% ticks on the sweep). Crossings alone (no clamp) still had 4 ticks > 0.3 u, worst +3.07.
- **Detour `moveAlongSurface` does not project `resultPos`** (its Y is whatever you passed) — ground it with `getPolyHeight` on the last visited poly. And a result stopped against the mesh's OUTER AABB edge (Cellblock +X at -253.0) is missed by the BV-tree `findNearestPoly` lookup, so `find_path` could never start from it: `NavMesh::pathable_toward` halves back toward the start.
- **`crates/entity/build.rs` did not rebuild `detour_wrapper.cpp` on edit** because `cc` prints `rerun-if-env-changed`, which disables cargo's default "any package file" rerun. Fixed with explicit `rerun-if-changed`; if a C++ edit seems to have no effect, check this first.
- Worktree-isolated Bash accepts `export VAR=1 && lane.sh cargo ...` (env-var revert hooks for revert-proofs), where `env VAR=x cmd` is refused.

Related: [[navmesh-containment-modes]], [[navmesh-probe-and-bsp-traps]].
