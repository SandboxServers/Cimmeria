---
name: sgc-w1-navmesh-groups
description: SGC_W1 navmesh has 68 components; the Gate Room with requisitions (47), briefing (42), Failsafe (3) and terminal (26) are separate groups, so nav_inspect same-component checks must run per group
metadata:
  type: project
---

SGC_W1 (`data/spaces/sgc_w1.nav`, NA26 advisory) has 68 navmesh components. nav_inspect exits 3 when probes land in more than one component, so one probe file cannot cover several rooms.

Groups measured for the US-01 placement seal (2026-10-18):

- Gate Room + Control Room + requisitions: component 47 (P01 chair z 46.7, P08 and P13 arrival (167.0, -10.8, 18.0)). The gate-axis strip x 172-175, z 8-18 has no floor poly and its skirt polys sit 0.3-0.9 m above y -10.9, so an arrival near x 170-171 is inside geometry; probe a 1.5 m ring of cells (h <= 0.2) before sealing an arrival.
- Briefing: component 42 (Woolsey, Hammond home, P15, P16).
- Failsafe: component 3 (P05, P06, P07, and the 1566 Jaffa pair at (120,-180) and (116,-176)).
- Requisitions room: floor y -10.85 is part of component 47 (same as the Gate Room); component 45 (y -3.75) is the storey above. Crate centres at the wall edge make nav_inspect pick the overhead poly (dy -7.1), so probe a standing point 2-4 m off the wall, not the crate.
- Terminal: component 26 (standing point (-47.5, 1.31, 34.4)).

`--gaps` finds no route within 3 m between components 3, 42 and 47. The cross-room route (elevator rows 55/76, unverified) is still owed a lab check.

Device and terminal target centres sit in navmesh holes; use a standing point 2.8 m away.

Why: the packet's acceptance expected one exit-0 run for all points; the mesh does not allow it.
How to apply: probe interact or arrival points group by group; for a route between groups, record a lab check rather than a navmesh pass.
