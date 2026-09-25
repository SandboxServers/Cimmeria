---
name: occluder-sizing-and-los-truth
description: NA27 collision-geometry LoS occluder: why outdoor worlds blow any per-world size budget, which tricks did/didn't shrink it, and the accuracy-metric traps
metadata:
  type: project
---

NA27 (#784) measured a collision-geometry occluder on all 23 maps (2026-09-25). The result was no-go: the biggest world, Agnos, needs 57 MB on disk and 288 MB RAM at 0.5 m cells, against a budget of 10 MB and 50 MB. The code is on `main`-bound branch `npcai/na27-occluder`, and the parked service integration is on `npcai/na27-occluder-phase2-wip`. Numbers are in `docs/analysis/npc-ai-restoration/worknotes/na27-occluder-phase1.md`.

**Why:** the outdoor maps carry 10-30 M collision triangles (Agnos 8.5 M StaticMesh). Any column grid over them costs hundreds of MB.

**How to apply:** before proposing any per-world geometry artifact, check these facts.

- **Terrain is cheap if stored as a heightfield.** SGW terrain is a world-aligned 1 m lattice on every map (only 1,533 fallback triangles across all 23). An exact vertex-height heightfield costs about 3 B/m² and removed Castle's terrain false blocks. Span columns over slopes were the dominant error source.
- **Trimming coverage to the navmesh footprint saves only 3-10%.** NA26's rebuilt outdoor meshes cover nearly the whole terrain.
- **`Omega_Site_CmdCenter` is pathological.** It has 0.48 M triangles on a 4.7 km grid, with 108 M spans. Giant triangles flood every cell. Cap triangle extent before blaming map size.
- **Accuracy metric trap.** NA16's "wrong given Blocked" counts rays grazing within 2-10 cm of a wall edge. Those dominate every grid's residual error regardless of cell size. Report the per-truth rates alongside it, and a "grazing within 0.1 m excluded" figure.
- **The navmesh LoS is worse on Castle than NA16 found on Cellblock.** On `main`'s `castle.nav` it calls 23% of truly blocked pairs clear, which means it sees through walls. That matters if anyone argues the navmesh ray is "safe but blind".
- **An endpoint clearance buys nothing** once spans carry sub-cell rectangles. At 0.3 m it only added false clears.
