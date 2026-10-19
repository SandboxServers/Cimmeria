---
name: beta-e1-anchors
description: Beta_Site_Evo_1 nav components, the nav_inspect placement-probe recipe, the dy sign trap, and the beam pads; the sealed anchor table is in docs/analysis/union-beta-e1-continuation/placement-seal.md
metadata:
  type: project
---

Beta E1 (`data/spaces/beta_site_evo_1.nav`, tiled XRCT, agent r0.6 h1.8 climb0.6) component ids: 251 = main valley/SGC (612686 m^2, ARRIVAL, hub, infirmary, Asgard ground, Science front); 70 = west valley (361261 m^2, Nine Peaks camp, SW ring (-791.6,-51.8,-92.6)); 355 = Asgard deck; 890 = Science interior; 889 = annex; 609 = ruin pin; 583/585 = range pin area. 251 and 70 have no walk route (closest direct approach 483.86 m, 7-hop chain with 15-17.8 m vertical hops): west-valley content is beam/ring only.

**Why:** the seal (UB-00b) needed every anchor on a named component; spec pins for the range (1055.6,14.6,334.3), ruin (128.2,22.1,358.8) and Science front z 768.3 land on other components or 28 m off the prefab.

**How to apply:**
- Probe: `nav_inspect <nav> --probes FILE --h-tol 0.3 --v-tol 1.0` (lines "NAME X Y Z"); exit 2 = out of tolerance, exit 3 = probes in different components. Include ARRIVAL (635.761,-60.134,388.448) in a run: same-component is the reachability proxy, but it is necessary not sufficient; confirm with `NavMesh::is_point_valid` and `find_path`.
- dy sign trap: `dy = probe.y - surface.y` (positive = probe floats above). Snapped y = y - dy. ARRIVAL surface is -60.83, not -59.4.
- The mesh is speckled: a strict 8/8 neighbour clearance finds nothing. Use at least 6 of 8 neighbours at 0.6 m, same component, |dy| < 0.4.
- Authored coordinates are prefab origins, often inside prefab holes; snap to a stand point nearby. Census (`archetype_census` with `CIMMERIA_PACKAGE_INDEX`) gives the origins.
- Region test: one-point AreaSet is a box at +-r in X/Z, widened 1.5 on all axes (`GENERIC_REGION_CHECK_THRESHOLD`), so "outside by N" is Chebyshev >= r + 1.5 + N.
- Beam pads (census): ground (746.480,-19.920,68.640) = spawn 149 on 251; deck (700.440,157.240,70.360) = spawn 151 on 355. The deck is not walkable from ARRIVAL.
- Science door gap between 890 and 251 is 0.90 m at (650.8,19.8,814.1)->(651.7,19.8,814.1), narrower than the 1.2 m agent: find_path may refuse it.
