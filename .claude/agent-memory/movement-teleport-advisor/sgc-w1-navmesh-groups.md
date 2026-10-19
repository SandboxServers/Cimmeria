---
name: sgc-w1-navmesh-groups
description: SGC_W1 navmesh has 68 components; the Gate Room (47), briefing (42), Failsafe (3), requisitions (45) and terminal (26) are separate groups, so nav_inspect same-component checks must run per group
metadata:
  type: project
---

SGC_W1 (`data/spaces/sgc_w1.nav`, NA26 advisory) has 68 navmesh components. nav_inspect exits 3 when probes land in more than one component, so one probe file cannot cover several rooms.

Groups measured for the US-01 placement seal (2026-10-18):

- Gate Room + Control Room: component 47 (P01 chair z 47.3, P08, P09, P13 arrival (170.79, -10.9, 11.0)).
- Briefing: component 42 (Woolsey, Hammond home, P15, P16).
- Failsafe: component 3 (P05, P06, P07, and the 1566 Jaffa pair at (120,-180) and (116,-176)).
- Requisitions crates: component 45 (floor y -3.7, not -10.95).
- Terminal: component 26 (standing point (-47.5, 1.31, 34.4)).

`--gaps` finds no route within 3 m between components 3, 42 and 47. The cross-room route (elevator rows 55/76, unverified) is still owed a lab check.

Device and terminal target centres sit in navmesh holes; use a standing point 2.8 m away.

Why: the packet's acceptance expected one exit-0 run for all points; the mesh does not allow it.
How to apply: probe interact or arrival points group by group; for a route between groups, record a lab check rather than a navmesh pass.
