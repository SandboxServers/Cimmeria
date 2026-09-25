---
name: na26-all-worlds-navmesh
description: NA26 (2026-09-25) shipped a .nav for all 24 spaces.xml worlds, all advisory except Castle_CellBlock; what that does to reject telemetry, the single-tile crop limit, and the 13-bit span-height trap
metadata:
  type: project
---

NA26 put a `data/spaces/*.nav` under every `entities/spaces.xml` world and seeded
21 more worlds `navmesh_mode = 'advisory'`. Castle_CellBlock (12) is the only
meshed world still on `enforce`.

**Why:** owner asked for every world's navmesh; a mesh nobody has walked under
containment must not start snapping players back (the Castle precedent).

**How to apply:**

- Advisory worlds emit no navmesh `movement.validation_reject` rows, and
  `movement.position_sample` is DEBUG and was not exported to SigNoz on
  2026-09-25. So after NA26 real-position evidence for promoting a world to
  `enforce` has to come from the TRACE `advisory_off_mesh_accepted` stream,
  switched on for the session. Rejects in SigNoz before 2026-09-25 are mostly
  Harset (space 65544) and carry no `world` field; map space ids by coordinate.
- nav file name = spaces.xml world name lower-cased, NOT `client_map`.
  SandBox plays on Harset_CmdCenter and loads `sandbox.nav` (a copy).
- Agnos, Lucia, Tollana, Beta_Site_Evo_1 are CROPPED (single-tile XRC cannot
  hold them); outside the crop there is no mesh. A "player fell through / NPC
  walks through walls in Lucia" report outside the crop is the crop, not a bug.
  Crops are in `data/spaces/README.md`. Full coverage needs a tiled Detour mesh.
- Recast span heights are 13-bit: `(bmax.y - bmin.y) / ch > 8191` flattens
  everything above onto one ceiling at exit 0. NavBuilder now refuses it;
  `bounds=` does not crop Y. Tollana ships at `ch=0.3` for this.
- The Harset test fixtures that used the 2012 mesh's Command Center hole now
  use a real SigNoz reject step, (217.61,-41.87,3.66) -> (215.32,-42.29,3.80),
  and an uncovered rooftop position (-148.3,-28.3,4.8). See
  [[navmesh-containment-modes]] and [[arrival-coordinate-offnavmesh]].
