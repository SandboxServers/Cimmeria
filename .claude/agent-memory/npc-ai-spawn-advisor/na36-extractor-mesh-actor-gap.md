---
name: na36-extractor-mesh-actor-gap
description: NA36 widened navmesh-extractor's StaticMeshActor filter to KActor/FracturedStaticMeshActor (unconditional) and InterpActor (opt-in, off by default); did NOT close the Harset raised-platform gap or spawn 308
metadata:
  type: project
---

NA36 (2026-09-25, branch `npcai/na36-extractor-gaps`) fixed a real bug in
`crates/navmesh-extractor`: the static-mesh walker only accepted class
`StaticMeshActor` exactly, silently dropping `InterpActor` / `KActor` /
`FracturedStaticMeshActor` exports (all `AStaticMeshActor` siblings with
identical shape) before the walk even visited them.
`staticmesh::MESH_ACTOR_CLASSES` now covers `KActor`/
`FracturedStaticMeshActor` unconditionally (zero shipped instances of
either, so this is currently a no-op safety net). **`InterpActor` is
opt-in, off by default** (`OPT_IN_MESH_ACTOR_CLASSES`,
`ExtractOptions::include_interp_actors`) — a same-day follow-up walked
back the first pass's unconditional inclusion after a review flagged
that `InterpActor` is disproportionately doors/gates/lifts/elevators in
this content (confirmed: 34 of 754 resolved InterpActors across 23 maps
are literal doors, another 15 are a Stargate's rotating chevron
mechanism), and a mover's cooked pose can be its usually-closed
design-time state rather than its runtime one — baking that into a
`.nav`/`.occ` risks sealing a doorway or blocking sight through an open
one. `harset.nav`/`harset.occ` ship built **with** the flag on — Harset's
31 InterpActors were checked by hand and are all static dressing (mostly
`GLB-RingTransporter00` ring-transport platforms), none a door. The
other 14 InterpActor-carrying maps (Agnos, Beta_Site_Evo_1, Castle,
Castle_CellBlock, Dakara_E1, Login_Map, Lucia, Menfa_Dark, Menfa_Light,
Omega_Site, SGC_W1, Sewer_Falls, Tollana) default off until each gets the
same per-instance check.

**Why this matters for future spawn-placement work:** the fix did NOT
resolve either piece of evidence it was framed around. The five clustered
Harset telemetry positions (`lv06/07/14/18/20` in
`docs/analysis/harset-rebuild/placements/data/harset_lastvalid_probes.txt`,
8-10 m above the nearest decoded geometry) still have zero geometry of any
class nearby at the target height after the fix — confirmed with `obj_slab`
and a full actor-proximity scan. Spawn 308 (`Harset_ShieldTower1`) also
still fails: its tower prefab was already decoded pre-fix, the problem is
Y-calibration on a hillside compound mesh, not a missing class. Both need
a live-client `.location` reading, not more extractor work.

**Why to apply:** before assuming "an undecoded actor class" explains an
off-mesh spawn or a telemetry gap, check whether any export of any class
sits near the target XZ at the target height at all — an extractor gap
requires a candidate export that failed to resolve, not just an absence
of geometry. See
[docs/analysis/npc-ai-restoration/worknotes/na36-extractor-mesh-actor-gap.md](../../../docs/analysis/npc-ai-restoration/worknotes/na36-extractor-mesh-actor-gap.md)
and [docs/engine/navmesh-build-pipeline.md §11](../../../docs/engine/navmesh-build-pipeline.md#11-mesh-actor-class-gap-interpactor--kactor--fracturedstaticmeshactor-na36-2026-09-25)
for the full evidence trail. Related: [[harset-zone-evidence]].
