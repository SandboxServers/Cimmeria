---
name: tiled-navmesh-seams
description: NA28 tiled .nav (XRCT) for the 7 big exteriors; the seam-island trap, grid-phase coverage loss, poly-ref bit budget, and how to validate a rebuilt mesh against the old one
metadata:
  type: project
---

NA28 (2026-09-25) made the seven big exteriors (Agnos, Beta_Site_Evo_1,
Dakara_E1, Lucia, Menfa_Dark, Menfa_Light, Tollana) whole-map tiled meshes
(`NavBuilder tile=128`, `XRCT` layout, loader auto-detects by magic). They
stay advisory.

**Why:** single-rcPolyMesh caps forced NA26 to crop or coarsen them; owner
approved the tiled follow-up.

**How to apply:**

- Tiled Recast keeps every small island that touches a tile border (Recast's
  region filter exempts border regions). Without NavBuilder's seam filter
  Agnos had 1,262 components under 10 m2 against 0 single-mesh. A tiled
  build with "lots of tiny components" means `seamFilter=0` or a filter bug.
- The filter measures region spans, not polygon area: contour
  simplification narrows thin walkways, so poly area over-removes.
- Moving the heightfield origin (crop corner -> chunk grid) shifts every
  voxel boundary; thin features change width by a cell. A lost catwalk after
  a rebuild with new `bounds=` is grid phase, not tiling (same-crop tiled
  build kept 99.74 % of old centroids; whole-map 99.1-99.9 %).
- 32-bit dtPolyRef: tile bits + poly bits <= 22. Beta at cs 0.3 / 128 cells
  is 13 + 8. Smaller tiles risk the budget.
- Tiling does NOT lift the 13-bit span height (tiles keep the map's Y range).
- Rebuild validation recipe that worked: old probes stay on-mesh, old
  polygon centroids snapped to the old surface re-tested with
  `is_point_valid` on the new mesh, and probe *groups* (components the
  probes land in) compared member-by-member, not just counted.
- NavBuilder's `chunked` bounds pad with the chunk GRID index (a chunk at
  grid X=3 pads bmin.x to 3.0, not 300) - matters when placing a synthetic
  feature on a tile seam.

See [[na26-all-worlds-navmesh]], [[detour-nearest-poly-escapes-its-box]].
