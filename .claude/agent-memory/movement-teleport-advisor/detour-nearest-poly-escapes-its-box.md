---
name: detour-nearest-poly-escapes-its-box
description: Detour findNearestPoly admits polys whose AABB overlaps the box, so its closest point can sit outside the box (seen 4.8 u above a ±4 box on castle_cellblock); any "snap within N" must re-check the result
metadata:
  type: project
---

`dtNavMeshQuery::findNearestPoly` selects candidates by polygon **bounds** overlapping the query box, then returns the closest point on the winner. That point is not clamped to the box. On `castle_cellblock.nav`, a query at (-126.25, 34.6, -104.59) with half-extents [8, 4, 8] returned the hallway floor at y 39.4, 4.8 u above: outside the requested ±4 vertical band.

**Why:** found while building NA15's bounded snaps (2026-09-25). A snap that trusts the box can move an NPC or a chase goal onto another storey, which is the floor-clip / storey-jump shape this advisor blocks.

**How to apply:** every "nearest point within R / ±H" helper must re-check both the horizontal radius (the box corner reaches R*sqrt(2)) and |dy| <= H on the returned point. `NavMesh::nearest_point_within` does; `find_nearest_poly` (±3 DEST box) and `SpaceManager::snap_to_navmesh` do not, so a reviewer should ask whether a caller of those can tolerate an out-of-box answer. Related: [[arrival-coordinate-offnavmesh]], [[castle-has-no-navmesh]].

NA15 geometry anchors on the same mesh, useful for future tick tests: MessHall_Guard1 -> ground-plane corner (-400, 0.2, -400) is a 340 u **partial** corridor ending at (-148.3, 24.8, -144.1); a route to a goal equal to the start's own snap point is a **one-waypoint** (degenerate) straight path.
