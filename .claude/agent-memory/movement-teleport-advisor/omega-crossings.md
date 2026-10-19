---
name: omega-crossings
description: Omega Site (worlds 18 and 80) measured arrival coordinates, beam pair 14/17, instanced-per-arrival world 80, missing world-18 respawner, alias pattern limits, H7 partly closed
metadata:
  type: project
---

Measured 2026-10-18 with `nav_inspect` through the build lane (Y is vertical; `dy = probe_y - floor_y`).

- World 18 (`omega_site.nav`, non-instanced, advisory, startup space): gate 5 row (34.705, -50.634, -149.092) is on the mesh but 0.41 m under the dais, inside the gate cylinder (point set 1009, r 2.5). Clean arrival for a `cross_world_teleport`: (34.53, -50.23, -146.59), dy +0.08, outside the volume, component 60. The -z half of the dais ring (r 3.5+) is 1.6 m lower; never put arrivals there.
- World 80 (`omega_site_cmdcenter.nav`, INSTANCED, advisory): the whole mesh sits at Y about 35.2. Pad 17 (6.121, 35.23, 9.987) dy +0.09, respawner 26 (10, 35.23, 9.987) dy +0.05. A world-18-style low Y lands far under the floor.
- The 18 to 80 beam is native: ring regions 14 and 17 are a cross-world pair (`ring_transport_regions.sql`). Do not rebuild it as a content teleport (loses the FSM disconnect/timeout paths). Content observes it through `teleport_in` (event key = region id) with `world` + `step_status` conditions.
- World 80 is private per arrival: `find_or_create_space` makes a new space every create; only a GM `destination_space_id` joins one. Squadmates beaming separately do not meet.
- World 18 has no respawner row, so death respawns in place with a WARN (death-loop hazard). World 80 has row 26.
- Instanced alias of a shipped map has no precedent (Debug Area is shared and non-instanced; historical CellBlocks are instanced on their own maps). The combination needs a lab proof.
- The earlier SGC claim that `.nav` files were absent from the worktree was wrong; they are in the main checkout `data/spaces/`.
- H7 (no log for content `cross_world_teleport`) is PARTLY closed: `transport.rs` logs INFO "Content: cross-world teleporting entity" with world, position, chain_id; there is still no arrival-side `movement.teleport` row and no dest-on-mesh result.

**Why:** the Union Omega campaign (UO-00..UO-08) seals arrival coordinates and the beam by these facts.
**How to apply:** re-measure (read-only `nav_inspect`) if the `.nav` files are rebuilt; verify the respawner and alias claims against the seeds before sealing.
