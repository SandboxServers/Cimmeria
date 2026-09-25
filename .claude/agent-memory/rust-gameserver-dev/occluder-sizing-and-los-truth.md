---
name: occluder-sizing-and-los-truth
description: NA27 collision-geometry LoS occluder: what made it fit (paging + explorable trim), build-determinism traps, and the accuracy-metric traps
metadata:
  type: project
---

NA27 (#784, D-NA13) ships a paged `data/spaces/<world>.occ` for all 23 client worlds: 90.6 MB in total, as of 2026-09-25. The unpaged, untrimmed phase-1 build was no-go, because Agnos needed 57 MB on disk and 288 MB of RAM. The owner then raised the budget. Numbers are in `docs/analysis/npc-ai-restoration/worknotes/na27-occluder-phase1.md` and in the `data/spaces/README.md` occluder section.

**Why:** the outdoor maps carry 10-30 M collision triangles. Only paging (64 m pages, unpacked within 132 m of a player) plus trimming to the explorable navmesh components made them cheap: Agnos is 5.9 MB resident with one player.

**How to apply:** before touching the occluder build or any per-world geometry artifact, know these.

**Size**
- **Terrain goes in a heightfield.** SGW terrain is a world-aligned 1 m lattice on every map. An exact vertex-height heightfield costs about 3 B/m². Span columns over slopes caused most of the false blocks.
- **Explorable trim.** Keep the navmesh components that hold a seed or map entry point, grow them across door and stair gaps, and add a 15 m margin. Clip giant triangles to that coverage. `Omega_Site_CmdCenter`'s few enormous triangles filled a 4.7 km grid: 682 MB became 6.4 MB.
- **PlayerStart lives in the persistent `<Map>.umap`, not the chunks.** `umap::enumerate_chunks` skips that file. Most maps have no PlayerStart at all, so five worlds fall back to keeping every component.

**Build determinism**
- **The StaticMesh walk's triangle order is not stable between runs.** It comes from hash-map iteration. Sort each chunk's triangles, and never merge spans incrementally: merge order changes the output.
- **`mv`/`shutil.move` of a `.bak` restores the OLD mtime,** so cargo keeps the stale (reverted) build. After a revert-proof restore, `touch` the file.

**Accuracy metrics**
- NA16's "wrong given Blocked" counts rays grazing within 2-10 cm of a wall edge, and those dominate every grid's residual error. Report the per-truth rates, and a figure with grazing within 0.1 m excluded, alongside it.
- On `main`'s `castle.nav` the navmesh ray calls 23% of truly blocked pairs clear, so it sees through walls.
