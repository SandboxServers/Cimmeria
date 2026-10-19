---
name: omega-placements
description: Omega (worlds 18/80) navmesh-sealed placement traps - prop y is a model origin, bridges unreachable from pad 17, probe y must be re-snapped
metadata:
  type: project
---

Measured 2026-10-18 for UO-01 (worknote `docs/analysis/union-zone-planning/omega-packets/worknotes/UO-01-placements.md`).

- Seed prop y values are model origins: banker prop y -51.96 sits 3.94 m under the floor (-48.02), so a first-pass probe is OUT OF TOLERANCE. Re-snap (y minus `dy`) and re-probe before choosing; also require `h` 0.00 (inside a polygon), not just `ok`.
- `nav_inspect` prints `dy`, not the floor: floor = probe_y - dy. Run the built exe directly (`B:\targets\<wt>\debug\nav_inspect.exe`) for many probes; the lane only logs.
- World 80 bridges E/W are separate components (12, 26) from pad 17 (11); no gap route within 3 m, so nothing can be seeded there.
- Human Weapons prop snaps to component 95, not the hub 60; stand the NPC 1.75 m away on 60.

**Why:** later Omega seed packets copy coordinates from the worknote.
**How to apply:** re-run the probes if either `.nav` is rebuilt; see [[omega-crossings]].
